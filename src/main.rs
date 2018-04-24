//#![feature(collection_placement)]
//#![feature(placement_in_syntax)]

extern crate byteorder;
extern crate mio;
extern crate rand;
extern crate slab;
//extern crate secp256k1;
extern crate ethcore_bigint as bigint;
extern crate memorydb;
extern crate patricia_trie;

#[macro_use] extern crate log;
extern crate env_logger;

pub mod network;
pub mod model;
pub mod storage;

use std::net::{SocketAddr, IpAddr};

use mio::Poll;
use mio::net::{TcpListener, TcpStream};

use network::node::*;
use network::connection::*;
use network::packet::SerializedBuffer;
use model::config::PORT;
use model::transaction::{TransactionObject, Transaction};
use storage::hive::Hive;
use model::config::Configuration;
use bigint::hash::H256;
use memorydb::MemoryDB;
//
use model::validator::MonteCarlo;
use model::transaction::Hash;
use model::transaction::HASH_NULL;
use std::collections::HashMap;
use std::collections::HashSet;
//
fn main() {

    env_logger::init().expect("Failed to init logger");
   /* //
    let m_c:MonteCarlo = MonteCarlo::new();
    let values :( &HashSet<Hash>, &HashMap<Hash, i64>,& Hash, &Hash, & mut HashMap<Hash, i64>, &u32, &u32, &HashSet<Hash>) =
        (&HashSet::new(), &HashMap::new(), &HASH_NULL, &HASH_NULL, & mut HashMap::new(), &0, &0, &HashSet::new());

    let validator:Hash = m_c.markov_chain_monte_carlo(values.0,
                                                     values.1,
                                                     values.2,
                                                     values.3,
                                                     values.4,
                                                     values.5,
                                                     values.6,
                                                     values.7);
    //
    */
    let host = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse host string");
    let addr = SocketAddr::new(host, PORT);
    let sock = TcpListener::bind(&addr).expect("Failed to bind address");

    let mut poll = Poll::new().expect("Failed to create Poll");
    let mut server = Node::new(sock);

    server.run(&mut poll).expect("Failed to run server");
}