use std::io::{self, ErrorKind};
use std::net::{SocketAddr, IpAddr};
use std::sync::{Arc, Weak, Mutex};

use mio::{Events, Poll, PollOpt, Ready, Token};
use mio::net::TcpListener;
#[cfg(any(target_os = "dragonfly",
target_os = "freebsd", target_os = "ios", target_os = "macos"))]
use mio::unix::UnixReady;
use slab;

use network::replicator_source::ReplicatorSource;
use network::packet::{SerializedBuffer};
use model::config::{Configuration, ConfigurationSettings, PORT};
use network::node::Node;

type Slab<T> = slab::Slab<T, Token>;

pub struct ReplicatorSourcePool {
    poll: Poll,
    sock: TcpListener,
    token: Token,
    conns: Slab<ReplicatorSource>,
    events: Events,
    node: Weak<Mutex<Node>>
}

impl ReplicatorSourcePool {
    pub fn new(config: &Configuration, node: Weak<Mutex<Node>>) -> Self {
        let host = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse host string");
        let addr = SocketAddr::new(host, config.get_int(ConfigurationSettings::Port).unwrap_or(PORT as i32) as u16);
        let sock = TcpListener::bind(&addr).expect("Failed to bind address");

        ReplicatorSourcePool {
            poll: Poll::new().expect("Failed to create Poll"),
            sock,
            token: Token(1_000_000),
            conns: Slab::with_capacity(config.get_int(ConfigurationSettings::MaxPeers).unwrap_or(5) as usize),
            events: Events::with_capacity(1024),
            node,
        }
    }

    pub fn init(&mut self) {
        let so = self.sock.as_sock();
//        self.node.upgrade().unwrap().lock().unwrap().set_receiver()
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.register()?;

        info!("Server run loop starting...");
        loop {
            let cnt = self.poll.poll(&mut self.events, None)?;

            let mut lst = vec![];

            for event in self.events.iter() {
                lst.push(event);
            }

            for event in lst {
                self.ready(event.token(), event.readiness());
            }

            self.tick();
        }
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

        for c in self.conns.iter_mut() {
            if c.is_reset() {
                reset_tokens.push(c.token);
            } else if c.is_idle() {
                c.reregister(&mut self.poll)
                    .unwrap_or_else(|e| {
                        warn!("Reregister failed {:?}", e);
                        c.mark_reset();
                        reset_tokens.push(c.token);
                    });
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
//        debug!("{:?} event = {:?}", token, event);
        let event = Ready::from(event);

        if event.is_error() {
            warn!("Error event for {:?}", token);
            self.find_connection_by_token(token).mark_reset();
            return;
        }

        if event.is_hup() {
            trace!("Hup event for {:?}", token);
            self.find_connection_by_token(token).mark_reset();
            return;
        }

        let event = Ready::from(event);

        if event.is_writable() {
            trace!("Write event for {:?}", token);
            assert_ne!(self.token, token, "Received writable event for Server");

            let conn = self.find_connection_by_token(token);

            if conn.is_reset() {
                info!("{:?} has already been reset", token);
                return;
            }

            conn.writable().unwrap_or_else(|e| {
                warn!("Write event failed for {:?}, {:?}", token, e);
                conn.mark_reset();
            });
        }

        if event.is_readable() {
            trace!("Read event for {:?}", token);
            if self.token == token {
                self.accept();
            } else {
                if self.find_connection_by_token(token).is_reset() {
                    info!("{:?} has already been reset", token);
                    return;
                }

                self.readable(token).unwrap_or_else(|e| {
                    warn!("Read event failed for {:?}: {:?}", token, e);
                    self.find_connection_by_token(token).mark_reset();
                });
            }
        }

        if self.token != token {
            self.find_connection_by_token(token).mark_idle();
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

            let token = match self.conns.vacant_entry() {
                Some(entry) => {
                    debug!("registering {:?} with poller", entry.index());
                    let c = ReplicatorSource::new(sock, entry.index());
                    entry.insert(c).index()
                }
                None => {
                    error!("Failed to insert connection into slab");
                    return;
                }
            };

            if let Err(e) = self.conns[token].register(&self.poll) {
                error!("Failed to register {:?} connection with poller, {:?}", token, e);
                self.conns.remove(token);
            }
        }
    }

    fn readable(&mut self, token: Token) -> io::Result<()> {
        let mut packets_data = Vec::<SerializedBuffer>::new();

        {
            let connection = self.find_connection_by_token(token);
            while let Some(buffer) = connection.readable()? {
                packets_data.push(buffer);
            }
        }

        for data in packets_data {
            let node = self.node.upgrade().unwrap();
            let mut node_guard = node.lock().unwrap();
            node_guard.on_connection_data_received(data);
        }

        Ok(())
    }

    fn find_connection_by_token(&mut self, token: Token) -> &mut ReplicatorSource {
        &mut self.conns[token]
    }
}