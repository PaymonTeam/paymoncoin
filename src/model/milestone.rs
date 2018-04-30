use model::transaction::*;

//#[derive(Clone)]
pub struct Milestone{
    index: u32,
    hash: Hash,
    pub latestSolidSubtangleMilestoneIndex: u32,
    pub latestMilestoneIndex: u32,
    pub latestSolidSubtangleMilestone: Hash,
    pub latestMilestone: Hash,
}
impl Milestone{
    pub fn new() -> Self{
        let index = 0u32;
        let hash = HASH_NULL;
        let latestSolidSubtangleMilestoneIndex = 0u32;
        let latestMilestoneIndex = 0u32;
        let latestSolidSubtangleMilestone:Hash = HASH_NULL;
        let latestMilestone:Hash = HASH_NULL;

        Milestone{
            index,
            hash,
            latestSolidSubtangleMilestoneIndex,
            latestMilestoneIndex,
            latestSolidSubtangleMilestone,
            latestMilestone
        }
    }

}
impl Clone for Milestone{
    fn clone(&self) -> Milestone {

        /*match *self {
            Milestone { index: ref __self_0_0,
                        hash: ref __self_0_1,
                        latestSolidSubtangleMilestoneIndex: ref __self_0_2,
                        latestMilestoneIndex: ref __self_0_3,
                        latestSolidSubtangleMilestone: ref __self_0_4,
                        latestMilestone: ref __self_0_5} => {

                ::std::clone::assert_receiver_is_clone(&(*__self_0_0));
                ::std::clone::assert_receiver_is_clone(&(*__self_0_1));
                ::std::clone::assert_receiver_is_clone(&(*__self_0_2));
                ::std::clone::assert_receiver_is_clone(&(*__self_0_3));
                ::std::clone::assert_receiver_is_clone(&(*__self_0_4));
                ::std::clone::assert_receiver_is_clone(&(*__self_0_5));
                *self
            }
        }*/
        let index = self.index.clone();
        let hash = self.hash.clone();
        let latestSolidSubtangleMilestoneIndex = self.latestSolidSubtangleMilestoneIndex.clone();
        let latestMilestoneIndex = self.latestMilestoneIndex.clone();
        let latestSolidSubtangleMilestone:Hash = self.latestSolidSubtangleMilestone.clone();
        let latestMilestone:Hash = self.latestMilestone.clone();

        Milestone{
            index,
            hash,
            latestSolidSubtangleMilestoneIndex,
            latestMilestoneIndex,
            latestSolidSubtangleMilestone,
            latestMilestone
        }

    }
}