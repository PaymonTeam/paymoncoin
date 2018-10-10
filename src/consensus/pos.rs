extern crate rhododendron;

use env_logger::LogBuilder;
use log::{LogRecord, LogLevelFilter};
use std::{
    env,
    collections::HashMap,
    fmt::{Debug, Formatter, Error},
    thread,
    time::Duration,
    sync::{Arc, Mutex}
};
use futures;
use futures::oneshot;
use futures::{AndThen};
use futures::executor::{Run, Spawn};
use futures::prelude::*;
use futures::task::Task;
use tokio;
//use network::node::{OutputStream, InputStream, PacketData, Pair};
use network::node::{PacketData, Pair};
use network::rpc::ConsensusValue;
use network::packet::Serializable;
use network::Neighbor;
use utils::AM;
use std::net::SocketAddr;
use std::hash::Hash;
use std::marker::PhantomData;
use self::rhododendron::*;

#[test]
pub fn pos_test() {
    let nodes_count = 4;

//    let agreement = agree()
}

#[derive(Debug)]
pub enum BFTError {
    NotAllValidatorsHaveValue,
    NotAllValidatorsHaveResultVector,
    InvalidNumberOfValidators,
    ConsensusWasNotAchieved,
}

type ValidatorIndex = u32;
type ValidatorDataType = u32;
const N: usize = 4;
const M: usize = 2;

#[derive(Debug)]
pub struct Validator {
    index: ValidatorIndex,
    loyal: bool,
    node: Neighbor,
}

impl Validator {
    pub fn from_index(index: ValidatorIndex, loyal: bool) -> Self {
        Validator {
            index,
            loyal,
            node: Neighbor::from_address("127.0.0.1".parse::<SocketAddr>().unwrap()),
        }
    }
}