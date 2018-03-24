use std::thread;

use network::replicator_sink_pool::ReplicatorSinkPool;
use network::replicator_source_pool::ReplicatorSourcePool;

struct Replicator {
    sink: ReplicatorSinkPool,
    source: ReplicatorSourcePool,
    node: Weak<Mutex<Node>>,
}

impl Replicator {
    pub fn new(config: &Configuration, node: Weak<Mutex<Node>>) -> Self {
        Replicator {
            sink: ReplicatorSinkPool::new(config,node.clone()),
            source: ReplicatorSourcePool::new(config,node.clone()),
            node
        }
    }

    pub fn run(&mut self) {
//      thread::spawn(|| { self.sink.run(); });
        thread::spawn(|| self.source.run() );
    }
}