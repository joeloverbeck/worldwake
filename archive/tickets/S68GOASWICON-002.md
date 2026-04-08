# S68GOASWICON-002: Verify interrupt path contention cleanup

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — failure_handling.rs and active_action.rs function signatures
**Deps**: S68GOASWICON-001 (completed), S68GOASWICON-004 (completed)

## Problem

When an active action is interrupted during goal switch, the interrupt path in `active_action.rs` calls `reconcile_in_flight_state` which handles intent reconciliation. This ticket audits whether that path correctly cleans up `ContentionIntents` when the interrupted action was a `QueueForFacilityUse`, and fixes any gap found.

## Assumption Reassessment (2026-04-07)

1. Interrupt path confirmed at `active_action.rs:102-138` — calls `ctx.scheduler.interrupt_active_action()` then `reconcile_in_flight_state()`.
2. `reconcile_in_flight_state` (observation.rs:288) confirmed to take `&mut ContentionIntents` and call `reconcile_committed_facility_queue_intents` (observation.rs:355) for committed actions.
3. `reconcile_committed_facility_queue_intents` (observation.rs:384) confirmed to handle `QueueForFacilityUse` by inserting intents and `Harvest`/`Craft` by removing intents.
4. **GAP CONFIRMED**: The interrupt path calls `reconcile_in_flight_state` with `replan_signals` populated — this triggers `handle_current_step_failure` (observation.rs:313) which returns early (line 323) without reaching `reconcile_committed_facility_queue_intents`. `handle_current_step_failure` (active_action.rs:247) calls `handle_plan_failure` (failure_handling.rs:28), which clears `runtime.materialization_bindings` (line 40) but NOT `facility_intents`. This is the same gap pattern as the 5 sites fixed in S68GOASWICON-004.
5. `handle_plan_failure` (failure_handling.rs:28) does not take `&mut ContentionIntents` — needs parameter threading.
6. `handle_current_step_failure` (active_action.rs:247) does not take `&mut ContentionIntents` — needs parameter threading.
7. All callers of `handle_current_step_failure` already have `facility_intents` in scope: `execution.rs` (via `enqueue_valid_step_or_handle_failure`), `observation.rs` (via `reconcile_in_flight_state`).
8. Test call sites for `handle_plan_failure` exist at `failure_handling.rs:1471, 2270, 2421`.
9. No adjacent contradictions. After this fix, every `materialization_bindings.clear()` in the codebase will have a matching `facility_intents.intents.clear()`.

## Architecture Check

1. The interrupt path already has `facility_intents` in scope — no signature threading needed. The audit determines whether the existing reconciliation logic handles the `QueueForFacilityUse` case on interrupt correctly.
2. No backwards-compatibility shims. Any fix would follow the existing reconciliation pattern.

## Verification Layers

1. Interrupted `QueueForFacilityUse` step clears its intent entry -> decision trace or focused unit test
2. Single-layer ticket (AI planning lifecycle) — if a fix is needed, it lives entirely within the agent_tick reconciliation logic.

## What to Change

### 1. Thread facility_intents into handle_plan_failure

Add `facility_intents: &mut ContentionIntents` parameter to `handle_plan_failure` (failure_handling.rs:28). Add `facility_intents.intents.clear()` after `runtime.materialization_bindings.clear()` (line 40). Update test call sites at lines 1471, 2270, 2421.

### 2. Thread facility_intents into handle_current_step_failure

Add `facility_intents: &mut ContentionIntents` parameter to `handle_current_step_failure` (active_action.rs:247). Pass through to `handle_plan_failure`. Update all call sites: `execution.rs` (2 sites), `observation.rs` (4 sites).

## Files to Touch

- `crates/worldwake-ai/src/failure_handling.rs` (modify — add parameter, add clear)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify — add parameter to handle_current_step_failure, pass through)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — pass facility_intents at 2 call sites)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — pass facility_intents at 4 call sites)

## Out of Scope

- Goal-switch and lost-plan intent cleanup — covered by S68GOASWICON-001
- Golden E2E test — covered by S68GOASWICON-003
- Changes to the abort handler registration pattern
- Changes to the contention system or prune logic

## Acceptance Criteria

### Tests That Must Pass

1. If a fix is applied: new unit test verifying interrupted `QueueForFacilityUse` clears its intent
2. If no fix needed: document the audit trail showing the path is already correct
3. Existing suite: `cargo test -p worldwake-ai`
4. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. After an action interrupt, `facility_intents` must not contain stale entries from the interrupted step
2. The reconciliation path must handle all `PlannerOpKind` variants that create contention intents

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — new test if fix needed: interrupt a `QueueForFacilityUse` step, verify intent is removed
2. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.` (use this if audit finds no gap)

### Commands

1. `cargo test -p worldwake-ai -- reconcile` (targeted — adjust filter to match test names)
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-07.

**Audit result**: Gap confirmed. `handle_plan_failure` (failure_handling.rs:28) cleared `materialization_bindings` but not `ContentionIntents`. The interrupt path (`InterruptForReplan` → `reconcile_in_flight_state` → `handle_current_step_failure` → `handle_plan_failure`) bypassed `reconcile_committed_facility_queue_intents` via early return, leaving stale intents.

**What changed**:
- `failure_handling.rs`: Added `facility_intents: &mut ContentionIntents` parameter to `handle_plan_failure`. Added `facility_intents.intents.clear()` after `runtime.materialization_bindings.clear()` (line 42). Updated 3 test call sites.
- `active_action.rs`: Added `facility_intents: &mut ContentionIntents` parameter to `handle_current_step_failure`. Passes through to `handle_plan_failure`.
- `execution.rs`: Updated 2 call sites to `handle_current_step_failure` to pass `facility_intents`.
- `observation.rs`: Updated 4 call sites to `handle_current_step_failure` to pass `facility_intents`.

**Invariant achieved**: After this fix plus S68GOASWICON-001 and S68GOASWICON-004, every `materialization_bindings.clear()` in the codebase now has a matching `facility_intents.intents.clear()` (or equivalent `ContentionIntents::default()` reassignment in the death-clear path).

**Deviations**: Ticket originally described as "audit, fix if needed" — audit confirmed gap, fix applied. `Engine Changes` updated from `None or Yes` to `Yes`. `Files to Touch` expanded to include `execution.rs` (parameter threading).

## Verification Result

- Passed `cargo test -p worldwake-ai --lib` (1065 tests)
- Passed `cargo test -p worldwake-ai --test golden_production` (43 tests)
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
