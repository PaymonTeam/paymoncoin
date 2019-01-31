use std::collections::{HashSet};
use std::str::FromStr;
use crate::consensus::Validator;
use crate::model::transaction::{Hash, Address, Account};
use crate::model::contract::{ContractsStorage, Error, ContractOutput, ContractAddress};
use serde_json as json;
use linked_hash_set::LinkedHashSet;
use std::hash;

#[derive(Default)]
pub struct ContractTransaction {
    value: u32,
    hash: Hash,
    from: Account,
    address: ContractAddress,
    contract_input: json::Map<String, json::Value>,
    timestamp: u64,
}

impl PartialEq for ContractTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

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
    validators_limit: u32,
}

impl ContractsManager {
    pub fn new() -> Self {
        Self {
            pos_contract_address: ContractAddress::from_str("P1111111111111111111111111111111111111111").unwrap(),
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

    pub fn execute_contracts(&mut self) -> Vec<ContractOutput> {
        let contracts_todo_num = 5;
        let mut vec = vec![];

        while let Some(ct) = self.transactions_queue.pop_front() {
            if vec.len() < contracts_todo_num {
                match self.call(&ct.from, &ct.address, ct.contract_input, ct.timestamp) {
                    Ok(r) => {
                        vec.push(r);
                    },
                    Err(e) => {
                        error!("failed to execute contract {:?} from {:?}", ct.address, ct.from);
                    }
                }
            }
        }

        vec
    }

    #[inline]
    pub fn call(&mut self, caller: &Account, address: &ContractAddress, input: json::Map<String, json::Value>, tx_timestamp: u64) -> Result<ContractOutput, Error> {
        let call_input = input.get("input").ok_or(Error::JsonParse("expected field 'input'".into()))?
            .as_object().ok_or(Error::JsonParse("expected object".into()))?;

        if address == self.pos_contract_address {
            use chrono::*;

            let timeout_secs = 15 * 60;
            let mut now = Utc::now();
            let secs_in_day: u32 = 24 * 60 * 60;
            let s = now.num_seconds_from_midnight();

            if secs_in_day - s > timeout_secs {
                // TODO: storage changed before
                match self.storage.call(&address, caller, call_input) {
                    Ok(r) => {
                        if self.future_validators.len() < self.validators_limit {
                            self.future_validators.insert(caller.0.clone());
                        }
                        r.storage_diff.
                            Ok(r)
                    },
                    Err(e) => Err(e)
                }
            } else {
                return Err(Error::Unknown("timeout".into()));
            }
        } else {
            return self.storage.call(address, caller, call_input);
        }
    }
}