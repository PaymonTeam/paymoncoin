use std::sync::{Arc, Weak, Mutex};
use network::node::Node;
use model::config::{Configuration, ConfigurationSettings};

pub struct ReplicatorSinkPool {

}

impl ReplicatorSinkPool {
    pub fn new(config: &Configuration, node: Weak<Mutex<Node>>) -> Self {

    }
}