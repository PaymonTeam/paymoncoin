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

use self::rand::{Rng, thread_rng};
use self::secp256k1::key::{PublicKey, SecretKey};
use self::secp256k1::{Secp256k1, Signature, RecoverableSignature, Message, RecoveryId, ContextFlag};
use self::crypto::digest::Digest;
use std::fmt::{Display, Formatter};
use self::crypto::sha3::{Sha3, Sha3Mode};

use self::rocksdb::DB;
use self::rocksdb::Options;
use self::patricia_trie::{TrieFactory, TrieSpec, TrieMut, TrieDBMut};
use self::hashdb::{HashDB, DBValue};
use self::bigint::hash::H256;
use self::memorydb::MemoryDB;
use std::hash::Hash;
use std::thread::Thread;
use std::thread;
use std::io;
use std::sync::mpsc;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};
use std::time::Duration;

#[derive(Copy, PartialEq, Eq, Clone, Debug)]
pub enum Error {
    InvalidAddress,
}

pub struct Hive {
//    tree: Box<TrieDBMut<'a>>,
    db: DB,
//    mdb: MemoryDB,
//    root: H256,
}

impl Hive {
    pub fn new() -> Hive {
//        let f = TrieFactory::new(TrieSpec::Generic);
//        let mut mdb = MemoryDB::new();
//        let root = H256::new();
//        let mut t = Box::new(TrieDBMut::new(&mut mdb, &mut root));

        let mut opts = Options::default();
        opts.set_max_background_compactions(2);
        opts.set_max_background_flushes(2);
        opts.create_if_missing(true);
        let db = DB::open(&opts, "db/data").unwrap();

        Hive {
//            tree: t,
            db,
//            mdb,
//            root,
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

        let mut sha = Sha3::keccak256();
        sha.input(seed);
        sha.input(&pk.serialize_uncompressed()[1..]);

        let mut buf: Vec<u8> = repeat(0).take((sha.output_bits()+7)/8).collect();
        sha.result(&mut buf);

        let addrLeft = buf[..].to_hex()[24..].to_uppercase();
        let mut appendByte = 0u16;
        for b in buf[12..].iter() {
            appendByte += *b as u16;
            appendByte %= 256;
        }

        let addr = format!("P{}{:X}", addrLeft, appendByte);
        println!("{}", addr);
        let mut bytes = [0u8; 21];
        bytes[..20].copy_from_slice(&buf[12..32]);
        bytes[20] = appendByte as u8;
        (bytes, addr)
    }

    fn add_transaction(&mut self) {
//        self.tree.insert(b"k", b"val");
//        let res = self.tree.get(b"k");
//        match res {
//            Ok(Some(val)) => println!("val={}", String::from_utf8_lossy(&val)),
//            Ok(None) => println!("Nothing found"),
//            Err(e) => println!("{}", e)
//        }
    }

    fn save_to_disk(&self) {
        self.db.put(b"my key", b"my value");
        match self.db.get(b"my key") {
            Ok(Some(value)) => println!("retrieved value '{}'", value.to_utf8().unwrap()),
            Ok(None) => println!("value not found"),
            Err(e) => println!("operational problem encountered: {}", e),
        }
        self.db.delete(b"my key").unwrap();
    }

//  fn encode(mut num:u64) -> String {
//      let alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"; //"012345678abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"; //  Base61
//      let base_count = alphabet.len() as u64;
//
//      let mut encode = String::new();
//
//      while num >= base_count {
//          let m = (num % base_count) as usize;
//  //        encode = alphabet.chars().nth(m).unwrap() + encode;
//          encode.push(alphabet.chars().nth(m).unwrap());
//          num = (num / base_count) as u64;
//      }
//
//      if num != 0 {
//  //        encode = alphabet[num] + encode
//          encode.push(alphabet.chars().nth(num as usize).unwrap());
//
//      }
//
//      return encode
//  }
}