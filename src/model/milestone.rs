use model::transaction::*;
use model::LedgerValidator;
use model::snapshot::Snapshot;
use model::TransactionValidator;
use network::packet::*;
use model::transaction_validator::TransactionError;

use std::collections::{HashSet, HashMap, LinkedList};
use utils::AM;
use storage::Hive;
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

//    pub fn from_bytes(mut bytes: SerializedBuffer) -> Self {
//        let mut index = 0u32;
//        let mut hash = HASH_NULL;
//        index.read_params(SerializedBuffer::from_slice(&key));
//        hash.read_params(SerializedBuffer::from_slice(&bytes));
//
//        MilestoneObject {
//            index,
//            hash
//        };
//
//        let mut milestone = MilestoneObject { index: 0, hash: HASH_NULL };
//        milestone.read_params(&mut bytes);
//        milestone
//    }

    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn get_hash(&self) -> Hash {
        self.hash.clone()
    }
}

impl Serializable for MilestoneObject {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(MilestoneObject::SVUID);

        stream.write_u32(self.index);
        stream.write_bytes(&self.hash);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.index = stream.read_u32();
        stream.read_bytes(&mut self.hash, HASH_SIZE);
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

//            if let Some(arc) = transaction_validator.upgrade() {
                if let Ok(m) = milestone_clone_1.lock() {
                    hive = m.hive.clone();
                    coordinator = m.coordinator.clone(); //Address::from_public_key(&m.coordinator);
                } else {
                    panic!("broken transaction_validator mutex");
                }
//            } else {
//                panic!("transaction_validator is null");
//            }

            info!("Waiting for Ledger Validator initialization...");
            while !ledger_initialized_clone_thread_1.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(1));
            }

            info!("tracker started");

            let mut shutting_down = false;

            while !shutting_down {
                let scan_time = SystemTime::now();
                let mut previous_latest_milestone_index = 0u32;

                if let Ok(self_p) = milestone_clone_1.lock() {
                    shutting_down = self_p.shutting_down;
//                    previous_latest_milestone_index = self_p.latest_milestone_index;
                    if shutting_down {
                        break;
                    }
                } else {
                    panic!("broken milestone mutex");
                }

                if let Ok(hive) = hive.lock() {
                    // TODO: optimize
                    if let Some(mut hashes) = hive.load_address_transactions(&coordinator) {
                        for hash in hashes {
                            if let Ok(mut milestone) = milestone_clone_1.lock() {
                                if milestone.analyzed_milestone_candidates.insert(hash) {
                                    if let Some(t) = hive.storage_load_transaction(&hash) {
                                        let valid: Validity = milestone.validate_milestone(&t, milestone.get_index(&t));
                                        match valid {
                                            Validity::Valid => {
                                                match hive.storage_latest_milestone() {
                                                    Some(milestone_manager) => {
                                                        if milestone_manager.index() > milestone.latest_milestone_index {
                                                            milestone.latest_milestone = milestone_manager.get_hash();
                                                            milestone.latest_milestone_index = milestone_manager.index();
                                                        }
                                                    }
                                                    None => continue
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

            if let Ok(ref mut milestone) = milestone_clone_2.lock() {
                if let Some(ref mut ledger_validator) = milestone.ledger_validator {
                    if let Ok(ref mut ledger_validator) = ledger_validator.lock() {
                        ledger_validator.init();
                    }
                }
                ledger_initialized_clone_thread_2.store(true, Ordering::SeqCst);
            }

            info!("tracker started");
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
                        milestone.update_latest_solid_subhive_milestone();
                    }

                    if previous_solid_subhive_latest_milestone_index != milestone.latest_solid_subhive_milestone_index {
                        info!("Latest SOLID SUBHIVE milestone has changed from #{} to #{}",
                        previous_solid_subhive_latest_milestone_index,
                                 milestone.latest_solid_subhive_milestone_index);
                    }
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
        if let Ok(hive) = self.hive.lock() {
            if let Some(latest) = hive.storage_latest_milestone() {
                let mut closest_milestone = hive.find_closest_next_milestone(self
                                                     .latest_solid_subhive_milestone_index,
                                                 self.testnet, self.milestone_start_index);
                while let Some(mut milestone_obj) = closest_milestone.clone() {
                    if milestone_obj.index() <= latest.index() && !self.shutting_down {
                        if let Ok(tx_v) = self.transaction_validator.lock() {
                            if let Some(ref mut arc) = self.ledger_validator {
                                if let Ok(mut l_v) = arc.lock() {
                                    let update_snapshot_is_ok = l_v.update_snapshot(&milestone_obj)?;
                                    if tx_v.check_solidity(milestone_obj.get_hash(), true)? &&
                                        milestone_obj.index() >= self.latest_solid_subhive_milestone_index && update_snapshot_is_ok {

                                        self.latest_solid_subhive_milestone = milestone_obj.get_hash();
                                        self.latest_solid_subhive_milestone_index = milestone_obj.index();

                                        closest_milestone = hive.storage_next_milestone(milestone_obj.index);
                                    } else {
                                        closest_milestone = hive.storage_next_milestone(milestone_obj.index);
                                        break;
                                    }
                                }
                            }
                        }
                    }
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
        unimplemented!();
    }

    pub fn shutdown(&mut self){
        self.shutting_down = true;
    }
}