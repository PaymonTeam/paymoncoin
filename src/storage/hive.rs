extern crate rand;
extern crate crypto;
extern crate log;

use self::crypto::digest::Digest;
use self::crypto::sha3::Sha3;
use crate::rocksdb::{DBIterator, DB, Options, IteratorMode, Direction, Column, WriteOptions, ReadOptions, Writable};
use std::io;
use std::num;
use std::sync::Arc;
use std::collections::{HashSet, HashMap, LinkedList};

use crate::model::{Milestone, MilestoneObject};
use crate::model::transaction_validator::TransactionError;
use crate::model::transaction::*;
use crate::model::approvee::Approvee;
use crate::model::{StateDiffObject, StateDiff};
use crate::model::contracts_manager::ContractInputOutput;
use serde::{Serialize, Deserialize};
use serde_pm::{SerializedBuffer, to_buffer, to_boxed_buffer, from_stream};
use std::time;
use std::str::FromStr;
use hex;
use std::env;

pub const SUPPLY : u64 = 1_000_000_000;

static CF_NAMES: [&str; 8] = [
    "transaction",
    "transaction-metadata",
    "address",
    "address_transactions",
    "approvee",
    "milestone",
    "state_diff",
    "contracts"
];

pub enum Error {
    IO(io::Error),
    Parse(num::ParseIntError),
    Str(String),
}

#[derive(Copy, PartialEq, Eq, Clone, Debug, Hash)]
pub enum CFType {
    Transaction = 0,
    TransactionMetadata,
    Address,
    AddressTransactions,
    Approvee,
    Milestone,
    StateDiff,
    Contracts,
}

pub struct Hive {
    db: DB,
    balances: HashMap<Address, u32>,
}

impl Hive {
    pub fn new() -> Self {
        let db = Hive::init_db();

        Hive {
            db,
            balances: HashMap::new(),
        }
    }

    pub fn init(&mut self) {}

    fn clear_db(db: &mut DB) {
        for name in CF_NAMES.iter() {
            let mut handle = db.cf_handle(name).unwrap();
            let mut it = db.iterator_cf(handle, IteratorMode::Start).unwrap();
            for (k,_) in it {
                db.delete_cf(handle, k.as_ref());
            }
        }
    }

    pub fn put_approvee(&mut self, approved: Hash, approvee: Hash) -> bool {
        self.storage_merge(CFType::Approvee, &approved, &approvee)
    }

    pub fn put_state_diff(&mut self, state_diff: &StateDiff) -> bool {
        let hash = &state_diff.hash;
        let state_diff = &state_diff.state_diff_object;

        self.storage_put(CFType::StateDiff, hash, state_diff)
    }

    pub fn put_address_transaction(&mut self, address: Address, transaction_hash: Hash) -> bool {
        self.storage_merge(CFType::AddressTransactions, &address, &transaction_hash)
    }

    pub fn storage_latest_milestone(&self) -> Option<MilestoneObject> {
        let mut it = self.db.iterator_cf(self.db.cf_handle(CF_NAMES[CFType::Milestone as usize])
                                             .unwrap(), IteratorMode::End).unwrap();
        match it.next() {
            Some((key, bytes)) => {
                let index = from_stream(&mut SerializedBuffer::from_slice(&key)).unwrap();
                let hash = from_stream(&mut SerializedBuffer::from_slice(&bytes)).unwrap();
//                let mut index = 0u32;
//                let mut hash = HASH_NULL;
//                index.read_params(&mut SerializedBuffer::from_slice(&key));
//                hash.read_params(&mut SerializedBuffer::from_slice(&bytes));

                Some(MilestoneObject {
                    index,
                    hash
                })
            }
            None => {
                warn!("get latest milestone from storage error");
                None
            }
        }
    }

    pub fn find_closest_next_milestone(&self, index: u32, testnet: bool, milestone_start_index: u32) -> Option<MilestoneObject> {
        if !testnet && index <= milestone_start_index {
            return self.storage_first_milestone();
        }

        return self.storage_next_milestone(index);
    }

    pub fn storage_first_milestone(&self) -> Option<MilestoneObject> {
        let mut it = self.db.iterator_cf(self.db.cf_handle(CF_NAMES[CFType::Milestone as usize]).unwrap(), IteratorMode::Start).unwrap();

        match it.next() {
            Some((key, bytes)) => {
                let index = from_stream(&mut SerializedBuffer::from_slice(&key)).unwrap();
                let hash = from_stream(&mut SerializedBuffer::from_slice(&bytes)).unwrap();
//                let mut index = 0u32;
//                let mut hash = HASH_NULL;
//                index.read_params(&mut SerializedBuffer::from_slice(&key));
//                hash.read_params(&mut SerializedBuffer::from_slice(&bytes));

                Some(MilestoneObject {
                    index,
                    hash
                })
            }
            None => {
                warn!("get first milestone from storage error");
                None
            }
        }
    }

    pub fn storage_next_milestone(&self, index: u32) -> Option<MilestoneObject> {
        let key = to_buffer(&index).unwrap(); //get_serialized_object(&index, false);
        let mut it = self.db.iterator_cf(self.db.cf_handle(CF_NAMES[CFType::Milestone as usize]).unwrap(), IteratorMode::From(&key, Direction::Forward)).unwrap();

        match it.next() {
            Some((key, bytes)) => {
//                let key = it.key().unwrap();
                let index = from_stream(&mut SerializedBuffer::from_slice(&key)).unwrap();
                let hash = from_stream(&mut SerializedBuffer::from_slice(&bytes)).unwrap();

//                let mut index = 0u32;
//                let mut hash = HASH_NULL;
//                index.read_params(&mut SerializedBuffer::from_slice(key.as_ref()));
//                hash.read_params(&mut SerializedBuffer::from_slice(bytes.as_ref()));

                Some(MilestoneObject {
                    index,
                    hash
                })
            }
            None => {
                debug!("get next milestone from storage error");
                None
            }
        }
    }

    pub fn storage_load_milestone(&self, index: u32) -> Option<MilestoneObject> {
        let vec = self.db.get_cf(self.db.cf_handle(CF_NAMES[CFType::Milestone as usize]).unwrap
        (), &to_buffer(&index).unwrap());
        match vec {
            Ok(res) => {
                let hash = from_stream(&mut SerializedBuffer::from_slice(&res?)).unwrap();
//                let mut hash = HASH_NULL;
//                hash.read_params(&mut SerializedBuffer::from_slice(&res?));

                Some(MilestoneObject {
                    index, hash
                })
            },
            Err(e) => {
                warn!("get milestone from storage error ({})", e);
                None
            }
        }
    }

    pub fn exists_state_diff(&self, hash: &Hash) -> bool {
        let vec = self.db.get_cf(self.db.cf_handle(CF_NAMES[CFType::StateDiff as usize]).unwrap(), hash);
        match vec {
            Ok(res) => res.is_some(),
            Err(_e) => return false
        }
    }

    pub fn put_milestone(&mut self, milestone: &MilestoneObject) -> bool {
        let key = to_buffer(&milestone.index).unwrap();
        self.storage_put(CFType::Milestone, &key, &milestone.hash)
    }

    pub fn put_transaction(&mut self, t: &Transaction) -> bool {
        let hash = t.get_hash();
        if hash == HASH_NULL || self.exists_transaction(hash.clone()) {
            return false;
        }

        let address = Address::from_public_key(&t.object.signature_pubkey);
        self.put_address_transaction(address, hash);
        self.put_address_transaction(t.object.address.clone(), hash);
        self.put_approvee(t.get_branch_transaction_hash(), hash);
        self.put_approvee(t.get_trunk_transaction_hash(), hash);
        self.storage_put(CFType::Transaction, &t.object.hash, &t.object)
    }

    pub fn storage_put<T>(&mut self, t: CFType, key: &[u8], packet: &T) -> bool where T : Serialize {
        let object = to_buffer(packet).unwrap();
        self.db.put_cf(self.db.cf_handle(CF_NAMES[t as usize]).unwrap(), key, &object).is_ok()
    }

    pub fn storage_merge<T>(&mut self, t: CFType, key: &[u8], packet: &T) -> bool where T : Serialize {
        let object = to_buffer(packet).unwrap();
        match self.db.merge_cf(self.db.cf_handle(CF_NAMES[t as usize]).unwrap(), key, &object) {
            Ok(_) => {
                debug!("merged {:?};{:?} into {}", key, hex::encode(&object), CF_NAMES[t as usize]);
                return true
            },
            Err(e) => error!("{:?}", e)
        };
        false
    }

    pub fn exists_transaction(&self, hash: Hash) -> bool {
        let vec = self.db.get_cf(self.db.cf_handle(CF_NAMES[CFType::Transaction as usize]).unwrap(), &hash);
        match vec {
            Ok(res) => res.is_some(),
            _ => false
        }
    }

    pub fn storage_load_transaction(&self, hash: &Hash) -> Option<Transaction> {
        let vec = self.db.get_cf(self.db.cf_handle(CF_NAMES[CFType::Transaction as usize]).unwrap(), hash);
        match vec {
            Ok(res) => {
                match res {
                    Some(ref res) => Some(Transaction::from_bytes(SerializedBuffer::from_slice(res))),
                    None => Some(Transaction::from_hash(hash.clone()))
                }
            },
            Err(e) => {
                warn!("get transaction from storage error ({})", e);
                Some(Transaction::from_hash(hash.clone()))
            }
        }
    }

    pub fn storage_load_state_diff(&self, hash: &Hash) -> Option<StateDiff> {
        let vec = self.db.get_cf(self.db.cf_handle(CF_NAMES[CFType::StateDiff as usize]).unwrap(), hash);
        match vec {
            Ok(res) => Some(StateDiff::from_bytes(SerializedBuffer::from_slice(&res?), hash.clone())),
            Err(e) => {
                warn!("get transaction from storage error ({})", e);
                None
            }
        }
    }

    pub fn load_address_transactions(&self, address: &Address) -> Option<Vec<Hash>> {
        let vec = self.db.get_cf(self.db.cf_handle(CF_NAMES[CFType::AddressTransactions as
            usize]).unwrap(), address);
        match vec {
            Ok(res) => {
                let buf = SerializedBuffer::from_slice(&res?);
                if buf.len() < HASH_SIZE || buf.len() % HASH_SIZE != 0 {
                    return None;
                }
                let mut arr = vec![];
                let mut pos = 0;
                while pos < buf.len() {
                    let mut hash = HASH_NULL;
                    hash.clone_from_slice(&buf[pos..(pos+HASH_SIZE)]);
                    arr.push(hash);
                    pos += HASH_SIZE;
                }
                Some(arr)
            },
            Err(e) => {
                warn!("get address transactions from storage error ({})", e);
                None
            }
        }
    }

    pub fn storage_load_approvee(&self, hash: &Hash) -> Option<Vec<Hash>> {
        let vec = self.db.get_cf(self.db.cf_handle(CF_NAMES[CFType::Approvee as usize]).unwrap(), hash);

        match vec {
            Ok(res) => {
                let buf = SerializedBuffer::from_slice(&res?);
                if buf.len() < HASH_SIZE || buf.len() % HASH_SIZE != 0 {
                    return Some(Vec::new());
                }
                let mut arr = vec![];
                let mut pos = 0;
                while pos < buf.len() {
                    let mut hash = HASH_NULL;
                    hash.clone_from_slice(&buf[pos..(pos+HASH_SIZE)]);
                    arr.push(hash);
                    pos += HASH_SIZE;
                }
                Some(arr)
            },
            Err(e) => {
                warn!("get transaction from storage error ({})", e);
                Some(Vec::new())
            }
        }
    }

    pub fn storage_get_address(&self, key: &Address) -> Option<u32> {
        let vec = self.db.get_cf(self.db.cf_handle(CF_NAMES[CFType::Address as usize]).unwrap(), &key);
        match vec {
            Ok(res) => {
                let num = from_stream(&mut SerializedBuffer::from_slice(&res?)).unwrap();
//                let mut num = 0;
//                num.read_params(&mut SerializedBuffer::from_slice(&res?));
                Some(num)
            },

            Err(e) => {
                warn!("get address from storage error ({})", e);
                None
            }
        }
    }

    pub fn apply_contract_state(&mut self, call: ContractInputOutput) {

    }

    fn init_db() -> DB {
        use crate::rocksdb::MergeOperands;
        fn concat_merge(_new_key: &[u8],
                        existing_val: Option<&[u8]>,
                        operands: &mut MergeOperands)
                        -> Vec<u8> {

            let mut result: Vec<u8> = Vec::with_capacity(operands.size_hint().0);
            existing_val.map(|v| {
                for e in v {
                    result.push(*e)
                }
            });
            for op in operands {
                for e in op {
                    result.push(*e)
                }
            }
            result
        }


        let mut opts = Options::new();
        opts.set_max_background_compactions(2);
        opts.set_max_background_flushes(2);
        opts.add_merge_operator("bytes_concat", concat_merge);

        let cfs_v = CF_NAMES.to_vec().iter().map(|_name| {
            let mut opts = Options::new();
            opts.set_max_write_buffer_number(2);
            opts.set_write_buffer_size(2 * 1024 * 1024);
            opts.add_merge_operator("bytes_concat", concat_merge);

//            ColumnFamilyDescriptor::new(*name, opts)
            opts
        }).collect::<Vec<_>>();

        use std::thread;
        let path = format!("db/data{}", env::var("API_PORT").unwrap());

        match DB::open_cf(&opts, &path, &CF_NAMES, cfs_v.as_slice()) {
            Ok(mut db) => {
                Hive::clear_db(&mut db);
                return db;
            },
            Err(_e) => {
                opts.create_if_missing(true);
                let mut db = DB::open(&opts, &path).expect("failed to create database");

                let mut opts = Options::new();
                for name in CF_NAMES.iter() {
                    db.create_cf(name, &opts);
                }

                opts.add_merge_operator("bytes_concat", concat_merge);

                let cfs_v = CF_NAMES.to_vec().iter().map(|_name| {
                    let mut opts = Options::new();
                    opts.set_max_write_buffer_number(2);
                    opts.set_write_buffer_size(2 * 1024 * 1024);
                    opts.add_merge_operator("bytes_concat", concat_merge);
//                    ColumnFamilyDescriptor::new(*name, opts)
                    opts
                }).collect::<Vec<_>>();

                drop(db);

                let db = DB::open_cf(&opts, &path, &CF_NAMES, cfs_v.as_ref()).expect("failed to open database");
                return db;
            }
        }
    }

    pub fn generate_address() -> (Address, PrivateKey, PublicKey) {
        use byteorder::{ByteOrder, LittleEndian};
        use secp256k1::*;
        use crate::rand::{thread_rng, Rng};
//        use self::ntrumls::{NTRUMLS, PQParamSetID};

//        let ntrumls = NTRUMLS::with_param_set(PQParamSetID::Security269Bit);
//        let (sk, pk) = ntrumls.generate_keypair().expect("failed to generate address");
        let secp = Secp256k1::<All>::new();
        let (sk, pk) = secp.generate_keypair(&mut thread_rng());

        let mut sha = Sha3::sha3_256();
        sha.input(&pk.serialize_uncompressed());

        let mut buf = [0u8; 32];
        sha.result(&mut buf);

        let _addr_left = hex::encode(&buf[..])[24..].to_string();//.to_uppercase();
        let offset = 32 - ADDRESS_SIZE + 1;
        let checksum_byte = Address::calculate_checksum(&buf[offset..]);

        let mut addr = ADDRESS_NULL;
        addr[..(ADDRESS_SIZE-1)].copy_from_slice(&buf[offset..32]);
        addr[ADDRESS_SIZE-1] = checksum_byte;
        (addr, sk, pk)
    }

    pub fn generate_address_from_private_key(sk: &PrivateKey) -> (Address, PublicKey) {
        use byteorder::{ByteOrder, LittleEndian};
//        use self::ntrumls::{NTRUMLS, PQParamSetID};

//        let ntrumls = NTRUMLS::with_param_set(PQParamSetID::Security269Bit);
//        let fg = ntrumls.unpack_fg_from_private_key(sk).expect("failed to unpack fg from private \
//        key");
//        let (_, pk) = ntrumls.generate_keypair_from_fg(&fg).expect("failed to generate address");
        use secp256k1::*;
        let secp = Secp256k1::<All>::new();
        let pk = PublicKey::from_secret_key(&secp, sk);
        let addr = Address::from_public_key(&pk);

        (addr, pk)
    }

    fn add_transaction(&mut self, _transaction: &TransactionObject) -> Result<(), Error> {
        unimplemented!();
        Ok(())
    }

    fn find_transaction(&mut self) {
        unimplemented!();
    }

    pub fn update_solid_transactions(&mut self, analyzed_hashes: &HashSet<Hash>) -> Result<(), TransactionError> {
        for hash in analyzed_hashes {
            let mut transaction = match self.storage_load_transaction(&hash) {
                Some(t) => t,
                None => return Err(TransactionError::InvalidHash)
            };

            self.update_heights(transaction.clone())?;

            if !transaction.is_solid() {
                transaction.update_solidity(true);
                self.update_transaction(&mut transaction)?;
            }
        }

        Ok(())
    }

    pub fn update_transaction(&mut self, t: &mut Transaction) -> Result<bool, TransactionError> {
        // TODO: check for existence?
        let hash = t.get_hash();
        if hash == HASH_NULL {
            return Ok(false);
        }

        let address = Address::from_public_key(&t.object.signature_pubkey);
        self.put_address_transaction(address, hash);
        self.put_approvee(t.get_branch_transaction_hash(), hash);
        self.put_approvee(t.get_trunk_transaction_hash(), hash);
        Ok(self.storage_put(CFType::Transaction, &t.object.hash, &t.object))
    }

    pub fn update_heights(&mut self, mut transaction: Transaction) -> Result<(), TransactionError> {
        let mut trunk = match self.storage_load_transaction(&transaction.get_trunk_transaction_hash()) {
            Some(t) => t,
            None => return Err(TransactionError::InvalidHash)
        };

        let mut transactions = vec![transaction.get_hash().clone()];

        while trunk.get_height() == 0 && trunk.get_type() != TransactionType::HashOnly && trunk
            .get_hash() != HASH_NULL {
            transaction = trunk.clone();
            trunk = match self.storage_load_transaction(&transaction.get_trunk_transaction_hash
            ()) {
                Some(t) => t,
                None => return Err(TransactionError::InvalidHash)
            };
            transactions.push(transaction.get_hash().clone());
        }

        while let Some(hash) = transactions.pop() {
            transaction = match self.storage_load_transaction(&hash) {
                Some(t) => t,
                None => return Err(TransactionError::InvalidHash)
            };
            let mut current_height = transaction.get_height();
            if trunk.get_hash() == HASH_NULL && trunk.get_height() == 0 && transaction.get_hash()
                != HASH_NULL {
                if current_height != 1 {
                    transaction.update_height(1);
                    self.update_transaction(&mut transaction)?;
                }
            } else if trunk.get_type() != TransactionType::HashOnly && transaction.get_height() == 0 {
                let new_height = 1 + trunk.get_height();
                if current_height != new_height {
                    transaction.update_height(new_height);
                    self.update_transaction(&mut transaction)?;
                }
            } else {
                break;
            }
            trunk = transaction.clone();
        }

        Ok(())
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IO(e)
    }
}

impl From<num::ParseIntError> for Error {
    fn from(e: num::ParseIntError) -> Self {
        Error::Parse(e)
    }
}
