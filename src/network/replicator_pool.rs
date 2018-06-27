use std::io::{self, ErrorKind};
use std::net::{SocketAddr, IpAddr};
use std::sync::{Arc, Weak, Mutex};

use mio::{Events, Poll, PollOpt, Ready, Token};
use mio::net::{TcpListener, TcpStream};
#[cfg(any(target_os = "dragonfly",
target_os = "freebsd", target_os = "ios", target_os = "macos"))]
use mio::unix::UnixReady;
use slab;

use network::replicator::ReplicatorSource;
use network::packet::{SerializedBuffer};
use model::config::{Configuration, ConfigurationSettings, PORT};
use network::node::Node;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{LockResult, MutexGuard};
use network::Neighbor;

type Slab<T> = slab::Slab<T, Token>;

pub struct ReplicatorSourcePool {
    poll: Poll,
    sock: TcpListener,
    token: Token,
    conns: Slab<Arc<Mutex<ReplicatorSource>>>,
    events: Events,
    node: Weak<Mutex<Node>>,
    node_rx: Receiver<()>,
}

impl ReplicatorSourcePool {
    pub fn new(config: &Configuration, node: Weak<Mutex<Node>>, node_rx: Receiver<()>) -> Self {
        let host = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse host string");
        let port = config.get_int(ConfigurationSettings::Port).unwrap_or(PORT as i32) as u16;
        let addr = SocketAddr::new(host, port);
        let sock = TcpListener::bind(&addr).expect("Failed to bind address");

        debug!("Started listener on port {}", port);

        ReplicatorSourcePool {
            poll: Poll::new().expect("Failed to create Poll"),
            sock,
            token: Token(1_000_000),
            conns: Slab::with_capacity(config.get_int(ConfigurationSettings::MaxPeers).unwrap_or(5) as usize),
            events: Events::with_capacity(1024),
            node,
            node_rx,
        }
    }

    pub fn init(&mut self) {

    }

    pub fn run(&mut self) -> io::Result<()> {
        use std::time::Duration;
        use std::thread;

        self.register()?;

        loop {
            if self.node_rx.try_recv().is_ok() {
                break;
            }

            let cnt = self.poll.poll(&mut self.events, Some(Duration::from_secs(1)))?;

            if cnt > 0 {
                let mut lst = vec![];

                for event in self.events.iter() {
                    lst.push(event);
                }

                for event in lst {
                    self.ready(event.token(), event.readiness());
                }

                self.tick();
            }

            if let Some(arc) = self.node.upgrade() {
                if let Ok(node) = arc.try_lock() {
                    if let Ok(mut neighbors) = node.neighbors.lock() {
                        for arc2 in neighbors.iter() {
                            let mut f = false;
                            if let Ok(mut n) = arc2.lock() {
                                if n.replicator_source.is_none() {
                                    use std::net;
                                    match net::TcpStream::connect_timeout(&n.addr, Duration::from_secs(3)) {
                                        Ok(stream) => {
                                            if let Ok(stream) = TcpStream::from_stream(stream) {
                                                n.replicator_source = self.add_replicator(stream);
                                            }
                                        }
                                        Err(e) => {
                                            error!("Couldn't connect to neighbor: {:?}", e);
                                        }
                                    }
                                } else {
                                    if let Some(ref weak) = n.replicator_source {
                                        if let Some(arc) = weak.upgrade() {
                                            if let Ok(ref mut replicator) = arc.lock() {
                                                if !replicator.send_queue.is_empty() {
                                                    replicator.interest.insert(Ready::writable());
                                                    replicator.reregister(&mut self.poll);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            thread::sleep(Duration::from_secs(1));
        }

        info!("Shutting down replicator source pool...");
        Ok(())
    }

    pub fn register(&mut self) -> io::Result<()> {
        self.poll.register(
            &self.sock,
            self.token,
            Ready::readable(),
            PollOpt::edge()
        ).or_else(|e| {
            error!("Failed to register server {:?}, {:?}", self.token, e);
            Err(e)
        })
    }

    fn tick(&mut self) {
        trace!("Handling end of tick");

        let mut reset_tokens = Vec::new();

        for arc in self.conns.iter_mut() {
            if let Ok(mut c) = arc.lock() {
                if c.is_reset() {
                    reset_tokens.push(c.token);
                } else if c.is_idle() {
                    c.reregister(&mut self.poll).unwrap_or_else(|e| {
                        warn!("Reregister failed {:?}", e);
                        c.mark_reset();
                        reset_tokens.push(c.token);
                    });
                }
            }
        }

        for token in reset_tokens {
            match self.conns.remove(token) {
                Some(_c) => {
                    debug!("reset connection; token={:?}", token);
                }
                None => {
                    warn!("Unable to remove connection for {:?}", token);
                }
            }
        }
    }

    fn ready(&mut self, token: Token, event: Ready) {
        debug!("{:?} event = {:?}", token, event);
        let event = Ready::from(event);

        if event.is_error() {
            warn!("Error event for {:?}", token);
            if let Ok(mut c) = self.find_connection_by_token(token) {
                c.mark_reset();
            }
            return;
        }

        if event.is_hup() {
            info!("Hup event for {:?}", token);
            if let Ok(mut c) = self.find_connection_by_token(token) {
                c.mark_reset();
            }
            return;
        }

        let event = Ready::from(event);

        if event.is_writable() {
            debug!("Write event for {:?}", token);
            assert_ne!(self.token, token, "Received writable event for Server");

            if let Ok(mut conn) = self.find_connection_by_token(token) {
                if conn.is_reset() {
                    debug!("{:?} has already been reset", token);
                    return;
                }

                conn.writable().unwrap_or_else(|e| {
                    debug!("Write event failed for {:?}, {:?}", token, e);
                    conn.mark_reset();
                });
            }
        }

        if event.is_readable() {
            debug!("Read event for {:?}", token);
            if self.token == token {
                self.accept();
            } else {
                if let Ok(mut conn) = self.find_connection_by_token(token) {
                    if conn.is_reset() {
                        debug!("{:?} has already been reset", token);
                        return;
                    }
                }

                self.readable(token).unwrap_or_else(|e| {
                    debug!("Read event failed for {:?}: {:?}", token, e);
                    if let Ok(mut conn) = self.find_connection_by_token(token) {
                        conn.mark_reset();
                    }
                });
            }
        }

        if self.token != token {
            if let Ok(mut c) = self.find_connection_by_token(token) {
                c.mark_idle();
            }
        }
    }

    fn accept(&mut self) {
        debug!("server accepting new socket");

        loop {
            let sock = match self.sock.accept() {
                Ok((sock, _)) => sock,
                Err(e) => {
                    if e.kind() == ErrorKind::WouldBlock {
                        debug!("accept encountered WouldBlock");
                    } else {
                        error!("Failed to accept new socket, {:?}", e);
                    }
                    return;
                }
            };

            if let Ok(addr) = sock.peer_addr() {
                if let Some(replicator_weak) = self.add_replicator(sock) {
                    if let Some(arc) = self.node.upgrade() {
                        if let Ok(mut node) = arc.lock() {
                            let mut neighbor_exsists = false;

                            if let Ok(mut neighbors) = node.neighbors.lock() {
                                for neighbor_arc in neighbors.iter() {
                                    if let Ok(neighbor) = neighbor_arc.lock() {
                                        if addr.ip() == neighbor.addr.ip() {
                                            neighbor_exsists = true;
                                            break;
                                        }
                                    }
                                }
                            }

                            if !neighbor_exsists {
                                if let Ok(mut neighbors) = node.neighbors.lock() {
                                    neighbors.push(Arc::new(Mutex::new(Neighbor::from_replicator_source(replicator_weak, addr.clone()))))
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn add_replicator(&mut self, sock: TcpStream) -> Option<Weak<Mutex<ReplicatorSource>>> {
        match self.conns.vacant_entry() {
            Some(entry) => {
                debug!("registering {:?} with poller", entry.index());
                let mut replicator = ReplicatorSource::new(sock, entry.index());
                if let Err(e) = replicator.register(&self.poll) {
                    error!("Failed to register  connection with poller, {:?}", e);
                    return None;
                }
                let c = Arc::new(Mutex::new(replicator));
                let weak = Arc::downgrade(&c.clone());
                entry.insert(c);

                Some(weak)
            }
            None => {
                error!("Failed to insert connection into slab");
                None
            }
        }
    }

    fn readable(&mut self, token: Token) -> io::Result<()> {
        let mut packets_data = Vec::<SerializedBuffer>::new();

        let mut addr : SocketAddr;
        if let Ok(mut connection) = self.find_connection_by_token(token) {
            addr = connection.get_address()?;

            while let Some(buffer) = connection.readable()? {
                packets_data.push(buffer);
            }
        } else {
            use std::io::Error;
            return Err(Error::from(ErrorKind::NotFound));
        }

        for data in packets_data {
            let node = self.node.upgrade().unwrap();
            let mut node_guard = node.lock().unwrap();
            node_guard.on_connection_data_received(data, addr);
        }

        Ok(())
    }

    fn find_connection_by_token(&self, token: Token) -> LockResult<MutexGuard<ReplicatorSource>> {
        self.conns[token].lock()
    }

    pub fn shutdown(&mut self) {
        info!("shutting down");
        self.poll.deregister(&self.sock);
    }
}

pub struct ReplicatorSinkPool {

}

impl ReplicatorSinkPool {

}