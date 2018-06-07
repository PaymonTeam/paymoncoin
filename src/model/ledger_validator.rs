use storage::Hive;
use model::{Milestone, MilestoneObject, TransactionRequester, Snapshot};
use std::collections::{HashSet, HashMap, LinkedList};
use model::transaction::*;
use model::{StateDiff, StateDiffObject};
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
                           latest_snapshot_index: u32, milestone: bool) -> Result<Option<HashMap<Address, i32>>, TransactionError> {
        let mut state = HashMap::<Address, i32>::new();
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
                // println!("hive lock 9");
                if let Ok(mut hive) = self.hive.lock() {
                    transaction = match hive.storage_load_transaction(&hash) {
                        Some(t) => t,
                        None => {
                            return Err(TransactionError::InvalidHash)
                        }
                    };
                } else {
                    panic!("broken hive mutex");
                }

                println!("transaction.object.snapshot={}", transaction.object.snapshot);
                println!("latest_snapshot_index={}", latest_snapshot_index);
                if transaction.object.snapshot == 0 || transaction.object.snapshot >
                    latest_snapshot_index {

                    number_of_analyzed_transactions += 1;
                    if transaction.get_type() == TransactionType::HashOnly {
                        if let Ok(mut tr) = self.transaction_requester.lock() {
                            tr.request_transaction(transaction.get_hash(), milestone);
                            return Ok(None);
                        }
                    } else {
                        // TODO: check balance
                        if transaction.object.value != 0 && counted_tx.insert(transaction.get_hash()) {
                            let from_address = Address::from_public_key(&transaction.object.signature_pubkey);
                            let address = transaction.object.address;

                            let value = match state.get(&address) {
                                Some(v) => {
                                    let (v, b) = (transaction.object.value as i32).overflowing_add(*v);
                                    if b {
                                        return Err(TransactionError::InvalidData);
                                    }
                                    v
                                }
                                None => transaction.object.value as i32
                            };

                            state.insert(address.clone(), value);

                            let value = match state.get(&from_address) {
                                Some(v) => {
                                    let (v, b) = (*v).overflowing_sub(transaction.object.value as i32);
                                    if b {
                                        return Err(TransactionError::InvalidData);
                                    }
                                    v
                                }
                                None => transaction.object.value as i32
                            };
                            state.insert(from_address.clone(), value);
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

                // println!("hive lock 10");
                if let Ok(mut hive) = self.hive.lock() {
                    transaction = match hive.storage_load_transaction(&na_hash) {
                        Some(t) => t,
                        None => {
                            // println!("hive unlock 10");
                            return Err(TransactionError::InvalidHash)
                        }
                    };

                    if transaction.object.snapshot == 0 {
                        if transaction.object.snapshot != index {
                            transaction.object.snapshot = index;
                            hive.update_transaction(&mut transaction);
                        }
                        info!("new solid tx: {:?}", transaction.get_hash());

                        non_analyzed_transactions.push_back(transaction.get_trunk_transaction_hash());
                        non_analyzed_transactions.push_back(transaction.get_branch_transaction_hash());
                    }
                } else {
                    panic!("broken hive mutex");
                }
                // println!("hive unlock 10");

            }
        }

        Ok(())
    }

    pub fn init(&mut self) -> Result<(), TransactionError> {
        if let Some(latest_consistent_milestone) = self.build_snapshot()? {
            info!("Loaded consistent milestone: {}", latest_consistent_milestone.index);

            if let Ok(mut m) = self.milestone.lock() {
                m.latest_solid_subhive_milestone = latest_consistent_milestone.get_hash();
                m.latest_solid_subhive_milestone_index = latest_consistent_milestone.index;
            }

        }
        Ok(())
    }

    pub fn check_consistency(&mut self, hashes: &Vec<Hash>) -> Result<bool, TransactionError> {
        let mut visited_hashes = HashSet::new();
        let mut diff = HashMap::new();

        for hash in hashes {
            if !self.update_diff(&mut visited_hashes, &mut diff, hash.clone())? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub fn update_snapshot(&mut self, milestone_obj: &MilestoneObject, latest_snapshot: &mut Snapshot) ->
    Result<bool,
        TransactionError> {
        let mut transaction;
        // println!("hive lock 11");
        if let Ok(mut hive) = self.hive.lock() {
            transaction = match hive.storage_load_transaction(&milestone_obj.hash) {
                Some(t) => t,
                None => {
                    // println!("hive unlock 11");
                    return Err(TransactionError::InvalidHash)
                }
            };
        } else {
            panic!("broken hive mutex");
        }
        // println!("hive unlock 11");

        let transaction_snapshot_index = transaction.object.snapshot;
        let mut has_snapshot = transaction_snapshot_index != 0;
        debug!("has_snapshot={}", has_snapshot);
        if !has_snapshot {
            let tail = transaction.get_hash();

            let mut milestone_latest_snapshot_index;

            milestone_latest_snapshot_index = /*milestone.*/latest_snapshot.index;

            let mut current_state;
            match self.get_latest_diff(&mut HashSet::new(), Some(tail),
                                       milestone_latest_snapshot_index,true)? {
                Some(cs) => {
                    current_state = cs;
                }
                None => return Ok(false)
            };

            let mut patched = /*milestone.*/latest_snapshot.patched_diff(current_state.clone());
            has_snapshot = Snapshot::is_consistent(&mut patched);

            debug!("has_snapshot2={}", has_snapshot);
            if has_snapshot {
                self.update_snapshot_milestone(milestone_obj.get_hash(), milestone_obj.index)?;
                let state_diff = StateDiff {
                    state_diff_object: StateDiffObject { state: current_state.clone() },
                    hash: milestone_obj.hash
                };

                if !current_state.is_empty() {
                    // println!("hive lock 12");
                    if let Ok(mut hive) = self.hive.lock() {
                        hive.put_state_diff(&state_diff);
                    }
                    // println!("hive unlock 12");
                }

                latest_snapshot.apply(&current_state, milestone_obj.index);
            }
        }
        Ok(has_snapshot)
    }

    fn build_snapshot(&self) -> Result<Option<MilestoneObject>, TransactionError> {
        let mut consistent_milestone = None;

        // println!("hive lock 13");
        if let Ok(hive) = self.hive.lock() {
            let mut candidate_milestone = hive.storage_first_milestone();
            while let Some(cm) = candidate_milestone {
                if cm.index % 10000 == 0 {
                    info!("Building snapshot... {}", cm.index);
                }

                if hive.exists_state_diff(&cm.get_hash()) {
                    if let Some(state_diff) = hive.storage_load_state_diff(&cm.get_hash()) {
                        if !state_diff.state_diff_object.state.is_empty() {
                            if let Ok(mut milestone) = self.milestone.lock() {
                                if Snapshot::is_consistent(&mut milestone.latest_snapshot
                                    .patched_diff(state_diff.state_diff_object.state.clone())) {
                                    milestone.latest_snapshot.apply(&state_diff.state_diff_object.state, cm.index);
                                    consistent_milestone = Some(cm.clone());
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }
                candidate_milestone = hive.storage_next_milestone(cm.index);
            }
        }
        Ok(consistent_milestone)
    }

    pub fn update_diff(&mut self, approved_hashes: &mut HashSet<Hash>, diff: &mut HashMap<Address, i64>, tip: Hash) -> Result<bool, TransactionError> {
        if let Ok(mut hive) = self.hive.lock() {
            match hive.storage_load_transaction(&tip) {
                Some(t) => {
                    if !t.is_solid() {
                        return Ok(false);
                    }
                },
                None => {
                    return Err(TransactionError::InvalidHash)
                }
            };
        }

        if approved_hashes.contains(&tip) {
            return Ok(true);
        }

        let mut visited_hashes = approved_hashes.clone();

        let milestone_latest_snapshot_index = match self.milestone.lock() {
            Ok(milestone) => milestone.latest_snapshot.index,
            Err(_) => panic!("broken milestone mutex")
        };

        let mut current_state = match self.get_latest_diff(&mut visited_hashes, Some(tip.clone()),
                                   milestone_latest_snapshot_index, false)? {
            Some(mut cs) => cs,
            None => return Ok(false)
        };
        println!("current_state={:?}", current_state);

        let is_consistent;
        if let Ok(mut milestone) = self.milestone.lock() {
            diff.iter().for_each(|(k, v)| {
                let new_value = match current_state.get(k) {
                    Some(old_value) => *v as i32 + *old_value,
                    None => v.clone() as i32
                };
                if current_state.get(k).is_none() {
                    current_state.insert(k.clone(), new_value);
                }
            });

            is_consistent = Snapshot::is_consistent(&mut milestone.latest_snapshot.patched_diff(current_state.clone()));
        } else {
            panic!("broken milestone mutex");
        }

        if is_consistent {
            for (k, v) in &current_state {
                diff.insert(k.clone(), v.clone() as i64);
            }
            for h in &visited_hashes {
                approved_hashes.insert(h.clone());
            }
        }
        Ok(is_consistent)
    }
}