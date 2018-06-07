use std::collections::{HashMap, HashSet, LinkedList};
use std::iter::Iterator;
use std::cmp::max;
use std::sync::{Arc, Mutex};

use model::transaction::*;
use storage::hive::Hive;
use model::milestone::{Milestone, MilestoneObject};
use utils::defines::AM;
use model::tips_view_model::TipsViewModel;
use model::ledger_validator::LedgerValidator;
use model::transaction_validator::TransactionValidator;
use model::transaction_validator::TransactionError;
use std::thread;

use rand::Rng;
use std::time::Duration;
use std::thread::JoinHandle;

extern crate rand;

#[derive(Hash, Eq, PartialEq, Debug)]
struct Pair(Hash, i64);

pub const RESCAN_TX_TO_REQUEST_INTERVAL: u32 = 750;
pub struct TipsManager {
    hive: AM<Hive>,
    max_depth: u32,
    milestone: AM<Milestone>,
    milestone_start_index: u32,
    tips_view_model: AM<TipsViewModel>,
    ledger_validator: AM<LedgerValidator>,
    transaction_validator: AM<TransactionValidator>,
    testnet: bool,
    //private int RATING_THRESHOLD = 75; // Must be in [0..100] range
    shutting_down: bool,
    solidity_rescan_handle: Option<JoinHandle<()>>
}


impl TipsManager {
    pub fn new(hive: AM<Hive>,
               milestone: AM<Milestone>,
               ledger_validator: AM<LedgerValidator>,
               transaction_validator: AM<TransactionValidator>,
               tips_view_model: AM<TipsViewModel>,
               max_depth: u32,
               testnet: bool,
               milestone_start_index: u32) -> AM<Self> {

        let tips_manager = TipsManager {
            hive,
            max_depth,
            milestone,
            milestone_start_index,
            tips_view_model,
            ledger_validator,
            transaction_validator,
            testnet,
            shutting_down: false,
            solidity_rescan_handle: None
        };
        return make_am!(tips_manager)
    }

    pub fn init(tm: AM<TipsManager>) {
        let tm_clone = tm.clone();

        let solidity_rescan_handle = thread::spawn(move|| {
            let mut shutting_down = false;
            while !shutting_down {
                if let Ok(s_am) = tm_clone.lock(){
                    s_am.scan_tips_for_solidity();
                }
                thread::sleep_ms(RESCAN_TX_TO_REQUEST_INTERVAL);

                if let Ok(s_am) = tm_clone.lock() {
                    shutting_down = s_am.shutting_down;
                }
            }
        });

        if let Ok(mut s_am) = tm.lock() {
            s_am.solidity_rescan_handle = Some(solidity_rescan_handle);
        }
    }

    pub fn get_max_depth(&self) -> u32 {
        self.max_depth
    }

    fn scan_tips_for_solidity(&self) -> Result<(), TransactionError> {
        if let Ok(mut t_v_m) = self.tips_view_model.lock() {
            let mut size = t_v_m.get_non_solid_tips_count();
            if size != 0 {
                if let Some(hash) = t_v_m.get_random_tip() {
                    let mut is_tip = true;
                    let mut tx;
                    // println!("hive lock 7");
                    if let Ok(hive) = self.hive.lock() {
                        tx = hive.storage_load_transaction(&hash).unwrap();
                    } else {
                        panic!("broken hive mutex");
                    }
                    // println!("hive unlock 7");
                    if tx.get_approvers(&self.hive).len() != 0 {
                        t_v_m.remove_tip(&hash);
                        is_tip = false;
                    }
                    if is_tip {
                        if let Ok(t_v) = self.transaction_validator.lock() {
                            if t_v.check_solidity(hash, false)? {
                                t_v_m.set_solid(&hash);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn transaction_to_approve(&self,
                                  visited_hashes: &mut HashSet<Hash>,
                                  diff: &mut HashMap<Address, i64>,
                                  reference: Option<Hash>,
                                  extra_tip: Option<Hash>,
                                  mut depth: u32,
                                  iterations: u32) -> Result<Option<Hash>, TransactionError> {
        if depth > self.max_depth {
            depth = self.max_depth;
        }

        let latest_solid_subhive_milestone_index;
        let latest_solid_subhive_milestone;

        if let Ok(milestone) = self.milestone.lock() {
            latest_solid_subhive_milestone_index = milestone.latest_solid_subhive_milestone_index;
            latest_solid_subhive_milestone = milestone.latest_milestone_index;
        } else {
            panic!("broken milestone mutex");
        }

        if latest_solid_subhive_milestone_index > self.milestone_start_index || latest_solid_subhive_milestone == self.milestone_start_index {
            let mut ratings: HashMap<Hash, i64> = HashMap::new();
            let mut analyzed_tips: HashSet<Hash> = HashSet::new();
            let mut max_depth_ok: HashSet<Hash> = HashSet::new();

            let tip = self.entry_point(reference,
                                       extra_tip.clone(),
                                       depth);

            self.serial_update_ratings(visited_hashes,
                                       tip,
                                       &mut ratings,
                                       &mut analyzed_tips,
                                       extra_tip.clone());
            analyzed_tips.clear();

            let update_diff_is_ok;
            if let Ok(mut lv) = self.ledger_validator.lock() {
                update_diff_is_ok = lv.update_diff(visited_hashes, diff, tip.clone())?;
            } else {
                panic!("ledger validator is broken");
            }

            if update_diff_is_ok {
                return Ok(self.markov_chain_monte_carlo(visited_hashes,
                                                          diff,
                                                          tip,
                                                          extra_tip,
                                                          &mut ratings,
                                                          iterations,
                                                          latest_solid_subhive_milestone_index -
                                                              depth * 2,
                                                          &mut max_depth_ok)?);
            } else {
                error!("starting tip failed consistency check");
                return Err(TransactionError::InvalidHash);
            }
        }
        return Ok(None);
    }

    fn entry_point(&self, reference: Option<Hash>, extra_tip: Option<Hash>, depth: u32) -> Hash {
        if extra_tip.is_none() {
            //trunk
            if let Some(r) = reference {
                return r;
            } else {
                if let Ok(mlstn) = self.milestone.lock() {
                    return mlstn.latest_solid_subhive_milestone;
                } else {
                    panic!("broken milestone mutex");
                }
            }
        }

        if let Ok(milestone) = self.milestone.lock() {
            //branch (extraTip)
            let milestone_index = match milestone.latest_solid_subhive_milestone_index - 1 > depth {
                true => milestone.latest_solid_subhive_milestone_index - depth - 1,
                false => 0
            };

            if let Ok(hive) = self.hive.lock() {
                if let Some(milestone) = hive.find_closest_next_milestone(milestone_index, self.testnet, self.milestone_start_index) {
                    let hash = milestone.get_hash();
                    if hash != HASH_NULL {
                        return hash;
                    }
                }
                return milestone.latest_solid_subhive_milestone;
            } else {
                panic!("broken hive mutex");
            }
        } else {
            panic!("broken milestone mutex");
        }
    }

    pub fn random_walk(&self,
                       visited_hashes: &HashSet<Hash>,
                       diff: &HashMap<Address, i64>,
                       start: Option<Hash>,
                       extra_tip: Option<Hash>,
                       ratings: &mut HashMap<Hash, i64>,
                       max_depth: u32,
                       max_depth_ok: &mut HashSet<Hash>) -> Result<Option<Hash>, TransactionError> {
        let mut rnd = rand::thread_rng();
        let mut tip = start.clone();
        let mut tail = tip.clone();
        let mut tips: Vec<Hash>;
        let mut tip_set: HashSet<Hash>;
        let mut analyzed_tips: HashSet<Hash> = HashSet::new();
        let mut traversed_tails = 0;
        let mut transaction_obj; // = Transaction::new();
        let mut approver_index: usize;
        let mut rating_weight: f64;
        let mut walk_ratings: Vec<f64>;
        let mut my_diff = diff.clone();
        let mut my_approved_hashes = visited_hashes.clone();

        while let Some(tip_hash) = tip {
            transaction_obj = match self.hive.lock() {
                Ok(hive) => hive.storage_load_transaction(&tip_hash).expect("tip is null"),
                Err(_) => {
                    panic!("hive mutex is broken");
                }
            };

            tip_set = transaction_obj.get_approvers(&self.hive);

            if transaction_obj.get_type() == TransactionType::HashOnly {
                info!("Reason to stop: transactionViewModel == null");
                break;
            }

            let check_solidity_is_ok = match self.transaction_validator.lock() {
                Ok(tv) => tv.check_solidity(transaction_obj.get_hash(), false)?,
                Err(_) => panic!("broken transaction validator mutex")
            };

            if !check_solidity_is_ok {
                info!("Reason to stop: !checkSolidity");
                break;
            }

            if self.below_max_depth(transaction_obj.get_hash(), max_depth, max_depth_ok) {
                info!("Reason to stop: !LedgerValidator");
                break;
            }

            let update_diff_is_ok = match self.ledger_validator.lock() {
                Ok(mut lv) => lv.update_diff(&mut my_approved_hashes, &mut my_diff,
                                             transaction_obj.get_hash())?,
                Err(_) => panic!("broken ledger validator mutex")
            };

            if !update_diff_is_ok {
                info!("Reason to stop: belowMaxDepth");
                break;
            }

            if extra_tip.is_some() && transaction_obj.get_hash() == extra_tip.unwrap() {
                info!("Reason to stop: transactionViewModel==extraTip");
                break;
            }

            tail = Some(tip_hash.clone());
            traversed_tails += 1;

            if tip_set.len() == 0 {
                info!("Reason to stop: TransactionViewModel is a tip");
                break;
            } else if tip_set.len() == 1 {
                tip = tip_set.iter().next().cloned();
            } else {
                tips = TipsManager::set_to_vec(&tip_set);
                if !ratings.contains_key(&tip_hash) {
                    self.serial_update_ratings(
                        &my_approved_hashes,
                        tip_hash,
                        ratings,
                        &mut analyzed_tips,
                        extra_tip);
                    analyzed_tips.clear();
                }

                walk_ratings = Vec::with_capacity(tips.len());
                let mut max_rating: f64 = 0.0;
                let mut tip_rating: i64 = match ratings.get(&tip_hash) {
                    Some(x) => *x,
                    None => {
                        warn!("no rating");
                        0
                    }
                };

                for i in 0..tips.len() {
                    let v = ((tip_rating - TipsManager::get_or_default(ratings, tips[i], 0i64))
                        as f32).powf(-3 as f32) as f64;
                    walk_ratings.push(v);
                    max_rating += v;
                }

                rating_weight = rnd.gen::<f64>() * max_rating;

                approver_index = tips.len();
                loop {
                    approverIndex -= 1;
                    if approver_index > 1 {
                        rating_weight -= walk_ratings[approver_index];
                        if rating_weight <= 0f64 {
                            break;
                        }
                    }
                }

                tip = tips.get(approver_index as usize).cloned();
                if let Some(h) = tip {
                    if transaction_obj.get_hash() == h {
                        break;
                    }
                }
            }
        }
        return Ok(tail);
    }

    pub fn markov_chain_monte_carlo(&self,
                                    visited_hashes: &HashSet<Hash>,
                                    diff: &HashMap<Address, i64>,
                                    tip: Hash,
                                    extra_tip: Option<Hash>,
                                    ratings: &mut HashMap<Hash, i64>,
                                    iterations: u32,
                                    max_depth: u32,
                                    max_depth_ok: &mut HashSet<Hash>,
                                    /*Random seed*/) -> Result<Option<Hash>, TransactionError> {
        let mut rnd = rand::thread_rng();
        let mut monte_carlo_integrations = HashMap::<Hash, i32>::new();
        let mut tail: Hash;
//        println!("visited_hashes={:?}", visited_hashes);
//        println!("diff={:?}", diff);
        for _ in 0..iterations {
            if let Some(tail) = self.random_walk(visited_hashes, diff, Some(tip), extra_tip,
                                                ratings, max_depth,
                                    max_depth_ok)? {
                // TODO: make binding
                if monte_carlo_integrations.contains_key(&tail) {
                    let v = monte_carlo_integrations.get(&tail).cloned().unwrap();
                    monte_carlo_integrations.insert(tail.clone(), v + 1);
                } else {
                    monte_carlo_integrations.insert(tail.clone(), 1);
                }
            }
        }
//        println!("monte_carlo_integrations={:?}", monte_carlo_integrations);
        let (reduced, _) = monte_carlo_integrations.into_iter().fold((HASH_NULL, 0), |(a, a_v), (b,
            b_v)| {
            if a_v > b_v {
                return (a, a_v);
            } else if a_v < b_v {
                return (b, b_v);
            } else if rnd.gen() {
                return (a, a_v);
            } else {
                return (b, b_v);
            }
        });

        if reduced == HASH_NULL {
            return Ok(None);
        } else {
            Ok(Some(reduced))
        }
    }

    fn set_to_vec(set: &HashSet<Hash>) -> Vec<Hash> {
        let mut hash_iterator = set.iter();
        let mut result: Vec<Hash> = Vec::with_capacity(set.len());
        if !set.is_empty() {
            loop {
                match hash_iterator.next() {
                    Some(hash) => result.push(hash.clone()),
                    None => break
                }
            }
        }
        return result;
    }

    fn serial_update_ratings(&self,
                             visited_hashes: &HashSet<Hash>,
                             tx_hash: Hash,
                             ratings: &mut HashMap<Hash, i64>,
                             analyzed_tips: &mut HashSet<Hash>,
                             extra_tip: Option<Hash>) {
        let mut hashes_to_rate = Vec::<Hash>::new();
        hashes_to_rate.push(tx_hash);
        let mut current_hash: Hash;
        let mut added_back;

        while !hashes_to_rate.is_empty() {
            current_hash = match hashes_to_rate.pop() {
                Some(hash) => hash,
                None => {
                    continue;
                }
            };

            let mut transaction = match self.hive.lock() {
                Ok(hive) => hive.storage_load_transaction(&current_hash).expect("no such transaction"),
                Err(_) => {
                    panic!("hive mutex is broken");
                }
            };

            added_back = false;
            let mut approvers = transaction.get_approvers(&self.hive).clone();
            for approver in &approvers {
                if ratings.get(approver).is_none() && *approver != current_hash {
                    if !added_back {
                        added_back = true;
                        hashes_to_rate.push(current_hash);
                    }
                    hashes_to_rate.push(*approver);
                }
            }
            if !added_back && analyzed_tips.insert(current_hash) {
                let rating = TipsManager::rating_calc(extra_tip, &visited_hashes, current_hash, &approvers, ratings);
                ratings.insert(current_hash.clone(), rating);
            }
        }
    }

    fn rating_calc(extra_tip: Option<Hash>, visited_hashes: &HashSet<Hash>, current_hash: Hash,
                   approvers: &HashSet<Hash>, ratings: &HashMap<Hash, i64>) -> i64 {
        let mut result: i64;
        result = match extra_tip.is_some() && visited_hashes.contains(&current_hash) {
            true => 0,
            false => 1
        };

        // TODO: cap_sum -> overflow_add
        result += approvers.iter().
            map(|x| ratings.get(x)).
            filter(|x| x.is_some()).
            fold(0i64, |a, b| TipsManager::cap_sum(a, *b.unwrap(), (<i64>::max_value() / 2)));
        return result;
    }

    fn get_or_default(map: &HashMap<Hash, i64>, key: Hash, default_value: i64) -> i64 {
        let result = match map.get(&key) {
            Some(x) => *x,
            None => default_value
        };
        result
    }

    fn below_max_depth(&self, tip: Hash, depth: u32, max_depth_ok: &mut HashSet<Hash>) -> bool {
        //if tip is confirmed stop

        // println!("hive lock 19");
        let mut transaction = match self.hive.lock() {
            Ok(hive) => hive.storage_load_transaction(&tip).expect("can't find transaction"),
            Err(_) => {
                panic!("hive mutex is broken");
            }
        };
        // println!("hive unlock 19");

        if transaction.object.get_snapshot_index() >= depth {
            return false;
        }

        //if tip unconfirmed, check if any referenced tx is confirmed below maxDepth
        let mut non_analyzed_transactions = LinkedList::new();
        non_analyzed_transactions.push_back(tip);
        let mut analyzed_transactions: HashSet<Hash> = HashSet::new();
        let mut hash: Hash;
        while non_analyzed_transactions.front() != None {
            hash = match non_analyzed_transactions.front() {
                Some(h) => *h,
                None => break
            };
            if analyzed_transactions.insert(hash) {
                // println!("hive lock 20");
                let mut transaction = match self.hive.lock() {
                    Ok(hive) => hive.storage_load_transaction(&hash).expect("can't load \
                    transaction"),
                    Err(_) => {
                        panic!("hive mutex is broken");
                    }
                };
                // println!("hive unlock 20");

                if transaction.object.get_snapshot_index() != 0 && transaction.object.get_snapshot_index() < depth {
                    return true;
                }
                if transaction.object.get_snapshot_index() == 0 {
                    if !max_depth_ok.contains(&hash) {
                        non_analyzed_transactions.push_back(transaction.get_trunk_transaction_hash());
                        non_analyzed_transactions.push_back(transaction.get_branch_transaction_hash());
                    }
                }
            }
        }
        max_depth_ok.insert(tip);
        return false;
    }

    pub fn recursive_update_ratings(&self,
                                    tx_hash: Hash,
                                    ratings: &mut HashMap<Hash, i64>,
                                    analyzed_tips: &mut HashSet<Hash>) -> i64 {
        let mut rating = 1;
        if analyzed_tips.insert(tx_hash) {
            // println!("hive lock 21");
            let mut transaction = match self.hive.lock() {
                Ok(hive) => hive.storage_load_transaction(&tx_hash).expect("tip is null"),
                Err(_) => {
                    panic!("hive mutex is broken");
                }
            };
            // println!("hive unlock 21");

            let mut approver_hashes = transaction.get_approvers(&self.hive);
            for approver in approver_hashes.iter() {
                rating = TipsManager::cap_sum(rating, self.recursive_update_ratings(approver.clone(), ratings, analyzed_tips), (<i64>::max_value() / 2));
            }
            ratings.insert(tx_hash.clone(), rating);
        } else {
            // TODO: optimize
            if ratings.contains_key(&tx_hash) {
                rating = match ratings.get(&tx_hash) {
                    Some(x) => *x,
                    None => 0,
                };
            } else {
                rating = 0;
            }
        }
        return rating;
    }

    pub fn shutdown(&mut self) {
        self.shutting_down = true;
        if let Some(mut jh) = self.solidity_rescan_handle.take() {
            jh.join();
        };
    }

    fn cap_sum(a: i64, b: i64, max: i64) -> i64 {
        if a + b < 0 || a + b > max {
            return max;
        }
        return a + b;
    }
}