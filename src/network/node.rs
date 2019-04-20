use std::io;
use crate::utils::config::{Configuration, ConfigurationSettings};
use std::sync::{Arc, Weak, Mutex};
use crate::network::neighbor::Neighbor;
use std::net::{TcpStream, SocketAddr, IpAddr};
use std::sync::mpsc::{Sender, Receiver};
use std::collections::{VecDeque, BTreeSet};
use crate::transaction::{TransactionObject, Transaction, TransactionType};
use crate::transaction::contract::{ContractStorage, ContractsStorage, ContractOutput};
use crate::transaction::contracts_manager::{ContractsManager};
use crate::storage::Hive;
use std::sync::atomic::{AtomicBool, Ordering, AtomicUsize};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use crate::transaction::transaction;
use crate::transaction::transaction::{Address, Hash, HASH_NULL};
use rand::{Rng, thread_rng};
use crate::utils::{AM, AWM};
use crate::network::{rpc};
use crate::transaction::*;
use futures::{Stream, Async};
use futures::prelude::*;
use futures::executor::Executor;
use crate::consensus::{self, Validator, ValidatorIndex, pos::{
    Error,
    Context,
    Secp256k1SignatureScheme,
    ROUND_DURATION,
}};

use serde_pm::{SerializedBuffer, from_stream, to_buffer, Identifiable};
use futures::{
    self,
    AndThen,
    executor::{Run, Spawn},
    future::{FutureResult},
    oneshot,
    prelude::*,
    sync::{mpsc, oneshot},
    task::Task,
    Future,
};
use rhododendron as bft;
use serde::Serialize;
use num_traits::real::Real;

type CommunicationType = bft::Communication<rpc::ContractsInputOutputs, [u8; 32], ValidatorIndex, consensus::Secp256k1Signature>;

#[derive(Clone)]
struct LocalValidator {
    pub sk: secp256k1::SecretKey,
    pub pk: secp256k1::PublicKey,
    pub address: ValidatorIndex,
}

pub struct Node {
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
    scoped_thread_join_handles: VecDeque<()>,
    transaction_validator: AM<TransactionValidator>,
    transaction_requester: AM<TransactionRequester>,
    tips_vm: AM<TipsViewModel>,
    milestone: AM<Milestone>,
    bft_in: AM<mpsc::UnboundedReceiver<(Address, CommunicationType)>>,
    bft_out: AM<mpsc::UnboundedSender<(CommunicationType)>>,
    contracts_manager: AM<ContractsManager>,
    local_validator: AM<Option<LocalValidator>>,
}

impl Node {
    pub fn new(hive: Weak<Mutex<Hive>>, config: Configuration, node_tx: Sender<()>, pmnc_rx:
    Receiver<()>, transaction_validator: AM<TransactionValidator>, transaction_requester:
               AM<TransactionRequester>, tips_vm: AM<TipsViewModel>, milestone: AM<Milestone>)
               -> Node {
        let (_, in_rx) = mpsc::unbounded();
        let (out_tx, _) = mpsc::unbounded();

        Node {
            hive,
            running: Arc::new(AtomicBool::new(true)),
            neighbors: make_am!(Vec::new()),
            config,
            node_tx,
            pmnc_rx,
            broadcast_queue: make_am!(VecDeque::new()),
            receive_queue: make_am!(VecDeque::new()),
            reply_queue: make_am!(VecDeque::new()),
            thread_join_handles: VecDeque::new(),
            scoped_thread_join_handles: VecDeque::new(),
            transaction_requester,
            transaction_validator,
            tips_vm,
            milestone,
            bft_in: make_am!(in_rx),
            bft_out: make_am!(out_tx),
            contracts_manager: make_am!(ContractsManager::new()),
            local_validator: make_am!(None),
        }
    }

    pub fn init(&mut self, replicator_jh: JoinHandle<()>) {
        if let Some(s) = self.config.get_string(ConfigurationSettings::Neighbors) {
            if s.len() > 0 {
                for addr in s.split(",") {
                    if let Ok(addr) = addr.parse::<SocketAddr>() {
                        if let Ok(mut neighbors) = self.neighbors.lock() {
                            if neighbors.iter().find(|arc| {
                                if let Ok(n) = arc.lock() {
                                    return n.addr == addr;
                                }
                                false
                            }).is_none() {
                                debug!("Added new neighbor ({:?})", addr);
                                neighbors.push(Arc::new(Mutex::new(Neighbor::from_address(addr))));
                            }
                        }
                    } else {
                        debug!("invalid address: {:?}", addr);
                    }
                }
            }
        }

        let (in_tx, in_rx) = mpsc::unbounded();
        let (out_tx, out_rx) = mpsc::unbounded::<(CommunicationType)>();

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

        let receiver_changed = Arc::new(AtomicBool::new(false));
        let running_weak = Arc::downgrade(&self.running.clone());
        let neighbors_weak = Arc::downgrade(&self.neighbors.clone());
        let receiver_changed_weak = Arc::downgrade(&receiver_changed.clone());
        let in_rx: AM<Option<_>> = make_am!(Some(in_rx));
        let in_rx_weak = Arc::downgrade(&in_rx.clone());
        let jh = thread::spawn(|| Node::bft_thread(running_weak, neighbors_weak, in_rx_weak, receiver_changed_weak));
        self.thread_join_handles.push_back(jh);

        self.thread_join_handles.push_back(replicator_jh);

        use crossbeam::scope;

        self.bft_out = make_am!(out_tx);

        let cm_weak = Arc::downgrade(&self.contracts_manager);
        let in_rx_clone = Arc::downgrade(&in_rx);
        let out_tx_clone = Arc::downgrade(&self.bft_out);
        let running_clone = Arc::downgrade(&self.running);
        let hive_weak = self.hive.clone();
        let local_validator_weak = Arc::downgrade(&self.local_validator);

        thread::spawn(move || {
            use secp256k1::*;
            use std::collections::HashMap;
            use std::str::FromStr;

            let mut current_index = 0;
            let running = running_clone.upgrade().unwrap();
            while !running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(1));
            }
            drop(running);
            let cm_weak_clone = cm_weak.clone();
            let lv = local_validator_weak.clone();

            loop {
                let local_validator = match lv.clone().upgrade() {
                    Some(v) => match v.lock() {
                        Ok(v) => (*v).clone(),
                        Err(_) => continue
                    },
                    None => continue
                };

                if let Some(local_validator) = local_validator {
                    let (in_tx, in_rx_new) = mpsc::unbounded();
                    let (out_tx, out_rx) = mpsc::unbounded();
                    {
                        let arc1 = in_rx.clone();
                        let mut v = arc1.lock().unwrap();
                        *v = Some(in_rx_new);
                        let arc2 = out_tx_clone.clone().upgrade().unwrap();
                        let mut v = arc2.lock().unwrap();
                        *v = out_tx;
                    }
                    receiver_changed.store(false, Ordering::SeqCst);

                    let arc3 = cm_weak.clone().upgrade().unwrap();
                    let mut cm = arc3.lock().unwrap();
                    let validators_ids = cm.future_validators.clone();
                    let node_count = validators_ids.len();
                    let max_faulty = (node_count as f32 / 3.0).floor() as usize;

                    let executed = cm.execute_contracts();
                    drop(cm);

                    let timer = tokio_timer::wheel().tick_duration(ROUND_DURATION).build();
                    let timer_copy = timer.clone();

                    let local_id = local_validator.address;
                    let mut validators = HashMap::new();
                    for vid in &validators_ids {
                        validators.insert(vid.clone(), Validator::from_index(vid.clone()));
                    }

                    let ctx = Context {
                        signature: Secp256k1SignatureScheme::new(local_validator.sk.clone()),
                        local_id,
                        proposal: Mutex::new(Some(rpc::ContractsInputOutputs { vec: executed.clone() })),
                        current_round: Arc::new(AtomicUsize::new(0)),
                        timer: timer_copy,
                        evaluated: Mutex::new(BTreeSet::new()),
                        node_count,
                        validators,
                        queue: vec![]
                    };

                    let hive_weak_clone = hive_weak.clone();
                    let f = bft::agree(
                        ctx,
                        node_count,
                        max_faulty,
                        out_rx.map_err(|_| Error),
                        in_tx.sink_map_err(|_| Error).with(move |t| Ok((/*local_id*/Address::from_str("").unwrap(), t))),
                    )
                        .map_err(|e| warn!("error on consensus {:?}", e))
                        .map(|r| {
                            info!("committed = {:?}", r);
                            match r.candidate {
                                Some(c) => {
//                                    let mut cm = cm_weak_clone.clone().upgrade().unwrap().lock().unwrap();
//                                    for call in executed {
//                                        cm.apply_state(call, hive_weak_clone);
//                                    }
                                }
                                None => {}
                            }
                        });
                    tokio::run(f);

                    receiver_changed.store(true, Ordering::SeqCst);
                }
                thread::sleep(Duration::from_secs(3));
            }
        });
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
                                        if transaction_requester.num_transactions_to_request() > 0 &&
                                            thread_rng().gen::<f64>() < 0.66 {
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
                                                    transaction = Box::new(hive.storage_load_transaction(&tip).unwrap());
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
                                        transaction = Box::new(hive.storage_load_transaction(&hash).unwrap());
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
                            info!("received tx: {:?}\n{:?}", t.get_hash(), t.object);
                            let _address = t.object.address.clone();
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

    fn bft_thread(running: Weak<AtomicBool>, neighbors: AWM<Vec<AM<Neighbor>>>, bft_in: AWM<Option<mpsc::UnboundedReceiver<(ValidatorIndex, CommunicationType)>>>, receiver_changed: Weak<AtomicBool>) {
        loop {
            let r = running.upgrade().unwrap();
            if r.load(Ordering::SeqCst) {
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }

        struct Trigger {
            receiver_changed: Weak<AtomicBool>,
        };

        impl Future for Trigger {
            type Item = ();
            type Error = ();

            fn poll(&mut self) -> Result<Async<<Self as Future>::Item>, <Self as Future>::Error> {
                thread::sleep(Duration::from_millis(50));
                let rc = self.receiver_changed.upgrade().unwrap();

                if rc.load(Ordering::SeqCst) {
                    return Ok(Async::Ready(()));
                } else {
                    return Ok(Async::NotReady);
                }
            }
        }

        loop {
            let trigger = Trigger { receiver_changed: receiver_changed.clone() };
            let mut arc1 = bft_in.clone().upgrade().unwrap();
            let mut bft_in = arc1.lock().unwrap();
            let bft_in = bft_in.take();
            if let Some(bft_in) = bft_in {
                let neighbors_c = neighbors.clone();

                tokio::run(bft_in.for_each(move |(i, c)| {
                    debug!("sending {:?} {:?}", i, c);
                    if let Some(arc) = neighbors_c.upgrade() {
                        if let Ok(neighbors) = arc.lock() {
                            for n in neighbors.iter() {
                                if let Ok(mut n) = n.lock() {
                                    info!("sending to {:?}", n.addr);
                                    n.send_packet(Box::new(c.clone()));
                                }
                            }
                        }
                    }
                    futures::future::ok(())
                }).select(trigger).map(|_| ()).map_err(|_| ()));
            }

            thread::sleep(Duration::from_secs(1));
        }
    }

    fn broadcast_thread(running: Weak<AtomicBool>, broadcast_queue: AWM<VecDeque<Transaction>>, neighbors: AWM<Vec<AM<Neighbor>>>, tr: AWM<TransactionRequester>/*, bft_in: mpsc::UnboundedReceiver<(usize, ConsensusType)>*/) {
        loop {

//            tokio::spawn(bft_in).
//            match bft_in.poll() {
//                Ok(Async::Ready(Some((i, t)))) => {
//                    debug!("bft_in received {}, {:?}", i, t);
//                }
//                Ok(Async::Ready(None)) => {
//
//                }
//                _ => {
//                    debug!("bft_in returned something wrong");
//                }
//            }


            if let Some(arc) = running.upgrade() {
                let b = arc.load(Ordering::SeqCst);
                if !b {
                    debug!("exiting broadcast thread");
                    break;
                }
            }

            let mut to_send = Vec::<Box<Transaction>>::new();

            if let Some(arc) = broadcast_queue.upgrade() {
                if let Ok(mut queue) = arc.lock() {
                    if let Some(v) = queue.pop_front() {
                        to_send.push(Box::new(v));
                    }
                }
            }

//            async {
//                let s = await!(bft_in.map(|(_, v)| v).collect());
//            }
            for packet in to_send {
                if let Some(arc) = neighbors.upgrade() {
                    if let Ok(neighbors) = arc.lock() {
                        for n in neighbors.iter() {
                            if let Ok(mut n) = n.lock() {
//                                n.send_packets(s.clone());
                                // TODO: make delay and clone neighbors list
                                info!("sending to {:?}", n.addr);
                                n.send_packet(packet.clone());

                                if let Some(arc) = tr.upgrade() {
                                    if let Ok(mut tr) = arc.lock() {
                                        // TODO: get probability from var
                                        if let Some(hash) = tr
                                            .poll_transaction_to_request(thread_rng().gen::<f64>() < 0.7) {
                                            n.send_packet(Box::new(rpc::RequestTransaction {
                                                hash
                                            }));
                                        } else {
                                            n.send_packet(Box::new(rpc::RequestTransaction {
                                                hash: HASH_NULL
                                            }));
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
        use std::collections::HashMap;
        use serde::{Serialize};
        use serde_pm::Identifiable;

        let neighbor;
        if let Ok(neighbors) = self.neighbors.lock() {
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

        let _length = data.limit();
        let _mark = data.position();
        let _message_id = data.read_i64().unwrap();
        let message_length = data.read_i32().unwrap();

        if message_length > data.remaining() as i32 {
            warn!("Received incorrect message length");
        }

        let svuid = data.read_u32().unwrap();

        debug!("received svuid {:X}", svuid);

        if svuid == TransactionObject::primary_type_id() {
            let mut transaction_object = from_stream(&mut data).expect("failed to deserialize tx");
            let mut transaction = Transaction::from_object(transaction_object);
            if let Ok(mut queue) = self.receive_queue.lock() {
                queue.push_back(transaction);
            }
        } else if svuid == rpc::RequestTransaction::primary_type_id() {
            let tx_request: rpc::RequestTransaction = from_stream(&mut data).unwrap();

            let mut hash = tx_request.hash;
            if let Ok(mut queue) = self.reply_queue.lock() {
                queue.push_back((hash, neighbor.clone()));
            }
        } else if svuid == CommunicationType::primary_type_id() {
            let v = from_stream(&mut data).unwrap();
            info!("received communication {:?}", v);
            self.bft_out.lock().unwrap().unbounded_send(v);
        } else if svuid == rpc::Signed::primary_type_id() {
//            use std::str::FromStr;
//            use crate::network::rpc::SignedData;
//
//            static ALLOWED_VALIDATOR_ADRESSES: [Address; 1] = [
//                Address::from_str("asdasd").unwrap(),
//            ];
//            static MINIMUM_STAKE: u64 = 100;
//
//            let signed: rpc::Signed = from_stream(&mut data).unwrap();
//            match signed.data {
//                SignedData::ApplyForValidator { address, stake } => {
//                    if address.verify() {
//                        let hive = self.hive.upgrade().unwrap().lock().unwrap();
//                        match hive.storage_get_address(&address) {
//                            Some(b) => {
//                                if b >= stake && stake >= MINIMUM_STAKE {
//                                    self.contracts_storage.call()
//                                } else {
//                                    warn!("{:?} attempted to be a validator")
//                                }
//                            }
//                        }
//                    }
//                }
//            }
        } else {
            warn!("Unknown SVUID {:X}", svuid);
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

        while let Some(_th) = self.scoped_thread_join_handles.pop_front() {}
    }
}

extern fn handle_sigint(_:i32) {
    println!("Interrupted!");
    panic!();
}
