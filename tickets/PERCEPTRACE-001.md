# PERCEPTRACE-001: Add PerceptionTraceSink for Debugging Institutional Belief Projection

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new opt-in trace system in worldwake-systems (perception module)
**Deps**: None (follows established ActionTraceSink / DecisionTraceSink / PoliticalTraceSink pattern)

## Problem

When debugging golden tests or AI behavior involving institutional beliefs ("Why didn't agent X learn about the office change?"), there is no diagnostic path except reading perception system source code. The project has three opt-in trace systems (DecisionTraceSink, ActionTraceSink, PoliticalTraceSink) but perception has none. This gap was exposed during S16BFORLEGEMEGOL-003 implementation where the Perception-Politics system ordering issue took ~30 minutes of source-code tracing to diagnose.

## Assumption Reassessment (2026-03-22)

1. Existing trace pattern: `ActionTraceSink` at `worldwake-sim/src/action_trace.rs` provides the model — zero-cost when disabled, structured events, queryable by agent/tick. Verified: `ActionTraceEvent`, `ActionTraceKind`, `ActionTraceSink` with `record()`, `events_for()`, `events_at()`, `dump_agent()`.
2. `perception_system()` at `worldwake-systems/src/perception.rs:19` processes events via `events_at_tick(tick)` and writes institutional beliefs via `record_institutional_belief()`. These are the two key observation points for tracing.
3. `SystemExecutionContext` at `worldwake-sim/src/system_dispatch.rs:13` already carries `politics_trace: Option<&mut PoliticalTraceSink>` — adding a perception trace follows the same pattern.
4. Golden harness already has `h.enable_action_tracing()` and `h.enable_politics_tracing()` — adding `h.enable_perception_tracing()` follows established conventions.

## Architecture Check

1. Follows the established trace pattern (ActionTrace, PoliticalTrace) — zero-cost when disabled, opt-in per-test, structured queryable events. No new architectural concepts.
2. No backwards-compatibility shims.

## Verification Layers

1. Trace records per-event institutional claims → unit test: inject political event, verify trace records projected claims
2. Trace records observation check outcomes → unit test: inject agent with low fidelity, verify trace records failed check
3. Zero-cost when disabled → unit test: run perception without trace, verify no allocation
4. GoldenHarness integration → golden test: enable perception tracing, query sink after step_once

## What to Change

### 1. Define `PerceptionTraceEvent` and `PerceptionTraceSink` (new file or extend perception.rs)

```rust
pub struct PerceptionTraceEvent {
    pub tick: Tick,
    pub observer: EntityId,
    pub event_id: EventId,
    pub observation_passed: bool,
    pub entity_observations: Vec<EntityId>,
    pub institutional_claims: Vec<(InstitutionalBeliefKey, InstitutionalClaim)>,
}

pub struct PerceptionTraceSink { /* Vec<PerceptionTraceEvent> */ }
```

Key queries: `events_for(observer)`, `events_at(tick)`, `claims_at(observer, tick)`, `dump_agent(observer)`.

### 2. Thread `Option<&mut PerceptionTraceSink>` through `SystemExecutionContext`

Add field alongside existing `politics_trace`.

### 3. Instrument `perception_system()` to record trace events

At the event processing loop (lines 35-92), record which events were processed, which agents passed observation checks, and which institutional claims were projected.

### 4. Add `enable_perception_tracing()` to GoldenHarness

Following `enable_action_tracing()` and `enable_politics_tracing()` patterns.

## Files to Touch

- `crates/worldwake-sim/src/system_dispatch.rs` (modify — add perception_trace field to SystemExecutionContext)
- `crates/worldwake-systems/src/perception.rs` (modify — define trace types, instrument perception_system)
- `crates/worldwake-sim/src/tick_step.rs` (modify — thread perception trace through run_systems)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add enable_perception_tracing)

## Out of Scope

- Tracing `observe_passive_local_entities` (entity-level observations) — focus on institutional belief projection first
- Enriching PoliticalTraceSink with pending claimant information
- Event delta inspection utilities

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: perception trace records institutional claims from political events
2. Unit test: perception trace records observation check failures
3. `cargo test --workspace` — all existing tests still pass
4. `cargo clippy --workspace` — no new warnings

### Invariants

1. Zero-cost when trace is None — no allocation or recording
2. Trace types follow the same patterns as ActionTraceSink (Serialize/Deserialize if existing traces do)
3. SystemExecutionContext remains backward-compatible (new Option field defaults to None)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs::tests::trace_records_institutional_claims` — verifies trace captures projected claims
2. `crates/worldwake-systems/src/perception.rs::tests::trace_absent_when_disabled` — verifies zero-cost

### Commands

1. `cargo test -p worldwake-systems perception`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
