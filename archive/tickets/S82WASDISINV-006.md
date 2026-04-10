# S82WASDISINV-006: Add FreeCarryCapacity ranking integration

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new match arms in ranking functions
**Deps**: S82WASDISINV-001

## Problem

`S82WASDISINV-001` already added compile-safe ranking coverage for `GoalKind::FreeCarryCapacity`, including a low-priority inert branch and zero motive score so the shared enum variant compiles without making disposal behavior live. This ticket now owns replacing that inert ranking with the real capacity-strain motive model.

## Assumption Reassessment (2026-04-10)

1. `priority_class()` and `motive_score()` already have `FreeCarryCapacity` arms from `S82WASDISINV-001`, but the current behavior is intentionally inert (`GoalPriorityClass::Low`, `motive_score == 0`) pending this ticket.
2. The owned delta is now semantic, not compile coverage: replace the inert motive behavior with the real capacity-strain computation while preserving low priority class.
3. `GoalPriorityClass::Low` exists at `goal_model.rs:2000`. Used by `LootCorpse`, `BuryCorpse`, `Patrol`, `ExploreLocation`, etc.
4. `GoalBeliefView::carry_capacity()` exists at `belief_view.rs:188`, but `GoalBeliefView::load_of_entity()` at `belief_view.rs:189` is not a lawful carried-load surface for agents: the runtime belief view forwards it to `worldwake_core::load_of_entity()`, which returns `LoadUnits(0)` for non-item entities such as agents. This ticket cannot use the spec's original `load_of_entity(agent)` arithmetic as written.
5. The live belief-view surface already exposes enough information to derive carried load lawfully for ranking: `commodity_quantity(agent, kind)` on `GoalBeliefView` plus `worldwake_core::load_per_unit(kind)`. This keeps the motive model on concrete carried inventory rather than a stale zero-load helper.
6. `score_product()` utility used throughout ranking module for combining weight and pressure.
7. The `ranking.rs` test harness `TestBeliefView` currently returns `None` for both `carry_capacity()` and `load_of_entity()` in its `GoalBeliefView` impl. This ticket therefore owns adding minimal test-view fields/wiring so focused ranking tests can prove the live capacity-strain motive path.

## Architecture Check

1. Standard pattern: refine the existing `priority_class()` and `motive_score()` arms for `FreeCarryCapacity` from inert scaffolding to live ranking behavior.
2. Motive score uses capacity strain as pressure, scaled by `enterprise_weight` — consistent with how other enterprise-adjacent goals use utility weights. Because the live `load_of_entity(agent)` surface is not carried load for agents, this ticket should derive carried load from concrete held commodity quantities rather than preserve the stale spec helper call.
3. No backward-compatibility shims.

## Verification Layers

1. `priority_class` returns `Low` for FreeCarryCapacity -> focused unit test
2. `motive_score` increases with capacity strain -> focused unit test with varying load levels
3. `motive_score` returns 0 when carry capacity unavailable -> focused unit test
4. Single-layer ticket (ranking only) — no cross-system verification needed

## What to Change

### 1. priority_class match arm

In `crates/worldwake-ai/src/ranking.rs`, in `priority_class()`:

```rust
GoalKind::FreeCarryCapacity => GoalPriorityClass::Low,
```

Add to the existing `Low` group alongside `LootCorpse`, `BuryCorpse`, etc.

### 2. motive_score match arm

In `motive_score()`:

```rust
GoalKind::FreeCarryCapacity => {
    let Some(carry_cap) = context.view.carry_capacity(context.agent) else { return 0 };
    let carried_load = CommodityKind::ALL
        .iter()
        .copied()
        .map(|kind| context.view.commodity_quantity(context.agent, kind).0 * worldwake_core::load_per_unit(kind).0)
        .sum::<u32>();
    let strain = Permille::new_unchecked(
        (carried_load * 1000 / u32::from(carry_cap.0).max(1)).min(1000) as u16
    );
    score_product(context.utility.enterprise_weight, strain)
}
```

### 3. ranking test harness parity

In `crates/worldwake-ai/src/ranking.rs` test support, add minimal `TestBeliefView` storage/wiring for `carry_capacity()` and `load_of_entity()` so focused `FreeCarryCapacity` ranking tests can exercise non-`None` load/capacity cases without changing production behavior.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify)

## Out of Scope

- Candidate generation (ticket 007)
- GoalKindPlannerExt (ticket 005)
- Golden tests (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. `priority_class` returns `GoalPriorityClass::Low` for `FreeCarryCapacity`
2. `motive_score` returns non-zero when carried commodity load is high relative to capacity
3. `motive_score` returns 0 when carry capacity is not available
4. `motive_score` scales proportionally with capacity strain
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `FreeCarryCapacity` ranking is live rather than inert: low priority class remains, but motive is driven by concrete carried commodity load relative to capacity instead of a stub `0`
2. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` (test module) — test priority class for FreeCarryCapacity
2. `crates/worldwake-ai/src/ranking.rs` (test module) — test motive score at various carried-load strain levels (0%, 50%, 80%, 100%)

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed: 2026-04-10

- Implemented live `FreeCarryCapacity` ranking in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) by replacing the inert zero-motive branch with capacity-strain scoring that uses `enterprise_weight`.
- Derived carried load from concrete held commodity quantities via `GoalBeliefView::commodity_quantity()` and `worldwake_core::load_per_unit()` instead of using `GoalBeliefView::load_of_entity(agent)`, which is not a lawful carried-load surface for agents.
- Added focused ranking coverage for low-priority classification, strain scaling, unavailable capacity, and proof that motive uses commodity-derived carried load rather than the agent load accessor.
- Extended the ranking test harness with minimal carry-capacity and entity-load storage so the focused ranking tests can exercise non-`None` cases.

Deviations from original plan:

- The ticket was corrected during reassessment because the original helper choice was stale: `load_of_entity(agent)` exists but resolves to intrinsic entity load and returns zero for non-item entities such as agents.
- The completed implementation therefore uses commodity-derived carried load rather than the spec's original `load_of_entity(agent)` arithmetic.

Verification results:

- `cargo fmt --all`
- `cargo test -p worldwake-ai free_carry_capacity_` -> passed
- `cargo test -p worldwake-ai claim_office_uses_enterprise_weight_and_medium_priority` -> passed
- `cargo test -p worldwake-ai` -> passed
- `cargo clippy --workspace --all-targets -- -D warnings` -> passed
