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
use network::replicator::Replicator;
use model::config::{PORT, Configuration, ConfigurationSettings};
use model::config;
use std::sync::{Arc, Weak, Mutex};
use std::io::{self, Read};

use std::env;
use log::{LogRecord, LogLevelFilter};
use env_logger::LogBuilder;
use std::thread;

fn main() {
    let format = |record: &LogRecord| {
        format!("[{} {:?}]: {}", record.level(), thread::current().id(), record.args())
    };

    let mut builder = LogBuilder::new();
    builder.format(format).filter(None, LogLevelFilter::Info);

    if env::var("RUST_LOG").is_ok() {
       builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init().unwrap();

    let ports = [0, 10001, 10002].iter();
    for port in ports {
        let port = *port;
        let mut neighbors = String::new();
        if port != 0 {
            let ports2 = [44832, 10001, 10002].iter();
            let v: Vec<String> = ports2.filter(|p| **p != port).map(|p| format!("127.0.0.1:{}", p)).collect();
            neighbors = v.join(" ");
        }
        println!("{}", neighbors);
        thread::spawn(move || {
            let mut config = Configuration::new();
            if port != 0 {
                config.set_string(ConfigurationSettings::Neighbors, &neighbors);
                config.set_int(ConfigurationSettings::Port, port);
            }

            let mut node = Arc::new(Mutex::new(Node::new(&config)));
            let replicator = Arc::new(Mutex::new(Replicator::new(&config, Arc::downgrade(&node.clone()))));

            replicator.lock().unwrap().run();

            {
                let mut guard = node.lock().unwrap();
                guard.init();
                guard.run().expect("Failed to run server");
            }

            use std::thread;
            use std::time::Duration;
            thread::sleep(Duration::from_secs(10));
            replicator.lock().unwrap().shutdown();
        });
    }

    use std::thread;
    use std::time::Duration;
    thread::sleep(Duration::from_secs(12));
}