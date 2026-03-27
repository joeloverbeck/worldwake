# S31-003: Implement `condition_changed` Detection Logic

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI exhaustion module
**Deps**: S31-001, S31-002

## Problem

The invalidation system needs a function that compares a single `ExhaustionInvalidationCondition` against the current agent belief state and baseline snapshot to determine whether the world has changed in ways that warrant re-searching the exhausted goal. This is the core delta-detection engine.

## Assumption Reassessment (2026-03-27)

1. `GoalBeliefView` already exposes the exact query surface this helper needs: `effective_place`, `commodity_quantity`, `unique_item_count`, `wounds`, `homeostatic_needs`, `visible_hostiles_for`, and `is_alive` in [`crates/worldwake-sim/src/belief_view.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs). No new belief-view API belongs in this ticket.
2. The live code already contains `ExhaustionInvalidationCondition`, `ExhaustionBaseline`, `derive_invalidation_conditions`, `build_baseline`, and `need_value` in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs). The original ticket narrative was stale: S31-003 is not introducing those types or mappings anymore, it is filling the missing `condition_changed` evaluation layer that S31-004 will consume.
3. The shared abstraction boundary under audit is the exhaustion invalidation contract: `ExhaustionInvalidationCondition` plus `ExhaustionBaseline` in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs). This ticket stays at that boundary and must not silently broaden into planner-loop rewiring or save-format policy changes.
4. `PositionChanged` must preserve the existing transit semantics from the current runtime invalidation path in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs): mid-transit waypoint churn is not a lawful reason to re-search. The condition only fires when `currently_in_transit == false` and the settled effective place differs from the baseline.
5. Reassessment after S31-002 still stands: `FacilitiesChanged` and `BlockerExpired` cannot use unconditional `true` defaults. Doing so would invalidate every exhausted goal carrying those conditions on the next planning pass, collapsing the cache for `Sleep`, `Wash`, `ProduceCommodity`, `ClaimOffice`, `SupportCandidateForOffice`, and `PunishAccused`. This helper must instead consume runtime-derived booleans passed from the shared runtime observation layer in S31-004.
6. `NeedCrossedThreshold` should use absolute delta against the stored baseline value. Signed direction is not part of the condition contract.
7. `CommodityChanged` must treat "not present in the baseline snapshot" as "baseline zero" for invalidation purposes: any current non-zero quantity is a change, while current zero remains unchanged.
8. `UniqueItemChanged` is part of the live enum and baseline shape even though the original ticket under-specified it. Exhaustive detection now requires explicit handling and focused coverage for that branch too.
9. `TargetDead` should continue using `view.is_alive()`, which already covers both actual death and entity purge from authoritative state.
10. The active S31 spec has stale persistence assumptions relative to current code: `SAVE_FORMAT_VERSION` is already `7` in [`crates/worldwake-sim/src/save_load.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs), not `6`. That mismatch is real but outside this ticket's implementation scope because S31-003 does not own save/load boundaries.

## Architecture Check

1. A pure `condition_changed` function with an exhaustive match over the live `ExhaustionInvalidationCondition` enum is cleaner than leaving the semantics implicit in future callers. It creates one canonical evaluation point for all later invalidation paths and keeps detection testable without planner-loop setup.
2. Facility and blocker invalidation must remain state-mediated through the existing runtime observation contract, not guessed locally inside `condition_changed`. Passing precomputed booleans from the runtime keeps the function deterministic without duplicating facility-signature or blocker-cleanup logic in the belief layer.
3. This targeted helper is more beneficial than the current architecture because the current architecture still relies on coarse dirty-domain clearing plus TTL. Completing the condition evaluator is a necessary step toward the cleaner per-goal invalidation design in `specs/S31-goal-aware-exhaustion-invalidation.md`, while still keeping this ticket surgically scoped.
4. Ideal-architecture note: the remaining `#[serde(default)]` compatibility fallback on the new exhaustion fields in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) does not match the repository's no-backward-compatibility rule. That should be removed as part of the broader S31 persistence cleanup, not hidden inside this helper ticket.

## Verification Layers

1. Position change detection (with transit filtering) -> unit test
2. Commodity quantity delta detection -> unit test
3. Unique-item count delta detection -> unit test
4. Need threshold crossing detection -> unit test
5. Wound count change detection -> unit test
6. Hostile count change detection -> unit test
7. Target death detection -> unit test
8. Facility dirty signal gating for `FacilitiesChanged` -> unit test
9. Blocker cleanup signal gating for `BlockerExpired` -> unit test
10. Single-layer ticket. No planner-loop wiring, trace assertions, or golden coverage belongs here yet.

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

- `invalidate_exhausted_goals` function (S31-004) and planner-loop call-site rewiring
- Recording conditions/baselines at exhaustion time (already largely in place; follow-on validation belongs with S31-005)
- Removing TTL and simplifying the planner skip predicate (S31-006)
- Golden tests and mixed-layer behavioral proof (S31-007 / later integrated verification)
- Save-format or serde-policy cleanup, including removing backward-compatibility defaults from exhaustion runtime structs
- Redesigning the runtime dirty observation model itself

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `PositionChanged` returns `false` when `currently_in_transit == true`
2. Unit test: `PositionChanged` returns `true` when settled and position differs from baseline
3. Unit test: `PositionChanged` returns `false` when settled and position matches baseline
4. Unit test: `CommodityChanged(Bread)` returns `true` when quantity differs from baseline
5. Unit test: `CommodityChanged(Bread)` returns `false` when quantity matches baseline
6. Unit test: `CommodityChanged(Bread)` returns `true` when commodity absent from baseline and current quantity > 0
7. Unit test: `UniqueItemChanged(SimpleTool)` returns `true` when count differs and `false` when it matches
8. Unit test: `WoundsChanged` returns `true` when wound count differs
9. Unit test: `NeedCrossedThreshold { Fatigue, Permille(100) }` returns `true` when delta >= 100
10. Unit test: `NeedCrossedThreshold { Fatigue, Permille(100) }` returns `false` when delta < 100
11. Unit test: `HostilesChanged` returns `true` when hostile count differs
12. Unit test: `TargetDead(entity)` returns `true` when entity is not alive
13. Unit test: `FacilitiesChanged` returns the passed runtime dirty signal
14. Unit test: `BlockerExpired` returns the passed runtime dirty signal
15. Focused crate suite and workspace verification commands pass

### Invariants

1. `condition_changed` is a pure function of its inputs — no side effects
2. `PositionChanged` never fires mid-transit (preserves existing waypoint filtering behavior)
3. Facility and blocker invalidation only fire when the runtime observation layer reported the corresponding domain change
4. Exhaustive matching covers every current invalidation condition variant without introducing new belief-view or runtime side channels

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exhaustion.rs` (inline `#[cfg(test)]`) — unit tests covering every condition variant plus runtime signal passthrough for `FacilitiesChanged` and `BlockerExpired`

### Commands

1. `cargo test -p worldwake-ai exhaustion::tests`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

## Outcome

- Completed: 2026-03-27
- What actually changed:
  - Added the pure `condition_changed` evaluator in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs).
  - Added focused unit coverage for position, commodity, unique-item, wound, hostile, need-threshold, target-death, and runtime-gated facility/blocker branches.
  - Reassessed and corrected the ticket narrative to match live code: the invalidation data types and derivation path were already implemented before this ticket.
- Deviations from original plan:
  - The original ticket assumed S31-003 still needed to introduce pieces that already existed. The delivered scope was narrowed to the missing evaluator and its tests.
  - No planner-loop rewiring, TTL removal, or save-format cleanup was folded into this ticket. Those remain separate architectural steps.
  - Added dead-code suppression on the staged exhaustion helpers so `cargo clippy --workspace` stays clean while later S31 tickets finish the runtime wiring.
- Verification results:
  - `cargo test -p worldwake-ai exhaustion::tests` passed.
  - `cargo test -p worldwake-ai` passed.
  - `cargo clippy --workspace` passed.
  - `cargo test --workspace` passed.
