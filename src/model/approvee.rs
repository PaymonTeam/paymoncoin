use std::sync::{Arc, Mutex};
use std::collections::HashSet;

use crate::model::transaction::*;
use crate::storage::hive::Hive;
use crate::utils::defines::AM;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Approvee {
    set: HashSet<Hash>,
}

impl Approvee {
    pub fn new_empty() -> Self {
        Approvee {
            set: HashSet::new(),
        }
    }

    pub fn new(set: &HashSet<Hash>) -> Self {
        Approvee {
            set: set.clone(),
        }
    }

    pub fn load(hive: &AM<Hive>, hash: &Hash) -> Option<Self> {
        if let Ok(hive) = hive.lock() {
            return Some(Approvee::new(&vec_to_set(&match hive.storage_load_approvee(hash) {
                Some(vec_h) => vec_h,
                None => return None
            })));
        } else {
            return None;
        }
    }

    pub fn get_hashes(&self) -> HashSet<Hash> {
        return self.set.clone();
    }
}

pub fn vec_to_set(vec: &Vec<Hash>) -> HashSet<Hash> {
    let mut set = HashSet::new();
    for it in vec.iter() {
        set.insert(*it);
    }
    return set;
}
