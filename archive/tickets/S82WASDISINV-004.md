# S82WASDISINV-004: Add PlannerOpKind::DropItem, GoalDispatchKey, and GoalDispatchDeclaration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new PlannerOpKind variant, live GoalDispatchDeclaration wiring for disposal
**Deps**: S82WASDISINV-001, S82WASDISINV-002

## Problem

The planner still lacks `PlannerOpKind::DropItem` and a live disposal dispatch declaration. `S82WASDISINV-001` already landed `GoalDispatchKey::FreeCarryCapacity` and an inert `DECL_FREE_CARRY_CAPACITY`; this ticket finishes the planner infrastructure so `FreeCarryCapacity` can route to a disposal-motivated drop action.

## Assumption Reassessment (2026-04-10)

1. `PlannerOpKind` enum at `planner_ops.rs:13-53` has 39 variants. `DropItem` does not exist. `classify_action_def()` at line 88 maps `(ActionDomain, name)` pairs. Currently `(ActionDomain::Transport, "pick_up" | "put_down" | "steal")` maps to `MoveCargo` at line 112.
2. `S82WASDISINV-001` already landed `GoalDispatchKey::FreeCarryCapacity`, added it to `ALL`, mapped `GoalKind::FreeCarryCapacity`, and added compile-safe downstream exhaustive handling. This ticket now owns the remaining live disposal routing, not dispatch-key introduction.
3. `GoalDispatchDeclaration` struct at `goal_dispatch_decl.rs:47-59` has all 7 required fields. `S82WASDISINV-001` already added an inert `DECL_FREE_CARRY_CAPACITY` with empty `relevant_ops` / `progress_barrier_ops`. This ticket must replace that inert declaration with the live `DropItem` wiring.
4. `PlannerTransitionKind::PutDownGroundLot` exists at `planner_ops.rs:69`. Used by `put_down` at line 216.
5. `PlannerOpSemantics` struct at lines 56-61 has `op_kind`, `may_appear_mid_plan`, `is_materialization_barrier`, `transition_kind`.

## Architecture Check

1. Standard pattern: add the new op kind and make the existing inert disposal dispatch declaration live. Follows the existing pattern for every GoalKind (e.g., `LootCorpse`, `BuryCorpse`, `Patrol`).
2. Reusing `PlannerTransitionKind::PutDownGroundLot` avoids duplication — the hypothetical state effect is identical to `put_down`.
3. No backward-compatibility shims.

## Verification Layers

1. `classify_action_def` maps `"drop_item"` to `PlannerOpKind::DropItem` -> focused unit test
2. `DECL_FREE_CARRY_CAPACITY` exposes live `DropItem` ops and barrier wiring -> focused unit test reading declaration via `.declaration()`
3. Single-layer ticket (planner infrastructure) — no cross-system verification needed

## What to Change

### 1. PlannerOpKind variant

In `crates/worldwake-ai/src/planner_ops.rs`, add `DropItem` to the enum.

### 2. classify_action_def mapping

In `classify_action_def()`, add before the existing Transport arm:

```rust
(ActionDomain::Transport, "drop_item") => Some(PlannerOpKind::DropItem),
```

### 3. PlannerOpSemantics

Add semantics for `DropItem` using `PlannerTransitionKind::PutDownGroundLot`:

```rust
PlannerOpKind::DropItem => PlannerOpSemantics {
    op_kind: PlannerOpKind::DropItem,
    may_appear_mid_plan: false,
    is_materialization_barrier: true,
    transition_kind: PlannerTransitionKind::PutDownGroundLot,
},
```

### 4. GoalDispatchDeclaration

In `crates/worldwake-ai/src/goal_dispatch_decl.rs`:

```rust
const FREE_CARRY_OPS: &[PlannerOpKind] = &[PlannerOpKind::DropItem];
const FREE_CARRY_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::DropItem];

const DECL_FREE_CARRY_CAPACITY: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "FreeCarryCapacity",
    provenance_family: None,
    relevant_ops: FREE_CARRY_OPS,
    invalidation_strategy: InvalidationStrategy::NoOpinion,
    feasibility_strategy: FeasibilityStrategy::AlwaysLikely,
    family_policy: SELF_CARE_POLICY,
    progress_barrier_ops: FREE_CARRY_BARRIER,
};
```

Replace the inert `DECL_FREE_CARRY_CAPACITY` from `S82WASDISINV-001` with live `DropItem` op/barrier wiring. `GoalDispatchKey::FreeCarryCapacity` already exists and should be reused, not reintroduced.

### 5. Exhaustive match sites

Add `PlannerOpKind::DropItem` arms to exhaustive matches in the AI crate and update any disposal dispatch/declaration sites that still treat `FreeCarryCapacity` as inert scaffolding.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- Any files with exhaustive matches on `PlannerOpKind` or live disposal dispatch declarations (modify)

## Out of Scope

- GoalKindPlannerExt implementation (ticket 005)
- Ranking integration (ticket 006)
- Candidate generation (ticket 007)
- Handler implementation (already in ticket 002)

## Acceptance Criteria

### Tests That Must Pass

1. `classify_action_def` returns `Some(PlannerOpKind::DropItem)` for a Transport domain action named `"drop_item"`
2. `DECL_FREE_CARRY_CAPACITY` has live `DropItem` `relevant_ops` and `progress_barrier_ops`
3. Declaration has correct `trace_label`, `relevant_ops`, `invalidation_strategy`, `feasibility_strategy`, `family_policy`
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `GoalDispatchKey::FreeCarryCapacity` remains the canonical dispatch identity for the disposal goal
2. Every `GoalKind` variant has a corresponding `GoalDispatchKey` mapping
3. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` (test module) — test `classify_action_def` for `drop_item`
2. `crates/worldwake-ai/src/goal_dispatch_decl.rs` (test module) — test declaration field values and live `DropItem` wiring

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed: 2026-04-10

- Added `PlannerOpKind::DropItem` in `crates/worldwake-ai/src/planner_ops.rs`, mapped `Transport/"drop_item"` in `classify_action_def()`, and gave it live `PlannerOpSemantics` using `PlannerTransitionKind::PutDownGroundLot`.
- Replaced inert `DECL_FREE_CARRY_CAPACITY` op wiring in `crates/worldwake-ai/src/goal_dispatch_decl.rs` with live `DropItem` `relevant_ops` / `progress_barrier_ops`.
- Landed required additive-fallout handling for the new planner op in `crates/worldwake-ai/src/agent_tick/observation.rs`, `crates/worldwake-ai/src/failure_handling.rs`, and `crates/worldwake-ai/src/goal_model.rs` so exhaustive `PlannerOpKind` consumers remain compile-safe and behaviorally aligned.
- Added focused tests for `drop_item` classification and semantics plus declaration wiring, and updated synthetic candidate / search assertions to reflect that both `put_down` and `drop_item` now lawfully synthesize `PutDownGroundLot` planner-only candidates.

Deviations from original plan:

- The ticket remained single-crate AI work, but broadened verification exposed additional in-scope fallout beyond the original file list. Because planner-only candidate synthesis keys off shared `PlannerTransitionKind::PutDownGroundLot`, making `DropItem` live also required updating synthetic candidate and search expectations rather than only declaration/exhaustive-match sites.

Verification:

- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
