use std::collections::{HashSet};
use std::str::FromStr;
use std::time;
use crate::consensus::Validator;
use crate::model::transaction::{Hash, Address, Account};
use crate::model::contract::{ContractsStorage, Error, ContractOutput};
use serde_json as json;

pub struct ContractsManager {
    executed: HashSet<Hash>,
    storage: ContractsStorage,
    pub pos_contract_hash: Hash,
    pub validators: HashSet<Validator>,
    pub future_validators: HashSet<Address>,
    validators_limit: u32,
    can_add_future_validators: ,
}

impl ContractsManager {
    pub fn new() -> Self {
        Self {
            pos_contract_hash: Hash::from_str("").unwrap(),
            validators: HashSet::new(),
            future_validators: HashSet::new(),
            validators_limit: 0,
            storage: ContractsStorage::new(),
            executed: HashSet::new(),
        }
    }

    #[inline]
    pub fn call(&mut self, caller: &Account, input: json::Map<String, json::Value>, tx_timestamp: u64) -> Result<ContractOutput, Error> {
        let hash = Hash::from_str(input
            .get("hash").ok_or(Error::JsonParse("expected field 'hash'".into()))?
            .as_str().ok_or(Error::JsonParse("expected string".into()))?)?;
        let call_input = input.get("input").ok_or(Error::JsonParse("expected field 'input'".into()))?
            .as_object().ok_or(Error::JsonParse("expected object".into()))?;

        if hash == self.pos_contract_hash {
            if (time::Instant::now()).as_secs() > 15 {
                // TODO: storage changed before
                match self.storage.call(&hash, caller, call_input) {
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
            return self.storage.call(&hash, caller, call_input);
        }
    }
}