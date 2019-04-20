use crate::transaction::transaction::*;
use crate::storage::LedgerValidator;
use crate::storage::snapshot::Snapshot;
use crate::transaction::TransactionValidator;
use crate::transaction::transaction_validator::TransactionError;

use std::collections::{HashSet, HashMap, LinkedList};
use crate::utils::AM;
use crate::storage::Hive;
use std::thread;
use std::time::SystemTime;
use std::cmp::max;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::io;
use std::time::Duration;
use std::thread::JoinHandle;

pub const RESCAN_INTERVAL: i32 = 5000;

pub enum Validity {
    Valid,
    Invalid,
    Incomplete,
}

#[derive(Clone)]
pub struct MilestoneObject {
    pub index: u32,
    pub hash: Hash,
}

impl MilestoneObject {
    pub const SVUID: i32 = 927164361;

    pub fn new(index: u32, hash: Hash) -> Self {
        MilestoneObject {
            index,
            hash,
        }
    }

    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn get_hash(&self) -> Hash {
        self.hash.clone()
    }
}

pub struct Milestone {
    pub hive: AM<Hive>,
    pub ledger_validator: Option<AM<LedgerValidator>>,
    pub latest_snapshot: Snapshot,
    pub coordinator: Address,
    pub transaction_validator: AM<TransactionValidator>,
    pub testnet: bool,
    pub num_of_keys_in_milestone: u32,
    pub milestone_start_index: u32,
    pub accept_any_testnet_coo: bool,
    pub analyzed_milestone_candidates: HashSet<Hash>,
    pub latest_solid_subhive_milestone_index: u32,
    pub latest_milestone_index: u32,

    pub latest_solid_subhive_milestone: Hash,
    pub latest_milestone: Hash,

    pub shutting_down: bool,

    pub latest_milestone_tracker_thread: Option<JoinHandle<()>>,
    pub solid_milestone_tracker_thread: Option<JoinHandle<()>>,
}

impl Milestone {
    pub fn new(hive: AM<Hive>,
               coordinator: Address,
               initial_snapshot: Snapshot,
               transaction_validator: AM<TransactionValidator>,
               testnet: bool,
               num_of_keys_in_milestone: u32,
               milestone_start_index: u32,
               accept_any_testnet_coo: bool) -> AM<Self> {
        let latest_milestone = HASH_NULL;
        let latest_solid_subhive_milestone = latest_milestone.clone();

        let milestone = Milestone {
            hive,
            transaction_validator,
            ledger_validator: None,
            latest_snapshot: initial_snapshot,
            coordinator,
            testnet,
            num_of_keys_in_milestone,
            milestone_start_index,
            accept_any_testnet_coo,
            analyzed_milestone_candidates: HashSet::new(),
            latest_solid_subhive_milestone_index: 0u32,
            latest_milestone_index: 0u32,
            latest_solid_subhive_milestone,
            latest_milestone,
            shutting_down: false,
            latest_milestone_tracker_thread: None,
            solid_milestone_tracker_thread: None,
        };

        make_am!(milestone)
    }

    pub fn init(milestone: AM<Milestone>, ledger_validator: AM<LedgerValidator>) {
        if let Ok(mut milestone) = milestone.lock() {
            milestone.ledger_validator = Some(ledger_validator);
        }

        let ledger_validator_initialized: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

        let ledger_initialized_clone_thread_1 = ledger_validator_initialized.clone();
        let milestone_clone_1 = milestone.clone();
        let ledger_initialized_clone_thread_2 = ledger_validator_initialized.clone();
        let milestone_clone_2 = milestone.clone();

        let latest_milestone_tracker_thread = thread::spawn(move || {
            let hive;
            let coordinator;

            if let Ok(m) = milestone_clone_1.lock() {
                hive = m.hive.clone();
                coordinator = m.coordinator.clone(); //Address::from_public_key(&m.coordinator);
            } else {
                panic!("broken transaction_validator mutex");
            }

            info!("Waiting for Ledger Validator initialization...");
            while !ledger_initialized_clone_thread_1.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(1));
            }

            info!("tracker started");

            let mut shutting_down = false;

            while !shutting_down {
                let scan_time = SystemTime::now();
                let previous_latest_milestone_index;

                if let Ok(m) = milestone_clone_1.lock() {
                    shutting_down = m.shutting_down;
                    previous_latest_milestone_index = m.latest_milestone_index;
                    if shutting_down {
                        break;
                    }
                } else {
                    panic!("broken milestone mutex");
                }

                let hashes: Vec<Hash>;
                let mut hashes_is_ok = false;
                if let Ok(hive) = hive.lock() {
                    hashes = match hive.load_address_transactions(&coordinator) {
                        Some(h) => {
                            hashes_is_ok = true;
                            h
                        }
                        None => {
                            hashes_is_ok = false;
                            Vec::new()
                        }
                    }
                } else {
                    panic!("broken hive mutex");
                }
                // TODO: optimize
                if hashes_is_ok {
                    for hash in hashes {
                        if let Ok(mut milestone) = milestone_clone_1.lock() {
                            if milestone.analyzed_milestone_candidates.insert(hash) {
                                let mut t;
                                if let Ok(hive) = hive.lock() {
                                    t = hive.storage_load_transaction(&hash);
                                } else {
                                    panic!("broken hive mutex");
                                }
                                if let Some(t) = t {
                                    let valid = milestone.validate_milestone(&t, milestone.get_index(&t));
                                    match valid {
                                        Validity::Valid => {
                                            if let Ok(hive) = hive.lock() {
                                                match hive.storage_latest_milestone() {
                                                    Some(milestone_manager) => {
                                                        if milestone_manager.index() > milestone.latest_milestone_index {
                                                            milestone.latest_milestone = milestone_manager.get_hash();
                                                            milestone.latest_milestone_index = milestone_manager.index();
                                                        }
                                                    }
                                                    None => {
                                                        continue;
                                                    }
                                                }
                                            }
                                        }

                                        Validity::Incomplete => {
                                            milestone.analyzed_milestone_candidates.remove(&t.get_hash());
                                        }

                                        Validity::Invalid => {
                                            //nothing to do
                                        }
                                    };
                                }
                            }
                        }
                    }
                }

                if let Ok(milestone) = milestone_clone_1.lock() {
                    if previous_latest_milestone_index != milestone.latest_milestone_index {
                        info!("Latest milestone has changed from #{} to #{}",
                                 previous_latest_milestone_index,
                                 milestone.latest_milestone_index);
                    }
                }

                let now = SystemTime::now();
                thread::sleep_ms(max(1i32, RESCAN_INTERVAL - (now.duration_since(scan_time).unwrap().as_secs() * 1000 + (now.duration_since(scan_time).unwrap().subsec_nanos() / 1000) as u64) as i32) as u32);
            }
        });

        let solid_milestone_tracker_thread = thread::spawn(move || {
            info!("initializing ledger validator...");

            let lv;
            if let Ok(ref mut milestone) = milestone_clone_2.lock() {
                if let Some(ref mut ledger_validator) = milestone.ledger_validator {
                    lv = ledger_validator.clone();
                } else {
                    panic!("ledger validator is null");
                }
            } else {
                panic!("broken milestone mutex");
            }

            if let Ok(ref mut ledger_validator) = lv.lock() {
                ledger_validator.init();
                ledger_initialized_clone_thread_2.store(true, Ordering::SeqCst);
            }

            info!("tracker #2 started");
            let mut shutting_down = false;
            while !shutting_down {
                if let Ok(self_p) = milestone_clone_2.lock() {
                    shutting_down = self_p.shutting_down;
                    if shutting_down {
                        break;
                    }
                }

                let scan_time = SystemTime::now();

                if let Ok(mut milestone) = milestone_clone_2.lock() {
                    let previous_solid_subhive_latest_milestone_index = milestone.latest_solid_subhive_milestone_index;
                    if milestone.latest_solid_subhive_milestone_index < milestone.latest_milestone_index {
                        debug!("latest_milestone_index = {}", milestone.latest_milestone_index);
                        if let Err(e) = milestone.update_latest_solid_subhive_milestone() {
                            error!("Error updating latest solid subhive milestone: {:?}", e);
                        }
                    }

                    if previous_solid_subhive_latest_milestone_index != milestone.latest_solid_subhive_milestone_index {
                        info!("Latest solid subhive milestone has changed from #{} to #{}",
                        previous_solid_subhive_latest_milestone_index,
                                 milestone.latest_solid_subhive_milestone_index);
                    }
                } else {
                    panic!("broken milestone mutex");
                }

                let now = SystemTime::now();
                thread::sleep_ms(max(1i32, RESCAN_INTERVAL - (now.duration_since(scan_time).unwrap().as_secs() * 1000 + (now.duration_since(scan_time).unwrap().subsec_nanos() / 1000) as u64) as i32) as u32);
            }
        });

        if let Ok(mut milestone) = milestone.lock() {
            milestone.latest_milestone_tracker_thread = Some(latest_milestone_tracker_thread);
            milestone.solid_milestone_tracker_thread = Some(solid_milestone_tracker_thread);
        }
    }

    fn update_latest_solid_subhive_milestone(&mut self) -> Result<(), TransactionError> {
        let latest;
        let mut closest_milestone;
        if let Ok(hive) = self.hive.lock() {
            latest = match hive.storage_latest_milestone() {
                Some(m) => m,
                _ => return Ok(())
            };
            closest_milestone = hive.find_closest_next_milestone(self.latest_solid_subhive_milestone_index,
                                             self.testnet, self.milestone_start_index);
        } else {
            panic!("broken hive mutex");
        }

        while let Some(milestone_obj) = closest_milestone.clone() {
            if milestone_obj.index() <= latest.index() && !self.shutting_down {

                if let Ok(tx_v) = self.transaction_validator.lock() {
                    if tx_v.check_solidity(milestone_obj.get_hash(), true)? &&
                        milestone_obj.index() >= self.latest_solid_subhive_milestone_index {

                        // TODO: handle errors
                        let update_snapshot_is_ok;
                        if let Some(ref mut arc) = self.ledger_validator {
                            if let Ok(mut l_v) = arc.lock() {
                                update_snapshot_is_ok = l_v.update_snapshot(&milestone_obj, &mut self.latest_snapshot)?;
                            } else {
                                panic!("broken ledger validator mutex");
                            }
                        } else {
                            panic!("ledger validator is None");
                        }
                        if update_snapshot_is_ok {
                            self.latest_solid_subhive_milestone = milestone_obj.get_hash();
                            self.latest_solid_subhive_milestone_index = milestone_obj.index();
                        }
                    } else {
                        break;
                    }
                }

                if let Ok(hive) = self.hive.lock() {
                    closest_milestone = hive.storage_next_milestone(milestone_obj.index);
                } else {
                    panic!("broken hive mutex");
                }
            }
        }
        Ok(())
    }

    pub fn validate_milestone(&self, transaction: &Transaction, index: u32) -> Validity {
        if index < 0 || index >= 0x200000 {
            return Validity::Invalid;
        }
        if let Ok(mut hive) = self.hive.lock() {
            if hive.storage_load_milestone(index).is_some() {
                return Validity::Valid;
            }

            if let Some(tx2) = hive.storage_load_transaction(&transaction.get_trunk_transaction_hash()) {
                if tx2.get_type() == TransactionType::Full {
                    if self.testnet && self.accept_any_testnet_coo ||
                        Address::from_public_key(&tx2.object.signature_pubkey) == self.coordinator {
                            hive.put_milestone(&MilestoneObject {
                                index,
                                hash: transaction.get_hash()
                            });
                        return Validity::Valid;
                    } else {
                        return Validity::Invalid;
                    }
                }
            }
        }
        return Validity::Invalid;
    }

    fn get_index(&self, tx: &Transaction) -> u32 {
        use byteorder::{BigEndian, ByteOrder};
        BigEndian::read_u32(&tx.object.tag[(HASH_SIZE - 4)..])
    }

    pub fn shutdown(&mut self){
        self.shutting_down = true;

        if let Some(jh) = self.latest_milestone_tracker_thread.take() {
            jh.join();
        };
        if let Some(jh) = self.solid_milestone_tracker_thread.take() {
            jh.join();
        };
    }
}