# S22-002: Replace JourneyCommitment and TravelDispositionProfile with frame equivalents

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — component removal, BeliefView trait changes, AI pipeline rewiring across 3 crates
**Deps**: S22-001 (new types must exist)

## Problem

All journey-specific types (`JourneyCommitment`, `JourneyCommitmentState`, `TravelDispositionProfile`, `JourneyPlanRelation`, `JourneyClearReason`, `JourneySwitchMarginSource`, `JourneyDebugSnapshot`, `JourneyRuntimeSnapshot`) and their consumers must be replaced with the generalized frame equivalents. Per P26: no backward compatibility layers — the old types are removed entirely.

## Assumption Reassessment (2026-03-24)

1. Journey types are consumed across 3 crates: worldwake-core (9 files), worldwake-sim (4 files), worldwake-ai (24 files), worldwake-systems (1 file: `tell_actions.rs`). Confirmed via grep.
2. Golden tests in `golden_ai_decisions.rs` and `golden_determinism.rs` reference `TravelDispositionProfile` in harness setup. These must be updated to use `IntentionDispositionProfile`.
3. `BeliefView` trait currently exposes `travel_disposition_profile()`. This must be replaced with `intention_disposition_profile()` and new `route_exists()` method.
4. `agent_tick/journey.rs` contains journey-specific logic that must be generalized in a renamed `agent_tick/frame.rs`.
5. `journey_switch_policy.rs` must be renamed to `frame_switch_policy.rs` with updated types.
6. `decision_runtime.rs` contains `JourneyPlanRelation`, `JourneyClearReason`, `JourneyRuntimeSnapshot`, and 5 free functions — all must be replaced.
7. This is the largest ticket in S22. It is a mechanical migration (type renames + generalization) but touches many files. The diff is large but structurally straightforward because the frame model is a strict superset of the journey model for the Travel domain.
8. `OmniscientBeliefView` must implement `route_exists()` by delegating to `Topology::find_route()`.
9. `PerAgentBeliefView` in `per_agent_belief_view.rs` must implement `route_exists()` (checking agent's believed topology).

## Architecture Check

1. Direct type replacement (not wrapping) is the cleanest approach. Every `JourneyCommitment` read/write becomes an `IntentionFrame` read/write with `IntentionDomain::Travel`. This avoids any adapter layer.
2. No backward-compatibility aliasing — P26 is strictly enforced. All old types are deleted, not deprecated.

## Verification Layers

1. All golden tests pass with frame types → `cargo test -p worldwake-ai` (golden E2E proof)
2. `classify_frame_plan_relation()` produces same decisions as old `classify_journey_plan_relation()` for travel domain → focused unit test
3. `route_exists()` on `OmniscientBeliefView` returns correct results → focused unit test
4. No orphaned journey references remain → `cargo build --workspace` + grep verification
5. Deterministic replay hashes may change (component names differ in serialization) — hashes must be recaptured in golden determinism tests

## What to Change

### 1. worldwake-core: Remove old types, update registrations

- Remove `JourneyCommitment`, `JourneyCommitmentState` from `intention.rs`
- Remove `travel_disposition.rs` module entirely
- Deregister `JourneyCommitment` and `TravelDispositionProfile` from `component_schema.rs`, `component_tables.rs`
- Remove their accessors from `world.rs`, `world_txn.rs`, `delta.rs`
- Update `lib.rs` to remove old re-exports, remove `travel_disposition` module declaration
- Update `test_utils.rs` to remove any journey helper builders

### 2. worldwake-sim: BeliefView migration

- Replace `travel_disposition_profile()` with `intention_disposition_profile()` in `belief_view.rs` trait
- Add `route_exists(&self, from: EntityId, to: EntityId) -> bool` to BeliefView trait
- Implement both methods on `OmniscientBeliefView` (reads `IntentionDispositionProfile` from world; delegates route check to `Topology::find_route()`)
- Implement both methods on `PerAgentBeliefView`
- Update mock BeliefView impls in `affordance_query.rs` and `trade_valuation.rs` tests

### 3. worldwake-ai: Pipeline rewiring

- Replace `AgentDecisionRuntime::last_journey_clear_reason` with `last_frame_clear_reason: Option<FrameClearReason>`
- Replace all journey free functions in `decision_runtime.rs` with frame equivalents
- Add `FramePlanRelation` enum to `decision_runtime.rs`
- Add `FrameRuntimeSnapshot` and `FrameDebugSnapshot` structs
- Rename `agent_tick/journey.rs` → `agent_tick/frame.rs`; generalize all functions
- Rename `journey_switch_policy.rs` → `frame_switch_policy.rs`; update all types
- Update `agent_tick/mod.rs`: all journey reads/writes become frame reads/writes
- Update `agent_tick/active_action.rs`, `execution.rs`, `planning.rs`, `observation.rs`
- Update `plan_selection.rs`, `interrupts.rs`, `failure_handling.rs`
- Update `candidate_generation.rs`, `planning_snapshot.rs`, `planning_state.rs`, `planner_ops.rs`, `plan_revalidation.rs`, `pressure.rs`, `ranking.rs`, `goal_model.rs`, `goal_explanation.rs`, `enterprise.rs`
- Update `lib.rs` re-exports
- Update `agent_tick/tests.rs` test setup to use `IntentionDispositionProfile`

### 4. worldwake-systems: tell_actions.rs

- Update any `TravelDispositionProfile` reference in `tell_actions.rs`

### 5. Golden tests: harness and hash updates

- Update `golden_ai_decisions.rs` harness setup: replace `TravelDispositionProfile` with `IntentionDispositionProfile`
- Recapture deterministic replay hashes in `golden_determinism.rs` if component registration changes serialization

## Files to Touch

### worldwake-core (modify)
- `crates/worldwake-core/src/intention.rs` (remove JourneyCommitment, JourneyCommitmentState)
- `crates/worldwake-core/src/travel_disposition.rs` (remove entirely)
- `crates/worldwake-core/src/component_schema.rs` (deregister old, already registered new from S22-001)
- `crates/worldwake-core/src/component_tables.rs` (deregister old)
- `crates/worldwake-core/src/lib.rs` (remove old module + re-exports)
- `crates/worldwake-core/src/world.rs` (remove old accessors)
- `crates/worldwake-core/src/world_txn.rs` (remove old accessors)
- `crates/worldwake-core/src/delta.rs` (remove old ComponentDelta variants)
- `crates/worldwake-core/src/test_utils.rs` (update helpers)

### worldwake-sim (modify)
- `crates/worldwake-sim/src/belief_view.rs`
- `crates/worldwake-sim/src/per_agent_belief_view.rs`
- `crates/worldwake-sim/src/affordance_query.rs` (mock impls in tests)
- `crates/worldwake-sim/src/trade_valuation.rs` (mock impls in tests)

### worldwake-ai (modify + rename)
- `crates/worldwake-ai/src/decision_runtime.rs`
- `crates/worldwake-ai/src/agent_tick/mod.rs`
- `crates/worldwake-ai/src/agent_tick/journey.rs` → `agent_tick/frame.rs` (rename + generalize)
- `crates/worldwake-ai/src/agent_tick/active_action.rs`
- `crates/worldwake-ai/src/agent_tick/execution.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/agent_tick/observation.rs`
- `crates/worldwake-ai/src/agent_tick/tests.rs`
- `crates/worldwake-ai/src/journey_switch_policy.rs` → `frame_switch_policy.rs` (rename + update)
- `crates/worldwake-ai/src/plan_selection.rs`
- `crates/worldwake-ai/src/interrupts.rs`
- `crates/worldwake-ai/src/failure_handling.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/planning_snapshot.rs`
- `crates/worldwake-ai/src/planning_state.rs`
- `crates/worldwake-ai/src/planner_ops.rs`
- `crates/worldwake-ai/src/plan_revalidation.rs`
- `crates/worldwake-ai/src/pressure.rs`
- `crates/worldwake-ai/src/ranking.rs`
- `crates/worldwake-ai/src/goal_model.rs`
- `crates/worldwake-ai/src/goal_explanation.rs`
- `crates/worldwake-ai/src/enterprise.rs`
- `crates/worldwake-ai/src/lib.rs`
- `crates/worldwake-ai/src/search/tests.rs`

### worldwake-systems (modify)
- `crates/worldwake-systems/src/tell_actions.rs`

### Golden tests (modify)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs`
- `crates/worldwake-ai/tests/golden_determinism.rs`

## Out of Scope

- Assumption population and evaluation logic (S22-003)
- Progress detection via PlannerOpKind (S22-004)
- Frame exhaustion → BlockedIntent creation (S22-005)
- Decision trace integration (S22-006)
- Non-Travel domain frame creation (Care, Escort, Errand, Generic) — this ticket only needs to support `IntentionDomain::Travel` to maintain existing behavior
- `FacilityQueueIntents` changes — orthogonal, untouched by this migration

## Acceptance Criteria

### Tests That Must Pass

1. All existing golden tests in `golden_ai_decisions.rs` pass with frame types replacing journey types
2. Deterministic replay in `golden_determinism.rs` passes (hashes recaptured if needed)
3. `cargo test --workspace` — all pass
4. `cargo clippy --workspace` — no new warnings
5. Focused unit test: `classify_frame_plan_relation()` for Travel domain matches old behavior
6. Focused unit test: `route_exists()` on OmniscientBeliefView returns true for connected places, false for disconnected

### Invariants

1. No references to `JourneyCommitment`, `JourneyCommitmentState`, `JourneyPlanRelation`, `JourneyClearReason`, `JourneySwitchMarginSource`, `JourneyDebugSnapshot`, `JourneyRuntimeSnapshot`, or `TravelDispositionProfile` remain in non-archived code
2. `IntentionFrame` with `IntentionDomain::Travel` produces identical agent behavior to old `JourneyCommitment` for travel scenarios
3. P26 enforced: no shims, aliases, deprecated wrappers, or re-exports of old types
4. BeliefView trait has `intention_disposition_profile()` and `route_exists()`, not `travel_disposition_profile()`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` (test module) — `classify_frame_plan_relation` focused tests
2. `crates/worldwake-ai/src/frame_switch_policy.rs` (test module) — renamed from journey_switch_policy tests
3. `crates/worldwake-ai/src/agent_tick/frame.rs` (test module) — renamed from journey tests
4. `crates/worldwake-ai/src/agent_tick/tests.rs` — updated harness setup
5. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — updated TravelDispositionProfile → IntentionDispositionProfile
6. `crates/worldwake-ai/tests/golden_determinism.rs` — hash recapture if needed
7. `crates/worldwake-sim/src/belief_view.rs` or `affordance_query.rs` tests — route_exists unit test

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-core`
4. `cargo clippy --workspace`
5. `cargo test --workspace`
6. `grep -r "JourneyCommitment\|TravelDispositionProfile\|JourneyPlanRelation\|JourneyClearReason" crates/ --include="*.rs" | grep -v "archive/"` — must return empty
