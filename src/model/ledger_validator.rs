use storage::Hive;
use model::{Milestone, TransactionRequester};
use std::collections::{HashSet, HashMap, LinkedList};
use model::transaction::*;
use utils::*;
use model::transaction_validator::TransactionError;
use std::i64;

pub struct LedgerValidator {
    hive: AM<Hive>,
    milestone: AM<Milestone>,
    transaction_requester: AM<TransactionRequester>,
    number_of_confirmed_transactions: usize,
}

impl LedgerValidator {
    pub fn new(hive: AM<Hive>, milestone: AM<Milestone>, transaction_requester: AM<TransactionRequester>) -> Self {
        LedgerValidator {
            hive, milestone, transaction_requester,
            number_of_confirmed_transactions: 0
        }
    }

    pub fn get_latest_diff(&mut self, visited_non_milestone_subtangle_hashes: &mut HashSet<Hash>,
                           tip: Option<Hash>,
                           latest_snapshot_index: u32, milestone: bool) -> Result<Option<HashMap<Address, i64>>, TransactionError> {
        let mut state = HashMap::<Address, i64>::new();
        let mut number_of_analyzed_transactions = 0;
        let mut counted_tx = HashSet::<Hash>::new();
        counted_tx.insert(HASH_NULL);

        visited_non_milestone_subtangle_hashes.insert(HASH_NULL);

        let mut non_analyzed_transactions = LinkedList::<Hash>::new();
        if let Some(tip) = tip {
            non_analyzed_transactions.push_back(tip.clone());
        }

        while let Some(hash) = non_analyzed_transactions.pop_front() {
            if visited_non_milestone_subtangle_hashes.insert(hash.clone()) {
                let mut transaction;

                if let Ok(mut hive) = self.hive.lock() {
                    transaction = match hive.storage_load_transaction(&hash) {
                        Some(t) => t,
                        None => return Err(TransactionError::InvalidHash)
                    };
                } else {
                    panic!("broken hive mutex");
                }

                if transaction.object.snapshot == 0 || transaction.object.snapshot >
                    latest_snapshot_index {

                    number_of_analyzed_transactions += 1;
                    if transaction.get_type() == TransactionType::HashOnly {
                        if let Ok(mut tr) = self.transaction_requester.lock() {
                            tr.request_transaction(transaction.get_hash(), milestone);
                            return Ok(None);
                        }
                    } else {
                        if transaction.object.value != 0 && counted_tx.insert(transaction.get_hash()) {
                            let address = transaction.object.address;
                            let value = match state.get(&address) {
                                Some(v) => {
                                    let (v, b) = (transaction.object.value as i64).overflowing_add(*v);
                                    if b {
                                        return Err(TransactionError::InvalidData);
                                    }
                                    v
                                }
                                None => transaction.object.value as i64
                            };
                            state.insert(address.clone(), value);
                        }

                        non_analyzed_transactions.push_back(transaction.get_trunk_transaction_hash());
                        non_analyzed_transactions.push_back(transaction.get_branch_transaction_hash());
                    }
                }
            }
        }

        debug!("analyzed txs = {}", number_of_analyzed_transactions);
        if tip.is_none() {
            self.number_of_confirmed_transactions = number_of_analyzed_transactions;
        }
        debug!("confirmed txs = {}", self.number_of_confirmed_transactions);

        Ok(Some(state))
    }

    fn update_snapshot_milestone(&mut self, hash: Hash, index: u32) -> Result<(),
        TransactionError> {
        let mut visited_hashes = HashSet::<Hash>::new();
        let mut non_analyzed_transactions = LinkedList::<Hash>::new();
        non_analyzed_transactions.push_back(hash.clone());

        while let Some(na_hash) = non_analyzed_transactions.pop_front() {
            if visited_hashes.insert(na_hash.clone()) {
                let mut transaction;

                if let Ok(mut hive) = self.hive.lock() {
                    transaction = match hive.storage_load_transaction(&na_hash) {
                        Some(t) => t,
                        None => return Err(TransactionError::InvalidHash)
                    };

                    if transaction.object.snapshot == 0 {
                        transaction.object.snapshot = index;
                        hive.update_transaction(&mut transaction);
                    }
                } else {
                    panic!("broken hive mutex");
                }

                non_analyzed_transactions.push_back(transaction.get_trunk_transaction_hash());
                non_analyzed_transactions.push_back(transaction.get_branch_transaction_hash());
            }
        }

        Ok(())
    }

    pub fn init(&mut self) {
    // TODO
    }

    pub fn update_snapshot(&mut self, milestone: &Milestone) -> Result<bool, TransactionError> {
        // TODO
//        let mut transaction;
//        if let Ok(mut hive) = self.hive.lock() {
//            transaction = match hive.storage_load_transaction(&na_hash) {
//                Some(t) => t,
//                None => return Err(TransactionError::InvalidHash)
//            };
//        } else {
//            panic!("broken hive mutex");
//        }
//
//        let transaction_snapshot_index = transaction.object.snapshot;
//        let mut has_snapshot = transaction_snapshot_index != 0;
//        if !has_snapshot {
//            let tail = transaction.get_hash();
////            let mut current_state = self.get_latest_diff(HashSet::new(), Some(tail), self
////                .milestone.)
//
//        }
//        Ok(has_snapshot)
        Ok(true)
    }

    // TODO
//    fn build_snapshot() -> MilestoneObject {
//
//    }

    pub fn update_diff(&mut self, approved_hashes: HashSet<Hash>, diff: HashMap<Address, i64>, tip:
    Hash) -> Result<bool, TransactionError> {
        // TODO
//        if let Ok(mut hive) = self.hive.lock() {
//            match hive.storage_load_transaction(&na_hash) {
//                Some(t) => {
//                    if t.is_solid() {
//                        return false;
//                    }
//                },
//                None => return Err(TransactionError::InvalidHash)
//            };
//        }

        let is_consistent = false; // TODO
        if is_consistent {
        }
        Ok(is_consistent)
    }
}