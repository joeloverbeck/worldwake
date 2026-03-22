# AITRACEPLAN-001: Expose Selected Plan Shape and Fallback Semantics in Decision Traces

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` decision-trace model, trace formatting, and focused tests
**Deps**: existing decision trace infrastructure in `crates/worldwake-ai/src/decision_trace.rs` and `crates/worldwake-ai/src/agent_tick.rs`, `docs/golden-e2e-testing.md`

## Problem

The current decision trace surface is strong on candidate generation and action outcome, but it is weaker than it should be in the middle of the pipeline where debugging often actually stalls: selected-plan shape and selection/fallback semantics.

During S13POLEMEGOLSUI-002, the trace could prove:

1. `ClaimOffice` was generated
2. a planning outcome existed
3. action start later failed with a locality precondition

But the trace did not make the critical middle question easy to answer:

"What exact plan shape did selection believe it had chosen, and was that plan immediately actionable, deferred, or just the least-bad fallback?"

That gap makes AI debugging slower and more inference-heavy than it should be.

## Assumption Reassessment (2026-03-18)

1. The current structured trace model in `crates/worldwake-ai/src/decision_trace.rs` already stores `PlanAttemptTrace.outcome` with `PlanSearchOutcome::Found { steps, terminal_kind }`, so individual search attempts do preserve plan shape. The missing piece is that `SelectionTrace` only records the selected `GoalKey` and optional goal switch; it does not identify the selected attempt or selected found-plan summary.
2. `plan_and_validate_next_step_traced()` in `crates/worldwake-ai/src/agent_tick.rs` builds `PlanSearchTrace.attempts`, runs `select_best_plan(...)`, and stores only `selection_trace.selected = Some(selected_goal)` plus `plan_continued`. That means the final trace does not directly expose whether the selected outcome was:
   - a newly adopted traced plan from this tick's search,
   - retention of the existing `runtime.current_plan` because challengers did not clear switch policy,
   - or snapshot-only continuation where no new search result was selected at all.
3. Because `select_best_plan(...)` in `crates/worldwake-ai/src/plan_selection.rs` may return either a newly found candidate plan or the already-active current plan, "selected attempt id" is not the right canonical abstraction by itself. The durable gap is missing **selection provenance** plus a canonical **selected plan summary** on the final trace surface.
4. `ExecutionTrace` records only `enqueued_step`, `revalidation_passed`, and broad execution failures. It does not explicitly classify whether the final selected state was actionable now, blocked at a progress barrier, retained-but-not-replanned, or snapshot-only continuation.
5. Existing focused coverage is broader than the original ticket implied:
   - `crates/worldwake-ai/src/decision_trace.rs` covers summary formatting, sink queries, and trace-model structs.
   - `crates/worldwake-ai/src/agent_tick.rs` already has focused traced-runtime coverage such as `trace_planning_outcome_for_hungry_agent`, `trace_force_law_office_skips_political_candidates_and_planning`, `trace_dead_agent`, `trace_active_action_interrupt`, and `tracing_disabled_produces_identical_behavior`.
   - `cargo test -p worldwake-ai -- --list` confirms there is still no focused test proving the final per-tick trace directly exposes the selected plan's steps, terminal semantics, and provenance without reverse-correlating `selection.selected`, `planning.attempts`, and `execution`.
6. `docs/golden-e2e-testing.md` already recommends decision traces for planner-search behavior. The main gap is the structured trace surface itself; a doc update is useful only to point golden authors at the richer canonical selected-plan fields once they exist.
7. Scope correction: this ticket should improve the canonical trace model and focused tests first. A docs update is secondary and should stay minimal. No planner-search or plan-selection policy changes are needed.

## Architecture Check

1. The clean solution is to extend the structured trace model so selected-plan semantics are first-class trace data, not an inferred relationship between `selection.selected`, `planning.attempts`, `plan_continued`, and `execution`. That keeps the final trace surface authoritative and machine-queryable.
2. Do not solve this with ad hoc test-only `eprintln!` dumps or bespoke golden-only helpers. The missing data belongs in the trace model itself so all focused tests, golden tests, and future debugging tools consume one canonical surface.
3. Do not duplicate the full planner state or add a second debug-only planner log. The robust architecture is to expose a compact selected-plan summary plus explicit provenance/reason class in the existing trace pipeline.
4. No backwards-compatibility alias layer should preserve both the old and new trace semantics in parallel. Once the richer selection trace exists, downstream tests should assert against it directly.
5. Prefer provenance categories that match real runtime behavior over attempt-index coupling. The ticket should preserve room for cases where the final selected state is "kept current plan" or "continued without replanning," which are important architectural distinctions for long-lived plan debugging.

## Verification Layers

1. Selected plan step shape and terminal semantics are directly queryable from the final decision trace -> focused unit/runtime tests in `decision_trace.rs` and `agent_tick.rs`
2. Selection provenance distinguishes newly selected traced plan vs retained current plan vs snapshot-only continuation -> focused `agent_tick` traced-runtime tests
3. Final selected state distinguishes actionable-now vs deferred/progress-barrier state -> focused `agent_tick` traced-runtime tests
3. Human-readable dump output remains aligned with the structured trace model -> focused formatting tests in `decision_trace.rs`
4. Golden docs can point authors at the richer selected-plan surface without encouraging inference from action start failures -> minimal doc update in `docs/golden-e2e-testing.md` once the code lands

## What to Change

### 1. Extend the selection-side trace model

Augment `SelectionTrace` and/or `ExecutionTrace` so the final per-tick trace directly carries a canonical selected-plan record with explicit provenance:

1. the selected plan summary itself when a plan exists on the final selected path
2. the selected plan's `steps`
3. the selected plan's `terminal_kind`
4. explicit provenance for whether the final selected path came from:
   - a newly selected traced plan from this tick's search,
   - retention of the previously active current plan,
   - or snapshot-only continuation without new search selection
5. whether the final selected state is actionable now or deferred behind a progress barrier / no immediate executable step

The model must let a test answer "what exact plan is this agent following right now, and why this path?" without reverse-joining through goal keys and attempt lists.

### 2. Record selection provenance in `agent_tick`

Update `plan_and_validate_next_step_traced()` in `crates/worldwake-ai/src/agent_tick.rs` so when `select_best_plan(...)` returns a plan, the trace records:

1. the selected plan summary itself
2. provenance for whether it was newly adopted from this tick's traced search or retained from the existing runtime plan
3. when snapshot-only continuation bypasses full search, a first-class continuation provenance instead of only `plan_continued = true`
4. whether the current `next_step` is valid now
5. whether the agent is following an immediately actionable plan or a deferred/progress-barrier path

The trace should explain selection semantics without requiring callers to infer them from subsequent action start failures or by correlating multiple trace substructures.

### 3. Strengthen trace formatting and docs

Update `crates/worldwake-ai/src/decision_trace.rs` formatting helpers so the rendered dump includes selected-plan provenance and shape in a compact, deterministic form.

Update `docs/golden-e2e-testing.md` after the code lands so it explicitly recommends this richer selected-plan surface when debugging planner behavior, without overstating it as a full mixed-layer proof surface.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `docs/golden-e2e-testing.md` (modify)

## Out of Scope

- changing planner search semantics or ranking policy
- changing `ClaimOffice`, `DeclareSupport`, or political action legality
- adding a separate planner replay/debug subsystem outside the decision trace model
- golden scenario rewrites unrelated to traceability

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --lib`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. A traced planning outcome must expose the exact final selected plan shape without reverse inference from unrelated failure signals.
2. The trace must distinguish "selected goal exists" from "final selected plan provenance" from "selected plan is actionable now" from "selected plan is deferred."
3. Structured trace data and human-readable dump output must derive from the same canonical trace fields.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — add focused tests proving the formatted planning trace includes selected-plan provenance plus compact plan shape/terminal semantics. Rationale: the human-readable surface should stop hiding the exact final selected path.
2. `crates/worldwake-ai/src/agent_tick.rs` — add focused traced-runtime tests proving the final trace exposes the selected plan summary directly and does not require reverse-correlation through `PlanSearchTrace.attempts`. Rationale: this is the core missing machine-queryable relationship.
3. `crates/worldwake-ai/src/agent_tick.rs` — add focused traced-runtime tests proving provenance distinguishes newly selected traced plan vs retained current plan vs snapshot-only continuation when applicable. Rationale: this is the actual architectural gap in the current surface.
4. `crates/worldwake-ai/src/agent_tick.rs` — add focused tests proving the trace distinguishes immediate actionable selection from deferred/progress-barrier selection. Rationale: this was the missing debugging surface encountered in S13POLEMEGOLSUI-002.
5. `docs/golden-e2e-testing.md` — document the richer selected-plan surface once implemented. Rationale: golden authors should know to assert selected-plan semantics directly instead of inferring them from failed starts.

### Commands

1. `cargo test -p worldwake-ai --lib`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-18
- What actually changed:
  - Added first-class selected-plan trace data in `crates/worldwake-ai/src/decision_trace.rs` via `SelectedPlanTrace` and `SelectedPlanSource`.
  - Updated `crates/worldwake-ai/src/agent_tick.rs` so traced planning now records canonical selected-plan provenance for fresh search selection, retained current-plan selection, and snapshot-only continuation.
  - Updated decision-trace summary/dump formatting to surface selected-plan provenance, terminal semantics, and compact path shape directly.
  - Added focused tests covering direct selected-plan exposure, provenance classification, and snapshot-continuation trace behavior.
  - Updated `docs/golden-e2e-testing.md` to point golden authors at the richer selected-plan trace surface.
- Deviations from original plan:
  - The implementation intentionally did not add a selected-attempt id as the primary abstraction. Current architecture can legally retain the existing runtime plan or continue via snapshot-only revalidation, so first-class provenance is cleaner and more extensible than attempt-index coupling.
  - The actionable-vs-deferred distinction is expressed through the canonical selected plan's `terminal_kind`, `next_step`, and the existing execution trace fields rather than a second parallel state taxonomy.
- Verification results:
  - Focused unit coverage:
    - `cargo test -p worldwake-ai --lib agent_tick::tests::determine_selected_plan_source_distinguishes_search_selection_from_retention -- --exact`
    - `cargo test -p worldwake-ai --lib agent_tick::tests::trace_planning_outcome_for_hungry_agent -- --exact`
    - `cargo test -p worldwake-ai --lib agent_tick::tests::trace_snapshot_continuation_records_selected_plan_provenance -- --exact`
    - `cargo test -p worldwake-ai --lib decision_trace::tests::summary_planning_includes_candidate_count -- --exact`
  - Broad verification:
    - `cargo test -p worldwake-ai --lib`
    - `cargo test -p worldwake-ai`
    - `cargo test --workspace`
    - `cargo clippy --workspace --all-targets -- -D warnings`
