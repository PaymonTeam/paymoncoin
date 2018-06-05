use network::packet::{Serializable, SerializedBuffer};
use model::{
    Transaction, TransactionObject,
    transaction::*
};

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
pub struct GetTransactionsToApprove {
    pub depth: u32,
    pub num_walks: u32,
    pub reference: Hash
}

impl GetTransactionsToApprove { pub const SVUID : i32 = 5; }

impl Serializable for GetTransactionsToApprove {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_u32(self.depth);
        stream.write_u32(self.num_walks);

        if self.reference == HASH_NULL {
            stream.write_bool(true);
            stream.write_bytes(&self.reference);
        } else {
            stream.write_bool(false);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.depth = stream.read_u32();
        self.num_walks = stream.read_u32();

        if stream.read_bool() {
            stream.read_bytes(&mut self.reference, HASH_SIZE);
        } else {
            self.reference = HASH_NULL;
        }
    }
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
