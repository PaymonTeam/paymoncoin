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
use std::collections::{HashMap, HashSet};
use model::transaction_validator::TransactionError;
use std::collections::LinkedList;

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
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct APIRequest<T: Serializable> {
    pub method: String,
    pub object: T,
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

//        let addresses: LinkedList<Address> = addrss.iter().map(|address| *address).collect::<LinkedList<Address>>();
        let mut hashes = Vec::<Hash>::new();
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
                                                        },
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