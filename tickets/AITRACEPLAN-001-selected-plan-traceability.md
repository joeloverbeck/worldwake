# AITRACEPLAN-001: Expose Selected Plan Shape and Fallback Semantics in Decision Traces

**Status**: PENDING
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
2. `plan_and_validate_next_step_traced()` in `crates/worldwake-ai/src/agent_tick.rs` builds `PlanSearchTrace.attempts`, runs `select_best_plan(...)`, and stores only `selection_trace.selected = Some(selected_goal)`. The trace therefore loses the direct linkage between the selected goal and the exact found attempt/step sequence that won selection.
3. `ExecutionTrace` records only `enqueued_step`, `revalidation_passed`, and broad execution failures. It does not explicitly distinguish whether the selected plan was immediately actionable, a deferred/progress-barrier path, or a fallback that happened to be best among bad options.
4. Existing focused trace coverage in `crates/worldwake-ai/src/decision_trace.rs` and `crates/worldwake-ai/src/agent_tick.rs` proves summaries, sink behavior, and some planning traces, but there is no focused test asserting that a selected plan's step list and terminal semantics are directly available from the final trace surface.
5. `docs/golden-e2e-testing.md` already tells authors to use decision traces for planner-search behavior, but it does not currently guarantee that the trace surface exposes selected-plan shape strongly enough for that guidance to be consistently useful. The gap is in the trace model first, and secondarily in trace documentation.

## Architecture Check

1. The clean solution is to extend the structured trace model so selected-plan semantics are first-class trace data, not an inferred relationship between `selection.selected` and a separate search-attempt list. That keeps the trace surface authoritative and machine-queryable.
2. Do not solve this with ad hoc test-only `eprintln!` dumps or bespoke golden-only helpers. The missing data belongs in the trace model itself so all focused tests, golden tests, and future debugging tools consume one canonical surface.
3. Do not duplicate the full planner state or add a second debug-only planner log. The robust architecture is to expose the already-selected plan and the reason-class of that selection in the existing trace pipeline.
4. No backwards-compatibility alias layer should preserve both the old and new trace semantics in parallel. Once the richer selection trace exists, downstream tests should assert against it directly.

## Verification Layers

1. Selected plan step shape is directly queryable from the final decision trace -> focused unit/runtime tests in `decision_trace.rs` and `agent_tick.rs`
2. Selection semantics distinguish actionable-now vs deferred/fallback plan states -> focused `agent_tick` tests over traced planning outcomes
3. Human-readable dump output remains aligned with the structured trace model -> focused formatting tests in `decision_trace.rs`
4. Golden tests can use the richer trace without inferring plan shape from action start failures -> doc update in `docs/golden-e2e-testing.md` once the code lands

## What to Change

### 1. Extend the selection-side trace model

Augment `SelectionTrace` and/or `ExecutionTrace` so the final per-tick trace directly carries:

1. the selected found attempt or an explicit summary of the selected plan
2. the selected plan's `steps`
3. the selected plan's `terminal_kind`
4. whether the chosen plan is immediately actionable, deferred behind a barrier, or a fallback/non-actionable selection state

The model must let a test answer "what exact plan won?" without reverse-joining through goal keys and attempt lists.

### 2. Record selection provenance in `agent_tick`

Update `plan_and_validate_next_step_traced()` in `crates/worldwake-ai/src/agent_tick.rs` so when `select_best_plan(...)` returns a plan, the trace records:

1. which search attempt it came from
2. the selected plan summary itself
3. whether the current `next_step` is valid now
4. whether the agent is continuing an existing validated plan vs adopting a new plan vs selecting a deferred/fallback path

The trace should explain selection semantics without requiring callers to infer them from subsequent action start failures.

### 3. Strengthen trace formatting and docs

Update `crates/worldwake-ai/src/decision_trace.rs` formatting helpers so the rendered dump includes selected-plan shape in a compact, deterministic form.

Update `docs/golden-e2e-testing.md` after the code lands so it explicitly recommends this richer selected-plan surface when debugging planner behavior.

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

1. A traced planning outcome must expose the exact selected plan shape without reverse inference from unrelated failure signals.
2. The trace must distinguish "selected goal exists" from "selected plan is actionable now" from "selected plan is deferred/fallback."
3. Structured trace data and human-readable dump output must derive from the same canonical trace fields.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — add focused tests proving the formatted planning trace includes selected-plan steps and terminal semantics. Rationale: the human-readable surface should stop hiding the exact plan that won.
2. `crates/worldwake-ai/src/agent_tick.rs` — add focused traced-runtime tests proving the selected plan is directly linked in the final trace and not only recoverable from `PlanSearchTrace.attempts`. Rationale: this is the core missing machine-queryable relationship.
3. `crates/worldwake-ai/src/agent_tick.rs` — add focused tests proving the trace distinguishes immediate actionable selection from deferred/barrier/fallback states. Rationale: this was the gap encountered while debugging S13POLEMEGOLSUI-002.
4. `docs/golden-e2e-testing.md` — document the richer trace surface once implemented. Rationale: golden authors should know to assert selected-plan shape directly instead of inferring it from failed starts.

### Commands

1. `cargo test -p worldwake-ai --lib`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
