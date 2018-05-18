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
use model::TransactionRequester;

// TODO: make Mutex
pub static mut SNAPSHOT_TIMESTAMP: u64 = 0; //Duration = Duration::from_secs(0);
pub static mut SNAPSHOT_TIMESTAMP_MS: u64 = 0; //Duration = Duration::from_secs(0);
pub const MAX_TIMESTAMP_FUTURE: u64 = 2 * 60 * 60; //Duration = Duration::from_secs(2 * 60 * 60);
pub const MAX_TIMESTAMP_FUTURE_MS: u64 = MAX_TIMESTAMP_FUTURE * 1000;

pub struct TransactionValidator {
    hive: AM<Hive>,
    tips_view_model: AM<TipsViewModel>,
    min_weight_magnitude: u32,
    propagation_thread: Option<JoinHandle<()>>,
    use_first: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    new_solid_transaction_list_one: AM<LinkedHashSet<Hash>>,
    new_solid_transaction_list_two: AM<LinkedHashSet<Hash>>,
    transaction_requester: AM<TransactionRequester>,
}

pub fn has_invalid_timestamp(transaction: &Transaction) -> bool {
    // TODO: make with millis
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

    unsafe {
        return transaction.object.timestamp < SNAPSHOT_TIMESTAMP && transaction.get_hash() ==
            HASH_NULL || transaction.object.timestamp > MAX_TIMESTAMP_FUTURE;
    }
}

pub enum TransactionError {
    InvalidTimestamp,
    InvalidHash,
    InvalidAddress,
    InvalidData
}

pub fn validate(transaction: &mut Transaction, mwm: u32) -> Result<(), TransactionError> {
    if has_invalid_timestamp(transaction) {
        return Err(TransactionError::InvalidTimestamp);
    }

    if !validate_transaction(transaction, mwm) {
        return Err(TransactionError::InvalidData);
    }

    if (transaction.weight_magnitude as u32) < mwm {
        return Err(TransactionError::InvalidHash);
    }

    if !transaction.object.address.verify() || (transaction.object.value != 0 && transaction.object.address.is_null()) {
        return Err(TransactionError::InvalidAddress);
    }

    Ok(())
}

impl TransactionValidator {
    pub fn new(hive: AM<Hive>, tips_view_model: AM<TipsViewModel>, snapshot_timestamp:
    u64, transaction_requester: AM<TransactionRequester>) -> AM<Self> {
        unsafe {
            SNAPSHOT_TIMESTAMP = snapshot_timestamp;
            SNAPSHOT_TIMESTAMP_MS = snapshot_timestamp * 1000;
        }

        let tv = TransactionValidator {
            hive,
            tips_view_model,
//            snapshot_timestamp,
            transaction_requester,
            min_weight_magnitude: 9,
            propagation_thread: None,
            use_first: Arc::new(AtomicBool::new(true)),
            running: Arc::new(AtomicBool::new(true)),
            new_solid_transaction_list_one: make_am!(LinkedHashSet::new()),
            new_solid_transaction_list_two: make_am!(LinkedHashSet::new()),
        };

        let transaction_validator: AM<TransactionValidator> = make_am!(tv);

        let transaction_validator_clone = Arc::downgrade(&transaction_validator.clone());

        transaction_validator.lock().unwrap().propagation_thread = Some(thread::spawn(move || {
            TransactionValidator::solid_transactions_propagation(transaction_validator_clone);
        }));

        transaction_validator
    }

    pub fn init(&mut self, testnet: bool, mwm: u32) {
        self.min_weight_magnitude = mwm;

        if !testnet && self.min_weight_magnitude < 18 {
            self.min_weight_magnitude = 18;
        }
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
        TransactionError> {
        if let Ok(mut hive) = self.hive.lock() {
            match hive.storage_load_transaction(&hash) {
                Some(t) => return Ok(t.is_solid()),
                None => return Err(TransactionError::InvalidHash)
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

                                    if let Ok(mut tr) = self.transaction_requester.lock() {
                                        tr.request_transaction(hash_pointer.clone(), milestone);
                                    } else {
                                        return Err(TransactionError::InvalidData);
                                    }

                                    solid = false;
                                    break;
                                } else {
                                    non_analyzed_hashes.push_back(t.object.trunk_transaction);
                                    non_analyzed_hashes.push_back(t.object.branch_transaction);
                                }
                            }
                        }
                        None => return Err(TransactionError::InvalidHash)
                    };
                }
            }
        }

        if solid {
            if let Ok(mut hive) = self.hive.lock() {
                hive.update_solid_transactions(&analyzed_hashes);
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

    fn solid_transactions_propagation(transaction_validator: AWM<TransactionValidator>) {
        let hive;
        let running;
        let use_first;
        let solid_transaction_list_one;
        let solid_transaction_list_two;

        if let Some(arc) = transaction_validator.upgrade() {
            if let Ok(tv) = arc.lock() {
                hive = tv.hive.clone();
                running = tv.running.clone();
                use_first = tv.use_first.clone();
                solid_transaction_list_one = tv.new_solid_transaction_list_one.clone();
                solid_transaction_list_two = tv.new_solid_transaction_list_two.clone();
            } else {
                panic!("broken transaction_validator mutex");
            }
        } else {
            panic!("transaction_validator is null");
        }

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
                                        if let Some(arc) = transaction_validator.upgrade() {
                                            if let Ok(mut tv) = arc.lock() {
                                                if tv.quiet_quick_set_solid(tx) {
                                                    hive.update_transaction(tx);
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
                                            } else {
                                                panic!("broken transaction_validator mutex");
                                            }
                                        } else {
                                            panic!("transaction_validator is null");
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

    fn quiet_quick_set_solid(&mut self, transaction: &mut Transaction) -> bool {
        match self.quick_set_solid(transaction) {
            Ok(b) => b,
            Err(_) => false
        }
    }

    fn quick_set_solid(&mut self, transaction: &mut Transaction) -> Result<bool,
        TransactionError> {
        if !transaction.is_solid() {
            let mut solid = true;

            let trunk_tx;
            let branch_tx;

            if let Ok(mut hive) = self.hive.lock() {
                if let Some(tx) = hive.storage_load_transaction(&transaction.object
                    .trunk_transaction) {
                    trunk_tx = tx;
                } else {
                    return Err(TransactionError::InvalidHash);
                }

                if let Some(tx) = hive.storage_load_transaction(&transaction.object
                    .branch_transaction) {
                    branch_tx = tx;
                } else {
                    return Err(TransactionError::InvalidHash);
                }
            } else {
                panic!("hive mutex is broken")
            }

            if !self.check_approvee(&trunk_tx) {
                solid = false;
            }

            if !self.check_approvee(&branch_tx) {
                solid = false;
            }

            if solid {
                transaction.update_solidity(solid);
                if let Ok(mut hive) = self.hive.lock() {
                    hive.update_heights(transaction.clone());
                } else {
                    panic!("hive mutex is broken")
                }

                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn update_status(&mut self, transaction: &mut Transaction) -> Result<(),
        TransactionError> {
        if let Ok(mut tr) = self.transaction_requester.lock() {
            tr.clear_transaction_request(transaction.get_hash());
        } else {
            error!("broken transaction_requester mutex");
            return Err(TransactionError::InvalidData);
        }

        if let Ok(mut hive) = self.hive.lock() {
            if let Ok(mut tvm) = self.tips_view_model.lock() {
                match hive.storage_load_approvee(&transaction.get_hash()) {
                    Some(vec) => {
                        if vec.len() == 0 {
                            tvm.add_tip(transaction.get_hash());
                        }
                    },
                    None => return Err(TransactionError::InvalidData)
                };

                tvm.remove_tip(&transaction.get_trunk_transaction_hash());
                tvm.remove_tip(&transaction.get_branch_transaction_hash());
            }
        }

        if self.quick_set_solid(transaction)? {
            self.add_solid_transaction(transaction.get_hash());
        }

        Ok(())
    }

    fn check_approvee(&mut self, approvee: &Transaction) -> bool {
        if approvee.get_type() == TransactionType::HashOnly {
            if let Ok(mut tr) = self.transaction_requester.lock() {
                tr.request_transaction(approvee.get_hash(), false);
            }
            return false;
        }

        if approvee.get_hash() == HASH_NULL {
            return true;
        }

        return approvee.is_solid();
    }

    pub fn get_min_weight_magnitude(&self) -> u32 {
        self.min_weight_magnitude
    }
}