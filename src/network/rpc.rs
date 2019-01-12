#[macro_use]
use serde_derive;
use serde::Serialize;
use serde_pm::SerializedBuffer;

use model::{
    Transaction, TransactionObject,
    transaction::*
};

/**
    GetNodeInfo
*/
#[derive(Serialize, Deserialize)]
pub struct GetNodeInfo {}

impl GetNodeInfo {
    pub const SVUID : i32 = 2;
}

/**
    NodeInfo
*/
#[derive(Serialize, Deserialize)]
pub struct NodeInfo {
    pub name: String,
}

impl NodeInfo {
    pub const SVUID : i32 = 3;
}

/**
    AttachTransaction
*/
//#[derive(Serialize, Deserialize)]
pub struct AttachTransaction {
    pub transaction: TransactionObject,
}

impl AttachTransaction { pub const SVUID : i32 = 4; }

/**
    BroadcastTransaction
*/
#[derive(Serialize, Deserialize)]
pub struct BroadcastTransaction {
    pub transaction: TransactionObject,
}

impl BroadcastTransaction { pub const SVUID : i32 = 15235324; }

/**
    GetTransactionsToApprove
*/
#[derive(Serialize, Deserialize)]
pub struct GetTransactionsToApprove {
    pub depth: u32,
    pub num_walks: u32,
    pub reference: Hash
}

impl GetTransactionsToApprove { pub const SVUID : i32 = 5; }

//impl Serializable for GetTransactionsToApprove {
//    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
//        stream.write_i32(Self::SVUID);
//        stream.write_u32(self.depth);
//        stream.write_u32(self.num_walks);
//
//        if self.reference == HASH_NULL {
//            stream.write_bool(true);
//            stream.write_bytes(&self.reference);
//        } else {
//            stream.write_bool(false);
//        }
//    }
//
//    fn read_params(&mut self, stream: &mut SerializedBuffer) {
//        self.depth = stream.read_u32();
//        self.num_walks = stream.read_u32();
//
//        if stream.read_bool() {
//            stream.read_bytes(&mut self.reference, HASH_SIZE);
//        } else {
//            self.reference = HASH_NULL;
//        }
//    }
//}

/**
    TransactionsToApprove
*/
#[derive(Serialize, Deserialize)]
pub struct TransactionsToApprove {
    pub trunk: Hash,
    pub branch: Hash,
}

impl TransactionsToApprove { pub const SVUID : i32 = 6; }

/**
    RequestTransaction
*/
#[derive(Serialize, Deserialize, PMIdentifiable)]
#[pm_identifiable(id = "0x0a8c76de")]
pub struct RequestTransaction {
    pub hash: Hash,
}

impl RequestTransaction { pub const SVUID : i32 = 17; }

/**
    GetBalances
*/
#[derive(Serialize, Deserialize)]
pub struct GetBalances {
    pub addresses: Vec<Address>,
    pub tips: Vec<Hash>,
    pub threshold: u8,
}

impl GetBalances { pub const SVUID : i32 = 7; }

/**
    Balances
*/
#[derive(Serialize, Deserialize)]
pub struct Balances {
    pub balances: Vec<u64>,
}

impl Balances { pub const SVUID : i32 = 8; }

#[derive(Serialize, Deserialize)]
pub struct GetInclusionStates {
    pub transactions: Vec<Hash>,
    pub tips: Vec<Hash>
}

impl GetInclusionStates { pub const SVUID : i32 = 12; }

#[derive(Serialize, Deserialize)]
pub struct InclusionStates {
    pub booleans: Vec<bool>
}

impl InclusionStates { pub const SVUID : i32 = 13; }

#[derive(Serialize, Deserialize)]
pub struct GetTips {
}

impl GetTips { pub const SVUID : i32 = 14; }

#[derive(Serialize, Deserialize)]
pub struct Tips {
    pub hashes: Vec<Hash>
}

impl Tips { pub const SVUID : i32 = 15; }

#[derive(Serialize, Deserialize)]
pub struct FindTransactions {
    pub addresses: Vec<Address>,
    pub tags: Vec<Hash>,
    pub approvees: Vec<Hash>,
}

impl FindTransactions { pub const SVUID : i32 = 16; }

#[derive(Serialize, Deserialize)]
pub struct FoundedTransactions {
    pub hashes: Vec<Hash>
}

impl FoundedTransactions { pub const SVUID : i32 = 16; }

#[derive(Serialize, Deserialize)]
pub struct GetTransactionsData {
    pub hashes: Vec<Hash>,
}

impl GetTransactionsData { pub const SVUID : i32 = 16; }

#[derive(Serialize, Deserialize)]
pub struct TransactionsData {
    pub transactions: Vec<String>,
}

impl TransactionsData { pub const SVUID : i32 = 16; }

#[derive(Serialize, Deserialize, Eq, Debug, Clone, PMIdentifiable)]
#[pm_identifiable(id = "0x183f2199")]
pub struct ConsensusValue {
    pub value: u32,
}

impl ConsensusValue { pub const SVUID : i32 = 167; }

use std::cmp::Ordering;

impl Ord for ConsensusValue {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl PartialOrd for ConsensusValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.value.cmp(&other.value))
    }
}

impl PartialEq for ConsensusValue {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Default for ConsensusValue {
    fn default() -> Self {
        ConsensusValue {
            value: u32::default(),
        }
    }
}