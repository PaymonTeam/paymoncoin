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
use network::node::{OutputStream, InputStream, PacketData, Pair};
use network::rpc::ConsensusValue;
use network::packet::Serializable;
use network::Neighbor;
use utils::AM;
use std::net::SocketAddr;
use std::hash::Hash;
use std::marker::PhantomData;

#[test]
pub fn pos_test() {
    let mut input = InputStream { queue: make_am!(vec![]) };
    let mut output = OutputStream { queue: make_am!(vec![]) };
//    Neighbor::from_address("")
//    let neighbors = make_am!(vec![
//
//    ]);

    let validators = make_am!(Vec::new());
//    let mut r = ConsensusValueRetriever {
//        input, output,
//        _pd: PhantomData {},
//    };
//    let (primed_tx, primed_rx) = oneshot();
    let consensus_value_future = ConsensusValueRetriever::retrieve::<i32>(input, output, 1i32, validators).map(|f| {
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

//#[derive(Debug, Clone)]
//enum ValidatorData<T> {
//    None,
//    Value(T),
//    Vector(HashMap<ValidatorIndex, T>),
//    ConsensusValue(T),
//}

#[derive(Debug)]
pub struct Validator {
    index: ValidatorIndex,
    loyal: bool,
    node: Neighbor,
//    data: ValidatorData<T>,
//    result: T,
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
//struct ValidatorManager<T> {
//    validators_num: usize,
//    validators: Vec<Validator<T>>,
//}

pub struct ConsensusValueRetriever {
//    input: InputStream
//    output: OutputStream<'a>,
//    _pd: PhantomData<&'b T>,
//    validators_indices: Vec<ValidatorIndex>,
//    value: T,
}

impl ConsensusValueRetriever {
     pub fn retrieve<'a, T>(mut input: InputStream<'a>, mut output: OutputStream<'a>, value: T, validators: AM<Vec<Validator>>) -> impl Future<Item=T, Error=BFTError> + 'a where
         T: Eq + Hash + Serializable + Send + Clone + Copy + 'a { //BFTFuture<T> {
//        let mut send_value = SendValueAndRetrieveVectorFuture::new(value, input, output, validators);
//
//        send_value.and_then(|vector| {
//            let send_vector = SendVectorAndRetrieveMatrixFuture::new(vector, input, output, validators);
//            send_vector.and_then(|matrix| {
//                let packet = Box::new(ConsensusValue { value: 2 });
//                let data = Pair::<Validator, Box<Serializable + Send>>::new(Validator::from_index(0u32, true), packet);
//                input.send_packet(data);
//                return BFTFuture::<T> {
//                    matrix,
//                    input,
//                    output,
//                    validators,
//                };
//            })
//        })


//        let input = input.clone() as InputStream<'b>;
//        let output = output.clone() as OutputStream<'b>;
//        return BFTFuture::<'b, U> {
//            matrix: HashMap::new(),
//            input,
//            output,
//            validators,
//        };
//         let mut send_value = SendValueAndRetrieveVectorFuture::<'b, U>::new(value, input, output, validators);
//         send_value
         S {
             value,
             _pd: PhantomData {}
         }
    }
}

pub struct S<'a, T> where T: Eq + Serializable + 'a {
    value: T,
    _pd: PhantomData<&'a T>
}

impl<'a, T> Future for S<'a, T> where T: Eq + Serializable + 'a {
    type Item = T;
    type Error = BFTError;

    fn poll(&mut self) -> Poll<<Self as Future>::Item, <Self as Future>::Error> {
        Ok(Async::Ready(self.value))
    }
}


pub struct SendValueAndRetrieveVectorFuture<'a, T> where T: Eq + Hash + Serializable + Send + Clone + Copy + 'a {
    value: T,
    vector: HashMap<ValidatorIndex, T>,
    input: InputStream<'a>,
    output: OutputStream<'a>,
    validators: AM<Vec<Validator>>,
}

impl<'a, T> SendValueAndRetrieveVectorFuture<'a, T> where T: Eq + Hash + Serializable + Send + Clone + Copy + 'a {
    fn new(value: T, input: InputStream<'a>, output: OutputStream<'a>, validators: AM<Vec<Validator>>) -> Self {
        SendValueAndRetrieveVectorFuture::<T> {
            value, input, output, validators,
            vector: HashMap::new()
        }
    }
}

impl<'a, T> Future for SendValueAndRetrieveVectorFuture<'a, T> where T: Eq + Hash + Serializable + Send + Clone + Copy + 'a {
    type Item = HashMap<ValidatorIndex, T>;
    type Error = BFTError;

    fn poll(&mut self) -> Poll<<Self as Future>::Item, <Self as Future>::Error> {
        let validators = self.validators.lock().unwrap();
        let c = self.value.clone();
//        self.output.send_packet(Pair::<_, Box<dyn Serializable + Send>>::new(validators[0], Box::new(c)));
        self.output.send_packet2(Arc::new(c.clone()));
        // TODO: create timeout
//        self.input.for_each(|p| {
//            let value = p.hi as T;
//            self.vector.insert(p.low.index, value);
//        });
        Ok(Async::Ready(self.vector))
    }
}

pub struct BFTFuture<'a, T> {
    matrix: HashMap<ValidatorIndex, HashMap<ValidatorIndex, T>>, //<SendVectorAndRetrieveMatrixFuture<T> as Future>::Item,
    input: InputStream<'a>,
    output: OutputStream<'a>,
    validators: AM<Vec<Validator>>,
}

impl<'a, T> Future for BFTFuture<'a, T> where T: Eq + Hash + Serializable {
    type Item = T;
    type Error = BFTError;

    fn poll(&mut self) -> Poll<<Self as Future>::Item, <Self as Future>::Error> {
        let validators_num = self.matrix.len();
//        if self.validators.len() != self.validators_num {
//            return Err(BFTError::InvalidNumberOfValidators);
//        }

//        for validator_vec in self.validators.iter() {
//            if validator_vec.value.is_none() {
//                return Err(BFTError::NotAllValidatorsHaveValue);
//            }
//            if validator_vec.result_vec.iter().any(|(_, v)| v.is_none()) {
//                return Err(BFTError::NotAllValidatorsHaveResultVector);
//            }
//        }

        let majority = validators_num * 2/3;
        debug!("majority: {}", majority + 1);

        let validator_vec = &mut self.matrix.get(self.matrix.keys().nth(0).unwrap()).unwrap(); {

            let indices = validator_vec.keys().cloned().collect::<Vec<u32>>();

            'v_loop: for i in &indices {
//                if *i == validator_vec.index {
//                    continue;
//                }

                let mut results = HashMap::<T, u32>::new(); // <value, score>

                for j in &indices {
                    if *i != *j {
                        let vec = self.matrix.get(j).unwrap_or(&HashMap::new());
                        match vec.get(i) {
                            Some(v) => {
                                if let Some(score) = results.get(v).cloned() {
                                    let new_score = score + 1;
//                                    debug!("score {}:{}", v, new_score);
                                    if new_score > majority as u32 {
                                        info!("consensus {}:{}", i, j);
//                                        info!("consensus {} {}:{}", validator_vec.index, i, v);
//                                        validator_vec.consensus_vec.insert(*i, Some(*v));
                                        results.clear();
                                        continue 'v_loop;
                                    }
                                    results.insert(*v, new_score);
                                } else {
                                    results.insert(*v, 1);
                                }
                            },
                            None => {}
                        }
                    }
                }

                let mut max_score = 0;
                let mut final_result = None;
                let mut consensus_failed = false;

                for (result, score) in &results {
                    let score = *score;
                    if score > max_score {
                        max_score = score;
                        final_result = Some(*result);
                    }
                }


                // TODO: check for double max_score
//                validator_vec.consensus_vec.insert(*i, final_result);
                if let Some(result) = final_result {
                    return Ok(Async::Ready(result));
                } else {
                    return Err(BFTError::ConsensusWasNotAchieved);
                }
            }
//            for (vec_from_v_index, vec_from_v) in &validator_vec.result_vec_vec {
//                 if self
//                if *vec_from_v_index == validator_vec.index {
//                    validator_vec.consensus_vec.insert(validator_vec.index, )
//                    continue;
//                }
//                results.insert()
//            }
        }
        Err(BFTError::ConsensusWasNotAchieved)
    }
}

pub struct SendVectorAndRetrieveMatrixFuture<'a, T> {
    value: HashMap<ValidatorIndex, T>,
    input: InputStream<'a>,
    output: OutputStream<'a>,
    validators: AM<Vec<Validator>>,
}

impl<'a, T> SendVectorAndRetrieveMatrixFuture<'a, T> {
    fn new(value: HashMap<ValidatorIndex, T>, input: InputStream<'a>, output: OutputStream<'a>, validators: AM<Vec<Validator>>) -> Self {
        SendVectorAndRetrieveMatrixFuture::<T> {
            value, input, output, validators
        }
    }
}

impl<'a, T> Future for SendVectorAndRetrieveMatrixFuture<'a, T> where T: Eq + Hash + Serializable + 'a {
    type Item = HashMap<ValidatorIndex, HashMap<ValidatorIndex, T>>;
    type Error = BFTError;

    fn poll(&mut self) -> Poll<<Self as Future>::Item, <Self as Future>::Error> {

    }
}