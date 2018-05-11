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
use utils::{AM, AWM};

pub struct PaymonCoin {
    pub node: AM<Node>,
    pub config: Configuration,
    pmnc_tx: Sender<()>,
    replicator_rx: Option<Receiver<()>>,
}

impl PaymonCoin {
    pub fn new(config: Configuration) -> Self {
        let mut hive = Arc::new(Mutex::new(Hive::new()));

        // used for shutdown replicator pool
        let (replicator_tx, replicator_rx) = channel::<()>();
        let (pmnc_tx, pmnc_rx) = channel::<()>();
//        let (tx, rx) = channel();

        let mut tips_vm = TipsViewModel::new();
        let mut node = Arc::new(Mutex::new(Node::new(Arc::downgrade(&hive.clone()), &config,
                                                     replicator_tx, pmnc_rx)));

        PaymonCoin {
            node,
            pmnc_tx,
            config,
            replicator_rx: Some(replicator_rx),
        }
    }

    pub fn run(&mut self) -> AM<Node> {
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