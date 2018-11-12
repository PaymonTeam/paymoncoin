extern crate rhododendron;

use env_logger::LogBuilder;
use log::{LogRecord, LogLevelFilter};
use std::{
    env,
    collections::HashMap,
    thread,
    fmt,
    time::Duration,
    sync::{Arc, Mutex, atomic::{Ordering, AtomicUsize} }
};
use futures;
use futures::oneshot;
use futures::{
    future::FutureResult,
    AndThen
};
use futures::executor::{Run, Spawn};
use futures::prelude::*;
use futures::task::Task;
use tokio;
//use network::node::{OutputStream, InputStream, PacketData, Pair};
use network::node::{PacketData, Pair};
use network::rpc::ConsensusValue;
use network::packet::{self, get_serialized_object, Serializable};
use network::Neighbor;
use utils::AM;
use std::net::SocketAddr;
use std::collections::BTreeSet;
use std::marker::PhantomData;
use self::rhododendron as bft;
use tokio_timer::{self, Timer};
//use secp256k1;
use crypto;
type ValidatorIndex = usize;
type ValidatorDataType = u32;
const N: usize = 4;
const M: usize = 2;

const ROUND_DURATION: Duration = Duration::from_millis(50);

//trait Hash {
//    fn calculate_hash(&self) -> &[u8];
//}

pub mod secp256k1 {
    pub struct SecretKey {

    }
    pub struct PublicKey {

    }

    #[derive(Debug, Clone)]
    pub struct Signature {

    }
    pub struct Message {

    }

    impl Message {
        pub fn from_slice(slice: &[u8]) -> Self {
            Message {}
        }
    }

    pub fn sign(data: &Message, sk: &SecretKey) -> Signature {
        Signature {}
    }
}

#[derive(Debug)]
struct Error;

impl From<bft::InputStreamConcluded> for Error {
    fn from(_: bft::InputStreamConcluded) -> Error {
        Error
    }
}

trait AsBytes {
    fn as_bytes(&self) -> &[u8];
}

trait Signature {
    type Hash: AsBytes;
    type Signature;
    type Message;
    type SecretKey;
    type PublicKey;

    fn calculate_hash(bytes: &[u8]) -> Self::Hash;
    fn calculate_signature(data: &[u8], sk: &Self::SecretKey) -> Self::Signature;
    fn get_index(&self) -> usize;
}

struct Secp256k1Signature {

}

//impl<'a> Signature for Secp256k1Signature {
//    type Hash = &'a [u8];
//    type Signature = secp256k1::Signature;
//    type Message = secp256k1::Message;
//    type SecretKey = secp256k1::SecretKey;
//    type PublicKey = secp256k1::PublicKey;
//
//    fn calculate_hash(bytes: &[u8]) -> <Self as Signature>::Hash {
//        use crypto::digest::Digest;
//        use crypto::sha3;
//        let mut sha = sha3::Sha3::sha3_256();
//        let mut buf = [0u8; 32];
//        sha.input(bytes);
//        sha.result(&mut buf);
//        &mut buf
//    }
//
//    fn calculate_signature(data: &'_ [u8], sk: &'_ <Self as Signature>::SecretKey) -> <Self as Signature>::Signature {
//        unimplemented!()
//    }
//
//    fn get_index(&self) -> usize {
//        unimplemented!()
//    }
//}
//use self::secp256k1::*;
//
//let data = match message {
//bft::Message::Propose(_, m) => {
//use network::packet::get_serialized_object;
//Message::from_slice(&get_serialized_object(&m, true)).expect("32 bytes")
//},
//bft::Message::Vote(bft::Vote::Commit(_, digest)) => {
//Message::from_slice(digest.as_bytes()).expect("32 bytes")
//}
//};
//
//
//sign(&data, secret_key)


struct Context<T, S>
    where S: Signature,
{
    local_id: ValidatorIndex,
    proposal: Mutex<Option<T>>,
    node_count: usize,
    current_round: Arc<AtomicUsize>,
    timer: Timer,
    evaluated: Mutex<BTreeSet<T>>,
    secret_key: S::SecretKey,
}

impl<T, S> bft::Context for Context<T, S>
    where T: fmt::Debug + Eq + Clone + Ord + Serializable + crypto::digest::Digest,
          S: Signature,
          S::Signature: fmt::Debug + Eq + Clone,
          S::Hash: ::std::hash::Hash + fmt::Debug + Eq + Clone + crypto::digest::Digest,
{
    type Error = Error;
    type Candidate = T;
    type Digest = S::Hash;
    type AuthorityId = ValidatorIndex;
    type Signature = S::Signature;
    type RoundTimeout = Box<Future<Item=(), Error=Self::Error>>;
    type CreateProposal = FutureResult<Self::Candidate, Error>;
    type EvaluateProposal = FutureResult<bool, Error>;

    fn local_id(&self) -> Self::AuthorityId {
        self.local_id.clone()
    }

    fn proposal(&self) -> Self::CreateProposal {
        let mut proposal = self.proposal.lock().unwrap().take().unwrap();

        Ok(proposal).into_future()
    }

    fn candidate_digest(&self, candidate: &Self::Candidate) -> Self::Digest {
        S::calculate_hash( &get_serialized_object(candidate, true))
    }

    fn sign_local(&self, message: bft::Message<Self::Candidate, Self::Digest>)
                  -> bft::LocalizedMessage<Self::Candidate, Self::Digest, Self::AuthorityId, Self::Signature>
    {
        let data = match message {
            bft::Message::Propose(_, ref m) => {
                get_serialized_object(m, true)
            },
            bft::Message::Vote(bft::Vote::Commit(_, ref digest)) |
            bft::Message::Vote(bft::Vote::Prepare(_, ref digest)) => {
                packet::SerializedBuffer::from_slice(digest.as_bytes())
            },
            bft::Message::Vote(bft::Vote::AdvanceRound(round)) => {
                let mut buffer = packet::SerializedBuffer::new_with_size(::std::mem::size_of_val(&round));
                buffer.write_u32(round);
                buffer
            }
        };

        let signature = S::calculate_signature(&data, &self.secret_key);

        match message {
            bft::Message::Propose(r, proposal) => bft::LocalizedMessage::Propose(bft::LocalizedProposal {
                round_number: r,
                digest: S::calculate_hash(&get_serialized_object(&proposal, true)),
                proposal,
                digest_signature: signature.clone(),
                full_signature: signature,
                sender: self.local_id.clone(),
            }),
            bft::Message::Vote(vote) => bft::LocalizedMessage::Vote(bft::LocalizedVote {
                vote,
                signature,
                sender: self.local_id.clone(),
            })
        }
    }

    fn round_proposer(&self, round: u32) -> Self::AuthorityId {
//        AuthorityId((round as usize) % self.node_count)
        (round as usize) % self.node_count
    }

    fn proposal_valid(&self, proposal: &Self::Candidate) -> FutureResult<bool, Error> {
//        self.evaluated.lock().unwrap().insert(proposal.0);
//
//        Ok(proposal.get_index() % 3 != 0).into_future()
        self.evaluated.lock().unwrap().insert(proposal.clone());

        Ok(true).into_future()
    }

    fn begin_round_timeout(&self, round: u32) -> Self::RoundTimeout {
        if (round as usize) < self.current_round.load(Ordering::SeqCst) {
            Box::new(Ok(()).into_future())
        } else {
            let mut round_duration = ROUND_DURATION;
            for _ in 0..round {
                round_duration *= 2;
            }

            let current_round = self.current_round.clone();
            let timeout = self.timer.sleep(round_duration)
                .map(move |_| {
                    current_round.compare_and_swap(round as usize, round as usize + 1, Ordering::SeqCst);
                })
                .map_err(|_| Error);

            Box::new(timeout)
        }
    }

    fn on_advance_round(
        &self,
        _acc: &bft::Accumulator<Self::Candidate, Self::Digest, Self::AuthorityId, Self::Signature>,
        _round: u32,
        _next_round: u32,
        _reason: bft::AdvanceRoundReason,
    ) {
    }
}

//fn calculate_signature_sk128<T:Consensus>(message: &bft::Message<T, T::Hash>, secret_key: &T::Signature) -> T::Signature {
//}

#[test]
pub fn pos_test() {
    let nodes_count = 4;
//    agree(context, 4, 1, )
}

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