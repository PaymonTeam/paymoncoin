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

pub struct API {
    listener: Listening,
    running: Arc<(Mutex<bool>, Condvar)>,
    pmnc: AM<PaymonCoin>
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct APIRequest<T : Serializable> {
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

        Self {
            listener,
            running,
            pmnc
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
                                        
                                        return Ok(Response::with((iron::status::Ok, "{}")));
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

    pub fn shutdown(&mut self) {
    }
}