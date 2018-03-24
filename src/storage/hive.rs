extern crate rand;
extern crate crypto;
extern crate rustc_serialize;
extern crate patricia_trie;
extern crate rocksdb;
extern crate log;
extern crate hashdb;
extern crate memorydb;
extern crate ethcore_bigint as bigint;
extern crate time;
extern crate ntrumls;

use self::crypto::digest::Digest;
use self::crypto::sha3::Sha3;
use self::rocksdb::{DB, Options, IteratorMode, ColumnFamilyDescriptor, ColumnFamily};
use self::patricia_trie::{TrieFactory, TrieSpec, TrieMut};
use self::hashdb::HashDB;
use self::bigint::hash::H256;
use self::memorydb::MemoryDB;
use std::collections::HashMap;
use std::io;
use std::num;

use model::transaction::{HASH_SIZE, ADDRESS_SIZE, TransactionObject, Transaction, Address, ADDRESS_NULL, HASH_NULL};
use network::packet::{SerializedBuffer, Serializable, get_serialized_object};

static CF_NAMES: [&str; 3] = ["transaction", "transaction-metadata", "address"];
const SUPPLY : u32 = 10_000;

#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum AddressError {
    InvalidAddress,
}

pub enum Error {
    IO(io::Error),
    Parse(num::ParseIntError),
    Str(String),
}

#[derive(Copy, PartialEq, Eq, Clone, Debug, Hash)]
enum CFType {
    Transaction = 0,
    TransactionMetadata,
    Address
}

pub struct Hive<'a> {
    tree: Box<TrieMut + 'a>,
    db: DB,
    balances: HashMap<Address, u32>,
    column_families: HashMap<CFType, ColumnFamily>,
}

impl<'a> Hive<'a> {
    pub fn new(mdb: &'a mut MemoryDB, root: &'a mut H256) -> Self {
        let f = TrieFactory::new(TrieSpec::Generic);
        let tree = f.create(mdb, root);
        let db = Hive::init_db();

        let mut column_families = HashMap::<_, _>::new();

        column_families.insert(CFType::Transaction, db.cf_handle(CF_NAMES[0]).unwrap());
        column_families.insert(CFType::TransactionMetadata, db.cf_handle(CF_NAMES[1]).unwrap());
        column_families.insert(CFType::Address, db.cf_handle(CF_NAMES[2]).unwrap());

        Hive {
            tree,
            db,
            balances: HashMap::new(),
            column_families,
        }
    }

    pub fn init(&mut self) {
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

    fn storage_put<T>(&mut self, t: CFType, key: &[u8], packet: &T) where T : Serializable {
        let object = get_serialized_object(packet, false);
        self.db.put_cf((*self.column_families.get(&t).unwrap()).clone(), key, &object);
    }

    fn storage_get_transaction(&mut self, key: &[u8]) -> Option<Transaction> {
        let vec = self.db.get_cf((*self.column_families.get(&CFType::Transaction).unwrap()).clone(), key);
        match vec {
            Ok(res) => Some(Transaction::from_bytes(SerializedBuffer::from_slice(&res?))),
            Err(e) => {
                warn!("get transaction from storage error ({})", e);
                None
            }
        }
    }

    fn storage_get_address(&mut self, key: &[u8]) -> Option<u32> {
        let vec = self.db.get_cf((*self.column_families.get(&CFType::Address).unwrap()).clone(), key);
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
        let mut opts = Options::default();
        opts.set_max_background_compactions(2);
        opts.set_max_background_flushes(2);

        match DB::open_cf(&opts, "db/data", &CF_NAMES) {
            Ok(mut db) => {
                Hive::clear_db(&mut db);
                return db;
            },
            Err(e) => {
                opts.create_if_missing(true);
                let mut db = DB::open(&opts, "db/data").expect("failed to create database");

                let opts = Options::default();
                for name in CF_NAMES.iter() {
                    db.create_cf(name, &opts);
                }

                let cfs_v = CF_NAMES.to_vec().iter().map(|name| {
                    let mut opts = Options::default();
//                opts.set_merge_operator()
                    opts.set_max_write_buffer_number(2);
                    opts.set_write_buffer_size(2 * 1024 * 1024);

                    ColumnFamilyDescriptor::new(*name, opts)
                }).collect();

                let db = DB::open_cf_descriptors(&opts, "db/data", cfs_v).expect("failed to open database");
                return db;
            }
        }
    }

    pub fn generate_address(fg: &[u8; 256], index: u32) -> Address {
        use byteorder::{ByteOrder, LittleEndian};
        use self::rustc_serialize::hex::ToHex;
        use self::ntrumls::{NTRUMLS, PQParamSetID};

        let ntrumls = NTRUMLS::with_param_set(PQParamSetID::Security269Bit);
        use super::super::std::mem;
        let fg_16 : [u16; 128] = unsafe { mem::transmute(*fg) };
        let (sk, pk) = ntrumls.generate_keypair_from_fg(&fg_16).expect("failed to generate address");

        let mut index_bytes = [0u8; 4];
        LittleEndian::write_u32(&mut index_bytes, index);

        let mut sha = Sha3::sha3_256();
        sha.input(fg);
        sha.input(&index_bytes);
        sha.input(&pk.0);

        //let mut buf: Vec<u8> = repeat(0).take((sha.output_bits()+7)/8).collect();
        let mut buf = [0u8; 32];
        sha.result(&mut buf);

        let addr_left = buf[..].to_hex()[24..].to_string();//.to_uppercase();
        let offset = 32 - ADDRESS_SIZE + 1;
        let checksum_byte = Address::calculate_checksum(&buf[offset..]);

//        let addr = format!("P{}{:x}", addr_left, append_byte);
        let mut addr = ADDRESS_NULL;
        addr[..20].copy_from_slice(&buf[offset..32]);
        addr[20] = checksum_byte;
        addr
    }

    fn add_transaction(&mut self, transaction: &TransactionObject) -> Result<(), Error> {
        self.tree.insert(b"k", b"val");

        Ok(())
    }

    fn find_transaction(&mut self) {
//        let res = self.tree.get(b"k");
//        match res {
//            Ok(Some(val)) => {
//                println!("val={}", String::from_utf8_lossy(&val));
//                Ok(())
//            },
//            Ok(None) => {
//                println!("Nothing found");
//                Err("transaction not found")
//            },
//            Err(e) => Err(e)
//        }
    }

    pub fn address_to_string(bytes: [u8; ADDRESS_SIZE]) -> String {
        let strs: Vec<String> = bytes.iter()
            .map(|b| format!("{:02X}", b))
            .collect();
        format!("P{}", strs.join(""))
    }

    pub fn address_to_bytes(address: String) -> Result<[u8; ADDRESS_SIZE], AddressError> {
        use self::rustc_serialize::hex::FromHex;

        if !address.starts_with("P") {
            return Err(AddressError::InvalidAddress);
        }

        match address[1..].to_string().from_hex() {
            Err(_) => return Err(AddressError::InvalidAddress),
            Ok(vec) => {
                let bytes:&[u8] = vec.as_ref();
                let mut ret_bytes = [0u8; 21];

                if bytes.len() == 21 {
                    ret_bytes.copy_from_slice(&bytes);
                    return Ok(ret_bytes)
                } else {
                    return Err(AddressError::InvalidAddress);
                }
            }
        };
    }

    pub fn load_balances(&mut self) -> Result<(), Error> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        use self::rustc_serialize::hex::{ToHex, FromHex};

        let mut f = File::open("db/snapshot.dat")?;
        let file = BufReader::new(&f);

        let mut total = 0;

        for line in file.lines() {
            let l = line?;
            let arr : Vec<&str> = l.splitn(2, ' ').collect();
            let (addr_str, balance) = (String::from(arr[0]), String::from(arr[1]).parse::<u32>()?);
            let mut arr = [0u8; ADDRESS_SIZE];
            arr.copy_from_slice(&addr_str[1..].from_hex().expect("failed to load snapshot")[..ADDRESS_SIZE]);
            let addr = Address(arr);
//            if !addr.verify() {
//                panic!("invalid address in snapshot");
//            }
            if self.balances.insert(addr, balance).is_some() {
                panic!("invalid snapshot");
            }

            self.storage_put(CFType::Address, &addr, &balance);

            total += balance;
        }

        if total != SUPPLY {
            panic!("corrupted snapshot")
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

#[test]
fn hive_test() {
    use self::rustc_serialize::hex::{ToHex, FromHex};
    use rand::Rng;

    let mut root = H256::new();
    let mut mdb = MemoryDB::new();
    let mut hive = Hive::new(&mut mdb, &mut root);
    hive.load_balances();

    let mut t0 = TransactionObject::new_random();
    hive.storage_put(CFType::Transaction, &t0.hash, &t0);
    let t1 = hive.storage_get_transaction(&t0.hash).expect("failed to load transaction object from db");
    assert_eq!(t0, t1.transaction);

    let addr0 = ADDRESS_NULL;

    let random_sk = false;

    let mut data = "2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD2FB5A00B0214EDBDA0A0A004F8A3DBBCC76744523A8A77484468E87EC59ABDBD".from_hex().expect("invalid sk");
    let mut sk_data = [0u8; 32 * 8];
    if random_sk {
        rand::thread_rng().fill_bytes(&mut sk_data);
    } else {
        sk_data.copy_from_slice(&data[..(32 * 8)]);
    }

    let addr = Hive::generate_address(&sk_data, 0);
    hive.storage_put(CFType::Address, &addr, &10000u32);
    let balance = hive.storage_get_address(&addr).expect("storage get address error");

    println!("pk={}", sk_data.to_hex().to_uppercase());
    println!("address={:?} balance={}", addr, balance);
}