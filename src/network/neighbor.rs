extern crate nix;

use std::io::{self, ErrorKind};
use std::rc::{Rc};
use std::sync::Arc;

use mio::{Events, Poll, PollOpt, Ready, Token};
use mio::net::TcpListener;
#[cfg(any(target_os = "dragonfly",
target_os = "freebsd", target_os = "ios", target_os = "macos"))]
use mio::unix::UnixReady;

#[cfg(any(target_os = "windows"))]
use mio::windows::{Binding,Overlapped};

use slab;

use network::connection::Connection;
use network::packet::{SerializedBuffer, Packet};
//use self::nix::sys::signal;

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
            token: Token(10_000_000),
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

//    #[cfg(any(target_os = "dragonfly",
//    target_os = "freebsd", target_os = "ios", target_os = "macos"))]
    fn ready(&mut self, poll: &mut Poll, token: Token, event: Ready) {
        debug!("{:?} event = {:?}", token, event);
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
        debug!("server conn readable; token={:?}", token);

//      for c in self.conns.iter_mut() {
//          c.send_message(rc_message.clone()).unwrap_or_else(|e| {
//              error!("Failed to queue message for {:?}: {:?}", c.token, e);
//              c.mark_reset();
//          });
//      }
        let mut packets_data = Vec::<SerializedBuffer>::new();

        {
            let connection = self.find_connection_by_token(token);
            while let Some(buffer) = connection.readable()? {
//                let mut rc_message = Rc::new(buffer);
                packets_data.push(buffer);
//            self.on_receive_data(connection, rc_message);
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
            // TODO: close connection
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
            conn.send_message(Rc::new(vec![1u8,2,3,4,5,6,7,8]));
        });

        if let Some(f) = funcs.get(&svuid) {
            f(connection);
        }
    }

    #[deprecated(since="0.1.0")]
    pub fn on_receive_data(&self, mut client: &mut Connection, mut buffer: Rc<SerializedBuffer>) {
        unsafe {
            let mut parse_later_buffer : Option<Rc<SerializedBuffer>> = None;

            if let Some(ref mut rest_of_the_data) = client.rest_of_the_data {
                let mut rotd = Rc::clone(rest_of_the_data);
                let rotd_ptr = Rc::into_raw(rotd) as *mut SerializedBuffer;
                rotd = Rc::from_raw(rotd_ptr);
                if client.last_packet_length == 0 {
                    let b_ptr = Rc::into_raw(buffer) as *mut SerializedBuffer;
                    buffer = Rc::from_raw(b_ptr);

                    if rest_of_the_data.capacity() - rest_of_the_data.position() >= buffer.limit() {
                        let lim = rest_of_the_data.position() + buffer.limit();

                        (*rotd_ptr).set_limit(lim);
                        (*rotd_ptr).write_bytes_serialized_buffer(Rc::get_mut(&mut buffer).expect("Get mut 4"));

                        buffer = Rc::clone(&rotd);
                    } else {
                        // TODO: optimize?
                        let mut new_buffer = SerializedBuffer::new_with_size(rest_of_the_data.limit() + buffer.limit());
                        (*rotd_ptr).rewind();
                        new_buffer.write_bytes_serialized_buffer(&mut (*rotd_ptr));
                        new_buffer.write_bytes_serialized_buffer(Rc::get_mut(&mut buffer).expect("Failed to get mut"));
                        buffer = Rc::new(new_buffer);
                        (*rotd_ptr).reuse();
                        *rest_of_the_data = Rc::clone(&buffer);
                    }
                } else {
                    let mut len = 0;
                    if client.last_packet_length - rest_of_the_data.position() <= buffer.limit() {
                        len = client.last_packet_length - rest_of_the_data.position();
                    } else {
                        len = buffer.limit();
                    }
                    let old_limit = buffer.limit();
                    let b_ptr = Rc::into_raw(buffer) as *mut SerializedBuffer;
                    buffer = Rc::from_raw(b_ptr);

                    Rc::get_mut(&mut buffer).expect("Get mut 5").set_limit(len);
                    (*rotd_ptr).write_bytes_serialized_buffer(&mut (*b_ptr));
                    (*b_ptr).set_limit(old_limit);
                    if rest_of_the_data.position() == client.last_packet_length {
                        if buffer.has_remaining() {
                            parse_later_buffer = Some(Rc::clone(&buffer));
                        } else {
                            parse_later_buffer = None;
                        }
                        buffer = Rc::clone(&rest_of_the_data);
                    } else {
                        return;
                    }
                }
            }

            while buffer.has_remaining() {
                let b_ptr = Rc::into_raw(buffer) as *mut SerializedBuffer;
                buffer = Rc::from_raw(b_ptr);

                let mut current_packet_length = 0;
                let mark = buffer.position();
                let f_byte = (*b_ptr).read_byte();

                if f_byte != 0x7f {
                    current_packet_length = (f_byte as u32) * 4;
                } else {
                    (*b_ptr).set_position(mark);
                    if buffer.remaining() < 4 {
                        match client.rest_of_the_data {
                            Some(ref mut rest_of_the_data) => {
                                let mut rotd = Rc::clone(rest_of_the_data);
                                let rotd_ptr = Rc::into_raw(rotd) as *mut SerializedBuffer;
                                rotd = Rc::from_raw(rotd_ptr);

                                if rest_of_the_data.position() != 0 {
                                    let mut tmp = SerializedBuffer::new_with_size(16384);
                                    tmp.write_bytes_serialized_buffer(&mut (*b_ptr));
                                    let pos = tmp.position();
                                    tmp.set_limit(pos);
                                    *rest_of_the_data = Rc::new(tmp);
                                    client.last_packet_length = 0;
                                } else {
                                    (*rotd_ptr).set_position(rest_of_the_data.limit());
                                }
                            }
                            None => {
                                let mut buf = SerializedBuffer::new_with_size(16384);
                                buf.write_bytes_serialized_buffer(&mut (*b_ptr));
                                let pos = buf.position();
                                buf.set_limit(pos);

                                client.rest_of_the_data = Some(Rc::new(buf));
                                client.last_packet_length = 0;
                            }
                        }
                        break;
                    }
                    current_packet_length = ((*b_ptr).read_u32() >> 8) * 4;
                }

                if current_packet_length % 4 != 0 || current_packet_length > 2 * 1024 * 1024 {
                    error!("received invalid packet length");
                    return;
                }

                if current_packet_length < buffer.remaining() as u32 {
                    info!("received message len < packet len ({}<{})", current_packet_length, buffer.remaining());
                } else if current_packet_length == buffer.remaining() as u32 {
                    info!("received message len == packet len ({})", current_packet_length);
                } else {
                    info!("received message len > packet len ({}>{})", current_packet_length, buffer.remaining());
                    let len = (current_packet_length + (if f_byte != 0x7f { 1 } else { 4 })) as usize;
                    let mut flag = false;
                    if let Some(ref mut rest_of_the_data) = client.rest_of_the_data {
                        if rest_of_the_data.capacity() < len {
                                flag = true; // -> client.rest_of_the_data = None;
                        }
                    }
                    if flag {
                        client.rest_of_the_data = None;
                    }

                    match client.rest_of_the_data {
                        None => {
                            (*b_ptr).set_position(mark);
                            let mut tmp = SerializedBuffer::new_with_size(len);
                            tmp.write_bytes_serialized_buffer(&mut (*b_ptr));
                            client.rest_of_the_data = Some(Rc::new(tmp));
                        }
                        Some(ref mut rest_of_the_data) => {
                            let mut rotd = Rc::clone(rest_of_the_data);
                            let rotd_ptr = Rc::into_raw(rotd) as *mut SerializedBuffer;
                            rotd = Rc::from_raw(rotd_ptr);

                            let lim = rest_of_the_data.limit();
                            (*rotd_ptr).set_position(lim);
                            (*rotd_ptr).set_limit(len);
                        }
                    }
                    client.last_packet_length = len;
                    return;
                }

                let old = buffer.limit();
                let pos = buffer.position();
                (*b_ptr).set_limit(pos + current_packet_length as usize);

//                self.on_connection_data_received(client, &mut buffer, current_packet_length);

                let lim = buffer.limit();
                (*b_ptr).set_position(lim);
                (*b_ptr).set_limit(old);

                let mut flag = false;
                if let Some(ref mut rest_of_the_data) = client.rest_of_the_data {
                    let mut rotd = Rc::clone(rest_of_the_data);
                    let rotd_ptr = Rc::into_raw(rotd) as *mut SerializedBuffer;
                    rotd = Rc::from_raw(rotd_ptr);

                    if (client.last_packet_length != 0 && rest_of_the_data.position() == client.last_packet_length)
                        || (client.last_packet_length == 0 && !rest_of_the_data.has_remaining()) {

                        rest_of_the_data.reuse();
                        flag = true; // -> client.rest_of_the_data = None
                    } else {
                        (*rotd_ptr).compact();
                        let lim = rest_of_the_data.position();
                        (*rotd_ptr).set_limit(lim);
                        (*rotd_ptr).set_position(0);
                    }
                }
                if flag {
                    client.rest_of_the_data = None;
                }

                if let Some(parseLaterBuffer) = parse_later_buffer {
                    buffer = Rc::clone(&parseLaterBuffer);
                    parse_later_buffer = None;
                }
            }
        }
    }

    fn find_connection_by_token(&mut self, token: Token) -> &mut Connection {
        &mut self.conns[token]
    }
}