# E13DECARC-009: PlanningBudget config, AgentDecisionRuntime, and affordance-keyed plan types

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None - AI-layer types
**Deps**: E13DECARC-004, E13DECARC-005

## Problem

E13 still needs three missing AI-side foundations:

1. bounded planning budgets that are clearly engineering/runtime limits rather than world laws
2. exact affordance-keyed plan steps that can be converted losslessly into `InputKind::RequestAction`
3. runtime-only decision state for the active goal and active plan, stored outside authoritative world state

Without these types, later tickets cannot implement plan search, revalidation, failure handling, or agent-tick integration cleanly.

## Assumption Reassessment (2026-03-11)

1. `worldwake-ai` currently contains `candidate_generation`, `goal_model`, `pressure`, `ranking`, and `enterprise`; the ticket's original target files do not exist yet and must be created.
2. `worldwake-ai/Cargo.toml` already depends on `worldwake-sim`; the original assumption that this dependency was still missing is stale.
3. `GoalKey` is owned by `worldwake-core`, not `worldwake-ai`, because authoritative blocked-intent memory depends on it.
4. `BeliefView` already includes the E13DECARC-005 extensions, including `estimate_duration(&DurationExpr, ...)`.
5. `InputKind::RequestAction { actor, def_id, targets, payload_override }` is the required AI output shape and already exists in `worldwake-sim`.
6. `ActionDefId`, `ActionPayload`, and `Permille` already exist and satisfy the value/serialization requirements needed here.
7. `PlannerOpKind` does not exist yet. Keeping it out of this ticket is cleaner because planner semantics belong to E13DECARC-010, while 009 should stay focused on execution-facing step and runtime containers.

## Architecture Check

1. `PlanningBudget` is AI runtime configuration, not authoritative world state and not a simulation law.
2. `PlannedStep` must store the exact executable affordance identity: `def_id`, ordered `targets`, and `payload_override`. No semantic placeholders, aliases, or lossy reconstruction.
3. `PlannerOpKind` should not be embedded into `PlannedStep` in this ticket. Execution identity and planner semantics are different concerns; coupling them here would create an unnecessary dependency on E13DECARC-010.
4. `PlannedPlan` should maintain the invariant that total estimated ticks are derived from its steps, not hand-maintained separately by callers.
5. `AgentDecisionRuntime` is transient scheduler/AI state only. It must not be registered in `component_schema.rs` or stored as an authoritative component.

## Proposed Scope Decision

The proposed direction remains more beneficial than the current architecture, with one correction:

- Adding `PlanningBudget`, `PlannedStep`, `PlannedPlan`, and `AgentDecisionRuntime` is the right move because they create a clean seam between candidate ranking, future planner search, and runtime execution.
- The original proposal should be narrowed so 009 owns execution-facing plan containers only, while E13DECARC-010 owns planner semantics (`PlannerOpKind`, semantics tables, goal-to-op mapping). That separation is more robust and extensible than mixing both concerns into one type now.

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
```

`Default` should use the prototype values already documented by the ticket:

- `max_candidates_to_plan = 4`
- `max_plan_depth = 6`
- `max_node_expansions = 128`
- `beam_width = 8`
- `switch_margin_permille = Permille(100)`
- `transient_block_ticks = 20`
- `structural_block_ticks = 200`

### 2. Define `PlannedStep`, `PlanTerminalKind`, and `PlannedPlan` in `worldwake-ai/src/planner_ops.rs`

```rust
pub struct PlannedStep {
    pub def_id: ActionDefId,
    pub targets: Vec<EntityId>,
    pub payload_override: Option<ActionPayload>,
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

Implementation requirement:

- provide a constructor/helper that derives `total_estimated_ticks` from the supplied steps so callers do not calculate it manually

### 3. Implement `PlannedStep -> InputKind::RequestAction` conversion

```rust
impl PlannedStep {
    pub fn to_request_action(&self, actor: EntityId) -> InputKind { ... }
}
```

This conversion must be lossless for `def_id`, ordered `targets`, and `payload_override`.

### 4. Define `AgentDecisionRuntime` in `worldwake-ai/src/decision_runtime.rs`

```rust
pub struct AgentDecisionRuntime {
    pub current_goal: Option<GoalKey>,
    pub current_plan: Option<PlannedPlan>,
    pub dirty: bool,
    pub last_priority_class: Option<GoalPriorityClass>,
}
```

This type must remain runtime-only and must not be registered as a component.

## Files to Touch

- `crates/worldwake-ai/src/budget.rs` (new)
- `crates/worldwake-ai/src/planner_ops.rs` (new)
- `crates/worldwake-ai/src/decision_runtime.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify for module wiring and re-exports)

## Out of Scope

- `PlannerOpKind` enum and semantics table - E13DECARC-010
- `PlanningSnapshot` / `PlanningState` - E13DECARC-011
- plan search algorithm and selection - E13DECARC-012
- failure handling / blocked-intent recording - E13DECARC-013
- plan revalidation - E13DECARC-014
- registering any of these AI runtime types as components

## Acceptance Criteria

### Tests That Must Pass

1. `PlanningBudget::default()` produces the documented prototype values.
2. `PlannedStep::to_request_action(actor)` preserves `def_id`, ordered `targets`, and `payload_override`.
3. `PlannedPlan::new(...)` derives `total_estimated_ticks` as the sum of step `estimated_ticks`.
4. `AgentDecisionRuntime::default()` has `current_goal = None`, `current_plan = None`, `dirty = false`, and `last_priority_class = None`.
5. `AgentDecisionRuntime` is not registered in `component_schema.rs`.
6. `PlanningBudget`, `PlannedStep`, `PlanTerminalKind`, and `PlannedPlan` round-trip through bincode.
7. Existing suite: `cargo test -p worldwake-ai`, `cargo test --workspace`, `cargo clippy --workspace`.

### Invariants

1. `PlannedStep` stores exact ordered `targets` and exact `payload_override`.
2. `PlannedStep` remains executable without consulting planner semantics tables.
3. `PlannedPlan.total_estimated_ticks` is derived from the step list, not independently authored.
4. `AgentDecisionRuntime` never becomes authoritative world state.
5. No `HashMap` / `HashSet` usage.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/budget.rs` - default budget values and serialization
2. `crates/worldwake-ai/src/planner_ops.rs` - request-action conversion, total tick derivation, serialization
3. `crates/worldwake-ai/src/decision_runtime.rs` - default runtime state
4. `crates/worldwake-ai/src/lib.rs` - public re-export smoke coverage if needed

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - corrected the ticket assumptions to match the live codebase before implementation
  - added `PlanningBudget` in `crates/worldwake-ai/src/budget.rs`
  - added `PlannedStep`, `PlanTerminalKind`, and `PlannedPlan` in `crates/worldwake-ai/src/planner_ops.rs`
  - added `AgentDecisionRuntime` in `crates/worldwake-ai/src/decision_runtime.rs`
  - wired the new modules through `crates/worldwake-ai/src/lib.rs`
- Deviations from original plan:
  - `PlannerOpKind` was intentionally kept out of this ticket even though the original draft mixed it into `PlannedStep`; that concern remains correctly owned by E13DECARC-010
  - `PlannedPlan` now has a constructor that derives `total_estimated_ticks` from the step list so callers do not maintain that field manually
  - the files were added as new modules rather than modifying pre-existing stubs, because those stubs were not present in the current crate layout
- Verification results:
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
