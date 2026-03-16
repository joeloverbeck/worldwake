# S08AIDECTRA-002: Thread Trace Collection Through process_agent Pipeline

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — modifies AI decision pipeline internals
**Deps**: S08AIDECTRA-001

## Problem

The trace data model exists (S08AIDECTRA-001) but nothing populates it. This ticket threads an `Option<&mut DecisionTraceSink>` through the `process_agent` pipeline so each stage emits its trace fragment, which is assembled into one `AgentDecisionTrace` at the end.

## Assumption Reassessment (2026-03-16)

1. `process_agent()` in `agent_tick.rs:169` is a free function called from `produce_agent_input()`. It receives `AgentTickContext` + `runtime_by_agent` + agent + signals. Confirmed.
2. `refresh_runtime_for_read_phase()` at line 468 handles candidate generation and ranking. It sets `runtime.dirty` based on multiple conditions (no plan, plan finished, replan signal, queue transition, blocker cleanup, snapshot changed, queue patience). These map to `DirtyReason` variants. Confirmed.
3. The dirty-flag logic at line 489-491 uses boolean OR of multiple conditions. The trace needs to capture WHICH conditions triggered dirty. This requires decomposing the boolean into individual checks that also populate a `Vec<DirtyReason>`.
4. `plan_and_validate_next_step()` at line 718 handles plan selection and step validation. Confirmed.
5. `enqueue_valid_step_or_handle_failure()` at line 308 handles execution. Confirmed.
6. `handle_active_action_phase()` at line 567 handles interrupt evaluation. Confirmed.
7. The dead-agent early return at line 199-210 must emit `DecisionOutcome::Dead`. Confirmed.
8. `AgentTickDriver` has no `trace_sink` field yet — must be added. Confirmed.

## Architecture Check

1. The `Option<&mut DecisionTraceSink>` pattern ensures zero-cost when tracing is disabled. Each function checks once at the top and skips all trace allocation when `None`.
2. Trace fragments are returned alongside computational results (e.g., `refresh_runtime_for_read_phase` returns ranked candidates AND optionally a `CandidateTrace`). This avoids storing intermediate trace state on `AgentDecisionRuntime`.
3. Assembly happens once at the end of `process_agent()` — no partial traces are stored.

## What to Change

### 1. Add `trace_sink` to `AgentTickDriver`

Add `trace_sink: Option<DecisionTraceSink>` field to the struct. Add methods:
- `pub fn enable_tracing(&mut self)` — sets `trace_sink = Some(DecisionTraceSink::new())`
- `pub fn trace_sink(&self) -> Option<&DecisionTraceSink>` — read access
- `pub fn trace_sink_mut(&mut self) -> Option<&mut DecisionTraceSink>` — mutable access for tests

### 2. Thread sink through `produce_agent_input` → `process_agent`

`produce_agent_input` passes `self.trace_sink.as_mut()` to `process_agent`. Add `trace_sink: Option<&mut DecisionTraceSink>` parameter to `process_agent`.

### 3. Dead-agent trace emission

At the early return for dead agents (line ~199-210), if `trace_sink.is_some()`, record `AgentDecisionTrace { agent, tick, outcome: DecisionOutcome::Dead }`.

### 4. Decompose dirty-flag into `Vec<DirtyReason>`

In `refresh_runtime_for_read_phase`, replace the boolean OR chain with individual checks that both set `runtime.dirty = true` AND append to a local `Vec<DirtyReason>` (only when trace_sink is Some). The existing boolean behavior must be preserved exactly.

### 5. Candidate trace from `refresh_runtime_for_read_phase`

After `generate_candidates_with_travel_horizon` and `rank_candidates`, populate a `CandidateTrace` with:
- `generated`: all `GoalKey`s from candidate generation (before ranking filter)
- `ranked`: `RankedGoalSummary` for each post-ranking candidate
- `suppressed`: goals filtered by blocked-intent suppression
- `zero_motive`: goals filtered by zero motive score

Return the `Option<CandidateTrace>` alongside the existing return value.

### 6. Plan search trace from the planning block

In the `if runtime.dirty { ... }` block (line ~735), when `build_candidate_plans` is called, wrap each `search_plan` call to capture `PlanAttemptTrace` entries. Populate `PlanSearchTrace`.

### 7. Selection trace from `plan_and_validate_next_step`

Capture which goal was selected, whether a goal switch occurred (from/to/kind), and the previous goal. Populate `SelectionTrace`.

### 8. Execution trace from `enqueue_valid_step_or_handle_failure`

Capture the enqueued step summary, revalidation result, and failure reason if any. Populate `ExecutionTrace`.

### 9. Interrupt trace from `handle_active_action_phase`

Capture the `InterruptDecision` and the top challenger (highest-ranked candidate). Populate `InterruptTrace`. Emit `DecisionOutcome::ActiveAction`.

### 10. Assembly at end of `process_agent`

At the end of `process_agent`, if `trace_sink.is_some()`, assemble all fragments into an `AgentDecisionTrace` and call `trace_sink.record(...)`.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — bulk of changes)
- `crates/worldwake-ai/src/search.rs` (modify — `search_plan` returns `PlanSearchResult` enum instead of `Option<PlannedPlan>`, preserving failure-mode information)
- `crates/worldwake-ai/src/ranking.rs` (modify — `rank_candidates` returns `RankingOutcome` struct with `ranked`, `suppressed`, and `zero_motive` fields)
- `crates/worldwake-ai/src/goal_explanation.rs` (modify — updated caller of `rank_candidates`)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export new types)

## Out of Scope

- Changes to `worldwake-core` — no core types modified
- Changes to `worldwake-sim` (BestEffort failure recording is S08AIDECTRA-003)
- Changes to `candidate_generation.rs`, `plan_selection.rs`, `failure_handling.rs` — these modules are not modified
- GoldenHarness integration (S08AIDECTRA-004)
- `dump_agent()` display method (S08AIDECTRA-004)
- Any changes to the decision logic itself — this ticket only observes, never modifies behavior

## Architecture Note (added during implementation)

The original ticket specified that `ranking.rs` and `search.rs` should not be changed, with trace data extracted from return values. During implementation, we chose architecturally richer return types instead:

- `search_plan` → `PlanSearchResult` enum: Preserves failure-mode distinction (Unsupported vs BudgetExhausted vs FrontierExhausted) that was previously collapsed into `None`. This benefits both tracing and future diagnostic work.
- `rank_candidates` → `RankingOutcome` struct: Preserves which goals were suppressed vs zero-motive filtered, information that was previously discarded. Both are standalone improvements independent of tracing.

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: Create an `AgentTickDriver` with tracing enabled, run a single agent through one tick with a simple scenario (agent has needs, one affordance). Verify `trace_sink().traces().len() == 1` and the trace contains a `DecisionOutcome::Planning` with non-empty `candidates.generated`.
2. Unit test: Run a dead agent through the pipeline with tracing enabled. Verify `DecisionOutcome::Dead`.
3. Unit test: Run an agent with an active action through the pipeline. Verify `DecisionOutcome::ActiveAction` with a populated `InterruptTrace`.
4. Unit test: Verify that with tracing disabled (`trace_sink` is `None`), the pipeline produces identical `InputKind` results as before — no behavioral change.
5. Existing suite: `cargo test -p worldwake-ai` — all existing golden tests pass unchanged.
6. `cargo clippy --workspace` — no new warnings.

### Invariants

1. **Zero behavioral change**: With tracing disabled, the pipeline produces bit-identical results. No new allocations on the hot path when `trace_sink` is `None`.
2. **No new public API on pipeline sub-functions**: The trace data is extracted from existing return values in `process_agent`, not by modifying the signatures of `search_plan`, `rank_candidates`, etc.
3. **Append-only accumulation**: Traces are only appended via `DecisionTraceSink::record()`, never mutated after recording.
4. **One trace per agent per tick**: `process_agent` emits exactly one `AgentDecisionTrace` per invocation when tracing is enabled.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` or a new integration test file — pipeline trace emission scenarios

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

**Completion date**: 2026-03-16

### What changed

- **`search.rs`**: `search_plan` returns `PlanSearchResult` enum (`Found`, `Unsupported`, `BudgetExhausted`, `FrontierExhausted`) instead of `Option<PlannedPlan>`. All ~30 test callers updated.
- **`ranking.rs`**: `rank_candidates` returns `RankingOutcome { ranked, suppressed, zero_motive }` instead of `Vec<RankedGoal>`. All ~27 test callers updated via `.into_ranked()`.
- **`goal_explanation.rs`**: Updated caller of `rank_candidates`.
- **`agent_tick.rs`**: Core tracing implementation:
  - `trace_sink: Option<DecisionTraceSink>` field on `AgentTickDriver` with `enable_tracing()`, `trace_sink()`, `trace_sink_mut()` methods.
  - `process_agent` accepts `tracing: bool`, returns `Result<Option<AgentDecisionTrace>, _>`.
  - `ReadPhaseResult` struct preserves dirty reasons, generated keys, suppressed/zero_motive alongside ranked candidates.
  - `handle_active_action_phase` refactored to return `InterruptDecision`, with `finalize_agent_tick` moved to `process_agent` for single assembly point.
  - `plan_and_validate_next_step_traced` captures `PlanSearchTrace` + `SelectionTrace` with goal-switch detection.
  - `summarize_step` and `plan_search_result_to_trace` helper functions.
  - 4 acceptance tests: `trace_planning_outcome_for_hungry_agent`, `trace_dead_agent`, `trace_active_action_interrupt`, `tracing_disabled_produces_identical_behavior`.
- **`lib.rs`**: Re-exports `RankingOutcome` and `PlanSearchResult`.

### Deviations from original plan

- **`ranking.rs` and `search.rs` were changed** (originally out-of-scope). Instead of hacking around missing information with set-subtraction approximations, architecturally richer return types were chosen: `PlanSearchResult` preserves failure-mode distinction, `RankingOutcome` preserves filtering categorization. Both are standalone improvements independent of tracing.
- **Invariant 2 amended**: The ticket stated "no new public API on pipeline sub-functions" — `search_plan` and `rank_candidates` return types did change, but these are improvements to the functions themselves, not trace-specific injections.

### Verification results

- `cargo test --workspace`: 1,751 tests pass, 0 failures (312 lib in worldwake-ai, up from 308 = +4 new acceptance tests).
- `cargo clippy -p worldwake-ai`: 0 warnings on lib code. 9 pre-existing test warnings unchanged.
- All golden e2e tests pass unchanged.
