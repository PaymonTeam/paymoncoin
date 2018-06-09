use std::sync::{Arc, Mutex};
use std::collections::HashSet;

use model::transaction::*;
use storage::hive::Hive;
use utils::defines::AM;

#[derive(Debug, PartialEq, Clone, RustcEncodable, RustcDecodable)]
pub struct Approvee {
    hash: Hash,
    set: HashSet<Hash>,
}

impl Approvee {
    pub fn new_empty() -> Self {
        Approvee {
            hash: HASH_NULL,
            set: HashSet::new(),
        }
    }

    pub fn new(hash: &Hash, set: &HashSet<Hash>) -> Self {
        Approvee {
            hash: hash.clone(),
            set: set.clone(),
        }
    }

    pub fn load(hive: &AM<Hive>, hash: &Hash) -> Option<Self> {
        if let Ok(mut hive) = hive.lock() {
            return Some(Approvee::new(hash, &vec_to_set(&match hive.storage_load_approvee(hash) {
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
