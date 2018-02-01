extern crate byteorder;
extern crate mio;
extern crate slab;

#[macro_use] extern crate log;
extern crate env_logger;

mod network;
mod model;
mod storage;

use std::net::{SocketAddr, IpAddr};

use mio::Poll;
use mio::net::TcpListener;

use network::neighbor::*;
use network::packet::SerializedBuffer;
use network::rpc::KeepAlive;
use model::config::PORT;
use model::transaction::Transaction;
use storage::hive::Hive;

fn main() {
//    let mut ad = [0u8; 21];
    let mut seed = [0u8; 32];
    seed[1] = 1;
    let (addr_bytes, addr_str) = Hive::generate_address(&seed);
    println!("{}=={} {:?}=={:?}", addr_str.clone(), Transaction::address_to_string(addr_bytes), addr_bytes, Transaction::address_to_bytes(addr_str).unwrap());

    let ka = KeepAlive::SVUID;
    let mut sb = SerializedBuffer::new_with_size(8);
    sb.write_i32(42);
    sb.set_position(0);
    println!("i={}", sb.read_i32());

    env_logger::init().expect("Failed to init logger");
    let host = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse host string");
    let addr = SocketAddr::new(host, PORT);
    let sock = TcpListener::bind(&addr).expect("Failed to bind address");

    let mut poll = Poll::new().expect("Failed to create Poll");
    let mut server = Neighbor::new(sock);
    server.run(&mut poll).expect("Failed to run server");
}
