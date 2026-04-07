# S68GOASWICON-002: Verify interrupt path contention cleanup

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None or Yes — depends on audit findings
**Deps**: S68GOASWICON-001

## Problem

When an active action is interrupted during goal switch, the interrupt path in `active_action.rs` calls `reconcile_in_flight_state` which handles intent reconciliation. This ticket audits whether that path correctly cleans up `ContentionIntents` when the interrupted action was a `QueueForFacilityUse`, and fixes any gap found.

## Assumption Reassessment (2026-04-07)

1. Interrupt path confirmed at `active_action.rs:102-122` — calls `ctx.scheduler.interrupt_active_action()` then `reconcile_in_flight_state()`.
2. `reconcile_in_flight_state` (observation.rs:287) confirmed to take `&mut ContentionIntents` and call `reconcile_committed_facility_queue_intents` (observation.rs:354) for committed actions.
3. `reconcile_committed_facility_queue_intents` (observation.rs:383) confirmed to handle `QueueForFacilityUse` by inserting intents (line 407) and `Harvest`/`Craft` by removing intents (line 416).
4. The interrupt path calls `reconcile_in_flight_state` with `replan_signals` populated — this triggers `handle_current_step_failure` (observation.rs:312) which returns early without reaching `reconcile_committed_facility_queue_intents`. The question is whether `handle_current_step_failure` clears intents for the failed step.
5. No adjacent contradictions exposed — this is a verification/audit ticket.

## Architecture Check

1. The interrupt path already has `facility_intents` in scope — no signature threading needed. The audit determines whether the existing reconciliation logic handles the `QueueForFacilityUse` case on interrupt correctly.
2. No backwards-compatibility shims. Any fix would follow the existing reconciliation pattern.

## Verification Layers

1. Interrupted `QueueForFacilityUse` step clears its intent entry -> decision trace or focused unit test
2. Single-layer ticket (AI planning lifecycle) — if a fix is needed, it lives entirely within the agent_tick reconciliation logic.

## What to Change

### 1. Audit the interrupt reconciliation path

Trace the code path when an active `QueueForFacilityUse` action is interrupted:

1. `handle_active_action_phase` (active_action.rs:102) detects `InterruptForReplan`
2. `scheduler.interrupt_active_action()` runs the action's abort handler
3. `reconcile_in_flight_state` (observation.rs:287) is called
4. Since `replan_signals` is populated, `handle_current_step_failure` is called (line 312)
5. Determine whether `handle_current_step_failure` removes the `QueueForFacilityUse` intent from `facility_intents`

### 2. Fix if needed

If the audit reveals that interrupted `QueueForFacilityUse` steps leave stale intents in `facility_intents`, add intent cleanup in the failure handler. The fix should remove the intent for the step's target facility from `facility_intents.intents`.

If the audit confirms the path is already correct (e.g., the abort handler or the subsequent goal-switch cleanup from S68GOASWICON-001 handles it), document the finding and close with no code change.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — only if audit finds a gap)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (read-only audit)

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
