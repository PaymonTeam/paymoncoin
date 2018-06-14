extern crate rand;
extern crate crypto;
extern crate rustc_serialize;
extern crate rocksdb;
extern crate log;
extern crate ntrumls;

use self::crypto::digest::Digest;
use self::crypto::sha3::Sha3;
use self::rocksdb::{DBIterator, DB, Options, IteratorMode, ColumnFamilyDescriptor, ColumnFamily};
use std::io;
use std::num;
use std::sync::Arc;
use std::collections::{HashSet, HashMap, LinkedList};
use ntrumls::*;

use model::{Milestone, MilestoneObject};
use model::transaction_validator::TransactionError;
use model::transaction::*;
use model::approvee::Approvee;
use model::{StateDiffObject, StateDiff};
use network::packet::{SerializedBuffer, Serializable, get_serialized_object};
use std::time;
use std::str::FromStr;

static CF_NAMES: [&str; 7] = ["transaction", "transaction-metadata", "address",
    "address_transactions", "approvee", "milestone", "state_diff"];
pub const SUPPLY : u32 = 10_000;

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

//    pub fn load_approvee(hash: &Hash) -> Approvee {
//        //TODO load
//        unimplemented!()
//    }

    pub fn init(&mut self) {
        use self::rustc_serialize::hex::FromHex;
        let mwm = 3;
        let coordinator = Address::from_str("P65DC4FEED4819C2910FA2DFC107399B7437ABAE2E7").unwrap();
        let mut th1 = HASH_NULL; // coordinator: 8000 -> Acc1
        let mut th2 = HASH_NULL; // coordinator: 2000 -> Acc2, trunk: th1, branch: th1
        let mut mh1 = HASH_NULL; // coordiator: milestone: 1, trunk: th1, branch: th2
        let mut mh2 = HASH_NULL; // null

        {
            let mls = ntrumls::NTRUMLS::with_param_set(PQParamSetID::Security269Bit);
            let mut genesis = TransactionObject {
                address: Address::from_str("PC19C342BA1A051A3BA7AF1DBBAA5E72469C94CC554").unwrap(),
                attachment_timestamp: time::SystemTime::now().elapsed().unwrap().as_secs() + 2,
                attachment_timestamp_lower_bound: 0u64,
                attachment_timestamp_upper_bound: 0u64,
                branch_transaction: HASH_NULL,
                trunk_transaction: HASH_NULL,
                hash: HASH_NULL,
                nonce: 0,
                tag: HASH_NULL,
                timestamp: time::SystemTime::now().elapsed().unwrap().as_secs(),
                value: 8000,
                data_type: TransactionType::Full,
                signature: Signature(vec![]),
                signature_pubkey: PublicKey(vec![]),
                snapshot: 1,
                solid: true,
                height: 1,
            };
            let mut genesis = Transaction::from_object(genesis);
            genesis.object.nonce = genesis.find_nonce(mwm);
            genesis.object.hash = genesis.calculate_hash();
            th1 = genesis.object.hash.clone();

            let sk =
                "8003FFFFF6A92AB90DD741AD2DBDB33D4AF90007BF75B80E61064FA6D59131875BC637A929E350ED11B004DECF4129732E1EB247571A6C5C54CC50692B6D31067E3814E7BEA3D974B23CEF8A974299CB07A5AA98B0679C83A60E445427F1054134033D4A2A51D6D8F706B3329A6BA28244ADBB8E2A5CA78A9580C7FC79AE4C962CB357579D1E854FECD60CA3765A326A1B93B528393C1830EABDDE7C72D9294B7EDC2378B1ADDA0F8F18894303910773BF2F48A1BF1F5B64D22F4E65838A7C00235592AB05CD68C48E19D8AC37EC46F14A750B614200B57398792616242321A381CAC49289B1A0AE6B6DB7207E4D83742381602A9C2E99AF52024F65082C30A8D51755B8BF3BE57354E878395E2652BC6B572794109F18D0D3CB4E869C043A7EB4DA6B50DCAFC6039D1CA8D87C611253D5E83EB1576D54D0043B3AD271D481DCBF9B7B29B84B07D594097163D4CF08C51E3E4EF026A9BAF08C6A51";
            let (sk, pk) = mls.generate_keypair_from_fg(&mls.unpack_fg_from_private_key
            (&PrivateKey(sk.from_hex().unwrap())).unwrap()).unwrap();
            genesis.object.signature = genesis.calculate_signature(&sk, &pk).expect("failed to \
            calculate signature");
            genesis.object.signature_pubkey = pk.clone();

            info!("genesis({:?})={}", th1, self.put_transaction(&genesis));
        }

        {
            let mls = ntrumls::NTRUMLS::with_param_set(PQParamSetID::Security269Bit);
            let mut genesis = TransactionObject {
                address: Address::from_str("PE138221B1A9CBEFCEAF03E17934A7373D6289F0536").unwrap(),
                attachment_timestamp: time::SystemTime::now().elapsed().unwrap().as_secs() + 3,
                attachment_timestamp_lower_bound: 0u64,
                attachment_timestamp_upper_bound: 0u64,
                branch_transaction: th1.clone(),
                trunk_transaction: th1.clone(),
                hash: HASH_NULL,
                nonce: 0,
                tag: HASH_NULL,
                timestamp: time::SystemTime::now().elapsed().unwrap().as_secs() + 1,
                value: 2000,
                data_type: TransactionType::Full,
                signature: Signature(vec![]),
                signature_pubkey: PublicKey(vec![]),
                snapshot: 1,
                solid: true,
                height: 1,
            };
            let mut genesis = Transaction::from_object(genesis);
            genesis.object.nonce = genesis.find_nonce(mwm);
            genesis.object.hash = genesis.calculate_hash();
            th2 = genesis.object.hash.clone();

            let sk =
                "8003FFFFF6A92AB90DD741AD2DBDB33D4AF90007BF75B80E61064FA6D59131875BC637A929E350ED11B004DECF4129732E1EB247571A6C5C54CC50692B6D31067E3814E7BEA3D974B23CEF8A974299CB07A5AA98B0679C83A60E445427F1054134033D4A2A51D6D8F706B3329A6BA28244ADBB8E2A5CA78A9580C7FC79AE4C962CB357579D1E854FECD60CA3765A326A1B93B528393C1830EABDDE7C72D9294B7EDC2378B1ADDA0F8F18894303910773BF2F48A1BF1F5B64D22F4E65838A7C00235592AB05CD68C48E19D8AC37EC46F14A750B614200B57398792616242321A381CAC49289B1A0AE6B6DB7207E4D83742381602A9C2E99AF52024F65082C30A8D51755B8BF3BE57354E878395E2652BC6B572794109F18D0D3CB4E869C043A7EB4DA6B50DCAFC6039D1CA8D87C611253D5E83EB1576D54D0043B3AD271D481DCBF9B7B29B84B07D594097163D4CF08C51E3E4EF026A9BAF08C6A51";
            let (sk, pk) = mls.generate_keypair_from_fg(&mls.unpack_fg_from_private_key
            (&PrivateKey(sk.from_hex().unwrap())).unwrap()).unwrap();
            genesis.object.signature = genesis.calculate_signature(&sk, &pk).expect("failed to \
            calculate signature");
            genesis.object.signature_pubkey = pk.clone();

            info!("genesis2({:?})={}", th2, self.put_transaction(&genesis));
        }
        // milestone
        {
            let mls = ntrumls::NTRUMLS::with_param_set(PQParamSetID::Security269Bit);
            let mut ms = TransactionObject {
                address: ADDRESS_NULL,
                attachment_timestamp: time::SystemTime::now().elapsed().unwrap().as_secs() + 4,
                attachment_timestamp_lower_bound: 0u64,
                attachment_timestamp_upper_bound: 0u64,
                branch_transaction: th1.clone(),
                trunk_transaction: th2.clone(),
                hash: HASH_NULL,
                nonce: 0,
                tag: HASH_NULL,
                timestamp: time::SystemTime::now().elapsed().unwrap().as_secs() + 2,
                value: 0,
                data_type: TransactionType::Full,
                signature: Signature(vec![]),
                signature_pubkey: PublicKey(vec![]),
                snapshot: 1,
                solid: true,
                height: 1,
            };
            let mut ms = Transaction::from_object(ms);
            ms.object.nonce = ms.find_nonce(mwm);
            ms.object.hash = ms.calculate_hash();
            mh1 = ms.object.hash.clone();

            let sk =
                "8003FFFFF6A92AB90DD741AD2DBDB33D4AF90007BF75B80E61064FA6D59131875BC637A929E350ED11B004DECF4129732E1EB247571A6C5C54CC50692B6D31067E3814E7BEA3D974B23CEF8A974299CB07A5AA98B0679C83A60E445427F1054134033D4A2A51D6D8F706B3329A6BA28244ADBB8E2A5CA78A9580C7FC79AE4C962CB357579D1E854FECD60CA3765A326A1B93B528393C1830EABDDE7C72D9294B7EDC2378B1ADDA0F8F18894303910773BF2F48A1BF1F5B64D22F4E65838A7C00235592AB05CD68C48E19D8AC37EC46F14A750B614200B57398792616242321A381CAC49289B1A0AE6B6DB7207E4D83742381602A9C2E99AF52024F65082C30A8D51755B8BF3BE57354E878395E2652BC6B572794109F18D0D3CB4E869C043A7EB4DA6B50DCAFC6039D1CA8D87C611253D5E83EB1576D54D0043B3AD271D481DCBF9B7B29B84B07D594097163D4CF08C51E3E4EF026A9BAF08C6A51";
            let (sk, pk) = mls.generate_keypair_from_fg(&mls.unpack_fg_from_private_key
            (&PrivateKey(sk.from_hex().unwrap())).unwrap()).unwrap();
            ms.object.signature = ms.calculate_signature(&sk, &pk).expect("failed to \
            calculate signature");
            ms.object.signature_pubkey = pk.clone();

            info!("milestone_transaction1={}", self.put_transaction(&ms));
            let milestone = MilestoneObject {
                index: 1,
                hash: mh1.clone()
            };
            info!("milestone1 ({:?}) ={}", mh1, self.put_milestone(&milestone));
        }

        let mut state = HashMap::new();
//        state.insert(Address::from_str("P65DC4FEED4819C2910FA2DFC107399B7437ABAE2E7").unwrap(), -8000);
        state.insert(Address::from_str("PC19C342BA1A051A3BA7AF1DBBAA5E72469C94CC554").unwrap(), 8000);
        state.insert(coordinator.clone(), -10000);
        state.insert(Address::from_str("PE138221B1A9CBEFCEAF03E17934A7373D6289F0536").unwrap(), 2000);

        let sdo = StateDiffObject {
            state
        };
        let sd = StateDiff {
            hash: mh1.clone(),
            state_diff_object: sdo
        };
        info!("state_diff1={}", self.put_state_diff(&sd));

        println!("put addr1={}", self.put_address_transaction(coordinator.clone(), th1.clone()));
        println!("put addr2={}", self.put_address_transaction(coordinator.clone(), th2.clone()));
        println!("put addr3={}", self.put_address_transaction(coordinator.clone(), mh1.clone()));
    }

    fn clear_db(db: &mut DB) {
        for name in CF_NAMES.iter() {
            let mut handle = db.cf_handle(name).unwrap();
            let mut it = db.iterator_cf(handle, IteratorMode::Start).unwrap();
            for (k,_) in it {
                db.delete_cf(handle, &k);
            }
        }
    }

    pub fn put_approvee(&mut self, approvee: Hash, approved: Hash) -> bool {
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
                let mut index = 0u32;
                let mut hash = HASH_NULL;
                index.read_params(&mut SerializedBuffer::from_slice(&key));
                hash.read_params(&mut SerializedBuffer::from_slice(&bytes));

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
                let mut index = 0u32;
                let mut hash = HASH_NULL;
                index.read_params(&mut SerializedBuffer::from_slice(&key));
                hash.read_params(&mut SerializedBuffer::from_slice(&bytes));

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
//        let mut it = self.db.iterator_cf(self.db.cf_handle(CF_NAMES[CFType::Milestone as usize])
//                                          .unwrap(), IteratorMode::Start).unwrap();
        let mut it = self.db.raw_iterator_cf(self.db.cf_handle(CF_NAMES[CFType::Milestone as usize]).unwrap()).unwrap();
        it.seek(&get_serialized_object(&index, false));
        it.next();

        match it.value() {
            Some(bytes) => {
                let key = it.key().unwrap();
                let mut index = 0u32;
                let mut hash = HASH_NULL;
                index.read_params(&mut SerializedBuffer::from_slice(&key));
                hash.read_params(&mut SerializedBuffer::from_slice(&bytes));

                Some(MilestoneObject {
                    index,
                    hash
                })
            }
            None => {
                warn!("get next milestone from storage error");
                None
            }
        }
    }

    pub fn storage_load_milestone(&self, index: u32) -> Option<MilestoneObject> {
        let vec = self.db.get_cf(self.db.cf_handle(CF_NAMES[CFType::Milestone as usize]).unwrap
        (), &get_serialized_object(&index, false));
        match vec {
            Ok(res) => {
                let mut hash = HASH_NULL;
                hash.read_params(&mut SerializedBuffer::from_slice(&res?));

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
            Err(e) => return false
        }
    }

    pub fn put_milestone(&mut self, milestone: &MilestoneObject) -> bool {
        let key = get_serialized_object(&milestone.index, false);
        self.storage_put(CFType::Milestone, &key, &milestone.hash)
    }

    pub fn put_transaction(&mut self, t: &Transaction) -> bool {
        self.storage_put(CFType::Transaction, &t.object.hash, &t.object)
    }

    pub fn storage_put<T>(&mut self, t: CFType, key: &[u8], packet: &T) -> bool where T : Serializable {
        let object = get_serialized_object(packet, false);
        self.db.put_cf(self.db.cf_handle(CF_NAMES[t as usize]).unwrap(), key, &object).is_ok()
    }

    pub fn storage_merge<T>(&mut self, t: CFType, key: &[u8], packet: &T) -> bool where T : Serializable {
        let object = get_serialized_object(packet, false);
        match self.db.merge_cf(self.db.cf_handle(CF_NAMES[t as usize]).unwrap(), key, &object) {
            Ok(_) => return true,
            Err(e) => println!("{:?}", e)
        };
        false
    }

    pub fn exists_transaction(&self, hash: Hash) -> bool {
        let vec = self.db.get_cf(self.db.cf_handle(CF_NAMES[CFType::Transaction as usize]).unwrap(), &hash);
        vec.is_ok()
    }

    pub fn storage_load_transaction(&self, key: &[u8]) -> Option<Transaction> {
        let vec = self.db.get_cf(self.db.cf_handle(CF_NAMES[CFType::Transaction as usize]).unwrap(), key);
        match vec {
            Ok(res) => Some(Transaction::from_bytes(SerializedBuffer::from_slice(&res?))),
            Err(e) => {
                warn!("get transaction from storage error ({})", e);
                None
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

    pub fn storage_load_approvee(&mut self, hash: &Hash) -> Option<Vec<Hash>> {
        let vec = self.db.get_cf(self.db.cf_handle(CF_NAMES[CFType::Approvee as usize]).unwrap
        (), hash);
        match vec {
            Ok(res) => {
                let buf = SerializedBuffer::from_slice(&res?);
                if buf.len() < HASH_SIZE || buf.len() % HASH_SIZE != 0 {
                    return Some(Vec::new());
//                    return None;
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
//                None
            }
        }
    }

    fn storage_get_address(&mut self, key: &[u8]) -> Option<u32> {
        let vec = self.db.get_cf(self.db.cf_handle(CF_NAMES[CFType::Address as usize]).unwrap(), key);
        match vec {
            Ok(res) => {
                let mut num = 0;
                num.read_params(&mut SerializedBuffer::from_slice(&res?));
                Some(num)
            },

            Err(e) => {
                warn!("get address from storage error ({})", e);
                None
            }
        }
    }

    fn init_db() -> DB {
        use self::rocksdb::merge_operator::MergeOperands;
        fn concat_merge(new_key: &[u8],
                        existing_val: Option<&[u8]>,
                        operands: &mut MergeOperands)
                        -> Option<Vec<u8>> {

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
            Some(result)
        }

        let mut opts = Options::default();
        opts.set_max_background_compactions(2);
        opts.set_max_background_flushes(2);
        opts.set_merge_operator("bytes_concat", concat_merge, None);

        let cfs_v = CF_NAMES.to_vec().iter().map(|name| {
            let mut opts = Options::default();
//                opts.set_merge_operator()
            opts.set_max_write_buffer_number(2);
            opts.set_write_buffer_size(2 * 1024 * 1024);
            opts.set_merge_operator("bytes_concat", concat_merge, None);

            ColumnFamilyDescriptor::new(*name, opts)
        }).collect();

        use std::thread;
        let path = format!("db/data{:?}", thread::current().id());

        match DB::open_cf_descriptors(&opts, path.clone(), cfs_v) {
            Ok(mut db) => {
                Hive::clear_db(&mut db);
                return db;
            },
            Err(e) => {
                opts.create_if_missing(true);
                let mut db = DB::open(&opts, path.clone()).expect("failed to create database");

                let mut opts = Options::default();
                for name in CF_NAMES.iter() {
                    db.create_cf(name, &opts);
                }

                opts.set_merge_operator("bytes_concat", concat_merge, None);

                let cfs_v = CF_NAMES.to_vec().iter().map(|name| {
                    let mut opts = Options::default();
//                opts.set_merge_operator()
                    opts.set_max_write_buffer_number(2);
                    opts.set_write_buffer_size(2 * 1024 * 1024);
                    opts.set_merge_operator("bytes_concat", concat_merge, None);

                    ColumnFamilyDescriptor::new(*name, opts)
                }).collect();

                drop(db);

                let db = DB::open_cf_descriptors(&opts, path, cfs_v).expect("failed to open database");
                return db;
            }
        }
    }

    pub fn generate_address() -> (Address, PrivateKey, PublicKey) {
        use byteorder::{ByteOrder, LittleEndian};
        use self::rustc_serialize::hex::ToHex;
        use self::ntrumls::{NTRUMLS, PQParamSetID};

        let ntrumls = NTRUMLS::with_param_set(PQParamSetID::Security269Bit);
        let (sk, pk) = ntrumls.generate_keypair().expect("failed to generate address");

//        let mut index_bytes = [0u8; 4];
//        LittleEndian::write_u32(&mut index_bytes, index);

        let mut sha = Sha3::sha3_256();
//        sha.input(fg);
//        sha.input(&index_bytes);
        sha.input(&pk.0);

        let mut buf = [0u8; 32];
        sha.result(&mut buf);

        let addr_left = buf[..].to_hex()[24..].to_string();//.to_uppercase();
        let offset = 32 - ADDRESS_SIZE + 1;
        let checksum_byte = Address::calculate_checksum(&buf[offset..]);

        let mut addr = ADDRESS_NULL;
        addr[..20].copy_from_slice(&buf[offset..32]);
        addr[20] = checksum_byte;
        (addr, sk, pk)
    }

    pub fn generate_address_from_private_key(sk: &PrivateKey) -> (Address, PublicKey) {
        use byteorder::{ByteOrder, LittleEndian};
        use self::rustc_serialize::hex::ToHex;
        use self::ntrumls::{NTRUMLS, PQParamSetID};

        let ntrumls = NTRUMLS::with_param_set(PQParamSetID::Security269Bit);
//        use super::super::std::mem;
//        let fg_16 : [u16; 128] = unsafe { mem::transmute(*fg) };
        let fg = ntrumls.unpack_fg_from_private_key(sk).expect("failed to unpack fg from private \
        key");
        let (_, pk) = ntrumls.generate_keypair_from_fg(&fg).expect("failed to generate address");

//        let mut index_bytes = [0u8; 4];
//        LittleEndian::write_u32(&mut index_bytes, index);

        let addr = Address::from_public_key(&pk);

        (addr, pk)
    }

    fn add_transaction(&mut self, transaction: &TransactionObject) -> Result<(), Error> {
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

    pub fn update_transaction(&mut self, transaction: &mut Transaction) -> Result<bool, TransactionError> {
        if transaction.get_hash() == HASH_NULL {
            return Ok(false);
        }

        Ok(self.put_transaction(transaction))
    }

    pub fn update_heights(&mut self, mut transaction: Transaction) -> Result<(),
        TransactionError> {
//        let mut transaction = &mut transaction.clone();

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
                    transaction.update_height(1);
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
