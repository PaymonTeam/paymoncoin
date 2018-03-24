extern crate nix;

use std::io::Error;
use super::replicator_source_pool::ReplicatorSourcePool;
use model::config::{Configuration, ConfigurationSettings};
use network::packet::{SerializedBuffer};
use std::sync::{Arc, Weak, Mutex};
use network::neighbor::Neighbor;

extern fn handle_sigint(_:i32) {
    println!("Interrupted!");
    panic!();
}

pub struct Node {
    running: bool,
    receiver: Option<Weak<Mutex<ReplicatorSourcePool>>>,
    neighbors: Vec<Neighbor>
}

impl Node {
    pub fn new(config: &Configuration) -> Node {
        Node {
            running: true,
            receiver: None,
            neighbors: Vec::new(),
        }
    }

    pub fn set_receiver(&mut self, receiver: &Weak<Mutex<ReplicatorSourcePool>>) {
        self.receiver = Some(receiver.clone());
    }

    pub fn run(&mut self) -> Result<(), Error> {
        if let Some(ref s) = self.receiver {
            if let Some(ref s) = s.upgrade() {
                let mut receiver = s.lock().unwrap();
                (*receiver).run();
            }
        }
        return Ok(());
    }

    pub fn on_connection_data_received(&mut self, mut data: SerializedBuffer) {
//        let connection = self.find_connection_by_token(token);
        let length = data.limit();
//
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
        use network::rpc;
        
//        let mut funcs = HashMap::<i32, fn(conn:&mut Connection)->()>::new();

//        funcs.insert(rpc::KeepAlive::SVUID, |conn: &mut Connection| {
//            let keep_alive = rpc::KeepAlive{};
//            conn.send_packet(keep_alive, 1);
//        });

//        if let Some(f) = funcs.get(&svuid) {
//            f(connection);
//        }
    }
}