use crate::{Component, EntityId, Permille};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Marks a place as an active bandit camp and points at its communal supplies.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BanditCamp {
    pub supplies: EntityId,
}

impl Component for BanditCamp {}

/// Per-camp policy inputs for later bandit action and regrouping systems.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BanditCampProfile {
    pub min_regroup_count: u8,
    pub establishment_duration_ticks: NonZeroU32,
    pub flee_wound_threshold: Permille,
    pub rally_place: Option<EntityId>,
}

impl Component for BanditCampProfile {}

#[cfg(test)]
mod tests {
    use super::{BanditCamp, BanditCampProfile};
    use crate::{traits::Component, EntityId, Permille};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;
    use std::num::NonZeroU32;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn bandit_camp_component_bounds() {
        assert_component_bounds::<BanditCamp>();
        assert_value_bounds::<BanditCamp>();
        assert_component_bounds::<BanditCampProfile>();
        assert_value_bounds::<BanditCampProfile>();
    }

    #[test]
    fn bandit_camp_roundtrips_through_bincode() {
        let camp = BanditCamp {
            supplies: entity(4),
        };

        let bytes = bincode::serialize(&camp).unwrap();
        let roundtrip: BanditCamp = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, camp);
    }

    #[test]
    fn bandit_camp_profile_roundtrips_through_bincode() {
        let profile = BanditCampProfile {
            min_regroup_count: 3,
            establishment_duration_ticks: NonZeroU32::new(12).unwrap(),
            flee_wound_threshold: Permille::new(650).unwrap(),
            rally_place: Some(entity(9)),
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: BanditCampProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }
}
