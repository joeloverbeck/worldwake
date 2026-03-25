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

#[derive(
    Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
)]
pub struct ViolationId(pub u64);

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
    /// Investigation confirmed a suspected theft of an owned entity.
    SuspectedTheft {
        missing_entity: EntityId,
        expected_place: EntityId,
        suspect: Option<EntityId>,
    },
}

/// A single recorded violation with expiry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecordedViolation {
    pub id: ViolationId,
    pub kind: ViolationKind,
    pub observed_tick: Tick,
    pub resolved_tick: Option<Tick>,
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
    pub next_violation_id: ViolationId,
}

impl Component for ViolationMemory {}

impl ViolationMemory {
    /// Returns `true` if an unexpired record exists for this violation kind.
    pub fn is_recorded(&self, kind: &ViolationKind, current_tick: Tick) -> bool {
        self.recorded_by_kind(kind, current_tick).is_some()
    }

    /// Records a violation, replacing any existing entry for the same kind.
    /// TTL is in ticks (added to `observed_tick` to compute `expires_tick`).
    pub fn record(&mut self, kind: ViolationKind, observed_tick: Tick, ttl: u32) -> ViolationId {
        if let Some(index) = self.violations.iter().position(|record| record.kind == kind) {
            let new_expires_tick = Tick(observed_tick.0 + u64::from(ttl));
            if self.violations[index].expires_tick > observed_tick {
                self.violations[index].observed_tick = observed_tick;
                self.violations[index].expires_tick = new_expires_tick;
                return self.violations[index].id;
            }

            let id = self.allocate_id();
            self.violations[index].id = id;
            self.violations[index].observed_tick = observed_tick;
            self.violations[index].resolved_tick = None;
            self.violations[index].expires_tick = new_expires_tick;
            return id;
        }

        let id = self.allocate_id();
        self.violations.push(RecordedViolation {
            id,
            kind,
            observed_tick,
            resolved_tick: None,
            expires_tick: Tick(observed_tick.0 + u64::from(ttl)),
        });
        id
    }

    /// Removes all entries where `current_tick >= expires_tick`.
    pub fn expire(&mut self, current_tick: Tick) {
        self.violations.retain(|r| r.expires_tick > current_tick);
    }

    #[must_use]
    pub const fn next_violation_id(&self) -> ViolationId {
        self.next_violation_id
    }

    pub fn recorded_by_kind(
        &self,
        kind: &ViolationKind,
        current_tick: Tick,
    ) -> Option<&RecordedViolation> {
        self.violations
            .iter()
            .find(|r| &r.kind == kind && r.expires_tick > current_tick)
    }

    pub fn unresolved_by_id(
        &self,
        id: ViolationId,
        current_tick: Tick,
    ) -> Option<&RecordedViolation> {
        self.violations
            .iter()
            .find(|r| r.id == id && r.expires_tick > current_tick && r.resolved_tick.is_none())
    }

    pub fn unresolved_records(&self, current_tick: Tick) -> Vec<&RecordedViolation> {
        self.violations
            .iter()
            .filter(|record| record.expires_tick > current_tick && record.resolved_tick.is_none())
            .collect()
    }

    pub fn resolve_id(&mut self, id: ViolationId, resolved_tick: Tick, ttl: u32) -> bool {
        let Some(record) = self
            .violations
            .iter_mut()
            .find(|record| record.id == id && record.expires_tick > resolved_tick)
        else {
            return false;
        };
        record.resolved_tick = Some(resolved_tick);
        record.expires_tick = Tick(resolved_tick.0 + u64::from(ttl));
        true
    }

    fn allocate_id(&mut self) -> ViolationId {
        let id = self.next_violation_id;
        self.next_violation_id = ViolationId(self.next_violation_id.0 + 1);
        id
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

    fn sample_suspected_theft(suspect: Option<EntityId>) -> ViolationKind {
        ViolationKind::SuspectedTheft {
            missing_entity: entity(4),
            expected_place: entity(10),
            suspect,
        }
    }

    // --- ViolationMemory::record deduplicates by kind ---

    #[test]
    fn record_replaces_existing_entry_for_same_kind() {
        let mut mem = ViolationMemory::default();
        let kind = sample_missing();

        let first_id = mem.record(kind.clone(), Tick(5), 10);
        assert_eq!(mem.violations.len(), 1);
        assert_eq!(mem.violations[0].id, first_id);
        assert_eq!(mem.violations[0].observed_tick, Tick(5));

        // Recording again replaces
        let second_id = mem.record(kind.clone(), Tick(8), 10);
        assert_eq!(mem.violations.len(), 1);
        assert_eq!(second_id, first_id);
        assert_eq!(mem.violations[0].observed_tick, Tick(8));
        assert_eq!(mem.violations[0].expires_tick, Tick(18));
    }

    #[test]
    fn record_different_kinds_accumulate() {
        let mut mem = ViolationMemory::default();
        let missing_id = mem.record(sample_missing(), Tick(1), 10);
        let depleted_id = mem.record(sample_depleted(), Tick(2), 10);
        let dead_id = mem.record(sample_dead(), Tick(3), 10);
        let theft_id = mem.record(sample_suspected_theft(Some(entity(8))), Tick(4), 10);
        assert_eq!(mem.violations.len(), 4);
        assert_ne!(missing_id, depleted_id);
        assert_ne!(missing_id, dead_id);
        assert_ne!(depleted_id, dead_id);
        assert_ne!(dead_id, theft_id);
    }

    #[test]
    fn record_rediscovered_expired_kind_gets_new_id() {
        let mut mem = ViolationMemory::default();
        let kind = sample_missing();

        let first_id = mem.record(kind.clone(), Tick(5), 5);
        let second_id = mem.record(kind, Tick(10), 5);

        assert_ne!(first_id, second_id);
        assert_eq!(mem.violations.len(), 1);
        assert_eq!(mem.violations[0].id, second_id);
        assert_eq!(mem.violations[0].observed_tick, Tick(10));
    }

    #[test]
    fn next_violation_id_advances_monotonically() {
        let mut mem = ViolationMemory::default();
        assert_eq!(mem.next_violation_id(), ViolationId(0));

        let _ = mem.record(sample_missing(), Tick(1), 10);
        let _ = mem.record(sample_depleted(), Tick(2), 10);

        assert_eq!(mem.next_violation_id(), ViolationId(2));
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

    #[test]
    fn active_by_id_returns_live_record() {
        let mut mem = ViolationMemory::default();
        let id = mem.record(sample_missing(), Tick(5), 10);

        assert_eq!(
            mem.unresolved_by_id(id, Tick(6)).map(|record| record.id),
            Some(id)
        );
        assert!(mem.unresolved_by_id(id, Tick(15)).is_none());
    }

    #[test]
    fn resolve_id_marks_record_resolved_and_extends_expiry() {
        let mut mem = ViolationMemory::default();
        let id = mem.record(sample_missing(), Tick(5), 10);

        assert!(mem.resolve_id(id, Tick(9), 10));
        assert!(mem.unresolved_by_id(id, Tick(10)).is_none());
        assert_eq!(mem.violations[0].resolved_tick, Some(Tick(9)));
        assert_eq!(mem.violations[0].expires_tick, Tick(19));
    }

    #[test]
    fn resolve_id_fails_for_expired_record() {
        let mut mem = ViolationMemory::default();
        let id = mem.record(sample_missing(), Tick(5), 5);

        assert!(!mem.resolve_id(id, Tick(10), 5));
    }

    #[test]
    fn resolved_record_remains_recorded_until_expiry() {
        let mut mem = ViolationMemory::default();
        let kind = sample_missing();
        let id = mem.record(kind.clone(), Tick(5), 10);

        assert!(mem.resolve_id(id, Tick(8), 10));
        assert!(mem.is_recorded(&kind, Tick(9)));
        assert!(mem.is_recorded(&kind, Tick(17)));
        assert!(!mem.is_recorded(&kind, Tick(18)));
    }

    #[test]
    fn unresolved_records_exclude_resolved_entries() {
        let mut mem = ViolationMemory::default();
        let first = mem.record(sample_missing(), Tick(5), 10);
        let _second = mem.record(sample_depleted(), Tick(6), 10);

        assert!(mem.resolve_id(first, Tick(8), 10));

        let unresolved = mem.unresolved_records(Tick(9));
        assert_eq!(unresolved.len(), 1);
        assert_eq!(unresolved[0].kind, sample_depleted());
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

    #[test]
    fn suspected_theft_roundtrips_with_known_suspect() {
        let kind = sample_suspected_theft(Some(entity(9)));
        let bytes = bincode::serialize(&kind).unwrap();
        let roundtrip: ViolationKind = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, kind);
    }

    #[test]
    fn suspected_theft_roundtrips_with_unknown_suspect() {
        let kind = sample_suspected_theft(None);
        let bytes = bincode::serialize(&kind).unwrap();
        let roundtrip: ViolationKind = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, kind);
    }

    #[test]
    fn suspected_theft_records_and_retrieves_from_violation_memory() {
        let mut mem = ViolationMemory::default();
        let kind = sample_suspected_theft(None);
        let id = mem.record(kind.clone(), Tick(7), 11);

        let record = mem.recorded_by_kind(&kind, Tick(10)).unwrap();
        assert_eq!(record.id, id);
        assert_eq!(record.kind, kind);
        assert_eq!(record.observed_tick, Tick(7));
        assert_eq!(record.expires_tick, Tick(18));
    }

    #[test]
    fn violation_kind_ord_is_stable_with_suspected_theft() {
        let mut variants = vec![
            sample_suspected_theft(Some(entity(8))),
            sample_dead(),
            sample_depleted(),
            sample_missing(),
            sample_suspected_theft(None),
        ];

        variants.sort();

        assert_eq!(
            variants,
            vec![
                sample_missing(),
                sample_depleted(),
                sample_dead(),
                sample_suspected_theft(None),
                sample_suspected_theft(Some(entity(8))),
            ]
        );
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
