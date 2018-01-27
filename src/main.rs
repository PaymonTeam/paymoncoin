extern crate byteorder;
extern crate mio;
extern crate slab;

#[macro_use] extern crate log;
extern crate env_logger;

mod network;
mod model;
mod storage;

use std::net::SocketAddr;

use mio::Poll;
use mio::net::TcpListener;

use network::neighbor::*;

fn main() {
    env_logger::init().expect("Failed to init logger");

    let addr = "127.0.0.1:8000".parse::<SocketAddr>().expect("Failed to parse host:port string");
    let sock = TcpListener::bind(&addr).expect("Failed to bind address");

    let mut poll = Poll::new().expect("Failed to create Poll");
    let mut server = Neighbor::new(sock);
    server.run(&mut poll).expect("Failed to run server");

}
