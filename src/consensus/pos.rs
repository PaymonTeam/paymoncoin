extern crate rhododendron;

use std::{
    collections::HashMap,
    env,
    fmt,
    sync::{Arc, atomic::{AtomicUsize, Ordering}, Mutex},
    thread,
    time::Duration,
    collections::BTreeSet,
    marker::PhantomData,
    net::SocketAddr,
};

use crypto;
use env_logger::LogBuilder;
use futures::{
    self,
    AndThen,
    executor::{Run, Spawn},
    future::FutureResult,
    oneshot,
    prelude::*,
    sync::{mpsc, oneshot},
    task::Task,
};
use log::{LogLevelFilter, LogRecord};
use secp256k1;
use tokio;
use tokio_timer::{self, Timer};

use network::{
    Neighbor,
    node::{PacketData, Pair},
    packet::{self, get_serialized_object, Serializable},
    rpc::{self, ConsensusValue},
//use node::{OutputStream, InputStream, PacketData, Pair};
};
use utils::AM;

use self::rhododendron as bft;

type ValidatorIndex = usize;
type ValidatorDataType = u32;
const N: usize = 4;
const M: usize = 2;

const ROUND_DURATION: Duration = Duration::from_millis(50);

struct Network<T> {
    endpoints: Vec<mpsc::UnboundedSender<T>>,
    input: mpsc::UnboundedReceiver<(usize, T)>,
}

impl<T: Clone + Send + 'static> Network<T> {
    fn new(nodes: usize)
           -> (Self, Vec<mpsc::UnboundedSender<(usize, T)>>, Vec<mpsc::UnboundedReceiver<T>>)
    {
        let mut inputs = Vec::with_capacity(nodes);
        let mut outputs = Vec::with_capacity(nodes);
        let mut endpoints = Vec::with_capacity(nodes);

        let (in_tx, in_rx) = mpsc::unbounded();
        for _ in 0..nodes {
            let (out_tx, out_rx) = mpsc::unbounded();
            inputs.push(in_tx.clone());
            outputs.push(out_rx);
            endpoints.push(out_tx);
        }

        let network = Network {
            endpoints,
            input: in_rx,
        };

        (network, inputs, outputs)
    }

    fn route_on_thread(self) {
        ::std::thread::spawn(move || { let _ = self.wait(); });
    }
}

impl<T: Clone> Future for Network<T> {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), Self::Error> {
        match try_ready!(self.input.poll()) {
            None => Ok(Async::Ready(())),
            Some((sender, item)) => {
                {
                    let receiving_endpoints = self.endpoints
                        .iter()
                        .enumerate()
                        .filter(|&(i, _)| i != sender)
                        .map(|(_, x)| x);

                    for endpoint in receiving_endpoints {
                        let _ = endpoint.unbounded_send(item.clone());
                    }
                }

                self.poll()
            }
        }
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

trait SignatureScheme {
    type Hash: AsRef<[u8]>;
    type Signature;
    type Message;
    type SecretKey;
    type PublicKey;

    fn calculate_hash(bytes: &[u8]) -> Self::Hash;
    fn calculate_signature(&self, data: &[u8]) -> Self::Signature;
    fn get_index(&self) -> usize;
}

struct Secp256k1Signature<'a> {
    _phantom: PhantomData<&'a ()>,
    context: secp256k1::Secp256k1,//secp256k1::Secp256k1<secp256k1::All>,
    secret_key: secp256k1::key::SecretKey,
}

impl<'a> SignatureScheme for Secp256k1Signature<'a> {
    type Hash = [u8; 32];
    type Signature = secp256k1::Signature;
    type Message = secp256k1::Message;
    type SecretKey = secp256k1::key::SecretKey;
    type PublicKey = secp256k1::key::PublicKey;

    fn calculate_hash(bytes: &[u8]) -> <Self as SignatureScheme>::Hash {
        use crypto::digest::Digest;
        use crypto::sha3;
        let mut sha = sha3::Sha3::sha3_256();
        let mut buf = [0u8; 32];
        sha.input(bytes);
        sha.result(&mut buf);
        buf
    }

    fn calculate_signature(&self, data: &[u8]) -> <Self as SignatureScheme>::Signature {
        let hash = Self::calculate_hash(data);
        let msg = secp256k1::Message::from_slice(&hash).expect("32 bytes");
        self.context.sign(&msg, &self.secret_key).unwrap()
    }

    fn get_index(&self) -> usize {
        unimplemented!()
    }
}

impl<'a> Secp256k1Signature<'a> {
    fn new(sk: <Self as SignatureScheme>::SecretKey) -> Self {
        use rand::{Rng, thread_rng};
        let k1 = secp256k1::Secp256k1::new();

        Secp256k1Signature {
            context: secp256k1::Secp256k1::new(),
            _phantom: PhantomData {},
            secret_key: sk,
        }
    }
}

struct Context<T, S>
{
    local_id: ValidatorIndex,
    proposal: Mutex<Option<T>>,
    node_count: usize,
    current_round: Arc<AtomicUsize>,
    timer: Timer,
    evaluated: Mutex<BTreeSet<T>>,
    signature: S,
}

impl<T, S> Context<T, S> where S: SignatureScheme,
                               T: fmt::Debug + Eq + Clone + Ord + Serializable,
                               S::Signature: fmt::Debug + Eq + Clone,
                               S::Hash: ::std::hash::Hash + fmt::Debug + Eq + Clone {
    pub fn new(proposal: T, signature: S, local_id: <Self as rhododendron::Context>::AuthorityId, node_count: usize) -> Self where T: Ord {
        Context {
            signature,
            local_id,
            proposal: Mutex::new(Some(proposal)),
            current_round: Arc::new(AtomicUsize::new(0)),
            timer: tokio_timer::wheel().tick_duration(ROUND_DURATION).build(),
            evaluated: Mutex::new(BTreeSet::new()),
            node_count,
        }
    }
}

impl<T, S> bft::Context for Context<T, S>
    where T: fmt::Debug + Eq + Clone + Ord + Serializable,
          S: SignatureScheme,
          S::Signature: fmt::Debug + Eq + Clone,
          S::Hash: ::std::hash::Hash + fmt::Debug + Eq + Clone,
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
                packet::SerializedBuffer::from_slice(digest.as_ref())
            },
            bft::Message::Vote(bft::Vote::AdvanceRound(round)) => {
                let mut buffer = packet::SerializedBuffer::new_with_size(::std::mem::size_of_val(&round));
                buffer.write_u32(round);
                buffer
            }
        };

        let signature = self.signature.calculate_signature(&data);

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

fn init_log() {
    use env_logger::LogBuilder;
    use log::{LogRecord, LogLevelFilter};
    use env;

    let format = |record: &LogRecord| {
        format!("[{}]: {}", record.level(), record.args())
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
}

#[test]
pub fn pos_test() {

    use secp256k1::*;
    use rand::thread_rng;

    let nodes_count = 4;
    let sk = Secp256k1::new().generate_keypair(&mut thread_rng()).unwrap().0;
//    let context = Context::new(rpc::ConsensusValue{value: 1}, Secp256k1Signature::new(sk));
}

#[test]
fn consensus_completes_with_minimum_good() {
    use secp256k1::*;
    use rand::thread_rng;

    init_log();

    let node_count = 10;
    let max_faulty = 3;

    let timer = tokio_timer::wheel().tick_duration(ROUND_DURATION).build();

    let (network, net_send, net_recv) = Network::new(node_count);
    network.route_on_thread();

    let nodes = net_send
        .into_iter()
        .zip(net_recv)
        .take(node_count - max_faulty)
        .enumerate()
        .map(|(i, (tx, rx))| {
//            let ctx = Context {
//                local_id: AuthorityId(i),
//                proposal: Mutex::new(i),
//                current_round: Arc::new(AtomicUsize::new(0)),
//                timer: timer.clone(),
//                evaluated: Mutex::new(BTreeSet::new()),
//                node_count,
//            };
            let sk = Secp256k1::new().generate_keypair(&mut thread_rng()).unwrap().0;
//            let ctx = Context::new(rpc::ConsensusValue{ value: i as u32 }, Secp256k1Signature::new(sk), i, node_count);
            let ctx = Context {
                signature: Secp256k1Signature::new(sk),
                local_id: i,
                proposal: Mutex::new(Some(rpc::ConsensusValue{ value: i as u32 })),
                current_round: Arc::new(AtomicUsize::new(0)),
                timer: timer.clone(),
                evaluated: Mutex::new(BTreeSet::new()),
                node_count,
            };
            rhododendron::agree(
                ctx,
                node_count,
                max_faulty,
                rx.map_err(|_| Error),
                tx.sink_map_err(|_| Error).with(move |t| Ok((i, t))),
            )
        })
        .collect::<Vec<_>>();

    let timeout = timeout_in(Duration::from_millis(500)).map_err(|_| Error);
    let results = ::futures::future::join_all(nodes)
        .map(Some)
        .select(timeout.map(|_| None))
        .wait()
        .map(|(i, _)| i)
        .map_err(|(e, _)| e)
        .expect("to complete")
        .expect("to not time out");

    for result in &results {
        assert_eq!(&result.justification.digest, &results[0].justification.digest);
    }
}

fn timeout_in(t: Duration) -> oneshot::Receiver<()> {
    let (tx, rx) = oneshot::channel();
    ::std::thread::spawn(move || {
        ::std::thread::sleep(t);
        let _ = tx.send(());
    });

    rx
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

