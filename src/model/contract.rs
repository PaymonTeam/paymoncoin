use std::collections::{HashMap, hash_map::Keys};
use super::transaction::{Hash, Account, Address, AddressError};
use std::hash;
use std::str::FromStr;
use std::marker::PhantomData;
use crypto::sha3;
use crypto::digest::Digest;
use crate::model::transaction::HASH_NULL;
use std::num;
use std::fmt::Debug;

#[derive(Debug)]
pub enum Error {
    JsonParse(String),
    UnknownStorageKey,
    Unknown(String),
    UnknownContractType,
    UnknownContract,
    Overflow,
    KeyExists,
    KeyDoesntExist,
}

impl From<json::Error> for Error {
    fn from(e: json::Error) -> Self {
        Error::JsonParse("failed to parse json".into())
    }
}

impl From<AddressError> for Error {
    fn from(e: AddressError) -> Self {
        Error::Unknown("invalid address".into())
    }
}

impl From<num::ParseIntError> for Error {
    fn from(e: num::ParseIntError) -> Self {
        Error::Unknown("can't parse integer".into())
    }
}

pub type StorageValue<T> = (Hash, T);
pub type StorageDiff = ContractStorage<StorageAction<String>>;

struct KeyBuilder {
    state: sha3::Sha3,
}

impl KeyBuilder {
    pub fn new<T>(var_name: T) -> Self where T: AsRef<[u8]> {
        let mut state = sha3::Sha3::sha3_256();
        state.input(var_name.as_ref());

        Self {
            state,
        }
    }

    pub fn chain<T>(mut self, key: T) -> Self where T: AsRef<[u8]> {
        self.state.input(key.as_ref());
        self
    }

    pub fn finalize(mut self) -> Hash {
        let mut hash = HASH_NULL;
        self.state.result(&mut hash);
        hash
    }
}

impl FromStr for KeyBuilder {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        use regex::Regex;
        let mut builder: KeyBuilder;
        let re = Regex::new(r"^(\w+)").unwrap();

        let capt = re.captures(s);
        if let Some(capt) = capt {
            if let Some(capt) = capt.get(1) {
                builder = KeyBuilder::new(capt.as_str());

                let re = Regex::new(r"\[\b(\w+)]").unwrap();
                for cap in re.captures_iter(s) {
                    builder = builder.chain(&cap[1]);
                }
                return Ok(builder);
            }
        }
        Err(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageAction<T> where T: Clone + PartialEq + Eq + Debug {
    Insert(T),
    Update(T),
    Remove(T),
}

impl<T> StorageAction<T> where T: Clone + PartialEq + Eq + Debug {
    pub fn map<B: Clone + PartialEq + Eq + Debug, F: FnOnce(T) -> B>(mut self, f: F) -> StorageAction<B> {
        match self {
            StorageAction::Insert(v) => StorageAction::Insert(f(v)),
            StorageAction::Update(v) => StorageAction::Update(f(v)),
            StorageAction::Remove(v) => StorageAction::Remove(f(v)),
        }
    }

    pub fn take(self) -> T {
        match self {
            StorageAction::Insert(v) => v,
            StorageAction::Update(v) => v,
            StorageAction::Remove(v) => v,
        }
    }
}

pub trait Export<R> {
    fn save<S, K>(&self, mut state: S, key: &K) -> R where
        S: crypto::digest::Digest + Clone,
        K: AsRef<[u8]>;
}

impl Export<Vec<StorageValue<String>>> for String {
    fn save<S, K>(&self, mut state: S, key: &K) -> Vec<StorageValue<String>> where
        S: crypto::digest::Digest + Clone,
        K: AsRef<[u8]>
    {
        state.input(key.as_ref());
        let digest = digest_result(state);
        vec![(digest, (*self).clone())]
    }
}

impl<K, V> Export<Vec<StorageValue<String>>> for HashMap<K, V>
    where K: AsRef<[u8]> + Eq + hash::Hash,
          V: Export<Vec<StorageValue<String>>> + Eq,
{
    fn save<S, T>(&self, mut state: S, key: &T) -> Vec<StorageValue<String>> where
        S: crypto::digest::Digest + Clone,
        T: AsRef<[u8]> {
        let mut vec = vec![];
        for (k, v) in self {
            let mut sub_vec = v.save(state.clone(), k);
            vec.append(&mut sub_vec);
        }
        vec
    }
}

impl Export<Vec<StorageValue<StorageAction<String>>>> for StorageAction<String> {
    fn save<S, K>(&self, mut state: S, key: &K) -> Vec<StorageValue<StorageAction<String>>> where
        S: crypto::digest::Digest + Clone,
        K: AsRef<[u8]>
    {
        state.input(key.as_ref());
        let digest = digest_result(state);
        let v = match self {
            StorageAction::Insert(ref v) => (digest, StorageAction::Insert(v.clone())),
            StorageAction::Update(ref v) => (digest, StorageAction::Update(v.clone())),
            StorageAction::Remove(ref v) => (digest, StorageAction::Remove(v.clone())),
        };
        vec![v]
    }
}

impl<K, V> Export<Vec<StorageValue<StorageAction<String>>>> for HashMap<K, StorageAction<V>>
    where K: AsRef<[u8]> + Eq + hash::Hash + Clone + Debug,
          V: Export<Vec<StorageValue<String>>> + Clone + Eq + Debug,
{
    fn save<S, T>(&self, mut state: S, key: &T) -> Vec<StorageValue<StorageAction<String>>> where
        S: crypto::digest::Digest + Clone,
        T: AsRef<[u8]> {
        state.input(key.as_ref());

        let mut vec = vec![];
        for (k, v) in self.iter() {
            let mut sub_vec = v.clone().take().save(state.clone(), k)
                .into_iter().map(|(d, s)| match v {
                StorageAction::Insert(_) => (d, StorageAction::Insert(s)),
                StorageAction::Update(_) => (d, StorageAction::Update(s)),
                StorageAction::Remove(_) => (d, StorageAction::Remove(s)),
            }).collect();
            vec.append(&mut sub_vec);
        }

        vec
    }
}

pub trait Storage<I> {
    type Hash: hash::Hash;
//    type Key: AsRef<[u8]>;
//    type Item;

    fn insert<K: AsRef<[u8]>, T: Export<Vec<StorageValue<I>>>>(&mut self, key: K, value: T);
    fn get(&self, key: &Self::Hash) -> Option<&I>;
    fn keys(&self) -> Keys<Self::Hash, I>;
//    fn insert_storage<S: Storage>(&mut self, key: Self::Key, value: S);
}

pub struct ContractStorage<T> {//where T: Export<String, Vec<StorageValue>> {
    inner: HashMap<Hash, T>,
}

impl<T> ContractStorage<T> {//where T: Export<String, Vec<StorageValue>> {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }
}

impl Storage<String> for ContractStorage<String> {
    type Hash = Hash;
//    type Key = String;

    fn insert<K: AsRef<[u8]>, T: Export<Vec<StorageValue<String>>>>(&mut self, key: K, value: T) {
        let state = crypto::sha3::Sha3::sha3_256();
        for (digest, v) in value.save(state, &key) {
            self.inner.insert(digest, v);
        }
    }

    fn get(&self, key: &<Self as Storage<String>>::Hash) -> Option<&String> {
        self.inner.get(key)
    }

    fn keys(&self) -> Keys<<Self as Storage<String>>::Hash, String> {
        self.inner.keys()
    }
}

impl Storage<StorageAction<String>> for ContractStorage<StorageAction<String>> {
    type Hash = Hash;
//    type Key = String;

    fn insert<K: AsRef<[u8]>, T: Export<Vec<StorageValue<StorageAction<String>>>>>(&mut self, key: K, value: T) {
        let state = crypto::sha3::Sha3::sha3_256();
        for (digest, v) in value.save(state, &key) {
            self.inner.insert(digest, v);
        }
    }

    fn get(&self, key: &<Self as Storage<StorageAction<String>>>::Hash) -> Option<&StorageAction<String>> {
        self.inner.get(key)
    }

    fn keys(&self) -> Keys<<Self as Storage<StorageAction<String>>>::Hash, StorageAction<String>> {
        self.inner.keys()
    }
}

#[derive(Default)]
pub struct ContractsStorage {
    storages: HashMap<Hash, ContractStorage<String>>,
    contracts: HashMap<Hash, Box<dyn Contract<String>>>,
}

pub struct ContractOutput {
    pub output: json::Value,
    pub storage_diff: Option<StorageDiff>,
    pub balance_diff: isize,
}

impl ContractsStorage {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn call(&mut self, hash: &Hash, caller: &Account, input: &json::Map<String, json::Value>) -> Result<ContractOutput, Error> {
        use self::StorageAction::*;

        debug!("request for contract {:?} call with input data {:?}", hash, input);

        if let Some(ref mut contract) = self.contracts.get_mut(hash) {
            if let Some(ref mut storage) = self.storages.get_mut(hash) {
                let output = contract.call(caller, input, storage)?;

                if let Some(ref diff) = output.storage_diff {
                    for (k, act) in &diff.inner {
                        match act {
                            Insert(v) => {
                                if !storage.inner.contains_key(k) {
                                    storage.inner.insert(k.clone(), v.clone());
                                } else {
                                    error!("attempt to insert value, but key exists");
                                    return Err(Error::KeyExists);
                                }
                            },
                            Update(v) => {
                                if storage.inner.contains_key(k) {
                                    storage.inner.insert(k.clone(), v.clone());
                                } else {
                                    error!("attempt to update value, but key doesn't exist");
                                    return Err(Error::KeyDoesntExist);
                                }
                            },
                            Remove(v) => {
                                if storage.inner.contains_key(k) {
                                    storage.inner.remove(k);
                                } else {
                                    error!("attempt to remove value, but key doesn't exist");
                                    return Err(Error::KeyDoesntExist);
                                }
                            },
                        }
                    }
                }

                let diff = output.balance_diff;
                let balance = caller.1 as usize;
                if diff != 0 {
                    let out_balance = if diff > 0 {
                        balance.checked_add(diff as usize).ok_or(Error::Overflow)?;
                    } else {
                        balance.checked_sub((-diff) as usize).ok_or(Error::Overflow)?;
                    };
                }

                return Ok(output);
            }
        }
        Err(Error::UnknownContract)
    }

    pub fn create(&mut self, creator: &Account, input: &json::Value) -> Result<(), Error> {
        let obj = input.as_object().ok_or(Error::JsonParse("expected object".into()))?;
        let contract_type = obj.get("type").ok_or(Error::JsonParse("expected field 'type'".into()))?
            .as_str().ok_or(Error::JsonParse("expected string".into()))?;
        let mut new_contract: Box<dyn Contract<String>>;
        let mut output: ContractOutput;

        if contract_type == "token" {
            let params = obj.get("params").ok_or(Error::JsonParse("expected field 'params'".into()))?
                .as_object().ok_or(Error::JsonParse("expected object".into()))?;

            let (output_c, new_contract_c) = TokenContract::create(creator, params)?;
            new_contract = new_contract_c;
            output = output_c;
        } else {
            return Err(Error::UnknownContractType);
        }

        let hash = Hash::sha3_160(input.to_string().as_bytes());
        if !self.contracts.contains_key(&hash) {
            self.contracts.insert(hash.clone(), new_contract);

            let mut storage = ContractStorage::<String>::new();
            if let Some(storage_diff) = output.storage_diff {
                for (k, v) in &storage_diff.inner {
                    if let StorageAction::Insert(ref v) = v {
                        storage.inner.insert(k.clone(), v.clone());
                    }
                }
            }
            self.storages.insert(hash, storage);
        }

        info!("created contract {:?} with input {}", hash, input.to_string());

        Ok(())
    }
}

pub trait Contract<T> where T: Export<Vec<StorageValue<String>>> {
    fn call(&mut self, caller: &Account, input: &json::Map<String, json::Value>, storage: &mut ContractStorage<T>) -> Result<ContractOutput, Error>;
//    fn create(caller: &Account, input: &json::Value) -> Result<(ContractOutput, TokenContract), Error>;
}

#[derive(Serialize, Deserialize)]
pub struct TokenContract {
    name: String,
    symbol: String,
    decimals: u64,
    total_supply: u64,
    balances: HashMap<Address, u64>,
    allowed: HashMap<Address, HashMap<Address, u64>>,
}

impl TokenContract {
    pub fn new(name: &str, symbol: &str, decimals: u64) -> Self {
        Self {
            name: name.into(),
            symbol: symbol.into(),
            decimals,
            total_supply: 0,
            balances: HashMap::new(),
            allowed: HashMap::new(),
        }
    }

    pub fn balance_of(&self, address: &Address, storage: &mut ContractStorage<String>) -> Result<ContractOutput, Error> {
        let balance = storage.get(&KeyBuilder::new("balances").chain(address).finalize()).ok_or(Error::UnknownStorageKey)?;
        let output = json!( {
            "balance": balance
        } );

        Ok(ContractOutput {
            storage_diff: None,
            balance_diff: 0,
            output,
        })
    }

    pub fn transfer(&self, caller: &Account, to: &Address, amount: u64, storage: &mut ContractStorage<String>) -> Result<ContractOutput, Error> {
        use self::StorageAction::*;
        let from = caller.0;

        let sender_balance_key = KeyBuilder::new("balances").chain(from).finalize();
        let sender_balance = u64::from_str(storage.get(&sender_balance_key).ok_or(Error::UnknownStorageKey)?)?;
        let out_sender_balance: u64 = sender_balance.checked_sub(amount).ok_or(Error::Overflow)?;
        let recipient_balance_key = KeyBuilder::new("balances").chain(to).finalize();

        // do we need to check this?
        let (is_new, recipient_balance) = match storage.get(&recipient_balance_key) {
            Some(ref s) => (false, u64::from_str(s)?),
            None => (true, 0)
        };
        let out_recipient_balance: u64 = recipient_balance.checked_add(amount).ok_or(Error::Overflow)?;
        let mut storage_diff = StorageDiff::new();
        let mut map = HashMap::<Address, StorageAction<String>>::new();

        let val = if is_new {
            Insert(out_recipient_balance.to_string())
        } else {
            Update(out_recipient_balance.to_string())
        };
        map.insert(to.clone(), val);
        map.insert(from, Update(out_sender_balance.to_string()));
        storage_diff.insert("balances", map);

        let output = json!( {
            "success": true
        } );

        Ok(ContractOutput {
            storage_diff: Some(storage_diff),
            balance_diff: 0,
            output,
        })
    }

    pub fn name(&self, storage: &mut ContractStorage<String>) -> Result<ContractOutput, Error> {
        let name = storage.get(&KeyBuilder::from_str("name").unwrap().finalize()).ok_or(Error::UnknownStorageKey)?;
        let output = json!( {
            "name": name
        } );

        Ok(ContractOutput {
            storage_diff: None,
            balance_diff: 0,
            output,
        })
    }

    fn create(owner: &Account, params: &json::Map<String, json::Value>) -> Result<(ContractOutput, Box<dyn Contract<String>>), Error> {
        use self::StorageAction::*;

        let name = params.get("name").ok_or(Error::JsonParse("expected field 'name'".into()))?
            .as_str().ok_or(Error::JsonParse("expected string".into()))?;
        let symbol = params.get("symbol").ok_or(Error::JsonParse("expected field 'symbol'".into()))?
            .as_str().ok_or(Error::JsonParse("expected string".into()))?;
        let decimals = params.get("decimals").ok_or(Error::JsonParse("expected field 'decimals'".into()))?
            .as_u64().ok_or(Error::JsonParse("expected u32".into()))?;
        // rust 1.32 doesn't support 'pow' function of two u64's,
        // so we need to cast it to u32
        if decimals > u32::max_value() as u64 {
            return Err(Error::JsonParse("expected u32".into()));
        }
        let total_supply: u64 = 10u64.pow(decimals as u32);

        let mut storage = ContractStorage::<StorageAction<String>>::new();
        storage.insert("name".to_string(), Insert(name.to_string()));
        storage.insert("symbol".to_string(), Insert(symbol.to_string()));
        storage.insert("decimals".to_string(), Insert(decimals.to_string()));
        storage.insert("total_supply".to_string(), Insert(total_supply.to_string()));
        let mut map = HashMap::<Address, StorageAction<String>>::new();
        map.insert(owner.0.clone(), Insert(total_supply.to_string()));
        storage.insert("balances".to_string(), map);

        let out = ContractOutput {
            output: json!({}),
            balance_diff: 0,
            storage_diff: Some(storage),
        };

        let contract = TokenContract::new(name, symbol, decimals);
        Ok((out, Box::new(contract)))
    }
}

impl Contract<String> for TokenContract {
    fn call(&mut self, caller: &Account, input: &json::Map<String, json::Value>, storage: &mut ContractStorage<String>) -> Result<ContractOutput, Error> {
//        use json::Value;
//        let json_val: Value = json::from_str(input)?;
//        let input = json_val.as_object().ok_or(Error::JsonParse("expected object".into()))?;

        let method = input
            .get("method").ok_or(Error::JsonParse("expected field 'method'".into()))?
            .as_str().ok_or(Error::JsonParse("expected string".into()))?;
        let args = input
            .get("arguments").ok_or(Error::JsonParse("expected field 'arguments'".into()))?
            .as_object().ok_or(Error::JsonParse("expected object".into()))?;

        if method == "balance_of" {
            let address = Address::from_str(args.get("address")
                .ok_or(Error::JsonParse("expected field 'address'".into()))?
                .as_str().ok_or(Error::JsonParse("expected string".into()))?)?;

            return self.balance_of(&address, storage);
        } else if method == "transfer" {
            let to = Address::from_str(args.get("to")
                .ok_or(Error::JsonParse("expected field 'to'".into()))?
                .as_str().ok_or(Error::JsonParse("expected string".into()))?)?;

            let amount = args.get("amount")
                .ok_or(Error::JsonParse("expected field 'amount'".into()))?
                .as_u64().ok_or(Error::JsonParse("expected u64".into()))?;

            return self.transfer(caller, &to, amount, storage);
        } else if method == "name" {
            return self.name(storage);
        }

        Err(Error::JsonParse("unknown method".into()))
    }
}

fn string_hash_function<T>(data: T) -> Hash where T: AsRef<[u8]> {
    Hash::sha3_160(data.as_ref())
}

#[inline]
fn digest_result<D: crypto::digest::Digest>(mut digest: D) -> Hash {
    let mut hash = HASH_NULL;
    digest.result(&mut hash);
    hash
}

mod tests {
    use super::*;
    use crate::init_log;

    #[test]
    fn storage_test() {
        init_log();

        use super::StorageAction::*;

        let acc1 = Account(Address::from_str("P111111111111111111111111111111111111111111").unwrap(), 1000);
        let acc2 = Account(Address::from_str("P222222222222222222222222222222222222222222").unwrap(), 2000);

        let mut contracts_storage = ContractsStorage::default();
        let create_contract_json: json::Value = json!({
            "type": "token",
            "params": {
                "name": "Test Token",
                "symbol": "TEST",
                "decimals": 6
            }
        });

        let contract_hash = string_hash_function(create_contract_json.to_string().as_bytes());
        contracts_storage.create(&acc1, &create_contract_json).expect("failed to create contract");

        let call_contract_json = json!({
            "method": "name",
            "arguments": {}
        });
        let call_contract_json_obj: &json::Map<String, json::Value> = call_contract_json.as_object().unwrap();

        let out = contracts_storage.call(&contract_hash, &acc2, call_contract_json_obj).expect("failed to call contract");
        assert!(out.storage_diff.is_none());
        assert_eq!(out.balance_diff, 0);
        assert_eq!(out.output, json!({ "name": "Test Token" }));

        let call_contract_json = json!({
            "method": "balance_of",
            "arguments": {
                "address": "P111111111111111111111111111111111111111111"
            }
        });

        let call_contract_json_obj: &json::Map<String, json::Value> = call_contract_json.as_object().unwrap();
        let out = contracts_storage.call(&contract_hash, &acc2, call_contract_json_obj).expect("failed to call contract");
        assert!(out.storage_diff.is_none());
        assert_eq!(out.balance_diff, 0);
        assert_eq!(out.output, json!({ "balance": "1000000" }));

        let call_contract_json = json!({
            "method": "transfer",
            "arguments": {
                "to": "P222222222222222222222222222222222222222222",
                "amount": 500_000
            }
        });

        let call_contract_json_obj: &json::Map<String, json::Value> = call_contract_json.as_object().unwrap();
        let mut out = contracts_storage.call(&contract_hash, &acc1, call_contract_json_obj).expect("failed to call contract");
        assert!(out.storage_diff.is_some());

        let sender_balance_key = KeyBuilder::new("balances").chain(acc1.0).finalize();
        let recipient_balance_key = KeyBuilder::new("balances").chain(acc2.0).finalize();
        let diff = out.storage_diff.take().unwrap();
        assert_eq!(diff.get(&sender_balance_key), Some(&Update(500_000.to_string())));
        assert_eq!(diff.get(&recipient_balance_key), Some(&Insert(500_000.to_string())));
        assert_eq!(out.balance_diff, 0);
        assert_eq!(out.output, json!({ "success": true }));

        let call_contract_json = json!({
            "method": "transfer",
            "arguments": {
                "to": "P222222222222222222222222222222222222222222",
                "amount": 500_001
            }
        });

        let call_contract_json_obj: &json::Map<String, json::Value> = call_contract_json.as_object().unwrap();
        let mut out = contracts_storage.call(&contract_hash, &acc1, call_contract_json_obj);
        assert!(out.is_err());
    }
}