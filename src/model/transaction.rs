extern crate rustc_serialize;
extern crate crypto;
extern crate ntrumls;

use model::approvee::*;
use network::packet::{Serializable, SerializedBuffer};
use self::crypto::digest::Digest;
use self::crypto::sha3::Sha3;
use std::ops::{Deref, DerefMut};
use storage::hive::Hive;
use std::collections::HashSet;
use std::clone::Clone;
use self::ntrumls::{NTRUMLS, Signature, PrivateKey, PublicKey, PQParamSetID};
use utils::defines::AM;
use std::str::FromStr;
use self::rustc_serialize::{
    hex::{FromHex, ToHex},
    Encodable, Decodable, Encoder, Decoder
};

pub const HASH_SIZE: usize = 20;
pub const ADDRESS_SIZE: usize = 21;
pub const TRANSACTION_SIZE: usize = 173 + 4; // HASH_SIZE + 1 (checksum byte)

pub const HASH_NULL: Hash = Hash([0u8; HASH_SIZE]);
pub const ADDRESS_NULL: Address = Address([0u8; ADDRESS_SIZE]);

#[derive(PartialEq, Clone, Copy, Eq, Hash)]
pub struct Hash(pub [u8; HASH_SIZE]);

impl Hash {
    pub fn trailing_zeros(&self) -> u16 {
        let mut zeros = 0u16;
        for i in 0..HASH_SIZE as usize {
            let x = self.0[i / 8];
            let y = (0b10000000 >> (i % 8)) as u8;

            if x & y != 0 {
                break;
            }
            zeros += 1u16;
        }
        zeros
    }

    pub fn is_null(&self) -> bool {
        *self == HASH_NULL
    }
}

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
        stream.read_bytes(&mut self.0, HASH_SIZE);
    }
}

impl Encodable for Hash {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        let strs: Vec<String> = self.0.iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        s.emit_str(&format!("{}", strs.join("")))
    }
}

impl Decodable for Hash {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        let v = d.read_str()?.from_hex().map_err(|e| d.error("failed to decode Hash"))?;
        if v.len() < HASH_SIZE {
            return Err(d.error("invalid hash size"));
        }

        let mut hash = HASH_NULL.clone();
        hash.clone_from_slice(&v[..HASH_SIZE]);
        Ok(hash)
    }
}

impl super::super::std::fmt::Debug for Hash {
    fn fmt(&self, f: &mut super::super::std::fmt::Formatter) -> super::super::std::fmt::Result {
//        use self::rustc_serialize::hex::ToHex;
        let strs: Vec<String> = self.0.iter()
            .map(|b| format!("{:02X}", b))
            .collect();
        write!(f, "Hash({})", strs.join(""))
//        write!(f, "P{}", self.0.to_hex())
    }
}

#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum AddressError {
    InvalidAddress,
}

#[derive(PartialEq, Copy, Clone, Eq, Hash)]
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
        stream.read_bytes(&mut self.0, ADDRESS_SIZE);
    }
}

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("P") {
            return Err(AddressError::InvalidAddress);
        }

        match s[1..].to_string().from_hex() {
            Err(_) => return Err(AddressError::InvalidAddress),
            Ok(vec) => {
                let bytes:&[u8] = vec.as_ref();
                let mut ret_bytes = [0u8; 21];

                if bytes.len() == 21 {
                    ret_bytes.copy_from_slice(&bytes);
                    return Ok(Address(ret_bytes))
                } else {
                    return Err(AddressError::InvalidAddress);
                }
            }
        };
    }
}

impl Encodable for Address {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        s.emit_str(&format!("{:?}", self))
    }
}

impl Decodable for Address {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        let s = d.read_str()?;
        Ok(Address::from_str(&s).map_err(|e| d.error("failed to decode Address"))?)
    }
}

impl super::super::std::fmt::Debug for Address {
    fn fmt(&self, f: &mut super::super::std::fmt::Formatter) -> super::super::std::fmt::Result {
//        use self::rustc_serialize::hex::ToHex;
        let strs: Vec<String> = self.0.iter()
            .map(|b| format!("{:02X}", b))
            .collect();
        write!(f, "P{}", strs.join(""))
//        write!(f, "P{}", self.0.to_hex())
    }
}

impl Address {
    pub fn is_null(&self) -> bool {
        self.0 == [0u8; ADDRESS_SIZE]
    }

    pub fn verify(&self) -> bool {
        Address::calculate_checksum(&self) == self.0[ADDRESS_SIZE - 1]
    }

    pub fn calculate_checksum(bytes: &[u8]) -> u8 {
        let mut checksum_byte = 0u16;
        for (i, b) in bytes.iter().enumerate() {
            if i & 1 == 0 {
                checksum_byte += *b as u16;
            } else {
                checksum_byte += (*b as u16) * 2;
            }
            checksum_byte %= 256;
        }
        checksum_byte as u8
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
        stream.read_bytes(&mut self.0, ADDRESS_SIZE);
    }
}

impl Serializable for Signature {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_byte_array(&self.0)
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        if let Some(v) = stream.read_byte_array() {
            self.0 = v;
        } else {
            error!("error reading byte array");
        }
    }
}

#[derive(Debug, PartialEq, Clone, RustcEncodable, RustcDecodable)]
pub enum TransactionType {
    HashOnly,
    Full,
}

#[derive(Debug, PartialEq, Clone, RustcEncodable, RustcDecodable)]
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
    pub signature: Signature,
    pub signature_pubkey: PublicKey,
    pub snapshot: u32,
    pub solid: bool,
}

pub struct Transaction {
    pub object: TransactionObject,
    pub bytes: SerializedBuffer,
    pub weight_magnitude: u16,
    pub approvers: Option<Approvee>,
}

impl Transaction {
    pub fn get_current_index(&self) -> u32 {
        self.object.current_index.clone()
    }

    pub fn get_type(&self) -> TransactionType{
        self.object.data_type.clone()
    }

    pub fn get_hash(&self) -> Hash {
        self.object.hash.clone()
    }

    pub fn is_solid(&self) -> bool {
        self.object.solid
    }

    pub fn get_approvers(&self, hive: &AM<Hive>) -> HashSet<Hash> {
        let mut res = self.approvers.clone();
        match res {
            Some(aprv) => aprv.get_hashes(),
            None => match Approvee::load(hive, &self.object.hash){
                Some(aprv) => aprv.get_hashes(),
                None => HashSet::new()
            }
        }
    }

    pub fn from_hash(hash: Hash) -> Self {
        let mut transaction = TransactionObject::from_hash(hash);
        let mut bytes = SerializedBuffer::new_with_size(TRANSACTION_SIZE);
        transaction.serialize_to_stream(&mut bytes);
        Transaction {
            weight_magnitude: transaction.hash.trailing_zeros(),
            object: transaction,
            bytes,
            approvers: None,
        }
    }

    pub fn get_trunk_transaction_hash(&self) -> Hash {
        self.object.trunk_transaction.clone()
    }

    pub fn get_branch_transaction_hash(&self) -> Hash {
        self.object.branch_transaction.clone()
    }

    pub fn from_bytes(mut bytes: SerializedBuffer) -> Self {
        let mut transaction = TransactionObject::new();
        transaction.read_params(&mut bytes);

        Transaction {
            weight_magnitude: transaction.hash.trailing_zeros(),
            object: transaction,
            bytes,
            approvers: None,
        }
    }

    pub fn from_object(mut transaction: TransactionObject) -> Self {
        use network::packet::calculate_object_size;
        let transaction_size = calculate_object_size(&transaction);
        let mut bytes = SerializedBuffer::new_with_size(transaction_size);
        transaction.serialize_to_stream(&mut bytes);

        Transaction {
            weight_magnitude: transaction.hash.trailing_zeros(),
            object: transaction,
            bytes,
            approvers: None,
        }
    }

    pub fn new() -> Self {
        let mut transaction = TransactionObject::new();
        let mut bytes = SerializedBuffer::new_with_size(TRANSACTION_SIZE);
        transaction.serialize_to_stream(&mut bytes);

        Transaction {
            weight_magnitude: transaction.hash.trailing_zeros(),
            object: transaction,
            bytes,
            approvers: None,
        }
    }

    pub fn calculate_signature(&mut self, sk: &PrivateKey, pk: &PublicKey) -> Option<Signature> {
        let ntrumls = NTRUMLS::with_param_set(PQParamSetID::Security269Bit);
        println!("signing {:?} {:?} {:?}", self.object.hash, sk, pk);
        ntrumls.sign(&self.object.hash, sk, pk)
    }

    pub fn new_random() -> Self {
        let mut transaction = TransactionObject::new_random();
        let mut bytes = SerializedBuffer::new_with_size(TRANSACTION_SIZE);
        transaction.serialize_to_stream(&mut bytes);

        Transaction {
            weight_magnitude: transaction.hash.trailing_zeros(),
            object: transaction,
            bytes,
            approvers: None,
        }
    }

    pub fn calculate_hash(&mut self) -> Hash {
        if self.bytes.position() == 0 {
            self.object.serialize_to_stream(&mut self.bytes);
        }

        let mut sb = SerializedBuffer::new_with_size(ADDRESS_SIZE + 4 + 8);
        sb.write_bytes(&self.object.address);
        sb.write_u32(self.object.value);
        sb.write_u64(self.object.timestamp);

        let mut sha = Sha3::sha3_256();
        sha.input(&sb.buffer);

        let mut buf = [0u8; HASH_SIZE];
        sha.result(&mut buf);
        Hash(buf)
    }

    pub fn find_nonce(&self, mwm: u32) -> u64 {
        let mut nonce = 0;
        let mut sha = Sha3::sha3_256();
        let mut buf = [0u8; 32];

        let mut in_buf = SerializedBuffer::new_with_size(HASH_SIZE * 2 + 8);
        in_buf.write_bytes(&self.object.branch_transaction);
        in_buf.write_bytes(&self.object.trunk_transaction);
        in_buf.write_u64(nonce);

        'nonce_loop: loop {
            sha.input(&in_buf.buffer);
            sha.result(&mut buf);

            for i in 0..mwm as usize {
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
        nonce
    }
}

impl TransactionObject {
    pub const SVUID: i32 = 342631123;

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
            signature: Signature(vec![]),
            signature_pubkey: PublicKey(vec![]),
            snapshot: 0u32,
            solid: false
        }
    }

    pub fn new_random() -> Self {
        use rand::Rng;
        use rand::thread_rng;

        let mut signature = Signature(Vec::new());
        let mut signature_pubkey = PublicKey(Vec::new());
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
        let snapshot = 0u32;
        thread_rng().fill_bytes(&mut signature.0);
        thread_rng().fill_bytes(&mut branch_transaction);
        thread_rng().fill_bytes(&mut address);
        thread_rng().fill_bytes(&mut trunk_transaction);
        thread_rng().fill_bytes(&mut bundle);

        TransactionObject {
            signature,
            signature_pubkey,
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
            snapshot,
            solid: false,
        }
    }
    pub fn get_snapshot_index(&self) -> u32{
        return self.snapshot;
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
        stream.write_byte_array(&self.signature.0);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        stream.read_bytes(&mut self.hash, HASH_SIZE);
        stream.read_bytes(&mut self.address, ADDRESS_SIZE);
        self.attachment_timestamp = stream.read_u64();
        self.attachment_timestamp_lower_bound = stream.read_u64();
        self.attachment_timestamp_upper_bound = stream.read_u64();
        stream.read_bytes(&mut self.branch_transaction, HASH_SIZE);
        stream.read_bytes(&mut self.trunk_transaction, HASH_SIZE);
        stream.read_bytes(&mut self.bundle, HASH_SIZE);
        self.current_index = stream.read_u32();
        self.last_index = stream.read_u32();
        self.nonce = stream.read_u64();
        stream.read_bytes(&mut self.tag, HASH_SIZE);
        self.timestamp = stream.read_u64();
        self.value = stream.read_u32();
        self.data_type = match stream.read_byte() {
            1 => TransactionType::Full,
            _ => TransactionType::HashOnly
        };
        self.signature = Signature(stream.read_byte_array().unwrap());
    }
}

impl Serializable for Transaction {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        self.object.serialize_to_stream(stream);
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        self.object.read_params(stream);
    }
}

// TODO: return Result
pub fn validate_transaction(transaction: &mut Transaction, mwm: u32) -> bool {
    // check hash
    let calculated_hash = transaction.calculate_hash();
    if transaction.object.hash != calculated_hash {
        println!(1);
        return false;
    }

    // check nonce
    let mut nonce = 0;
    let mut sha = Sha3::sha3_256();
    let mut buf = [0u8; 32];

    let mut in_buf = SerializedBuffer::new_with_size(HASH_SIZE * 2 + 8);
    in_buf.write_bytes(&transaction.object.branch_transaction);
    in_buf.write_bytes(&transaction.object.trunk_transaction);
    in_buf.write_u64(transaction.object.nonce);

    sha.input(&in_buf.buffer);
    sha.result(&mut buf);

//    for i in 0..MIN_WEIGHT_MAGNITUDE as usize {
    for i in 0..mwm as usize {
        let x = buf[i / 8];
        let y = (0b10000000 >> (i % 8)) as u8;

        if x & y != 0 {
            println!(2);
            return false;
        }
    }

    // check signature
    let sign = &transaction.object.signature;
    let pk = &transaction.object.signature_pubkey;
    let ntrumls = NTRUMLS::with_param_set(PQParamSetID::Security269Bit);
//    let pk = PublicKey(address_from.0.to_vec());
    println!("{:?}", pk);
    println!("{:?}", transaction.object.hash);
    println!("{:?}", transaction.object.signature);
    ntrumls.verify(&transaction.object.hash, sign, &pk)
}