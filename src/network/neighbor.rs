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
        let sb = packet::get_serialized_object(&packet, true);

        if let Some(ref o) = self.source {
//            if let Some(arc) = o.upgrade() {
//                if let Ok(mut replicator) = arc.lock() {
//                    o.send_packet(packet, 0);
//                }
//            }
        }
    }
}