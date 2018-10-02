use env_logger::LogBuilder;
use log::{LogRecord, LogLevelFilter};
use std::{
    env,
    collections::HashMap,
    fmt::{Debug, Formatter, Error},
    thread,
    time::Duration,
};
use futures;
use futures::executor::{Run, Spawn};
use futures::prelude::*;
use futures::task::Task;
use tokio;
use network::node::{OutputStream, InputStream, PacketData, Pair};
use network::rpc::ConsensusValue;
use network::packet::Serializable;
use network::Neighbor;
use utils::AM;
use std::net::SocketAddr;

#[test]
pub fn pos_test() {
    let mut input = InputStream { queue: vec![] };
    let mut output = OutputStream { queue: vec![] };
//    Neighbor::from_address("")
//    let neighbors = make_am!(vec![
//
//    ]);

    let consensus_value_future = ConsensusValueRetriever::retrieve(input, output, 1).map(|f| {
        info!(":)");
    }).map_err(|e| {
        error!("{:?}", e);
    });

    tokio::spawn(consensus_value_future);
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

#[derive(Debug, Clone)]
enum ValidatorData<T> {
    None,
    Value(T),
    Vector(HashMap<ValidatorIndex, Option<T>>),
    ConsensusValue(T),
}

#[derive(Debug, Clone)]
struct Validator<T> {
    index: ValidatorIndex,
    loyal: bool,
//    node: Neighbor,
    data: ValidatorData<T>,
    result: Option<T>,
}

struct ValidatorManager<T> {
    validators_num: usize,
    validators: Vec<Validator<T>>,
}

pub struct ConsensusValueRetriever {
    input: InputStream,
    output: OutputStream,
//    validators_indices: Vec<ValidatorIndex>,
//    value: T,
}

impl ConsensusValueRetriever {
    pub fn retrieve<T>(mut input: InputStream, mut output: OutputStream, value: T/*, validators: AM<Neighbor>*/) -> impl Future<Item=T, Error=BFTError> { //BFTFuture<T> {
        let send_value = SendValueAndRetrieveVectorFuture { value };

        send_value.and_then(|vector| {
            let send_vector = SendVectorAndRetrieveMatrixFuture { value: vector };
            send_vector.and_then(|matrix| {
                let packet = Box::new(ConsensusValue { value: 2 });
                let data = Pair::<Neighbor, Box<Serializable + Send>>::new(Neighbor::from_address("127.0.0.1".parse::<SocketAddr>().unwrap()), packet);
                input.send_packet(data);
                return BFTFuture::<T> {
                    matrix
                };
            })
        })
    }
}

pub struct BFTFuture<T> {
    matrix: <SendVectorAndRetrieveMatrixFuture<T> as Future>::Item
}

impl<T> Future for BFTFuture<T> {
    type Item = T;
    type Error = BFTError;

    fn poll(&mut self) -> Poll<<Self as Future>::Item, <Self as Future>::Error> {

    }
}

pub struct SendValueAndRetrieveVectorFuture<T> {
    value: T,
}

impl<T> SendValueAndRetrieveVectorFuture<T> {
    fn new(value: T) -> Self {
        SendValueAndRetrieveVectorFuture::<T> {
            value
        }
    }
}

impl<T> Future for SendValueAndRetrieveVectorFuture<T> {
    type Item = HashMap<ValidatorIndex, T>;
    type Error = BFTError;

    fn poll(&mut self) -> Poll<<Self as Future>::Item, <Self as Future>::Error> {

    }
}

pub struct SendVectorAndRetrieveMatrixFuture<T> {
    value: HashMap<ValidatorIndex, T>,
}

impl<T> SendVectorAndRetrieveMatrixFuture<T> {
    fn new(value: T) -> Self {
        SendVectorAndRetrieveMatrixFuture::<T> {
            value
        }
    }
}

impl<T> Future for SendVectorAndRetrieveMatrixFuture<T> {
    type Item = HashMap<ValidatorIndex, HashMap<ValidatorIndex, T>>;
    type Error = BFTError;

    fn poll(&mut self) -> Poll<<Self as Future>::Item, <Self as Future>::Error> {

    }
}

impl<T> Validator<T> where T: Clone {
    fn new(index: u32, loyal: bool) -> Self {
        Validator {
            data: ValidatorData::None,
            result: None,
            index,
            loyal,
        }
    }

    fn set_value(&mut self, value: T) {
        self.data = ValidatorData::Value(value);
    }

    fn send_value(&mut self, validator: &mut Validator<T>) {
        match self.data {
            ValidatorData::Value(ref value) => {
                validator.receive_value(self.index, value.clone());
            }
            _ => {}
            // TODO: other variants
        }
    }

    fn receive_value(&mut self, from: ValidatorIndex, value: T) {
        match self.data {
            ValidatorData::Vector(ref mut vec) => {
                vec.insert(from, Some(value));
                return;
            },
            ValidatorData::Value(_) => {
                // continue
            },
            _ => {
                return;
            }
        }

        match self.data.clone() {
            ValidatorData::Value(v) => {
                let mut vector = HashMap::<ValidatorIndex, _>::new();
                vector.insert(self.index, Some(v));
                vector.insert(from, Some(value));
                self.data = ValidatorData::Vector(vector);
            },
            _ => {}
        }
    }

    fn send_vector(&self, validator: &mut Validator<T>) {
//        if self.result_vec.iter().any(|(_, v)| v.is_none()) {
//            // ret err
//            error!("Couldn't send vec from {}", self.index);
//            return;
//        }
//
//        validator.result_vec_vec.insert(self.index, self.result_vec.iter().map(|(k, v)|
//            (*k, if self.loyal { *v } else { Some(self.index + self.result_vec.capacity() as u32 * 2) } )
//        ).collect());

    }

    fn obtain_value(&mut self) -> Option<SendValueAndRetrieveVectorFuture<T>> {
        if let ValidatorData::Value(v) = self.data {
            return Some(SendValueAndRetrieveVectorFuture::new(v));
        }
        None
    }
}

impl<T> ValidatorManager<T> where T: Clone {
    fn new(validators_num: usize) -> Self {
        ValidatorManager {
            validators_num,
            validators: Vec::new()
        }
    }

    fn process(&mut self) {
        if self.validators.len() != self.validators_num {
            warn!("Validators set isn't consistent")
        }

//        // obtaining value
//        for v in &mut self.validators {
//            v.process_value();
//            v.result_vec.insert(v.index, v.value);
//        }
//
//        // sending value to all validators (obtaining vector)
//        for mut v in self.validators.clone() {
//            for v_to in &mut self.validators {
//                if v.index != v_to.index {
//                    v.send_value(v_to);
//                }
//            }
//        }
//
//        // obtaining vector
//        for v in &mut self.validators {
//            v.result_vec_vec.insert(v.index, v.result_vec.clone());
//        }
//
//        // sending obtained vector to all validators12
//        for mut v in self.validators.clone() {
//            for v_to in &mut self.validators {
//                if v.index != v_to.index {
//                    v.send_vector(v_to);
//                }
//            }
//        }
    }

    fn add_validator(&mut self, index: ValidatorIndex, loyal: bool) -> bool {
        // check for duplicate
        if self.validators.iter().any(|v| v.index == index) {
            return false;
        }

        // 'sending' validator to others
//        for validator in &mut self.validators {
//            validator.result_vec.insert(index, None);
//        }

//        // adding validator to other ones
        let mut validator = Validator::new(index, loyal);
//        for v in &mut self.validators {
//            validator.result_vec.insert(v.index, None);
//        }
        self.validators.push(validator);
//        for h in self.validators.iter() {
//
//        }
        true
    }

    fn bft_check(&mut self) -> Result<Vec<u32>, BFTError> {
//        if self.validators.len() != self.validators_num {
//            return Err(BFTError::InvalidNumberOfValidators);
//        }
//
//        for validator in self.validators.iter() {
//            if validator.value.is_none() {
//                return Err(BFTError::NotAllValidatorsHaveValue);
//            }
//            if validator.result_vec.iter().any(|(_, v)| v.is_none()) {
//                return Err(BFTError::NotAllValidatorsHaveResultVector);
//            }
//        }
//
//        let majority = self.validators_num * 2/3;
//        debug!("majority: {}", majority + 1);
//
//
////        for validator in &mut self.validators {
//        let validator = &mut self.validators[0]; {
//
//            let indices = validator.result_vec.keys().cloned().collect::<Vec<u32>>();
//
//            'v_loop: for i in &indices {
////                if *i == validator.index {
////                    continue;
////                }
//
//                let mut results = HashMap::<u32, u32>::new(); // <value, score>
//
//                for j in &indices {
//                    if *i != *j {
//                        match validator.result_vec_vec.get(j) {
//                            Some(vec) => {
//                                match vec.get(i) {
//                                    Some(Some(v)) => {
//                                        if let Some(score) = results.get(v).cloned() {
//                                            let new_score = score + 1;
//                                            debug!("score {}:{}", v, new_score);
//                                            if new_score > majority as u32 {
//                                                info!("consensus {} {}:{}", validator.index, i, v);
//                                                validator.consensus_vec.insert(*i, Some(*v));
//                                                results.clear();
//                                                continue 'v_loop;
//                                            }
//                                            results.insert(*v, new_score);
//                                        } else {
//                                            results.insert(*v, 1);
//                                        }
//                                    },
//                                    Some(&None) => {}
//                                    None => {}
//                                }
//                            },
//                            None => {}
//                        }
//                    }
//                }
//
//                let mut max_score = 0;
//                let mut final_result = None;
//                let mut consensus_failed = false;
//
//                for (result, score) in &results {
//                    let score = *score;
//                    if score > max_score {
//                        max_score = score;
//                        final_result = Some(*result);
//                    }
//                }
//
//
//                // TODO: check for double max_score
//
//
//                validator.consensus_vec.insert(*i, final_result);
//            }
////            for (vec_from_v_index, vec_from_v) in &validator.result_vec_vec {
////                 if self
////                if *vec_from_v_index == validator.index {
////                    validator.consensus_vec.insert(validator.index, )
////                    continue;
////                }
////                results.insert()
////            }
//        }
        Ok(vec![])
    }
}

impl<T> Debug for ValidatorManager<T> where T: Debug {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "validators: {}/{}\n", self.validators.len(), self.validators_num);
        for v in &self.validators {
            write!(f, "  [{}]: {:?}\n", v.index, v.data);
        }
        Ok(())
    }
}

fn init() {
    let mut vm = ValidatorManager::<ValidatorDataType>::new(N);
    for i in 0..N {
        vm.add_validator(i as u32, i >= M);
    }

    vm.process();
    info!("{:?}", vm);
    info!("{:?}", vm.bft_check());
    info!("{:?}", vm);
}

fn main() {
    let format = |record: &LogRecord| {
        format!("[{}]: {}", record.level(), record.args())
//        format!("[{} {:?}]: {}", record.level(), thread::current().id(), record.args())
    };

    let mut builder = LogBuilder::new();
    builder.format(format)
        .filter(None, LogLevelFilter::Info)
        .filter(Some("futures"), LogLevelFilter::Error)
        .filter(Some("tokio"), LogLevelFilter::Error)
        .filter(Some("tokio-io"), LogLevelFilter::Error)
        .filter(Some("hyper"), LogLevelFilter::Error)
        .filter(Some("iron"), LogLevelFilter::Error);

    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init().unwrap();

    thread::sleep(Duration::from_secs(1));
    init();
}

