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
    pub appName: String,
    pub appVersion: String,
    pub latestMilestone: Hash,
    pub latestMilestoneIndex: u32,
    pub latestSolidSubhiveMilestone: Hash,
    pub latestSolidSubhiveMilestoneIndex: u32,
    pub neighbors: i32,
    pub tips: i32,
    pub transactionsToRequest: i32
}

impl NodeInfo {
    pub const SVUID : i32 = 3;
}

impl Serializable for NodeInfo {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_string(self.appName.clone());
        stream.write_string(self.appVersion.clone());
        self.latestMilestone.serialize_to_stream(stream);
        stream.write_u32(self.latestMilestoneIndex);
        self.latestSolidSubhiveMilestone.serialize_to_stream(stream);
        stream.write_u32(self.latestSolidSubhiveMilestoneIndex);
        stream.write_i32(self.neighbors);
        stream.write_i32(self.tips);
        stream.write_i32(self.transactionsToRequest);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.appName = stream.read_string();
        self.appVersion = stream.read_string();
        let _ = stream.read_i32();
        self.latestMilestone.read_params(stream);
        self.latestMilestoneIndex = stream.read_u32();
        let _ = stream.read_i32();
        self.latestSolidSubhiveMilestone.read_params(stream);
        self.latestSolidSubhiveMilestoneIndex = stream.read_u32();
        self.neighbors = stream.read_i32();
        self.tips = stream.read_i32();
        self.transactionsToRequest = stream.read_i32();
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
        let _ = stream.read_i32();
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
        let _ = stream.read_i32();
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
        let _ = stream.read_i32();
        self.trunk.read_params(stream);
        let _ = stream.read_i32();
        self.branch.read_params(stream);
    }
}

/**
    RequestTransaction
*/
#[derive(RustcDecodable, RustcEncodable)]
pub struct RequestTransaction {
    pub hash: Hash,
}

impl RequestTransaction { pub const SVUID : i32 = 17; }

impl Serializable for RequestTransaction {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        self.hash.serialize_to_stream(stream);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        let _ = stream.read_i32();
        self.hash.read_params(stream);
    }
}

/**
    GetBalances
*/
#[derive(RustcDecodable, RustcEncodable)]
pub struct GetBalances {
    pub addresses: Vec<Address>,
    pub tips: Vec<Hash>,
    pub threshold: u8,
}

impl GetBalances { pub const SVUID : i32 = 7; }

impl Serializable for GetBalances {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);

        stream.write_u32(self.addresses.len() as u32);
        for addr in &self.addresses {
//            addr.serialize_to_stream(stream);
            stream.write_bytes(&addr);
        }

        stream.write_u32(self.tips.len() as u32);
        for v in &self.tips {
//            v.serialize_to_stream(stream);
            stream.write_bytes(&v);
        }

        stream.write_byte(self.threshold);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.addresses.clear();
        let len = stream.read_u32();
        for _ in 0..len {
            let mut addr = ADDRESS_NULL;
            addr.read_params(stream);
            self.addresses.push(addr);
        }

        self.tips.clear();
        let len = stream.read_u32();
        for _ in 0..len {
            let mut v = HASH_NULL;
            v.read_params(stream);
            self.tips.push(v);
        }

        self.threshold = stream.read_byte();
    }
}

/**
    Balances
*/
#[derive(RustcDecodable, RustcEncodable)]
pub struct Balances {
    pub balances: Vec<u64>,
}

impl Balances { pub const SVUID : i32 = 8; }

impl Serializable for Balances {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_u32(self.balances.len() as u32);
        for b in &self.balances {
//            b.serialize_to_stream(stream);
            stream.write_u64(*b);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.balances.clear();

        let len = stream.read_u32();
        for _ in 0..len {
            let balance = stream.read_u64();
            self.balances.push(balance);
        }
    }
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct GetInclusionStates {
    pub transactions: Vec<Hash>,
    pub tips: Vec<Hash>
}

impl GetInclusionStates { pub const SVUID : i32 = 12; }

impl Serializable for GetInclusionStates {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_u32(self.transactions.len() as u32);
        for hash in &self.transactions {
            stream.write_bytes(&hash);
        }
        stream.write_u32(self.tips.len() as u32);
        for hash in &self.tips {
            stream.write_bytes(&hash);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.transactions.clear();
        let len_tx = stream.read_u32();
        for _ in 0..len_tx {
            let mut v = HASH_NULL;
//            v.read_params(stream);
            stream.read_bytes(&mut v, HASH_SIZE);
            self.transactions.push(v);
        }
        self.transactions.clear();
        let len_tps = stream.read_u32();
        for _ in 0..len_tps {
            let mut v = HASH_NULL;
//            v.read_params(stream);
            stream.read_bytes(&mut v, HASH_SIZE);
            self.tips.push(v);
        }
    }
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct InclusionStates {
    pub booleans: Vec<bool>
}

impl InclusionStates { pub const SVUID : i32 = 13; }

impl Serializable for InclusionStates {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_u32(self.booleans.len() as u32);
        for b in &self.booleans {
            stream.write_bool(*b);
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
}

impl GetTips { pub const SVUID : i32 = 14; }

impl Serializable for GetTips {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {}
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct Tips {
    pub hashes: Vec<Hash>
}

impl Tips { pub const SVUID : i32 = 15; }

impl Serializable for Tips {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_u32(self.hashes.len() as u32);
        for hash in &self.hashes {
            stream.write_bytes(&hash);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.hashes.clear();

        let len = stream.read_u32();
        for _ in 0..len {
            let mut hash: Hash = HASH_NULL;
            stream.read_bytes(&mut hash, HASH_SIZE);
            self.hashes.push(hash);
        }
    }
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct FindTransactions {
    pub addresses: Vec<Address>,
    pub tags: Vec<Hash>,
    pub approvees: Vec<Hash>,
}

impl FindTransactions { pub const SVUID : i32 = 16; }

impl Serializable for FindTransactions {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);

        stream.write_u32(self.addresses.len() as u32);
        for v in &self.addresses {
            stream.write_bytes(&v);
        }

        stream.write_u32(self.tags.len() as u32);
        for v in &self.tags {
            stream.write_bytes(&v);
        }

        stream.write_u32(self.approvees.len() as u32);
        for v in &self.approvees {
            stream.write_bytes(&v);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.addresses.clear();
        self.tags.clear();
        self.approvees.clear();

        let len = stream.read_u32();
        for _ in 0..len {
            let mut addr = ADDRESS_NULL;
            stream.read_bytes(&mut addr, ADDRESS_SIZE);
            self.addresses.push(addr);
        }

        let len = stream.read_u32();
        for _ in 0..len {
            let mut tag = HASH_NULL;
            stream.read_bytes(&mut tag, HASH_SIZE);
            self.tags.push(tag);
        }

        let len = stream.read_u32();
        for _ in 0..len {
            let mut approvee = HASH_NULL;
            stream.read_bytes(&mut approvee, HASH_SIZE);
            self.approvees.push(approvee);
        }
    }
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct FoundedTransactions {
    pub hashes: Vec<Hash>
}

impl FoundedTransactions { pub const SVUID : i32 = 16; }

impl Serializable for FoundedTransactions {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
        stream.write_u32(self.hashes.len() as u32);
        for hash in &self.hashes {
            stream.write_bytes(&hash);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.hashes.clear();

        let len = stream.read_u32();
        for _ in 0..len {
            let mut hash = HASH_NULL;
            stream.read_bytes(&mut hash, HASH_SIZE);
            self.hashes.push(hash);
        }
    }
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct GetTransactionsData {
    pub hashes: Vec<Hash>,
}

impl GetTransactionsData { pub const SVUID : i32 = 16; }

impl Serializable for GetTransactionsData {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);

        stream.write_u32(self.hashes.len() as u32);
        for v in &self.hashes {
            stream.write_bytes(&v);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.hashes.clear();

        let len = stream.read_u32();
        for _ in 0..len {
            let mut tag = HASH_NULL;
            stream.read_bytes(&mut tag, HASH_SIZE);
            self.hashes.push(tag);
        }
    }
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct TransactionsData {
    pub transactions: Vec<String>,
}

impl TransactionsData { pub const SVUID : i32 = 16; }

impl Serializable for TransactionsData {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);

        stream.write_u32(self.transactions.len() as u32);
        for tx in &self.transactions {
            stream.write_string(tx.clone())
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.transactions.clear();

        let len = stream.read_u32();
        for _ in 0..len {
            let tx = stream.read_string();
            self.transactions.push(tx);
        }
    }
}
