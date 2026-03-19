# S15STAFAIEME-007: Add Request-Resolution Traceability And Provenance

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` request/runtime traceability, `worldwake-ai` trace consumption/tests
**Deps**: S15STAFAIEME-002

## Problem

The runtime currently exposes two strong trace surfaces for mixed AI/action debugging: decision traces in [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) and action traces in [crates/worldwake-sim/src/action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs). The gap is the boundary between them inside [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs): request resolution and affordance reproduction. When a concrete request is rejected before authoritative start, the current architecture does not emit a first-class trace explaining whether the affordance reproduced, whether `BestEffort` fell back to the concrete request, whether authoritative start was attempted, or what provenance the request had.

That gap weakens explainability, slows ticket reassessment, and makes S08/S15-style debugging depend on source reading rather than on world traceability.

## Assumption Reassessment (2026-03-19)

1. The authoritative request pipeline lives in `tick_step.rs`, with `apply_input` at [crates/worldwake-sim/src/tick_step.rs:215](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L215) and `resolve_affordance` at [crates/worldwake-sim/src/tick_step.rs:378](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L378). This is the exact layer where stale concrete requests were previously dying as `TickStepError::RequestedAffordanceUnavailable`.
2. The current code already distinguishes `ActionRequestMode::BestEffort` during request handling at [crates/worldwake-sim/src/tick_step.rs:263](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L263) and [crates/worldwake-sim/src/tick_step.rs:423](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L423), but the trace surfaces stop at either decision output or action lifecycle output.
3. Existing focused coverage now proves the new S15 runtime behavior but not the missing traceability layer: `best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches` in [crates/worldwake-sim/src/tick_step.rs:1474](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs#L1474), `derive_blocking_fact_uses_authoritative_trade_start_failure_when_belief_is_stale` in [crates/worldwake-ai/src/failure_handling.rs:1274](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs#L1274), and the S15 trade golden pair in [crates/worldwake-ai/tests/golden_trade.rs:875](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs#L875) and [crates/worldwake-ai/tests/golden_trade.rs:884](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs#L884).
4. This is a mixed-layer ticket. The target is neither candidate generation nor pure golden coverage; it is shared runtime observability for the handoff from AI/external request intent into authoritative start. Full action registries are required for at least one integration proof because the gap only becomes meaningful when a real concrete action request traverses the runtime.
5. The missing observability is architectural, not domain-specific. Trade exposed it first, but the underlying blind spot sits in the shared request-resolution substrate and should be fixed once there rather than by adding trade-only debug paths.
6. `docs/FOUNDATIONS.md` requires fully traceable chains of consequence and prohibits hidden drama logic. A request that disappears or mutates at the runtime boundary without a first-class trace undermines that standard even when behavior is otherwise correct.
7. Scope correction: this ticket should not add one-off logging strings or trade-specific trace records. It should add a durable shared trace surface for request resolution, with structured provenance and rejection/start-attempt facts that other domains can reuse.

## Architecture Check

1. A dedicated request-resolution trace is cleaner than trying to overload either decision traces or action traces with pre-start runtime semantics they do not own. The request layer has its own contract and should have its own structured surface.
2. Provenance should be modeled directly, not inferred ad hoc from call sites. If the engine needs to know whether a request came from fresh AI planning, retained intent, or manual external input, that should be explicit in the request trace schema rather than reverse-engineered later.
3. No backwards-compatibility aliasing or shim trace formats should be introduced. Add the new trace surface directly and update any helper APIs/tests to use it.

## Verification Layers

1. A concrete request records whether affordance reproduction succeeded, fell back, or failed before start -> focused runtime tests on the new request-resolution trace sink.
2. Authoritative start is either attempted or skipped with an explicit reason -> focused runtime tests plus existing action-trace assertions where start is actually reached.
3. Request provenance is preserved from submission through runtime handling -> integration coverage in `worldwake-ai` or golden harness tests that submit fresh AI and manual/retained requests.
4. Mixed-layer debugging no longer requires inferring the pre-start boundary from absence of action events -> golden/integration assertions that read request-resolution traces directly instead of using missing commits as a proxy.

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

Extend the request submission/runtime path so the engine can distinguish at least:

- fresh current-tick AI request
- retained or replayed stale request
- manual or external request

If the existing request type already contains enough information to derive some of this cleanly, expose it explicitly at the trace layer instead of forcing tests to infer it indirectly.

### 3. Add focused and integration coverage

Add focused runtime coverage around the new trace sink and at least one integration proof that reads the new trace in a mixed AI/runtime scenario. Reuse the S15 stale trade shape where helpful, but keep the new focused tests domain-agnostic where possible.

## Files to Touch

- `crates/worldwake-sim/src/tick_step.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify, if trace types/sink need export)
- `crates/worldwake-sim/src/action_trace.rs` (modify only if shared trace infrastructure belongs here)
- `crates/worldwake-ai/tests/` (modify, targeted integration/golden assertions)
- `docs/golden-e2e-testing.md` (modify only if examples need to point at the new trace after implementation)

## Out of Scope

- changing trade-specific business rules
- replacing decision traces or action traces with a monolithic unified trace
- adding ad hoc `eprintln!` instrumentation as the durable solution
- changing planner selection/ranking behavior

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
2. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
3. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback_replays_deterministically -- --exact`
4. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. The runtime must expose a first-class, structured explanation for request-resolution outcomes before authoritative start rather than forcing downstream debugging to infer them from missing action events.
2. Request provenance must remain explicit and deterministic across AI and non-AI submission paths.
3. The new trace surface must be shared infrastructure, not a trade-only special case.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/tick_step.rs` — add focused tests proving the new request-resolution trace records reproduction success/failure, concrete fallback, and start-attempt facts.
2. `crates/worldwake-ai/tests/golden_trade.rs` or a targeted integration test under `crates/worldwake-ai/src/` — add assertions that the stale trade request emits the expected request-resolution trace with preserved provenance.

### Commands

1. `cargo test -p worldwake-sim best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
2. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
3. `cargo test -p worldwake-ai --test golden_trade`
