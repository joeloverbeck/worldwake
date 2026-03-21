# S11WOULIFAUD-001: WoundList deprivation lookup methods

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `WoundList` API extension in worldwake-core
**Deps**: None

## Problem

The spec requires worsening existing deprivation wounds instead of creating duplicates (deliverable B). That logic needs to find an existing wound by `DeprivationKind`. No such lookup exists on `WoundList` today.

## Assumption Reassessment (2026-03-21)

1. `crates/worldwake-core/src/wounds.rs` currently exposes `WoundList::next_wound_id()`, `wound_load()`, `wound_ids()`, and `has_bleeding_wounds()`. It does not expose `find_deprivation_wound` or `find_deprivation_wound_mut`, so downstream code currently has no canonical deprivation-specific lookup surface on the authoritative wound container.
2. `WoundCause::Deprivation(DeprivationKind)` is the exact current authoritative discriminator in `crates/worldwake-core/src/wounds.rs`; `DeprivationKind` remains the correct lookup key for starvation vs dehydration wounds.
3. Existing focused coverage already spans the downstream behavior this helper is meant to support: `needs::tests::needs_system_adds_starvation_wound_and_resets_hunger_exposure`, `needs::tests::needs_system_adds_dehydration_wound_and_resets_thirst_exposure`, `needs::tests::needs_system_requires_another_full_tolerance_period_before_second_wound` in `crates/worldwake-systems/src/needs.rs`; `combat::tests::non_bleeding_wounds_recover_when_physiology_is_tolerable`, `combat::tests::recovery_is_blocked_when_physiology_exceeds_tolerable_thresholds`, and `combat::tests::healed_wounds_are_removed_from_wound_list` in `crates/worldwake-systems/src/combat.rs`; and AI/golden wound-priority coverage in `crates/worldwake-ai/src/ranking.rs` plus `crates/worldwake-ai/tests/golden_emergent.rs`. This ticket still remains core-only because it adds a missing authoritative lookup primitive rather than changing those behaviors.
4. No ordering dependency exists in this ticket. The contract is a deterministic data lookup on `WoundList`, not action lifecycle, event-log, or planner ordering.
5. No heuristic is being removed or weakened. This is an API addition to the authoritative wound container only.
6. N/A — not a stale-request, contested-affordance, or start-failure ticket.
7. N/A — not a political office-claim ticket.
8. N/A — no `ControlSource`, queued-input, or runtime-intent manipulation.
9. No scenario isolation choice is needed because this is focused unit coverage in `worldwake-core`, not a golden scenario.
10. Mismatch corrected: `specs/S11-wound-lifecycle-audit.md` is broader than this ticket and contains at least one stale assumption relative to the current codebase. In particular, the spec claims the `no_recovery_combat_profile()` workaround no longer exists, but it is still present in `crates/worldwake-ai/tests/golden_harness/mod.rs` and used by `crates/worldwake-ai/tests/golden_emergent.rs`. This ticket should therefore stay narrowly scoped to the core `WoundList` helper required by the deprivation-worsening work, not claim that the broader spec narrative fully matches current code.

## Architecture Check

1. Placing the lookup on `WoundList` is cleaner than re-implementing deprivation scans in `worldwake-systems/src/needs.rs` or any future care/combat callers. `WoundList` already owns the authoritative storage and query surface, so deprivation-specific lookup belongs with the collection rather than as an ad hoc system helper.
2. A more generic predicate-based search API would be broader than the current need and would invite call-site-specific filtering logic back into downstream layers. The domain-specific deprivation lookup is the smaller, more robust extension.
3. No backwards-compatibility shims or alias paths are introduced; this is a pure additive API.

## Verification Layers

1. Immutable lookup returns the matching deprivation wound and ignores non-matching deprivation kinds -> focused unit test in `crates/worldwake-core/src/wounds.rs`
2. Mutable lookup updates the existing wound in place without changing its `WoundId` -> focused unit test in `crates/worldwake-core/src/wounds.rs`
3. Combat wounds are never returned by deprivation lookup -> focused unit test in `crates/worldwake-core/src/wounds.rs`
4. Empty lists return `None` for both helper variants -> focused unit test in `crates/worldwake-core/src/wounds.rs`
5. Single-layer ticket: no additional AI/action/event-log verification layer is applicable because no runtime behavior changes in this ticket.

## What to Change

### 1. Add `find_deprivation_wound()` to `WoundList`

```rust
/// Returns an immutable reference to the first wound caused by the given DeprivationKind.
pub fn find_deprivation_wound(&self, kind: DeprivationKind) -> Option<&Wound> {
    self.wounds.iter().find(|w| matches!(w.cause, WoundCause::Deprivation(k) if k == kind))
}
```

### 2. Add `find_deprivation_wound_mut()` to `WoundList`

```rust
/// Returns a mutable reference to the first wound caused by the given DeprivationKind.
pub fn find_deprivation_wound_mut(&mut self, kind: DeprivationKind) -> Option<&mut Wound> {
    self.wounds.iter_mut().find(|w| matches!(w.cause, WoundCause::Deprivation(k) if k == kind))
}
```

### 3. Add unit tests in `wounds.rs` `mod tests`

Add focused coverage for immutable lookup, mutable lookup preserving identity, and empty-list behavior. The immutable lookup test must also prove that combat wounds are ignored.

## Files to Touch

- `crates/worldwake-core/src/wounds.rs` (modify)

## Out of Scope

- Changing `append_deprivation_wound` / introducing `worsen_or_create_deprivation_wound` in `crates/worldwake-systems/src/needs.rs` (that is the downstream deprivation-worsening work, not this prerequisite helper ticket)
- Any changes to combat.rs wound progression
- Any AI/ranking changes
- Refreshing or correcting the broader `specs/S11-wound-lifecycle-audit.md` narrative beyond what is necessary to scope this ticket accurately
- Adding or modifying golden tests

## Acceptance Criteria

### Tests That Must Pass

1. `find_deprivation_wound_returns_match` — list with starvation + combat wounds; find starvation succeeds, find dehydration returns None
2. `find_deprivation_wound_mut_updates_severity` — find mutable ref, modify severity via `saturating_add`, assert list reflects the change and WoundId is unchanged
3. `find_deprivation_wound_returns_none_for_empty_list` — default WoundList, both methods return None
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `WoundList` public API is a pure extension — no existing method signatures change
2. `find_deprivation_wound` matches only `WoundCause::Deprivation(kind)`, never `WoundCause::Combat`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/wounds.rs::tests::find_deprivation_wound_returns_match` — verifies deprivation-kind match, miss for another kind, and non-match against combat wounds
2. `crates/worldwake-core/src/wounds.rs::tests::find_deprivation_wound_mut_updates_severity` — verifies in-place mutation through the helper while preserving the original `WoundId`
3. `crates/worldwake-core/src/wounds.rs::tests::find_deprivation_wound_returns_none_for_empty_list` — verifies the empty-list edge case for both immutable and mutable helpers

### Commands

1. `cargo test -p worldwake-core -- find_deprivation`
2. `cargo clippy -p worldwake-core`
3. `cargo test -p worldwake-core`

## Outcome

- Completion date: 2026-03-21
- Actually changed: added `WoundList::find_deprivation_wound()` and `WoundList::find_deprivation_wound_mut()` in `crates/worldwake-core/src/wounds.rs`, plus focused unit coverage for immutable lookup, mutable in-place mutation, empty-list behavior, and combat-wound exclusion.
- Deviation from original plan: the implementation matched the intended core API addition, but the ticket itself was first corrected to reflect real current coverage and the stale broader-spec assumption about `no_recovery_combat_profile()`. No systems, combat, or AI code was changed in this ticket.
- Verification results: `cargo test -p worldwake-core find_deprivation_wound`, `cargo clippy -p worldwake-core --all-targets -- -D warnings`, and `cargo test -p worldwake-core` all passed.
