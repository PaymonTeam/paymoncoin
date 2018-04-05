use std::io::Error;

use model::config::{Configuration, ConfigurationSettings};
use std::sync::{Arc, Weak, Mutex};
use mio::tcp::TcpStream;
use std::collections::VecDeque;
use network::packet;
use network::packet::{SerializedBuffer, Serializable};
use std::net::{SocketAddr, IpAddr};
use network::replicator::Replicator;

pub struct Neighbor {
    pub addr: SocketAddr,
    pub replicator: Option<Weak<Mutex<Replicator>>>,
}

impl Neighbor {
    pub fn from_connection(conn: &Arc<Mutex<TcpStream>>) -> Self {
        let conn = conn.clone();
        let addr = conn.lock().unwrap().peer_addr().expect("invalid address");

        Neighbor {
            addr,
            replicator: None,
        }
    }

    pub fn from_address(addr: SocketAddr) -> Self {
        Neighbor {
            addr,
            replicator: None,
        }
    }

    pub fn from_replicator(replicator: Weak<Mutex<Replicator>>, addr: SocketAddr) -> Self {
        Neighbor {
            addr,
            replicator: Some(replicator),
        }
    }

    pub fn get_sockaddr(&self) -> SocketAddr {
        self.addr.clone()
    }

    pub fn send_packet<T>(&mut self, packet: T) where T : Serializable {
        let sb = packet::get_serialized_object(&packet, true);

        if let Some(ref o) = self.replicator {
            if let Some(arc) = o.upgrade() {
                if let Ok(mut replicator) = arc.lock() {
                    replicator.send_packet(packet, 0);
                }
            }
        }
    }
}