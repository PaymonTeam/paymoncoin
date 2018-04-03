use std::thread;

use network::replicator_sink_pool::ReplicatorSinkPool;
use network::replicator_source_pool::ReplicatorSourcePool;
use std::sync::{Arc, Weak, Mutex};
use network::node::Node;
use model::config::{Configuration, ConfigurationSettings};
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;

pub struct Replicator {
    replicator_sink_tx: Option<Sender<bool>>,
    replicator_source_tx: Option<Sender<bool>>,
    node: Weak<Mutex<Node>>,
    config: Configuration,
}

impl Replicator {
    pub fn new(config: &Configuration, node: Weak<Mutex<Node>>) -> Self {
        Replicator {
            replicator_sink_tx: None,
            replicator_source_tx: None,
            node,
            config: config.clone(),
        }
    }

    pub fn run(&mut self) {
        let (replicator_source_tx, replicator_source_rx) = channel::<bool>();
        let (replicator_sink_tx, replicator_sink_rx) = channel::<bool>();
        self.replicator_source_tx = Some(replicator_source_tx);
        self.replicator_sink_tx = Some(replicator_sink_tx);

        // replicator source pool -> replicator sink pool
        let (source_sink_tx, source_sink_rx) = channel();

        let node = self.node.clone();
        let config = self.config.clone();
        thread::spawn(move || { ReplicatorSourcePool::new(&config, node, replicator_source_rx, source_sink_tx).run(); });

        let node = self.node.clone();
        let config = self.config.clone();
        thread::spawn(move || { ReplicatorSinkPool::new(&config, node,replicator_sink_rx, source_sink_rx).run(); });
    }

    pub fn shutdown(&self) {
        if let Some(ref tx) = self.replicator_sink_tx {
            tx.send(true);
        };
        if let Some(ref tx) = self.replicator_source_tx {
            tx.send(true);
        };
    }
}