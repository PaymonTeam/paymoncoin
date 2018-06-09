use std::io;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::rc::Rc;
use std::sync::Arc;

use byteorder::{ByteOrder, BigEndian};

use mio::{Poll, PollOpt, Ready, Token};
use mio::net::TcpStream;
use network::packet::{SerializedBuffer, Serializable, calculate_object_size};
use std::collections::VecDeque;
use std::net::SocketAddr;

pub struct Replicator {
    sock: TcpStream,
    pub token: Token,
    interest: Ready,
    send_queue: VecDeque<Arc<SerializedBuffer>>,
    is_idle: bool,
    is_reset: bool,
    read_continuation: Option<u32>,
    write_continuation: bool,
    pub rest_of_the_data: Option<Arc<SerializedBuffer>>,
    pub last_packet_length: usize
}

#[cfg(any(target_os = "dragonfly",
target_os = "freebsd", target_os = "ios", target_os = "macos",target_os="linux"))]
use mio::unix::UnixReady;

impl Replicator {
    #[cfg(any(target_os = "dragonfly",
    target_os = "freebsd", target_os = "ios", target_os = "macos",target_os="linux"))]
    pub fn new(sock: TcpStream, token: Token) -> Replicator {
        Replicator {
            sock,
            token,
            interest: Ready::from(UnixReady::hup()),
            send_queue: VecDeque::new(),
            is_idle: true,
            is_reset: false,
            read_continuation: None,
            write_continuation: false,
            rest_of_the_data: None,
            last_packet_length: 0,
        }
    }

    #[cfg(any(target_os = "windows"))]
    pub fn new(sock: TcpStream, token: Token) -> Replicator {
        Replicator {
            sock,
            token,
            interest: Ready::from(Ready::hup()),
            send_queue: VecDeque::new(),
            is_idle: true,
            is_reset: false,
            read_continuation: None,
            write_continuation: false,
            last_packet_length: 0,
            rest_of_the_data: None
        }
    }

    pub fn get_address(&self) -> io::Result<SocketAddr> {
        self.sock.peer_addr()
    }

    pub fn readable(&mut self) -> io::Result<Option<SerializedBuffer>> {
        let msg_len = match self.read_message_length()? {
            None => { return Ok(None); },
            Some(n) => n,
        };

        if msg_len == 0 {
            return Ok(None);
        }

        let msg_len = msg_len as usize;
        debug!("Expected message length is {}", msg_len);

        let mut recv_buf : Vec<u8> = Vec::with_capacity(msg_len);
        unsafe { recv_buf.set_len(msg_len); }
//
//        // UFCS: resolve "multiple applicable items in scope [E0034]" error
        let sock_ref = <TcpStream as Read>::by_ref(&mut self.sock);
        match sock_ref.take(msg_len as u64).read(&mut recv_buf) {
            Ok(n) => {
                if n < msg_len as usize {
                    return Err(Error::new(ErrorKind::InvalidData, "Did not read enough bytes"));
                }

                self.read_continuation = None;

                Ok(Some(SerializedBuffer::from_slice(&recv_buf)))
            }
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    self.read_continuation = Some(msg_len as u32);
                    Ok(None)
                } else {
                    error!("Failed to read buffer for token {:?}, error: {}", self.token, e);
                    Err(e)
                }
            }
        }
    }

    fn read_message_length(&mut self) -> io::Result<Option<u32>> {
        if let Some(n) = self.read_continuation {
            return Ok(Some(n));
        }

        let mut f_byte_buf = [0u8; 1];
        if self.sock.peek(&mut f_byte_buf).is_ok() {
            let f_byte = f_byte_buf[0];
            if f_byte != 0x7f {
                self.sock.read(&mut f_byte_buf).expect("Failed to read f_byte");
                let i = (f_byte_buf[0] as u32) * 4;
                return Ok(Some(i));
            } else {
                let mut buf = [0u8; 4];
                self.sock.read(&mut buf).expect("Failed to read 4-byte buf");
                let msg_len = BigEndian::read_u32(buf.as_ref());
                return Ok(Some((msg_len >> 8) * 4));
            }
        } else {
            return Ok(None);
        }
    }

    pub fn send_packet<T>(&mut self, packet: T, message_id : i64) where T: Serializable {
        let message_length = calculate_object_size(&packet);
        let size = 8 + 4 + message_length as usize;
        let mut buffer = SerializedBuffer::new_with_size(size);
        buffer.set_position(0);
        buffer.write_i64(message_id);
        buffer.write_i32(message_length as i32);
        packet.serialize_to_stream(&mut buffer);
        self.send_serialized_data(buffer);
    }

    pub fn send_serialized_data(&mut self, mut buff: SerializedBuffer) {
        buff.rewind();

        let mut buffer_len = 0;
        let mut packet_length = (buff.limit() / 4) as i32;

        if packet_length < 0x7f {
            buffer_len += 1;
        } else {
            buffer_len += 4;
        }

        let mut buffer = SerializedBuffer::new_with_size(buffer_len);
        if packet_length < 0x7f {
            buffer.write_byte(packet_length as u8);
        } else {
            packet_length = (packet_length << 8) + 0x7f;
            buffer.write_i32(packet_length);
        }

        buffer.rewind();
        buff.rewind();

        self.send_queue.push_back(Arc::new(buffer));
        self.send_queue.push_back(Arc::new(buff));

        if !self.interest.is_writable() {
            self.interest.insert(Ready::writable());
        }
    }

    pub fn writable(&mut self) -> io::Result<()> {
        self.send_queue.pop_front()
            .ok_or(Error::new(ErrorKind::Other, "Could not pop send queue"))
            .and_then(|buf| {
//                match self.write_message_length(&buf) {
//                    Ok(None) => {
//                        self.send_queue.push(buf);
//                        return Ok(());
//                    },
//                    Ok(Some(())) => {
//                        ()
//                    },
//                    Err(e) => {
//                        error!("Failed to send buffer for {:?}, error: {}", self.token, e);
//                        return Err(e);
//                    }
//                }

                let lim = buf.limit();
                match self.sock.write(&(*buf).buffer[0..lim]) {
                    Ok(n) => {
                        debug!("CONN : we wrote {} bytes", n);
                        self.write_continuation = false;
                        Ok(())
                    },
                    Err(e) => {
                        if e.kind() == ErrorKind::WouldBlock {
                            debug!("client flushing buf; WouldBlock");

                            self.send_queue.push_front(buf);
                            self.write_continuation = true;
                            Ok(())
                        } else {
                            error!("Failed to send buffer for {:?}, error: {}", self.token, e);
                            Err(e)
                        }
                    }
                }
            })?;

        if self.send_queue.is_empty() {
            self.interest.remove(Ready::writable());
        }

        Ok(())
    }

    fn write_message_length(&mut self, buf: &Rc<Vec<u8>>) -> io::Result<Option<()>> {
        if self.write_continuation {
            return Ok(Some(()));
        }

        let len = buf.len();
        let mut send_buf = [0u8; 8];
        BigEndian::write_u64(&mut send_buf, len as u64);

        match self.sock.write(&send_buf) {
            Ok(n) => {
                debug!("Sent message length of {} bytes", n);
                Ok(Some(()))
            }
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    debug!("client flushing buf; WouldBlock");

                    Ok(None)
                } else {
                    error!("Failed to send buffer for {:?}, error: {}", self.token, e);
                    Err(e)
                }
            }
        }
    }

    pub fn register(&mut self, poll: &Poll) -> io::Result<()> {
        trace!("connection register; token={:?}", self.token);

        self.interest.insert(Ready::readable());

        poll.register(
            &self.sock,
            self.token,
            self.interest,
            PollOpt::edge() | PollOpt::oneshot()
        ).and_then(|(),| {
            self.is_idle = false;
            Ok(())
        }).or_else(|e| {
            error!("Failed to reregister {:?}, {:?}", self.token, e);
            Err(e)
        })
    }

    pub fn reregister(&mut self, poll: &mut Poll) -> io::Result<()> {
        trace!("connection reregister; token={:?}", self.token);

        poll.reregister(
            &self.sock,
            self.token,
            self.interest,
            PollOpt::edge() | PollOpt::oneshot()
        ).and_then(|(),| {
            self.is_idle = false;
            Ok(())
        }).or_else(|e| {
            error!("Failed to reregister {:?}, {:?}", self.token, e);
            Err(e)
        })
    }

    pub fn mark_reset(&mut self) {
        trace!("connection mark_reset; token={:?}", self.token);

        self.is_reset = true;
    }

    #[inline]
    pub fn is_reset(&self) -> bool {
        self.is_reset
    }

    pub fn mark_idle(&mut self) {
        trace!("connection mark_idle; token={:?}", self.token);

        self.is_idle = true;
    }

    #[inline]
    pub fn is_idle(&self) -> bool {
        self.is_idle
    }
}
