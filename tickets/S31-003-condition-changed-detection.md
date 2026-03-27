# S31-003: Implement `condition_changed` Detection Logic

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI exhaustion module
**Deps**: S31-001, S31-002

## Problem

The invalidation system needs a function that compares a single `ExhaustionInvalidationCondition` against the current agent belief state and baseline snapshot to determine whether the world has changed in ways that warrant re-searching the exhausted goal. This is the core delta-detection engine.

## Assumption Reassessment (2026-03-27)

1. `GoalBeliefView` provides all required query methods: `effective_place`, `commodity_quantity`, `unique_item_count`, `wounds`, `homeostatic_needs`, `visible_hostiles_for`, `is_alive` — verified at `crates/worldwake-sim/src/belief_view.rs:29-70`.
2. `PositionChanged` must filter mid-transit waypoints per spec Section 5: only fires when `currently_in_transit == false` and position differs from baseline.
3. Reassessment after S31-002: `FacilitiesChanged` and `BlockerExpired` cannot safely use unconditional `true` defaults. Doing so would invalidate every exhausted goal carrying those conditions on the next planning pass, collapsing the cache for `Sleep`, `Wash`, `ProduceCommodity`, `ClaimOffice`, `SupportCandidateForOffice`, and `PunishAccused`. This ticket must instead consume runtime-derived change signals sourced from `DirtySet::FACILITIES` and `DirtySet::BLOCKER_CLEANUP` (or equivalent precomputed booleans) at the `invalidate_exhausted_goals` call boundary.
4. `NeedCrossedThreshold` uses absolute difference >= `threshold_delta` — no signed comparison needed, just magnitude.
5. `CommodityChanged` must handle the case where the commodity wasn't in the baseline (any non-zero quantity is a change).
6. `TargetDead` uses `view.is_alive()` which covers both death and entity purge.
7. `need_value` helper was added in S31-002.

## Architecture Check

1. Pure function with exhaustive match over `ExhaustionInvalidationCondition`. Clean, testable, no side effects.
2. Facility and blocker invalidation must remain state-mediated through the existing runtime observation contract, not guessed locally inside `condition_changed`. Passing precomputed booleans from the runtime keeps the function deterministic without duplicating facility-signature or blocker-cleanup logic in the belief layer.

## Verification Layers

1. Position change detection (with transit filtering) -> unit test
2. Commodity quantity delta detection -> unit test
3. Need threshold crossing detection -> unit test
4. Wound count change detection -> unit test
5. Hostile count change detection -> unit test
6. Target death detection -> unit test
7. Facility dirty signal gating for `FacilitiesChanged` -> unit test
8. Blocker cleanup signal gating for `BlockerExpired` -> unit test
8. Single-layer ticket (detection logic only). No integration with planning pipeline yet.

## What to Change

### 1. Add `condition_changed` to `crates/worldwake-ai/src/exhaustion.rs`

```rust
pub(crate) fn condition_changed(
    condition: &ExhaustionInvalidationCondition,
    baseline: &ExhaustionBaseline,
    view: &dyn GoalBeliefView,
    agent: EntityId,
    currently_in_transit: bool,
    facilities_changed: bool,
    blocker_expired: bool,
) -> bool
```

Implement per the spec Section 6 logic:
- `PositionChanged`: false if in transit; compare `view.effective_place(agent)` vs `baseline.position`
- `CommodityChanged(kind)`: compare current quantity vs baseline for that kind; non-zero when absent from baseline = change
- `UniqueItemChanged(kind)`: compare current count vs baseline for that kind
- `WoundsChanged`: compare `view.wounds(agent).len()` vs `baseline.wound_count`
- `FacilitiesChanged`: return the runtime-derived `facilities_changed` signal
- `BlockerExpired`: return the runtime-derived `blocker_expired` signal
- `HostilesChanged`: compare `view.visible_hostiles_for(agent).len()` vs `baseline.hostile_count`
- `NeedCrossedThreshold { need, threshold_delta }`: absolute difference between current and baseline need value >= threshold_delta
- `TargetDead(target)`: `!view.is_alive(target)`

## Files to Touch

- `crates/worldwake-ai/src/exhaustion.rs` (modify)

## Out of Scope

- `invalidate_exhausted_goals` function (S31-004) — this ticket is just the per-condition check
- Wiring into the planning pipeline (S31-004, S31-005)
- Removing TTL (S31-006)
- Golden tests (S31-007)
- Redesigning the runtime dirty observation model itself

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `PositionChanged` returns `false` when `currently_in_transit == true`
2. Unit test: `PositionChanged` returns `true` when settled and position differs from baseline
3. Unit test: `PositionChanged` returns `false` when settled and position matches baseline
4. Unit test: `CommodityChanged(Bread)` returns `true` when quantity differs from baseline
5. Unit test: `CommodityChanged(Bread)` returns `false` when quantity matches baseline
6. Unit test: `CommodityChanged(Bread)` returns `true` when commodity absent from baseline and current quantity > 0
7. Unit test: `WoundsChanged` returns `true` when wound count differs
8. Unit test: `NeedCrossedThreshold { Fatigue, Permille(100) }` returns `true` when delta >= 100
9. Unit test: `NeedCrossedThreshold { Fatigue, Permille(100) }` returns `false` when delta < 100
10. Unit test: `HostilesChanged` returns `true` when hostile count differs
11. Unit test: `TargetDead(entity)` returns `true` when entity is not alive
12. Unit test: `FacilitiesChanged` returns the passed runtime dirty signal
13. Unit test: `BlockerExpired` returns the passed runtime dirty signal
14. Existing suite: `cargo test --workspace`

### Invariants

1. `condition_changed` is a pure function of its inputs — no side effects
2. `PositionChanged` never fires mid-transit (preserves existing waypoint filtering behavior)
3. Facility and blocker invalidation only fire when the runtime observation layer reported the corresponding domain change

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exhaustion.rs` (inline `#[cfg(test)]`) — unit tests covering every condition variant plus runtime signal passthrough for `FacilitiesChanged` and `BlockerExpired`

### Commands

1. `cargo test -p worldwake-ai exhaustion`
2. `cargo clippy --workspace && cargo test --workspace`
