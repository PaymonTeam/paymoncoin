extern crate linked_hash_set;

use utils::{AM, AWM};
use storage::Hive;
use model::{TipsViewModel, Transaction};
use model::transaction::*;
use std::time::Duration;
use std::thread::JoinHandle;
use std::thread;
use std::time;
use std::sync::{Arc, Mutex};
use std::collections::{HashSet, LinkedList};
use std::sync::atomic::{AtomicBool, Ordering};
use self::linked_hash_set::LinkedHashSet;

// TODO: make Mutex
pub static mut SNAPSHOT_TIMESTAMP: Duration = Duration::from_secs(0);
pub static MAX_TIMESTAMP_FUTURE: Duration = Duration::from_secs(2 * 60 * 60);

pub struct TransactionValidator {
    hive: AM<Hive>,
    tips_view_model: AM<TipsViewModel>,
    min_weight_magnitude: u32,
//    snapshot_timestamp: Duration,
//    max_timestamp_future: Duration,
    propagation_thread: Option<JoinHandle<()>>,
    use_first: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    new_solid_transaction_list_one: AM<LinkedHashSet<Hash>>,
    new_solid_transaction_list_two: AM<LinkedHashSet<Hash>>,
//    transaction_requester: AM<TransactionRequester>,
}

pub fn has_invalid_timestamp(transaction: &Transaction) -> bool {
//    let max_future_ts: Duration = time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap() +
//        MAX_TIMESTAMP_FUTURE;
//
//    if transaction.object.attachment_timestamp == 0 {
//        unsafe {
//            return transaction.object.timestamp < SNAPSHOT_TIMESTAMP.as_secs() && transaction.get_hash() ==
//                HASH_NULL || transaction.object.timestamp > max_future_ts.as_secs();
//        }
//    }
//    unsafe {
//        return transaction.object.attachment_timestamp < (SNAPSHOT_TIMESTAMP.as_secs()*1000u64 +
//            SNAPSHOT_TIMESTAMP.subsec_millis() as u64) || transaction.object.attachment_timestamp >
//            max_future_ts.as_secs()*1000u64 + max_future_ts.subsec_millis() as u64;
//    }
    true
}

pub enum TransactionValidationError {
    InvalidTimestamp,
    InvalidHash,
    InvalidAddress,
    InvalidData
}

pub fn validate(transaction: &mut Transaction, mwm: u32) -> Result<(), TransactionValidationError> {
    if has_invalid_timestamp(transaction) {
        return Err(TransactionValidationError::InvalidTimestamp);
    }

    if !validate_transaction(transaction, mwm) {
        return Err(TransactionValidationError::InvalidData);
    }

    if (transaction.weight_magnitude as u32) < mwm {
        return Err(TransactionValidationError::InvalidHash);
    }

    if !transaction.object.address.verify() || (transaction.object.value != 0 && transaction.object.address.is_null()) {
        return Err(TransactionValidationError::InvalidAddress);
    }

    Ok(())
}

fn quiet_quick_set_solid(transaction: &mut Transaction) -> bool {
    match quick_set_solid(transaction) {
        Ok(b) => b,
        Err(_) => false
    }
}

fn quick_set_solid(transaction: &mut Transaction) -> Result<bool, TransactionValidationError> {
    if !transaction.is_solid() {
        let mut solid = true;
        // TODO
    }

    Ok(false)
}

impl TransactionValidator {
    pub fn new(hive: AM<Hive>, tips_view_model: AM<TipsViewModel>, snapshot_timestamp:
    Duration) -> Self {
        unsafe {
            SNAPSHOT_TIMESTAMP = snapshot_timestamp;
        }

        TransactionValidator {
            hive,
            tips_view_model,
//            snapshot_timestamp,

            min_weight_magnitude: 9,
            propagation_thread: None,
            use_first: Arc::new(AtomicBool::new(true)),
            running: Arc::new(AtomicBool::new(true)),
            new_solid_transaction_list_one: make_am!(LinkedHashSet::new()),
            new_solid_transaction_list_two: make_am!(LinkedHashSet::new()),
        }
    }

    pub fn init(&mut self, testnet: bool, mwm: u32) {
        self.min_weight_magnitude = mwm;

        if !testnet && self.min_weight_magnitude < 18 {
            self.min_weight_magnitude = 18;
        }

        let hive_clone = self.hive.clone();
        let running_clone = self.running.clone();
        let use_first_clone = self.use_first.clone();
        let list1 = self.new_solid_transaction_list_one.clone();
        let list2 = self.new_solid_transaction_list_one.clone();

        self.propagation_thread = Some(thread::spawn(move || {
            TransactionValidator::solid_transactions_propagation(hive_clone, running_clone,
                                                                 use_first_clone, list1, list2);
        }));
    }

    pub fn shutdown(&mut self) {
        self.running.store(false, Ordering::SeqCst);
//        if let Some(ref jh) = self.propagation_thread {
//            jh.join();
//        }
        if let Some(jh) = self.propagation_thread.take() {
            jh.join();
        }
    }

    pub fn check_solidity(&self, hash: Hash, milestone: bool) -> Result<bool,
        TransactionValidationError> {
        if let Ok(mut hive) = self.hive.lock() {
            match hive.storage_load_transaction(&hash) {
                Some(t) => return Ok(t.is_solid()),
                None => return Err(TransactionValidationError::InvalidHash)
            };
        }

        let mut analyzed_hashes = HashSet::<Hash>::new();
        analyzed_hashes.insert(HASH_NULL);
        let mut solid = true;
        let mut non_analyzed_hashes = LinkedList::<Hash>::new();
        non_analyzed_hashes.push_back(hash.clone());

        while let Some(ref hash_pointer) = non_analyzed_hashes.pop_front() {
            if analyzed_hashes.insert(hash_pointer.clone()) {
                if let Ok(mut hive) = self.hive.lock() {
                    match hive.storage_load_transaction(&hash_pointer) {
                        Some(t) => {
                            if !t.is_solid() {
                                if t.object.data_type == TransactionType::HashOnly &&
                                    *hash_pointer != HASH_NULL {
                                    // TODO
//                                    self.transaction_requester.request_transaction
//                                          (hash_pointer, milestone);
                                    solid = false;
                                    break;
                                } else {
                                    non_analyzed_hashes.push_back(t.object.trunk_transaction);
                                    non_analyzed_hashes.push_back(t.object.branch_transaction);
                                }
                            }
                        }
                        None => return Err(TransactionValidationError::InvalidHash)
                    };
                }
            }
        }

        if solid {
            if let Ok(hive) = self.hive.lock() {
                // TODO
//                hive.update_solid_transactions(analyzed_hashes);
            }
        }

        analyzed_hashes.clear();
        Ok(solid)
    }

    pub fn add_solid_transaction(&mut self, hash: Hash) {
        if self.use_first.load(Ordering::Relaxed) {
            if let Ok(mut list) = self.new_solid_transaction_list_one.lock() {
                list.insert(hash);
            }
        } else {
            if let Ok(mut list) = self.new_solid_transaction_list_two.lock() {
                list.insert(hash);
            }
        }
    }

    fn solid_transactions_propagation(hive: AM<Hive>, running: Arc<AtomicBool>, use_first:
    Arc<AtomicBool>, solid_transaction_list_one: AM<LinkedHashSet<Hash>>, solid_transaction_list_two: AM<LinkedHashSet<Hash>>) {
        while running.load(Ordering::SeqCst) {
            let mut new_solid_hashes = HashSet::<Hash>::new();
            use_first.store(!use_first.load(Ordering::SeqCst), Ordering::SeqCst);

            if use_first.load(Ordering::SeqCst) {
                if let Ok(lst) = solid_transaction_list_two.lock() {
                    for hash in lst.iter() {
                        new_solid_hashes.insert(hash.clone());
                    }
                }
            } else {
                if let Ok(lst) = solid_transaction_list_one.lock() {
                    for hash in lst.iter() {
                        new_solid_hashes.insert(hash.clone());
                    }
                }
            }

            let mut it = new_solid_hashes.iter();
            while let Some(hash) = it.next() {
                if running.load(Ordering::SeqCst) {
                    if let Ok(mut hive) = hive.lock() {
                        if let Some(transaction) = hive.storage_load_transaction(&hash) {
                            if let Some(approvers) = hive.storage_load_approvee(&hash) {
                                for h in approvers {
                                    if let Some(ref mut tx) = hive.storage_load_transaction(&hash) {
                                        if quiet_quick_set_solid(tx) {
                                            // TODO:
//                                            hive.update(tx);
                                        } else if transaction.is_solid() {
                                            if use_first.load(Ordering::Relaxed) {
                                                if let Ok(mut list) = solid_transaction_list_one.lock() {
                                                    list.insert(hash.clone());
                                                }
                                            } else {
                                                if let Ok(mut list) = solid_transaction_list_two.lock() {
                                                    list.insert(hash.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if use_first.load(Ordering::Relaxed) {
                if let Ok(mut list) = solid_transaction_list_two.lock() { list.clear(); }
            } else {
                if let Ok(mut list) = solid_transaction_list_one.lock() { list.clear(); }
            }

            thread::sleep(Duration::from_millis(500));
        }
    }



    pub fn get_min_weight_magnitude(&self) -> u32 {
        self.min_weight_magnitude
    }
}