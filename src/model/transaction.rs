extern crate rustc_serialize;
extern crate crypto;

use network::packet::{Serializable, SerializedBuffer};
use self::crypto::digest::Digest;
use self::crypto::sha3::Sha3;
use std::ops::{Deref, DerefMut};

pub const MIN_WEIGHT_MAGNITUDE: u8 = 8;

pub const HASH_SIZE: usize = 20;
pub const ADDRESS_SIZE: usize = 21; // HASH_SIZE + 1 (checksum byte)
pub const TRANSACTION_SIZE: usize = 173 + 4;

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Hash([u8; HASH_SIZE]);
impl Deref for Hash {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}
impl DerefMut for Hash {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}
impl Serializable for Hash {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(0);
        stream.write_bytes(&self.0);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        stream.read_bytes(&mut self.0,HASH_SIZE);
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Address(pub [u8; ADDRESS_SIZE]);
impl Deref for Address {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}
impl DerefMut for Address {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}
impl Serializable for Address {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(0);
        stream.write_bytes(&self.0);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        stream.read_bytes(&mut self.0,ADDRESS_SIZE);
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Account(pub Address, pub u32);
impl Deref for Account {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}
impl DerefMut for Account {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}
impl Serializable for Account {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(0);
        stream.write_bytes(&self.0);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        stream.read_bytes(&mut self.0,ADDRESS_SIZE);
    }
}

pub const HASH_NULL : Hash = Hash([0u8; HASH_SIZE]);
pub const ADDRESS_NULL : Address = Address([0u8; ADDRESS_SIZE]);

#[derive(Debug, PartialEq)]
pub enum TransactionType {
    HashOnly,
    Full
}

#[derive(Debug, PartialEq)]
pub struct TransactionObject {
    pub address: Address,
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
    pub data_type: TransactionType,
}

pub struct Transaction {
    pub transaction: TransactionObject,
    pub bytes: SerializedBuffer,
}

impl Transaction {
    pub fn from_bytes(mut bytes: SerializedBuffer) -> Self {
        let mut transaction = TransactionObject::new();
        transaction.read_params(&mut bytes);

        Transaction {
            transaction,
            bytes
        }
    }

    pub fn from_object(mut transaction: TransactionObject) -> Self {
        let mut bytes = SerializedBuffer::new_with_size(TRANSACTION_SIZE);
        transaction.serialize_to_stream(&mut bytes);

        Transaction {
            transaction,
            bytes
        }
    }

    pub fn new() -> Self {
        let mut transaction = TransactionObject::new();
        let mut bytes = SerializedBuffer::new_with_size(TRANSACTION_SIZE);
        transaction.serialize_to_stream(&mut bytes);

        Transaction {
            transaction,
            bytes
        }
    }

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
        Hash(buf)
    }

    pub fn find_nonce(&self) -> Option<u64> {
        let mut nonce = 0;
        let mut sha = Sha3::sha3_256();
        let mut buf = [0u8; 32];

        let mut in_buf = SerializedBuffer::new_with_size(HASH_SIZE * 2 + 8);
        in_buf.write_bytes(&self.transaction.branch_transaction);
        in_buf.write_bytes(&self.transaction.trunk_transaction);
        in_buf.write_u64(nonce);

        'nonce_loop: loop {
            sha.input(&in_buf.buffer);
            sha.result(&mut buf);

            for i in 0..MIN_WEIGHT_MAGNITUDE as usize {
                let x = buf[i / 8];
                let y = (0b10000000 >> (i % 8)) as u8;

                if x & y != 0 {
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

    pub fn new() -> Self {
        TransactionObject::from_hash(HASH_NULL)
    }

    pub fn from_hash(hash: Hash) -> Self {
        TransactionObject {
            address: ADDRESS_NULL,
            attachment_timestamp: 0u64,
            attachment_timestamp_lower_bound: 0u64,
            attachment_timestamp_upper_bound: 0u64,
            branch_transaction: HASH_NULL,
            trunk_transaction: HASH_NULL,
            bundle: HASH_NULL,
            current_index: 0u32,
            hash,
            last_index: 0u32,
            nonce: 0u64,
            tag: HASH_NULL,
            timestamp: 0u64,
            value: 0u32,
            data_type: TransactionType::HashOnly,
        }
    }

    pub fn new_random() -> Self {
        use rand::Rng;
        use rand::thread_rng;

        let mut address = ADDRESS_NULL;
        let mut branch_transaction = HASH_NULL;
        let mut trunk_transaction = HASH_NULL;
        let mut bundle = HASH_NULL;
        let mut hash = HASH_NULL;

        let attachment_timestamp = 0u64;
        let attachment_timestamp_lower_bound = 0u64;
        let attachment_timestamp_upper_bound = 0u64;
        let timestamp = 0u64;
        let nonce = 0u64;
        let current_index = 0u32;
        let last_index = 0u32;
        let value = 0u32;

        thread_rng().fill_bytes(&mut branch_transaction);
        thread_rng().fill_bytes(&mut address);
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
            tag: HASH_NULL,
            timestamp,
            value,
            data_type: TransactionType::Full,
        }
    }
}

impl Serializable for TransactionObject {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(TransactionObject::SVUID);
        stream.write_bytes(&self.hash);
        stream.write_bytes(&self.address);
        stream.write_u64(self.attachment_timestamp);
        stream.write_u64(self.attachment_timestamp_lower_bound);
        stream.write_u64(self.attachment_timestamp_upper_bound);
        stream.write_bytes(&self.branch_transaction);
        stream.write_bytes(&self.trunk_transaction);
        stream.write_bytes(&self.bundle);
        stream.write_u32(self.current_index);
        stream.write_u32(self.last_index);
        stream.write_u64(self.nonce);
        stream.write_bytes(&self.tag);
        stream.write_u64(self.timestamp);
        stream.write_u32(self.value);
        let b = match self.data_type {
            TransactionType::HashOnly => 0,
            TransactionType::Full => 1,
        };
        stream.write_byte(b);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
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
        self.data_type =  match stream.read_byte() {
            1 => TransactionType::Full,
            _ => TransactionType::HashOnly
        };
    }
}

impl Serializable for Transaction {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        self.transaction.serialize_to_stream(stream);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.transaction.read_params(stream);
    }
}
