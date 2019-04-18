use std::sync::Mutex;
use std::collections::HashMap;
use crate::transaction::transaction::*;
use crate::storage::hive;
use crate::transaction::transaction_validator::TransactionError;

pub const SNAPSHOT_PUBKEY: &str = "ABC123";
pub const SNAPSHOT_INDEX: u32 = 1;
pub static INITIAL_SNAPSHOT: Option<Mutex<Snapshot>> = None;

// TODO: make singleton
#[derive(Clone)]
pub struct Snapshot {
    pub state: HashMap<Address, i64>,
    pub index: u32,
}

impl Snapshot {
    pub fn init(path: String, _snapshot_sig_path: String) -> Option<Snapshot> {
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

    fn init_initial_state(_snapshot_file_path: String) -> Result<HashMap<Address, i64>,
        hive::Error> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
//        use self::rustc_serialize::hex::{ToHex, FromHex};
        use hex::{encode, decode};

        let mut balances = HashMap::<Address, i64>::new();

        let f = File::open("db/snapshot.dat")?;
        let file = BufReader::new(&f);

        let mut total = 0i64;

        for line in file.lines() {
            let l = line?;
            let arr : Vec<&str> = l.splitn(2, ' ').collect();
            let (addr_str, balance) = (String::from(arr[0]), String::from(arr[1]).parse::<i64>()?);
            let mut arr = [0u8; ADDRESS_SIZE];
            arr.copy_from_slice(&decode(&addr_str[1..]).expect("failed to load snapshot")[..ADDRESS_SIZE]);
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

        if total != hive::SUPPLY as i64 {
            panic!("corrupted snapshot")
        }

        Ok(balances)
    }

    pub fn get_balance(&self, addr: &Address) -> Option<i64> {
        match self.state.get(addr) {
            Some(i) => Some(*i),
            None => None
        }
    }

    pub fn patched_diff(&self, diff: HashMap<Address, i64>) -> HashMap<Address, i64> {
        diff.into_iter().map(|(address, balance)| {
            let new_balance = match self.state.get(&address) {
                Some(n) => *n,
                None => 0
            } + balance;

            (address, new_balance)
        }).collect()
    }

    pub fn apply(&mut self, patch: &HashMap<Address, i64>, new_index: u32) -> Result<(), TransactionError> {
        if patch.values().sum::<i64>() != 0 {
            error!("Diff isn't consistent");
            return Err(TransactionError::InvalidData);
        }

        patch.iter().for_each(|(address, balance)| {
//            let new_balance = match self.state.get(address) {
//                Some(n) => *n as i32,
//                None => 0
//            } + *balance;

            let new_balance = match self.state.get(address) {
                Some(n) => {
                    *n + balance.clone()
                },
                None => {
                    balance.clone()
                }
            };

            self.state.insert(address.clone(), new_balance);
        });

        self.index = new_index;

        Ok(())
    }

    pub fn is_consistent(state: &mut HashMap<Address, i64>) -> bool {
        let mut consistent = true;
        let mut to_remove = Vec::<Address>::new();

        for (k, v) in state.iter() {
            if *v <= 0 {
                if *v < 0 {
                    info!("Skipping negative value for address: {:?}: {}", k, v);
                    consistent = false;
                    break;
                }
                to_remove.push(k.clone());
            }
        }

        for addr in &to_remove {
            state.remove(addr);
        }

        consistent
    }
}