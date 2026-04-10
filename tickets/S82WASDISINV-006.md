# S82WASDISINV-006: Add FreeCarryCapacity ranking integration

**Status**: PENDING
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
4. `carry_capacity()` and `load_of_entity()` accessors exist on `GoalBeliefView` at `belief_view.rs:187-188`. Already used in `candidate_generation.rs:3051-3054`.
5. `score_product()` utility used throughout ranking module for combining weight and pressure.

## Architecture Check

1. Standard pattern: refine the existing `priority_class()` and `motive_score()` arms for `FreeCarryCapacity` from inert scaffolding to live ranking behavior.
2. Motive score uses capacity strain as pressure, scaled by `enterprise_weight` — consistent with how other enterprise-adjacent goals use utility weights.
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
    let Some(load) = context.view.load_of_entity(context.agent) else { return 0 };
    let strain = Permille::new_unchecked(
        (u32::from(load.0) * 1000 / u32::from(carry_cap.0).max(1)).min(1000) as u16
    );
    score_product(context.utility.enterprise_weight, strain)
}
```

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify)

## Out of Scope

- Candidate generation (ticket 007)
- GoalKindPlannerExt (ticket 005)
- Golden tests (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. `priority_class` returns `GoalPriorityClass::Low` for `FreeCarryCapacity`
2. `motive_score` returns non-zero when load is high relative to capacity
3. `motive_score` returns 0 when carry capacity is not available
4. `motive_score` scales proportionally with capacity strain
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `FreeCarryCapacity` ranking is live rather than inert: low priority class remains, but motive is driven by capacity strain instead of a stub `0`
2. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` (test module) — test priority class for FreeCarryCapacity
2. `crates/worldwake-ai/src/ranking.rs` (test module) — test motive score at various strain levels (0%, 50%, 80%, 100%)

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
