//! Shared core types for epistemic actions and grounded-goal stale-evidence barriers.

use crate::{CommodityKind, Component, EntityId, Permille};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::num::NonZeroU32;

/// Subject of a grounded epistemic barrier or social query.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum EpistemicSubject {
    /// A specific entity believed to still be at the given place.
    EntityLocation { entity: EntityId, place: EntityId },
    /// A believed source expected to still have supply available at this place.
    SupplyAvailability {
        commodity: CommodityKind,
        source: EntityId,
        place: EntityId,
    },
}

/// Per-agent parameters governing deliberate epistemic behavior.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EpistemicDispositionProfile {
    /// Staleness level at which a grounded goal inserts an epistemic barrier requiring evidence refresh.
    pub stale_evidence_barrier_threshold: Permille,
    /// Duration in ticks for an ask-witness action.
    pub witness_query_duration_ticks: NonZeroU32,
    /// How long (in ticks) the agent remembers a previous ask-witness attempt to avoid redundant queries.
    pub ask_memory_retention_ticks: u32,
}

impl Default for EpistemicDispositionProfile {
    fn default() -> Self {
        Self {
            stale_evidence_barrier_threshold: Permille::new_unchecked(400),
            witness_query_duration_ticks: NonZeroU32::new(2).unwrap(),
            ask_memory_retention_ticks: 12,
        }
    }
}

impl Component for EpistemicDispositionProfile {}

#[cfg(test)]
mod tests {
    use super::{EpistemicDispositionProfile, EpistemicSubject};
    use crate::{CommodityKind, Component, Permille, test_utils::entity_id};
    use serde::{Serialize, de::DeserializeOwned};
    use std::fmt::Debug;
    use std::num::NonZeroU32;

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_component<T: Component>() {}

    #[test]
    fn epistemic_types_satisfy_expected_bounds() {
        assert_value_bounds::<EpistemicSubject>();
        assert_value_bounds::<EpistemicDispositionProfile>();
        assert_component::<EpistemicDispositionProfile>();
    }

    #[test]
    fn epistemic_subject_entity_location_roundtrips_through_bincode() {
        let subject = EpistemicSubject::EntityLocation {
            entity: entity_id(11, 0),
            place: entity_id(3, 0),
        };

        let bytes = bincode::serialize(&subject).unwrap();
        let roundtrip: EpistemicSubject = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, subject);
    }

    #[test]
    fn epistemic_subject_supply_availability_roundtrips_through_bincode() {
        let subject = EpistemicSubject::SupplyAvailability {
            commodity: CommodityKind::Bread,
            source: entity_id(12, 0),
            place: entity_id(4, 0),
        };

        let bytes = bincode::serialize(&subject).unwrap();
        let roundtrip: EpistemicSubject = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, subject);
    }

    #[test]
    fn epistemic_disposition_profile_roundtrips_through_bincode() {
        let profile = EpistemicDispositionProfile {
            stale_evidence_barrier_threshold: Permille::new(400).unwrap(),
            witness_query_duration_ticks: NonZeroU32::new(3).unwrap(),
            ask_memory_retention_ticks: 17,
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: EpistemicDispositionProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn epistemic_disposition_profile_default_matches_fixture_baseline() {
        let profile = EpistemicDispositionProfile::default();

        assert_eq!(
            profile.stale_evidence_barrier_threshold,
            Permille::new(400).unwrap()
        );
        assert_eq!(
            profile.witness_query_duration_ticks,
            NonZeroU32::new(2).unwrap()
        );
        assert_eq!(profile.ask_memory_retention_ticks, 12);
    }
}
