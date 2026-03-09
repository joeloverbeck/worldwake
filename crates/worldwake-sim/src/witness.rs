//! Deterministic witness sets resolved at event creation time.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use worldwake_core::EntityId;

/// Entities that directly perceived or could potentially perceive an event.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WitnessData {
    pub direct_witnesses: BTreeSet<EntityId>,
    pub potential_witnesses: BTreeSet<EntityId>,
}

#[cfg(test)]
mod tests {
    use super::WitnessData;
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::BTreeSet;
    use std::fmt::Debug;
    use worldwake_core::EntityId;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn assert_traits<T: Clone + Debug + Eq + Serialize + DeserializeOwned>() {}

    #[test]
    fn witness_data_satisfies_required_traits() {
        assert_traits::<WitnessData>();
    }

    #[test]
    fn witness_data_accepts_empty_sets() {
        let witness_data = WitnessData::default();

        assert!(witness_data.direct_witnesses.is_empty());
        assert!(witness_data.potential_witnesses.is_empty());
    }

    #[test]
    fn witness_data_uses_deterministic_set_ordering() {
        let witness_data = WitnessData {
            direct_witnesses: BTreeSet::from([entity(3), entity(1), entity(2)]),
            potential_witnesses: BTreeSet::from([entity(6), entity(4), entity(5)]),
        };

        assert_eq!(
            witness_data
                .direct_witnesses
                .into_iter()
                .collect::<Vec<_>>(),
            vec![entity(1), entity(2), entity(3)]
        );
        assert_eq!(
            witness_data
                .potential_witnesses
                .into_iter()
                .collect::<Vec<_>>(),
            vec![entity(4), entity(5), entity(6)]
        );
    }

    #[test]
    fn witness_data_roundtrips_through_bincode() {
        let witness_data = WitnessData {
            direct_witnesses: BTreeSet::from([entity(2), entity(1)]),
            potential_witnesses: BTreeSet::from([entity(4), entity(3)]),
        };

        let bytes = bincode::serialize(&witness_data).unwrap();
        let roundtrip: WitnessData = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, witness_data);
    }
}
