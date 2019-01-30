use std::collections::HashSet;
use crate::model::transaction::Hash;
use crate::model::contract::ContractsStorage;

pub struct ContractsManager {
    executed: HashSet<Hash>,
    storage: ContractsStorage,
}

impl ContractsManager {
    pub fn apply_for_validator() {
    }
}