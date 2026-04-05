//! Scene-level evidence and aftermath artifacts stored on places.

use crate::{Component, EntityId, Permille, Tick};
use serde::{Deserialize, Serialize};

/// Unique identifier for an evidence entry within a place's scene evidence.
#[derive(
    Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
)]
pub struct EvidenceEntryId(pub u64);

/// Coarse-grained scene disturbance categories.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum DisturbanceKind {
    CombatAftermath,
    ForcedEntry,
    AbandonedGoods,
    WildernessRelief,
}

/// Persistent physical evidence left behind at a place.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum EvidenceKind {
    ContainerTampered {
        container: EntityId,
        tampered_at: Tick,
    },
    BloodTrail {
        from_place: EntityId,
        severity: Permille,
        caused_by: Option<EntityId>,
    },
    DisturbanceMarker {
        place: EntityId,
        kind: DisturbanceKind,
        created_at: Tick,
    },
    MovementTrace {
        entity: EntityId,
        departed_from: EntityId,
        direction: EntityId,
        observed_at: Tick,
    },
}

/// One evidence record with independent decay timing.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct EvidenceEntry {
    pub id: EvidenceEntryId,
    pub kind: EvidenceKind,
    pub created_at: Tick,
    pub decay_ticks: u32,
}

/// Scene-level evidence stored authoritatively on places.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SceneEvidence {
    pub evidence: Vec<EvidenceEntry>,
    pub next_entry_id: u64,
}

impl Component for SceneEvidence {}

#[cfg(test)]
mod tests {
    use super::{
        DisturbanceKind, EvidenceEntry, EvidenceEntryId, EvidenceKind, SceneEvidence,
    };
    use crate::{traits::Component, EntityId, Permille, Tick};
    use serde::{de::DeserializeOwned, Serialize};
    use std::fmt::Debug;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    fn assert_roundtrip<T>(value: &T)
    where
        T: Clone + Eq + Debug + Serialize + DeserializeOwned,
    {
        let bytes = bincode::serialize(value).unwrap();
        let roundtrip: T = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, *value);
    }

    #[test]
    fn scene_evidence_component_and_value_bounds() {
        assert_component_bounds::<SceneEvidence>();
        assert_value_bounds::<EvidenceEntryId>();
        assert_value_bounds::<DisturbanceKind>();
        assert_value_bounds::<EvidenceKind>();
        assert_value_bounds::<EvidenceEntry>();
        assert_value_bounds::<SceneEvidence>();
    }

    #[test]
    fn evidence_identifier_and_variants_roundtrip_through_bincode() {
        assert_roundtrip(&EvidenceEntryId(7));

        for kind in [
            EvidenceKind::ContainerTampered {
                container: entity(10),
                tampered_at: Tick(11),
            },
            EvidenceKind::BloodTrail {
                from_place: entity(12),
                severity: Permille::new(650).unwrap(),
                caused_by: Some(entity(13)),
            },
            EvidenceKind::DisturbanceMarker {
                place: entity(14),
                kind: DisturbanceKind::WildernessRelief,
                created_at: Tick(15),
            },
            EvidenceKind::MovementTrace {
                entity: entity(16),
                departed_from: entity(17),
                direction: entity(18),
                observed_at: Tick(19),
            },
        ] {
            assert_roundtrip(&kind);
        }
    }

    #[test]
    fn scene_evidence_roundtrips_through_bincode() {
        let scene = SceneEvidence {
            evidence: vec![
                EvidenceEntry {
                    id: EvidenceEntryId(1),
                    kind: EvidenceKind::ContainerTampered {
                        container: entity(20),
                        tampered_at: Tick(21),
                    },
                    created_at: Tick(22),
                    decay_ticks: 200,
                },
                EvidenceEntry {
                    id: EvidenceEntryId(2),
                    kind: EvidenceKind::DisturbanceMarker {
                        place: entity(23),
                        kind: DisturbanceKind::CombatAftermath,
                        created_at: Tick(24),
                    },
                    created_at: Tick(24),
                    decay_ticks: 50,
                },
            ],
            next_entry_id: 3,
        };

        assert_roundtrip(&scene);
    }
}
