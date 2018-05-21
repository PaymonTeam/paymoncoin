use std::{
    sync::{mpsc::{Receiver, Sender, channel}, Arc, Weak, Mutex},
    io::{self, Read},
    env,
    collections::VecDeque,
    thread,
    thread::{Builder, JoinHandle},
    time::Duration,
};
use storage::Hive;
use network::node::*;
use network::replicator_pool::ReplicatorPool;
use model::config::{PORT, Configuration, ConfigurationSettings};
use model::config;
use model::TipsViewModel;
use model::transaction::Address;
use utils::{AM, AWM};
use model::*;
use std::time;
use std::str::FromStr;

pub struct PaymonCoin {
    hive: AM<Hive>,
    pub node: AM<Node>,
    pub config: Configuration,
    pmnc_tx: Sender<()>,
    replicator_rx: Option<Receiver<()>>,
    coordinator: Address,
    tips_vm: AM<TipsViewModel>,
    transaction_requester: AM<TransactionRequester>,
    transaction_validator: AM<TransactionValidator>,
    milestone: AM<Milestone>,
    ledger_validator: AM<LedgerValidator>,
    tips_manager: AM<TipsManager>
}

impl PaymonCoin {
    pub fn new(config: Configuration) -> Self {
        let snapshot_timestamp = 1526912331;
        let coordinator = Address::from_str("P97bbb1e9e4a6ac3137f93c0607608219507ff4870a").unwrap();
        let num_keys_milestone = 22;
        let milestone_start_index = 1;

        let snapshot = Snapshot::init("db/snapshot.dat".to_string(), "".to_string()).expect("Can't \
        load \
        snapshot");
        let mut hive = Arc::new(Mutex::new(Hive::new()));

        // used for shutdown replicator pool
        let (replicator_tx, replicator_rx) = channel::<()>();
        let (pmnc_tx, pmnc_rx) = channel::<()>();
//        let (tx, rx) = channel();

        let mut tips_vm: AM<TipsViewModel> = make_am!(TipsViewModel::new());
        let mut transaction_requester: AM<TransactionRequester> = make_am!(TransactionRequester::new(hive.clone(), 0.001));
        let mut transaction_validator = TransactionValidator::new(hive.clone(), tips_vm.clone(),
                                                                  snapshot_timestamp, transaction_requester.clone());
        let mut milestone = Milestone::new(hive.clone(), coordinator.clone(), snapshot,
                                           transaction_validator.clone(), true,
                                           num_keys_milestone, milestone_start_index, true);

        let mut node = Arc::new(Mutex::new(Node::new(Arc::downgrade(&hive.clone()), &config,
                                                     replicator_tx, pmnc_rx,
                                                     transaction_validator.clone(),
                                                     transaction_requester.clone(), tips_vm
                                                         .clone(), milestone.clone())));
        let mut ledger_validator: AM<LedgerValidator> = make_am!(LedgerValidator::new(hive.clone(),
                                                                              milestone.clone(),
                                                        transaction_requester.clone()));
        let mut tips_manager = TipsManager::new(hive.clone(), milestone.clone(), ledger_validator
            .clone(), transaction_validator.clone(), tips_vm.clone(), 15, true,
                                                milestone_start_index);
        PaymonCoin {
            hive,
            node,
            pmnc_tx,
            config,
            replicator_rx: Some(replicator_rx),
            coordinator,
            tips_vm,
            transaction_requester,
            transaction_validator,
            milestone,
            ledger_validator,
            tips_manager,
        }
    }

    pub fn run(&mut self) -> AM<Node> {
        if let Ok(mut hive) = self.hive.lock() {
            hive.init();
        }
        Milestone::init(self.milestone.clone(), self.ledger_validator.clone());
        if let Ok(mut tv) = self.transaction_validator.lock() {
            tv.init(true, 9);
        }
        TipsManager::init(self.tips_manager.clone());

        let node_copy = self.node.clone();
        let config_copy = self.config.clone();
        let replicator_rx = self.replicator_rx.take().unwrap();
//        let (tx, replicator_rx) = channel::<()>();

        let replicator_jh = thread::spawn(move || {
            let mut replicator_pool = ReplicatorPool::new(&config_copy, Arc::downgrade(&node_copy),
                                                          replicator_rx);
            replicator_pool.run();
        });

        {
            let mut guard = self.node.lock().unwrap();
            guard.init(replicator_jh);
//            guard.run().expect("Failed to run server");
        }

        self.node.clone()
    }

    pub fn shutdown(&mut self) {
//        self.node.lock().unwrap().shutdown();
    }
}