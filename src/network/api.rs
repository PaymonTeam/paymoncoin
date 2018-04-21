use rustc_serialize::json::Json;

use iron;
use iron::{Iron, Request, Response, IronResult, AfterMiddleware, Chain};
use iron::prelude::*;
use iron::status;
use network::Node;
use network::paymoncoin::PaymonCoin;
use utils::{AM, AWM};
use std;
use std::io::Read;

struct DefaultContentType;
impl AfterMiddleware for DefaultContentType {
    fn after(&self, _req: &mut Request, mut resp: Response) -> IronResult<Response> {
        resp.headers.set(iron::headers::ContentType::json());
        Ok(resp)
    }
}

pub struct API {
    pmnc: AM<PaymonCoin>
}

impl API {
    pub fn new(pmnc: AM<PaymonCoin>, port: u16) -> Self {
        let mut chain = Chain::new(API::info);
        chain.link_after(DefaultContentType);
        Iron::new(chain)
            .http(format!("localhost:{}", port))
            .expect("failed to start API server");

        Self {
            pmnc
        }
    }

    fn format_error_response(err: &str) -> Response {
        Response::with((iron::status::Ok, format!("{{\"error\":\"{}\"}}\n", err.to_string())))
    }

    fn info(req: &mut Request) -> IronResult<Response> {
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
        let mut json = Json::from_str(
            std::str::from_utf8(&body)
                .map_err(|e| IronError::new(e, (status::InternalServerError, "Invalid UTF-8 string")))?)
            .map_err(|e| IronError::new(e, (status::InternalServerError, "Invalid JSON")))?;

        match json.as_object() {
            Some(o) => {

                if !o.contains_key("method") {
                    return Ok(API::format_error_response("No 'method' parameter"));
                }

                match o.get("method").unwrap().as_string() {
                    Some(method) => {
                        match method {
                            "getNodeInfo" => {
                                println!("getNodeInfo");
                            }
                            _ => return Ok(API::format_error_response("Unknown 'method' parameter"))
                        }
                    }
                    None => return Ok(API::format_error_response("Invalid 'method' parameter"))
                }
            }
            None => return Ok(API::format_error_response("Invalid request"))
        };

        let resp_body = "".to_string();
        Ok(Response::with((iron::status::Ok, resp_body)))
    }

}