extern crate byteorder;
extern crate mio;
extern crate rand;
extern crate slab;

#[macro_use] extern crate log;
extern crate env_logger;

pub mod network;
pub mod model;
pub mod storage;

use std::net::{SocketAddr, IpAddr};

use mio::Poll;
use mio::net::{TcpListener, TcpStream};

use network::neighbor::*;
use network::connection::*;
use network::packet::SerializedBuffer;
use network::rpc::KeepAlive;
use model::config::PORT;
use model::transaction::Transaction;
use storage::hive::Hive;
use network::packet::{Packet, get_object_size};

type Slab<T> = slab::Slab<T, usize>;

fn main() {
    env_logger::init().expect("Failed to init logger");

    let host = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse host string");
    let addr = SocketAddr::new(host, PORT);
    let sock = TcpListener::bind(&addr).expect("Failed to bind address");

    let mut poll = Poll::new().expect("Failed to create Poll");
    let mut server = Neighbor::new(sock);
    server.run(&mut poll).expect("Failed to run server");
}
