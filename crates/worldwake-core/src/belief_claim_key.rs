use crate::{EntityBeliefAspect, EntityId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BeliefClaimKey {
    pub subject: EntityId,
    pub aspect: EntityBeliefAspect,
}

#[cfg(test)]
mod tests {
    use super::BeliefClaimKey;
    use crate::{EntityBeliefAspect, EntityId};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;

    fn assert_value_bounds<T: Copy + Clone + Eq + Ord + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn belief_claim_key_satisfies_required_bounds() {
        assert_value_bounds::<BeliefClaimKey>();
    }

    #[test]
    fn belief_claim_key_roundtrips_through_bincode() {
        let key = BeliefClaimKey {
            subject: EntityId {
                slot: 7,
                generation: 2,
            },
            aspect: EntityBeliefAspect::Inventory(crate::CommodityKind::Bread),
        };

        let bytes = bincode::serialize(&key).unwrap();
        let roundtrip: BeliefClaimKey = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, key);
    }
}
