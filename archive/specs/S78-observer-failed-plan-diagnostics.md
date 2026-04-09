# S78: Observer Failed-Plan Diagnostics

**Status**: COMPLETED

## Summary

The simulation observer binary already traces affordances and failed-plan outcomes (budget-exhausted, frontier-exhausted), but the failed-plan output includes only outcome labels without diagnostic context. This spec enhances the observer binary's failed-plan output with derived diagnostic columns — best partial depth, operators considered, agent location, and target-belief presence — computed from existing trace data, enabling root-cause diagnosis of agent behavioral failures directly from observer output.

## Phase

Phase 7: Consequence Carriers (adjunct)

## Crates

- `worldwake-cli` (observer binary output formatting — primary)
- `worldwake-ai` (read-only dependency on existing trace types)

## Dependencies

- No spec dependencies. Builds on existing `PlanSearchOutcome`, `PlanAttemptTrace`, `SearchExpansionSummary`, and `AffordanceTrace` infrastructure in `worldwake-ai`.

## Design Goals

- Failed-plan traces include enough context to diagnose why the planner could not find a plan.
- No changes to AI crate types — all diagnostics are derived in the observer from existing trace data.
- No runtime cost when traces are not enabled (trace sinks are opt-in).
- Observer output remains human-readable and scannable.

## Non-Goals

- Changing the planner's search algorithm or fallback behavior.
- Adding new trace sink types (existing `DecisionTraceSink` is sufficient).
- Real-time trace streaming (observer is a batch post-hoc analysis tool).
- Affordance tracing (already fully implemented).
- Modifying `PlanSearchOutcome`, `PlanAttemptTrace`, or any other AI crate types.

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P29 (Debuggability Is a Product Feature) | Directly serves this principle — failed-plan diagnostics answer "why didn't the agent do X?" |
| P20 (Resource-Bounded Practical Reasoning) | Diagnostics explain how bounded reasoning constraints (budget, frontier) affected planning |
| P26 (Systems Interact Through State) | Trace data flows through state (trace sinks), not cross-system calls |
| P27 (Derived Summaries Are Caches) | Observer-computed diagnostics are derived views over authoritative trace data, never stored as truth |

## Section H: Causal Hooks

### Information-Path Analysis

No causal hooks. This is tooling infrastructure, not simulation logic.

### Positive-Feedback Analysis

No feedback loops. Tooling only.

### Concrete Dampeners

N/A.

### Stored State vs. Derived

- **Stored state**: No new stored state. Existing `PlanAttemptTrace` and `SearchExpansionSummary` are transient trace data, not authoritative world state.
- **Derived**: All new observer output columns are derived from existing trace data at formatting time.

---

## Deliverables

### D1: Observer Enhanced Failed-Plan Table

**File**: `crates/worldwake-cli/src/bin/observer.rs`

Enhance the "Failed plan attempts" table (currently at lines ~1081-1113) to include diagnostic columns derived from existing trace data.

**Current table columns**: `| Tick | Goal | Outcome | Expansions |`

**Enhanced table columns**: `| Tick | Goal | Outcome | Expansions | Max Depth | Candidates | Location | Had Target Beliefs |`

**Column derivation from existing trace types**:

| Column | Source | Derivation |
|--------|--------|------------|
| Tick | `AgentDecisionTrace.tick` | Direct (already used) |
| Goal | `PlanAttemptTrace.goal.kind` | Direct via `{:?}` (already used) |
| Outcome | `PlanAttemptTrace.outcome` | Match on variant (already used) |
| Expansions | `PlanSearchOutcome::{BudgetExhausted,FrontierExhausted}.expansions_used` | Direct (already used, type `u16`) |
| Max Depth | `PlanAttemptTrace.expansion_summaries` | `expansion_summaries.iter().map(\|s\| s.depth).max().unwrap_or(0)` |
| Candidates | `PlanAttemptTrace.expansion_summaries` | `expansion_summaries.iter().map(\|s\| s.candidates_generated).sum::<u16>()` |
| Location | `PlanningPipelineTrace.affordances` | `affordances.as_ref().and_then(\|a\| a.place)`, rendered as entity debug name or "?" |
| Had Target Beliefs | See D2 | `true` / `false` / `n/a` |

The observer must navigate the trace hierarchy to reach `PlanAttemptTrace`:
```
AgentDecisionTrace
  .outcome: DecisionOutcome::Planning(PlanningPipelineTrace)
    .planning: PlanSearchTrace
      .attempts: Vec<PlanAttemptTrace>
```

`Location` comes from the parent `PlanningPipelineTrace.affordances.place`, not from the individual attempt — all attempts in one planning pass share the same agent location.

### D2: Target-Belief Presence Check

**File**: `crates/worldwake-cli/src/bin/observer.rs`

For the "Had Target Beliefs" column, the observer checks whether the agent's belief snapshot (available via the trace) contains an `EntitySummary` for the goal's target entity.

**Concrete definition**:
- Extract the target `EntityId` from the `GoalKind` variant, if it has one (e.g., `EngageHostile { target }`, `TreatWounds { patient }`, `SearchForMissing { subject }`, `LootCorpse { corpse }`, etc.).
- For `GoalKind` variants without an entity target (e.g., `Sleep`, `Relieve`, `Wash`, `ConsumeOwnedCommodity`, `AcquireCommodity`, `ProduceCommodity`, `SellCommodity`, `RestockCommodity`, `ReduceDanger`), the column shows `n/a`.
- For variants with a target, check whether the planning pipeline's belief snapshot contains knowledge of that entity. The exact check depends on what belief data the trace exposes. If the trace does not expose belief content, this column is deferred to a follow-up spec.

**Implementation note**: If the existing trace types do not carry enough belief snapshot data to determine target-belief presence at observer read time, this column should be omitted from the initial implementation and noted as a future enhancement. The remaining columns (Max Depth, Candidates, Location) are all derivable from current trace data with certainty.

### D3: Failure Frequency Breakdown

**File**: `crates/worldwake-cli/src/bin/observer.rs`

Add a summary section after the failed-plan table with objective frequency counts:

```
### Failed Plan Frequency Breakdown
- frontier-exhausted: 15 / 20
- budget-exhausted: 5 / 20
- Max Depth = 0 (no operators available): 3 / 20
- Had Target Beliefs = false: 12 / 20
```

This is a purely mechanical count of column values — no interpretive heuristics or causal claims. Computed from the same data used to populate the table.

### D4: Profile-Driven Parameters

No new per-agent profile. Diagnostic enrichment is controlled by trace sink opt-in, which already exists.

## SystemFn Integration

No new SystemFn. Changes are entirely to the observer binary's output formatting, operating on existing trace data.

## Component Registration

No new components.

## Cross-System Interactions

- **Plan search → Trace sink**: Existing data flow. Search populates `PlanAttemptTrace` with `expansion_summaries` (already happens; no changes).
- **Trace sink → Observer binary**: Observer reads existing trace data and formats enhanced output (adds columns derived from existing fields).

## Verification

1. Run observer binary on `scenarios/cli-evaluation.ron`. Failed-plan table should show the enhanced columns (Max Depth, Candidates, Location, and optionally Had Target Beliefs).
2. The frequency breakdown should correctly count failure modes from the table.
3. `cargo test -p worldwake-ai` — existing tests pass (no AI crate changes).
4. `cargo clippy --workspace --all-targets -- -D warnings` clean.

## Outcome

Completed on 2026-04-09.

- Enhanced the observer failed-plan table with `Max Depth`, `Candidates`, `Location`, and `Had Target Beliefs`.
- Added a mechanical failed-plan frequency breakdown including `Had Target Beliefs = false: N / T`.
- Landed a bounded `TargetBeliefPresence` carrier on `PlanAttemptTrace` so the observer can render planning-time target-belief presence truthfully.

## Deviations

- The final implementation did require a narrow `worldwake-ai` trace-surface change. The original draft assumed all diagnostics could be derived in `worldwake-cli`, but `Had Target Beliefs` needed a bounded planning-time trace carrier to preserve the correct time boundary.
- The completed proof surface used crate-scoped checks: focused `worldwake-ai` trace tests, focused and crate-level `worldwake-cli` tests, `cargo clippy -p worldwake-cli --all-targets -- -D warnings`, and a runtime observer report on `scenarios/cli-evaluation.ron`.

## Verification Result

- Passed `cargo test -p worldwake-ai agent_tick::planning::tests::planning_time_target_belief_presence_marks_present_absent_and_na`
- Passed `cargo test -p worldwake-cli --bin observer`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
- Passed `cargo run -p worldwake-cli --bin observer -- scenarios/cli-evaluation.ron --output /tmp/s78-observer-report-003.md`
