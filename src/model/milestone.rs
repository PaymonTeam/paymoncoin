use model::transaction::*;

#[derive(Clone)]
pub struct Milestone {
    index: u32,
    hash: Hash,
    pub latest_solid_subtangle_milestone_index: u32,
    pub latest_milestone_index: u32,
    pub latest_solid_subtangle_milestone: Hash,
    pub latest_milestone: Hash,
}

impl Milestone {
    pub fn new() -> Self {
        let index = 0u32;
        let hash = HASH_NULL;
        let latest_solid_subtangle_milestone_index = 0u32;
        let latest_milestone_index = 0u32;
        let latest_solid_subtangle_milestone: Hash = HASH_NULL;
        let latest_milestone: Hash = HASH_NULL;

        Milestone {
            index,
            hash,
            latest_solid_subtangle_milestone_index,
            latest_milestone_index,
            latest_solid_subtangle_milestone,
            latest_milestone,
        }
    }
}