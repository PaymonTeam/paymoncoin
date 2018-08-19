extern crate linked_hash_set;

use self::linked_hash_set::LinkedHashSet;
use model::transaction::*;
use std::time::{Duration, SystemTime};
use std::time;
use rand::{Rng, thread_rng};
use storage::Hive;
use utils::*;

const MAX_TX_REQ_QUEUE_SIZE: usize = 10000usize;

pub struct TransactionRequester {
    milestone_transactions_to_request: LinkedHashSet<Hash>,
    pub transactions_to_request: LinkedHashSet<Hash>,
    last_time: SystemTime,
    p_remove_request: f64,
    hive: AM<Hive>,
}

impl TransactionRequester {
    pub fn new(hive: AM<Hive>, p_remove_request: f64) -> Self {
        TransactionRequester {
            milestone_transactions_to_request: LinkedHashSet::new(),
            transactions_to_request: LinkedHashSet::new(),
            last_time: SystemTime::now(),
            p_remove_request,
            hive,
        }
    }

    pub fn get_requested_transactions(&self) -> Vec<Hash> {
        let mut arr = vec![];

        arr.append(&mut self.milestone_transactions_to_request.iter().cloned().collect::<Vec<Hash>>());
        arr.append(&mut self.transactions_to_request.iter().cloned().collect::<Vec<Hash>>());

        arr
    }

    pub fn num_transactions_to_request(&self) -> usize {
        self.milestone_transactions_to_request.len() + self.transactions_to_request.len()
    }

    pub fn clear_transaction_request(&mut self, hash: Hash) -> bool {
        let milestone = self.milestone_transactions_to_request.remove(&hash);
        let normal = self.milestone_transactions_to_request.remove(&hash);

        milestone || normal
    }

    pub fn request_transaction(&mut self, hash: Hash, milestone: bool) {
        debug!("Added tx to request {:?}", hash);
        if let Ok(hive) = self.hive.lock() {
            if hash != HASH_NULL && !hive.exists_transaction(hash.clone()) {
                if milestone {
                    self.transactions_to_request.remove(&hash);
                    self.milestone_transactions_to_request.insert(hash);
                } else {
                    if !self.milestone_transactions_to_request.contains(&hash) && !self
                        .transactions_to_request_is_full() {
                        self.transactions_to_request.insert(hash);
                    }
                }
            }
        }
    }

    pub fn transactions_to_request_is_full(&self) -> bool {
        self.transactions_to_request.len() >= MAX_TX_REQ_QUEUE_SIZE
    }

    pub fn poll_transaction_to_request(&mut self, milestone: bool) -> Option<Hash> {
        let mut hash: Option<Hash> = None;

        {
            let mut request_set: &mut LinkedHashSet<Hash>;

            if milestone {
                request_set = &mut self.milestone_transactions_to_request;
                if request_set.is_empty() {
                    request_set = &mut self.transactions_to_request;
                }
            } else {
                request_set = &mut self.transactions_to_request;
                if request_set.is_empty() {
                    request_set = &mut self.milestone_transactions_to_request;
                }
            }

            let mut to_remove = Vec::<Hash>::new();
            for h in request_set.iter() {
                to_remove.push(h.clone());
                hash = Some(h.clone());

                // println!("hive lock 23");
                if let Ok(hive) = self.hive.lock() {
                    if hive.exists_transaction(hash.unwrap().clone()) {
                        info!("Removing existing tx from request list: {:?}", hash.unwrap());
                    } else {
//                    if !self.transactions_to_request_is_full() {
//                        request_set.insert(hash.unwrap().clone());
//                    }
                        // println!("hive unlock 23");
                        break;
                    }
                }
                // println!("hive unlock 23");
            }

            for h in &to_remove {
                request_set.remove(h);
            }
        }

        if !milestone && thread_rng().gen_range(0.0, 1.0) < self.p_remove_request {
            if let Some(ref hash) = hash {
                self.transactions_to_request.remove(hash);
            }
        }

        let now = SystemTime::now();
        if let Ok(dur) = now.duration_since(self.last_time) {
            if dur.as_secs() > 10 {
                self.last_time = now.clone();
            }
        }

        hash
    }
}
