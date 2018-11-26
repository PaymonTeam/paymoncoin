use std::collections::HashMap;

use std::fs::File;
use std::io::prelude::*;
use std::io::{BufRead, BufReader};

// TODO: remove
pub const PORT: u16 = 44832;

#[derive(Clone)]
pub struct Configuration {
    params: HashMap<u8, ConfigurationValue>
}

#[derive(Clone)]
pub enum ConfigurationValue {
    String(String),
    Int(i32),
    Float(f32),
    Bool(bool),
}

#[derive(Debug, Copy, Clone)]
pub enum ConfigurationSettings {
    Config,
    Port,
    ApiHost,
    UdpReceiverPort,
    TcpReceiverPort,
    TestNet,
    Debug,
    RemoteLimitApi,
    RemoteAuth,
    Neighbors,
    DBPath,
    DBCacheSize,
    PRemoveRequest,
    PDropTransaction,
    PSelectMilestoneChild,
    PSendMilestone,
    PReplyRandomTip,
    PPropagateRequest,
    MainDB,
    SendLimit,
    MaxPeers,
    DNSResolutionEnabled,
    DNSRefresherEnabled,
    Coordinator,
    Revalidate,
    RescanDB,
    MinRandomWalks,
    MaxRandomWalks,
    MaxFindTransactions,
    MaxRequestsList,
    MaxBodyLength,
    MaxDepth,
    MainNetMWM,
    TestNetMWM,
    QSizeNode,
    PDropCacheEntry,
    CacheSizeBytes,
}

impl Configuration {
    pub fn set_string(&mut self, param: ConfigurationSettings, value: &str) {
        let _ = self.params.insert(param as u8, ConfigurationValue::String(value.to_string()));
    }

    pub fn set_int(&mut self, param: ConfigurationSettings, value: i32) {
        let _ = self.params.insert(param as u8, ConfigurationValue::Int(value));
    }

    pub fn set_float(&mut self, param: ConfigurationSettings, value: f32) {
        let _ = self.params.insert(param as u8, ConfigurationValue::Float(value));
    }

    pub fn set_bool(&mut self, param: ConfigurationSettings, value: bool) {
        let _ = self.params.insert(param as u8, ConfigurationValue::Bool(value));
    }

    pub fn get_string(&self, param: ConfigurationSettings) -> Option<String> {
        let param = param as u8;

        self.params.get(&param).and_then(|p| {
            if let &ConfigurationValue::String(ref v) = p {
                Some(v.clone())
            } else {
                None
            }
        })
    }

    pub fn get_int(&self, param: ConfigurationSettings) -> Option<i32> {
        let param = param as u8;

        self.params.get(&param).and_then(|p| {
            if let &ConfigurationValue::Int(v) = p {
                Some(v)
            } else {
                None
            }
        })
    }

    pub fn get_float(&mut self, param: ConfigurationSettings) -> Option<f32> {
        let param = param as u8;

        self.params.get(&param).and_then(|p| {
            if let &ConfigurationValue::Float(v) = p {
                Some(v)
            } else {
                None
            }
        })
    }

    pub fn get_bool(&mut self, param: ConfigurationSettings) -> Option<bool> {
        let param = param as u8;

        self.params.get(&param).and_then(|p| {
            if let &ConfigurationValue::Bool(v) = p {
                Some(v)
            } else {
                None
            }
        })
    }

    pub fn new() -> Self {
        let mut config = Configuration {
            params : HashMap::<u8, ConfigurationValue>::new()
        };

        let mut params_map = HashMap::<String, ConfigurationSettings>::new();
        params_map.insert("port".to_string(), ConfigurationSettings::Port);
        params_map.insert("api_host".to_string(), ConfigurationSettings::ApiHost);
        params_map.insert("udp_port".to_string(), ConfigurationSettings::UdpReceiverPort);
        params_map.insert("tcp_port".to_string(), ConfigurationSettings::TcpReceiverPort);
        params_map.insert("debug".to_string(), ConfigurationSettings::Debug);
        params_map.insert("neighbors".to_string(), ConfigurationSettings::Neighbors);
        params_map.insert("max_peers".to_string(), ConfigurationSettings::MaxPeers);

        config.set_int(ConfigurationSettings::Port, 44832);
        config.set_string(ConfigurationSettings::ApiHost, "localhost");
        config.set_int(ConfigurationSettings::UdpReceiverPort, 14600);
        config.set_int(ConfigurationSettings::TcpReceiverPort, 15600);
        config.set_bool(ConfigurationSettings::TestNet, false);
        config.set_bool(ConfigurationSettings::Debug, false);
        config.set_string(ConfigurationSettings::RemoteLimitApi, "");
        config.set_string(ConfigurationSettings::RemoteAuth, "");
        config.set_string(ConfigurationSettings::Neighbors, "");
        config.set_string(ConfigurationSettings::DBPath, "data");
        config.set_int(ConfigurationSettings::DBCacheSize, 100000); //KB
        config.set_string(ConfigurationSettings::Config, "config.ini");
        config.set_float(ConfigurationSettings::PRemoveRequest, 0.01);
        config.set_float(ConfigurationSettings::PDropTransaction, 0.0);
        config.set_float(ConfigurationSettings::PSelectMilestoneChild, 0.7);
        config.set_float(ConfigurationSettings::PSendMilestone, 0.02);
        config.set_float(ConfigurationSettings::PReplyRandomTip, 0.66);
        config.set_float(ConfigurationSettings::PPropagateRequest, 0.01);
        config.set_string(ConfigurationSettings::MainDB, "rocksdb");
        config.set_float(ConfigurationSettings::SendLimit, -1.0);
        config.set_int(ConfigurationSettings::MaxPeers, 5);
        config.set_bool(ConfigurationSettings::DNSRefresherEnabled, true);
        config.set_bool(ConfigurationSettings::DNSResolutionEnabled, true);
        config.set_bool(ConfigurationSettings::Revalidate, false);
        config.set_bool(ConfigurationSettings::RescanDB, false);
        config.set_int(ConfigurationSettings::MainNetMWM, 8);
        config.set_int(ConfigurationSettings::TestNetMWM, 7);

        config.set_int(ConfigurationSettings::MinRandomWalks, 5);
        config.set_int(ConfigurationSettings::MaxRandomWalks, 27);
        config.set_int(ConfigurationSettings::MaxDepth, 15);

        config.set_int(ConfigurationSettings::MaxFindTransactions, 100000);
        config.set_int(ConfigurationSettings::MaxRequestsList, 1000);
        config.set_int(ConfigurationSettings::MaxBodyLength, 1000000);

        config.set_int(ConfigurationSettings::QSizeNode, 1000);
        config.set_float(ConfigurationSettings::PDropCacheEntry, 0.02);
        config.set_int(ConfigurationSettings::CacheSizeBytes, 15000);

        let mut f = File::open(config.get_string(ConfigurationSettings::Config).unwrap()).expect
        ("config file not found");
        let file = BufReader::new(&f);
        for line in file.lines() {
            if let Ok(l) = line {
                let param = l.splitn(2, '=').collect::<Vec<&str>>();
                if param.len() == 2 {
                    if let Some(cs) = params_map.get(param[0]) {
                        let key = param[0];
                        let value = String::from(String::from(param[1]).trim());
                        match config.params.get(&(cs.clone() as u8)) {
                            Some(ConfigurationValue::String(_)) => config.set_string(*cs, &value),
                            Some(ConfigurationValue::Int(_)) => config.set_int(*cs, value
                                .parse::<i32>().expect(&format!("Invalid param {:?}", cs))),
                            Some(ConfigurationValue::Float(_)) => config.set_float(*cs, value
                                .parse::<f32>().expect(&format!("Invalid param {:?}", cs))),
                            Some(ConfigurationValue::Bool(_)) => config.set_bool(*cs, value
                                .parse::<bool>().expect(&format!("Invalid param {:?}", cs))),
                            _ => panic!("Invalid param {}", key)
                        }
                    }
                }
            }
        }

        config
    }

}