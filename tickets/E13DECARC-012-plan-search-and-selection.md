# E13DECARC-012: Bounded plan search algorithm and plan selection

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — AI-layer logic
**Deps**: E13DECARC-009, E13DECARC-010, E13DECARC-011

## Problem

For each top-ranked grounded candidate, the planner must run a deterministic bounded best-first search over `PlanningState`, using `get_affordances()` as the sole successor generator, filtering to relevant `PlannerOpKind`s, and honoring budget limits. Plan selection then chooses the best valid plan using ranked candidates and deterministic lexicographic ordering with anti-thrashing rules.

## Assumption Reassessment (2026-03-11)

1. `PlanningSnapshot` and `PlanningState` from E13DECARC-011 implement `BeliefView`.
2. `get_affordances()` in `worldwake-sim` accepts `&dyn BeliefView` — confirmed.
3. `PlannerOpSemantics` and semantics table from E13DECARC-010.
4. `PlanningBudget` with `max_plan_depth`, `max_node_expansions`, `beam_width` from E13DECARC-009.
5. `PlannedStep`, `PlannedPlan`, `PlanTerminalKind` from E13DECARC-009.
6. `GoalSemantics` trait (`is_satisfied`, `is_progress_barrier`) from E13DECARC-010.

## Architecture Check

1. Search is deterministic best-first with explicit budget caps.
2. Successors come ONLY from `get_affordances(&planning_state, actor, registry)` — no custom precondition logic.
3. Successors filtered to current goal's relevant `PlannerOpKind`s.
4. Search stops when goal satisfied OR valid progress barrier reached.
5. Budget exhaustion yields no plan for that candidate (no partial invalid plans).
6. Plan selection uses anti-thrashing: current valid plan not replaced unless new plan is strictly higher priority class or beats by switch margin.

## What to Change

### 1. Implement plan search in `worldwake-ai/src/search.rs`

```rust
pub fn search_plan(
    snapshot: &PlanningSnapshot,
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    budget: &PlanningBudget,
) -> Option<PlannedPlan>
```

Algorithm:
1. Build initial `PlanningState` from snapshot
2. Run deterministic bounded best-first search:
   - Generate successors via `get_affordances(&state, actor, registry)`
   - Filter to goal's `relevant_op_kinds()`
   - Look up each affordance in semantics table
   - For each valid successor:
     - Estimate duration via `state.estimate_duration()`
     - If duration unavailable for mid-plan action, branch is invalid
     - If indefinite (combat), only valid as leaf
     - Apply op to get next `PlanningState`
     - Check if goal satisfied or progress barrier reached
3. Respect budgets:
   - `max_plan_depth`: maximum steps in a plan
   - `max_node_expansions`: total nodes expanded
   - `beam_width`: maximum successors to keep per level
4. On budget exhaustion: return `None`

### 2. Implement goal satisfaction checks

Full implementations of `GoalSemantics` for each `GoalKind`:
- `ConsumeOwnedCommodity`: relevant drive below medium band in planning state
- `AcquireCommodity(SelfConsume)`: actor controls commodity locally, or at progress barrier
- `ReduceDanger`: no current attackers and danger below high band
- `ProduceCommodity`: production step completes
- `SellCommodity`: sell step completes
- etc.

### 3. Implement plan selection in `worldwake-ai/src/plan_selection.rs`

```rust
pub fn select_best_plan(
    candidates: &[RankedGoal],
    plans: &[(GoalKey, Option<PlannedPlan>)],
    current: &AgentDecisionRuntime,
    budget: &PlanningBudget,
) -> Option<PlannedPlan>
```

Selection rules:
1. Choose by highest `GoalPriorityClass`
2. Then highest `motive_score`
3. Then lowest `total_estimated_ticks`
4. Then deterministic step-sequence ordering

Anti-thrashing:
- Don't replace current valid plan unless:
  - Current plan became invalid
  - New plan is strictly higher priority class
  - Same-class new plan beats current by `switch_margin_permille`

### 4. Handle materialization barriers

When a barrier step is the last step:
- Plan ends there with `PlanTerminalKind::ProgressBarrier`
- Top-level goal remains active
- Replanning occurs next tick from new beliefs

## Files to Touch

- `crates/worldwake-ai/src/search.rs` (modify — was empty stub)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — was empty stub)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — full `GoalSemantics` implementations)

## Out of Scope

- Plan revalidation — E13DECARC-014
- Failure handling / BlockedIntent writing — E13DECARC-013
- Reactive interrupts — E13DECARC-015
- Agent tick integration — E13DECARC-016
- Unconstrained generic search across every action family

## Acceptance Criteria

### Tests That Must Pass

1. Search with simple eat goal: agent has food at same place -> 1-step Consume plan
2. Search with travel+eat: agent has food at adjacent place -> Travel + Consume plan
3. Search with acquire: agent lacks food, seller exists -> Travel + TradeAcquire plan (ends at barrier)
4. Search respects `max_plan_depth` — returns None when depth exceeded
5. Search respects `max_node_expansions` — returns None when budget exhausted
6. Search filters to relevant `PlannerOpKind`s — Attack not considered for Consume goal
7. Materialization barrier ends plan with `PlanTerminalKind::ProgressBarrier`
8. Indefinite combat only appears as leaf step
9. Duration estimation failure invalidates the branch
10. Plan selection picks highest priority class first
11. Plan selection does not replace current plan unless switch margin exceeded
12. Plan selection is fully deterministic (same inputs -> same output)
13. Goal satisfaction predicates work correctly for at least ConsumeOwnedCommodity, AcquireCommodity, ReduceDanger
14. Budget exhaustion returns `None`, never a partial invalid plan
15. Existing suite: `cargo test --workspace`

### Invariants

1. Successors come only from `get_affordances()` — no custom precondition duplication
2. Search is deterministic and bounded
3. No full world clone per search node
4. Budget exhaustion never returns partial invalid plans
5. Anti-thrashing prevents same-class plan switching below margin
6. All collections use `BTreeMap`/`BTreeSet`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` — search algorithm tests with various scenarios
2. `crates/worldwake-ai/src/plan_selection.rs` — selection and anti-thrashing tests

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
