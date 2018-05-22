extern crate nix;
extern crate ntrumls;

use std::io::Error;
use model::config::{Configuration, ConfigurationSettings};
use network::packet::{SerializedBuffer};
use std::sync::{Arc, Weak, Mutex};
use network::neighbor::Neighbor;
use std::net::{TcpStream, SocketAddr, IpAddr};
use std::sync::mpsc::{Sender, Receiver};
use std::collections::VecDeque;
use model::{TransactionObject, Transaction, TransactionType};
use self::ntrumls::{NTRUMLS, PQParamSetID, PublicKey, PrivateKey};
use storage::Hive;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use model::transaction;
//#[macro_use]
//use utils;
use utils::{AM, AWM};
use network::rpc;
use model::*;

extern fn handle_sigint(_:i32) {
    println!("Interrupted!");
    panic!();
}

pub struct Node {
    hive: AWM<Hive>,
    pub neighbors: AM<Vec<AM<Neighbor>>>,
    config: Configuration,
    node_tx: Sender<()>,
    pmnc_rx: Receiver<()>,
    broadcast_queue: AM<VecDeque<Transaction>>,
    receive_queue: AM<VecDeque<Transaction>>,
    running: Arc<AtomicBool>,
    thread_join_handles: VecDeque<JoinHandle<()>>,
    transaction_validator: AM<TransactionValidator>,
    transaction_requester: AM<TransactionRequester>,
    tips_vm: AM<TipsViewModel>,
    milestone: AM<Milestone>
}

impl Node {
    pub fn new(hive: Weak<Mutex<Hive>>, config: &Configuration, node_tx: Sender<()>, pmnc_rx:
    Receiver<()>, transaction_validator: AM<TransactionValidator>, transaction_requester:
    AM<TransactionRequester>, tips_vm: AM<TipsViewModel>, milestone: AM<Milestone>)
        -> Node {
        Node {
            hive,
            running: Arc::new(AtomicBool::new(true)),
            neighbors: make_am!(Vec::new()),
            config: config.clone(),
            node_tx,
            pmnc_rx,
            broadcast_queue: make_am!(VecDeque::new()),
            receive_queue: make_am!(VecDeque::new()),
            thread_join_handles: VecDeque::new(),
            transaction_requester,
            transaction_validator,
            tips_vm,
            milestone
        }
    }

    pub fn init(&mut self, replicator_jh: JoinHandle<()>) {
        if let Some(s) = self.config.get_string(ConfigurationSettings::Neighbors) {
            if s.len() > 0 {
                for addr in s.split(" ") {
                    if let Ok(addr) = addr.parse::<SocketAddr>() {
                        if let Ok(mut neighbors) = self.neighbors.lock() {
                            neighbors.push(Arc::new(Mutex::new(Neighbor::from_address(addr))));
                        }
                    } else {
                        debug!("invalid address: {:?}", addr);
                    }
                }
            }
        }

        let running_weak = Arc::downgrade(&self.running.clone());
        let broadcast_queue_weak = Arc::downgrade(&self.broadcast_queue.clone());
        let neighbors_weak = Arc::downgrade(&self.neighbors.clone());
        let jh = thread::spawn(|| Node::broadcast_thread(running_weak, broadcast_queue_weak, neighbors_weak));
        self.thread_join_handles.push_back(jh);

        let running_weak = Arc::downgrade(&self.running.clone());
        let receive_queue_weak = Arc::downgrade(&self.receive_queue.clone());
        let broadcast_queue_weak = Arc::downgrade(&self.broadcast_queue.clone());
        let hive_weak = self.hive.clone();
        let tv_weak = Arc::downgrade(&self.transaction_validator.clone());
        let jh = thread::spawn(|| Node::receive_thread(running_weak, receive_queue_weak,
                                                       broadcast_queue_weak, hive_weak, tv_weak));
        self.thread_join_handles.push_back(jh);

        self.thread_join_handles.push_back(replicator_jh);
    }

    fn receive_thread(running: Weak<AtomicBool>, receive_queue: AWM<VecDeque<Transaction>>,
                      broadcast_queue: AWM<VecDeque<Transaction>>, hive: AWM<Hive>, tv: AWM<TransactionValidator>) {
        loop {
            if let Some(arc) = running.upgrade() {
                let b = arc.load(Ordering::SeqCst);
                if !b { break; }

                if let Some(arc) = receive_queue.upgrade() {
                    if let Ok(mut queue) = arc.lock() {
                        if let Some(mut t) = queue.pop_front() {
                            let address = t.object.address.clone();
                            let validated = transaction::validate_transaction(&mut t, 9);
                            println!("validated={}", validated);

                            if validated {
                                let mut stored;
                                if let Some(arc) = hive.upgrade() {
                                    if let Ok(mut hive) = arc.lock() {
                                        stored = hive.put_transaction(&t);
                                        println!("stored={}", stored);
                                    } else {
                                        panic!("broken hive mutex");
                                    }
                                } else {
                                    continue;
                                }

                                if stored {
                                    if let Some(arc) = tv.upgrade() {
                                        if let Ok(mut tv) = arc.lock() {
                                            if let Err(e) = tv.update_status(&mut t) {
                                                error!("update status err {:?}", e);
                                            }
                                        }
                                    }
                                    if let Some(arc) = broadcast_queue.upgrade() {
                                        if let Ok(mut broadcast_queue) = arc.lock() {
                                            broadcast_queue.push_back(t);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                thread::sleep(Duration::from_secs(1));
            }
        }
    }

    fn broadcast_thread(running: Weak<AtomicBool>, broadcast_queue: AWM<VecDeque<Transaction>>, neighbors: AWM<Vec<AM<Neighbor>>>) {
        loop {
            if let Some(arc) = running.upgrade() {
                let b = arc.load(Ordering::SeqCst);
                if !b { break; }

                if let Some(arc) = broadcast_queue.upgrade() {
                    if let Ok(mut queue) = arc.lock() {
                        if let Some(t) = queue.pop_front() {
                            if let Some(arc) = neighbors.upgrade() {
                                if let Ok(neighbors) = arc.lock() {
                                    for n in neighbors.iter() {
                                        if let Ok(mut n) = n.lock() {
                                            let transaction = t.object.clone();
                                            n.send_packet(transaction);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                thread::sleep(Duration::from_secs(1));
            }
        }
    }

    pub fn run(&mut self) -> Result<(), Error> {
        return Ok(());
    }

    // TODO: return Result
    pub fn on_api_broadcast_transaction_received(&mut self, bt: rpc::BroadcastTransaction) {
        if let Ok(mut queue) = self.receive_queue.lock() {
            let mut transaction = Transaction::from_object(bt.transaction);
            queue.push_back(transaction);
//            return Ok(());
        }
    }

    pub fn on_connection_data_received(&mut self, mut data: SerializedBuffer, addr: SocketAddr) {
        if let Ok(mut neighbors) = self.neighbors.lock() {
            let neighbor = match neighbors.iter().find(
                |arc| {
                    if let Ok(n) = arc.lock() { return n.addr == addr }
                    false
                }) {
                Some(arc) => arc,
                None => {
                    info!("Received data from unknown neighbor {:?}", addr);
                    return;
                }
            };
        }
        let length = data.limit();

//        if length == 4 {
//            connection.mark_reset();
//            return;
//        }

        let mark = data.position();
        let message_id = data.read_i64();
        let message_length = data.read_i32();

        if message_length != data.remaining() as i32 {
            error!("Received incorrect message length");
            return;
        }

        let svuid = data.read_i32();

        use std::collections::HashMap;
        use network::packet::Serializable;

        match svuid {
            TransactionObject::SVUID => {
                let mut transaction_object = TransactionObject::new();
                transaction_object.read_params(&mut data);

                let mut transaction = Transaction::from_object(transaction_object);
                if let Ok(mut queue) = self.receive_queue.lock() {
                    queue.push_back(transaction);
                }
            }
            rpc::AttachTransaction::SVUID => {
                let mut transaction_object = TransactionObject::new();
                transaction_object.read_params(&mut data);

                let mut transaction = Transaction::from_object(transaction_object);
                if let Ok(mut queue) = self.receive_queue.lock() {
                    queue.push_back(transaction);
                }
            }
            _ => {
                warn!("Unknown SVUID {}", svuid);
            }
        }

//        let mut funcs = HashMap::<i32, fn(n: TS<Neighbor>)->()>::new();
//
//        funcs.insert(TransactionObject::SVUID, |n: TS<Neighbor>| {
//
////            let keep_alive = rpc::KeepAlive{};
////            conn.send_packet(keep_alive, 1);
//        });
//
//        if let Some(f) = funcs.get(&svuid) {
//            f(connection);
//        }
    }

    pub fn shutdown(&mut self) {
        self.node_tx.send(());

        info!("Shutting down node threads...");
        self.running.store(false, Ordering::SeqCst);

        while let Some(th) = self.thread_join_handles.pop_front() {
            th.join();
        }
    }
}