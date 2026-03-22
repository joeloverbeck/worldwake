# S15STAFAIEME-007: Add Request-Resolution Traceability And Provenance

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` request/runtime traceability, `worldwake-ai` trace consumption/tests
**Deps**: S15STAFAIEME-002

## Problem

The runtime currently exposes two strong trace surfaces for mixed AI/action debugging: decision traces in [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) and action traces in [crates/worldwake-sim/src/action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs). The gap is the boundary between them inside [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs): request resolution and affordance reproduction. When a concrete request is rejected before authoritative start, the current architecture does not emit a first-class trace explaining whether the affordance reproduced, whether `BestEffort` fell back to the concrete request, whether authoritative start was attempted, or what provenance the request had.

That gap weakens explainability, slows ticket reassessment, and makes S08/S15-style debugging depend on source reading rather than on world traceability.

## Assumption Reassessment (2026-03-19)

1. The authoritative request pipeline lives in `tick_step.rs`, with `apply_input` at [crates/worldwake-sim/src/tick_step.rs:215](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L215) and `resolve_affordance` at [crates/worldwake-sim/src/tick_step.rs:378](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L378). This is the exact layer where stale concrete requests were previously dying as `TickStepError::RequestedAffordanceUnavailable`.
2. The current code already distinguishes `ActionRequestMode::BestEffort` during request handling at [crates/worldwake-sim/src/tick_step.rs:263](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L263) and [crates/worldwake-sim/src/tick_step.rs:423](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L423). It also already records authoritative pre-start rejection through `Scheduler::record_action_start_failure` in [crates/worldwake-sim/src/scheduler.rs:177](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/scheduler.rs#L177) and `ActionTraceKind::StartFailed` in [crates/worldwake-sim/src/action_trace.rs:48](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs#L48). The true gap is narrower: current traces do not expose whether a request matched a reproduced affordance versus using the BestEffort concrete fallback before that authoritative start attempt.
3. Existing focused and golden coverage already proves much of the broader S15/S08 contract that this ticket originally described. Current tests include `best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches` in [crates/worldwake-sim/src/tick_step.rs:1474](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L1474), `derive_blocking_fact_uses_authoritative_trade_start_failure_when_belief_is_stale` in [crates/worldwake-ai/src/failure_handling.rs:1274](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs#L1274), `golden_contested_harvest_start_failure_recovers_via_remote_fallback` in [crates/worldwake-ai/tests/golden_production.rs:1021](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs#L1021), and `golden_local_trade_start_failure_recovers_via_production_fallback` in [crates/worldwake-ai/tests/golden_trade.rs:875](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs#L875). The ticket must not claim those scenarios are still missing.
4. This remains a mixed-layer ticket, but the intended verification surface is now precise: focused runtime request-resolution tracing in `worldwake-sim`, plus one targeted AI/runtime proof that the provenance recorded on the request survives submission and is visible when the runtime resolves it. New production-domain goldens are not the primary deliverable here.
5. The missing observability is architectural, not trade-specific. Trade exposed it first, but the shared substrate is the request-resolution path inside `tick_step.rs`; the fix should live there and be reusable by any action domain.
6. `docs/FOUNDATIONS.md` requires fully traceable chains of consequence. Today, a debugger can see that a BestEffort request later `StartFailed`, but not whether the runtime reproduced a lawful current affordance or fell back to the stale concrete request. That hidden branch weakens causal traceability at the authority boundary.
7. Scope correction: do not add a monolithic replacement for decision traces, action traces, or scheduler start failures. Add a small first-class request-resolution trace surface for the pre-start branch decision and thread explicit request provenance through `InputKind::RequestAction` so the trace records whether the request came from AI plan submission or external/manual input. Fresh-vs-retained AI plan provenance inside the decision system is already covered by `SelectedPlanSource` in [crates/worldwake-ai/src/decision_trace.rs:277](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs#L277); duplicating that at the runtime layer is out of scope.

## Architecture Check

1. A dedicated request-resolution trace is cleaner than overloading action traces with pre-start branch semantics. `ActionTraceKind::StartFailed` should remain the lifecycle fact, while the new request-resolution trace should answer the earlier question: did the request bind to a reproduced affordance or fall back to the concrete request identity?
2. Provenance should be modeled directly on `InputKind::RequestAction`, not inferred later from queue timing or controller state. That keeps the authority path deterministic and avoids magic interpretation rules in tests.
3. The cleaner architecture is additive and layered: request-resolution trace for runtime binding facts, scheduler start-failure records for authoritative rejection facts, and decision trace for next-tick AI reconciliation. No aliasing, no backward-compatibility shim formats, and no trace type trying to explain another layer's job.

## Verification Layers

1. Request binding branch (`reproduced current affordance` vs `BestEffort fallback` vs `no binding`) -> focused runtime tests on the new request-resolution trace sink.
2. Authoritative start is either attempted or skipped with an explicit reason -> focused runtime tests plus existing `ActionTraceKind::StartFailed` / scheduler `ActionStartFailure` assertions where start is actually reached.
3. Request provenance is preserved from submission through runtime handling -> targeted AI/runtime integration coverage for AI-submitted BestEffort requests and focused/runtime coverage for external/manual requests.
4. Mixed-layer debugging no longer requires inferring the pre-start boundary from absence of action events -> golden/integration assertions that read the request-resolution trace directly instead of using missing commits as a proxy.

## What to Change

### 1. Add a first-class request-resolution trace surface

Introduce a shared trace sink in `worldwake-sim` for request-resolution events emitted from `apply_input` / `resolve_affordance`. The schema should capture, at minimum:

- request actor and requested action identity
- request provenance
- whether current affordance reproduction succeeded
- whether a `BestEffort` concrete fallback path was used
- whether authoritative start was attempted
- structured rejection reason when resolution ends before start

### 2. Thread provenance through request submission

Extend `InputKind::RequestAction` so the engine can distinguish, at minimum:

- AI-planned request submission
- external/manual request submission

Replay and save/load paths must preserve the same provenance bit-for-bit. Do not introduce an inference rule based on `scheduled_tick` or controller state.

### 3. Add focused and integration coverage

Add focused runtime coverage around the new trace sink and at least one integration proof that reads the new trace in a mixed AI/runtime scenario. Reuse the S15 stale trade shape where helpful, but do not duplicate already-existing S15 production/trade golden contracts.

## Files to Touch

- `crates/worldwake-sim/src/input_event.rs` (modify)
- `crates/worldwake-sim/src/request_resolution_trace.rs` (new)
- `crates/worldwake-sim/src/tick_step.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify, if trace types/sink need export)
- `crates/worldwake-ai/src/agent_tick.rs` (modify, to tag AI-submitted requests)
- `crates/worldwake-ai/tests/` (modify, targeted integration/golden assertions)
- `crates/worldwake-ai/tests/golden_harness/` (modify only if the new trace sink needs harness exposure)
- `crates/worldwake-sim/src/replay_state.rs` (modify if new request provenance must round-trip through replay tests)
- `crates/worldwake-sim/src/save_load.rs` (modify if new request provenance must round-trip through save/load tests)

## Out of Scope

- changing trade-specific business rules
- replacing decision traces or action traces with a monolithic unified trace
- adding ad hoc `eprintln!` instrumentation as the durable solution
- changing planner selection/ranking behavior
- delivering the already-existing S15 trade/production start-failure goldens again
- introducing a separate runtime concept of retained-plan provenance that duplicates `SelectedPlanSource`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
2. `cargo test -p worldwake-sim tick_step::tests::resolve_affordance_uses_shared_request_binding_rule -- --exact`
3. `cargo test -p worldwake-sim tick_step::tests::reproduced_request_records_request_resolution_trace_before_start -- --exact`
4. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
5. Existing suite: `cargo test --workspace`
6. Existing suite: `cargo clippy --workspace`

### Invariants

1. The runtime must expose a first-class, structured explanation for request-resolution outcomes before authoritative start rather than forcing downstream debugging to infer them from missing action events.
2. Request provenance must remain explicit and deterministic across AI and external/manual submission paths, including replay/save-load round-trips.
3. The new trace surface must be shared infrastructure, not a trade-only special case.
4. The new trace surface must not duplicate scheduler start-failure records or decision-trace selected-plan provenance.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/tick_step.rs` — `reproduced_request_records_request_resolution_trace_before_start` verifies the reproduced-affordance branch records a first-class request-resolution event before authoritative start.
2. `crates/worldwake-sim/src/tick_step.rs` — `strict_request_records_resolution_rejection_without_start_attempt` verifies pre-start rejection records a structured `RejectedBeforeStart` outcome instead of disappearing behind missing action events.
3. `crates/worldwake-sim/src/tick_step.rs` — `best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches` now also asserts the request-resolution trace records the BestEffort fallback branch before the authoritative `StartFailed` fact.
4. `crates/worldwake-ai/tests/golden_production.rs` — `golden_contested_harvest_start_failure_recovers_via_remote_fallback` now asserts AI-submitted requests preserve `AiPlan` provenance and record reproduced binding in a live mixed AI/runtime scenario.
5. `crates/worldwake-ai/tests/golden_trade.rs` — `golden_local_trade_start_failure_recovers_via_production_fallback` now asserts the external/manual stale request preserves provenance and emits a first-class request-resolution event before later authoritative rejection.
6. `crates/worldwake-sim/src/input_event.rs` and replay/save-load round-trip coverage — request serialization surfaces now round-trip explicit provenance instead of implicitly assuming all requests are equivalent.

### Commands

1. `cargo test -p worldwake-sim tick_step::tests::best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
2. `cargo test -p worldwake-sim tick_step::tests::resolve_affordance_uses_shared_request_binding_rule -- --exact`
3. `cargo test -p worldwake-sim tick_step::tests::reproduced_request_records_request_resolution_trace_before_start -- --exact`
4. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
5. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
6. `cargo test --workspace`
7. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-19
- What actually changed:
  - Added explicit request provenance to `InputKind::RequestAction` with deterministic round-trip coverage.
  - Added a dedicated `RequestResolutionTraceSink` in `worldwake-sim` for the pre-start branch decision: reproduced affordance, BestEffort fallback, or rejection before start.
  - Threaded the new trace through `tick_step()` and exposed it to golden harness tests.
  - Strengthened focused runtime and mixed AI/runtime golden tests to assert request-resolution behavior directly.
  - Bumped `SAVE_FORMAT_VERSION` because queued request serialization changed.
- Deviations from original plan:
  - The original ticket overstated missing S15 coverage. The production/trade S15 start-failure goldens already existed, so the implemented scope narrowed to the remaining authority-bound request-resolution blind spot.
  - Runtime provenance was intentionally limited to `AiPlan` vs `External`; retained-plan provenance remains owned by `SelectedPlanSource` in decision traces rather than duplicated at the runtime layer.
  - While driving workspace gates to green, a few stale workspace tests and missing field propagations outside the original ticket surface were corrected so `cargo test --workspace` and `cargo clippy --workspace` would pass.
- Verification results:
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-ai --test golden_trade`
  - `cargo test -p worldwake-ai --test golden_production`
  - `cargo test -p worldwake-systems --lib`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
