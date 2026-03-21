# S11WOULIFAUD-001: WoundList deprivation lookup methods

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `WoundList` API extension in worldwake-core
**Deps**: None

## Problem

The spec requires worsening existing deprivation wounds instead of creating duplicates (deliverable B). That logic needs to find an existing wound by `DeprivationKind`. No such lookup exists on `WoundList` today.

## Assumption Reassessment (2026-03-21)

1. `WoundList` in `crates/worldwake-core/src/wounds.rs` has `next_wound_id()`, `wound_load()`, `wound_ids()`, `has_bleeding_wounds()`. No `find_deprivation_wound` or `find_deprivation_wound_mut` exists.
2. `WoundCause::Deprivation(DeprivationKind)` variant exists (wounds.rs line 44). `DeprivationKind` is the key for lookup.
3. Not an AI ticket — pure data-structure extension.
4. No ordering dependency.
5. N/A — no heuristic removal.
6. N/A.
7. N/A.
8. N/A.
9. N/A.
10. No mismatch. Spec matches codebase.

## Architecture Check

1. Placing lookup methods directly on `WoundList` is the natural location — it owns `wounds: Vec<Wound>` and already has query methods (`has_bleeding_wounds`, `wound_load`). No alternative is cleaner.
2. No backwards-compatibility shims. Pure addition.

## Verification Layers

1. `find_deprivation_wound` returns correct match → focused unit test
2. `find_deprivation_wound_mut` allows mutation → focused unit test
3. Empty list returns `None` → focused unit test
4. Combat wounds are not matched → focused unit test
5. Single-layer ticket (core data structure). No AI/action/event layers involved.

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

Three tests as specified below in acceptance criteria.

## Files to Touch

- `crates/worldwake-core/src/wounds.rs` (modify)

## Out of Scope

- Changing `append_deprivation_wound` in needs.rs (that is S11WOULIFAUD-003)
- Any changes to combat.rs wound progression
- Any AI/ranking changes
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

1. `crates/worldwake-core/src/wounds.rs::tests::find_deprivation_wound_returns_match` — verifies correct match and miss
2. `crates/worldwake-core/src/wounds.rs::tests::find_deprivation_wound_mut_updates_severity` — verifies mutation through mutable ref
3. `crates/worldwake-core/src/wounds.rs::tests::find_deprivation_wound_returns_none_for_empty_list` — edge case

### Commands

1. `cargo test -p worldwake-core -- find_deprivation`
2. `cargo clippy -p worldwake-core`
3. `cargo test -p worldwake-core`
