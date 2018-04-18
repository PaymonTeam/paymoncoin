use network::packet::{Serializable, SerializedBuffer};
use model::{
    Transaction,
    transaction::Hash
};

/**
    GetNodeInfo
*/
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
pub struct NodeInfo {

}

impl NodeInfo {
    pub const SVUID : i32 = 3;
}

impl Serializable for NodeInfo {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
    }
}

/**
    AttachTransaction
*/
pub struct AttachTransaction {
    pub transaction: Transaction,
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
pub struct BroadcastTransaction {
    pub transaction: Transaction,
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
pub struct GetTransactionsToApprove {}

impl GetTransactionsToApprove { pub const SVUID : i32 = 5; }

impl Serializable for GetTransactionsToApprove {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) { stream.write_i32(Self::SVUID); }

    fn read_params(&mut self, stream: &mut SerializedBuffer) { }
}

/**
    TransactionsToApprove
*/
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
pub struct GetBalances {}

impl GetBalances { pub const SVUID : i32 = 7; }

impl Serializable for GetBalances {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) { stream.write_i32(Self::SVUID); }

    fn read_params(&mut self, stream: &mut SerializedBuffer) { }
}

/**
    Balances
*/
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
