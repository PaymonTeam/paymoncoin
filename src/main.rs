extern crate byteorder;
extern crate mio;
extern crate rand;
extern crate slab;
extern crate env_logger;
extern crate rustc_serialize;
extern crate iron;
extern crate ntrumls;
extern crate linked_hash_set;
extern crate crypto;
extern crate futures;

#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;

#[macro_use]
extern crate tokio_io;

#[macro_use] pub mod utils;
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

use network::node::*;
//use network::replicator_pool::ReplicatorSourcePool;
use model::config::{PORT, Configuration, ConfigurationSettings};
use model::config;
use network::paymoncoin::PaymonCoin;
use env_logger::LogBuilder;
use log::{LogRecord, LogLevelFilter};
use storage::Hive;
use network::api::API;

fn main() {
    let format = |record: &LogRecord| {
        format!("[{}]: {}", record.level(), record.args())
//        format!("[{} {:?}]: {}", record.level(), thread::current().id(), record.args())
    };

    let mut builder = LogBuilder::new();
    builder.format(format)
        .filter(None, LogLevelFilter::Info)
        .filter(Some("futures"), LogLevelFilter::Error)
        .filter(Some("tokio"), LogLevelFilter::Error)
        .filter(Some("tokio-io"), LogLevelFilter::Error)
        .filter(Some("hyper"), LogLevelFilter::Error)
        .filter(Some("iron"), LogLevelFilter::Error);

    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init().unwrap();

    let mut jhs = VecDeque::new();

    let jh = Builder::new().name(format!("pmnc")).spawn(move || {
        let mut config = Configuration::new();
        let port = config.get_int(ConfigurationSettings::Port).unwrap();

        let pmnc = Arc::new(Mutex::new(PaymonCoin::new(config)));

        let node_arc = pmnc.lock().unwrap().run();

        let pmnc_clone = pmnc.clone();
//            let mut api_running = Arc::new(AtomicBool::from(true));
//            let api_running_clone = api_running.clone();
        let api_running = Arc::new((Mutex::new(true), Condvar::new()));
        let api_running_clone = api_running.clone();

        let api_jh = thread::spawn(move || {
            let mut api = API::new(pmnc_clone, (port + 10) as u16, api_running_clone);
            api.run();
            drop(api);
        });

        thread::sleep(Duration::from_secs(10));
//            if let Ok(n) = node_arc.lock() {
//                n.neighbors.lock().pu
//            }
        thread::sleep(Duration::from_secs(10000));

        {
            //            api_running.store(false, Ordering::SeqCst);
            let &(ref lock, ref cvar) = &*api_running;
            let mut is_running = lock.lock().unwrap();
            *is_running = false;
            cvar.notify_one();
        }

        api_jh.join();
        {
//                if let Ok(mut p) = pmnc.lock() {
            pmnc.lock().and_then(|mut p| {
                p.shutdown();
                Ok(p)
            });
//                    p.shutdown();
//                }
        }
    }).unwrap();

    jhs.push_back(jh);

    while let Some(jh) = jhs.pop_front() {
        jh.join();
    }
}

#[test]
fn hive_test() {
    use model::{Transaction, TransactionObject};
    use model::transaction::{Hash, ADDRESS_NULL, HASH_SIZE};
    use storage::hive::{CFType};

    use self::rustc_serialize::hex::{ToHex, FromHex};
    use rand::Rng;

    let mut hive = Hive::new();
    hive.init();

    let h0 = Hash([1u8; HASH_SIZE]);
    let h1 = Hash([2u8; HASH_SIZE]);
    let h2 = Hash([3u8; HASH_SIZE]);

    assert!(hive.put_approvee(h1, h0));
    assert!(hive.put_approvee(h2, h0));

    let hashes = hive.storage_load_approvee(&h0).expect("failed to load hashes");
//    println!("{:?}", hashes);

    let mut t0 = TransactionObject::new_random();
    hive.storage_put(CFType::Transaction, &t0.hash, &t0);
    let t1 = hive.storage_load_transaction(&t0.hash).expect("failed to load transaction from db");
    assert_eq!(t0, t1.object);

    let addr0 = ADDRESS_NULL;

    let random_sk = true;

    let mut data =
        "2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A\
        3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87\
        EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A\
        0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A774\
        84468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B021\
        4EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD".from_hex().expect("invalid sk");
//    let mut sk_data = [0u8; 32 * 8];
//    if random_sk {
//        rand::thread_rng().fill_bytes(&mut sk_data);
//    } else {
//        sk_data.copy_from_slice(&data[..(32 * 8)]);
//    }

//    let (addr, sk, pk) = Hive::generate_address(&sk_data, 0);
//    hive.storage_put(CFType::Address, &addr, &10000u32);
//    let balance = hive.storage_get_address(&addr).expect("storage get address error");

//    println!("sk={}", sk_data.to_hex().to_uppercase());
//    println!("address={:?}", addr);
//    println!("address={:?} balance={}", addr, balance);
}

#[test]
fn hive_transaction_test() {
    use model::{Transaction, TransactionObject};
    use model::transaction::ADDRESS_NULL;
    use storage::hive::{CFType};

    use self::rustc_serialize::hex::{ToHex, FromHex};
    use rand::Rng;

    let mut hive = Hive::new();
    hive.init();

    let mut t0 = TransactionObject::new_random();
    hive.storage_put(CFType::Transaction, &t0.hash, &t0);
    let t1 = hive.storage_load_transaction(&t0.hash).expect("failed to load transaction from db");
    assert_eq!(t0, t1.object);

    let addr0 = ADDRESS_NULL;

    let random_sk = true;

    let mut data =
        "2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A\
        3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87\
        EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A\
        0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A774\
        84468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B021\
        4EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD".from_hex().expect("invalid sk");
//    let mut sk_data = [0u8; 32 * 8];
////    if random_sk {
////        rand::thread_rng().fill_bytes(&mut sk_data);
////    } else {
////        sk_data.copy_from_slice(&data[..(32 * 8)]);
////    }

//    let addr = Hive::generate_address(&sk_data, 0);
//    hive.storage_put(CFType::Address, &addr, &10000u32);
//    let balance = hive.storage_get_address(&addr).expect("storage get address error");

//    println!("sk={}", sk_data.to_hex().to_uppercase());
//    println!("address={:?}", addr);
//    println!("address={:?} balance={}", addr, balance);
}

#[test]
fn pos_test() {

}