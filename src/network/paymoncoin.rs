use std::{
    sync::{mpsc::{Receiver, Sender, channel}, Arc, Weak, Mutex},
    io::{self, Read},
    env,
    collections::VecDeque,
    thread,
    thread::{Builder, JoinHandle},
    time::Duration,
};

use crate::storage::Hive;
use crate::network::node::*;
use crate::network::replicator_new::ReplicatorNew;
use crate::model::config::{PORT, Configuration, ConfigurationSettings};
use crate::model::config;
use crate::model::TipsViewModel;
use crate::model::transaction::Address;
use crate::utils::{AM, AWM};
use crate::model::*;
use std::time;
use std::str::FromStr;
use crossbeam::scope;

pub struct PaymonCoin {
    pub hive: AM<Hive>,
    pub node: AM<Node>,
    pub config: Configuration,
    pmnc_tx: Sender<()>,
    replicator_rx: Option<Receiver<()>>,
    pub coordinator: Address,
    pub tips_vm: AM<TipsViewModel>,
    pub transaction_requester: AM<TransactionRequester>,
    pub transaction_validator: AM<TransactionValidator>,
    pub milestone: AM<Milestone>,
    pub ledger_validator: AM<LedgerValidator>,
    pub tips_manager: AM<TipsManager>
}

impl PaymonCoin {
    pub fn new(config: Configuration) -> Self {
        let snapshot_timestamp = 1526912331;
        let coordinator = Address::from_str("P65DC4FEED4819C2910FA2DFC107399B7437ABAE2E7").unwrap();
        let num_keys_milestone = 22;
        let milestone_start_index = 1;

        let snapshot = Snapshot::init("db/snapshot.dat".to_string(), "".to_string()).expect("Can't \
        load snapshot");
        let hive = Arc::new(Mutex::new(Hive::new()));

        // used for shutdown replicator pool
        let (replicator_tx, replicator_rx) = channel::<()>();
        let (pmnc_tx, pmnc_rx) = channel::<()>();
//        let (tx, rx) = channel();

        let tips_vm: AM<TipsViewModel> = make_am!(TipsViewModel::new());
        let transaction_requester: AM<TransactionRequester> = make_am!(TransactionRequester::new(hive.clone(), 0.001));
        let transaction_validator = TransactionValidator::new(hive.clone(), tips_vm.clone(),
                                                                  snapshot_timestamp, transaction_requester.clone());
        let milestone = Milestone::new(hive.clone(), coordinator.clone(), snapshot,
                                           transaction_validator.clone(), true,
                                           num_keys_milestone, milestone_start_index, true);
        debug!("s3={}", config.get_string(ConfigurationSettings::Neighbors).unwrap());

        let node = Arc::new(Mutex::new(Node::new(Arc::downgrade(&hive.clone()), config.clone(),
                                                     replicator_tx, pmnc_rx,
                                                     transaction_validator.clone(),
                                                     transaction_requester.clone(),
                                                     tips_vm.clone(),
                                                     milestone.clone())));
        let ledger_validator: AM<LedgerValidator> = make_am!(LedgerValidator::new(hive.clone(),
                                                                              milestone.clone(),
                                                        transaction_requester.clone()));
        let tips_manager = TipsManager::new(hive.clone(), milestone.clone(), ledger_validator
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
        Milestone::init(self.milestone.clone(), self.ledger_validator.clone());
        if let Ok(mut tv) = self.transaction_validator.lock() {
            let test_net = self.config.get_bool(ConfigurationSettings::TestNet).unwrap_or(false);
            let wmw = if test_net {
                self.config.get_int(ConfigurationSettings::MainNetMWM).unwrap_or(9)
            } else {
                self.config.get_int(ConfigurationSettings::TestNetMWM).unwrap_or(8)
            } as u32;
            tv.init(test_net, wmw);
        }
        TipsManager::init(self.tips_manager.clone());

        let node_copy = self.node.clone();
        let config_copy = self.config.clone();
        let _replicator_rx = self.replicator_rx.take().unwrap();

        let replicator_jh = thread::spawn(move || {
            let mut replicator_pool = ReplicatorNew::new(&config_copy, Arc::downgrade(&node_copy));
            replicator_pool.run();
        });

        {
            let mut node = self.node.lock().unwrap();
            node.init(replicator_jh);
        }

        self.node.clone()
    }

    pub fn shutdown(&mut self) {
//        self.node.lock().unwrap().shutdown();
        if let Ok(mut m) = self.milestone.lock() {
            m.shutdown();
        }
        if let Ok(mut tm) = self.tips_manager.lock() {
            tm.shutdown();
        }
        if let Ok(mut n) = self.node.lock() {
            n.shutdown();
        }
        if let Ok(mut tv) = self.transaction_validator.lock() {
            tv.shutdown();
        }
//        if let Ok(mut h) = self.hive.lock() {
//            h.shutdown();
//        }
    }
}