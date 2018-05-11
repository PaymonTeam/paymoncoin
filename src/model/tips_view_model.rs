use std::collections::HashSet;
use model::transaction::Hash;
use rand::{Rng, thread_rng};

extern crate linked_hash_set;
use self::linked_hash_set::LinkedHashSet;

pub const MAX_TIPS: u32 = 5000;

pub struct TipsViewModel {
    pub tips: LinkedHashSet<Hash>,
    pub solid_tips: LinkedHashSet<Hash>,
}

impl TipsViewModel {
    pub fn new() -> Self {
        TipsViewModel {
            tips: LinkedHashSet::new(),
            solid_tips: LinkedHashSet::new(),
        }
    }

    pub fn add_tip(&mut self, hash: Hash) {
        let mut vacancy = MAX_TIPS as usize - self.tips.len();
        while vacancy <= 0 {
            self.tips.pop_front();
            vacancy += 1;
        }
        self.tips.insert(hash);
    }

    pub fn remove_tip(&mut self, hash: &Hash) {
        if !self.tips.remove(hash) {
            self.solid_tips.remove(hash);
        }
    }

    pub fn set_solid(&mut self, hash: &Hash) {
        if self.tips.remove(hash) {
            let mut vacancy = MAX_TIPS as usize - self.solid_tips.len();
            while vacancy <= 0 {
                self.solid_tips.pop_front();
                vacancy += 1;
            }
            self.solid_tips.insert(*hash);
        }
    }

    pub fn get_tips(&self) -> HashSet<Hash> {
        let mut set = HashSet::<Hash>::new();

        for t in &self.tips {
            set.insert(t.clone());
        }

        for t in &self.solid_tips {
            set.insert(t.clone());
        }

        set
    }

    pub fn get_random_tip(&mut self) -> Option<Hash> {
        let len = self.tips.len();
        if len == 0 {
            return None;
        }

        let index = thread_rng().gen_range(0, len);

        let hash = self.tips.iter().skip(index).next().cloned();
        hash
    }

    pub fn get_random_solid_tip(&mut self) -> Option<Hash> {
        let len = self.solid_tips.len();
        if len == 0 {
            return self.get_random_tip();
        }

        let index = thread_rng().gen_range(0, len);

        let hash = self.solid_tips.iter().skip(index).next().cloned();
        hash
    }

    pub fn get_non_solid_tips_count(&self) -> usize {
        self.tips.len()
    }

    pub fn get_tips_count(&self) -> usize {
        self.tips.len() + self.solid_tips.len()
    }
}
