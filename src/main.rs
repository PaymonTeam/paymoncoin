extern crate serde_pm;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json as json;
extern crate byteorder;
extern crate mio;
extern crate rand;
extern crate slab;
extern crate env_logger;
extern crate iron;
extern crate linked_hash_set;
extern crate crypto;
#[macro_use]
extern crate futures;
extern crate crossbeam;
extern crate hex;
extern crate tokio_timer;
extern crate parity_rocksdb as rocksdb;
extern crate secp256k1;
extern crate rhododendron;
#[macro_use]
extern crate serde_pm_derive;
extern crate chrono;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate tokio_io;
extern crate tokio;

#[macro_use]
pub mod utils;
pub mod consensus;
pub mod network;
pub mod model;
pub mod storage;

use std::{
    sync::{mpsc::channel, Condvar, Arc, Weak, Mutex, atomic::{AtomicBool, Ordering}},
    io::{self, Read},
    env,
    collections::VecDeque,
    thread,
    thread::Builder,
    time::Duration,
};

use mio::Poll;
use mio::net::TcpListener;

use crate::network::node::*;
use crate::model::config::{PORT, Configuration, ConfigurationSettings};
use crate::model::config;
use crate::network::paymoncoin::PaymonCoin;
use env_logger::LogBuilder;
use log::{LogRecord, LogLevelFilter};
use crate::storage::Hive;
use crate::network::api::API;

pub fn init_log() {
    let format = |record: &LogRecord| {
        use chrono::Local;
        let time = Local::now().format("%H:%M:%S");
        format!("[{} {}]: {}", record.level(), time, record.args())
//        format!("[{} {:?}]: {}", record.level(), thread::current().id(), record.args())
    };

    let mut builder = LogBuilder::new();
    builder.format(format)
        .filter(None, LogLevelFilter::Info)
        .filter(Some("futures"), LogLevelFilter::Error)
        .filter(Some("tokio"), LogLevelFilter::Error)
        .filter(Some("tokio-io"), LogLevelFilter::Error)
        .filter(Some("hyper"), LogLevelFilter::Error)
        .filter(Some("iron"), LogLevelFilter::Error)
        .filter(Some("serde_pm"), LogLevelFilter::Error)
        .filter(Some("bft"), LogLevelFilter::Trace)
    ;

    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init().unwrap();
}

fn main() {
    init_log();

    let mut jhs = VecDeque::new();

    let jh = Builder::new().name(format!("pmnc")).spawn(move || {
        let mut config = Configuration::new();

        if let Ok(port) = env::var("API_PORT") {
            config.set_int(ConfigurationSettings::Port, port.parse::<i32>().unwrap());
        }
        if let Ok(s) = env::var("NEIGHBORS") {
            config.set_string(ConfigurationSettings::Neighbors, &s);
        }
        let port = config.get_int(ConfigurationSettings::Port).unwrap();

        let pmnc = Arc::new(Mutex::new(PaymonCoin::new(config)));

        let _node_arc = pmnc.lock().unwrap().run();

        let pmnc_clone = pmnc.clone();
        let api_running = Arc::new((Mutex::new(true), Condvar::new()));
        let api_running_clone = api_running.clone();

        let api_jh = thread::spawn(move || {
            let mut api = API::new(pmnc_clone, (port + 10) as u16, api_running_clone);
            api.run();
            drop(api);
        });

        {
            let &(ref lock, ref cvar) = &*api_running;
            let mut is_running = lock.lock().unwrap();
            *is_running = false;
            cvar.notify_one();
        }

        api_jh.join();
        {
            pmnc.lock().and_then(|mut p| {
                p.shutdown();
                Ok(p)
            });
        }
    }).unwrap();

    jhs.push_back(jh);

    while let Some(jh) = jhs.pop_front() {
        jh.join();
    }
}