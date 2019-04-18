extern crate crypto;
extern crate base64;

use serde_pm_derive;
use std::fmt;
use std::hash;
use serde_pm::{Identifiable, SerializedBuffer, from_stream, to_buffer};
use self::crypto::digest::Digest;
use self::crypto::sha3::Sha3;
use std::ops::{Deref, DerefMut};
use crate::storage::hive::Hive;
use std::collections::HashSet;
use std::clone::Clone;
use crate::secp256k1;
use crate::utils::defines::AM;
use std::str::FromStr;
use hex;
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde_bytes;

pub type PublicKey = secp256k1::PublicKey;
pub type PrivateKey = secp256k1::SecretKey;
pub type Signature = secp256k1::Signature;
pub type Message = secp256k1::Message;

pub const HASH_SIZE: usize = 32;
pub const ADDRESS_SIZE: usize = 21;

pub const HASH_NULL: Hash = Hash([0u8; HASH_SIZE]);
pub const ADDRESS_NULL: Address = Address([0u8; ADDRESS_SIZE]);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hash(pub [u8; HASH_SIZE]);

impl Hash {
    pub fn sha3_256(bytes: &[u8]) -> Self {
        let mut sha = Sha3::sha3_256();
        sha.input(bytes);
        let mut buf = [0u8; HASH_SIZE];
        sha.result(&mut buf);
        Hash(buf)
    }

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

impl From<Hash> for Message {
    fn from(h: Hash) -> Self {
        Message::from_slice(&h.0).unwrap()
    }
}

impl Deref for Hash {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl DerefMut for Hash {
    fn deref_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl hash::Hash for Hash {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        state.write(&self.0);
    }

    fn hash_slice<H: hash::Hasher>(data: &[Self], state: &mut H) where Self: Sized {
        unimplemented!()
    }
}

impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> where
        S: Serializer {
        serializer.serialize_str(&hex::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error> where
        D: Deserializer<'de> {
        deserializer.deserialize_str(StringVisitor::<Hash>::new())
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Hash({})", hex::encode(self.0))
    }
}

impl Default for Hash {
    fn default() -> Hash {
        HASH_NULL
    }
}

impl FromStr for Hash {
    type Err = HashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match hex::decode(s.to_string()) {
            Err(_) => return Err(HashError::InvalidHash),
            Ok(vec) => {
                let bytes: &[u8] = vec.as_ref();
                let mut ret_bytes = [0u8; HASH_SIZE];

                if bytes.len() == HASH_SIZE {
                    ret_bytes.copy_from_slice(&bytes);
                    return Ok(Hash(ret_bytes))
                } else {
                    return Err(HashError::InvalidHash);
                }
            }
        };
    }
}

#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum AddressError {
    InvalidAddress,
}

#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum HashError {
    InvalidHash,
}

#[derive(PartialEq, Copy, Clone, Eq, Hash, Ord, PartialOrd)]
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

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("P") {
            return Err(AddressError::InvalidAddress);
        }

        match hex::decode(s[1..].to_string()) {
            Err(_) => return Err(AddressError::InvalidAddress),
            Ok(vec) => {
                let bytes:&[u8] = vec.as_ref();
                let mut ret_bytes = [0u8; ADDRESS_SIZE];

                if bytes.len() == ADDRESS_SIZE {
                    ret_bytes.copy_from_slice(&bytes);
                    return Ok(Address(ret_bytes))
                } else {
                    return Err(AddressError::InvalidAddress);
                }
            }
        };
    }
}

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> where
        S: Serializer {
        serializer.serialize_str(&format!("{:?}", self))
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error> where
        D: Deserializer<'de> {
        deserializer.deserialize_str(StringVisitor::<Address>::new())
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let strs: Vec<String> = self.0.iter().map(|b| format!("{:02X}", b)).collect();
        write!(f, "P{}", strs.join(""))
    }
}

impl Default for Address {
    fn default() -> Address {
        ADDRESS_NULL
    }
}

impl Address {
    pub fn is_null(&self) -> bool {
        self.0 == [0u8; ADDRESS_SIZE]
    }

    pub fn verify(&self) -> bool {
        Address::calculate_checksum(&self[..ADDRESS_SIZE-1]) == self.0[ADDRESS_SIZE - 1]
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

    // TODO: may cause panic?
    pub fn from_public_key(pk: &PublicKey) -> Self {
//        let mut buf = [0u8; 32];
//        let mut sha = Sha3::sha3_256();
//        sha.input(&[0u8, 12]);
//        sha.result(&mut buf);

        let mut sha = Sha3::sha3_256();
        sha.input(&pk.serialize_uncompressed());

        let mut buf = [0u8; 32];
        sha.result(&mut buf);

        let offset = 32 - ADDRESS_SIZE + 1;
        let checksum_byte = Address::calculate_checksum(&buf[offset..]);

        let mut addr = ADDRESS_NULL;
        addr[..(ADDRESS_SIZE-1)].copy_from_slice(&buf[offset..32]);
        addr[ADDRESS_SIZE-1] = checksum_byte;
        addr
    }
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Copy, Clone)]
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

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    HashOnly,
    Full,
}

mod serde_secp_public {
    use serde::*;
    use super::StringVisitor;

    struct IntVisitor;

    impl<'de> de::Visitor<'de> for IntVisitor {
        type Value = u32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("integer")
        }
    }

    struct BytesVisitor;

    impl<'de> de::Visitor<'de> for BytesVisitor {
        type Value = ();

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("bytes")
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<secp256k1::PublicKey, D::Error> where D: de::Deserializer<'de> {
        let s = d.deserialize_str(StringVisitor::<String>::new())?;
        debug!("s={}", s);
        let data = &hex::decode(&s).unwrap();
        debug!("data={:?}", data);
        let key = secp256k1::PublicKey::from_slice(data).unwrap();
        debug!("key={}", key);
        debug!("key={:?}", key);
        debug!("key={:?}", &key.serialize()[..]);
        Ok(key)
    }

    pub fn serialize<S>(t: &secp256k1::PublicKey, s: S) -> Result<S::Ok, S::Error> where S: Serializer {
        s.serialize_str(&hex::encode(&t.serialize()[..]))
    }
}

mod serde_secp_signature {
    use serde::*;
    use super::StringVisitor;

    pub fn deserialize<'de, D>(d: D) -> Result<secp256k1::Signature, D::Error> where D: de::Deserializer<'de> {
        let s = d.deserialize_str(StringVisitor::<String>::new())?;
        Ok(secp256k1::Signature::from_compact(&hex::decode(&s).unwrap()).unwrap())
    }

    pub fn serialize<S>(t: &secp256k1::Signature, s: S) -> Result<S::Ok, S::Error> where S: Serializer {
        s.serialize_str(&hex::encode(&t.serialize_compact()[..]))
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, PMIdentifiable)]
#[pm_identifiable(id = "0x146C22D3")]
pub struct TransactionObject {
    pub address: Address,
    pub attachment_timestamp: u64,
    pub attachment_timestamp_lower_bound: u64,
    pub attachment_timestamp_upper_bound: u64,
    pub branch_transaction: Hash,
    pub trunk_transaction: Hash,
    pub hash: Hash,
    pub nonce: u64,
    pub tag: Hash,
    pub timestamp: u64,
    pub value: u32, // TODO: change to u64?
    pub data_type: TransactionType,
    #[serde(with = "serde_secp_signature")]
    pub signature: Signature,
    #[serde(with = "serde_secp_public")]
    pub signature_pubkey: PublicKey,
    pub snapshot: u32,
    pub solid: bool,
    pub height: u64,
    pub data: String,
}

#[derive(Clone, Serialize, Deserialize, PMIdentifiable)]
#[pm_identifiable(id = "0x73f73812")]
pub struct Transaction {
//    #[serde(flatten)]
    pub object: TransactionObject,
    #[serde(skip)]
    pub bytes: SerializedBuffer,
    #[serde(skip)]
    pub weight_magnitude: u16,
    #[serde(skip)]
    pub approvers: Option<Approvee>,
}

impl Default for Transaction {
    fn default() -> Self {
        Transaction::new()
    }
}

impl Transaction {
    pub fn get_type(&self) -> TransactionType {
        self.object.data_type.clone()
    }

    pub fn get_hash(&self) -> Hash {
        self.object.hash.clone()
    }

    pub fn is_solid(&self) -> bool {
        self.object.solid
    }

    pub fn get_approvers(&mut self, hive: &AM<Hive>) -> HashSet<Hash> {
        let res = self.approvers.clone();
        match res {
            Some(aprv) => aprv.get_hashes(),
            None => match Approvee::load(hive, &self.get_hash()) {
                Some(aprv) => {
                    let hashes = aprv.get_hashes();
                    self.approvers = Some(Approvee::new(hashes.clone()));
                    hashes
                },
                None => {
                    let empty_set = HashSet::new();
                    self.approvers = Some(Approvee::new(empty_set.clone()));
                    empty_set
                }
            }
        }
    }

    pub fn update_height(&mut self, height: u64) {
        self.object.height = height;
    }

    pub fn get_height(&self) -> u64 {
        self.object.height
    }

    // Generates empty transaction with hash
    pub fn from_hash(hash: Hash) -> Self {
        let transaction = TransactionObject::from_hash(hash);
        let bytes = to_buffer(&transaction).unwrap();
//        let mut bytes = SerializedBuffer::new_with_size(TRANSACTION_SIZE);
//        transaction.serialize_to_stream(&mut bytes);
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
        let transaction = from_stream::<TransactionObject>(&mut bytes).expect("failed to create tx from bytes");
//        let mut transaction = TransactionObject::new();
//        transaction.read_params(&mut bytes);
//        transaction.data_type = TransactionType::Full;

        Transaction {
            weight_magnitude: transaction.hash.trailing_zeros(),
            object: transaction,
            bytes,
            approvers: None,
        }
    }

    pub fn from_object(transaction: TransactionObject) -> Self {
//        use network::packet::calculate_object_size;
//        let transaction_size = calculate_object_size(&transaction);
//        let mut bytes = SerializedBuffer::new_with_size(transaction_size);
//        transaction.serialize_to_stream(&mut bytes);
        use serde_pm::to_buffer;
        let bytes = to_buffer(&transaction).unwrap();

        Transaction {
            weight_magnitude: transaction.hash.trailing_zeros(),
            object: transaction,
            bytes,
            approvers: None,
        }
    }

    pub fn new() -> Self {
        let transaction = TransactionObject::new();
        let bytes = to_buffer(&transaction).unwrap();
//        let mut bytes = SerializedBuffer::new_with_size(TRANSACTION_SIZE);
//        transaction.serialize_to_stream(&mut bytes);

        Transaction {
            weight_magnitude: transaction.hash.trailing_zeros(),
            object: transaction,
            bytes,
            approvers: None,
        }
    }

    pub fn calculate_signature(&mut self, sk: &PrivateKey, _pk: &PublicKey) -> Signature {
        use secp256k1::*;
        let mut secp = Secp256k1::<All>::new();
        debug!("signing {:?}", self.object.hash);
        secp.sign(&Message::from_slice(&self.object.hash).unwrap(), sk)
    }

//    pub fn new_random() -> Self {
//        let transaction = TransactionObject::new_random();
//        let bytes = to_buffer(&transaction).unwrap();
//
//        Transaction {
//            weight_magnitude: transaction.hash.trailing_zeros(),
//            object: transaction,
//            bytes,
//            approvers: None,
//        }
//    }

    pub fn calculate_hash(&mut self) -> Hash {
        if self.bytes.position() == 0 {
            // TODO: make convenient function for this
            self.bytes.write_bytes(&to_buffer(&self.object).unwrap());
//            self.object.serialize_to_stream(&mut self.bytes);
        }

        let mut sb = SerializedBuffer::new_with_size(ADDRESS_SIZE + 4 + 8 + HASH_SIZE + self.object.data.len());
        sb.write_bytes(&self.object.address);
        sb.write_u32(self.object.value);
        sb.write_u64(self.object.timestamp);
        sb.write_bytes(&self.object.tag);
        sb.write_bytes(self.object.data.as_bytes());
        sb.rewind();

        let mut sha = Sha3::sha3_256();
        sha.input(&sb);

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

    pub fn update_solidity(&mut self, solid: bool) -> bool {
        if solid != self.object.solid {
            self.object.solid = solid;
            return true;
        }

        false
    }

    /* Converts transaction data to Base64 */
    pub fn get_base64_data(&mut self) -> String {
        if self.bytes.position() == 0 {
//            self.object.serialize_to_stream(&mut self.bytes);
            self.bytes.write_bytes(&to_buffer(&self.object).unwrap());
        }
        return base64::encode(&self.bytes.buffer[0..self.bytes.limit()]);
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
            hash,
            nonce: 0u64,
            tag: HASH_NULL,
            timestamp: 0u64,
            value: 0u32,
            data_type: TransactionType::HashOnly,
            signature: Signature::from_der(&[48, 69, 2, 33, 0, 198, 75, 25, 36, 21, 119, 72, 101, 39, 51, 196, 18, 148, 229, 246, 227, 149, 195, 98, 106, 142, 145, 31, 55, 66, 164, 184, 173, 79, 219, 146, 47, 2, 32, 52, 123, 165, 205, 214, 41, 2, 126, 245, 132, 110, 184, 92, 148, 82, 246, 49, 46, 10, 234, 105, 118, 37, 214, 107, 32, 36, 72, 243, 233, 97, 143]).unwrap(),
            signature_pubkey: PublicKey::from_slice(&[3, 27, 132, 197, 86, 123, 18, 100, 64, 153, 93, 62, 213, 170, 186, 5, 101, 215, 30, 24, 52, 96, 72, 25, 255, 156, 23, 245, 233, 213, 221, 7, 143]).unwrap(),
            snapshot: 0u32,
            solid: false,
            height: 0,
            data: String::default(),
        }
    }

//    pub fn new_random() -> Self {
//        use rand::Rng;
//        use rand::thread_rng;
//
//        let mut signature = Signature(Vec::new());
//        let signature_pubkey = PublicKey(Vec::new());
//        let mut address = ADDRESS_NULL;
//        let mut branch_transaction = HASH_NULL;
//        let mut trunk_transaction = HASH_NULL;
//        let mut bundle = HASH_NULL;
//        let hash = HASH_NULL;
//
//        let attachment_timestamp = 0u64;
//        let attachment_timestamp_lower_bound = 0u64;
//        let attachment_timestamp_upper_bound = 0u64;
//        let timestamp = 0u64;
//        let nonce = 0u64;
//        let _current_index = 0u32;
//        let _last_index = 0u32;
//        let value = 0u32;
//        let snapshot = 0u32;
//        thread_rng().fill_bytes(&mut signature.0);
//        thread_rng().fill_bytes(&mut branch_transaction);
//        thread_rng().fill_bytes(&mut address);
//        thread_rng().fill_bytes(&mut trunk_transaction);
//        thread_rng().fill_bytes(&mut bundle);
//
//        TransactionObject {
//            signature,
//            signature_pubkey,
//            address,
//            attachment_timestamp,
//            attachment_timestamp_lower_bound,
//            attachment_timestamp_upper_bound,
//            branch_transaction,
//            trunk_transaction,
//            hash,
//            nonce,
//            tag: HASH_NULL,
//            timestamp,
//            value,
//            data_type: TransactionType::Full,
//            snapshot,
//            solid: false,
//            height: 0,
//            data: String::default(),
//        }
//    }

    pub fn get_snapshot_index(&self) -> u32{
        return self.snapshot;
    }
}

// TODO: return Result
pub fn validate_transaction(transaction: &mut Transaction, mwm: u32) -> bool {
    // check hash
    let calculated_hash = transaction.calculate_hash();
    if transaction.object.hash != calculated_hash {
        error!("wrong hash {:?}", calculated_hash);
        return false;
    }

    // check nonce
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
            error!("wrong mwm {:?}", buf);
            return false;
        }
    }

    // check signature
    let sign = &transaction.object.signature;
    let pk = &transaction.object.signature_pubkey;
    debug!("pk: {:?}", pk);
    debug!("signature: {:?}", sign);
    debug!("signature: {}", sign);
//    let ntrumls = NTRUMLS::with_param_set(PQParamSetID::Security269Bit);
//    ntrumls.verify(&transaction.object.hash, sign, &pk)
    use secp256k1::*;

    let mut secp = Secp256k1::<All>::new();
    let message = transaction.object.hash.into();
    debug!("msg={:?}", message);
    secp.verify(&message, sign, pk).is_ok()
}

struct StringVisitor<T: FromStr>(std::marker::PhantomData<T>);

use serde::de;

impl<T: FromStr> StringVisitor<T> {
    fn new() -> Self {
        StringVisitor(std::marker::PhantomData{})
    }
}

impl<'de, T> serde::de::Visitor<'de> for StringVisitor<T>
    where T: FromStr
{
    type Value = T;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        T::from_str(v).map_err(|_| E::custom(format!("invalid address")))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        T::from_str(&v).map_err(|_| E::custom(format!("invalid address")))
    }
}


#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Approvee {
    set: HashSet<Hash>,
}

impl Approvee {
    pub fn new_empty() -> Self {
        Approvee {
            set: HashSet::new(),
        }
    }

    pub fn new(set: HashSet<Hash>) -> Self {
        Approvee {
            set,
        }
    }

    pub fn load(hive: &AM<Hive>, hash: &Hash) -> Option<Self> {
        use std::iter::FromIterator;

        if let Ok(hive) = hive.lock() {
            return match hive.storage_load_approvee(hash) {
                Some(vec_h) => Some(Approvee::new(HashSet::from_iter(vec_h.into_iter()))),
                None => None
            };
        } else {
            return None;
        }
    }

    pub fn get_hashes(&self) -> HashSet<Hash> {
        return self.set.clone();
    }
}