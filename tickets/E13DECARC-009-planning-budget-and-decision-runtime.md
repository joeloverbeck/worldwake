# E13DECARC-009: PlanningBudget config, AgentDecisionRuntime, and PlannedStep/PlannedPlan types

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — AI-layer types
**Deps**: E13DECARC-004

## Problem

The planner needs explicit budget limits (max depth, max expansions, beam width, switch margin), the plan step type must store exact affordance-keyed data for lossless `InputKind::RequestAction` conversion, and runtime decision state must be held outside the authoritative world state.

## Assumption Reassessment (2026-03-11)

1. `ActionDefId` is an index type in `worldwake-sim` — confirmed.
2. `ActionPayload` enum exists in `worldwake-sim` — confirmed.
3. `InputKind::RequestAction { actor, def_id, targets, payload_override }` — confirmed.
4. `Permille` exists — confirmed.
5. `GoalKey`, `GoalPriorityClass` from E13DECARC-004.

## Architecture Check

1. `PlanningBudget` holds engineering budgets, not world-simulation laws.
2. `PlannedStep` stores EXACT `def_id`, `targets` (ordered), `payload_override` — matches `InputKind::RequestAction` losslessly.
3. `AgentDecisionRuntime` is NOT a component. It lives in scheduler/AI runtime, never in `component_schema.rs`.
4. Transient blocked-intent TTL constants are AI config, not world tuning.

## What to Change

### 1. Define `PlanningBudget` in `worldwake-ai/src/budget.rs`

```rust
pub struct PlanningBudget {
    pub max_candidates_to_plan: u8,
    pub max_plan_depth: u8,
    pub max_node_expansions: u16,
    pub beam_width: u8,
    pub switch_margin_permille: Permille,
    pub transient_block_ticks: u32,
    pub structural_block_ticks: u32,
}

impl Default for PlanningBudget {
    // max_candidates_to_plan = 4
    // max_plan_depth = 6
    // max_node_expansions = 128
    // beam_width = 8
    // switch_margin_permille = Permille(100)  // 10%
    // transient_block_ticks = 20
    // structural_block_ticks = 200
}
```

### 2. Define `PlannedStep` and `PlannedPlan` in `worldwake-ai/src/planner_ops.rs` or dedicated file

```rust
pub struct PlannedStep {
    pub def_id: ActionDefId,
    pub targets: Vec<EntityId>,
    pub payload_override: Option<ActionPayload>,
    pub op_kind: PlannerOpKind,
    pub estimated_ticks: u32,
    pub is_materialization_barrier: bool,
}

pub enum PlanTerminalKind {
    GoalSatisfied,
    ProgressBarrier,
}

pub struct PlannedPlan {
    pub goal: GoalKey,
    pub steps: Vec<PlannedStep>,
    pub total_estimated_ticks: u32,
    pub terminal_kind: PlanTerminalKind,
}
```

### 3. Implement `PlannedStep -> InputKind::RequestAction` conversion

```rust
impl PlannedStep {
    pub fn to_request_action(&self, actor: EntityId) -> InputKind { ... }
}
```

### 4. Define `AgentDecisionRuntime` in `worldwake-ai/src/decision_runtime.rs`

```rust
pub struct AgentDecisionRuntime {
    pub current_goal: Option<GoalKey>,
    pub current_plan: Option<PlannedPlan>,
    pub dirty: bool,
    pub last_priority_class: Option<GoalPriorityClass>,
}
```

This must NOT be registered in `component_schema.rs`.

## Files to Touch

- `crates/worldwake-ai/src/budget.rs` (modify — was empty stub)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — was empty stub, add PlannedStep/PlannedPlan)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — was empty stub)
- `crates/worldwake-ai/src/lib.rs` (modify — re-exports)

## Out of Scope

- `PlannerOpKind` enum and semantics table — E13DECARC-010
- `PlanningSnapshot` / `PlanningState` — E13DECARC-011
- Plan search algorithm — E13DECARC-012
- Plan revalidation — E13DECARC-014
- Registering any of these as components (they are transient AI runtime data)

## Acceptance Criteria

### Tests That Must Pass

1. `PlanningBudget::default()` produces the documented prototype values
2. `PlannedStep::to_request_action(actor)` produces matching `InputKind::RequestAction` with same `def_id`, `targets`, `payload_override`
3. `PlannedPlan.total_estimated_ticks` equals sum of step `estimated_ticks`
4. `AgentDecisionRuntime::default()` has `current_goal = None`, `dirty = false`
5. `AgentDecisionRuntime` is NOT in `component_schema.rs` (grep-verify)
6. All types round-trip through bincode (except `AgentDecisionRuntime` which doesn't need to)
7. Existing suite: `cargo test --workspace`

### Invariants

1. `PlannedStep` stores exact ordered `targets` — not semantic placeholders
2. `AgentDecisionRuntime` is never registered as a component
3. `PlanningBudget` values are engineering budgets, not world-simulation laws
4. No `HashMap`/`HashSet` usage

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/budget.rs` — default values test
2. `crates/worldwake-ai/src/planner_ops.rs` — PlannedStep conversion test, PlannedPlan ticks summation
3. `crates/worldwake-ai/src/decision_runtime.rs` — default state test

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
