extern crate rand;
extern crate crypto;
extern crate secp256k1;
extern crate rustc_serialize;
extern crate patricia_trie;
extern crate rocksdb;
extern crate log;
extern crate hashdb;
extern crate memorydb;
extern crate ethcore_bigint as bigint;
extern crate time;

use self::secp256k1::key::{PublicKey, SecretKey};
use self::secp256k1::{Secp256k1, ContextFlag};
use self::crypto::digest::Digest;
use self::crypto::sha3::Sha3;
use self::rocksdb::{DB, Options, IteratorMode};
use self::patricia_trie::{TrieFactory, TrieSpec, TrieMut, TrieDBMut};
use self::hashdb::{HashDB, DBValue};
use self::bigint::hash::H256;
use self::memorydb::MemoryDB;
use std::hash::Hash;
use std::cell::{RefMut, RefCell};
use std::collections::HashMap;

use model::config::Configuration;
use model::transaction::{ADDRESS_SIZE, TransactionObject};

#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum Error {
    InvalidAddress,
}

pub struct Hive<'a> {
    tree: Box<TrieMut + 'a>,
    db: DB,

}

static cf_names : [&str; 3] = ["transaction", "transaction-metadata", "address"];

impl<'a> Hive<'a> {

    pub fn new(mdb: &'a mut MemoryDB, root: &'a mut H256) -> Self {
        let f = TrieFactory::new(TrieSpec::Generic);
        let tree = f.create(mdb, root);
        let db = Hive::init_db();

        Hive {
            tree,
            db,
        }
    }

    pub fn init(&mut self) {
//        self.init_db();
    }

    fn clear_db(db: &mut DB) {
        for name in cf_names.iter() {
            let mut handle = db.cf_handle(name).unwrap();
            let mut it = db.iterator_cf(handle, IteratorMode::Start).unwrap();
            for (k,_) in it {
                db.delete_cf(handle, &k);
            }
        }
    }

    fn init_db() -> DB {
        let mut opts = Options::default();
        opts.set_max_background_compactions(2);
        opts.set_max_background_flushes(2);

        match DB::open_cf(&opts, "db/data", &cf_names) {
            Ok(mut db) => {
                Hive::clear_db(&mut db);
                return db;
            },
            Err(e) => {
                opts.create_if_missing(true);
                let mut db = DB::open(&opts, "db/data").expect("failed to create database");

                let opts = Options::default();
                for name in cf_names.iter() {
                    db.create_cf(name, &opts);
                }

                let db = DB::open_cf(&opts, "db/data", &cf_names).expect("failed to open database");
                return db;
            }
        }
    }

    pub fn generate_address(seed: &[u8; 32]) -> ([u8; 21], String) {
//        let mut sk_data = [0u8; 32];
//        rand::thread_rng().fill_bytes(&mut sk_data);

        use std::iter::repeat;
        use self::rustc_serialize::hex::ToHex;

        let secp = Secp256k1::with_caps(ContextFlag::Full);

        let sk = SecretKey::from_slice(&secp, seed).unwrap();
        let pk = PublicKey::from_secret_key(&secp, &sk).unwrap();

        let mut sha = Sha3::sha3_256();
        sha.input(seed);
        sha.input(&pk.serialize_uncompressed()[1..]);

        //let mut buf: Vec<u8> = repeat(0).take((sha.output_bits()+7)/8).collect();
        let mut buf = [0u8; 32];
        sha.result(&mut buf);

        let addr_left = buf[..].to_hex()[24..].to_uppercase();
        let mut append_byte = 0u16;
        let offset = 32 - ADDRESS_SIZE;
        for (i, b) in buf[offset..].iter().enumerate() {
            if i & 1 == 0 {
                append_byte += *b as u16;
            } else {
                append_byte += (*b as u16) * 2;
            }
            append_byte %= 256;
        }

        let addr = format!("P{}{:X}", addr_left, append_byte);
        let mut bytes = [0u8; ADDRESS_SIZE];
        bytes[..20].copy_from_slice(&buf[offset..32]);
        bytes[20] = append_byte as u8;
        (bytes, addr)
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

    pub fn address_to_bytes(address: String) -> Result<[u8; ADDRESS_SIZE], Error> {
        use self::rustc_serialize::hex::FromHex;

        if !address.starts_with("P") {
            return Err(Error::InvalidAddress);
        }

        match address[1..].to_string().from_hex() {
            Err(_) => return Err(Error::InvalidAddress),
            Ok(vec) => {
                let bytes:&[u8] = vec.as_ref();
                let mut ret_bytes = [0u8; 21];

                if bytes.len() == 21 {
                    ret_bytes.copy_from_slice(&bytes);
                    return Ok(ret_bytes)
                } else {
                    return Err(Error::InvalidAddress);
                }
            }
        };
//        let bytes = address[1..].to_string().from_hex().unwrap_or(|| return Err(Error::InvalidAddress));

    }
}