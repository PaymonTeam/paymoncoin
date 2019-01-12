use std::io;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::rc::Rc;
use std::sync::Arc;

use byteorder::{ByteOrder, NativeEndian, BigEndian, LittleEndian};

use mio::{Poll, PollOpt, Ready, Token};
use mio::net::TcpStream;
use serde::{Serialize, Deserialize};
use serde_pm::{SerializedBuffer, to_buffer};
use std::collections::VecDeque;
use std::net::SocketAddr;

#[cfg(any(target_os = "dragonfly",
target_os = "freebsd", target_os = "ios", target_os = "macos",target_os="linux"))]
use mio::unix::UnixReady;

pub struct ReplicatorSource {
    sock: TcpStream,
    pub token: Token,
    pub interest: Ready,
    pub send_queue: VecDeque<Arc<SerializedBuffer>>,
    is_idle: bool,
    is_reset: bool,
    read_continuation: Option<u32>,
    write_continuation: bool,
    pub rest_of_the_data: Option<Arc<SerializedBuffer>>,
    pub last_packet_length: usize
}

impl ReplicatorSource {
    #[cfg(any(target_os = "dragonfly",
    target_os = "freebsd", target_os = "ios", target_os = "macos",target_os="linux"))]
    pub fn new(sock: TcpStream, token: Token) -> ReplicatorSource {
        ReplicatorSource {
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
    pub fn new(sock: TcpStream, token: Token) -> ReplicatorSource {
        ReplicatorSource {
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
            None => {
                return Err(Error::new(ErrorKind::ConnectionReset, "Read None bytes"));
//                return Ok(None);
            },
            Some(n) => n,
        };

        if msg_len == 0 {
//            return Err(Error::new(ErrorKind::ConnectionReset, "Read 0 bytes"));
            return Ok(None);
        }

        let msg_len = msg_len as usize;
        debug!("Expected message length is {}", msg_len);

        let mut recv_buf : Vec<u8> = Vec::with_capacity(msg_len);
        unsafe { recv_buf.set_len(msg_len); }
//        let mut recv_buf = [0u8; msg_len];
//        // UFCS: resolve "multiple applicable items in scope [E0034]" error
        let sock_ref = <TcpStream as Read>::by_ref(&mut self.sock);
        match sock_ref.read_exact(&mut recv_buf) {//take(msg_len as u64).read(&mut recv_buf) {
            Ok(()) => {
//                debug!("n={}", n);
//                if n < msg_len as usize {
//                    return Err(Error::new(ErrorKind::InvalidData, "Did not read enough bytes"));
//                }

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
                let msg_len = NativeEndian::read_u32(buf.as_ref());
                return Ok(Some((msg_len >> 8) * 4));
            }
        } else {
            return Ok(None);
        }
    }

    pub fn send_packet<T>(&mut self, packet: T, message_id: i64) where T: Serializable {
        let message_length = calculate_object_size(&packet);
        let size = match message_length % 4 == 0 {
            true => 8 + 4 + message_length as usize,
            false => {
                let additional = 4 - (message_length % 4) as usize;
                8 + 4 + message_length as usize + additional
            }
        };
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
//        self.reregister()
//        self.writable();
    }

    pub fn writable(&mut self) -> io::Result<()> {
//        self.send_queue.pop_front()
//            .ok_or(Error::new(ErrorKind::Other, "Could not pop send queue"))
//            .and_then(|buf| {
////                match self.write_message_length(&buf) {
////                    Ok(None) => {
////                        self.send_queue.push(buf);
////                        return Ok(());
////                    },
////                    Ok(Some(())) => {
////                        ()
////                    },
////                    Err(e) => {
////                        error!("Failed to send buffer for {:?}, error: {}", self.token, e);
////                        return Err(e);
////                    }
////                }
//
//                let lim = buf.limit();
//                match self.sock.write(&(*buf).buffer[0..lim]) {
//                    Ok(n) => {
//                        debug!("CONN : we wrote {} bytes", n);
//                        self.write_continuation = false;
//                        Ok(())
//                    },
//                    Err(e) => {
//                        if e.kind() == ErrorKind::WouldBlock {
//                            debug!("client flushing buf; WouldBlock");
//
//                            self.send_queue.push_front(buf);
//                            self.write_continuation = true;
//                            Ok(())
//                        } else {
//                            error!("Failed to send buffer for {:?}, error: {}", self.token, e);
//                            Err(e)
//                        }
//                    }
//                }
//            })?;

        while let Some(buf) = self.send_queue.pop_front() {
            let lim = buf.limit();
            match self.sock.write(&buf.buffer[0..lim]) {
                Ok(n) => {
                    debug!("CONN : we wrote {} bytes", n);
                    self.write_continuation = false;
//                    Ok(())
                },
                Err(e) => {
                    if e.kind() == ErrorKind::WouldBlock {
                        debug!("client flushing buf; WouldBlock");

                        self.send_queue.push_front(buf);
                        self.write_continuation = true;
//                        Ok(())
                    } else {
                        error!("Failed to send buffer for {:?}, error: {}", self.token, e);
//                        Err(e)
                    }
                }
            };
        }

        if self.send_queue.is_empty() {
            self.interest.remove(Ready::writable());
            self.is_idle = false;
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
        debug!("connection register; token={:?}", self.token);

        self.interest.insert(Ready::readable());

        poll.register(
            &self.sock,
            self.token,
            self.interest,
            PollOpt::edge() | PollOpt::oneshot()
        ).and_then(|(),| {
            self.is_idle = true;
            Ok(())
        }).or_else(|e| {
            error!("Failed to reregister {:?}, {:?}", self.token, e);
            Err(e)
        })
    }

    pub fn reregister(&mut self, poll: &mut Poll) -> io::Result<()> {
        debug!("connection reregister; token={:?}", self.token);
        poll.reregister(
            &self.sock,
            self.token,
            self.interest, //Ready::readable(), //Ready::empty(),
            PollOpt::edge() | PollOpt::oneshot()
        ).and_then(|(),| {
            self.is_idle = true;
            Ok(())
        }).or_else(|e| {
            error!("Failed to reregister {:?}, {:?}", self.token, e);
            Err(e)
        })
    }

    pub fn mark_reset(&mut self) {
        debug!("connection mark_reset; token={:?}", self.token);

        self.is_reset = true;
    }

    #[inline]
    pub fn is_reset(&self) -> bool {
        self.is_reset
    }

    pub fn mark_idle(&mut self) {
        debug!("connection mark_idle; token={:?}", self.token);

        self.is_idle = true;
    }

    #[inline]
    pub fn is_idle(&self) -> bool {
        self.is_idle
    }
}

pub struct ReplicatorSink {
    sock: TcpStream,
    pub token: Token,
    interest: Ready,
    shutdown: bool,
}

impl ReplicatorSink {
    fn new(sock: TcpStream, token: Token) -> Self {

        ReplicatorSink {
            sock,
            token,
            interest: Ready::empty(),
            shutdown: false,
        }
    }

//    fn readable(&mut self, poll: &mut Poll) -> io::Result<()> {
//        debug!("client socket readable");
//
//        let mut buf = self.mut_buf.take().unwrap();
//
//        match self.sock.try_read_buf(&mut buf) {
//            Ok(None) => {
//                debug!("CLIENT : spurious read wakeup");
//                self.mut_buf = Some(buf);
//            }
//            Ok(Some(r)) => {
//                debug!("CLIENT : We read {} bytes!", r);
//
//                // prepare for reading
//                let mut buf = buf.flip();
//
//                while buf.has_remaining() {
//                    let actual = buf.read_byte().unwrap();
//                    let expect = self.rx.read_byte().unwrap();
//
//                    assert!(actual == expect, "actual={}; expect={}", actual, expect);
//                }
//
//                self.mut_buf = Some(buf.flip());
//
//                self.interest.remove(Ready::readable());
//
//                if !self.rx.has_remaining() {
//                    self.next_msg(poll).unwrap();
//                }
//            }
//            Err(e) => {
//                panic!("not implemented; client err={:?}", e);
//            }
//        };
//
//        if !self.interest.is_empty() {
//            assert!(self.interest.is_readable() || self.interest.is_writable(), "actual={:?}", self.interest);
//            poll.reregister(&self.sock, self.token, self.interest,
//                            PollOpt::edge() | PollOpt::oneshot())?;
//        }
//
//        Ok(())
//    }

//    fn writable(&mut self, poll: &mut Poll) -> io::Result<()> {
//        debug!("client socket writable");
//
//        match self.sock.try_write_buf(&mut self.tx) {
//            Ok(None) => {
//                debug!("client flushing buf; WOULDBLOCK");
//                self.interest.insert(Ready::writable());
//            }
//            Ok(Some(r)) => {
//                debug!("CLIENT : we wrote {} bytes!", r);
//                self.interest.insert(Ready::readable());
//                self.interest.remove(Ready::writable());
//            }
//            Err(e) => debug!("not implemented; client err={:?}", e)
//        }
//
//        if self.interest.is_readable() || self.interest.is_writable() {
//            poll.reregister(&self.sock, self.token, self.interest,PollOpt::edge() | PollOpt::oneshot())?;
//        }
//
//        Ok(())
//    }

//    fn next_msg(&mut self, poll: &mut Poll) -> io::Result<()> {
//        if self.msgs.is_empty() {
//            self.shutdown = true;
//            return Ok(());
//        }
//
//        let curr = self.msgs.remove(0);
//
//        debug!("client prepping next message");
//        self.tx = SliceBuf::wrap(curr.as_bytes());
//        self.rx = SliceBuf::wrap(curr.as_bytes());
//
//        self.interest.insert(Ready::writable());
//        poll.reregister(&self.sock, self.token, self.interest,
//                        PollOpt::edge() | PollOpt::oneshot())
//    }
}