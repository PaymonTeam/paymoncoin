//#![feature(collection_placement)]
//#![feature(placement_in_syntax)]

extern crate byteorder;
extern crate mio;
extern crate rand;
extern crate slab;
extern crate secp256k1;
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

use network::neighbor::*;
use network::connection::*;
use network::packet::SerializedBuffer;
use network::rpc::KeepAlive;
use model::config::PORT;
use model::transaction::Transaction;
use storage::hive::Hive;
use network::packet::{Packet, get_object_size};
use model::config::Configuration;
use bigint::hash::H256;
use memorydb::MemoryDB;
use patricia_trie::{TrieFactory, TrieSpec, TrieMut, TrieDBMut};

type Slab<T> = slab::Slab<T, usize>;

use secp256k1::key::{PublicKey, SecretKey};
use secp256k1::{Secp256k1, Signature, RecoverableSignature, Message, RecoveryId, ContextFlag};
use rand::{Rng, thread_rng};

fn main() {
    env_logger::init().expect("Failed to init logger");
    let mut root = H256::new();
    let mut mdb = MemoryDB::new();
    let hive = Hive::new(&mut mdb, &mut root);

    let host = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse host string");
    let addr = SocketAddr::new(host, PORT);
    let sock = TcpListener::bind(&addr).expect("Failed to bind address");

    let mut poll = Poll::new().expect("Failed to create Poll");
    let mut server = Neighbor::new(sock);

    server.run(&mut poll).expect("Failed to run server");
}
