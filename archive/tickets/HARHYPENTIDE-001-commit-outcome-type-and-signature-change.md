# HARHYPENTIDE-001: CommitOutcome type and ActionCommitFn signature change

**Status**: COMPLETED
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
3. All action commit handlers in `worldwake-systems` still return `Result<(), ActionError>`, but the concrete affected set is broader than the original draft implies. Confirmed commit handlers are:
   - `transport_actions.rs`: `commit_pick_up`, `commit_put_down`
   - `needs_actions.rs`: `commit_noop`, `commit_eat`, `commit_drink`, `commit_toilet`, `commit_wash`
   - `travel_actions.rs`: `commit_travel`
   - `production_actions.rs`: `commit_harvest`, `commit_craft`
   - `trade_actions.rs`: `commit_trade`
   - `combat.rs`: `commit_defend`, `commit_loot`, `commit_attack`, `commit_heal`
4. `tick_action` in `worldwake-sim/src/tick_action.rs` currently calls `(handler.on_commit)(...)`, commits the world transaction, and collapses the successful path to `TickOutcome::Committed`. There is no existing caller-visible commit payload.
5. The helper/test blast radius is wider than the original draft. In addition to `action_handler.rs` and `planning_state.rs`, commit-signature helpers currently exist in:
   - `crates/worldwake-ai/src/plan_revalidation.rs`
   - `crates/worldwake-sim/src/action_handler_registry.rs`
   - `crates/worldwake-sim/src/affordance_query.rs`
   - `crates/worldwake-sim/src/interrupt_abort.rs`
   - `crates/worldwake-sim/src/start_gate.rs`
   - `crates/worldwake-sim/src/tick_action.rs`
   - `crates/worldwake-sim/src/tick_step.rs`
6. The current codebase already has authoritative commit paths that materialize new entities beyond partial `pick_up`, including:
   - `commit_harvest` and `commit_craft` creating output lots
   - `commit_toilet` creating a waste lot
   - `commit_trade` materializing split-off lots during partial bundle transfer
   - `commit_loot` materializing split-off lots during partial corpse loot
   This ticket should not solve all semantic tagging for those paths, but the contract change must not box the architecture into a `pick_up`-only model.

## Architecture Check

1. This is a backward-incompatible signature change by design (Principle 13: no compatibility shims). Every handler must update.
2. The commit contract is only useful if it reaches a caller. Returning `CommitOutcome` from handlers but immediately discarding it in `tick_action` would preserve the current blind architecture under a different type name. This ticket therefore must thread `CommitOutcome` through `TickOutcome::Committed`.
3. `CommitOutcome::empty()` remains the zero-cost default for non-materializing handlers and for materializing handlers whose semantic tagging is intentionally deferred by later tickets.
4. `MaterializationTag` must be additive and future-proof. The initial implementation should at minimum support the split-off lot case required by the hardening spec, but the surrounding types and enum shape must stay generic enough to absorb harvest/craft/trade/loot materializations without another contract redesign.
5. The cleanest architecture is to keep the execution layer authoritative and typed:
   - handlers emit authoritative `CommitOutcome`
   - `tick_action` surfaces it as part of `TickOutcome::Committed`
   - higher layers may ignore it for now, but they must not lose access to it

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

### 3. Update `tick_action` and committed return path

In `crates/worldwake-sim/src/tick_action.rs`, the call to `(handler.on_commit)(...)` currently discards the result. Update `TickOutcome::Committed` to carry `CommitOutcome` so the caller can access materializations without introducing side channels or scheduler-local caches.

This requires corresponding mechanical updates anywhere `TickOutcome::Committed` is matched or asserted, including `tick_step.rs` and the relevant unit/integration tests.

### 4. Update all handler implementations in `worldwake-systems`

Every `commit_*` function must change its return type from `Result<(), ActionError>` to `Result<CommitOutcome, ActionError>` and return `Ok(CommitOutcome::empty())` instead of `Ok(())`.

Affected files and functions:
- `transport_actions.rs`: `commit_pick_up`, `commit_put_down`
- `needs_actions.rs`: `commit_eat`, `commit_drink`, `commit_noop` (sleep), `commit_toilet`, `commit_wash`
- `travel_actions.rs`: `commit_travel`
- `production_actions.rs`: `commit_harvest`, `commit_craft`
- `trade_actions.rs`: `commit_trade`
- `combat.rs`: `commit_attack`, `commit_defend`, `commit_loot`, `commit_heal`

### 5. Update helper/test commit shims

All helper/test commit functions that currently return `Result<(), ActionError>` must return `CommitOutcome::empty()`. This includes `action_handler.rs`, `planning_state.rs`, `plan_revalidation.rs`, `action_handler_registry.rs`, `affordance_query.rs`, `interrupt_abort.rs`, `start_gate.rs`, `tick_action.rs`, and `tick_step.rs`.

## Files to Touch

- `crates/worldwake-sim/src/action_handler.rs` (modify — new types + signature change)
- `crates/worldwake-sim/src/lib.rs` (modify — export new types)
- `crates/worldwake-sim/src/tick_action.rs` (modify — propagate `CommitOutcome`)
- `crates/worldwake-sim/src/tick_step.rs` (modify — adapt `TickOutcome::Committed` handling)
- `crates/worldwake-sim/src/action_handler_registry.rs` (modify — test helper signature updates)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — test helper signature updates)
- `crates/worldwake-sim/src/interrupt_abort.rs` (modify — test helper signature updates)
- `crates/worldwake-sim/src/start_gate.rs` (modify — test helper signature updates)
- `crates/worldwake-systems/src/transport_actions.rs` (modify)
- `crates/worldwake-systems/src/needs_actions.rs` (modify)
- `crates/worldwake-systems/src/travel_actions.rs` (modify)
- `crates/worldwake-systems/src/production_actions.rs` (modify)
- `crates/worldwake-systems/src/trade_actions.rs` (modify)
- `crates/worldwake-systems/src/combat.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify — test helper signature updates)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — test helper signature updates)

## Out of Scope

- Actual materialization data in `commit_pick_up` (that is HARHYPENTIDE-006)
- Semantic materialization tagging for existing harvest/craft/toilet/trade/loot entity creation paths unless needed to keep the contract internally coherent
- Planner identity model (`HypotheticalEntityId`, `PlanningEntityRef`)
- `MaterializationBindings` runtime table
- Any changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. All existing handler tests in `transport_actions.rs`, `needs_actions.rs`, `travel_actions.rs`, `production_actions.rs`, `trade_actions.rs`, `combat.rs` still pass with the new return type.
2. `action_handler_hooks_are_callable` test in `action_handler.rs` passes.
3. `action_handler_on_commit_can_mutate_world_through_world_txn` test passes.
4. New unit test: `CommitOutcome::empty()` returns zero materializations.
5. New unit test: `CommitOutcome` with `Materialization { tag: SplitOffLot, entity }` is constructible and inspectable.
6. New or updated unit test: successful `tick_action` commit returns `TickOutcome::Committed` carrying the handler-provided `CommitOutcome`.
7. Existing suite: `cargo test --workspace`
8. Existing lint: `cargo clippy --workspace`

### Invariants

1. No handler behavior changes — all non-materializing handlers return `CommitOutcome::empty()`.
2. No backward-compatibility aliases (`Result<(), ActionError>` path is fully replaced).
3. `CommitOutcome`, `Materialization`, `MaterializationTag` derive `Clone, Debug, Eq, PartialEq` for testability.
4. `TickOutcome::Committed` preserves access to the emitted `CommitOutcome`; the commit payload is not discarded in `tick_action`.
5. All action execution tests continue to pass unchanged apart from the committed-outcome shape.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_handler.rs` — unit tests for `CommitOutcome` construction and `MaterializationTag` variants.
2. `crates/worldwake-sim/src/tick_action.rs` — verify a commit outcome emitted by a handler is surfaced by `TickOutcome::Committed`.
3. Existing handler and scheduler-adjacent tests — updated committed-outcome pattern matches and helper signatures.

### Commands

1. `cargo test -p worldwake-sim action_handler`
2. `cargo test -p worldwake-systems`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-12
- Actual changes:
  - Added `CommitOutcome`, `Materialization`, and `MaterializationTag` to `worldwake-sim` and exported them from `lib.rs`.
  - Changed `ActionCommitFn` from `Result<(), ActionError>` to `Result<CommitOutcome, ActionError>`.
  - Changed `TickOutcome::Committed` to carry the emitted `CommitOutcome` so commit results are not discarded at the scheduler boundary.
  - Updated all current action commit handlers in `worldwake-systems` to return `CommitOutcome::empty()`.
  - Updated commit helper shims in `worldwake-sim` and `worldwake-ai` tests to the new signature.
  - Added unit coverage for `CommitOutcome::empty()`, materialization construction, and `tick_action` returning the handler-supplied commit outcome.
- Deviations from original plan:
  - The ticket scope was corrected before implementation to reflect the real blast radius, including `tick_step.rs` and multiple helper/test modules that were omitted from the original draft.
  - Existing entity-materializing commits outside transport (`harvest`, `craft`, `toilet`, partial `trade`, partial `loot`) still return `CommitOutcome::empty()`. This ticket established the execution contract and propagation path only; semantic tagging for those handlers remains future work.
  - No `worldwake-ai` production logic changed, but `worldwake-ai` test helpers were updated because they depended on the old commit signature.
- Verification results:
  - `cargo test -p worldwake-sim action_handler` passed
  - `cargo test -p worldwake-systems` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
