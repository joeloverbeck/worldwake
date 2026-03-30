# S50AFFTRACE-001: Add affordance trace to decision trace pipeline

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — worldwake-ai (decision trace), worldwake-sim (affordance query)
**Deps**: None

## Problem

The decision trace system records candidates, ranking, plan search, selection, and execution outcome — but does not record which concrete affordances were available to the agent at the start of the decision tick. When debugging "why didn't the agent do X?", the first diagnostic question is "was X even available as an affordance?" Currently answering this requires reading production code to understand action constraints, place tags, and precondition checks.

This violates Principle 29 (Debuggability Is a Product Feature): "The simulation must support questions such as: Why did this agent do that?" The affordance set is the causal input to candidate generation and plan search — it determines the space of possible actions the planner can consider. Without tracing it, the earliest causal boundary in the decision pipeline is opaque.

**Concrete example**: During E20COMBEH-006 implementation, agents at EastFieldTrail (outdoor: Trail + Field tags) didn't travel because `relieve_wilderness` was locally available. The decision trace showed Relieve was selected, but not *why* the planner chose local relief over travel-to-latrine. An affordance trace would have shown `relieve_wilderness` in the available set, immediately explaining the planner's choice.

## Assumption Reassessment (2026-03-30)

1. **Affordance query**: `get_affordances()` in `crates/worldwake-sim/src/affordance_query.rs` returns `Vec<Affordance>` for an agent. Each `Affordance` contains the action `ActionDefId`, resolved targets, and constraint satisfaction. This is the data source for the trace.
2. **Decision trace pipeline**: `AgentTickDriver` in `crates/worldwake-ai/src/agent_tick.rs` runs the decision pipeline per tick. Tracing is opt-in via `enable_tracing()`. The trace is stored in `DecisionTraceSink` (`crates/worldwake-ai/src/decision_trace.rs`).
3. **Existing trace stages**: CandidateTrace (generated, ranked, suppressed), PlanSearchTrace (attempts, outcomes), SelectionTrace (selected plan), ExecutionTrace (action outcome). No affordance stage exists.
4. **Performance**: Candidate generation calls `get_affordances_for_defs()` (a filtered variant taking a `BTreeSet<ActionDefId>` subset), not the full `get_affordances()`. The trace requires a new `get_affordances()` call to capture the complete affordance set, but this call is gated on `tracing == true` so zero-cost-when-disabled is preserved. The cost when tracing is the affordance query plus memory to store the affordance list per agent per tick, bounded by registered action definitions × target combinations.
5. **Existing coverage for affordance correctness**: Focused unit tests in `crates/worldwake-systems/src/needs_actions.rs` (e.g., `relieve_wilderness_accepts_outdoor_places`, `relieve_wilderness_rejects_indoor_places`) and `crates/worldwake-sim/src/affordance_query.rs` test affordance constraint evaluation. The trace is for debugging golden/E2E scenarios, not for replacing focused coverage.
6. **Single-layer ticket**: This ticket adds a new field to the decision trace struct and populates it from the existing affordance query result. No cross-system interaction.

## Architecture Check

1. Adding an affordance stage to the decision trace follows the existing pattern: each pipeline stage has a corresponding trace struct, populated when tracing is enabled, zero-cost when disabled. This is cleaner than ad-hoc `eprintln` debugging or indirect inference from missing action-trace events.
2. No backward-compatibility shims. The new trace field is additive — `Option<AffordanceTrace>` or an empty default.

## Verification Layers

1. Affordance trace populated when tracing enabled → focused test in `crates/worldwake-ai/src/agent_tick.rs` or `crates/worldwake-ai/src/decision_trace.rs`
2. Affordance trace empty when tracing disabled → focused test (zero-cost verification)
3. Trace content matches `get_affordances()` output → focused test comparing trace against direct affordance query
4. Existing golden tests unaffected → `cargo test -p worldwake-ai`

## What to Change

### 1. New `AffordanceTrace` struct in `crates/worldwake-ai/src/decision_trace.rs`

```rust
/// Summary of affordances available to the agent at the start of the decision tick.
#[derive(Clone, Debug)]
pub struct AffordanceSummary {
    pub def_id: ActionDefId,
    pub action_name: String,
    pub target_count: usize,
}

/// Trace of affordances available to the agent at decision time.
#[derive(Clone, Debug)]
pub struct AffordanceTrace {
    pub available: Vec<AffordanceSummary>,
    pub place: Option<EntityId>,
}
```

Store only action name + target count per affordance, not full target lists, to keep trace size bounded.

### 2. Populate `AffordanceTrace` in agent tick pipeline

In the decision pipeline (where `get_affordances()` is already called), capture the result into `AffordanceTrace` when tracing is enabled.

### 3. Add `affordances` field to `PlanningPipelineTrace`

```rust
pub struct PlanningPipelineTrace {
    pub affordances: Option<AffordanceTrace>,  // NEW
    pub dirty: DirtySet,
    pub candidates: CandidateTrace,
    // ... existing fields
}
```

### 4. Expose in `dump_agent()` output

Add an affordance section to the human-readable `dump_agent()` method in `DecisionTraceSink`:
```
=== Tick 5 ===
  Place: EastFieldTrail
  Affordances: [travel(2 targets), relieve_wilderness(0 targets), eat(1 target)]
  Candidates: ...
```

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify: add `AffordanceSummary`, `AffordanceTrace`, add field to `PlanningPipelineTrace`, update `dump_agent`)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify: populate affordance trace via new `get_affordances()` call gated on `tracing == true`)

## Out of Scope

- Plan search cost comparison tracing (planner alternatives with costs)
- Body cost override tracing in action traces (separate concern)
- Changes to affordance query logic itself
- Changes to any golden test assertions (this is infrastructure for debugging)

## Acceptance Criteria

### Tests That Must Pass

1. Focused test: affordance trace populated when tracing enabled
2. Focused test: affordance trace empty/None when tracing disabled
3. Existing suite: `cargo test -p worldwake-ai`
4. Full workspace: `cargo test --workspace`

### Invariants

1. Tracing disabled → zero additional allocation per tick (opt-in contract preserved)
2. Affordance trace content matches the affordances the planner actually used for that tick
3. No existing golden test assertions change

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` or `crates/worldwake-ai/src/decision_trace.rs` — focused test that affordance trace is populated with correct action names when tracing is enabled
2. `crates/worldwake-ai/src/decision_trace.rs` — focused test that affordance trace is None when tracing is disabled

### Commands

1. `cargo test -p worldwake-ai -- affordance_trace`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
4. `scripts/verify.sh`

## Outcome

**Completion date**: 2026-03-30

**What changed**:
- `crates/worldwake-ai/src/decision_trace.rs`: Added `AffordanceSummary` and `AffordanceTrace` structs. Added `affordances: Option<AffordanceTrace>` field to `PlanningPipelineTrace`. Added affordance rendering to `format_outcome()` (place + available affordances with target counts).
- `crates/worldwake-ai/src/agent_tick/mod.rs`: Calls `get_affordances()` inside `tracing.then()` closure in the planning path; builds and stores `AffordanceTrace`. Zero-cost when tracing disabled.
- `crates/worldwake-ai/src/lib.rs`: Re-exported `AffordanceSummary` and `AffordanceTrace`.
- `crates/worldwake-ai/src/agent_tick/tests.rs`: Two focused tests (`affordance_trace_populated_when_tracing_enabled`, `affordance_trace_absent_when_tracing_disabled`).

**Deviations from original plan**:
- Ticket assumed `get_affordances()` was already called during candidate generation. Actually, candidate generation uses `get_affordances_for_defs()` (filtered variant). A new `get_affordances()` call was added, gated on `tracing == true`.
- File path corrected from `agent_tick.rs` to `agent_tick/mod.rs` (module directory structure).

**Verification**: `cargo clippy --workspace` clean, `cargo test --workspace` all passing (888 AI lib tests + full workspace).
