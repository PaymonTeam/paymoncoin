use std::io::Error;

use model::config::{Configuration, ConfigurationSettings};
use network::replicator_source::ReplicatorSource;
use network::replicator_sink::ReplicatorSink;
use std::sync::{Arc, Weak, Mutex};
use mio::tcp::TcpStream;
use std::collections::VecDeque;
use network::packet;
use network::packet::{SerializedBuffer, Serializable};
use std::net::{SocketAddr, IpAddr};

pub struct Neighbor {
    pub sock: Option<Arc<Mutex<TcpStream>>>,
    pub packet_queue: VecDeque<SerializedBuffer>,
    pub addr: SocketAddr,
    pub has_sink: bool,
    pub has_source: bool,
}

impl Neighbor {
    pub fn from_connection(conn: &Arc<Mutex<TcpStream>>) -> Self {
        let conn = conn.clone();
        let addr = conn.lock().unwrap().peer_addr().expect("invalid address");

        Neighbor {
            sock: None,
            packet_queue: VecDeque::new(),
            addr,
            has_sink: false,
            has_source: true,
        }
    }

    pub fn from_address(addr: SocketAddr) -> Self {
        Neighbor {
            sock: None,
            packet_queue: VecDeque::new(),
            addr,
            has_sink: false,
            has_source: false,
        }
    }

    pub fn get_sockaddr(&self) -> SocketAddr {
        self.addr.clone()
    }

    pub fn send_packet<T>(&mut self, packet: T) where T : Serializable {
        let sb = packet::get_serialized_object(&packet, true);
        self.packet_queue.push_front(sb);
    }

    pub fn next_data(&mut self) -> Option<SerializedBuffer> {
        self.packet_queue.pop_front()
    }

//    pub fn run(&mut self) -> Result<(), Error> {
//
//    }
}