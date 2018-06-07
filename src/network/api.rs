use rustc_serialize::json;
use rustc_serialize::json::Json;

use iron;
use iron::{Iron, Request, Response, IronResult, AfterMiddleware, Chain, Listening};
use iron::prelude::*;
use iron::status;
use network::Node;
use network::paymoncoin::PaymonCoin;
use utils::{AM, AWM};
use std;
use std::io::Read;
use network::rpc;
use network::packet::Serializable;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use model::transaction::*;
use model::*;
use std::collections::{HashMap, HashSet, LinkedList};
use model::transaction_validator::TransactionError;
use std::collections::linked_list::Iter;
#[macro_export]
macro_rules! format_success_response {
    ($a:ident) => {
        Ok(Response::with((iron::status::Ok, json::encode(&$a).unwrap())))
    };
}

struct DefaultContentType;

impl AfterMiddleware for DefaultContentType {
    fn after(&self, _req: &mut Request, mut resp: Response) -> IronResult<Response> {
        resp.headers.set(iron::headers::ContentType::json());
        Ok(resp)
    }
}

static mut PMNC: Option<AM<PaymonCoin>> = None;
const MILESTONE_START_INDEX: u32 = 0;
const MIN_RANDOM_WALKS: u32 = 5;
const MAX_RANDOM_WALKS: u32 = 27;
const MAX_DEPTH: u32 = 15;

pub struct API {
    listener: Listening,
    running: Arc<(Mutex<bool>, Condvar)>,
    paymoncoin: AM<PaymonCoin>,

}

#[derive(RustcDecodable, RustcEncodable)]
pub struct APIRequest<T: Serializable> {
    pub method: String,
    pub object: T,
}

pub enum APIError{
    InvalidStringParametr,
    TipAbsent,
    TheSubHiveIsNotSolid,
    InvalidData
}
impl API {
    pub fn new(pmnc: AM<PaymonCoin>, port: u16, running: Arc<(Mutex<bool>, Condvar)>) -> Self {
        info!("Running API on port {}", port);
        let mut chain = Chain::new(API::api);
        chain.link_after(DefaultContentType);
        let listener = Iron::new(chain)
            .http(format!("127.0.0.1:{}", port))
            .expect("failed to start API server");
        let pmnc_clone = pmnc.clone();
        unsafe {
            PMNC = Some(pmnc);
        }

        Self {
            listener,
            running,
            paymoncoin: pmnc_clone
        }
    }

    pub fn run(&mut self) {
        let &(ref lock, ref cvar) = &*self.running;
        let mut is_running = lock.lock().unwrap();
        while *is_running {
            is_running = cvar.wait(is_running).unwrap();
        }
        self.listener.close().unwrap();
    }

    fn format_error_response(err: &str) -> Response {
        Response::with((iron::status::Ok, format!("{{\"error\":\"{}\"}}\n", err.to_string())))
    }

    fn get_transactions_to_approve(pmnc: &mut PaymonCoin, mut depth: u32, reference: Option<Hash>, mut num_walks: u32) ->
                                                                                              Result<Option<(Hash, Hash)>, TransactionError> {
        if num_walks > MAX_RANDOM_WALKS || num_walks == 0 {
            num_walks = MAX_RANDOM_WALKS;
        }

        let max_depth = match pmnc.tips_manager.lock() {
            Ok(tm) => tm.get_max_depth(),
            Err(_) => panic!("broken tips manager mutex")
        };
        if depth > max_depth {
            depth = max_depth;
        }

        let mut visited_hashes = HashSet::new();
        let mut diff = HashMap::new();
        let mut h0: Option<Hash>;
        let mut h1: Option<Hash>;

        if let Some(hash) = reference {
            let hive = match pmnc.hive.lock() {
                Ok(h) => h,
                _ => panic!("broken hive mutex")
            };
            if !hive.exists_transaction(hash) {
                return Err(TransactionError::InvalidHash);
            } else {
                let milestone_latest_solid_subhive_milestone_index = match pmnc.milestone.lock() {
                    Ok(l) => l.latest_solid_subhive_milestone_index,
                    _ => panic!("broken milestone mutex")
                };

                let tx = hive.storage_load_transaction(&hash).unwrap();

                if tx.object.snapshot != 0 && tx.object.snapshot < milestone_latest_solid_subhive_milestone_index - depth {
                    return Err(TransactionError::InvalidTimestamp);
                }
            }
        }

        if let Ok(tips_manager) = pmnc.tips_manager.lock() {
            h0 = tips_manager.transaction_to_approve(&mut visited_hashes, &mut diff, reference,
                                                     None, depth, num_walks)?;
        } else {
            panic!("broken tips manager mutex");
        }

        if let Ok(ref mut ledger_validator) = pmnc.ledger_validator.lock() {
            if h0.is_none() || !ledger_validator.update_diff(&mut visited_hashes, &mut diff, h0.unwrap())? {
                return Ok(None);
            }
        } else {
            panic!("broken tips manager mutex");
        }

        if let Ok(tips_manager) = pmnc.tips_manager.lock() {
            h1 = tips_manager.transaction_to_approve(&mut visited_hashes, &mut diff, reference,
                                                     h0, depth, num_walks)?;
        } else {
            panic!("broken tips manager mutex");
        }

        if let Ok(ref mut ledger_validator) = pmnc.ledger_validator.lock() {
            if h1.is_none() || !ledger_validator.update_diff(&mut visited_hashes, &mut diff, h1.unwrap())? {
                return Ok(None);
            }

            if h0.unwrap() == HASH_NULL || h1.unwrap() == HASH_NULL {
                error!("tips are HASH_NULL");
                return Ok(None);
            }

            if ledger_validator.check_consistency(&vec![h0.unwrap(), h1.unwrap()])? {
                return Ok(Some((h0.unwrap(), h1.unwrap())));
            } else {
                error!("inconsistent tips pair selected");
                return Err(TransactionError::InvalidData);
            }
        } else {
            panic!("broken tips manager mutex");
        }
    }

    fn invalid_subtangle_status(pmnc: &mut PaymonCoin) -> bool {
        if let Ok(milestone) = pmnc.milestone.lock() {
            return milestone.latest_solid_subhive_milestone_index == MILESTONE_START_INDEX;
        } else {
            return false;
        }
    }

    fn api(req: &mut Request) -> IronResult<Response> {
        if req.method != iron::method::Post {
            return Ok(API::format_error_response("Wrong HTTP method"));
        }

        match req.headers.get::<iron::headers::ContentType>() {
            Some(ct) => if ct.0 != iron::headers::ContentType::json().0 {
                return Ok(API::format_error_response("Wrong content-type"));
            },
            None => return Ok(API::format_error_response("Wrong content-type")),
        };

        let version = match req.headers.get_raw("X-PMNC-API-Version") {
            Some(version) => format!("Version: {}\n", std::str::from_utf8(&version[0]).unwrap()),
            None => return Ok(API::format_error_response("Not API request")),
        };

        let mut body = Vec::new();
        req.body.read_to_end(&mut body).map_err(|e| IronError::new(e,
                                                                   (status::InternalServerError,
                                                                    "Error reading request")))?;
        let json_str = std::str::from_utf8(&body).map_err(|e| IronError::new(e,
                                                                             (status::InternalServerError, "Invalid UTF-8 string")))?;
        let mut json = Json::from_str(json_str).map_err(|e| IronError::new(e,
                                                                           (status::InternalServerError, "Invalid JSON")))?;

        match json.as_object() {
            Some(o) => {
                if !o.contains_key("method") {
                    return Ok(API::format_error_response("No 'method' parameter"));
                }

                match o.get("method").unwrap().as_string() {
                    Some(method) => {
                        match method {
                            "broadcastTransaction" => {
                                println!("broadcastTransaction");
                                match json::decode::<rpc::BroadcastTransaction>(&json_str) {
                                    Ok(bt) => {
                                        unsafe {
                                            if let Some(ref arc) = PMNC {
                                                if let Ok(pmnc) = arc.lock() {
                                                    if let Ok(mut node) = pmnc.node.lock() {
                                                        node.on_api_broadcast_transaction_received(bt);
                                                    }
                                                }
                                            }
                                        }
                                        return Ok(Response::with((iron::status::Ok, "{}")));
                                    }
                                    Err(e) => return Ok(API::format_error_response("Invalid data"))
                                };
                            }
                            "getTransactionsToApprove" => {
                                println!("getTransactionsToApprove");
                                match json::decode::<rpc::GetTransactionsToApprove>(&json_str) {
                                    Ok(object) => {
                                        unsafe {
                                            if let Some(ref arc) = PMNC {
                                                if let Ok(ref mut pmnc) = arc.lock() {
                                                    if API::invalid_subtangle_status(pmnc) {
                                                        return Ok(API::format_error_response("The subhive has not been updated yet"));
                                                    }
                                                    use rand::{thread_rng, Rng};
//                                                    let depth = 3;
//                                                    let num_walks = thread_rng().gen_range(2, 5);
                                                    let depth = object.depth;
                                                    let mut num_walks = match object.num_walks {
                                                        0 => 1,
                                                        v => v
                                                    };
                                                    if num_walks < MIN_RANDOM_WALKS {
                                                        num_walks = MIN_RANDOM_WALKS;
                                                    }

//                                                    let reference = match object.reference {
//                                                        v => Some(v),
//                                                        HASH_NULL => None,
//                                                    };

                                                    let reference;
                                                    if object.reference == HASH_NULL {
                                                        reference = None;
                                                    } else {
                                                        reference = Some(object.reference);
                                                    }

                                                    if depth < 0 || (reference.is_none() && depth == 0) {
                                                        return Ok(API::format_error_response("Invalid depth input"));
                                                    }

                                                    info!("num_walks={}", num_walks);

                                                    match API::get_transactions_to_approve(pmnc,
                                                                                           depth,
                                                                                           reference,
                                                                                           num_walks) {
                                                        Ok(Some((trunk, branch))) => {
                                                            let result = rpc::TransactionsToApprove {
                                                                branch, trunk
                                                            };
                                                            return format_success_response!(result);
                                                        },
                                                        Ok(None) => return Ok(API::format_error_response("None")),
                                                        _ => return Ok(API::format_error_response("Internal error"))
                                                    }
                                                }
                                            }
                                        };
                                        return Ok(API::format_error_response("Internal error"));
                                    }
                                    Err(e) => return Ok(API::format_error_response("Invalid data"))
                                };
                            }
                            "getNodeInfo" => {
                                println!("getNodeInfo");
                                let result = rpc::NodeInfo {
                                    name: "PMNC 0.1".to_string()
                                };
                                format_success_response!(result)
                            }
                            "getBalances" => {
                                println!("getBalances");
                                let result = rpc::Balances {
                                    balances: vec![1000u32, 2000u32, ]
                                };
                                format_success_response!(result)
                            }
                            _ => Ok(API::format_error_response("Unknown 'method' parameter"))
                        }
                    }
                    None => Ok(API::format_error_response("Invalid 'method' parameter"))
                }
            }
            None => Ok(API::format_error_response("Invalid request"))
        }
    }

    pub fn shutdown(&mut self) {}

    pub fn get_tips(&self) -> Result<LinkedList<Hash>, &'static str> {
        if let Ok(paymoncoin) = self.paymoncoin.lock() {
            if let Ok(tips_vm) = paymoncoin.tips_vm.lock() {
                return Ok(tips_vm.get_tips().iter().map(|x| *x).collect::<LinkedList<Hash>>());
            } else {
                panic!("broken tips_view_model mutex")
            }
        } else {
            panic!("broken paymoncoin mutex");
        }
    }
    pub fn get_balances(&self, addrss: LinkedList<Address>, tips: LinkedList<Hash>, threshold: i32) ->
    Result<(LinkedList<i32>, LinkedList<Hash>, u32), TransactionError> {
        if threshold <= 0 || threshold > 100 {
            return Err(TransactionError::InvalidData);
        }

        let addresses: LinkedList<Address> = addrss.iter().map(|address| *address).collect::<LinkedList<Address>>();
        let mut hashes: LinkedList<Hash> = LinkedList::new();
        let mut balances: HashMap<Address, i32> = HashMap::new();
        let mut index: u32 = 0;
        if let Ok(pmnc) = self.paymoncoin.lock() {
            if let Ok(mls) = pmnc.milestone.lock() {
                index = mls.latest_snapshot.index;
                if tips.len() == 0 {
                    hashes.push_back(mls.latest_solid_subhive_milestone);
                } else {
                    hashes = tips.iter().map(|address| *address).collect::<LinkedList<Hash>>();
                }
            }
        }
        for address in addresses.iter() {
            let mut value: i32 = 0;
            if let Ok(pmnc) = self.paymoncoin.lock() {
                if let Ok(mls) = pmnc.milestone.lock() {
                    value = match mls.latest_snapshot.get_balance(address) {
                        Some(v) => v,
                        None => panic!("Invalid balance")
                    }
                }
            }
            balances.insert(*address, value);
        }

        let mut visited_hashes: HashSet<Hash> = HashSet::new();
        let mut diff: HashMap<Address, i64> = HashMap::new();

        for tip in hashes.iter() {
            if let Ok(pmc) = self.paymoncoin.lock() {
                if let Ok(hive) = pmc.hive.lock() {
                    if !hive.exists_transaction(*tip) {
                        return Err(TransactionError::InvalidAddress);
                    }
                } else {
                    panic!("broken hive mutex");
                }
            } else {
                panic!("broken paymoncoin mutex");
            }
            if let Ok(pmc) = self.paymoncoin.lock() {
                if let Ok(mut lv) = pmc.ledger_validator.lock() {
                    let update_diff_is_ok = lv.update_diff(&mut visited_hashes, &mut diff, *tip)?;
                    if !update_diff_is_ok {
                        return Err(TransactionError::InvalidAddress);
                    }
                } else {
                    panic!("broken hive mutex");
                }
            } else {
                panic!("broken paymoncoin mutex");
            }
        }
        diff.iter().for_each(|(key, value)| {
            let new_value: i32;
            let is_get: bool;
            match balances.get(key) {
                Some(v) => {
                    new_value = *v + (*value as i32);
                    is_get = true;
                }
                None => {
                    new_value = 0;
                    is_get = false;
                }
            }
            if is_get {
                balances.remove(key);
                balances.insert(*key, new_value);
            }
        });

        let elements: LinkedList<i32> = addresses.iter().map(|address| *balances.get(address).unwrap())
            .collect::<LinkedList<i32>>();

        return Ok((elements, hashes, index));
    }

    pub fn find_transaction_statement (json_obj: Option<json::Object>) -> Result<Vec<Hash>, APIError> {
        let mut found_transactions: HashSet<Hash> = HashSet::new();
        let mut contains_key = false;
        let mut request: HashMap<String,i64> = HashMap::new();
        let mut bundles_transactions: HashSet<Hash> = HashSet::new();
        let request_clone = request.clone();
        if request_clone.contains_key("bundles") {
            let bundles: HashSet<Hash> = API::get_parameter_as_set(& request, "bundles".to_string(), HASH_SIZE)?;
            for bundle in bundles.iter() {
                // TODO bundle
                //bundles_transactions.addAll(BundleViewModel.load(instance.tangle, new Hash(bundle)).getHashes());
            }
            for hash in bundles_transactions.iter() {
                found_transactions.insert(*hash);
            }
            contains_key = true;
        }
        let mut addresses_transactions: HashSet<Hash> = HashSet::new();
        if request_clone.contains_key("addresses") {
            let mut addresses: HashSet<Address> = API::get_parameter_as_set_addresses(& request, "addresses".to_string(), HASH_SIZE)?;
            for address in addresses.iter() {
                //TODO
                //addresses_transactions.addAll(AddressViewModel.load(instance.tangle, new Hash(address)).getHashes());
            }

            for hash in addresses_transactions.iter() {
                found_transactions.insert(*hash);
            }
            contains_key = true;
        }

        let mut tags_transactions: HashSet<Hash> = HashSet::new();
        if request_clone.contains_key("tags") {
            let mut tags: HashSet<Hash> = API::get_parameter_as_set(& request,"tags".to_string(),0)?;
            for tag in tags.iter() {
                //TODO
                //tag = padTag(tag);
                //tagsTransactions.addAll(TagViewModel.load(instance.tangle, new Hash(tag)).getHashes());
            }
            if tags_transactions.is_empty() {
                for tag in tags.iter() {
                    //tag = padTag(tag);
                    //tagsTransactions.addAll(TagViewModel.loadObsolete(instance.tangle, new Hash(tag)).getHashes());
                }
            }
            for hash in tags_transactions.iter() {
                found_transactions.insert(*hash);
            }
            contains_key = true;
        }

        let mut approvee_transactions: HashSet<Hash> = HashSet::new();
        if request_clone.contains_key("approvees") {
            let approvees: HashSet<Hash> = API::get_parameter_as_set(& request, "approvees".to_string(), HASH_SIZE)?;
            for approvee in approvees.iter() {
                //   approveeTransactions.addAll(TransactionViewModel.fromHash(instance.tangle, new Hash(approvee)).getApprovers(instance.tangle).getHashes());
            }
            for hash in approvee_transactions.iter() {
                found_transactions.insert(*hash);
            }
            contains_key = true;
        }

        if !contains_key {
            return Err(APIError::InvalidData);
        }

        //Using multiple of these input fields returns the intersection of the values.
        if request.contains_key("bundles") {
            found_transactions.intersection(&bundles_transactions);
        }
        if request.contains_key("addresses") {
            found_transactions.intersection(&addresses_transactions);
        }
        if request.contains_key("tags") {
            found_transactions.intersection(&tags_transactions);
        }
        if request.contains_key("approvees") {
            found_transactions.intersection(&approvee_transactions);
        }
        //TODO
        /*if found_transactions.size() > maxFindTxs {
            return ErrorResponse.create(overMaxErrorMessage);
        }*/

        let elements: Vec<Hash> = found_transactions.iter()
            .map(|tx| *tx)
            .collect::<Vec<Hash>>();

        return Ok(elements);
    }
    fn get_parameter_as_set(request: &HashMap<String, i64>, param_name: String, size: usize)->  Result<HashSet<Hash>, APIError> {
        let list = API::get_parameter_as_list(request, param_name, size)?;
        let mut hashset: Vec<Hash> = list.iter().map(|hash| *hash).collect::<Vec<Hash>>();
        let mut result: HashSet<Hash> = HashSet::new();
        for hash in hashset.iter(){
            result.insert(*hash);
        }
        if result.contains(&HASH_NULL) {
            return Err(APIError::InvalidStringParametr);
        }
        return Ok(result);
    }
    fn get_parameter_as_set_addresses(request: & HashMap<String, i64>, param_name: String, size: usize)->  Result<HashSet<Address>, APIError> {
        let list = API::get_parameter_as_list_addresses(& request, param_name, size)?;
        let mut hashset: Vec<Address> = list.iter().map(|adr| *adr).collect::<Vec<Address>>();
        let mut result: HashSet<Address> = HashSet::new();
        for adr in hashset.iter(){
            result.insert(*adr);
        }
        if result.contains(&ADDRESS_NULL) {
            return Err(APIError::InvalidStringParametr);
        }
        return Ok(result);
    }
    fn get_parameter_as_list(request: & HashMap<String, i64>, param_name: String, size: usize) -> Result<LinkedList<Hash>, APIError>{

        unimplemented!();
    }
    fn get_parameter_as_list_addresses(request: & HashMap<String, i64>, param_name: String, size: usize) -> Result<LinkedList<Address>, APIError>{

        unimplemented!();
    }
    pub fn get_new_inclusion_state_statement(trans: &LinkedList<Hash>, tps: &LinkedList<Hash> ) -> Result<Vec<bool>, APIError> {
        let transactions = trans.clone();
        let tips = tps.clone();
        let number_of_non_met_transactions = transactions.len();
        let mut inclusion_states: Vec<i32> = Vec::with_capacity(number_of_non_met_transactions);

        let mut tips_index: LinkedList<u32> = LinkedList::new();
        {
            for tip in tips.iter() {
                let tx = Transaction::from_hash(*tip);
                if tx.get_type() != TransactionType::HashOnly {
                    tips_index.push_back(tx.object.get_snapshot_index());
                }
            }
        }

        let min_tips_index = tips_index.iter().fold(0u32, |a, b| {
            if a < *b {
                return a;
            } else {
                return *b;
            }
            return 0u32;
        });
        if min_tips_index > 0 {
            let max_tips_index = tips_index.iter().fold(0u32, |a, b| {
                if a > *b {
                    return a;
                } else {
                    return *b;
                }
                return 0u32;
            });
            let mut count = 0;
            for hash in transactions.iter() {
                let transaction = Transaction::from_hash(*hash);
                if transaction.get_type() == TransactionType::HashOnly || transaction.object.get_snapshot_index() == 0 {
                    inclusion_states[count] = -1;
                } else if transaction.object.get_snapshot_index() > max_tips_index {
                    inclusion_states[count] = -1;
                } else if transaction.object.get_snapshot_index() < max_tips_index {
                    inclusion_states[count] = 1;
                }
                count += 1;
            }
        }
        let mut analyzed_tips: HashSet<Hash> = HashSet::new();
        let mut same_index_transaction_count: HashMap<u32, u32> = HashMap::new();
        let mut same_index_tips: HashMap<u32, LinkedList<Hash>> = HashMap::new();
        for tip in tips.iter() {
            let transaction = Transaction::from_hash(*tip);
            if transaction.get_type() == TransactionType::HashOnly {
                return Err(APIError::TipAbsent);
            }
            let snapshot_index = transaction.object.get_snapshot_index();
            if !same_index_tips.contains_key(&snapshot_index) {
                let mut list = LinkedList::new();
                same_index_tips.insert(snapshot_index, list);
            }
            same_index_tips.get_mut(&snapshot_index).unwrap().push_back(*tip);
        }

        for i in 0..inclusion_states.len() {
            // LinkedList::get() impl
            if inclusion_states[i] == 0 {
                let mut it: Hash = HASH_NULL;
                if transactions.len() > i {
                    let mut count: usize = 0;
                    for itr in transactions.iter() {
                        if count == i {
                            it = *itr;
                        } else {
                            count += 1;
                        }
                    }
                } else {
                    panic!("Incorrect index");
                }
                let transaction = Transaction::from_hash(it);
                let snapshot_index = transaction.object.get_snapshot_index();
                if !same_index_transaction_count.contains_key(&snapshot_index) {
                    same_index_transaction_count.insert(snapshot_index, 0);
                }
                let same_index_transaction_count_clone = same_index_transaction_count.clone();
                same_index_transaction_count.insert(snapshot_index, same_index_transaction_count_clone.get(&snapshot_index).unwrap() + 1);
            }
        }

        for index in same_index_transaction_count.keys() {
            let mut same_index_tip: LinkedList<Hash> = match same_index_tips.get(index){
                Some(list) => (*list).clone(),
                None => panic!("None returned")
            };
            if !same_index_tip.is_empty() {
                //has tips in the same index level
                let flag = API::exhaustive_search_within_index(& mut same_index_tip,
                                                          & mut analyzed_tips,
                                                          &transactions,
                                                          & mut inclusion_states,
                                                          *same_index_transaction_count.get(index).unwrap(),
                                                          *index)?;
                if !flag {
                    return Err(APIError::TheSubHiveIsNotSolid);
                }
            }
        }

        let mut inclusion_states_boolean: Vec<bool> = Vec::with_capacity(inclusion_states.len());
        for i in 0..inclusion_states.len() {
            inclusion_states_boolean[i] = inclusion_states[i] == 1;
        }
        {
            return Ok(inclusion_states_boolean);
        }
    }
    fn exhaustive_search_within_index(non_analyzed_transactions: & mut LinkedList<Hash>,
                                      analyzed_tips: & mut HashSet<Hash>,
                                      transactions: &LinkedList<Hash>,
                                      inclusion_states: & mut Vec<i32>,
                                      count: u32,
                                      index: u32) -> Result<bool, APIError> {
        let mut pointer: Option<Hash>;
        let mut count_clone = count.clone();
        pointer = non_analyzed_transactions.pop_front();
        'MAIN_LOOP:
        while pointer != None {
            let hash = pointer.unwrap();
            if analyzed_tips.insert(hash) {
                let transaction = Transaction::from_hash(hash);
                if transaction.object.get_snapshot_index() == index {
                    if transaction.get_type() == TransactionType::HashOnly {
                        return Ok(false);
                    } else {
                        for i in 0..inclusion_states.len() {
                            // LinkedList::get() impl
                            let mut it: Hash = HASH_NULL;
                            if transactions.len() > i {
                                let mut count: usize = 0;
                                for itr in transactions.iter() {
                                    if count == i {
                                        it = *itr;
                                    } else {
                                        count += 1;
                                    }
                                }
                            } else {
                                panic!("Incorrect index");
                            }
                            if inclusion_states[i] < 1 && hash == it {
                                inclusion_states[i] = 1;
                                count_clone -= 1;
                                if count_clone <= 0 {
                                    break 'MAIN_LOOP;
                                }
                            }
                        }
                        non_analyzed_transactions.push_back(transaction.get_trunk_transaction_hash());
                        non_analyzed_transactions.push_back(transaction.get_branch_transaction_hash());
                    }
                }
            }
            pointer = non_analyzed_transactions.pop_front();
        }
        return Ok(true);
    }
}

