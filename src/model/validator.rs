use std::collections::HashMap;
use std::collections::HashSet;
use std::iter::Iterator;
use model::transaction::*;
use network::packet::{Serializable, SerializedBuffer};
use storage::hive::Hive;
use std::collections::LinkedList;
use rand::Rng;
use std::i64::MAX;
use std::collections::hash_map::Entry;

extern crate rand;

#[derive(Hash, Eq, PartialEq, Debug)]
struct Pair(Hash, i64);

pub struct Validator {}

pub struct MonteCarlo {
    // TODO: использовать utils::AM
    hive: Hive,
}

impl MonteCarlo {
    pub fn new() -> Self {
        let hive = Hive::new();
        MonteCarlo {
            hive,
        }
    }

    // TODO: сделать по-другому
//    pub fn set_hive(&mut self, hive_: &Hive) {
//        self.hive = *hive_.clone();
//    }

    pub fn random_walk(&self,
                       visited_hashes: &HashSet<Hash>,
                       diff: &HashMap<Hash, i64>,
                       start: &Hash,
                       extra_tip: &Hash,
                       ratings: &mut HashMap<Hash, i64>,
                       max_depth: &u32,
                       max_depth_ok: &HashSet<Hash>) -> Hash {
        let mut rnd = rand::thread_rng(); // f32 randomer
        let mut tip = start.clone();
        let mut tail = tip.clone();
        let mut tips: Vec<Hash>;
        let mut tip_set: HashSet<Hash>;
        let mut analyzed_tips: HashSet<Hash> = HashSet::new();
        let mut traversed_tails = 0;
        let mut transaction_obj = Transaction::new();
        let mut approver_index: usize;
        let mut rating_weight: f32;
        let mut walk_ratings: Vec<f32>;
        let mut my_diff = diff.clone();
        let mut my_approved_hashes = visited_hashes.clone();

        while !tip.is_null() {
            transaction_obj = Transaction::from_hash(tip.clone());
            tip_set = transaction_obj.get_approvers(&self.hive).clone();

            if transaction_obj.get_current_index() == 0 {
                /*if transactionObj.getType() == transactionObj.PREFILLED_SLOT {
                    break;
                } else if !transactionValidator.checkSolidity(transactionViewModel.getHash(), false) {
                    break;
                } else if belowMaxDepth(transactionViewModel.getHash(), maxDepth, maxDepthOk) {
                    break;
                } else if !ledgerValidator.updateDiff(myApprovedHashes, myDiff, transactionViewModel.getHash()) {
                    break;
                }  else*/ if transaction_obj.calculate_hash() == *extra_tip {
                    break;
                }
            }

            tail = tip.clone();
            traversed_tails += 1;

            if tip_set.capacity() == 0 {
                break;
            } else if tip_set.capacity() == 1 {
                let mut hash_iterator = tip_set.iter();

                match hash_iterator.next() {
                    // FIXME: нельзя просто делать unwrap, иначе это может вызвать ошибку
                    Some(hash) => tip = tip_set.get(&hash).unwrap().clone(),
                    None => tip = HASH_NULL
                }
            } else {
                // walk to the next approver
                tips = MonteCarlo::set_to_vec(&tip_set);
                if !ratings.contains_key(&tip) {
                    MonteCarlo::serial_update_ratings(&self,
                                                      &my_approved_hashes,
                                                      &tip,
                                                      ratings,
                                                      &mut analyzed_tips,
                                                      &extra_tip);
                    analyzed_tips.clear();
                }

                walk_ratings = Vec::with_capacity(tips.capacity());
                let mut max_rating: f32 = 0f32;
                let mut tip_rating: i64 = match ratings.get(&tip) {
                    Some(x) => *x,
                    None => break
                };
                for i in 0..tips.capacity() {
                    walk_ratings[i] = ((tip_rating - MonteCarlo::get_or_default(ratings, &tips[i], 0i64)) as f32).powf(-3 as f32);
                    max_rating += walk_ratings[i];
                }

                rating_weight = rnd.gen::<f32>() * max_rating;
                approver_index = tips.capacity();
                for i in tips.capacity()..0 {
                    approver_index = i;
                    rating_weight -= walk_ratings[approver_index];
                    if rating_weight <= 0 as f32 {
                        break;
                    }
                }
                tip = tips[approver_index as usize].clone();
                if transaction_obj.calculate_hash() == tip {
                    break;
                }
            }
        }
        return tail;
    }

    pub fn markov_chain_monte_carlo(&self,
                                    visited_hashes: &HashSet<Hash>,
                                    diff: &HashMap<Hash, i64>,
                                    tip: &Hash,
                                    extra_tip: &Hash,
                                    ratings: &mut HashMap<Hash, i64>,
                                    iterations: &u32,
                                    max_depth: &u32,
                                    max_depth_ok: &HashSet<Hash>,
                                    /*Random seed*/) -> Hash {
        let mut rnd = rand::thread_rng();
        let mut monte_carlo_integrations: &mut HashMap<Hash, i64> = &mut HashMap::new();
        let mut map_clone = monte_carlo_integrations.clone();
        let mut tail: Hash;
        for i in *iterations..0 {
            tail = self.random_walk(visited_hashes, diff, tip, extra_tip, ratings, max_depth, max_depth_ok);
            if monte_carlo_integrations.contains_key(&tail) {
                // TODO: unwrap
                MonteCarlo::put(monte_carlo_integrations, &tail, &(*(&map_clone.get(&tail).unwrap()) + 1));
            } else {
                MonteCarlo::put(monte_carlo_integrations, &tail, &1);
            }
        }

        let res_set = monte_carlo_integrations.iter()
            .map(|(x, y)| Pair(*x, *y))
            .collect::<HashSet<_>>();

        return res_set.iter()
            .fold(HASH_NULL, |a, b| {
                if *monte_carlo_integrations.get(&a).unwrap() > b.1 {
                    return a;
                } else if *monte_carlo_integrations.get(&a).unwrap() < b.1 {
                    return b.0;
                } else if rnd.gen() {
                    return a;
                } else {
                    return b.0;
                }
            });
    }

    fn set_to_vec(set: &HashSet<Hash>) -> Vec<Hash> {
        let mut hash_iterator = set.iter();
        let mut result: Vec<Hash> = Vec::new();
        if !set.is_empty() {
            loop {
                match hash_iterator.next() {
                    Some(hash) => result.push(*set.get(hash).unwrap()),
                    None => break
                }
            }
        }
        return result;
    }

    fn serial_update_ratings(&self,
                             visited_hashes: &HashSet<Hash>,
                             tx_hash: &Hash,
                             ratings: &mut HashMap<Hash, i64>,
                             analyzed_tips: &mut HashSet<Hash>,
                             extra_tip: &Hash) {
        let mut hashes_to_rate: LinkedList<Hash> = LinkedList::new();
        hashes_to_rate.push_front(*tx_hash);
        let mut current_hash: Hash;
        let mut added_back: bool;
        while !hashes_to_rate.is_empty() {
            match hashes_to_rate.pop_front() {
                Some(hash) => current_hash = hash,
                None => {
                    println!("Stack is empty!");
                    return;
                }
            }
            let mut transaction: Transaction = Transaction::from_hash(current_hash);
            added_back = false;
            let mut approvers: HashSet<Hash> = transaction.get_approvers(&self.hive).clone();
            for approver in &approvers {
                let mut flag: bool = match ratings.get(approver) {
                    Some(..) => true,
                    None => false
                };
                if flag && *approver != current_hash {
                    if !added_back {
                        added_back = true;
                        hashes_to_rate.push_front(current_hash);
                    }
                    hashes_to_rate.push_front(*approver);
                }
            }
            if !added_back && MonteCarlo::add(analyzed_tips, &current_hash) {
                let rating: i64 = MonteCarlo::rating_calc(&extra_tip, &visited_hashes, &current_hash, &approvers, ratings);
                MonteCarlo::put(ratings, &current_hash, &rating);
            }
        }
    }

    fn add(set: &mut HashSet<Hash>, curr: &Hash) -> bool {
        match set.get(curr) {
            Some(..) => {
                return false;
            }
            None => {
                set.insert(*curr);
                return true;
            }
        }
    }

    fn rating_calc(extra_tip: &Hash, visited_hashes: &HashSet<Hash>, current_hash: &Hash, approvers: &HashSet<Hash>, ratings: &HashMap<Hash, i64>) -> i64 {
        let mut result: i64;
        result = match *extra_tip == HASH_NULL && visited_hashes.contains(current_hash) {
            true => 0,
            false => 1
        };

        result += approvers.iter().
            map(|x| ratings.get(x)).
            filter(|x| *x != None).
            fold(0, |a, b| cap_sum(&a, &b.unwrap(), &(<i64>::max_value() / 2)));
        return result;
    }

    fn put(map: &mut HashMap<Hash, i64>, key: &Hash, value: &i64) -> Option<i64> {
        let result: i64;
        match map.contains_key(&key) {
            true => {
                // TODO: unwrap
                result = *map.get(&key).unwrap();
                map.insert(*key, *value);
                return Some(result);
            }
            false => {
                map.insert(*key, *value);
                return None;
            }
        };
    }

    fn get_or_default(map: &HashMap<Hash, i64>, key: &Hash, default_value: i64) -> i64 {
        let result: i64;
        result = match map.get(key) {
            Some(x) => *x,
            None => default_value
        };
        return result;
    }
}

fn cap_sum(a: &i64, b: &i64, max: &i64) -> i64 {
    if *a + *b < 0 || *a + *b > *max {
        return *max;
    }
    return *a + *b;
}
/*
fn belowMaxDepth(tip:&Hash, depth:&i32, max_depth_ok: HashSet<Hash> )-> bool {
//if tip is confirmed stop
    /*if TransactionObject.fromHash(tangle, tip).snapshotIndex() >= depth {
        return false;
    }*/
//if tip unconfirmed, check if any referenced tx is confirmed below maxDepth
    let mut non_analyzed_transactions = LinkedList::new() ;
    let mut analyzed_transcations:HashSet <Hash>  = HashSet::new();
    let mut hash:Hash;
    while (hash = non_analyzed_transactions.front()) != None {
        if analyzed_transcations.insert(hash) {
            let mut  transaction: Transaction = Transaction::new();
            transaction.from_hash(&hash);
           /* if transaction.snapshotIndex() != 0 && transaction.snapshotIndex() < depth {
                return true;
            }*/
            if transaction.snapshot_index() == 0 {
                if max_depth_ok.contains(hash) {

                } else {
                    non_analyzed_transactions.offer(transaction.get_trunk_transaction_hash());
                    non_analyzed_transactions.offer(transaction.get_branch_transaction_hash());
                }
            }
        }
    }
    maxDepthOk.add(tip);
    return false;
}*/
