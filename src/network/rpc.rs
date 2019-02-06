#[macro_use]
use serde_derive;
use serde::{Serialize, Deserialize};
use serde_pm::{SerializedBuffer, Boxed, Identifiable};
use std::fmt::Debug;

use crate::model::{
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

/**
    AttachTransaction
*/
//#[derive(Serialize, Deserialize)]
pub struct AttachTransaction {
    pub transaction: TransactionObject,
}

/**
    BroadcastTransaction
*/
#[derive(Serialize, Deserialize)]
pub struct BroadcastTransaction {
    pub transaction: TransactionObject,
}

/**
    GetTransactionsToApprove
*/
#[derive(Serialize, Deserialize)]
pub struct GetTransactionsToApprove {
    pub depth: u32,
    pub num_walks: u32,
    pub reference: Hash
}

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

/**
    RequestTransaction
*/
#[derive(Serialize, Deserialize, PMIdentifiable)]
#[pm_identifiable(id = "0x0a8c76de")]
pub struct RequestTransaction {
    pub hash: Hash,
}

/**
    GetBalances
*/
#[derive(Serialize, Deserialize)]
pub struct GetBalances {
    pub addresses: Vec<Address>,
    #[serde(default)]
    pub tips: Vec<Hash>,
    #[serde(default)]
    pub threshold: u8,
}

/**
    Balances
*/
#[derive(Serialize, Deserialize)]
pub struct Balances {
    pub balances: Vec<u64>,
}

#[derive(Serialize, Deserialize)]
pub struct GetInclusionStates {
    pub transactions: Vec<Hash>,
    pub tips: Vec<Hash>
}

#[derive(Serialize, Deserialize)]
pub struct InclusionStates {
    pub booleans: Vec<bool>
}

#[derive(Serialize, Deserialize)]
pub struct GetTips {
}

#[derive(Serialize, Deserialize)]
pub struct Tips {
    pub hashes: Vec<Hash>
}

#[derive(Serialize, Deserialize)]
pub struct FindTransactions {
    pub addresses: Vec<Address>,
    pub tags: Vec<Hash>,
    pub approvees: Vec<Hash>,
}

#[derive(Serialize, Deserialize)]
pub struct FoundedTransactions {
    pub hashes: Vec<Hash>
}

#[derive(Serialize, Deserialize)]
pub struct GetTransactionsData {
    pub hashes: Vec<Hash>,
}

#[derive(Serialize, Deserialize)]
pub struct TransactionsData {
    pub transactions: Vec<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, PMIdentifiable, Default, PartialOrd, Ord)]
#[pm_identifiable(id = "0x20fcaac6")]
pub struct ContractsInputOutputs {
    pub vec: Vec<crate::model::contracts_manager::ContractInputOutput>,
}

#[derive(Serialize, Deserialize, Eq, Debug, Clone, PMIdentifiable, Default)]
#[pm_identifiable(id = "0x183f2199")]
pub struct ConsensusValue {
    pub value: u32,
}

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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, PMIdentifiable)]
#[pm_identifiable(id = "0xac73d2ff")]
pub struct Signed
//    where T: Serialize + PartialEq + Eq + Default + Debug + Clone + Identifiable
{
    pub signature: Vec<u8>,
    pub hash: Hash,
    pub data: SignedData,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum SignedData {
    ApplyForValidator {
        stake: u64,
        address: Address,
    }
}

//#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, PMIdentifiable, Default)]
//#[pm_identifiable(id = "0x24e88fca")]
//pub struct ApplyForValidator {
//    stake: u64,
//    address: Address,
//}