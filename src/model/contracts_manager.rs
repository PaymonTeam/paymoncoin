use std::collections::{HashSet};
use std::str::FromStr;
use crate::consensus::Validator;
use crate::model::transaction::{Hash, Address, Account};
use crate::model::contract::{ContractsStorage, Error, ContractOutput, ContractAddress};
use serde_json as json;
use linked_hash_set::LinkedHashSet;
use std::hash;
use crate::storage::Hive;
use crate::utils::AWM;
use std::cmp::Ordering;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ContractTransaction {
    pub value: u32,
    pub hash: Hash,
    pub from: Account,
    pub address: ContractAddress,
    pub contract_input: json::Map<String, json::Value>,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractInputOutput {
    pub input: ContractTransaction,
    pub output: ContractOutput,
}

impl PartialEq for ContractInputOutput {
    fn eq(&self, other: &Self) -> bool {
        self.input == other.input
    }
}

impl PartialOrd for ContractInputOutput {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.input.partial_cmp(&other.input)
    }
}

impl Eq for ContractInputOutput {}

impl Ord for ContractInputOutput {
    fn cmp(&self, other: &Self) -> Ordering {
        self.input.cmp(&other.input)
    }
}

impl PartialEq for ContractTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl PartialOrd for ContractTransaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.hash.partial_cmp(&other.hash)
    }
}

impl Ord for ContractTransaction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.hash.cmp(&other.hash)
    }
}

impl Eq for ContractTransaction {}

impl hash::Hash for ContractTransaction {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        use std::hash::Hash;
        self.hash.hash(state)
    }
}

pub struct ContractsManager {
    executed: HashSet<Hash>,
    transactions_queue: LinkedHashSet<ContractTransaction>,
    storage: ContractsStorage,
    pub pos_contract_address: ContractAddress,
    pub validators: HashSet<Validator>,
    pub future_validators: HashSet<Address>,
    validators_limit: usize,
}

impl ContractsManager {
    pub fn new() -> Self {
        Self {
            pos_contract_address: ContractAddress::from_str("P000000000000000000000000000000000000000000").unwrap(),
            validators: HashSet::new(),
            future_validators: HashSet::new(),
            validators_limit: 0,
            storage: ContractsStorage::new(),
            executed: HashSet::new(),
            transactions_queue: LinkedHashSet::new(),
        }
    }

    pub fn add_contract_tx_to_queue(&mut self, value: u32, hash: Hash, from: Account, address: ContractAddress, contract_input: json::Map<String, json::Value>, timestamp: u64) {
        let ct = ContractTransaction {
            value, hash, from, address, contract_input, timestamp,
        };
        self.transactions_queue.insert(ct);
    }

    pub fn remove_from_queue(&mut self, hashes: &HashSet<Hash>) {
        for hash in hashes {
            // hack for fast remove
            let mut temp = ContractTransaction::default();
            temp.hash = hash.clone();
            self.transactions_queue.remove(&temp);
        }
    }

    pub fn execute_contracts(&mut self) -> Vec<ContractInputOutput> {
        let contracts_todo_num = 5;
        let mut vec = vec![];

        while let Some(ct) = self.transactions_queue.pop_front() {
            if vec.len() < contracts_todo_num {
                if ct.address == self.pos_contract_address {
                    use chrono::*;

                    let timeout_secs = 15 * 60;
                    let mut now = Utc::now();
                    let secs_in_day: u32 = 24 * 60 * 60;
                    let s = now.num_seconds_from_midnight();

                    if secs_in_day - s > timeout_secs {
                        match self.input(&ct.from, &ct.address, ct.contract_input.clone(), ct.timestamp) {
                            Ok(r) => {
                                if self.future_validators.len() < self.validators_limit {
                                    self.future_validators.insert(ct.from.0.clone());
                                    vec.push(ContractInputOutput {
                                        output: r,
                                        input: ct.clone(),
                                    });
                                } else {
                                    error!("validators limit exceeded");
                                }
                            },
                            Err(e) => {
                                error!("failed to execute contract {:?} from {:?}", ct.address, ct.from);
                            }
                        }
                    }
                } else {
                    match self.input(&ct.from, &ct.address, ct.contract_input.clone(), ct.timestamp) {
                        Ok(r) => {
                            vec.push(ContractInputOutput {
                                output: r,
                                input: ct.clone(),
                            });
                        },
                        Err(e) => {
                            error!("failed to execute contract {:?} from {:?}", ct.address, ct.from);
                        }
                    }
                }
            }
        }

        vec
    }

    pub fn input(&mut self, caller: &Account, address: &ContractAddress, input: json::Map<String, json::Value>, tx_timestamp: u64) -> Result<ContractOutput, Error> {
        let call_input = input.get("input").ok_or(Error::JsonParse("expected field 'input'".into()))?
            .as_object().ok_or(Error::JsonParse("expected object".into()))?;

        self.storage.call(address, caller, call_input)
        // or create...
        // or remove
    }

    #[inline]
    pub fn apply_state(&mut self, call: ContractInputOutput, hive: AWM<Hive>) {
        self.storage.apply_state(call, hive);
    }
}