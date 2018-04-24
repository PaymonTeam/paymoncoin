extern crate byteorder;
extern crate mio;
extern crate rand;
extern crate slab;

#[macro_use] extern crate log;
extern crate env_logger;

pub mod network;
pub mod model;
pub mod storage;

use std::io::prelude::*;
use std::io::*;
use std::net::{TcpStream, SocketAddr, IpAddr};
use byteorder::{ByteOrder, BigEndian};
use network::packet::{SerializedBuffer, Serializable, get_object_size};
use network::rpc::KeepAlive;
use model::config::PORT;
use network::rpc;

struct Client {
    pub read_continuation : Option<u32>,
    stream: TcpStream
}

impl Client {
    fn connect(host: &str) -> Self {
        let host = host.parse::<IpAddr>().expect("Failed to parse host string");
        let addr = SocketAddr::new(host, PORT);
        let mut stream = TcpStream::connect(addr).expect("Couldn't not connect to neighbor"); //.unwrap();
        stream.set_nonblocking(true);

        Client {
            read_continuation: None,
            stream
        }
    }

    pub fn send_packet<T>(&mut self, packet: T, message_id : i64) where T: Serializable {
        let message_length = get_object_size(&packet);
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
        self.stream.write_all(&buffer.buffer[0..buffer.limit()]);
        buff.rewind();
        self.stream.write_all(&buff.buffer[0..buff.limit()]);
    }


    fn read_message_length(&mut self) -> Option<u32> {
        if let Some(n) = self.read_continuation {
            return Some(n);
        }

        let sock = &mut self.stream;

        let mut f_byte_buf = [0u8; 1];
        if sock.peek(&mut f_byte_buf).is_ok() {
            let f_byte = f_byte_buf[0];
            if f_byte != 0x7f {
                sock.read(&mut f_byte_buf).expect("Failed to read f_byte");
                let i = (f_byte_buf[0] as u32) * 4;
                return Some(i);
            } else {
                let mut buf = [0u8; 4];
                sock.read(&mut buf).expect("Failed to read 4-byte buf");
                let msg_len = BigEndian::read_u32(buf.as_ref());
                return Some((msg_len >> 8) * 4);
            }
        } else {
            return None;
        }
    }

    fn read(&mut self) -> Option<SerializedBuffer> {
        let msg_len = match self.read_message_length() {
            None => { return None; },
            Some(n) => n,
        };

        if msg_len == 0 {
            return None;
        }

        let msg_len = msg_len as usize;

        let mut recv_buf : Vec<u8> = Vec::with_capacity(msg_len);
        unsafe { recv_buf.set_len(msg_len); }

        let sock_ref = <TcpStream as Read>::by_ref(&mut self.stream);
        match sock_ref.take(msg_len as u64).read(&mut recv_buf) {
            Ok(n) => {
                if n < msg_len as usize {
                    println!("Did not read enough bytes");
                    return None;
                }

                self.read_continuation = None;
                Some(SerializedBuffer::from_slice(&recv_buf))
            }
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    self.read_continuation = Some(msg_len as u32);
                } else {
                    error!("Failed to read buffer, error: {}", e);
                }
                None
            }
        }
    }

    fn tick(&mut self) {
        let mut packets_data = Vec::<SerializedBuffer>::new();

        while let Some(buff) = self.read() {
            println!("Got buffer len={}", buff.limit());
            packets_data.push(buff);
        }

        for data in packets_data {
            self.on_data_received(data);
        }
    }

    fn on_data_received(&mut self, mut data: SerializedBuffer) {
        let length = data.limit();

        if length == 4 {
            error!("got data len 4");
            return;
        }

        let mark = data.position();
        let message_id = data.read_i64();
        let message_length = data.read_i32();

        if message_length != data.remaining() as i32 {
            error!("received incorrect message length");
            return;
        }

        let svuid = data.read_i32();

        if svuid == rpc::KeepAlive::SVUID {
            let keep_alive = rpc::KeepAlive{};
            self.send_packet(keep_alive, 1);
        }
    }
}

fn main() {
    let mut cl = Client::connect("127.0.0.1");

    let packet = KeepAlive {};
    cl.send_packet(packet, 1337);

    loop {
        cl.tick();
    };
}
