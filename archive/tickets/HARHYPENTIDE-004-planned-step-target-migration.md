# HARHYPENTIDE-004: PlannedStep target migration to PlanningEntityRef

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planner step model and search (`worldwake-ai`)
**Deps**: HARHYPENTIDE-002 (planning identity model)
**Spec Reference**: HARDENING-hypothetical-entity-identity.md, Section C.1–C.2

## Problem

`PlanningState` already models authoritative and hypothetical entities distinctly, but `PlannedStep.targets` is still `Vec<EntityId>`. That leaves the persisted plan model and the execution boundary inconsistent with the planning state identity model. Until `PlannedStep` migrates, search cannot honestly record hypothetical targets, and the runtime boundary still conflates planning identity with execution identity.

## Assumption Reassessment (2026-03-12, corrected)

1. `PlanningEntityRef`, `HypotheticalEntityId`, hypothetical registry, and `PlanningEntityRef`-keyed override maps already exist in `crates/worldwake-ai/src/planning_state.rs` — confirmed. This ticket must not try to re-introduce that work.
2. Planner-visible carry/load support already exists in `PlanningState` (`carry_capacity_ref`, `load_of_entity_ref`, `remaining_carry_capacity_ref`) — confirmed. This ticket must not claim HARHYPENTIDE-003 work.
3. `CommitOutcome`, `Materialization`, and `MaterializationTag` already exist in `worldwake-sim` — confirmed. This ticket must stay out of action-handler contract work.
4. `PlannedStep` in `crates/worldwake-ai/src/planner_ops.rs` still has `pub targets: Vec<EntityId>` — confirmed.
5. `PlannedStep::to_request_action` still converts targets directly to `InputKind::RequestAction` — confirmed. This method is now the wrong architectural boundary because it hides target resolution.
6. `search_plan` in `crates/worldwake-ai/src/search.rs` still stores raw affordance targets directly into `PlannedStep` — confirmed.
7. `revalidate_next_step` in `crates/worldwake-ai/src/plan_revalidation.rs` still compares `step.targets` as raw `EntityId`s — confirmed.
8. `agent_tick.rs` still enqueues `step.to_request_action(agent)` directly — confirmed.
9. `failure_handling.rs` and several planner/runtime tests also assume `step.targets.first().copied()` returns an `EntityId` — confirmed. These are part of the migration surface and cannot be ignored.
10. `AgentDecisionRuntime` still has no materialization binding table — confirmed, but that belongs to HARHYPENTIDE-005, not this ticket.

## Architecture Check

1. `PlannedStep.targets` must become `Vec<PlanningEntityRef>` so the persisted plan model matches the planning-state identity model. No dual storage and no alias field.
2. `PlannedStep::to_request_action` should be removed. Resolution must happen at the planning-to-execution boundary, not inside the step model.
3. `InputKind::RequestAction` remains `EntityId`-based. Planning types stop at the AI/runtime boundary.
4. This ticket must not add ad hoc compatibility shims such as `legacy_targets`, target aliases, or fake `EntityId` encodings for hypothetical refs.
5. Runtime materialization bindings stay in HARHYPENTIDE-005. For this ticket, unresolved hypothetical targets must fail cleanly instead of being guessed or silently coerced.

## What to Change

### 1. Change `PlannedStep.targets` type

```rust
pub struct PlannedStep {
    pub def_id: ActionDefId,
    pub targets: Vec<PlanningEntityRef>,  // was Vec<EntityId>
    pub payload_override: Option<ActionPayload>,
    pub op_kind: PlannerOpKind,
    pub estimated_ticks: u32,
    pub is_materialization_barrier: bool,
}
```

### 2. Remove `PlannedStep::to_request_action`

This method directly converts targets to `EntityId`. Replace it with target resolution logic in `agent_tick.rs` (see step 4).

### 3. Update search to produce `PlanningEntityRef` targets

In `crates/worldwake-ai/src/search.rs`, when constructing `PlannedStep` from affordances, wrap affordance targets in `PlanningEntityRef::Authoritative(...)`. Search still reasons over authoritative affordance matches today; this migration makes the persisted step model honest without inventing hypothetical targets prematurely.

### 4. Add explicit target resolution at the execution boundary

Between plan step retrieval and input queue submission, add:

```rust
fn resolve_step_targets(
    step: &PlannedStep,
) -> Option<Vec<EntityId>> {
    step.targets.iter().map(|target| match target {
        PlanningEntityRef::Authoritative(id) => Some(*id),
        PlanningEntityRef::Hypothetical(_) => None,
    }).collect()
}
```

Use this in `agent_tick.rs` before enqueue. If any hypothetical ref is unresolved, do not enqueue; treat the step as invalid and force replanning. This preserves the correct boundary now and leaves HARHYPENTIDE-005 free to extend the resolver with bindings instead of rewriting call sites again.

### 5. Update revalidation and planner-side helpers

`revalidate_next_step` and any planner/runtime helper that inspects `step.targets` must understand `PlanningEntityRef`.

- Revalidation should resolve authoritative refs for affordance matching and fail cleanly on hypothetical refs.
- Failure handling should only derive blocker metadata from authoritative refs and return `None` for hypothetical refs instead of assuming every target is executable.

### 6. Update `PlannedPlan` serialization

`PlanningEntityRef` already derives `Serialize, Deserialize`; keep `PlannedPlan` bincode roundtrip coverage after the migration.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — `PlannedStep.targets` type, remove `to_request_action`, add shared resolution helpers)
- `crates/worldwake-ai/src/search.rs` (modify — wrap affordance targets in `PlanningEntityRef::Authoritative`)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — add target resolution before enqueue)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — resolve refs before affordance comparison)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — stop assuming all step targets are authoritative)
- `crates/worldwake-ai/src/goal_model.rs` / `plan_selection.rs` / tests that construct `PlannedStep` (modify — use typed targets)

## Out of Scope

- Introducing `MaterializationBindings` population from `CommitOutcome` (HARHYPENTIDE-005)
- Exact pickup transition producing hypothetical targets (that is HARHYPENTIDE-006)
- Changing `apply_hypothetical_transition` to accept hypothetical targets before HARHYPENTIDE-006 needs them
- Changes to `worldwake-sim` action execution layer
- Changes to `worldwake-systems`
- Changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. `PlannedStep` stores `Vec<PlanningEntityRef>`.
2. Authoritative step targets resolve to the correct `EntityId`s before enqueue.
3. Hypothetical step targets fail cleanly at the execution boundary and in revalidation until HARHYPENTIDE-005 introduces bindings.
4. `PlannedPlan` bincode roundtrip works with `PlanningEntityRef` targets.
5. Search produces steps with `PlanningEntityRef::Authoritative` wrapping for current affordance-driven plans.
6. Failure handling and related planner/runtime helpers no longer assume `step.targets` is a raw `EntityId` vector.
7. Existing planner and runtime tests pass after migrating test fixtures to typed targets.
8. `cargo test --workspace`
9. `cargo clippy --workspace`

### Invariants

1. `InputKind::RequestAction` remains `EntityId`-only — no planning types leak into execution.
2. No `PlannedStep::to_request_action` method remains (no compatibility path).
3. `PlanningEntityRef` is the only valid target type in persisted plans.
4. Unresolved hypothetical refs always fail cleanly (replan/invalid step, never panic).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — `PlannedStep` serialization and resolution tests with authoritative and hypothetical refs.
2. `crates/worldwake-ai/src/plan_revalidation.rs` — revalidation with authoritative refs, unresolved hypothetical refs fail cleanly.
3. `crates/worldwake-ai/src/agent_tick.rs` — enqueue uses resolved authoritative IDs only.
4. `crates/worldwake-ai/src/failure_handling.rs` — blocker derivation remains safe when step targets are hypothetical.

### Commands

1. `cargo test -p worldwake-ai planner_ops`
2. `cargo test -p worldwake-ai plan_revalidation`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - Migrated `PlannedStep.targets` from `Vec<EntityId>` to `Vec<PlanningEntityRef>`.
  - Removed the direct `PlannedStep::to_request_action` execution shortcut.
  - Added shared planner-target resolution helpers so authoritative passthrough and later hypothetical binding can use one code path.
  - Updated search to persist authoritative affordance bindings as `PlanningEntityRef::Authoritative(...)`.
  - Updated `agent_tick.rs` to resolve planned targets explicitly before enqueue and fail cleanly when a step contains unresolved hypothetical refs.
  - Updated `plan_revalidation.rs` and `failure_handling.rs` to stop assuming every planned target is an `EntityId`.
  - Migrated affected planner/runtime tests and added explicit coverage for hypothetical-resolution failure and bincode roundtrip continuity.
- Deviations from original plan:
  - Did not add `MaterializationBindings` or touch `AgentDecisionRuntime`; that remains correctly owned by HARHYPENTIDE-005.
  - Did not change `apply_hypothetical_transition` to accept `PlanningEntityRef`; current planner transitions still operate on authoritative affordance bindings, and exact hypothetical-target transitions remain HARHYPENTIDE-006 work.
  - Included `failure_handling.rs`, `plan_selection.rs`, and related test fixtures in the migration surface because the raw-target assumption extended beyond the files originally listed.
- Verification results:
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
