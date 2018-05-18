extern crate rustc_serialize;

use std::sync::Mutex;
use std::collections::HashMap;
use model::transaction::*;
use storage::hive;

pub const SNAPSHOT_PUBKEY: &str = "ABC123";
pub const SNAPSHOT_INDEX: u32 = 1;
pub static INITIAL_SNAPSHOT: Option<Mutex<Snapshot>> = None;

// TODO: make singleton
#[derive(Clone)]
pub struct Snapshot {
    pub state: HashMap<Address, u32>,
    pub index: u32,
}

impl Snapshot {
    pub fn init(path: String, snapshot_sig_path: String) -> Option<Snapshot> {
        if INITIAL_SNAPSHOT.is_none() {

            // TODO: check file signature

            match Snapshot::init_initial_state(path.clone()) {
                Ok(state) => {
                    let snapshot = Snapshot {
                        state,
                        index: 0
                    };
                    return Some(snapshot);
                },
                Err(_) => {
                    error!("failed to init initial snapshot state");
                    return None;
                }
            }
        }
        None
    }

    fn init_initial_state(snapshot_file_path: String) -> Result<HashMap<Address, u32>, hive::Error> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        use self::rustc_serialize::hex::{ToHex, FromHex};

        let mut balances = HashMap::<Address, u32>::new();

        let mut f = File::open("db/snapshot.dat")?;
        let file = BufReader::new(&f);

        let mut total = 0u32;

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

            if balances.insert(addr, balance).is_some() {
                panic!("invalid snapshot");
            }

//            self.storage_put(CFType::Address, &addr, &balance);
            let (v, b) = total.overflowing_add(balance);
            if b {
                panic!("incorrect total balance");
            }
            total = v;
        }

        if total != hive::SUPPLY {
            panic!("corrupted snapshot")
        }

        Ok(balances)
    }

    pub fn get_balance(&self, addr: &Address) -> Option<u32> {
        match self.state.get(addr) {
            Some(i) => Some(*i),
            None => None
        }
    }

    pub fn patched_diff(&mut self, mut diff: HashMap<Address, i32>) -> HashMap<Address, i32> {
        diff.iter_mut().map(|(address, balance)| {
            (address, balance)
        });
        diff
    }
}