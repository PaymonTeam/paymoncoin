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

    fn get_transactions_to_approve(pmnc: &mut PaymonCoin, mut depth: u32, mut num_walks: u32) ->
    Result<Option<(Hash, Hash)>, TransactionError> {
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

        if let Ok(tips_manager) = pmnc.tips_manager.lock() {
            h0 = tips_manager.transaction_to_approve(&mut visited_hashes, &mut diff, HASH_NULL,
                                                     HASH_NULL, depth, num_walks)?;
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
            h1 = tips_manager.transaction_to_approve(&mut visited_hashes, &mut diff, HASH_NULL,
                                                     h0.unwrap(), depth, num_walks)?;
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
                                    Ok(bt) => {
                                        unsafe {
                                            if let Some(ref arc) = PMNC {
                                                if let Ok(ref mut pmnc) = arc.lock() {
                                                    if API::invalid_subtangle_status(pmnc) {
                                                        return Ok(API::format_error_response("The subhive has not been updated yet"));
                                                    }
                                                    let depth = 3;
                                                    let num_walks = 1;
                                                    match API::get_transactions_to_approve(pmnc, depth, num_walks) {
                                                        Ok(Some((trunk, branch))) => {
                                                            let result = rpc::TransactionsToApprove {
                                                                branch,
                                                                trunk
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

    pub fn find_transaction_statement(request: HashMap<String, i64>) -> Result<Vec<Hash>, TransactionError> {
        let mut found_transactions: HashSet<Hash> = HashSet::new();
        let mut contains_key = false;

        let mut bundles_transactions: HashSet<Hash> = HashSet::new();
        if request.contains_key("bundles") {
            // TODO get_parameter_as_set
            let bundles: HashSet<Hash> = HashSet::new();//get_parameter_as_set(request, "bundles", HASH_SIZE);
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
        if request.contains_key("addresses") {
            //TODO
            let mut addresses: HashSet<Address> = HashSet::new();//getParameterAsSet(request, "addresses", HASH_SIZE);
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
        if request.contains_key("tags") {
            let mut tags: HashSet<Hash> = HashSet::new(); //getParameterAsSet(request,"tags",0);
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
        if request.contains_key("approvees") {
            //TODO
            let approvees: HashSet<Hash> = HashSet::new();//getParameterAsSet(request, "approvees", HASH_SIZE);
            for approvee in approvees.iter() {
                //   approveeTransactions.addAll(TransactionViewModel.fromHash(instance.tangle, new Hash(approvee)).getApprovers(instance.tangle).getHashes());
            }
            for hash in approvee_transactions.iter() {
                found_transactions.insert(*hash);
            }
            contains_key = true;
        }

        if !contains_key {
            return Err(TransactionError::InvalidData);
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
    fn get_parameter_as_set(request: HashMap<String, i64>, param_name: String, size: i32)->  Result<HashSet<Hash>,APIError>  {
    /*
            HashSet<String> result = getParameterAsList(request,paramName,size).stream().collect(Collectors.toCollection(HashSet::new));
            if (result.contains(Hash.NULL_HASH.toString())) {
                throw new ValidationException("Invalid " + paramName + " input");
            }
            return result;
        */
        unimplemented!();
    }
    /*        private AbstractResponse getNewInclusionStateStatement(final List<String> trans, final List<String> tps) throws Exception {
            final List<Hash> transactions = trans.stream().map(Hash::new).collect(Collectors.toList());
            final List<Hash> tips = tps.stream().map(Hash::new).collect(Collectors.toList());
            int numberOfNonMetTransactions = transactions.size();
            final int[] inclusionStates = new int[numberOfNonMetTransactions];

            List<Integer> tipsIndex = new LinkedList<>();
            {
                for(Hash tip: tips) {
                    TransactionViewModel tx = TransactionViewModel.fromHash(instance.tangle, tip);
                    if (tx.getType() != TransactionViewModel.PREFILLED_SLOT) {
                        tipsIndex.add(tx.snapshotIndex());
                    }
                }
            }
            int minTipsIndex = tipsIndex.stream().reduce((a,b) -> a < b ? a : b).orElse(0);
            if(minTipsIndex > 0) {
                int maxTipsIndex = tipsIndex.stream().reduce((a,b) -> a > b ? a : b).orElse(0);
                int count = 0;
                for(Hash hash: transactions) {
                    TransactionViewModel transaction = TransactionViewModel.fromHash(instance.tangle, hash);
                    if(transaction.getType() == TransactionViewModel.PREFILLED_SLOT || transaction.snapshotIndex() == 0) {
                        inclusionStates[count] = -1;
                    } else if(transaction.snapshotIndex() > maxTipsIndex) {
                        inclusionStates[count] = -1;
                    } else if(transaction.snapshotIndex() < maxTipsIndex) {
                        inclusionStates[count] = 1;
                    }
                    count++;
                }
            }

            Set<Hash> analyzedTips = new HashSet<>();
            Map<Integer, Integer> sameIndexTransactionCount = new HashMap<>();
            Map<Integer, Queue<Hash>> sameIndexTips = new HashMap<>();
            for (final Hash tip : tips) {
                TransactionViewModel transactionViewModel = TransactionViewModel.fromHash(instance.tangle, tip);
                if (transactionViewModel.getType() == TransactionViewModel.PREFILLED_SLOT){
                    return ErrorResponse.create("One of the tips absents");
                }
                int snapshotIndex = transactionViewModel.snapshotIndex();
                sameIndexTips.putIfAbsent(snapshotIndex, new LinkedList<>());
                sameIndexTips.get(snapshotIndex).add(tip);
            }
            for(int i = 0; i < inclusionStates.length; i++) {
                if(inclusionStates[i] == 0) {
                    TransactionViewModel transactionViewModel = TransactionViewModel.fromHash(instance.tangle, transactions.get(i));
                    int snapshotIndex = transactionViewModel.snapshotIndex();
                    sameIndexTransactionCount.putIfAbsent(snapshotIndex, 0);
                    sameIndexTransactionCount.put(snapshotIndex, sameIndexTransactionCount.get(snapshotIndex) + 1);
                }
            }
            for(Integer index : sameIndexTransactionCount.keySet()) {
                Queue<Hash> sameIndexTip = sameIndexTips.get(index);
                if (sameIndexTip != null) {
                    //has tips in the same index level
                    if (!exhaustiveSearchWithinIndex(sameIndexTip, analyzedTips, transactions, inclusionStates, sameIndexTransactionCount.get(index), index)) {
                        return ErrorResponse.create("The subtangle is not solid");
                    }
                }
            }
            final boolean[] inclusionStatesBoolean = new boolean[inclusionStates.length];
            for(int i = 0; i < inclusionStates.length; i++) {
                inclusionStatesBoolean[i] = inclusionStates[i] == 1;
            }
            {
                return GetInclusionStatesResponse.create(inclusionStatesBoolean);
            }
        }*/
}

