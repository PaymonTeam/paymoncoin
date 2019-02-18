use std::collections::HashMap;
use crate::model::transaction::*;
use serde::{Serialize, Deserialize};
use serde_pm::{SerializedBuffer, from_stream};

#[derive(Serialize, Deserialize)]
pub struct StateDiffObject {
    pub state: HashMap<Address, i64>
}

impl StateDiffObject {
    pub const SVUID: i32 = 29537194;

    pub fn new() -> Self {
        StateDiffObject {
            state: HashMap::new()
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct StateDiff {
    pub state_diff_object: StateDiffObject,
    pub hash: Hash
}

impl StateDiff {
    pub fn from_bytes(mut bytes: SerializedBuffer, hash: Hash) -> Self {
        let state_diff_object = from_stream(&mut bytes).unwrap();

        StateDiff {
            state_diff_object,
            hash
        }
    }
}
