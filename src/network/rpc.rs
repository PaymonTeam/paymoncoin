use network::packet::{Serializable, SerializedBuffer};
use model::{
    Transaction, TransactionObject,
    transaction::Hash,
    transaction::Address,
    transaction::HASH_SIZE,
    transaction::HASH_NULL,
    transaction::ADDRESS_NULL,
    transaction::ADDRESS_SIZE
};
use std::collections::LinkedList;

/**
    GetNodeInfo
*/
#[derive(RustcDecodable, RustcEncodable)]
pub struct GetNodeInfo {}

impl GetNodeInfo {
    pub const SVUID : i32 = 2;
}

impl Serializable for GetNodeInfo {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
    }
}

/**
    NodeInfo
*/
#[derive(RustcDecodable, RustcEncodable)]
pub struct NodeInfo {
    pub name: String,
}

impl NodeInfo {
    pub const SVUID : i32 = 3;
}

impl Serializable for NodeInfo {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_string(self.name.clone());
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.name = stream.read_string();
    }
}

/**
    AttachTransaction
*/
//#[derive(RustcDecodable, RustcEncodable)]
pub struct AttachTransaction {
    pub transaction: TransactionObject,
}

impl AttachTransaction { pub const SVUID : i32 = 4; }

impl Serializable for AttachTransaction {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        self.transaction.serialize_to_stream(stream);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.transaction.read_params(stream);
    }
}

/**
    BroadcastTransaction
*/
#[derive(RustcDecodable, RustcEncodable)]
pub struct BroadcastTransaction {
    pub transaction: TransactionObject,
}

impl BroadcastTransaction { pub const SVUID : i32 = 15235324; }

impl Serializable for BroadcastTransaction {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        self.transaction.serialize_to_stream(stream);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.transaction.read_params(stream);
    }
}

/**
    GetTransactionsToApprove
*/
#[derive(RustcDecodable, RustcEncodable)]
pub struct GetTransactionsToApprove {}

impl GetTransactionsToApprove { pub const SVUID : i32 = 5; }

impl Serializable for GetTransactionsToApprove {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) { stream.write_i32(Self::SVUID); }

    fn read_params(&mut self, stream: &mut SerializedBuffer) { }
}

/**
    TransactionsToApprove
*/
#[derive(RustcDecodable, RustcEncodable)]
pub struct TransactionsToApprove {
    pub trunk: Hash,
    pub branch: Hash,
}

impl TransactionsToApprove { pub const SVUID : i32 = 6; }

impl Serializable for TransactionsToApprove {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        self.trunk.serialize_to_stream(stream);
        self.branch.serialize_to_stream(stream);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.trunk.read_params(stream);
        self.branch.read_params(stream);
    }
}

/**
    GetBalances
*/
#[derive(RustcDecodable, RustcEncodable)]
pub struct GetBalances {}

impl GetBalances { pub const SVUID : i32 = 7; }

impl Serializable for GetBalances {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) { stream.write_i32(Self::SVUID); }

    fn read_params(&mut self, stream: &mut SerializedBuffer) { }
}

/**
    Balances
*/
#[derive(RustcDecodable, RustcEncodable)]
pub struct Balances {
    pub balances: Vec<u32>,
}

impl Balances { pub const SVUID : i32 = 8; }

impl Serializable for Balances {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_u32(self.balances.len() as u32);
        for b in &self.balances {
            b.serialize_to_stream(stream);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.balances.clear();

        let len = stream.read_u32();
        for _ in 0..len {
            let balance = stream.read_u32();
            self.balances.push(balance);
        }
    }
}
#[derive(RustcDecodable, RustcEncodable)]
pub struct FindTransactionByHash {
    pub list_hash: LinkedList<Hash>,
}
impl FindTransactionByHash { pub const SVUID : i32 = 9; }

impl Serializable for FindTransactionByHash {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_u32(self.list_hash.len() as u32);
        for hash in &self.list_hash {
            hash.serialize_to_stream(stream);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.list_hash.clear();

        let len = stream.read_u32();
        for _ in 0..len {
            if stream.read_bool() {
                let mut hash: Hash = HASH_NULL;
                stream.read_bytes(&mut *hash, HASH_SIZE);
                self.list_hash.push_back(hash);
            } else {
                self.list_hash.push_back(HASH_NULL);
            }
        }
    }


}
#[derive(RustcDecodable, RustcEncodable)]
pub struct FindTransactionByAddress {
    pub list_adr: LinkedList<Address>,
}
impl FindTransactionByAddress { pub const SVUID : i32 = 10; }

impl Serializable for FindTransactionByAddress {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_u32(self.list_adr.len() as u32);
        for adr in &self.list_adr {
            adr.serialize_to_stream(stream);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.list_adr.clear();

        let len = stream.read_u32();
        for _ in 0..len {
            if stream.read_bool() {
                let mut adr: Address = ADDRESS_NULL;
                stream.read_bytes(&mut adr, ADDRESS_SIZE);
                self.list_adr.push_back(adr);
            } else {
                self.list_adr.push_back(ADDRESS_NULL);
            }
        }
    }
}
#[derive(RustcDecodable, RustcEncodable)]
pub struct FindTransaction {
    pub hashes: Vec<Hash>,
}

impl FindTransaction { pub const SVUID : i32 = 11; }

impl Serializable for FindTransaction {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_u32(self.hashes.len() as u32);
        for hash in &self.hashes {
            hash.serialize_to_stream(stream);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        let len = stream.read_u32();
        for _ in 0..len {
            if stream.read_bool() {
                let mut hash: Hash = HASH_NULL;
                stream.read_bytes(&mut *hash, HASH_SIZE);
                self.hashes.push(hash);
            } else {
                self.hashes.push(HASH_NULL);
            }
        }
    }
}
#[derive(RustcDecodable, RustcEncodable)]
pub struct GetNewInclusionStateStatement {
    pub list_tx: LinkedList<Hash>,
    pub list_tps: LinkedList<Hash>
}

impl GetNewInclusionStateStatement { pub const SVUID : i32 = 12; }

impl Serializable for GetNewInclusionStateStatement {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_u32(self.list_tx.len() as u32);
        for hash in &self.list_tx {
            hash.serialize_to_stream(stream);
        }
        stream.write_u32(self.list_tps.len() as u32);
        for hash in &self.list_tps {
            hash.serialize_to_stream(stream);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        let len_tx = stream.read_u32();
        for _ in 0..len_tx {
            if stream.read_bool() {
                let mut hash: Hash = HASH_NULL;
                stream.read_bytes(&mut *hash, HASH_SIZE);
                self.list_tx.push_back(hash);
            } else {
                self.list_tx.push_back(HASH_NULL);
            }
        }
        let len_tps = stream.read_u32();
        for _ in 0..len_tps {
            if stream.read_bool() {
                let mut hash: Hash = HASH_NULL;
                stream.read_bytes(&mut *hash, HASH_SIZE);
                self.list_tps.push_back(hash);
            } else {
                self.list_tps.push_back(HASH_NULL);
            }
        }
    }
}
#[derive(RustcDecodable, RustcEncodable)]
pub struct GetInclusionStates {
    pub booleans: Vec<bool>
}

impl GetInclusionStates { pub const SVUID : i32 = 13; }

impl Serializable for GetInclusionStates {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_u32(self.booleans.len() as u32);
        for b in &self.booleans {
            b.serialize_to_stream(stream);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        let len = stream.read_u32();
        for _ in 0..len {
            let b = stream.read_bool();
            self.booleans.push(b);
        }
    }
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct GetTips {
    pub hashes: LinkedList<Hash>
}

impl GetTips { pub const SVUID : i32 = 14; }

impl Serializable for GetTips {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_u32(self.hashes.len() as u32);
        for hash in &self.hashes {
            hash.serialize_to_stream(stream);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        let len = stream.read_u32();
        for _ in 0..len {
            if stream.read_bool() {
                let mut hash: Hash = HASH_NULL;
                stream.read_bytes(&mut *hash, HASH_SIZE);
                self.hashes.push_back(hash);
            } else {
                self.hashes.push_back(HASH_NULL);
            }
        }
    }
}

