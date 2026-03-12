# HARHYPENTIDE-005: Materialization binding runtime

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — decision runtime binding logic (`worldwake-ai`) and committed-action outcome plumbing (`worldwake-sim`)
**Deps**: HARHYPENTIDE-001 (CommitOutcome type), HARHYPENTIDE-004 (PlannedStep uses `PlanningEntityRef`)
**Spec Reference**: HARDENING-hypothetical-entity-identity.md, Section D.4-D.5, Section C.3

## Problem

After a real action commits and creates new authoritative entities (reported via `CommitOutcome`), later plan steps that target the corresponding hypothetical entities need a way to resolve those hypothetical refs to the newly created authoritative IDs. Without a binding table and a runtime path for committed outcomes to reach the AI runtime, the planner's hypothetical world and the authoritative execution world remain disconnected.

## Assumption Reassessment (2026-03-12, corrected)

1. `CommitOutcome`, `Materialization`, and `MaterializationTag` already exist in `crates/worldwake-sim/src/action_handler.rs` — confirmed. This ticket must not re-introduce the action-handler contract change from HARHYPENTIDE-001.
2. `PlannedStep.targets` already stores `Vec<PlanningEntityRef>` in `crates/worldwake-ai/src/planner_ops.rs` — confirmed. This ticket must not repeat HARHYPENTIDE-004.
3. `AgentDecisionRuntime` in `crates/worldwake-ai/src/decision_runtime.rs` still has no materialization binding table — confirmed.
4. `revalidate_next_step` in `crates/worldwake-ai/src/plan_revalidation.rs` still resolves targets through `authoritative_targets(...)`, so every hypothetical target currently fails revalidation — confirmed.
5. `agent_tick.rs` does not call `tick_action` directly and does not receive `CommitOutcome` today — confirmed. The scheduler owns action progression.
6. `worldwake-sim/src/tick_step.rs` discards committed outcomes after incrementing the completed-action count — confirmed. There is currently no persisted per-agent committed-action record for autonomous controllers to consume.
7. `AutonomousController::produce_agent_input(...)` currently receives only agent-scoped replan signals, not committed-action outcomes — confirmed.
8. `transport_actions.rs` still returns `CommitOutcome::empty()` from `commit_pick_up`, even on the split path — confirmed, but that remains HARHYPENTIDE-006 work. This ticket must provide the runtime consumer path without pulling that handler change forward.

## Architecture Check

1. `MaterializationBindings` belongs in `AgentDecisionRuntime`, not in authoritative ECS state.
2. Binding resolution must reuse the existing `PlanningEntityRef` resolution boundary instead of inventing a second target-resolution path.
3. The AI runtime must not reach into scheduler internals or `tick_action` return values directly. The clean boundary is: scheduler records committed outcomes, autonomous-controller input production receives agent-scoped committed actions, and `AgentTickDriver` consumes them.
4. Planner expectation metadata must be explicit on the step that creates hypothetical entities. The runtime should bind materializations from step-local expectations, not by guessing from future steps or action names.
5. Unresolved hypothetical refs must fail revalidation and enqueue cleanly, forcing replanning rather than silent coercion.
6. This ticket provides runtime consumption and binding only. Producing non-empty `CommitOutcome.materializations` in real handlers remains owned by HARHYPENTIDE-006.

## What to Change

### 1. Introduce runtime binding types in `worldwake-ai`

```rust
pub struct MaterializationBindings {
    pub hypothetical_to_authoritative: BTreeMap<HypotheticalEntityId, EntityId>,
}

pub struct ExpectedMaterialization {
    pub tag: MaterializationTag,
    pub hypothetical_id: HypotheticalEntityId,
}
```

Add `materialization_bindings: MaterializationBindings` to `AgentDecisionRuntime`.

### 2. Extend `PlannedStep` with explicit materialization expectations

```rust
pub struct PlannedStep {
    // existing fields...
    pub expected_materializations: Vec<ExpectedMaterialization>,
}
```

This keeps the binding contract attached to the step that created the hypothetical entity. It is more robust and extensible than inferring bindings from action names or from later step targets.

### 3. Expose committed action outcomes to autonomous controllers

Add a scheduler/runtime-facing committed-action record in `worldwake-sim` that preserves:

- actor
- action def / instance identity
- commit tick
- `CommitOutcome`

Store committed actions in `Scheduler` until the next input-production phase and surface agent-scoped committed actions through the autonomous-controller input path.

This is the required architectural correction: `AgentTickDriver` cannot bind from `CommitOutcome` unless the scheduler publishes completed-action results.

### 4. Reconcile in-flight steps from committed outcomes, not disappearance alone

In `crates/worldwake-ai/src/agent_tick.rs`:

1. When a step is in flight and the active action is gone, first consult the current agent's committed-action records.
2. If a matching committed action exists, bind `CommitOutcome.materializations` to `step.expected_materializations`.
3. On deterministic mismatch (missing tag, count mismatch, duplicate binding target) treat the step as failed and trigger replanning.
4. Only advance the step index after successful binding application.
5. If the action vanished without either a committed-action record or a replan signal, do not silently assume success; treat that as invalid runtime state and replan.

### 5. Resolve hypothetical targets through bindings in both enqueue and revalidation

Replace direct authoritative-only resolution with shared binding-aware resolution:

- enqueue path in `agent_tick.rs`
- `revalidate_next_step(...)` in `plan_revalidation.rs`

Authoritative refs pass through unchanged. Hypothetical refs resolve via `MaterializationBindings`. Any unresolved hypothetical ref fails cleanly.

### 6. Clear bindings when adopting a new plan

Whenever `AgentDecisionRuntime` adopts a new plan or drops the current one for a replan/goal switch, clear `materialization_bindings`. Old hypothetical IDs must never leak across plan lifetimes.

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add binding types and runtime field)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — add `ExpectedMaterialization` to `PlannedStep`)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — consume committed outcomes, apply bindings, resolve targets with bindings, clear on plan replacement)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — binding-aware target resolution)
- `crates/worldwake-ai/src/lib.rs` (modify — export new runtime/planner types)
- `crates/worldwake-sim/src/scheduler.rs` (modify — retain committed action records until controller consumption)
- `crates/worldwake-sim/src/autonomous_controller.rs` (modify — pass agent-scoped committed actions into controller input production)
- `crates/worldwake-sim/src/tick_step.rs` (modify — record committed action outcomes during action progression)
- `crates/worldwake-sim/src/lib.rs` (modify — export new committed-action types if needed)

## Out of Scope

- Changing real action handlers to emit non-empty `CommitOutcome.materializations` (HARHYPENTIDE-006)
- Exact pickup transition semantics or `PutDownGroundLot` planner semantics (HARHYPENTIDE-006)
- Planning identity model changes already completed in HARHYPENTIDE-002 and HARHYPENTIDE-004
- Carry-capacity/load belief changes already completed in HARHYPENTIDE-003
- Changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. `MaterializationBindings::bind`, `resolve`, and `clear` behave deterministically.
2. `PlannedStep` carries explicit `expected_materializations`.
3. Scheduler/tick-step plumbing preserves committed `CommitOutcome` records until autonomous-controller input production can consume them.
4. `AgentTickDriver` applies committed materializations to bindings and only advances the step after successful binding.
5. Binding mismatch between `step.expected_materializations` and `CommitOutcome.materializations` triggers replanning.
6. Hypothetical planned targets resolve successfully when bindings exist.
7. Unresolved hypothetical planned targets fail revalidation and enqueue cleanly.
8. Bindings are cleared when a new plan is adopted or a current plan is dropped.
9. Existing workspace tests pass.
10. `cargo clippy --workspace` passes.

### Invariants

1. `MaterializationBindings` is runtime-only state, never authoritative world state.
2. Hypothetical-target resolution has exactly one execution boundary: `PlanningEntityRef` -> `EntityId` via bindings.
3. Step completion is no longer inferred solely from action disappearance when binding-sensitive work is involved.
4. Unresolved or mismatched materializations force replanning rather than silent degradation.
5. No compatibility alias path is introduced for raw/hypothetical target mixing.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` — binding CRUD and lifecycle tests.
2. `crates/worldwake-ai/src/plan_revalidation.rs` — binding-aware revalidation for authoritative, resolved hypothetical, and unresolved hypothetical targets.
3. `crates/worldwake-ai/src/agent_tick.rs` — committed-action reconciliation, binding application, mismatch handling, and binding clearing on plan replacement.
4. `crates/worldwake-sim/src/tick_step.rs` or `scheduler.rs` — committed outcomes are retained and exposed to controllers.

### Commands

1. `cargo test -p worldwake-ai decision_runtime`
2. `cargo test -p worldwake-ai plan_revalidation`
3. `cargo test -p worldwake-ai agent_tick`
4. `cargo test -p worldwake-sim tick_step`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - Added `MaterializationBindings` to `AgentDecisionRuntime` and exposed it from `worldwake-ai`.
  - Added explicit `ExpectedMaterialization` metadata to `PlannedStep`.
  - Added committed-action retention in `Scheduler` and passed agent-scoped committed outcomes through the autonomous-controller input path.
  - Updated `AgentTickDriver` to reconcile in-flight steps from committed outcomes, apply bindings deterministically, and stop treating vanished actions as implicit success.
  - Updated enqueue and revalidation paths to resolve `PlanningEntityRef` targets through runtime bindings.
  - Cleared bindings whenever plans are dropped or replaced so hypothetical IDs cannot leak across plan lifetimes.
  - Added focused tests for binding CRUD, scheduler committed-action retention, binding-aware revalidation, and agent runtime reconciliation helpers.
- Deviations from original plan:
  - Expanded scope into `worldwake-sim` because the original ticket incorrectly assumed `AgentTickDriver` directly saw `tick_action` outcomes. Without scheduler/controller plumbing, runtime binding was not implementable cleanly.
  - Did not change real action handlers to emit non-empty materializations. That remains correctly owned by HARHYPENTIDE-006.
  - Reused the existing `resolve_planning_targets_with(...)` boundary instead of introducing a second target-resolution abstraction.
- Verification results:
  - `cargo test -p worldwake-ai decision_runtime -- --nocapture`
  - `cargo test -p worldwake-ai plan_revalidation -- --nocapture`
  - `cargo test -p worldwake-ai agent_tick -- --nocapture`
  - `cargo test -p worldwake-sim autonomous_controller -- --nocapture`
  - `cargo test -p worldwake-sim tick_step -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace -- -D warnings`
