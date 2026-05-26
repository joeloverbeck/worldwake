//! Runtime occupancy for scarce self-care facilities and places.

use crate::{Component, EntityId, GoalKey, Tick};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SelfCareOccupancy {
    pub occupant: EntityId,
    pub use_kind: SelfCareUseKind,
    pub started_tick: Tick,
    pub goal_key: GoalKey,
}

impl Component for SelfCareOccupancy {}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SelfCareUseKind {
    Wash,
    LatrineRelief,
    Eat,
    Drink,
    WildernessRelief,
}

#[cfg(test)]
mod tests {
    use super::{SelfCareOccupancy, SelfCareUseKind};
    use crate::{Component, EntityId, GoalKey, GoalKind, Tick};

    fn assert_component_bounds<T: Component>() {}

    #[test]
    fn self_care_occupancy_satisfies_component_bounds() {
        assert_component_bounds::<SelfCareOccupancy>();
    }

    #[test]
    fn self_care_occupancy_roundtrips_through_bincode() {
        let occupancy = SelfCareOccupancy {
            occupant: EntityId {
                slot: 7,
                generation: 0,
            },
            use_kind: SelfCareUseKind::Wash,
            started_tick: Tick(12),
            goal_key: GoalKey::from(GoalKind::Wash),
        };

        let bytes = bincode::serialize(&occupancy).unwrap();
        let roundtrip: SelfCareOccupancy = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, occupancy);
    }
}
