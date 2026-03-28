//! Shared core types for epistemic actions and grounded-goal verification barriers.

use crate::{CommodityKind, Component, EntityId, Permille};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::num::NonZeroU32;

/// Subject of a proactive belief-verification attempt.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum VerificationSubject {
    /// Verify whether a specific entity is still at the believed place.
    EntityLocation { entity: EntityId, place: EntityId },
    /// Verify whether a believed source still has supply available at this place.
    SupplyAvailability {
        commodity: CommodityKind,
        source: EntityId,
        place: EntityId,
    },
}

/// Per-agent parameters governing proactive belief-verification behavior.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VerificationDispositionProfile {
    pub belief_verification_threshold: Permille,
    pub verify_belief_duration_ticks: NonZeroU32,
    pub witness_query_duration_ticks: NonZeroU32,
    pub ask_memory_retention_ticks: u32,
}

impl Component for VerificationDispositionProfile {}

#[cfg(test)]
mod tests {
    use super::{VerificationDispositionProfile, VerificationSubject};
    use crate::{test_utils::entity_id, CommodityKind, Component, Permille};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;
    use std::num::NonZeroU32;

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_component<T: Component>() {}

    #[test]
    fn epistemic_types_satisfy_expected_bounds() {
        assert_value_bounds::<VerificationSubject>();
        assert_value_bounds::<VerificationDispositionProfile>();
        assert_component::<VerificationDispositionProfile>();
    }

    #[test]
    fn verification_subject_entity_location_roundtrips_through_bincode() {
        let subject = VerificationSubject::EntityLocation {
            entity: entity_id(11, 0),
            place: entity_id(3, 0),
        };

        let bytes = bincode::serialize(&subject).unwrap();
        let roundtrip: VerificationSubject = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, subject);
    }

    #[test]
    fn verification_subject_supply_availability_roundtrips_through_bincode() {
        let subject = VerificationSubject::SupplyAvailability {
            commodity: CommodityKind::Bread,
            source: entity_id(12, 0),
            place: entity_id(4, 0),
        };

        let bytes = bincode::serialize(&subject).unwrap();
        let roundtrip: VerificationSubject = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, subject);
    }

    #[test]
    fn verification_disposition_profile_roundtrips_through_bincode() {
        let profile = VerificationDispositionProfile {
            belief_verification_threshold: Permille::new(400).unwrap(),
            verify_belief_duration_ticks: NonZeroU32::new(5).unwrap(),
            witness_query_duration_ticks: NonZeroU32::new(3).unwrap(),
            ask_memory_retention_ticks: 17,
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: VerificationDispositionProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }
}
