//#![feature(collection_placement)]
//#![feature(placement_in_syntax)]

extern crate byteorder;
extern crate mio;
extern crate rand;
extern crate slab;
extern crate ethcore_bigint as bigint;
extern crate memorydb;
extern crate patricia_trie;

#[macro_use] extern crate log;
extern crate env_logger;

pub mod network;
pub mod model;
pub mod storage;

use mio::Poll;
use mio::net::TcpListener;

use network::node::*;
use network::replicator_source_pool::ReplicatorSourcePool;
use model::config::{PORT, Configuration, ConfigurationSettings};
use model::config;
use std::sync::{Arc, Weak, Mutex};

fn main() {
    env_logger::init().expect("Failed to init logger");

    let mut config = Configuration::new();

    let mut node = Arc::new(Mutex::new(Node::new(&config)));
    let receiver = Arc::new(Mutex::new(ReplicatorSourcePool::new(&config, Arc::downgrade(&node.clone()))));

    let mut guard = node.lock().unwrap();
    guard.set_receiver(&Arc::downgrade(&receiver));

    receiver.lock().unwrap().run();
    guard.run().expect("Failed to run server");
}