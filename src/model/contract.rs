use std::collections::{HashMap, hash_map::Keys};
use super::transaction::{Hash, Account, Address, AddressError};
use std::hash;
use std::str::FromStr;
use std::marker::PhantomData;
use crypto::sha3;
use crypto::digest::Digest;
use crate::model::transaction::HASH_NULL;
//use serde_json as json;

#[derive(Debug)]
pub enum Error {
    JsonParse(String),
    UnknownStorageKey,
    Unknown(String),
    UnknownContractType,
    UnknownContract,
}

#[derive(Clone)]
pub enum StorageAction<T> where T: Clone {
    Insert(T),
    Update(T),
    Remove(T),
}

impl<T> StorageAction<T> where T: Clone {
    pub fn map<B: Clone, F: FnOnce(T) -> B>(mut self, f: F) -> StorageAction<B> {
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

pub type StorageValue<T> = (Hash, T);

fn string_hash_function<T>(data: T) -> Hash where T: AsRef<[u8]> {
    Hash::sha3_160(data.as_ref())
}

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

pub trait Export<K, R> where K: AsRef<[u8]> {
    fn save(&self, key: K) -> R;
}

impl<K: AsRef<[u8]>> Export<K, Vec<StorageValue<String>>> for String {
    fn save(&self, key: K) -> Vec<StorageValue<String>> {
        let digest = Hash::sha3_160(key.as_ref());
        vec![(digest, (*self).clone())]
    }
}

impl<K, V: ToString> Export<K, Vec<StorageValue<String>>> for HashMap<K, V>
    where K: AsRef<[u8]> + Eq + hash::Hash
{
    fn save(&self, key: K) -> Vec<StorageValue<String>> {
        unimplemented!();
//        let mut vec = vec![];
//
//        for (k, v) in self.iter() {
//            for (d, sv) in v.to_string().save(k.as_ref()) {
//                let digest = Hash::sha3_160(d.as_ref());
//                vec.push((digest, sv))
//            }
//        }
//        vec
    }
}

impl Export<String, Vec<StorageValue<StorageAction<String>>>> for StorageAction<String> {
    fn save(&self, key: String) -> Vec<StorageValue<StorageAction<String>>> {
        let digest = Hash::sha3_160(key.as_ref());
        let v = match self {
            StorageAction::Insert(ref v) => (digest, StorageAction::Insert(v.clone())),
            StorageAction::Update(ref v) => (digest, StorageAction::Update(v.clone())),
            StorageAction::Remove(ref v) => (digest, StorageAction::Remove(v.clone())),
        };
        vec![v]
    }
}

impl<K, KK, V: ToString + Clone> Export<K, Vec<StorageValue<StorageAction<String>>>> for StorageAction<HashMap<KK, V>>
    where K: AsRef<[u8]>,
          KK: AsRef<[u8]> + Eq + hash::Hash + Clone
{
    fn save(&self, key: K) -> Vec<StorageValue<StorageAction<String>>> {
        let t = match self {
            StorageAction::Insert(_) => StorageAction::Insert(()),
            StorageAction::Update(_) => StorageAction::Update(()),
            StorageAction::Remove(_) => StorageAction::Remove(()),
        };
        let mut sha = sha3::Sha3::sha3_256();
        sha.input(key.as_ref());
        let map = self.clone().take();
        let mut vec = vec![];
        for (k, v) in map.iter() {
            let mut sha_cloned = sha.clone();
            sha_cloned.input(k.as_ref());
            let mut digest = HASH_NULL;
            sha_cloned.result(&mut digest);
            vec.push((digest, v.to_string()));
        }

        match t {
            StorageAction::Insert(_) => vec.into_iter().map(|(d, v)| (d, StorageAction::Insert(v))).collect(),
            StorageAction::Update(_) => vec.into_iter().map(|(d, v)| (d, StorageAction::Update(v))).collect(),
            StorageAction::Remove(_) => vec.into_iter().map(|(d, v)| (d, StorageAction::Remove(v))).collect(),
        }
    }
}
/*
impl<K, KK, KKK, V: ToString + Clone> Export<K, Vec<StorageValue<StorageAction<String>>>> for StorageAction<HashMap<KK, HashMap<KKK, V>>>
    where K: AsRef<[u8]>,
          KK: AsRef<[u8]> + Eq + hash::Hash + Clone,
          KKK: AsRef<[u8]> + Eq + hash::Hash + Clone,
{
    fn save(&self, key: K) -> Vec<StorageValue<StorageAction<String>>> {
        let t = match self {
            StorageAction::Insert(_) => StorageAction::Insert(()),
            StorageAction::Update(_) => StorageAction::Update(()),
            StorageAction::Remove(_) => StorageAction::Remove(()),
        };

        let mut sha = sha3::Sha3::sha3_256();
        sha.input(key.as_ref());

        let map = self.clone().take();
        let mut vec = vec![];
        for (k, m) in map.iter() {
            for (kk, v) in m.iter() {
                let mut sha_cloned = sha.clone();
                sha_cloned.input(k.as_ref());
                sha_cloned.input(kk.as_ref());
                let mut digest = HASH_NULL;
                sha_cloned.result(&mut digest);
                vec.push((digest, v.to_string()));
            }
        }

        match t {
            StorageAction::Insert(_) => vec.into_iter().map(|(d, v)| (d, StorageAction::Insert(v))).collect(),
            StorageAction::Update(_) => vec.into_iter().map(|(d, v)| (d, StorageAction::Update(v))).collect(),
            StorageAction::Remove(_) => vec.into_iter().map(|(d, v)| (d, StorageAction::Remove(v))).collect(),
        }
    }
}
*/
pub trait Storage<I> {
    type Hash: hash::Hash;
//    type Key: AsRef<[u8]>;
//    type Item;

    fn insert<K: AsRef<[u8]>, T: Export<K, Vec<StorageValue<I>>>>(&mut self, key: K, value: T);
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

    fn insert<K: AsRef<[u8]>, T: Export<K, Vec<StorageValue<String>>>>(&mut self, key: K, value: T) {
        for (digest, v) in value.save(key) {
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

    fn insert<K: AsRef<[u8]>, T: Export<K, Vec<StorageValue<StorageAction<String>>>>>(&mut self, key: K, value: T) {
        for (digest, v) in value.save(key) {
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

pub type StorageDiff = ContractStorage<StorageAction<String>>;

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
    pub fn call(&mut self, hash: &Hash, caller: &Account, input: &json::Map<String, json::Value>) -> Result<ContractOutput, Error> {
        debug!("request for contract {:?} call with input data {:?}", hash, input);

        if let Some(ref mut contract) = self.contracts.get_mut(hash) {
            if let Some(ref mut storage) = self.storages.get_mut(hash) {
                return contract.call(caller, input, storage);
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

pub trait Contract<T> where T: Export<String, Vec<StorageValue<String>>> {
    fn call(&mut self, caller: &Account, input: &json::Map<String, json::Value>, storage: &mut ContractStorage<T>) -> Result<ContractOutput, Error>;
//    fn create(caller: &Account, input: &json::Value) -> Result<(ContractOutput, TokenContract), Error>;
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
//        let address_string = format!("{:?}", owner.0);
        let mut map = HashMap::<Address, u64>::new();
        map.insert(owner.0.clone(), total_supply);
        storage.insert("balances".to_string(), Insert(map));

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
        } else if method == "name" {
            return self.name(storage);
        }

        Err(Error::JsonParse("unknown method".into()))
    }
}

mod tests {
    use super::*;
    use crate::init_log;

    #[test]
    fn storage_test() {
        init_log();

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
    }
}