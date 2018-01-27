extern crate nix;

use std::io::{self, ErrorKind};
use std::rc::Rc;

use mio::{Events, Poll, PollOpt, Ready, Token};
use mio::net::TcpListener;
use mio::unix::UnixReady;
use slab;

use network::connection::Connection;
use self::nix::sys::signal;

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
            // Give our server token a number much larger than our slab capacity. The slab used to
            // track an internal offset, but does not anymore.
            token: Token(10_000_000),

            // SERVER is Token(1), so start after that
            // we can deal with a max of 126 connections
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

            let mut i = 0;

            trace!("processing events... cnt={}; len={}", cnt, self.events.len());

            while i < cnt {
                // TODO this would be nice if it would turn a Result type. trying to convert this
                // into a io::Result runs into a problem because .ok_or() expects std::Result and
                // not io::Result
                let event = self.events.get(i).expect("Failed to get event");

                trace!("event={:?}; idx={:?}", event, i);
                self.ready(poll, event.token(), event.readiness());

                i += 1;
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

    fn ready(&mut self, poll: &mut Poll, token: Token, event: Ready) {
        debug!("{:?} event = {:?}", token, event);

        let event = UnixReady::from(event);

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

        // We never expect a write event for our `Server` token . A write event for any other token
        // should be handed off to that connection.
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

        // A read event for our `Server` token means we are establishing a new connection. A read
        // event for any other token should be handed off to that connection.
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
        debug!("server conn readable; token={:?}", token);

        while let Some(message) = self.find_connection_by_token(token).readable()? {
            let rc_message = Rc::new(message);

            for c in self.conns.iter_mut() {
                c.send_message(rc_message.clone()).unwrap_or_else(|e| {
                    error!("Failed to queue message for {:?}: {:?}", c.token, e);
                    c.mark_reset();
                });
            }
        }

        Ok(())
    }

    /// Find a connection in the slab using the given token.
    fn find_connection_by_token(&mut self, token: Token) -> &mut Connection {
        &mut self.conns[token]
    }
}