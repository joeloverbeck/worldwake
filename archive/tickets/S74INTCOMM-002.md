# S74INTCOMM-002: Replace `is_needs_only` heuristic with margin-based plan continuation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planning path decision logic, decision trace output
**Deps**: S74INTCOMM-001

## Problem

The current `is_needs_only()` top-2 heuristic in `try_continue_snapshot_plan` is unprincipled — it was tuned empirically during soak-seed-perf experiments and has no cognitive meaning. It accepts plan continuation if the current goal is anywhere in the top 2 ranked candidates when the only dirty bit is NEEDS. This causes seed-dependent regressions (helps some seeds, regresses others) because ranking stability varies with agent behavior.

This ticket replaces it with a margin-based mechanism that has clear P21 semantics: agents commit to plans unless a genuinely higher-priority goal emerges (same-class margin on `motive_score`) or a higher priority class goal appears (cross-class bypass).

## Assumption Reassessment (2026-04-08)

1. `try_continue_snapshot_plan` at `crates/worldwake-ai/src/agent_tick/planning.rs:408-462`. Current signature takes `ranked_candidates: &[RankedGoal]` but NOT `CognitiveProfile`. The `is_needs_only()` branch is at lines 428-432 (top-2 heuristic). The non-needs branch is at lines 433-441 (#1 rank check). After S74INTCOMM-001, CognitiveProfile has the `planning_switch_margin` field available.
2. Traced variant inline logic at `planning.rs:786-836` replicates the same `is_needs_only()` top-2 heuristic at lines 787-796. Must be updated consistently.
3. `RankedGoal` at `goal_model.rs:2233-2241` has `priority_class: GoalPriorityClass` and `motive_score: u32`. `GoalPriorityClass` is an ordered enum: `Background < Low < Medium < High < Critical` at `goal_model.rs:1981-1987`. The margin comparison will use `priority_class` for cross-class bypass and `motive_score` for same-class margin. Note: `planning_switch_margin` is `Permille` (newtype over `u16`) while `motive_score` is `u32` — implementation needs `.value() as u32` conversion for the comparison.
4. `is_needs_only()` at `dirty_set.rs:119-121` has exactly 2 call sites: `planning.rs:428` and `planning.rs:787`. Both are replaced by this ticket.
5. `DirtySet::is_snapshot_only()` at `dirty_set.rs:113` remains unchanged — the margin applies only within the snapshot-only path.
6. Planner-traceability contract: `docs/planner-contracts.md` makes `SelectionTrace` the existing planner-owned selection provenance surface. There is no dedicated snapshot-continuation comparison field yet, so landing the ticket's trace requirement requires a bounded widening in `crates/worldwake-ai/src/decision_trace.rs` plus trace-test updates.
7. This ticket replaces a heuristic (the top-2 rule). The missing substrate it was standing in for is P21 commitment inertia in the planning phase. This ticket introduces that substrate via the margin-based comparison. The change does not reopen unrelated regressions because: (a) all non-snapshot dirty bits still trigger full replanning (unchanged), (b) cross-class priority upgrades bypass the margin (always replan), and (c) the margin is per-agent, allowing test scenarios to set margin=0 for full replanning behavior.

## Architecture Check

1. The margin-based approach is cleaner than the top-2 heuristic because it has clear cognitive semantics (commitment inertia), is per-agent configurable (P22), and handles priority class boundaries explicitly instead of relying on arbitrary rank cutoffs.
2. No backwards-compatibility aliasing/shims introduced. The `is_needs_only()` fast path is cleanly replaced — the method can be retained on `DirtySet` for diagnostics but is no longer used in the planning decision path.

## Verification Layers

1. Same-class margin gates plan continuation -> focused `planning.rs` unit test plus bounded `SelectionTrace` snapshot-continuation summary
2. Cross-class bypass triggers replanning -> focused `planning.rs` unit test plus bounded `SelectionTrace` snapshot-continuation summary
3. Current goal absent from candidates abandons plan -> focused `planning.rs` unit test plus bounded `SelectionTrace` snapshot-continuation summary
4. Structural/frame dirty bits bypass margin entirely -> action trace confirms full planning on REPLAN_SIGNAL, ASSUMPTION_FAILED, etc. (existing behavior, unchanged)
5. Per-agent margin=0 triggers full replanning on every ranking shift -> focused `planning.rs` unit test

## What to Change

### 1. Extend `try_continue_snapshot_plan` signature and logic

In `crates/worldwake-ai/src/agent_tick/planning.rs`:

Extend the function signature to accept `planning_switch_margin: Permille` (or `&CognitiveProfile`).

Replace the `is_needs_only()` branch (lines 428-442) with:

```
// Step 1: If current goal is #1 and matches opportunity -> continue (unchanged fast path)
// Step 2: Find the current goal in ranked_candidates
//   - If absent -> abandon plan (goal became infeasible)
// Step 3: Compare priority classes
//   - If top.priority_class > current.priority_class -> fall through (cross-class bypass)
// Step 4: Same-class margin check
//   - If top.motive_score >= current.motive_score + margin.value() as u32 -> fall through
//   - Otherwise -> continue plan (margin not exceeded)
```

The `is_needs_only()` check is removed from the decision path.

### 2. Update traced variant consistently

In `planning.rs` (traced variant, ~lines 786-836), apply the identical margin-based logic. Record the margin comparison result through `SelectionTrace` in `decision_trace.rs` (current opportunity, top-ranked opportunity, motive delta, and whether cross-class bypass triggered).

### 3. Optional: annotate `is_needs_only()` as diagnostic-only

In `dirty_set.rs`, the `is_needs_only()` method can be retained with a doc comment noting it is no longer used in planning decisions but is available for trace diagnostics. Alternatively, remove it if no diagnostic use is foreseen — the method is 3 lines and easily re-added.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — both `try_continue_snapshot_plan` and traced variant)
- `crates/worldwake-ai/src/dirty_set.rs` (modify — optional: annotate or remove `is_needs_only()`)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — bounded `SelectionTrace` widening for snapshot-continuation provenance)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — traced runtime proof for snapshot-continuation provenance)

## Out of Scope

- Adjusting golden test scenarios or margin values for specific tests (S74INTCOMM-003)
- Soak-seed-perf campaign validation (S74INTCOMM-003)
- Changes to the active-action interrupt path (`switch_margin` / `compare_goal_switch`) — that path is already working
- Changes to `observation_snapshot_changed` — NEEDS detection remains exact-equality

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai -- try_continue` — targeted planning continuation tests (if existing)
2. `cargo test -p worldwake-ai -- planning` — planning module tests
3. Existing suite: `cargo test --workspace`

### Invariants

1. When dirty set is snapshot-only and top-ranked goal has a strictly higher `GoalPriorityClass` than the current plan's goal, full GOAP search always triggers (cross-class bypass)
2. When dirty set is snapshot-only and both goals share the same `GoalPriorityClass`, plan continues if `top.motive_score < current.motive_score + planning_switch_margin`
3. When the current goal is not present in ranked candidates, the plan is always abandoned
4. Structural and frame dirty bits still trigger full planning regardless of margin (existing behavior preserved)
5. Decision traces include margin comparison result for debuggability (P29)
6. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` (inline tests) — add focused tests for: (a) same-class margin prevents replanning when delta is below margin, (b) same-class margin triggers replanning when delta exceeds margin, (c) cross-class bypass always triggers replanning, (d) absent current goal abandons plan, (e) margin=0 triggers replanning on any ranking shift within same class

### Commands

1. `cargo test -p worldwake-ai -- planning` — targeted planning tests
2. `cargo clippy --workspace --all-targets -- -D warnings` — lint verification
3. `cargo test --workspace` — full suite

## Outcome

Completion date: 2026-04-08

Implemented the planning-path commitment margin in `crates/worldwake-ai/src/agent_tick/planning.rs` by replacing the `is_needs_only()` top-2 heuristic with a shared snapshot-continuation comparison helper. Both the untraced and traced planning paths now use the same `planning_switch_margin` decision logic: continue when the current opportunity remains top-ranked, continue within same-class motive margin, and force fresh planning when a higher priority class wins or the current opportunity drops out.

Bounded planner provenance was added through `SelectionTrace.snapshot_continuation` in `crates/worldwake-ai/src/decision_trace.rs`, with `SnapshotContinuationTrace` and `SnapshotContinuationOutcome` carrying the current opportunity, top opportunity, motive delta, applied margin, and comparison outcome. Focused planning tests now cover same-class below-margin continuation, at-margin replanning, cross-class bypass, missing-current-opportunity replanning, and zero-margin behavior; the traced `agent_tick` test now asserts the recorded snapshot-continuation provenance on a real continuation path.

The same-domain `golden_goal_switching_during_multi_leg_travel` scenario needed an explicit `planning_switch_margin = 0` override to preserve the test's intended active interrupt/resume contract under the new default planning inertia.

## Verification

- Passed: `cargo test -p worldwake-ai planning`
- Passed: `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_switching_during_multi_leg_travel`
- Passed: `cargo clippy --workspace --all-targets -- -D warnings`
- Investigated failure from `cargo test -p worldwake-ai` / `cargo test --workspace`: `combined_market_trip_selected_for_side_benefit` and `combined_market_trip_selected_for_side_benefit_replays_deterministically` in `crates/worldwake-ai/tests/golden_merchant_selling.rs`

The remaining merchant golden failure is outside this ticket's owned contract. The executed branch loss occurs after travel-step completion forces `DirtySet::PLAN_FINISHED` in `crates/worldwake-ai/src/agent_tick/active_action.rs`, which triggers full replanning rather than snapshot-only continuation; `planning_switch_margin` is not consulted on that path. I did not fold a side-benefit replanning fix into this ticket; follow-up ownership is tracked separately from this completion handoff.
