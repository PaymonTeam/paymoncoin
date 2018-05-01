use std::sync::{Arc, Mutex};
use std::collections::HashSet;

use model::transaction::*;
use storage::hive::Hive;
use utils::defines::AM;
#[derive(Debug, PartialEq, Clone, RustcEncodable, RustcDecodable)]
pub struct Approvee{
    hash: Hash,
    set: HashSet<Hash>
}
impl Approvee{
    pub fn new_empty() -> Self{
        Approvee{
            hash: HASH_NULL,
            set: HashSet::new()
        }
    }
    pub fn new(hash_: &Hash) -> Self{
        Approvee{
            hash: *hash_,
            set: HashSet::new()
        }
    }
    pub fn load(hive: &AM<Hive>, hash: &Hash) -> Option<Self>{
        if let Ok( mut hive_) = hive.lock(){
            return Some(Approvee::new(&match hive_.storage_load_approvee(hash){
                Some(h) => h,
                None => HASH_NULL
            }));
        }else{
            return None;
        }

    }
    pub fn get_hashes(&self) -> HashSet<Hash>{
        return self.set.clone();
    }
}
/**/