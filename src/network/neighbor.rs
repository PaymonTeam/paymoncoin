extern crate futures;

use std::io::Error;

use model::config::{Configuration, ConfigurationSettings};
use std::sync::{Arc, Weak, Mutex};
use mio::tcp::TcpStream;
use std::collections::VecDeque;
use network::packet;
use network::packet::{SerializedBuffer, Serializable};
use std::net::{SocketAddr, IpAddr};
//use network::replicator::*;
use network::replicator_new::*;

pub struct Neighbor {
    pub addr: SocketAddr,
//    pub replicator_source: Option<Weak<Mutex<ReplicatorSource>>>,
//    pub replicator_sink: Option<Weak<Mutex<ReplicatorSource>>>,
    pub sink: Option<ReplicatorSink>,
    pub source: Option<ReplicatorSource>,
    pub connecting: bool,
}

impl Neighbor {
    pub fn from_connection(conn: &Arc<Mutex<TcpStream>>) -> Self {
        let conn = conn.clone();
        let addr = conn.lock().unwrap().peer_addr().expect("invalid address");

        Neighbor {
            addr,
            source: None,
            sink: None,
            connecting: false,
        }
    }

    pub fn from_address(addr: SocketAddr) -> Self {
        Neighbor {
            addr,
            source: None,
            sink: None,
            connecting: false,
        }
    }

    pub fn from_replicator_source(replicator: /*Weak<Mutex<*/ReplicatorSource/*>>*/, addr: SocketAddr) -> Self {
        Neighbor {
            addr,
            source: Some(replicator),
            sink: None,
            connecting: false,
        }
    }

    pub fn from_replicator_sink(replicator: /*Weak<Mutex<*/ReplicatorSink/*>>*/, addr: SocketAddr)
        -> Self {
        Neighbor {
            addr,
            source: None,
            sink: Some(replicator),
            connecting: false,
        }
    }

    pub fn get_sockaddr(&self) -> SocketAddr {
        self.addr.clone()
    }

    pub fn send_packet<T>(&mut self, packet: T) where T : Serializable {
        use network::packet::{SerializedBuffer, Serializable, calculate_object_size};
        use self::futures::{Sink, Future};
        let message_id = 0;

        let sb = packet::get_serialized_object(&packet, true);

        if let Some(ref mut o) = self.sink {
            let message_length = calculate_object_size(&packet);
            let size = match message_length % 4 == 0 {
                true => 8 + 4 + message_length as usize,
                false => {
                    let additional = 4 - (message_length % 4) as usize;
                    8 + 4 + message_length as usize + additional
                }
            };

            let mut buff = SerializedBuffer::new_with_size(size);
            buff.set_position(0);
            buff.write_i64(message_id);
            buff.write_i32(message_length as i32);
            packet.serialize_to_stream(&mut buff);

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

            o.tx.clone()
                .send((&buffer).to_vec()).wait().unwrap()
                .send((&buff).to_vec()).wait().unwrap()
                .flush().wait().unwrap();

//            if let Some(arc) = o.upgrade() {
//                if let Ok(mut replicator) = arc.lock() {
//                    o.send_packet(packet, 0);
//                }
//            }
        }
    }
}