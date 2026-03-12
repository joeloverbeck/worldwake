# HARHYPENTIDE-001: CommitOutcome type and ActionCommitFn signature change

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — action handler contract (`worldwake-sim`), all handler implementations (`worldwake-systems`)
**Deps**: None (foundation ticket — all others depend on this)
**Spec Reference**: HARDENING-hypothetical-entity-identity.md, Section D.1–D.3

## Problem

Actions whose commit path creates new authoritative entities (e.g. partial `pick_up` splitting a lot) have no way to report those creations back to the caller. The current `ActionCommitFn` returns `Result<(), ActionError>`, discarding entity-creation information that the planner runtime will need for hypothetical-to-authoritative binding.

## Assumption Reassessment (2026-03-12)

1. `ActionCommitFn` is defined in `crates/worldwake-sim/src/action_handler.rs:21` as `-> Result<(), ActionError>` — confirmed.
2. `ActionHandler` struct at line 38 stores `on_commit: ActionCommitFn` — confirmed.
3. All handler implementations across `worldwake-systems` (`commit_pick_up`, `commit_put_down`, `commit_eat`, `commit_drink`, `commit_sleep`, `commit_toilet`, `commit_wash`, `commit_travel`, `commit_harvest`, `commit_craft`, `commit_trade`, `commit_attack`, `commit_defend`, `commit_loot`, `commit_heal`) return `Result<(), ActionError>` — confirmed.
4. `tick_action` in `worldwake-sim/src/tick_action.rs` calls `(handler.on_commit)(...)` and discards the unit result — confirmed.
5. The test helper `noop_commit` in `action_handler.rs` tests and `planning_state.rs` tests also returns `Result<(), ActionError>` — confirmed.

## Architecture Check

1. This is a backward-incompatible signature change by design (Principle 13: no compatibility shims). Every handler must update.
2. `CommitOutcome::empty()` is the zero-cost default for non-materializing handlers — no behavior change for them.
3. The `MaterializationTag` enum is designed to grow; only `SplitOffLot` is added now. Future tags (e.g. `CraftOutput`, `HarvestYield`) will be added when those systems need materialization tracking.

## What to Change

### 1. Add `CommitOutcome`, `Materialization`, and `MaterializationTag` types in `worldwake-sim`

```rust
pub struct CommitOutcome {
    pub materializations: Vec<Materialization>,
}

pub struct Materialization {
    pub tag: MaterializationTag,
    pub entity: EntityId,
}

pub enum MaterializationTag {
    SplitOffLot,
}

impl CommitOutcome {
    pub fn empty() -> Self { Self { materializations: Vec::new() } }
}
```

Place these in `crates/worldwake-sim/src/action_handler.rs` alongside the existing handler types. Export from `lib.rs`.

### 2. Change `ActionCommitFn` signature

From: `-> Result<(), ActionError>`
To: `-> Result<CommitOutcome, ActionError>`

### 3. Update `tick_action` call site

In `crates/worldwake-sim/src/tick_action.rs`, the call to `(handler.on_commit)(...)` currently discards the result. Update to propagate `CommitOutcome` through the `TickOutcome` or return path so the caller can access materializations.

### 4. Update all handler implementations in `worldwake-systems`

Every `commit_*` function must change its return type from `Result<(), ActionError>` to `Result<CommitOutcome, ActionError>` and return `Ok(CommitOutcome::empty())` instead of `Ok(())`.

Affected files and functions:
- `transport_actions.rs`: `commit_pick_up`, `commit_put_down`
- `needs_actions.rs`: `commit_eat`, `commit_drink`, `commit_noop` (sleep), `commit_toilet`, `commit_wash`
- `travel_actions.rs`: `commit_travel`
- `production_actions.rs`: `commit_harvest`, `commit_craft`
- `trade_actions.rs`: `commit_trade`
- `combat.rs`: `commit_attack`, `commit_defend`, `commit_loot`, `commit_heal`

### 5. Update test helpers

All test `noop_commit` functions in `action_handler.rs` tests, `planning_state.rs` tests, and `planner_ops.rs` tests must return `CommitOutcome::empty()`.

## Files to Touch

- `crates/worldwake-sim/src/action_handler.rs` (modify — new types + signature change)
- `crates/worldwake-sim/src/lib.rs` (modify — export new types)
- `crates/worldwake-sim/src/tick_action.rs` (modify — propagate `CommitOutcome`)
- `crates/worldwake-systems/src/transport_actions.rs` (modify)
- `crates/worldwake-systems/src/needs_actions.rs` (modify)
- `crates/worldwake-systems/src/travel_actions.rs` (modify)
- `crates/worldwake-systems/src/production_actions.rs` (modify)
- `crates/worldwake-systems/src/trade_actions.rs` (modify)
- `crates/worldwake-systems/src/combat.rs` (modify)

## Out of Scope

- Actual materialization data in `commit_pick_up` (that is HARHYPENTIDE-006)
- Planner identity model (`HypotheticalEntityId`, `PlanningEntityRef`)
- `MaterializationBindings` runtime table
- Any changes to `worldwake-ai`
- Any changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. All existing handler tests in `transport_actions.rs`, `needs_actions.rs`, `travel_actions.rs`, `production_actions.rs`, `trade_actions.rs`, `combat.rs` still pass with the new return type.
2. `action_handler_hooks_are_callable` test in `action_handler.rs` passes.
3. `action_handler_on_commit_can_mutate_world_through_world_txn` test passes.
4. New unit test: `CommitOutcome::empty()` returns zero materializations.
5. New unit test: `CommitOutcome` with `Materialization { tag: SplitOffLot, entity }` is constructible and inspectable.
6. Existing suite: `cargo test --workspace`
7. Existing lint: `cargo clippy --workspace`

### Invariants

1. No handler behavior changes — all non-materializing handlers return `CommitOutcome::empty()`.
2. No backward-compatibility aliases (`Result<(), ActionError>` path is fully replaced).
3. `CommitOutcome`, `Materialization`, `MaterializationTag` derive `Clone, Debug, Eq, PartialEq` for testability.
4. All action execution tests continue to pass unchanged (semantic behavior is identical).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_handler.rs` — unit tests for `CommitOutcome` construction and `MaterializationTag` variants.
2. All existing handler tests — updated return type expectations (mechanical change).

### Commands

1. `cargo test -p worldwake-sim action_handler`
2. `cargo test -p worldwake-systems`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
