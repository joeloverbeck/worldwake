# S78: Observer Failed-Plan Diagnostics

**Status**: Draft

## Summary

The simulation observer binary already traces affordances and failed-plan outcomes (budget-exhausted, frontier-exhausted), but the failed-plan output includes only outcome labels without diagnostic context. This spec enhances the failed-plan trace output with candidate quality summaries, blocker descriptions, and operator filtering reasons, enabling root-cause diagnosis of agent behavioral failures directly from observer output.

## Phase

Phase 7: Consequence Carriers (adjunct)

## Crates

- `worldwake-ai` (decision trace structs, plan search outcome enrichment)
- `worldwake-cli` (observer binary output formatting)

## Dependencies

- No spec dependencies. Builds on existing `PlanSearchOutcome` and `DecisionTrace` infrastructure.

## Design Goals

- Failed-plan traces include enough context to diagnose why the planner could not find a plan.
- No runtime cost when traces are not enabled (trace sinks are opt-in).
- Observer output remains human-readable and scannable.

## Non-Goals

- Changing the planner's search algorithm or fallback behavior.
- Adding new trace sink types (existing `DecisionTraceSink` is sufficient).
- Real-time trace streaming (observer is a batch post-hoc analysis tool).
- Affordance tracing (already fully implemented).

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P29 (Debuggability Is a Product Feature) | Directly serves this principle -- failed-plan diagnostics answer "why didn't the agent do X?" |
| P20 (Resource-Bounded Practical Reasoning) | Diagnostics explain how bounded reasoning constraints (budget, frontier) affected planning |
| P26 (Systems Interact Through State) | Trace data flows through state (trace sinks), not cross-system calls |

## Section H: Causal Hooks

### Information-Path Analysis

No causal hooks. This is tooling infrastructure, not simulation logic.

### Positive-Feedback Analysis

No feedback loops. Tooling only.

### Concrete Dampeners

N/A.

### Stored State vs. Derived

- **Stored state**: Enriched `PlanSearchOutcome` variants carry additional diagnostic fields. These are transient trace data, not authoritative world state.
- **Derived**: Observer output formatting is derived from trace data.

---

## Deliverables

### D1: Enrich `PlanSearchOutcome` with Diagnostic Context

**File**: `crates/worldwake-ai/src/search/mod.rs`

**Current `PlanSearchOutcome`**:
- `BudgetExhausted { expansions_used: u32 }`
- `FrontierExhausted { expansions_used: u32 }`
- `Unsupported`
- `Found { ... }`

**New fields** on failure variants:

```rust
pub enum PlanSearchOutcome {
    BudgetExhausted {
        expansions_used: u32,
        best_partial_depth: u16,          // deepest node reached before budget ran out
        goal_kind: GoalKind,              // which goal was being planned for
    },
    FrontierExhausted {
        expansions_used: u32,
        operators_considered: u16,        // how many operator candidates were evaluated
        goal_kind: GoalKind,
    },
    Unsupported {
        goal_kind: GoalKind,
    },
    Found { /* unchanged */ },
}
```

These fields are populated during search at negligible cost (already tracked internally).

### D2: Enrich `DecisionTraceEvent` Failed-Plan Entry

**File**: `crates/worldwake-ai/src/decision_trace.rs`

Add a `FailedPlanDiagnostic` struct stored alongside existing failed-plan trace events:

```rust
pub struct FailedPlanDiagnostic {
    pub goal_kind: GoalKind,
    pub outcome: PlanSearchOutcome,
    pub available_operators: u16,
    pub agent_place: EntityId,
    pub beliefs_about_goal_target: bool,  // did the agent have beliefs about the target entity?
}
```

Record this diagnostic in the existing `DecisionTraceEvent::PlanSearchCompleted` (or equivalent) path when the outcome is not `Found`.

### D3: Observer Binary Output Enhancement

**File**: `crates/worldwake-cli/src/bin/observer.rs`

Enhance the "Failed plan attempts" table (currently at lines ~1082-1110) to include the new diagnostic fields:

```
## Failed Plan Attempts

| Tick | Goal | Outcome | Expansions | Depth | Operators | Location | Had Target Beliefs |
|------|------|---------|------------|-------|-----------|----------|--------------------|
| 42   | Eat  | frontier-exhausted | 12 | 0 | 3 | BarrenCamp | false |
| 85   | Drink | budget-exhausted | 64 | 2 | 5 | BarrenCamp | true |
```

Add a summary section after the table:

```
### Diagnosis Summary
- 15/20 failed plans had `Had Target Beliefs = false` → likely perception/belief issue (see S77)
- 3/20 failed plans had `Depth = 0` → no operators available at agent's location
- 2/20 failed plans had `budget-exhausted` with `Depth >= 2` → search budget too small for plan complexity
```

The summary is computed by the observer binary from the trace data, not stored.

### D4: Profile-Driven Parameters

No new per-agent profile. Diagnostic enrichment is controlled by trace sink opt-in, which already exists.

## SystemFn Integration

No new SystemFn. Changes are to trace data structures (populated during plan search) and observer binary output (post-hoc formatting).

## Component Registration

No new components.

## Cross-System Interactions

- **Plan search -> Trace sink**: Search populates `PlanSearchOutcome` with diagnostic fields (already writes to trace sink; this adds fields to existing writes).
- **Trace sink -> Observer binary**: Observer reads trace data and formats enhanced output (existing data flow; this adds columns to existing table).

## Verification

1. Run observer binary on `scenarios/cli-evaluation.ron`. Failed-plan table should show diagnostic columns.
2. The diagnosis summary should identify the dominant failure mode (e.g., "no target beliefs").
3. `cargo test -p worldwake-ai` -- existing tests pass (diagnostic fields have defaults for non-trace paths).
4. `cargo clippy --workspace --all-targets -- -D warnings` clean.
