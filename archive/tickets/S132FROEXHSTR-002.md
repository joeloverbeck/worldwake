# S132FROEXHSTR-002: Route frontier exhaustion recording through strategy dispatch

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes - `worldwake-ai` planner exhaustion recording
**Deps**: `archive/tickets/S132FROEXHSTR-001.md`, `archive/specs/S132-frontier-exhaustion-strategy.md`

## Problem

After `S132FROEXHSTR-001` declares frontier-exhaustion strategy with each goal dispatch declaration, the runtime still needs to stop using the local `GoalKind` allow-list in `frontier_exhaustion_entry`. The helper should read the declared strategy and construct the same `ExhaustionEntry` shapes as today.

## Assumption Reassessment (2026-05-01)

1. The live runtime path is `crates/worldwake-ai/src/agent_tick/planning.rs::record_exhausted_goals`, which calls `frontier_exhaustion_entry` when a plan result is `PlanSearchResult::FrontierExhausted`.
2. The current `frontier_exhaustion_entry` allow-list maps `GoalKind::Sleep`, self-consume `GoalKind::AcquireCommodity`, and `GoalKind::Patrol` to `ExhaustionEntry::budget_retry_pending`; every other `GoalKind` maps to `ExhaustionEntry::frontier_exhausted`.
3. Existing focused tests already prove the current runtime behavior for `Sleep`, self-consume acquisition, `Patrol`, and generic frontier suppression in `crates/worldwake-ai/src/agent_tick/planning.rs`.
4. This ticket depends on `S132FROEXHSTR-001` adding `FrontierExhaustionStrategy` and `GoalDispatchDeclaration.frontier_exhaustion_strategy`. The shared data contract under audit is `GoalDispatchKey::from_goal_kind(goal).declaration().frontier_exhaustion_strategy`.
5. The live `GoalKind` operators under test are the existing `record_exhausted_goals` synthetic opportunities. This ticket does not change `get_affordances`, `generate_candidates`, `search_plan`, or authoritative action start.
6. This is not a golden-driven change. The intended invariant is lower-level: `FrontierExhausted` recording dispatches from declared goal metadata, while resulting `ExhaustionRetryState`, `next_retry_tick`, and `suppresses_planning()` behavior remain unchanged.
7. No timing contract changes. Existing `BudgetRetryPending` retry eligibility and `next_retry_tick` calculation remain owned by `ExhaustionEntry::budget_retry_pending`.
8. No stale-request, contested-affordance, or start-failure boundary is involved. The first behavior boundary is planner exhaustion recording, not request resolution or authoritative start.
9. No information-path refactor. The strategy affects only agent-local planner runtime state in `AgentDecisionRuntime.exhaustion_cache`.
10. Adjacent contradiction classification: if a preserved-default goal now records `BudgetRetryPending`, that is a regression caused by this ticket. If future behavior argues that another recurring goal should retry on cooldown, that is a separate strategy-declaration ticket.

## Architecture Check

1. Reading `frontier_exhaustion_strategy` from `GoalDispatchDeclaration` removes the downstream variant allow-list and makes the goal metadata declaration the single source of truth.
2. Preserving the two constructor arms in `frontier_exhaustion_entry` keeps behavior explicit while removing the brittle `GoalKind` match.
3. No backwards-compatibility aliasing or shim path is introduced; the old local allow-list is deleted.

## Verification Layers

1. Strategy dispatch chooses constructor arm -> focused unit test that drives `frontier_exhaustion_entry` or `record_exhausted_goals` through `CooldownRetry` and `PermanentUntilInvalidator`.
2. Runtime exhaustion cache stores the same retry-state shapes as before -> focused `record_exhausted_goals` tests in `agent_tick::planning`.
3. Existing AI-focused regression surface remains intact -> `cargo test -p worldwake-ai agent_tick::planning::tests::record_exhausted_goals_records_`.
4. Golden/E2E coverage is not required for this ticket because the contract is a lower-layer planner metadata dispatch with existing focused runtime tests.

## What to Change

### 1. Refactor frontier exhaustion entry

Update `crates/worldwake-ai/src/agent_tick/planning.rs::frontier_exhaustion_entry` to read:

`GoalDispatchKey::from_goal_kind(goal_kind).declaration().frontier_exhaustion_strategy`

Then dispatch:

- `FrontierExhaustionStrategy::CooldownRetry` -> `ExhaustionEntry::budget_retry_pending(...)`
- `FrontierExhaustionStrategy::PermanentUntilInvalidator` -> `ExhaustionEntry::frontier_exhausted(...)`

Delete the local `GoalKind` allow-list and `_` default arm.

### 2. Preserve runtime behavior

Keep existing `record_exhausted_goals` behavior for:

- `Sleep` frontier exhaustion entering `BudgetRetryPending`.
- `AcquireCommodity { purpose: SelfConsume, .. }` frontier exhaustion entering `BudgetRetryPending` and incrementing the acquisition-exhaustion tracker.
- `Patrol` frontier exhaustion entering `BudgetRetryPending`.
- Representative non-retry goals entering `FrontierExhausted` and suppressing planning.

### 3. Add explicit strategy-dispatch coverage

Add or adjust focused tests so one assertion proves the runtime helper uses the declared strategy rather than a local `GoalKind` match. Prefer a narrow test in `agent_tick::planning::tests` that covers both declared strategy arms without requiring a full golden harness.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify tests if needed)

## Out of Scope

- Adding new `GoalKind` variants.
- Changing which existing declarations are `CooldownRetry` beyond the S132 table.
- Changing exhaustion invalidation conditions, baselines, cooldown decay, or save format.
- Changing candidate generation, ranking, search, action validation, or golden scenarios.

## Acceptance Criteria

### Tests That Must Pass

1. `record_exhausted_goals_records_sleep_frontier_exhaustion_as_budget_retry` passes.
2. `record_exhausted_goals_records_self_consume_acquire_frontier_exhaustion_as_retry` passes.
3. `record_exhausted_goals_records_patrol_frontier_exhaustion_as_budget_retry` passes.
4. `record_exhausted_goals_records_frontier_exhaustion_as_suppressing_retry_state` passes.
5. Existing suite: `cargo test -p worldwake-ai agent_tick::planning::tests::frontier_exhaustion`

### Invariants

1. `frontier_exhaustion_entry` no longer matches directly on individual `GoalKind` retry exceptions.
2. `CooldownRetry` still produces `ExhaustionRetryState::BudgetRetryPending`, a populated `next_retry_tick`, and `suppresses_planning() == false`.
3. `PermanentUntilInvalidator` still produces `ExhaustionRetryState::FrontierExhausted`, no cooldown retry tick, and `suppresses_planning() == true`.
4. Existing acquisition-exhaustion tracker increments for self-consume acquisition frontier exhaustion are preserved.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` - focused runtime coverage for declared strategy dispatch through both frontier-exhaustion arms.

### Commands

1. `cargo test -p worldwake-ai agent_tick::planning::tests::frontier_exhaustion`
2. `cargo test -p worldwake-ai agent_tick::planning::tests::record_exhausted_goals_records_`
3. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-05-01.

- Updated `frontier_exhaustion_entry` to read `GoalDispatchKey::from_goal_kind(goal_kind).declaration().frontier_exhaustion_strategy`.
- Deleted the local `GoalKind` frontier-exhaustion retry allow-list from the runtime helper.
- Preserved the existing constructor behavior: declared `CooldownRetry` goals produce `BudgetRetryPending`, and declared `PermanentUntilInvalidator` goals produce `FrontierExhausted`.
- Added focused helper-level coverage for both declared strategy arms while preserving the existing `record_exhausted_goals` regressions for sleep, self-consume acquisition, patrol, and permanent suppression.
- No save-format bump was required because no serialized runtime state, save carrier, or `ExhaustionEntry` shape changed.

## Deviations

- The focused helper test asserts that cooldown retry populates a retry tick rather than pinning the exact tick value; cooldown arithmetic remains owned by `ExhaustionEntry::budget_retry_pending`.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::frontier_exhaustion -- --list`.
- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::record_exhausted_goals_records_ -- --list`.
- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::frontier_exhaustion`.
- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::record_exhausted_goals_records_`.
- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
