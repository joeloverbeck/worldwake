# S39LIMSIDBEN-002: Integrate side-benefits into plan selection and traces

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI plan-selection scoring, decision-trace surfaces, and selection call-site wiring
**Deps**: [archive/tickets/S39LIMSIDBEN-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/S39LIMSIDBEN-001.md), [archive/specs/S33-opportunity-scoped-goal-identity.md](/home/joeloverbeck/projects/worldwake/archive/specs/S33-opportunity-scoped-goal-identity.md), [archive/specs/S22-generalized-intention-frames.md](/home/joeloverbeck/projects/worldwake/archive/specs/S22-generalized-intention-frames.md)

## Problem

Even after the pure side-benefit substrate exists, the live AI selection path still ignores it. [`select_best_plan()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs) currently chooses among searched plans using only priority class, primary motive, and plan cost, and the selection trace cannot explain any future side-benefit tie-breaks. Without this ticket, `S39` would have a scoring helper that never affects actual plan choice or decision-trace debugging.

## Assumption Reassessment (2026-04-02)

1. The canonical selection boundary is still [`select_best_plan()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs). It receives `&[RankedGoal]`, searched plans, the current runtime plan, frame context, and switch margins, so post-search side-benefit scoring can still stay at this layer without modifying search itself.
2. The live sort helper [`compare_ranked_plans`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs) currently compares only `GoalPriorityClass`, `motive_score`, `total_estimated_ticks`, `steps`, and `goal`. There is no current slot for `PlanValue.total_value`.
3. The exact shared abstraction boundary under audit is AI post-search selection: ranked candidates from generation/search on one side, and final `PlannedPlan` retention/switching on the other. This ticket must keep that boundary canonical rather than adding a second side-benefit-based planner or search path.
4. Goal switching remains governed by [`compare_relation_aware_goal_switch`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/frame_switch_policy.rs) and the frame-aware relation logic already in [`plan_selection.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs). Under the spec and `FOUNDATIONS`, side-benefits must not alter the primary motive used for switch-margin decisions.
5. The trace surface is currently missing any side-benefit provenance. [`SelectionTrace`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) and [`SelectedPlanTrace`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) only expose selected opportunity, selected plan, source, goal switch, previous goal, and replacement summary.
6. The live selection-call-site wiring already loads `UtilityProfile` in runtime planning. [`agent_tick/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs) reads `UtilityProfile` from authoritative world state before planning/selection, so threading `side_benefit_weight` through existing selection calls matches the current per-agent-parameter pattern.
7. This is a mixed-layer AI ticket, but the boundary stays inside AI. Candidate generation and planner search remain unchanged; only post-search selection and trace summarization move.
8. If ordering diverges after this change, the intended ordering layer is plan-selection ordering, not action-lifecycle ordering or authoritative world-state timing. The compared branches are symmetric only when primary `priority_class` and primary `motive_score` are equal; side-benefits should only break ties inside that existing selection layer.
9. Mismatch + correction: the spec describes adding side-benefits to `SelectionTrace`, but the live code also needs a winning-plan summary path through [`summarize_selected_plan()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) and the planning call sites. This ticket therefore owns both selection logic and trace/model fallout across AI modules, not just `plan_selection.rs`.

## Architecture Check

1. Keeping side-benefits as a post-search overlay inside `select_best_plan()` is cleaner than modifying planner search or candidate generation because it preserves single-goal search semantics while still allowing bounded multi-benefit tie-breaking.
2. No backwards-compatibility shims are introduced. There is one canonical selection path, one canonical side-benefit substrate from `001`, and one trace surface for explaining the resulting choice.

## Verification Layers

1. Same-priority, same-primary-motive plans can be reordered by bounded side-benefit value -> focused `worldwake-ai` plan-selection tests in [`plan_selection.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs)
2. Higher `GoalPriorityClass` still beats lower-class plans even when the lower-class branch has larger side-benefits -> focused `worldwake-ai` plan-selection tests
3. Goal switching and frame retention still use primary motive only, not `total_value` -> focused `worldwake-ai` plan-selection tests covering current-plan retention and frame-aware switching
4. Selected-plan and decision-trace summaries surface side-benefit provenance for the winning branch -> focused AI trace/model tests in [`decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) and [`agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
5. If trace formatting proves the outcome but not enough provenance, the strongest lower-layer proof remains the focused selection tests on `select_best_plan()`; no follow-up traceability ticket is needed if winning-plan side-benefit summaries are exposed here

## What to Change

### 1. Integrate `PlanValue` into plan selection

Thread `side_benefit_weight` into [`select_best_plan()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs), compute `PlanValue` for each searched plan using the shared substrate from `001`, and update the same-priority, same-primary-motive comparison slot so bounded `total_value` can break ties without overriding priority class or frame-switch rules.

### 2. Extend selection and plan traces

Add side-benefit trace structures to the AI selection/debug surfaces and thread them through the winning-plan summary path so traces can explain which secondary goals were recognized and how they influenced `total_value`.

### 3. Update AI call sites and direct test constructors

Propagate the new selection parameter and trace fields through [`agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), [`agent_tick/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs), and any sibling test literals or constructors that build `SelectionTrace`, `SelectedPlanTrace`, or related ranked-goal summaries.

## Files to Touch

- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs` (modify if shared ranked-goal summary/debug fields need to carry the new trace data)

## Out of Scope

- Changing candidate generation, planner search, or action execution to explicitly add new steps for side benefits
- Any golden/E2E coverage proving market-trip behavior
- Any authoritative-world or system-layer changes outside AI selection/debug surfaces

## Acceptance Criteria

### Tests That Must Pass

1. Side-benefits can break ties between equal-priority, equal-primary-motive plans without changing the winning branch when priority class alone should decide the outcome.
2. Goal-switch margin behavior remains keyed to primary motive only, even when `total_value` differs.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Side-benefits never create new plans or modify searched plans; they only rescore already-found plans at the post-search selection boundary.
2. There is one canonical explanatory trace for side-benefit-influenced selection; no second debug-only or heuristic-only path is introduced.

## Test Plan

### New/Modified Tests

1. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs` — focused selection tests for tie-breaking, cap behavior, and “priority/switch margin still use primary motive only”.
2. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs` and `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs` — summary/formatting tests proving the winning plan carries side-benefit provenance.

### Commands

1. `cargo test -p worldwake-ai plan_selection::tests::selection_prefers_higher_priority_class_before_cost -- --exact --nocapture`
2. `cargo test -p worldwake-ai decision_trace -- --nocapture`
3. `cargo test -p worldwake-ai`
