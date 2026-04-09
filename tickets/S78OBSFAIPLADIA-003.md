# S78OBSFAIPLADIA-003: Trace planning-time target-belief presence for failed-plan diagnostics

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — worldwake-ai decision-trace surface plus observer formatting
**Deps**: S78OBSFAIPLADIA-002

## Problem

The observer now emits a truthful deferral note for `Had Target Beliefs` because failed-plan rows do not currently have access to a planning-time belief snapshot or equivalent bounded carrier proving whether the actor knew the goal target at the time of planning. This leaves one intended S78 diagnostic unresolved: developers still cannot distinguish “planner failed despite knowing the target” from “planner failed because the target was not in the actor's planning-time belief inventory.”

## Assumption Reassessment (2026-04-09)

1. `PlanningPipelineTrace` in `crates/worldwake-ai/src/decision_trace.rs:227` currently stores affordances, candidate traces, planning attempts, selection, execution, blocker summaries, exhaustion snapshot, and patrol-route snapshots, but no planning-time `known_entities` inventory or equivalent target-belief presence carrier.
2. `PlanAttemptTrace` in `crates/worldwake-ai/src/decision_trace.rs:843` stores only goal, anchor, outcome, binding rejections, and expansion summaries. The failed-plan table in `crates/worldwake-cli/src/bin/observer.rs` therefore cannot prove planning-time target-belief presence from the attempt rows alone.
3. The shared abstraction boundary under audit is: planning-time actor belief state in `worldwake-ai` → persisted decision trace carrier in `worldwake-ai::decision_trace` → observer rendering in `worldwake-cli/src/bin/observer.rs`.
4. `CandidateTrace.evidence[*].knowledge_path` is not a sufficient substitute. It records candidate-motivation provenance for emitted candidates, not a complete planning-time entity-summary inventory, and can lawfully omit target-presence facts for goals whose emission depends on self or institutional knowledge instead.
5. The active S78 spec already allows this diagnostic to be deferred when the trace model lacks enough belief content. Ticket `S78OBSFAIPLADIA-002` exercised that fallback honestly, so the remaining work is a new traceability slice rather than leftover implementation debt inside `002`.

## Architecture Check

1. The clean fix is to trace a bounded planning-time target-belief-presence carrier in `worldwake-ai`, then render the observer column from that stored trace data. This preserves the time boundary and avoids reconstructing planning-time truth from end-of-run world state.
2. No backwards-compatibility shims should be introduced. The observer should switch from the current deferral note to the real column once the trace carrier exists.

## Verification Layers

1. Planning trace records whether target-belief presence was known at planning time for failed attempts -> focused `worldwake-ai` trace/unit coverage
2. Observer renders `Had Target Beliefs` from the stored trace carrier, not end-of-run world state -> focused `worldwake-cli` coverage
3. End-to-end failed-plan report shows `true` / `false` / `n/a` rows and matching frequency breakdown counts -> observer runtime output inspection

## What to Change

### 1. Add a bounded planning-time target-belief trace carrier

In `crates/worldwake-ai/src/decision_trace.rs` and the trace-construction path that populates `PlanningPipelineTrace`, add a derived carrier that records, for each relevant failed-plan row or goal target, whether the actor had a planning-time belief entry for that target entity.

Keep the carrier narrowly scoped:
- it must reflect planning-time state, not end-of-run world state
- it must support `true` / `false` / `n/a` rendering for observer failed-plan rows
- it must not pretend to be a full serialized belief snapshot unless the implementation genuinely stores one

### 2. Render the real observer column and update the breakdown

In `crates/worldwake-cli/src/bin/observer.rs`:
- replace the current deferral note with the real `Had Target Beliefs` column
- render `n/a` for targetless goals
- add `Had Target Beliefs = false: N / T` to the frequency breakdown once the stored carrier supports it truthfully

### 3. Remove the temporary deferral wording

Update the observer output and any nearby ticket/proof prose so the report no longer claims the column is deferred once the trace carrier is live.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/...` trace construction site(s) that populate `PlanningPipelineTrace` (modify)
- `crates/worldwake-cli/src/bin/observer.rs` (modify)
- `tickets/S78OBSFAIPLADIA-003.md` (this ticket, close-out only)

## Out of Scope

- Reconstructing planning-time belief presence from end-of-run `AgentBeliefStore`
- General-purpose belief-snapshot archival beyond what this observer diagnostic needs
- Changing planner search behavior or candidate legality

## Acceptance Criteria

### Tests That Must Pass

1. Failed-plan observer rows show `Had Target Beliefs` as `true` / `false` / `n/a` from planning-time trace data
2. Frequency breakdown includes `Had Target Beliefs = false: N / T` with counts matching displayed rows
3. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Planning-time target-belief presence is derived from stored trace data, not reconstructed from end-of-run world state
2. Observer output remains valid markdown and uses only mechanical counts in the frequency breakdown

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/...` focused trace test(s) proving the new planning-time target-belief carrier
2. `crates/worldwake-cli/src/bin/observer.rs` focused tests proving column rendering and breakdown counting

### Commands

1. `cargo test -p worldwake-cli --bin observer`
2. `cargo test -p worldwake-cli`
3. `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
