# S78OBSFAIPLADIA-001: Enhance observer failed-plan table with diagnostic columns

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — observer binary output only
**Deps**: None

## Problem

The observer binary's failed-plan table shows only Tick, Goal, Outcome, and Expansions. When diagnosing why agents fail to find plans, developers must manually cross-reference trace data to determine search depth, candidate counts, and agent location. This ticket adds three derived columns (Max Depth, Candidates, Location) computed from existing trace data, making root-cause triage possible from the observer output alone.

## Assumption Reassessment (2026-04-09)

1. `PlanSearchOutcome` exists at `crates/worldwake-ai/src/decision_trace.rs:854` with variants `BudgetExhausted { expansions_used: u16 }`, `FrontierExhausted { expansions_used: u16 }`, `Unsupported`, `Found { .. }`. The observer's current failed-plan output is at `crates/worldwake-cli/src/bin/observer.rs:1079-1113`, matching on `FrontierExhausted` and `BudgetExhausted` variants only.
2. `PlanAttemptTrace` at `decision_trace.rs:843` carries `expansion_summaries: Vec<SearchExpansionSummary>`. Each `SearchExpansionSummary` (line 760) has `depth: u8` and `candidates_generated: u16`. These are the source for Max Depth and Candidates columns.
3. The shared abstraction boundary is the trace hierarchy: `AgentDecisionTrace.outcome → DecisionOutcome::Planning(PlanningPipelineTrace) → PlanSearchTrace.attempts → Vec<PlanAttemptTrace>`. The observer already navigates this hierarchy at lines 1051-1113. `AffordanceTrace.place: Option<EntityId>` at line 220 provides agent location, accessible via `PlanningPipelineTrace.affordances`.
4. Auto-correction: the live observer boundary is `driver.trace_sink().traces_for(agent_id) -> Vec<&AgentDecisionTrace>`, not owned `AgentDecisionTrace` values. The helper collecting failed attempts must therefore operate on borrowed trace entries. This is a mechanical signature correction only; it does not widen scope or change behavior.
5. Auto-correction: the ticket originally described command-only verification, but `crates/worldwake-cli/src/bin/observer.rs` supports narrow local unit tests cleanly. Focused helper tests for Max Depth, Candidates, and Location were added and the verification commands were narrowed from workspace-wide commands to honest crate-scoped proof plus one real observer run. Safe because the ticket is CLI-only and the broader workspace commands were not required to prove the owned surface.

## Architecture Check

1. All diagnostic columns are derived in the observer from existing trace fields — no AI crate types are modified, preserving zero blast radius. This is cleaner than enriching `PlanSearchOutcome` because it avoids data duplication and keeps the trace model minimal.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. Max Depth column correctly derived from `expansion_summaries` → focused unit tests + observer output inspection
2. Candidates column correctly summed from `expansion_summaries` → focused unit tests + observer output inspection
3. Location column extracted from `AffordanceTrace.place` → focused unit tests + observer output inspection
4. Single-layer ticket (CLI output formatting only) — no cross-system verification needed

## What to Change

### 1. Enhance table header and row formatting

In `crates/worldwake-cli/src/bin/observer.rs`, modify the failed-plan table section (currently lines ~1086-1112):

- Change header from `| Tick | Goal | Outcome | Expansions |` to `| Tick | Goal | Outcome | Expansions | Max Depth | Candidates | Location |`
- For each `PlanAttemptTrace` in the failed attempts:
  - **Max Depth**: `attempt.expansion_summaries.iter().map(|s| s.depth).max().unwrap_or(0)`
  - **Candidates**: `attempt.expansion_summaries.iter().map(|s| u32::from(s.candidates_generated)).sum::<u32>()`
  - **Location**: Extract from the parent `PlanningPipelineTrace.affordances.as_ref().and_then(|a| a.place)`. Since all attempts in one planning pass share the same location, this must be threaded from the outer loop that iterates `AgentDecisionTrace` entries. Render using `EntityId` display form (for example `e7g2`) or `"?"` if `None`.

### 2. Thread `AffordanceTrace.place` through the iteration

The current iteration collects failed attempts from borrowed `AgentDecisionTrace` entries. The `place` lives on `PlanningPipelineTrace`, not on `PlanAttemptTrace`, so the collection helper must also carry the `Option<EntityId>` place from `affordances` for each failed attempt row.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify)

## Out of Scope

- Modifying `PlanSearchOutcome`, `PlanAttemptTrace`, or any AI crate types
- Adding the "Had Target Beliefs" column (ticket 002)
- Adding a frequency breakdown summary (ticket 002)
- Changing the planner's search algorithm or fallback behavior
- Adding new trace sink types

## Acceptance Criteria

### Tests That Must Pass

1. Observer binary runs on `scenarios/cli-evaluation.ron` and the failed-plan table shows 7 columns (Tick, Goal, Outcome, Expansions, Max Depth, Candidates, Location)
2. Max Depth values are consistent with expansion summary data (0 when no expansions occurred)
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. No AI crate types are modified — all diagnostics are derived in the observer
2. Observer output remains valid markdown table format

## Test Plan

### New/Modified Tests

1. Added focused unit tests in `crates/worldwake-cli/src/bin/observer.rs` covering Max Depth derivation, Candidates summation, zero-expansion fallback, and Location fallback formatting.

### Commands

1. `cargo test -p worldwake-cli --bin observer`
2. `cargo test -p worldwake-cli`
3. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
4. `cargo run -p worldwake-cli --bin observer -- scenarios/cli-evaluation.ron --ticks 200 --output /tmp/s78-observer-report.md`

## Outcome

Completed on 2026-04-09.

- Enhanced the observer failed-plan markdown table with derived `Max Depth`, `Candidates`, and `Location` columns, all computed from existing trace data in `crates/worldwake-cli/src/bin/observer.rs`.
- Added small internal helpers so the failed-plan derivation is testable without changing any AI trace types or widening the CLI surface.
- Confirmed the real observer output on `scenarios/cli-evaluation.ron` includes the new 7-column failed-plan table, with rows like `budget-exhausted | 300 | 9 | 1483 | e0g0`.

## Verification Result

- Passed `cargo test -p worldwake-cli --bin observer`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
- Passed `cargo run -p worldwake-cli --bin observer -- scenarios/cli-evaluation.ron --ticks 200 --output /tmp/s78-observer-report.md`
