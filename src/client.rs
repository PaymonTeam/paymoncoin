extern crate byteorder;
extern crate mio;
extern crate rand;
extern crate slab;
//extern crate ethcore_bigint as bigint;
//extern crate memorydb;
//extern crate patricia_trie;
extern crate env_logger;
extern crate rustc_serialize;
extern crate iron;
extern crate ntrumls;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate native_windows_gui as nwg;

#[macro_use]
pub mod utils;
pub mod network;
pub mod model;
pub mod storage;

use std::io::{self, Read};
use nwg::{Event, Ui, simple_message, fatal_message, dispatch_events};
use rustc_serialize::json;
use rustc_serialize::json::{Json, ToJson};
use network::rpc;
use network::api::APIRequest;
use std::collections::BTreeMap;
use std::io::BufReader;
use std::io::Write;
use std::net::SocketAddr;
use std::io::BufRead;
use std::time;

use AppId::*;
use ntrumls::{Signature, PrivateKey, PublicKey, NTRUMLS, PQParamSetID};
use storage::Hive;
use model::{Transaction, TransactionObject};
use model::transaction::*;

use rand::Rng;
use self::rustc_serialize::hex::{ToHex, FromHex};
use std::str::FromStr;

#[derive(Debug, Clone, Hash)]
pub enum AppId {
    // Controls
    MainWindow,

    PrivateKeyInput,
    GeneratePrivateKeyButton,
    CopyPrivateKeyButton,

    AddressInput,
    GenerateAddressButton,
    CopyAddressButton,

    NodesCB,
    NodeAddressInput,
    NodeRemoveButton,
    NodeAddButton,

    SendToInput,
    SendAmountInput,
    SendButton,

    StatusLabel,

    TransactionsList,

    Label(u8),

    // Events
    GeneratePrivateKey,
    GenerateAddress,
    AddNeighbor,
    RemoveNeighbor,
    Send,
    MainWindowLoad,

    // Resources
    MainFont,
    TextFont,
}

static mut SK:Option<PrivateKey> = None;
static mut PK:Option<PublicKey> = None;
static mut ADDRESS:Option<Address> = None;
static mut NEIGHBORS:Option<Vec<SocketAddr>> = None;
static mut NTRUMLS_INSTANCE:Option<NTRUMLS> = None;

fn send_coins(addr: Address, amount: u32) {
    let mut h0;
    let mut h1;

    unsafe {
        if let Some(ref n) = NEIGHBORS {
            let mut st = json::encode(&rpc::GetTransactionsToApprove { }).unwrap();
            let mut s = Json::from_str(&st).unwrap();
            s.as_object_mut().unwrap().insert("method".to_string(), "getTransactionsToApprove".to_string().to_json());

            match send_request(s, n[0].clone()) {
                Some(json) => {
                    match json.as_object() {
                        Some(json) => {
                            println!("{:?}", json);
                            if !json.contains_key("trunk") || !json.contains_key("branch") {
                                error!("no tips");
                                return;
                            } else {
                                h0 = HASH_NULL;
                                h0.copy_from_slice(&json.get("trunk").unwrap().as_string().unwrap
                                ().from_hex().unwrap());
                                h1 = HASH_NULL;
                                h1.copy_from_slice(&json.get("branch").unwrap().as_string().unwrap
                                ().from_hex().unwrap());
                            }
                        }
                        _ => {
                            println!("no json");
                            return;
                        }
                    }
                }, //println!("Server name: {}", json["name"]),
                None => {
                    println!("no reponse");
                    return;
                }
            }
        } else {
            return;
        }
    }

    println!("sending {} to {:?}", amount, addr);
//    let mut st = json::encode(&rpc::GetNodeInfo {}).unwrap();
    let mut transaction = TransactionObject {
        address: addr,
        attachment_timestamp: 0u64,
        attachment_timestamp_lower_bound: 0u64,
        attachment_timestamp_upper_bound: 0u64,
        branch_transaction: h0,
        trunk_transaction: h1,
        hash: HASH_NULL,
        nonce: 0,
        tag: HASH_NULL,
        timestamp: time::SystemTime::now().elapsed().unwrap().as_secs(),
        value: amount,
        data_type: TransactionType::Full,
        signature: Signature(vec![]),
        signature_pubkey: PublicKey(vec![]),
        snapshot: 0,
        solid: false,
        height: 0,
    };

    let mut transaction = Transaction::from_object(transaction);
    let mwm = 8u32;
    transaction.object.nonce = transaction.find_nonce(mwm);
    transaction.object.hash = transaction.calculate_hash();

    unsafe {
        if let Some(ref sk) = SK {
            if let Some(ref pk) = PK {
//            if let Some(my_addr) = ADDRESS {
                transaction.object.signature = transaction.calculate_signature(sk, pk).expect("failed to calculate signature");
                println!("signed");
                transaction.object.signature_pubkey = pk.clone();

                println!("{:?}", pk);
                println!("{:?}", transaction.object.hash);
                println!("{:?}", transaction.object.signature);
//            } else {
//                println!("Address is None")
//            }
            } else {
                println!("pk is none");
            }
        } else {
            println!("sk is none");
        }
    }

    let mut st = json::encode(&rpc::BroadcastTransaction { transaction: transaction.object })
        .unwrap();
    let mut s = Json::from_str(&st).unwrap();
    s.as_object_mut().unwrap().insert("method".to_string(), "broadcastTransaction".to_string()
        .to_json());

    unsafe {
        if let Some(ref n) = NEIGHBORS {
            match send_request(s, n[0].clone()) {
                Some(json) => return, //println!("Server name: {}", json["name"]),
                None => println!("no reponse")
            }
        }
    }

}

fn send_request(request: Json, addr: SocketAddr) -> Option<Json> {
    let content = request.to_string();
    let content_length = content.len();

    println!("sending json {}", content);

    let incoming_request = format!("POST / HTTP/1\
    .0\r\ncontent-type:application/json\r\nX-PMNC-API-Version: 0\
    .1\r\nHost:localhost\r\ncontent-length:{}\r\n\r\n{}", content_length, content);

    if let Ok(mut s) = std::net::TcpStream::connect(addr) {
        s.write(incoming_request.as_bytes()).unwrap();
        let mut reader = BufReader::new(&mut s);
        let mut resp = String::new();

        while let Ok(l) = reader.read_line(&mut resp) {
            if l > 0 {
                if resp.starts_with("HTTP") {
                    let v: Vec<&str> = resp.splitn(3, ' ').collect();
                    let ret_code = v[1];
                    if ret_code != "200" {
                        error!("error code: {}", ret_code);
                        return None;
                    }
                } else if resp.starts_with('{') {
                    break;
                }
            } else {
                resp = "".to_string();
                break;
            }
            resp = "".to_string();
        }
        println!("resp={}", resp);

        if let Ok(json) = Json::from_str(&resp) {
            return Some(json);
        } else {
            return None;
        }
    } else {
        println!("connection failed");
        return None;
    }
}

fn send_transaction() {
}

fn add_neighbor(addr: String) -> bool {
    unsafe {
        if NEIGHBORS.is_none() {
            NEIGHBORS = Some(vec!["127.0.0.1:80".parse::<SocketAddr>().unwrap()]);
        }
    }
    if let Ok(ip) = addr.parse::<SocketAddr>() {
        unsafe {
            if let Some(ref mut n) = NEIGHBORS {
                if !n.contains(&ip) {
                    n.push(ip.clone());
                }
//              NEIGHBORS.unwrap().push(ip.clone());
            }
        }
        return true;
    }
    false
}

fn generate_private_key() -> (String, String) {
    let (addr, sk, pk) = Hive::generate_address();

    let sk_str = sk.0.to_hex().to_uppercase();

    unsafe {
        SK = Some(sk);
        PK = Some(pk);
    }

    (sk_str, format!("{:?}", addr))
}

fn generate_address_from_private_key(sk_string: String) -> Result<String, String> {
    match sk_string.from_hex() {
        Ok(hex) => {
            unsafe {
                if let Some(ref mls) = NTRUMLS_INSTANCE {
                    if let Some(ref fg) = mls.unpack_fg_from_private_key(&PrivateKey(hex)) {
                        let (sk, pk) = mls.generate_keypair_from_fg(fg).unwrap();
                        let addr_str = format!("{:?}", Address::from_public_key(&pk));
                        SK = Some(sk);
                        PK = Some(pk);
                        return Ok(addr_str);
                    } else {
                        return Err("failed to unpack fg".to_string());
                    }
                } else {
                    return Err("Internal error #1".to_string());
                }
            }
        }
        Err(e) => Err(format!("{:?}", e))
    }
}

nwg_template!(
    head: setup_ui<AppId>,
    controls: [
        (MainWindow, nwg_window!( title="PaymonCoin"; size=(640, 480) )),

    // private key
        (Label(0), nwg_label!( parent=MainWindow; text="Private key"; position=(5,15); size=(75,25); font=Some(TextFont) )),
        (PrivateKeyInput, nwg_textinput!( parent=MainWindow; position=(80,13); size=(185,22); font=Some(TextFont) )),
        (GeneratePrivateKeyButton, nwg_button!( parent=MainWindow; text="Generate"; position=
        (270, 13); size=(80,22); font=Some(MainFont) )),
        (CopyPrivateKeyButton, nwg_button!( parent=MainWindow; text="Copy"; position=
        (270+85, 13); size=(80,22); font=Some(MainFont) )),

    // address
        (Label(1), nwg_label!( parent=MainWindow; text="Address"; position=(5,40); size=(75,
        25); font=Some(TextFont) )),
        (AddressInput, nwg_textinput!( parent=MainWindow; position=(80,13+25); size=(185,22);
        font=Some(TextFont) )),
        (GenerateAddressButton, nwg_button!( parent=MainWindow; text="Generate"; position=
        (270, 13+25); size=(80,22); font=Some(MainFont) )),
        (CopyAddressButton, nwg_button!( parent=MainWindow; text="Copy"; position=
        (270+85, 13+25); size=(80,22); font=Some(MainFont) )),

    // neighbors
        (Label(2), nwg_label!( parent=MainWindow; text="Neighbors"; position=(5,40+35); size=(75,
        25); font=Some(TextFont) )),
        (NodesCB, nwg_listbox!( data=String; parent=MainWindow; position=(80,13+25+35); size=(185,
        70); font=Some(TextFont) )),
        (NodeAddressInput, nwg_textinput!( parent=MainWindow; position=(80+185+5,13+25+35); size=
        (185,22); font=Some(TextFont) )),
        (NodeAddButton, nwg_button!( parent=MainWindow; text="Add"; position=
        (270, 13+25+35+25); size=(80,22); font=Some(MainFont) )),
        (NodeRemoveButton, nwg_button!( parent=MainWindow; text="Remove"; position=
        (270+85, 13+25+35+25); size=(80,22); font=Some(MainFont) )),

    // sending
        (Label(3), nwg_label!( parent=MainWindow; text="Send"; position=(5,40+35+85); size=(75,
        25); font=Some(TextFont) )),
        (SendToInput, nwg_textinput!( parent=MainWindow; position=(80,13+25+35+85); size=
        (185,22); font=Some(TextFont); placeholder=Some("To address") )),
        (SendAmountInput, nwg_textinput!( parent=MainWindow; position=(80,13+25+35+85+25);
        size=(185,22); font=Some(TextFont); placeholder=Some("Amount") )),
        (SendButton, nwg_button!( parent=MainWindow; text="Send"; position=
        (270, 13+25+35+85); size=(80,22); font=Some(MainFont) )),

    // status
        (Label(4), nwg_label!( parent=MainWindow; text="Status: "; position=(5,
        40+35+85+60); size=(75,25); font=Some(TextFont) )),
        (StatusLabel, nwg_label!( parent=MainWindow; text=""; position=(5+75,
        40+35+85+60); size=(500,25); font=Some(TextFont) )),

    //transactions
        (Label(5), nwg_label!( parent=MainWindow; text="Transactions"; position=(5,
        40+35+85+60+35); size=(75,25); font=Some(TextFont) )),
        (TransactionsList, nwg_listbox!( data=String; parent=MainWindow; position=(5,
        40+35+85+60+35+25); size=(630, 200); font=Some(TextFont) ))
    ];
    events: [
        (GeneratePrivateKeyButton, GeneratePrivateKey, Event::Click, |ui,_,_,_| {
//            simple_message("msg", &format!("Hello {}!", your_name.get_text()) );
            let (mut sk_input, mut address_input) = nwg_get_mut!(ui; [(PrivateKeyInput,
nwg::TextInput), (AddressInput, nwg::TextInput)]);
            let (sk_string, address_string) = generate_private_key();
            sk_input.set_text(&sk_string);
            address_input.set_text(&address_string);
        }),
        (GenerateAddressButton, GenerateAddress, Event::Click, |ui,_,_,_| {
            let (mut sk_input, mut address_input) = nwg_get_mut!(ui; [(PrivateKeyInput,
nwg::TextInput), (AddressInput, nwg::TextInput)]);
            let mut status_label = nwg_get_mut!(ui; (StatusLabel, nwg::Label));

            let sk_string = sk_input.get_text();
            match generate_address_from_private_key(sk_string.clone()) {
                Ok(address_string) => address_input.set_text(&address_string),
                Err(e) => status_label.set_text(&e)
            };
        }),
        (NodeAddButton, AddNeighbor, Event::Click, |ui,_,_,_| {
            let mut neighbors_list = nwg_get_mut!(ui; (NodesCB, nwg::ListBox<String>));
            let mut node_address_input = nwg_get_mut!(ui; (NodeAddressInput, nwg::TextInput));

            let s = node_address_input.get_text();
            if add_neighbor(s.clone()) {
                neighbors_list.push(s);
                neighbors_list.sync();
            }
//            let sk_string = sk_input.get_text();
//            match generate_address_from_private_key(sk_string.clone()) {
//                Ok(address_string) => address_input.set_text(&address_string),
//                Err(e) => status_label.set_text(&e)
//            };
        }),
        (SendButton, Send, Event::Click, |ui,_,_,_| {
            let (mut send_address_input, mut send_amount_input) = nwg_get_mut!(ui; [
            (SendToInput, nwg::TextInput), (SendAmountInput, nwg::TextInput)]);
            let mut status_label = nwg_get_mut!(ui; (StatusLabel, nwg::Label));

            if let Ok(addr) = send_address_input.get_text().parse::<Address>() {
                if let Ok(amount) = send_amount_input.get_text().parse::<u32>() {
                    send_coins(addr, amount);
                } else {
                    status_label.set_text("Wrong amount");
                }
            } else {
                status_label.set_text("Wrong participant address");
            }
        })
    ];
    resources: [
        (MainFont, nwg_font!(family="Arial"; size=16)),
        (TextFont, nwg_font!(family="Arial"; size=16))
    ];
    values: []
);

#[cfg(windows)]
fn main() {
    unsafe {
        NTRUMLS_INSTANCE = Some(NTRUMLS::with_param_set(PQParamSetID::Security269Bit));
    }

    let app: Ui<AppId>;

    match Ui::new() {
        Ok(_app) => { app = _app; }
        Err(e) => { fatal_message("Fatal Error", &format!("{:?}", e)); }
    }

    if let Err(e) = setup_ui(&app) {
        fatal_message("Fatal Error", &format!("{:?}", e));
    }

    dispatch_events();
}