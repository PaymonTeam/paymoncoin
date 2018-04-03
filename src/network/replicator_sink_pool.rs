use std::sync::{Arc, Weak, Mutex};
use network::node::Node;
use model::config::{Configuration, ConfigurationSettings};
use std::sync::mpsc::Receiver;
use network::replicator_sink::ReplicatorSink;
use std::thread;
use std::time::Duration;
use network::neighbor::Neighbor;

pub struct ReplicatorSinkPool {
    replicator_rx: Receiver<bool>,  // needed to determine shutdown
    replicator_source_rx: Receiver<()>, // needed to receive new connections
    node: Weak<Mutex<Node>>,
}

impl ReplicatorSinkPool {
    pub fn new(config: &Configuration, node: Weak<Mutex<Node>>, replicator_rx: Receiver<bool>, replicator_source_rx: Receiver<()>) -> Self {
        ReplicatorSinkPool {
            replicator_rx,
            replicator_source_rx,
            node,
        }
    }

    pub fn run(&mut self) {
        thread::sleep(Duration::from_secs(2));
        loop {
            if let Ok(sd) = self.replicator_rx.try_recv() {
                break;
            }

            if let Some(arc) = self.node.upgrade() {
                if let Ok(node) = arc.lock() {
                    for arc2 in &node.neighbors {
                        let mut f = false;
                        if let Ok(n) = arc2.lock() {
                            if !n.has_sink {
                                f = true;
                            }
                        }
                        if f {
                            ReplicatorSinkPool::create_sink(Arc::downgrade(&arc2.clone()));
                        }
                    }
                }
            }

            thread::sleep(Duration::from_secs(1));
        }

        info!("Shutting down replicator sink pool...");
    }

    pub fn create_sink(neighbor: Weak<Mutex<Neighbor>>) {
        {
            let arc = neighbor.upgrade().unwrap();
            let mut guard = arc.lock().unwrap();
            guard.has_sink = true;
            info!("creating sink for IP {:?}", guard.addr);
        }
        thread::spawn(move || ReplicatorSink::new(neighbor).run().expect("fail"));
    }
}