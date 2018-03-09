extern crate rustc_serialize;
extern crate crypto;

use network::packet::{Packet, SerializedBuffer};
use self::crypto::digest::Digest;
use self::crypto::sha3::Sha3;

pub const MIN_WEIGHT_MAGNITUDE: u8 = 1;

pub const HASH_SIZE: usize = 20;
pub const ADDRESS_SIZE: usize = 21; // HASH_SIZE + 1 (checksum byte)
pub const TRANSACTION_SIZE: usize = 173 + 4;

pub type Hash = [u8; HASH_SIZE];

//#[derive(Debug)]
pub enum TransactionType {
    HashOnly,
    Full
}

//#[derive(Debug)]
pub struct Transaction {
    pub transaction: TransactionObject,
    pub bytes: SerializedBuffer,
}

pub struct TransactionObject {
    pub address: [u8; ADDRESS_SIZE],
    pub attachment_timestamp: u64,
    pub attachment_timestamp_lower_bound: u64,
    pub attachment_timestamp_upper_bound: u64,
    pub branch_transaction: Hash,
    pub trunk_transaction: Hash,
    pub bundle: Hash,
    pub current_index: u32,
    pub hash: Hash,
    pub last_index: u32,
    pub nonce: u64,
    pub tag: Hash,
    pub timestamp: u64,
    pub value: u32,

    pub typ: TransactionType,
}

impl Transaction {
    pub fn new_random() -> Self {
        let mut transaction = TransactionObject::new_random();
        let mut bytes = SerializedBuffer::new_with_size(TRANSACTION_SIZE);
        transaction.serialize_to_stream(&mut bytes);

        Transaction {
            transaction,
            bytes
        }
    }

    pub fn get_hash(&mut self) -> Hash {
        if self.bytes.position() == 0 {
            self.transaction.serialize_to_stream(&mut self.bytes);
        }

        let mut sha = Sha3::sha3_256();
        sha.input(&self.bytes.buffer);

        let mut buf = [0u8; HASH_SIZE];
        sha.result(&mut buf);
        buf
    }

    pub fn find_nonce(&self) -> Option<u64> {
        let mut nonce = 0;
        let mut sha = Sha3::sha3_256();
        let mut buf = [0u8; 32];

        let mut in_buf = SerializedBuffer::new_with_size(HASH_SIZE * 2 + 8);
        in_buf.write_bytes(&self.transaction.branch_transaction, HASH_SIZE);
        in_buf.write_bytes(&self.transaction.trunk_transaction, HASH_SIZE);
        in_buf.write_u64(nonce);

        'nonce_loop: loop {
            sha.input(&in_buf.buffer);
            sha.result(&mut buf);

            for i in 0..(MIN_WEIGHT_MAGNITUDE+1) as usize {
                if buf[i] != 0 {
                    sha.reset();
                    nonce += 1;
                    in_buf.set_position(40);
                    in_buf.write_u64(nonce);
                    continue 'nonce_loop;
                }
            }
            break;
        }
        Some(nonce)
    }
}

impl TransactionObject {
    pub const SVUID : i32 = 342631123;

    pub fn new(hash: [u8; HASH_SIZE]) -> Self {
        TransactionObject {
            address: [0u8; ADDRESS_SIZE],
            attachment_timestamp: 0u64,
            attachment_timestamp_lower_bound: 0u64,
            attachment_timestamp_upper_bound: 0u64,
            branch_transaction: [0u8; HASH_SIZE],
            trunk_transaction: [0u8; HASH_SIZE],
            bundle: [0u8; HASH_SIZE],
            current_index: 0u32,
            hash,
            last_index: 0u32,
            nonce: 0u64,
            tag: *b"00000000000000000000",
            timestamp: 0u64,
            value: 0u32,
            typ: TransactionType::HashOnly,
        }
    }

    pub fn new_random() -> Self {
        use rand::Rng;
        use rand::thread_rng;

        let mut address = [0u8; ADDRESS_SIZE];
        let mut branch_transaction = [0u8; HASH_SIZE];
        let mut trunk_transaction = [0u8; HASH_SIZE];
        let mut bundle = [0u8; HASH_SIZE];
        let mut hash = [0u8; HASH_SIZE];

        let attachment_timestamp = 0u64;
        let attachment_timestamp_lower_bound = 0u64;
        let attachment_timestamp_upper_bound = 0u64;
        let timestamp = 0u64;
        let nonce = 0u64;
        let current_index = 0u32;
        let last_index = 0u32;
        let value = 0u32;

        thread_rng().fill_bytes(&mut address);
        thread_rng().fill_bytes(&mut branch_transaction);
        thread_rng().fill_bytes(&mut trunk_transaction);
        thread_rng().fill_bytes(&mut bundle);

        TransactionObject {
            address,
            attachment_timestamp,
            attachment_timestamp_lower_bound,
            attachment_timestamp_upper_bound,
            branch_transaction,
            trunk_transaction,
            bundle,
            current_index,
            hash,
            last_index,
            nonce,
            tag: *b"00000000000000000000",
            timestamp,
            value,
            typ: TransactionType::Full,
        }
    }
}

impl Packet for TransactionObject {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(TransactionObject::SVUID);
        stream.write_bytes(&self.hash,HASH_SIZE);
        stream.write_bytes(&self.address,ADDRESS_SIZE);
        stream.write_u64(self.attachment_timestamp);
        stream.write_u64(self.attachment_timestamp_lower_bound);
        stream.write_u64(self.attachment_timestamp_upper_bound);
        stream.write_bytes(&self.branch_transaction,HASH_SIZE);
        stream.write_bytes(&self.trunk_transaction,HASH_SIZE);
        stream.write_bytes(&self.bundle,HASH_SIZE);
        stream.write_u32(self.current_index);
        stream.write_u32(self.last_index);
        stream.write_u64(self.nonce);
        stream.write_bytes(&self.tag,HASH_SIZE);
        stream.write_u64(self.timestamp);
        stream.write_u32(self.value);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer, error: bool) {
        stream.read_bytes(&mut self.hash,HASH_SIZE);
        stream.read_bytes(&mut self.address,ADDRESS_SIZE);
        self.attachment_timestamp = stream.read_u64();
        self.attachment_timestamp_lower_bound = stream.read_u64();
        self.attachment_timestamp_upper_bound = stream.read_u64();
        stream.read_bytes(&mut self.branch_transaction,HASH_SIZE);
        stream.read_bytes(&mut self.trunk_transaction,HASH_SIZE);
        stream.read_bytes(&mut self.bundle,HASH_SIZE);
        self.current_index = stream.read_u32();
        self.last_index = stream.read_u32();
        self.nonce = stream.read_u64();
        stream.read_bytes(&mut self.tag,HASH_SIZE);
        self.timestamp = stream.read_u64();
        self.value = stream.read_u32();
    }
}
