//! Typed invalidation domain set for agent replan diagnostics.
//!
//! `DirtySet` is a hand-rolled bitflag newtype over `u16` that tracks
//! which domains triggered a replan. It is the single authoritative
//! representation for invalidation domains in both runtime logic and traces.

use std::fmt;
use std::ops::{BitOr, BitOrAssign};

/// A set of invalidation domains encoded as a `u16` bitfield.
///
/// Three domain categories:
/// - **Structural** (bits 0–5): plan lifecycle events (no plan, plan finished, etc.)
/// - **Snapshot** (bits 6–11): observation dimensions (position, needs, wounds, etc.)
/// - **Frame** (bits 12–14): frame lifecycle events introduced by S22
///
/// Bit 15 is reserved and unused.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DirtySet(u16);

// ---------------------------------------------------------------------------
// Structural bits (0–5)
// ---------------------------------------------------------------------------

impl DirtySet {
    /// Agent has no plan at all.
    pub const NO_PLAN: DirtySet = DirtySet(1 << 0);
    /// Current plan finished executing.
    pub const PLAN_FINISHED: DirtySet = DirtySet(1 << 1);
    /// Explicit replan signal from the engine.
    pub const REPLAN_SIGNAL: DirtySet = DirtySet(1 << 2);
    /// Action queue transitioned (e.g. step completed).
    pub const QUEUE_TRANSITION: DirtySet = DirtySet(1 << 3);
    /// A blocker was cleaned up, freeing a previously blocked goal.
    pub const BLOCKER_CLEANUP: DirtySet = DirtySet(1 << 4);
    /// Queue patience exhausted — agent waited too long.
    pub const QUEUE_PATIENCE: DirtySet = DirtySet(1 << 5);

    // -----------------------------------------------------------------------
    // Snapshot bits (6–11)
    // -----------------------------------------------------------------------

    /// Agent's effective place changed.
    pub const POSITION: DirtySet = DirtySet(1 << 6);
    /// Homeostatic needs changed.
    pub const NEEDS: DirtySet = DirtySet(1 << 7);
    /// Wound state changed.
    pub const WOUNDS: DirtySet = DirtySet(1 << 8);
    /// Commodity inventory signature changed.
    pub const COMMODITY: DirtySet = DirtySet(1 << 9);
    /// Unique item inventory changed.
    pub const UNIQUE_ITEMS: DirtySet = DirtySet(1 << 10);
    /// Facility access signature changed.
    pub const FACILITIES: DirtySet = DirtySet(1 << 11);

    // -----------------------------------------------------------------------
    // Frame lifecycle bits (12–14)
    // -----------------------------------------------------------------------

    /// Frame blocked (S22 frame lifecycle).
    pub const FRAME_BLOCKAGE: DirtySet = DirtySet(1 << 12);
    /// Frame patience exhausted (S22 frame lifecycle).
    pub const FRAME_PATIENCE: DirtySet = DirtySet(1 << 13);
    /// Frame assumption failed (S22 frame lifecycle).
    pub const ASSUMPTION_FAILED: DirtySet = DirtySet(1 << 14);

    // -----------------------------------------------------------------------
    // Aggregate masks
    // -----------------------------------------------------------------------

    /// All structural bits (0–5).
    pub const STRUCTURAL_MASK: DirtySet =
        DirtySet((1 << 0) | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4) | (1 << 5));

    /// All snapshot bits (6–11).
    pub const SNAPSHOT_MASK: DirtySet =
        DirtySet((1 << 6) | (1 << 7) | (1 << 8) | (1 << 9) | (1 << 10) | (1 << 11));

    /// All frame lifecycle bits (12–14).
    pub const FRAME_MASK: DirtySet = DirtySet((1 << 12) | (1 << 13) | (1 << 14));

    // -----------------------------------------------------------------------
    // Methods
    // -----------------------------------------------------------------------

    /// Returns `true` if no invalidation domains are set.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Inserts all bits from `other` into this set.
    #[inline]
    pub fn insert(&mut self, other: DirtySet) {
        self.0 |= other.0;
    }

    /// Returns `true` if this set contains all bits in `other`.
    #[inline]
    pub fn contains(self, other: DirtySet) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns `true` if this set shares any bit with `other`.
    #[inline]
    pub fn contains_any(self, other: DirtySet) -> bool {
        (self.0 & other.0) != 0
    }

    /// Returns `true` if the set is non-empty and contains only snapshot
    /// domain bits (no structural or frame bits).
    #[inline]
    pub fn is_snapshot_only(self) -> bool {
        !self.is_empty() && (self.0 & !Self::SNAPSHOT_MASK.0) == 0
    }

    /// Returns a human-readable list of set domain names, pipe-separated.
    /// Returns `"CLEAN"` when the set is empty.
    pub fn display_names(self) -> String {
        const NAMES: &[(u16, &str)] = &[
            (1 << 0, "NO_PLAN"),
            (1 << 1, "PLAN_FINISHED"),
            (1 << 2, "REPLAN_SIGNAL"),
            (1 << 3, "QUEUE_TRANSITION"),
            (1 << 4, "BLOCKER_CLEANUP"),
            (1 << 5, "QUEUE_PATIENCE"),
            (1 << 6, "POSITION"),
            (1 << 7, "NEEDS"),
            (1 << 8, "WOUNDS"),
            (1 << 9, "COMMODITY"),
            (1 << 10, "UNIQUE_ITEMS"),
            (1 << 11, "FACILITIES"),
            (1 << 12, "FRAME_BLOCKAGE"),
            (1 << 13, "FRAME_PATIENCE"),
            (1 << 14, "ASSUMPTION_FAILED"),
        ];

        if self.is_empty() {
            return "CLEAN".to_string();
        }

        let mut parts = Vec::new();
        for &(bit, name) in NAMES {
            if self.0 & bit != 0 {
                parts.push(name);
            }
        }
        parts.join("|")
    }
}

// ---------------------------------------------------------------------------
// Trait implementations
// ---------------------------------------------------------------------------

impl BitOr for DirtySet {
    type Output = DirtySet;

    #[inline]
    fn bitor(self, rhs: DirtySet) -> DirtySet {
        DirtySet(self.0 | rhs.0)
    }
}

impl BitOrAssign for DirtySet {
    #[inline]
    fn bitor_assign(&mut self, rhs: DirtySet) {
        self.0 |= rhs.0;
    }
}

impl fmt::Display for DirtySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display_names())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_set_default_is_empty() {
        assert!(DirtySet::default().is_empty());
    }

    #[test]
    fn dirty_set_insert_sets_bits() {
        let mut set = DirtySet::default();
        set.insert(DirtySet::NEEDS);
        assert!(!set.is_empty());
        assert!(set.contains(DirtySet::NEEDS));
    }

    #[test]
    fn dirty_set_is_snapshot_only_true_for_snapshot_bits() {
        let set = DirtySet::NEEDS | DirtySet::POSITION;
        assert!(set.is_snapshot_only());
    }

    #[test]
    fn dirty_set_is_snapshot_only_false_with_structural() {
        let set = DirtySet::NEEDS | DirtySet::NO_PLAN;
        assert!(!set.is_snapshot_only());
    }

    #[test]
    fn dirty_set_is_snapshot_only_false_with_frame() {
        let set = DirtySet::NEEDS | DirtySet::FRAME_BLOCKAGE;
        assert!(!set.is_snapshot_only());
    }

    #[test]
    fn dirty_set_contains_checks_individual_and_combined() {
        let set = DirtySet::NEEDS | DirtySet::WOUNDS;
        assert!(set.contains(DirtySet::NEEDS));
        assert!(set.contains(DirtySet::WOUNDS));
        assert!(set.contains(DirtySet::NEEDS | DirtySet::WOUNDS));
        assert!(!set.contains(DirtySet::POSITION));
    }

    #[test]
    fn dirty_set_display_names_empty_shows_clean() {
        assert_eq!(DirtySet::default().display_names(), "CLEAN");
    }

    #[test]
    fn dirty_set_display_names_single() {
        assert_eq!(DirtySet::NO_PLAN.display_names(), "NO_PLAN");
    }

    #[test]
    fn dirty_set_display_names_multiple() {
        let set = DirtySet::NEEDS | DirtySet::POSITION;
        assert_eq!(set.display_names(), "POSITION|NEEDS");
    }

    #[test]
    fn dirty_set_snapshot_mask_covers_six_bits() {
        let mask = DirtySet::SNAPSHOT_MASK;
        // Contains all 6 snapshot bits.
        assert!(mask.contains(DirtySet::POSITION));
        assert!(mask.contains(DirtySet::NEEDS));
        assert!(mask.contains(DirtySet::WOUNDS));
        assert!(mask.contains(DirtySet::COMMODITY));
        assert!(mask.contains(DirtySet::UNIQUE_ITEMS));
        assert!(mask.contains(DirtySet::FACILITIES));
        // Does not contain structural or frame bits.
        assert!(!mask.contains(DirtySet::NO_PLAN));
        assert!(!mask.contains(DirtySet::FRAME_BLOCKAGE));
    }

    #[test]
    fn dirty_set_frame_mask_covers_three_bits() {
        let mask = DirtySet::FRAME_MASK;
        assert!(mask.contains(DirtySet::FRAME_BLOCKAGE));
        assert!(mask.contains(DirtySet::FRAME_PATIENCE));
        assert!(mask.contains(DirtySet::ASSUMPTION_FAILED));
        // Does not contain structural or snapshot bits.
        assert!(!mask.contains(DirtySet::NO_PLAN));
        assert!(!mask.contains(DirtySet::NEEDS));
    }

    #[test]
    fn dirty_set_structural_mask_covers_six_bits() {
        let mask = DirtySet::STRUCTURAL_MASK;
        assert!(mask.contains(DirtySet::NO_PLAN));
        assert!(mask.contains(DirtySet::PLAN_FINISHED));
        assert!(mask.contains(DirtySet::REPLAN_SIGNAL));
        assert!(mask.contains(DirtySet::QUEUE_TRANSITION));
        assert!(mask.contains(DirtySet::BLOCKER_CLEANUP));
        assert!(mask.contains(DirtySet::QUEUE_PATIENCE));
        // Does not contain snapshot or frame bits.
        assert!(!mask.contains(DirtySet::NEEDS));
        assert!(!mask.contains(DirtySet::FRAME_BLOCKAGE));
    }

    #[test]
    fn dirty_set_bitor_combines() {
        let set = DirtySet::NEEDS | DirtySet::POSITION;
        assert!(set.contains(DirtySet::NEEDS));
        assert!(set.contains(DirtySet::POSITION));
    }

    #[test]
    fn dirty_set_bitor_assign_combines() {
        let mut set = DirtySet::NEEDS;
        set |= DirtySet::POSITION;
        assert!(set.contains(DirtySet::NEEDS));
        assert!(set.contains(DirtySet::POSITION));
    }
}
