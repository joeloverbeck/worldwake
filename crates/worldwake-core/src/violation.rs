//! Expectation-violation types for belief-perception mismatch detection.
//!
//! When an agent observes a mismatch between a prior belief and current
//! perception, a [`ViolationKind`] describes the concrete discrepancy.
//! [`ViolationMemory`] prevents repeated reactive goal generation for the
//! same already-noticed mismatch. [`ViolationDispositionProfile`] governs
//! per-agent investigation behavior (P2, P20).

use crate::ids::{EntityId, Tick};
use crate::items::CommodityKind;
use crate::numerics::Permille;
use crate::traits::Component;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// A detected mismatch between prior belief and current local observation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ViolationKind {
    /// Agent believed entity was at this place; entity is absent on observation.
    EntityMissing {
        entity: EntityId,
        expected_place: EntityId,
    },
    /// Agent believed commodity was available at a source here; source is depleted.
    SupplyDepleted {
        commodity: CommodityKind,
        source: EntityId,
        place: EntityId,
    },
    /// Agent believed entity was alive; entity is now dead.
    EntityDead { entity: EntityId },
}

/// A single recorded violation with expiry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecordedViolation {
    pub kind: ViolationKind,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
}

/// Records detected violations to prevent repeated reactive goal generation
/// for the same already-noticed mismatch.
///
/// Follows the same pattern as [`crate::BlockedIntentMemory`]: a `Vec` with
/// expiry-based retention and per-kind deduplication.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ViolationMemory {
    pub violations: Vec<RecordedViolation>,
}

impl Component for ViolationMemory {}

impl ViolationMemory {
    /// Returns `true` if an unexpired record exists for this violation kind.
    pub fn is_recorded(&self, kind: &ViolationKind, current_tick: Tick) -> bool {
        self.violations
            .iter()
            .any(|r| &r.kind == kind && r.expires_tick > current_tick)
    }

    /// Records a violation, replacing any existing entry for the same kind.
    /// TTL is in ticks (added to `observed_tick` to compute `expires_tick`).
    pub fn record(&mut self, kind: ViolationKind, observed_tick: Tick, ttl: u32) {
        self.violations.retain(|r| r.kind != kind);
        self.violations.push(RecordedViolation {
            kind,
            observed_tick,
            expires_tick: Tick(observed_tick.0 + u64::from(ttl)),
        });
    }

    /// Removes all entries where `current_tick >= expires_tick`.
    pub fn expire(&mut self, current_tick: Tick) {
        self.violations.retain(|r| r.expires_tick > current_tick);
    }
}

/// Per-agent parameters governing investigation behavior.
/// Enables agent diversity (P20) for violation response.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ViolationDispositionProfile {
    /// Duration in ticks for the investigate action. Per-agent curiosity/thoroughness.
    pub investigation_duration_ticks: NonZeroU32,
    /// How many ticks before a recorded violation expires from memory.
    pub violation_memory_retention_ticks: u32,
    /// Base motive weight for investigation goals.
    pub investigation_motive_weight: Permille,
    /// Additional motive when the agent owns the missing entity.
    pub ownership_motive_bonus: Permille,
}

impl Component for ViolationDispositionProfile {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EntityId;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn sample_missing() -> ViolationKind {
        ViolationKind::EntityMissing {
            entity: entity(1),
            expected_place: entity(10),
        }
    }

    fn sample_depleted() -> ViolationKind {
        ViolationKind::SupplyDepleted {
            commodity: CommodityKind::Apple,
            source: entity(2),
            place: entity(10),
        }
    }

    fn sample_dead() -> ViolationKind {
        ViolationKind::EntityDead { entity: entity(3) }
    }

    // --- ViolationMemory::record deduplicates by kind ---

    #[test]
    fn record_replaces_existing_entry_for_same_kind() {
        let mut mem = ViolationMemory::default();
        let kind = sample_missing();

        mem.record(kind.clone(), Tick(5), 10);
        assert_eq!(mem.violations.len(), 1);
        assert_eq!(mem.violations[0].observed_tick, Tick(5));

        // Recording again replaces
        mem.record(kind.clone(), Tick(8), 10);
        assert_eq!(mem.violations.len(), 1);
        assert_eq!(mem.violations[0].observed_tick, Tick(8));
        assert_eq!(mem.violations[0].expires_tick, Tick(18));
    }

    #[test]
    fn record_different_kinds_accumulate() {
        let mut mem = ViolationMemory::default();
        mem.record(sample_missing(), Tick(1), 10);
        mem.record(sample_depleted(), Tick(2), 10);
        mem.record(sample_dead(), Tick(3), 10);
        assert_eq!(mem.violations.len(), 3);
    }

    // --- ViolationMemory::expire ---

    #[test]
    fn expire_removes_entries_past_ttl() {
        let mut mem = ViolationMemory::default();
        mem.record(sample_missing(), Tick(5), 10); // expires at 15
        mem.record(sample_depleted(), Tick(5), 20); // expires at 25

        mem.expire(Tick(15));
        assert_eq!(mem.violations.len(), 1);
        assert_eq!(mem.violations[0].kind, sample_depleted());
    }

    #[test]
    fn expire_at_exact_boundary_removes() {
        let mut mem = ViolationMemory::default();
        mem.record(sample_missing(), Tick(5), 10); // expires at 15

        // At tick 15, expires_tick(15) > current_tick(15) is false -> removed
        mem.expire(Tick(15));
        assert!(mem.violations.is_empty());
    }

    #[test]
    fn expire_keeps_unexpired_entries() {
        let mut mem = ViolationMemory::default();
        mem.record(sample_missing(), Tick(5), 10); // expires at 15

        mem.expire(Tick(14));
        assert_eq!(mem.violations.len(), 1);
    }

    // --- ViolationMemory::is_recorded ---

    #[test]
    fn is_recorded_true_for_unexpired() {
        let mut mem = ViolationMemory::default();
        let kind = sample_missing();
        mem.record(kind.clone(), Tick(5), 10);

        assert!(mem.is_recorded(&kind, Tick(5)));
        assert!(mem.is_recorded(&kind, Tick(14)));
    }

    #[test]
    fn is_recorded_false_after_expiry() {
        let mut mem = ViolationMemory::default();
        let kind = sample_missing();
        mem.record(kind.clone(), Tick(5), 10); // expires at 15

        assert!(!mem.is_recorded(&kind, Tick(15)));
        assert!(!mem.is_recorded(&kind, Tick(100)));
    }

    #[test]
    fn is_recorded_false_for_unrecorded_kind() {
        let mem = ViolationMemory::default();
        assert!(!mem.is_recorded(&sample_missing(), Tick(0)));
    }

    // --- ViolationKind serde round-trip ---

    #[test]
    fn violation_kind_serde_roundtrip_entity_missing() {
        let kind = sample_missing();
        let bytes = bincode::serialize(&kind).unwrap();
        let roundtrip: ViolationKind = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, kind);
    }

    #[test]
    fn violation_kind_serde_roundtrip_supply_depleted() {
        let kind = sample_depleted();
        let bytes = bincode::serialize(&kind).unwrap();
        let roundtrip: ViolationKind = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, kind);
    }

    #[test]
    fn violation_kind_serde_roundtrip_entity_dead() {
        let kind = sample_dead();
        let bytes = bincode::serialize(&kind).unwrap();
        let roundtrip: ViolationKind = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, kind);
    }

    // --- ViolationDispositionProfile ---

    #[test]
    fn disposition_profile_constructs_with_valid_fields() {
        let profile = ViolationDispositionProfile {
            investigation_duration_ticks: NonZeroU32::new(3).unwrap(),
            violation_memory_retention_ticks: 50,
            investigation_motive_weight: Permille::new(500).unwrap(),
            ownership_motive_bonus: Permille::new(200).unwrap(),
        };

        assert_eq!(profile.investigation_duration_ticks.get(), 3);
        assert_eq!(profile.violation_memory_retention_ticks, 50);
        assert_eq!(profile.investigation_motive_weight, Permille::new(500).unwrap());
        assert_eq!(profile.ownership_motive_bonus, Permille::new(200).unwrap());
    }

    #[test]
    fn disposition_profile_serde_roundtrip() {
        let profile = ViolationDispositionProfile {
            investigation_duration_ticks: NonZeroU32::new(5).unwrap(),
            violation_memory_retention_ticks: 30,
            investigation_motive_weight: Permille::new(700).unwrap(),
            ownership_motive_bonus: Permille::new(100).unwrap(),
        };
        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: ViolationDispositionProfile = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, profile);
    }

    // --- ViolationKind Ord is deterministic ---

    #[test]
    fn violation_kind_ord_is_stable() {
        let mut kinds = vec![sample_dead(), sample_missing(), sample_depleted()];
        kinds.sort();
        let mut kinds2 = kinds.clone();
        kinds2.sort();
        assert_eq!(kinds, kinds2);
    }
}
