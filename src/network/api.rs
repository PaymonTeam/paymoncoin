use rustc_serialize::json;
use rustc_serialize::json::Json;
extern crate base64;

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
use std::collections::{HashMap, HashSet};
use model::transaction_validator::TransactionError;

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
const MAX_FIND_TXS: usize = 100;
const MAX_GET_TX_DATA: usize = 100;

pub struct API {
    listener: Listening,
    running: Arc<(Mutex<bool>, Condvar)>,
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct APIRequest<T: Serializable> {
    pub method: String,
    pub object: T,
}

#[derive(Debug)]
pub enum APIError {
    InvalidStringParametr,
    TipAbsent,
    TheSubHiveIsNotSolid,
    InvalidData,
    InvalidRequest,
    NoneParameter,
    IncorrectJsonParsing,
    Overflow
}

impl API {
    pub fn new(pmnc: AM<PaymonCoin>, port: u16, running: Arc<(Mutex<bool>, Condvar)>) -> Self {
        info!("Running API on port {}", port);
        let mut chain = Chain::new(API::api);
        chain.link_after(DefaultContentType);
        let listener = Iron::new(chain)
            .http(format!("127.0.0.1:{}", port))
            .expect("failed to start API server");

        unsafe {
            PMNC = Some(pmnc);
        }

        Self {
            listener,
            running,
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

    fn get_balances(pmnc: &mut PaymonCoin,
                    addresses: Vec<Address>,
                    mut hashes: Vec<Hash>,
                    threshold: u8) -> Result<Vec<u64>, TransactionError> {
        if threshold <= 0 || threshold > 100 {
            return Err(TransactionError::InvalidData);
        }

        let mut balances = HashMap::<Address, i64>::new();
        let mut index;

        if let Ok(mls) = pmnc.milestone.lock() {
            index = mls.latest_snapshot.index;
            if hashes.is_empty() {
                hashes.push(mls.latest_solid_subhive_milestone);
            }
        } else {
            panic!("broken milestone mutex");
        }

        for address in addresses.iter() {
            let mut value;
            if let Ok(mls) = pmnc.milestone.lock() {
                value = mls.latest_snapshot.get_balance(address).unwrap_or(0);
                debug!("val {} for {:?}", value, address);
            } else {
                panic!("broken milestone mutex");
            }
            balances.insert(*address, value);
        }

        let mut visited_hashes = HashSet::<Hash>::new();
        let mut diff = HashMap::<Address, i64>::new();

        for tip in hashes.iter() {
            if let Ok(hive) = pmnc.hive.lock() {
                if !hive.exists_transaction(*tip) {
                    return Err(TransactionError::InvalidAddress);
                }
            } else {
                panic!("broken hive mutex");
            }

            if let Ok(mut lv) = pmnc.ledger_validator.lock() {
                if !lv.update_diff(&mut visited_hashes, &mut diff, *tip)? {
                    return Err(TransactionError::InvalidAddress);
                }
            } else {
                panic!("broken hive mutex");
            }
        }

        diff.iter().for_each(|(key, value)| {
            let mut new_value = 0;
            let has_value;

            match balances.get(key) {
                Some(v) => {
                    new_value = v + value;
                    has_value = true;
                }
                None => {
                    has_value = false;
                }
            }

            if has_value {
                balances.insert(*key, new_value);
            }
        });

        let elements = addresses.iter().map(|address| *balances.get(address).unwrap() as u64)
            .collect::<Vec<u64>>();

        return Ok(elements);
    }

    pub fn get_new_inclusion_state_statement(pmnc: &mut PaymonCoin,
                                             transactions: &Vec<Hash>,
                                             tps: &Vec<Hash>) -> Result<Vec<bool>, APIError> {
        let tips = tps.clone();
        let number_of_non_met_transactions = transactions.len();
        let mut inclusion_states = vec![0; number_of_non_met_transactions];
        debug!("start={:?} t={:?}", inclusion_states, transactions);
        let mut tips_index: Vec<u32> = Vec::new();
        if let Ok(hive) = pmnc.hive.lock() {
            for tip in tips.iter() {
                let tx = hive.storage_load_transaction(tip).unwrap();
                if tx.get_type() != TransactionType::HashOnly {
                    tips_index.push(tx.object.get_snapshot_index());
                }
            }
        } else {
            panic!("broken hive mutex");
        }

        let min_tips_index = tips_index.iter().fold(0u32, |a, b| {
            if a < *b {
                return a;
            } else {
                return *b;
            }
        });

        if min_tips_index > 0 {
            let max_tips_index = tips_index.iter().fold(0u32, |a, b| {
                if a > *b {
                    return a;
                } else {
                    return *b;
                }
            });

            let mut count = 0;
            if let Ok(hive) = pmnc.hive.lock() {
                for hash in transactions.iter() {
                    let transaction = hive.storage_load_transaction(hash).unwrap();
                    if transaction.get_type() == TransactionType::HashOnly || transaction.object.get_snapshot_index() == 0 {
                        inclusion_states[count] = -1;
                    } else if transaction.object.get_snapshot_index() > max_tips_index {
                        inclusion_states[count] = -1;
                    } else if transaction.object.get_snapshot_index() < max_tips_index {
                        inclusion_states[count] = 1;
                    }
                    count += 1;
                }
            } else {
                panic!("broken hive mutex");
            }
        }

        let mut analyzed_tips: HashSet<Hash> = HashSet::new();
        let mut same_index_transaction_count: HashMap<u32, u32> = HashMap::new();
        let mut same_index_tips: HashMap<u32, Vec<Hash>> = HashMap::new();

        if let Ok(hive) = pmnc.hive.lock() {
            // TODO: rem
            if let Ok(m) = pmnc.milestone.lock() {
                let transaction = hive.storage_load_transaction(&m.latest_solid_subhive_milestone).unwrap();
                same_index_tips.insert(transaction.object.snapshot, vec![m.latest_solid_subhive_milestone]);
            } else {
                panic!()
            }

            for tip in tips.iter() {
                let transaction = hive.storage_load_transaction(tip).unwrap();
                if transaction.get_type() == TransactionType::HashOnly {
                    return Err(APIError::TipAbsent);
                }
                let snapshot_index = transaction.object.get_snapshot_index();
                if !same_index_tips.contains_key(&snapshot_index) {
                    same_index_tips.insert(snapshot_index, Vec::new());
                }
                same_index_tips.get_mut(&snapshot_index).unwrap().push(*tip);
            }

            for i in 0..inclusion_states.len() {
                if inclusion_states[i] == 0 {
                    let transaction = hive.storage_load_transaction(&transactions[i]).unwrap();
                    let snapshot_index = transaction.object.get_snapshot_index();

                    if !same_index_transaction_count.contains_key(&snapshot_index) {
                        same_index_transaction_count.insert(snapshot_index, 0);
                    }
                    let map = same_index_transaction_count.clone();
                    same_index_transaction_count.insert(snapshot_index, map.get(&snapshot_index).unwrap() + 1);
                }
            }
        } else {
            panic!("broken hive mutex");
        }

        for index in same_index_transaction_count.keys() {
            if let Some(same_index_tip) = same_index_tips.get(index) {
                if !same_index_tip.is_empty() {
                    //has tips in the same index level
                    let flag = API::exhaustive_search_within_index(pmnc,
                                                                   &mut same_index_tip.clone(),
                                                                   &mut analyzed_tips,
                                                                   &transactions,
                                                                   &mut inclusion_states,
                                                                   *same_index_transaction_count.get(index).unwrap(),
                                                                   *index)?;
                    if !flag {
                        return Err(APIError::TheSubHiveIsNotSolid);
                    }
                }
            }
        }

        let mut inclusion_states_boolean: Vec<bool> = vec![false; inclusion_states.len()];

        for i in 0..inclusion_states.len() {
            inclusion_states_boolean[i] = inclusion_states[i] == 1;
        }
        Ok(inclusion_states_boolean)
    }

    fn exhaustive_search_within_index(pmnc: &mut PaymonCoin,
                                      non_analyzed_transactions: &mut Vec<Hash>,
                                      analyzed_tips: &mut HashSet<Hash>,
                                      transactions: &Vec<Hash>,
                                      inclusion_states: &mut Vec<i32>,
                                      mut count: u32,
                                      index: u32) -> Result<bool, APIError> {
        'main_loop: while let Some(pointer) = non_analyzed_transactions.pop() {
            if analyzed_tips.insert(pointer) {
                if let Ok(hive) = pmnc.hive.lock() {
                    let transaction = hive.storage_load_transaction(&pointer).unwrap();

                    if transaction.object.get_snapshot_index() == index {
                        if transaction.get_type() == TransactionType::HashOnly {
                            return Ok(false);
                        } else {
                            for i in 0..inclusion_states.len() {
                                if inclusion_states[i] < 1 && pointer == transactions[i] {
                                    inclusion_states[i] = 1;
                                    count -= 1;
                                    debug!("count={}", count);
                                    if count <= 0 {
                                        break 'main_loop;
                                    }
                                }
                            }
                            non_analyzed_transactions.push(transaction.get_trunk_transaction_hash());
                            non_analyzed_transactions.push(transaction.get_branch_transaction_hash());
                        }
                    }
                } else {
                    panic!("broken hive mutex");
                }
            }
        }

        return Ok(true);
    }

    fn invalid_subtangle_status(pmnc: &mut PaymonCoin) -> bool {
        if let Ok(milestone) = pmnc.milestone.lock() {
            return milestone.latest_solid_subhive_milestone_index == MILESTONE_START_INDEX;
        } else {
            return false;
        }
    }

    pub fn get_tips(pmnc: &mut PaymonCoin) -> Result<Vec<Hash>, APIError> {
        if let Ok(tips_vm) = pmnc.tips_vm.lock() {
            return Ok(tips_vm.get_tips().iter().map(|x| *x).collect::<Vec<Hash>>());
        } else {
            panic!("broken tips_view_model mutex")
        }
    }

    pub fn get_transactions_data(pmnc: &mut PaymonCoin, hashes: &Vec<Hash>) -> Result<Vec<String>, APIError> {
        use network::packet::SerializedBuffer;
        let mut elements = Vec::<String>::new();

        if let Ok(hive) = pmnc.hive.lock() {
            for hash in hashes {
                let mut tx = hive.storage_load_transaction(hash).unwrap();
                let string = tx.get_base64_data();
                elements.push(string);
            }
        } else {
            panic!("broken hive mutex")
        }

        if elements.len() > MAX_GET_TX_DATA {
            return Err(APIError::Overflow);
        }

        Ok(elements)
    }

    pub fn find_transactions(pmnc: &mut PaymonCoin,
                             addresses: &Vec<Address>,
                             tags: &Vec<Hash>,
                             approvees: &Vec<Hash>) -> Result<Vec<Hash>, APIError> {
        let mut found_transactions = HashSet::<Hash>::new();
        let mut contains_key = false;

        if addresses.is_empty() && tags.is_empty() && approvees.is_empty() {
            return Err(APIError::InvalidRequest);
        }

        let mut addresses_transactions = HashSet::<Hash>::new();
        if let Ok(hive) = pmnc.hive.lock() {
            for addr in addresses {
                if let Some(hashes) = hive.load_address_transactions(addr) {
                    for hash in hashes {
                        addresses_transactions.insert(hash);
                        found_transactions.insert(hash);
                    }
                }
            }
        }

        // TODO: make search for a Tag

        let mut approvee_transactions = HashSet::<Hash>::new();
        if let Ok(hive) = pmnc.hive.lock() {
            for approvee in approvees {
                if let Some(hashes) = hive.storage_load_approvee(approvee) {
                    for hash in hashes {
                        approvee_transactions.insert(hash);
                        found_transactions.insert(hash);
                    }
                }
            }
        }

        if !addresses_transactions.is_empty() {
            found_transactions.retain(|e| addresses_transactions.contains(e));
        }

        if !approvee_transactions.is_empty() {
            found_transactions.retain(|e| approvee_transactions.contains(e));
        }

        if found_transactions.len() > MAX_FIND_TXS {
            return Err(APIError::Overflow);
        }

        Ok(found_transactions.into_iter().collect::<Vec<Hash>>())
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
                                debug!("broadcastTransaction");
                                match json::decode::<rpc::BroadcastTransaction>(&json_str) {
                                    Ok(bt) => {
                                        unsafe {
                                            if let Some(ref arc) = PMNC {
                                                if let Ok(pmnc) = arc.lock() {
                                                    if let Ok(mut node) = pmnc.node.lock() {
                                                        debug!("rcvd tx");
                                                        node.on_api_broadcast_transaction_received(bt);
                                                    }
                                                }
                                            }
                                        }
                                        return Ok(Response::with((iron::status::Ok, "{}")));
                                    }
                                    Err(e) => return Ok(API::format_error_response
                                                            (&format!("Invalid data \
                                    {:?}", e)))
                                };
                            }
                            "getTransactionsToApprove" => {
                                debug!("getTransactionsToApprove");
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

                                                    debug!("num_walks={}", num_walks);

                                                    match API::get_transactions_to_approve(pmnc,
                                                                                           depth,
                                                                                           reference,
                                                                                           num_walks) {
                                                        Ok(Some((trunk, branch))) => {
                                                            let result = rpc::TransactionsToApprove {
                                                                branch,
                                                                trunk,
                                                            };
                                                            return format_success_response!(result);
                                                        }
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
                                debug!("getNodeInfo");
                                let result = rpc::NodeInfo {
                                    name: "PMNC 0.1".to_string()
                                };
                                format_success_response!(result)
                            }
                            "getBalances" => {
                                debug!("getBalances");

                                match json::decode::<rpc::GetBalances>(&json_str) {
                                    Ok(object) => {
                                        unsafe {
                                            if let Some(ref arc) = PMNC {
                                                if let Ok(ref mut pmnc) = arc.lock() {
                                                    match API::get_balances(pmnc,
                                                                            object.addresses,
                                                                            object.tips,
                                                                            object.threshold) {
                                                        Ok(balances) => {
                                                            let result = rpc::Balances {
                                                                balances
                                                            };
                                                            return format_success_response!(result);
                                                        }
                                                        Err(e) => {
                                                            error!("{:?}", e);
                                                            return Ok
                                                                (API::format_error_response
                                                                    ("Internal error"));
                                                        }
                                                    }
                                                }
                                            }
                                        };
                                        return Ok(API::format_error_response("Internal error"));
                                    }
                                    Err(e) => return Ok(API::format_error_response("Invalid data"))
                                };
                            }
                            "getInclusionStates" => {
                                match json::decode::<rpc::GetInclusionStates>(&json_str) {
                                    Ok(object) => {
                                        unsafe {
                                            if let Some(ref mut arc) = PMNC {
                                                if let Ok(ref mut pmnc) = arc.lock() {
                                                    match API::get_new_inclusion_state_statement(pmnc,
                                                                                                 &object.transactions.clone(),
                                                                                                 &object.tips.clone()) {
                                                        Ok(vec) => {
                                                            let result = rpc::InclusionStates {
                                                                booleans: vec
                                                            };
                                                            return format_success_response!(result);
                                                        }
                                                        _ => return Ok(API::format_error_response("Internal error"))
                                                    }
                                                } else {
                                                    panic!("broken pmnc mutex");
                                                }
                                            } else {
                                                panic!("None returned");
                                            }
                                        }
                                    }
                                    Err(e) => return Ok(API::format_error_response("Invalid data"))
                                }
                            }
                            "findTransactions" => {
                                match json::decode::<rpc::FindTransactions>(&json_str) {
                                    Ok(object) => {
                                        unsafe {
                                            if let Some(ref mut arc) = PMNC {
                                                if let Ok(ref mut pmnc) = arc.lock() {
                                                    match API::find_transactions(pmnc, &object.addresses, &object.tags, &object.approvees) {
                                                        Ok(vec) => {
                                                            let result = rpc::FoundedTransactions {
                                                                hashes: vec
                                                            };
                                                            return format_success_response!(result);
                                                        }
                                                        _ => return Ok(API::format_error_response("Internal error"))
                                                    }
                                                } else {
                                                    panic!("broken pmnc mutex");
                                                }
                                            } else {
                                                panic!("None returned");
                                            }
                                        }
                                    }
                                    Err(e) => return Ok(API::format_error_response("Invalid data"))
                                }
                            }
                            "getTips" => {
                                match json::decode::<rpc::GetTips>(&json_str) {
                                    Ok(object) => {
                                        unsafe {
                                            if let Some(ref mut arc) = PMNC {
                                                if let Ok(ref mut pmnc) = arc.lock() {
                                                    match API::get_tips(pmnc) {
                                                        Ok(vec) => {
                                                            let result = rpc::Tips {
                                                                hashes: vec
                                                            };
                                                            return format_success_response!(result);
                                                        }
                                                        _ => return Ok(API::format_error_response("Internal error"))
                                                    }
                                                } else {
                                                    panic!("broken pmnc mutex");
                                                }
                                            } else {
                                                panic!("None returned");
                                            }
                                        }
                                    }
                                    Err(e) => return Ok(API::format_error_response("Invalid data"))
                                }
                            }
                            "getTransactionsData" => {
                                match json::decode::<rpc::GetTransactionsData>(&json_str) {
                                    Ok(object) => {
                                        unsafe {
                                            if let Some(ref mut arc) = PMNC {
                                                if let Ok(ref mut pmnc) = arc.lock() {
                                                    match API::get_transactions_data(pmnc, &object.hashes) {
                                                        Ok(vec) => {
                                                            let result = rpc::TransactionsData {
                                                                transactions: vec
                                                            };
                                                            return format_success_response!(result);
                                                        }
                                                        _ => return Ok(API::format_error_response("Internal error"))
                                                    }
                                                } else {
                                                    panic!("broken pmnc mutex");
                                                }
                                            } else {
                                                panic!("None returned");
                                            }
                                        }
                                    }
                                    Err(e) => return Ok(API::format_error_response("Invalid data"))
                                }
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
}