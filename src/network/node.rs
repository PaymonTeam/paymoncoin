use std::io::Error;
use model::config::{Configuration, ConfigurationSettings};
use network::packet::{SerializedBuffer};
use std::sync::{Arc, Weak, Mutex};
use network::neighbor::Neighbor;
use std::net::{TcpStream, SocketAddr, IpAddr};
use std::sync::mpsc::{Sender, Receiver};
use std::collections::VecDeque;
use model::{TransactionObject, Transaction, TransactionType};
use ntrumls::{NTRUMLS, PQParamSetID, PublicKey, PrivateKey};
use storage::Hive;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use model::transaction;
use model::transaction::{Hash, HASH_NULL};
use rand::{Rng, thread_rng};
use utils::{AM, AWM};
use network::{rpc, packet::Serializable};
use model::*;
use futures::{Stream, Async};
use futures::prelude::*;
use futures::executor::Executor;
use consensus::Validator;

extern fn handle_sigint(_:i32) {
    println!("Interrupted!");
    panic!();
}

pub struct Pair<U, V> {
    pub low: U,
    pub hi: V,
}

impl<U, V> Pair<U, V> {
    pub fn new(low: U, hi: V) -> Self {
        Pair {
            low,
            hi
        }
    }
}

pub struct Node<'a> {
    hive: AWM<Hive>,
    pub neighbors: AM<Vec<AM<Neighbor>>>,
    config: Configuration,
    node_tx: Sender<()>,
    pmnc_rx: Receiver<()>,
    pub broadcast_queue: AM<VecDeque<Transaction>>,
    receive_queue: AM<VecDeque<Transaction>>,
    reply_queue: AM<VecDeque<(Hash, AM<Neighbor>)>>,
    running: Arc<AtomicBool>,
    thread_join_handles: VecDeque<JoinHandle<()>>,
    transaction_validator: AM<TransactionValidator>,
    transaction_requester: AM<TransactionRequester>,
    tips_vm: AM<TipsViewModel>,
    milestone: AM<Milestone>,
    to_send: OutputStream<'a>,
    received_consensus_values: InputStream<'a>,
}

pub type PacketData<'a> = Pair<Validator, Box<dyn Serializable + Send + 'a>>;

#[derive(Clone)]
pub struct OutputStream<'a> {
    queue: AM<Vec<PacketData<'a>>>,
}

impl<'a> OutputStream<'a> {
    fn new() -> Self {
        OutputStream {
            queue: make_am!(Vec::new()),
        }
    }

    pub fn send_packet(&mut self, packet: PacketData<'a>) {
        let mut q = self.queue.lock().unwrap();
        q.push(packet);
    }

    pub fn send_packet2(&mut self, b: Arc<dyn Serializable + 'a>) {
        drop(b);
    }
}

impl<'a> Stream for OutputStream<'a> {
    type Item = PacketData<'a>;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Option<<Self as Stream>::Item>>, <Self as Stream>::Error> {
        let mut q = self.queue.lock().unwrap();
        Ok(Async::Ready(q.pop()))
    }
}

#[derive(Clone)]
pub struct InputStream<'a> {
    queue: AM<Vec<PacketData<'a>>>,
}

impl<'a> InputStream<'a> {
    fn new() -> Self {
        InputStream {
            queue: make_am!(Vec::new()),
        }
    }

    pub fn send_packet(&mut self, packet: PacketData) {
        let mut q = self.queue.lock().unwrap();
        q.push(packet);
    }
}

impl<'a> Stream for InputStream<'a> {
    type Item = PacketData<'a>;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Option<<Self as Stream>::Item>>, <Self as Stream>::Error> {
        let mut q = self.queue.lock().unwrap();
        Ok(Async::Ready(q.pop()))
//        Ok(Async::NotReady)
    }
}

impl<'a> Node<'a> {
    pub fn new(hive: Weak<Mutex<Hive>>, config: &Configuration, node_tx: Sender<()>, pmnc_rx:
        Receiver<()>, transaction_validator: AM<TransactionValidator>, transaction_requester:
        AM<TransactionRequester>, tips_vm: AM<TipsViewModel>, milestone: AM<Milestone>)
        -> Node {
        let to_send = OutputStream::new();
        let received_consensus_values = InputStream::new();

        Node {
            hive,
            running: Arc::new(AtomicBool::new(true)),
            neighbors: make_am!(Vec::new()),
            config: config.clone(),
            node_tx,
            pmnc_rx,
            broadcast_queue: make_am!(VecDeque::new()),
            receive_queue: make_am!(VecDeque::new()),
            reply_queue: make_am!(VecDeque::new()),
            thread_join_handles: VecDeque::new(),
            transaction_requester,
            transaction_validator,
            tips_vm,
            milestone,
            to_send,
            received_consensus_values,
        }
    }

    pub fn init(&mut self, replicator_jh: JoinHandle<()>) {
        if let Some(s) = self.config.get_string(ConfigurationSettings::Neighbors) {
            if s.len() > 0 {
                for addr in s.split(" ") {
                    if let Ok(addr) = addr.parse::<SocketAddr>() {
                        if let Ok(mut neighbors) = self.neighbors.lock() {
                            if neighbors.iter().find(|arc| {
                                if let Ok(n) = arc.lock() {
                                    return n.addr.ip() == addr.ip();
                                }
                                false
                            }).is_none() {
                                neighbors.push(Arc::new(Mutex::new(Neighbor::from_address(addr))));
                            }
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
        let tr_weak = Arc::downgrade(&self.transaction_requester.clone());
        let jh = thread::spawn(|| Node::broadcast_thread(running_weak, broadcast_queue_weak, neighbors_weak,
                                                         tr_weak));
        self.thread_join_handles.push_back(jh);

        let running_weak = Arc::downgrade(&self.running.clone());
        let receive_queue_weak = Arc::downgrade(&self.receive_queue.clone());
        let broadcast_queue_weak = Arc::downgrade(&self.broadcast_queue.clone());
        let hive_weak = self.hive.clone();
        let tv_weak = Arc::downgrade(&self.transaction_validator.clone());
        let jh = thread::spawn(|| Node::receive_thread(running_weak, receive_queue_weak,
                                                       broadcast_queue_weak, hive_weak, tv_weak));
        self.thread_join_handles.push_back(jh);

        let running_weak = Arc::downgrade(&self.running.clone());
        let reply_queue_weak = Arc::downgrade(&self.reply_queue.clone());
        let hive_weak = self.hive.clone();
        let tr_weak = Arc::downgrade(&self.transaction_requester.clone());
        let ms_weak = Arc::downgrade(&self.milestone.clone());
        let tvm_weak = Arc::downgrade(&self.tips_vm.clone());
        let jh = thread::spawn(|| Node::reply_thread(running_weak, reply_queue_weak, hive_weak,
                                                     tr_weak, ms_weak, tvm_weak));
        self.thread_join_handles.push_back(jh);

        self.thread_join_handles.push_back(replicator_jh);
    }

    fn reply_thread(running: Weak<AtomicBool>, reply_queue: AWM<VecDeque<(Hash, AM<Neighbor>)>>,
                    hive: AWM<Hive>, tr: AWM<TransactionRequester>, milestone: AWM<Milestone>,
                    tvm: AWM<TipsViewModel>) {
        loop {
            if let Some(arc) = running.upgrade() {
                let b = arc.load(Ordering::SeqCst);
                if !b { break; }

                if let Some(arc) = reply_queue.upgrade() {
                    if let Ok(mut queue) = arc.lock() {
                        if let Some((hash, neighbor_am)) = queue.pop_front() {
                            let transaction;

                            if hash == HASH_NULL {
                                if let Some(arc) = tr.upgrade() {
                                    if let Ok(mut transaction_requester) = arc.lock() {
                                        // TODO: make P independent var
                                        if transaction_requester.num_transactions_to_request() >
                                            0 && thread_rng().gen::<f64>() < 0.66 {
                                            let tip;
                                            if thread_rng().gen::<f64>() < 0.02 {
                                                if let Some(arc) = milestone.upgrade() {
                                                    if let Ok(mut ms) = arc.lock() {
                                                        tip = ms.latest_milestone;
                                                    } else { panic!("broken milestone mutex"); }
                                                } else { continue; }
                                            } else {
                                                if let Some(arc) = tvm.upgrade() {
                                                    if let Ok(mut tvm) = arc.lock() {
                                                        tip = tvm.get_random_solid_tip().unwrap_or(HASH_NULL);
                                                    } else { panic!("broken tvm mutex"); }
                                                } else { continue; }
                                            };
                                            if let Some(arc) = hive.upgrade() {
                                                if let Ok(mut hive) = arc.lock() {
                                                    transaction = hive.storage_load_transaction(&tip).unwrap();
                                                } else {
                                                    panic!("broken hive mutex");
                                                }
                                            } else {
                                                continue;
                                            }
                                        } else {
                                            continue;
                                        }
                                    } else { panic!("broken transaction requester mutex"); }
                                } else { continue; }
                            } else {
                                if let Some(arc) = hive.upgrade() {
                                    if let Ok(mut hive) = arc.lock() {
                                        transaction = hive.storage_load_transaction(&hash).unwrap();
                                    } else {
                                        panic!("broken hive mutex");
                                    }
                                } else {
                                    continue;
                                }
                            }

                            if transaction.get_type() == TransactionType::Full {
                                if let Ok(mut n) = neighbor_am.lock() {
                                    n.send_packet(transaction);
                                }
                            } else {
                                // TODO: make P independent var
                                if hash != HASH_NULL && thread_rng().gen::<f64>() < 0.01 {
                                    if let Some(arc) = tr.upgrade() {
                                        if let Ok(mut transaction_requester) = arc.lock() {
                                            transaction_requester.request_transaction(hash, false);
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

    fn receive_thread(running: Weak<AtomicBool>, receive_queue: AWM<VecDeque<Transaction>>,
                      broadcast_queue: AWM<VecDeque<Transaction>>, hive: AWM<Hive>, tv:
                      AWM<TransactionValidator>) {
        loop {
            if let Some(arc) = running.upgrade() {
                let b = arc.load(Ordering::SeqCst);
                if !b { break; }

                if let Some(arc) = receive_queue.upgrade() {
                    if let Ok(mut queue) = arc.lock() {
                        if let Some(mut t) = queue.pop_front() {
                            info!("received tx: {:?}", t.get_hash());
                            let address = t.object.address.clone();
                            let validated = transaction::validate_transaction(&mut t, 3);
                            info!("validated={}", validated);

                            if validated {
                                let mut stored;
                                if let Some(arc) = hive.upgrade() {
                                    if let Ok(mut hive) = arc.lock() {
                                        stored = hive.put_transaction(&t);
                                        info!("stored={}", stored);
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
                                                continue;
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

    fn broadcast_thread(running: Weak<AtomicBool>, broadcast_queue: AWM<VecDeque<Transaction>>, neighbors: AWM<Vec<AM<Neighbor>>>, tr: AWM<TransactionRequester>) {
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
                                            // TODO: make delay and clone neighbors list
                                            info!("sending to {:?}", n.addr);
                                            let transaction = t.object.clone();
                                            n.send_packet(transaction);
                                            if let Some(arc) = tr.upgrade() {
                                                if let Ok(mut tr) = arc.lock() {
                                                    // TODO: get probability from var
                                                    if let Some(hash) = tr
                                                        .poll_transaction_to_request(thread_rng().gen::<f64>() < 0.7) {
                                                        n.send_packet(rpc::RequestTransaction {
                                                            hash
                                                        });
                                                    } else {
                                                        n.send_packet(rpc::RequestTransaction {
                                                            hash: HASH_NULL
                                                        });
                                                    }
                                                }
                                            }
                                        }
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
        let mut neighbor;
        if let Ok(mut neighbors) = self.neighbors.lock() {
            neighbor = match neighbors.iter().find(
                |arc| {
                    if let Ok(n) = arc.lock() { return n.addr.ip() == addr.ip() }
                    false
                }) {
                Some(arc) => arc.clone(),
                None => {
                    info!("Received data from unknown neighbor {:?}", addr);
                    return;
                }
            };
        } else {
            panic!("broken neighbors mutex");
        }
        let length = data.limit();

//        if length == 4 {
//            connection.close();
//            return;
//        }

        let mark = data.position();
        let message_id = data.read_i64();
        let message_length = data.read_i32();

//        if message_length != data.remaining() as i32 {
//            error!("Received incorrect message length");
//            return;
//        }

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
            rpc::RequestTransaction::SVUID => {
                let mut tx_request = rpc::RequestTransaction { hash: HASH_NULL };
                tx_request.read_params(&mut data);

                let mut hash = tx_request.hash;
                if let Ok(mut queue) = self.reply_queue.lock() {
                    queue.push_back((hash, neighbor.clone()));
                }
            }
            _ => {
                warn!("Unknown SVUID {}", svuid);
            }
        }
    }

    pub fn broadcast(&mut self, transaction: Transaction) {
        if let Ok(ref mut queue) = self.broadcast_queue.lock() {
            queue.push_back(transaction);
        }
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