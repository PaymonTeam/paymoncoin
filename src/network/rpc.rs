use network::packet::{Packet, SerializedBuffer};
use model::transaction::Transaction;

enum RPC {

}

pub struct KeepAlive {
//    stream: SerializedBuffer,
}

impl KeepAlive {
    pub const SVUID : i32 = 2;
}

impl Packet for KeepAlive {
    fn read_params(&mut self, stream: &mut SerializedBuffer, error: bool) {
    }

    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(KeepAlive::SVUID);
    }
}

pub struct GetInfo {
    pub name: String
}

impl GetInfo {
    pub const SVUID : i32 = 342834823;
}

impl Packet for GetInfo {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(GetInfo::SVUID);
        stream.write_string(self.name.clone());
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer, error: bool) {
        self.name = stream.read_string();
    }
}

pub const HASH_SIZE: usize = 20;
pub const ADDRESS_SIZE: usize = 21; // HASH_SIZE + 1 (checksum byte)

impl Packet for Transaction {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        use std::mem::size_of;
        stream.write_i32(Transaction::SVUID);
        stream.write_bytes(&self.address,ADDRESS_SIZE);
        stream.write_u64(self.attachment_timestamp);
        stream.write_u64(self.attachment_timestamp_lower_bound);
        stream.write_u64(self.attachment_timestamp_upper_bound);
        stream.write_bytes(&self.branch_transaction,HASH_SIZE);
        stream.write_bytes(&self.trunk_transaction,HASH_SIZE);
        stream.write_bytes(&self.bundle,HASH_SIZE);
        stream.write_u32(self.current_index);
        stream.write_bytes(&self.hash,HASH_SIZE);
        stream.write_u32(self.last_index);
        stream.write_u64(self.nonce);
        stream.write_string(self.tag.clone());
        stream.write_u64(self.timestamp);
        stream.write_u32(self.value);

    }

    fn read_params(&mut self, stream: &mut SerializedBuffer, error: bool) {
        stream.read_bytes(&mut self.address,ADDRESS_SIZE);
        self.attachment_timestamp = stream.read_u64();
        self.attachment_timestamp_lower_bound = stream.read_u64();
        self.attachment_timestamp_upper_bound = stream.read_u64();
        stream.read_bytes(&mut self.branch_transaction,HASH_SIZE);
        stream.read_bytes(&mut self.trunk_transaction,HASH_SIZE);
        stream.read_bytes(&mut self.bundle,HASH_SIZE);
        self.current_index = stream.read_u32();
        stream.read_bytes(&mut self.hash,HASH_SIZE);
        self.last_index = stream.read_u32();
        self.nonce = stream.read_u64();
        self.tag = stream.read_string();
        self.timestamp = stream.read_u64();
        self.value = stream.read_u32();
    }
}



