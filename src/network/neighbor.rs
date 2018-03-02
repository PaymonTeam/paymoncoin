extern crate nix;

use std::io::{self, ErrorKind};

use mio::{Events, Poll, PollOpt, Ready, Token};
use mio::net::TcpListener;
#[cfg(any(target_os = "dragonfly",
target_os = "freebsd", target_os = "ios", target_os = "macos"))]
use mio::unix::UnixReady;

use slab;

use network::connection::Connection;
use network::packet::{SerializedBuffer};

extern fn handle_sigint(_:i32) {
    println!("Interrupted!");
    panic!();
}

type Slab<T> = slab::Slab<T, Token>;

pub struct Neighbor {
    sock: TcpListener,
    token: Token,
    conns: Slab<Connection>,
    events: Events,
    running: bool,
}

impl Neighbor {
    pub fn new(sock: TcpListener) -> Neighbor {
        Neighbor {
            sock,
            token: Token(1_000_000),
            conns: Slab::with_capacity(128),
            events: Events::with_capacity(1024),
            running: true,
        }
    }

    pub fn run(&mut self, poll: &mut Poll) -> io::Result<()> {
        self.register(poll)?;

        info!("Server run loop starting...");
        loop {
            let cnt = poll.poll(&mut self.events, None)?;

            let mut lst = vec![];

            for event in self.events.iter() {
                lst.push(event);
            }

            for event in lst {
                self.ready(poll, event.token(), event.readiness());
            }

            self.tick(poll);
        }
    }

    pub fn register(&mut self, poll: &mut Poll) -> io::Result<()> {
        poll.register(
            &self.sock,
            self.token,
            Ready::readable(),
            PollOpt::edge()
        ).or_else(|e| {
            error!("Failed to register server {:?}, {:?}", self.token, e);
            Err(e)
        })
    }

    fn tick(&mut self, poll: &mut Poll) {
        trace!("Handling end of tick");

        let mut reset_tokens = Vec::new();

        for c in self.conns.iter_mut() {
            if c.is_reset() {
                reset_tokens.push(c.token);
            } else if c.is_idle() {
                c.reregister(poll)
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

//    #[cfg(any(target_os = "dragonfly",
//    target_os = "freebsd", target_os = "ios", target_os = "macos"))]
    fn ready(&mut self, poll: &mut Poll, token: Token, event: Ready) {
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
                self.accept(poll);
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

    fn accept(&mut self, poll: &mut Poll) {
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
                    let c = Connection::new(sock, entry.index());
                    entry.insert(c).index()
                }
                None => {
                    error!("Failed to insert connection into slab");
                    return;
                }
            };

            match self.find_connection_by_token(token).register(poll) {
                Ok(_) => {},
                Err(e) => {
                    error!("Failed to register {:?} connection with poller, {:?}", token, e);
                    self.conns.remove(token);
                }
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
            self.on_connection_data_received(token, data);
        }

        Ok(())
    }

    fn on_connection_data_received(&mut self, token: Token, mut data: SerializedBuffer) {
        let connection = self.find_connection_by_token(token);
        let length = data.limit();

        if length == 4 {
            connection.mark_reset();
            return;
        }

        let mark = data.position();
        let message_id = data.read_i64();
        let message_length = data.read_i32();

        if message_length != data.remaining() as i32 {
            error!("Received incorrect message length");
            return;
        }

        let svuid = data.read_i32();

        use std::collections::HashMap;
        use network::rpc;
        
        let mut funcs = HashMap::<i32, fn(conn:&mut Connection)->()>::new();

        funcs.insert(rpc::KeepAlive::SVUID, |conn: &mut Connection| {
            let keep_alive = rpc::KeepAlive{};
            conn.send_packet(keep_alive, 1);
        });

        if let Some(f) = funcs.get(&svuid) {
            f(connection);
        }
    }

    fn find_connection_by_token(&mut self, token: Token) -> &mut Connection {
        &mut self.conns[token]
    }
}