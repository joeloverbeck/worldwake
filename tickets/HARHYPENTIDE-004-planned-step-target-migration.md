# HARHYPENTIDE-004: PlannedStep target migration to PlanningEntityRef

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planner step model and search (`worldwake-ai`)
**Deps**: HARHYPENTIDE-002 (PlanningEntityRef type must exist)
**Spec Reference**: HARDENING-hypothetical-entity-identity.md, Section C.1–C.2

## Problem

`PlannedStep.targets` is `Vec<EntityId>`, which cannot represent hypothetical entities created by earlier planner steps. This makes multi-step plans involving entity-creating transitions impossible to express.

## Assumption Reassessment (2026-03-12)

1. `PlannedStep` in `crates/worldwake-ai/src/planner_ops.rs:279` has `pub targets: Vec<EntityId>` — confirmed.
2. `PlannedStep::to_request_action` at line 290 converts targets directly to `InputKind::RequestAction` with `targets: self.targets.clone()` — confirmed.
3. `apply_hypothetical_transition` at line 242 takes `targets: &[EntityId]` — confirmed.
4. `search_plan` in `crates/worldwake-ai/src/search.rs` constructs `PlannedStep` with `EntityId` targets from affordances — confirmed.
5. `revalidate_next_step` at `plan_revalidation.rs:6` compares `step.targets` against affordance targets — confirmed.
6. `agent_tick.rs` retrieves steps and calls `to_request_action` to produce `InputKind` — confirmed.

## Architecture Check

1. Changing `PlannedStep.targets` to `Vec<PlanningEntityRef>` is the single point of truth for planner targeting. No dual storage.
2. `to_request_action` must be replaced with a resolution step in `agent_tick.rs` that converts `PlanningEntityRef` → `EntityId` via the binding table before constructing `InputKind`.
3. `InputKind::RequestAction` remains `EntityId`-based — the execution layer is not contaminated with planning types.

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

In `crates/worldwake-ai/src/search.rs`, when constructing `PlannedStep` from affordances, wrap affordance targets in `PlanningEntityRef::Authoritative(...)`. The search already knows targets from affordance binding — this is a mechanical wrapping.

### 4. Add target resolution in `agent_tick.rs`

Between plan step retrieval and input queue submission, add:

```rust
fn resolve_step_targets(
    step: &PlannedStep,
    bindings: &MaterializationBindings,
) -> Option<Vec<EntityId>> {
    step.targets.iter().map(|target| match target {
        PlanningEntityRef::Authoritative(id) => Some(*id),
        PlanningEntityRef::Hypothetical(hid) => bindings.resolve(*hid),
    }).collect()
}
```

If any hypothetical ref is unresolved → trigger replan (don't enqueue). Construct `InputKind::RequestAction` with the resolved `Vec<EntityId>`.

### 5. Update `apply_hypothetical_transition` signature

Change from `targets: &[EntityId]` to `targets: &[PlanningEntityRef]`. The function body will need to resolve refs as needed for its internal logic.

### 6. Update `PlannedPlan` serialization

Ensure `PlanningEntityRef` derives `Serialize, Deserialize` so `PlannedPlan` bincode roundtrip continues to work.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — `PlannedStep.targets` type, remove `to_request_action`, update `apply_hypothetical_transition`)
- `crates/worldwake-ai/src/search.rs` (modify — wrap affordance targets in `PlanningEntityRef::Authoritative`)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — add target resolution before enqueue)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — resolve refs before affordance comparison)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add `MaterializationBindings` field)

## Out of Scope

- Introducing `MaterializationBindings` population from `CommitOutcome` (that is HARHYPENTIDE-005)
- Exact pickup transition producing hypothetical targets (that is HARHYPENTIDE-006)
- Changes to `worldwake-sim` action execution layer
- Changes to `worldwake-systems`
- Changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. `PlannedStep` with `PlanningEntityRef::Authoritative` targets resolves to correct `EntityId`s.
2. `PlannedStep` with `PlanningEntityRef::Hypothetical` targets resolves when binding exists.
3. Unresolved hypothetical ref → resolution returns `None` → replan triggered.
4. `PlannedPlan` bincode roundtrip works with `PlanningEntityRef` targets.
5. Search produces steps with `PlanningEntityRef::Authoritative` wrapping for standard affordance targets.
6. `apply_hypothetical_transition` works with `PlanningEntityRef` targets.
7. All existing planner tests pass (targets wrapped in `Authoritative`).
8. Existing suite: `cargo test --workspace`
9. Existing lint: `cargo clippy --workspace`

### Invariants

1. `InputKind::RequestAction` remains `EntityId`-only — no planning types leak into execution.
2. No `PlannedStep::to_request_action` method remains (no compatibility path).
3. `PlanningEntityRef` is the only valid target type in persisted plans.
4. Unresolved hypothetical refs always fail cleanly (replan, never panic).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — `PlannedStep` with `PlanningEntityRef` targets, bincode roundtrip.
2. `crates/worldwake-ai/src/plan_revalidation.rs` — revalidation with authoritative refs (unchanged behavior), revalidation with hypothetical refs (binding-resolved).
3. `crates/worldwake-ai/src/agent_tick.rs` — target resolution tests.

### Commands

1. `cargo test -p worldwake-ai planner_ops`
2. `cargo test -p worldwake-ai plan_revalidation`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
