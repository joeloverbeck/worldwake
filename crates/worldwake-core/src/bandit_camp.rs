use crate::{Component, EntityId, Permille, Tick};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Marks a place as an active bandit camp and points at its communal supplies.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BanditCamp {
    pub faction: EntityId,
    pub supplies: EntityId,
    pub empty_since_tick: Option<Tick>,
}

impl Component for BanditCamp {}

/// Faction-scoped policy inputs for camp regrouping and establishment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BanditFactionPolicy {
    pub min_regroup_count: u8,
    pub establishment_duration_ticks: NonZeroU32,
    pub abandonment_grace_ticks: NonZeroU32,
    pub flee_wound_threshold: Permille,
    pub rally_place: Option<EntityId>,
}

impl Component for BanditFactionPolicy {}

#[cfg(test)]
mod tests {
    use super::{BanditCamp, BanditFactionPolicy};
    use crate::{traits::Component, EntityId, Permille, Tick};
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
        assert_component_bounds::<BanditFactionPolicy>();
        assert_value_bounds::<BanditFactionPolicy>();
    }

    #[test]
    fn bandit_camp_roundtrips_through_bincode() {
        let camp = BanditCamp {
            faction: entity(2),
            supplies: entity(4),
            empty_since_tick: Some(Tick(7)),
        };

        let bytes = bincode::serialize(&camp).unwrap();
        let roundtrip: BanditCamp = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, camp);
    }

    #[test]
    fn bandit_faction_policy_roundtrips_through_bincode() {
        let profile = BanditFactionPolicy {
            min_regroup_count: 3,
            establishment_duration_ticks: NonZeroU32::new(12).unwrap(),
            abandonment_grace_ticks: NonZeroU32::new(5).unwrap(),
            flee_wound_threshold: Permille::new(650).unwrap(),
            rally_place: Some(entity(9)),
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: BanditFactionPolicy = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }
}
