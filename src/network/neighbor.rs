use std::io::Error;

use model::config::{Configuration, ConfigurationSettings};
use network::replicator_source::ReplicatorSource;
use std::sync::{Arc, Weak, Mutex};
use std::net::SocketAddr;

pub struct Neighbor {
    source: Option<Weak<Mutex<ReplicatorSource>>>,
    sink: Option<Weak<Mutex<ReplicatorSource>>>,
    addr: SocketAddr,
}

impl Neighbor {
    pub fn from_connection(conn: &Weak<Mutex<ReplicatorSource>>) -> Self {
        let conn = conn.clone();
        let addr = conn.upgrade().unwrap().lock().unwrap().get_address().expect("invalid address");

        Neighbor {
            conn: Some(conn),
            addr
        }
    }

    pub fn from_address(addr: SocketAddr) -> Self {
        Neighbor {
            conn: None,
            addr
        }
    }

//    pub fn run(&mut self) -> Result<(), Error> {
//
//    }
}