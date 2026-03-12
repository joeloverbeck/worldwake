# HARHYPENTIDE-005: Materialization binding runtime

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — decision runtime binding logic (`worldwake-ai`)
**Deps**: HARHYPENTIDE-001 (CommitOutcome type), HARHYPENTIDE-004 (PlannedStep uses PlanningEntityRef)
**Spec Reference**: HARDENING-hypothetical-entity-identity.md, Section D.4–D.5, Section C.3

## Problem

After a real action commits and creates new authoritative entities (reported via `CommitOutcome`), later plan steps that target the corresponding hypothetical entities need a way to resolve those hypothetical refs to the newly created authoritative IDs. Without a binding table, the runtime cannot bridge the planner's hypothetical world and the authoritative world.

## Assumption Reassessment (2026-03-12)

1. `AgentDecisionRuntime` in `decision_runtime.rs` has no binding table — confirmed.
2. `CommitOutcome` will be introduced by HARHYPENTIDE-001 with `Vec<Materialization>` — pending that ticket.
3. `PlannedStep.targets` will be `Vec<PlanningEntityRef>` after HARHYPENTIDE-004 — pending that ticket.
4. `agent_tick.rs` calls `tick_action` and then advances the step index — confirmed. The commit outcome is currently discarded.
5. `revalidate_next_step` in `plan_revalidation.rs` compares step targets against affordances — confirmed.

## Architecture Check

1. `MaterializationBindings` is planner/runtime state, not a world component. It lives in `AgentDecisionRuntime`.
2. Binding is deterministic: the planner knows which hypothetical ID it expects, and the `CommitOutcome` reports which authoritative entity was created with which `MaterializationTag`. Matching is by tag + ordinal position.
3. Unresolved bindings trigger replanning — never silent degradation.

## What to Change

### 1. Introduce `MaterializationBindings`

```rust
pub struct MaterializationBindings {
    pub hypothetical_to_authoritative: BTreeMap<HypotheticalEntityId, EntityId>,
}

impl MaterializationBindings {
    pub fn new() -> Self { Self { hypothetical_to_authoritative: BTreeMap::new() } }

    pub fn bind(&mut self, hyp: HypotheticalEntityId, auth: EntityId) {
        self.hypothetical_to_authoritative.insert(hyp, auth);
    }

    pub fn resolve(&self, hyp: HypotheticalEntityId) -> Option<EntityId> {
        self.hypothetical_to_authoritative.get(&hyp).copied()
    }

    pub fn clear(&mut self) {
        self.hypothetical_to_authoritative.clear();
    }
}
```

### 2. Add `MaterializationBindings` to `AgentDecisionRuntime`

```rust
pub struct AgentDecisionRuntime {
    // ... existing fields ...
    pub materialization_bindings: MaterializationBindings,
}
```

Clear bindings when a new plan is adopted (goal switch or replan).

### 3. Add planner expectation metadata to `PlannedStep`

Steps that create hypothetical entities need to carry enough metadata to bind outputs deterministically. Add:

```rust
pub struct PlannedStep {
    // ... existing fields ...
    pub expected_materializations: Vec<ExpectedMaterialization>,
}

pub struct ExpectedMaterialization {
    pub tag: MaterializationTag,
    pub hypothetical_id: HypotheticalEntityId,
}
```

### 4. Wire `CommitOutcome` → binding table in `agent_tick.rs`

After a step commits:
1. Receive `CommitOutcome` from `tick_action`
2. Match `CommitOutcome.materializations` against `step.expected_materializations` by tag and order
3. For each match: `bindings.bind(expected.hypothetical_id, materialization.entity)`
4. If count mismatch → log warning and trigger replan

### 5. Wire binding resolution into `revalidate_next_step`

Pass `MaterializationBindings` to revalidation. Resolve hypothetical refs before comparing against affordance targets. Unresolved refs → revalidation fails → replan.

### 6. Clear bindings on replan/goal-switch

When `AgentDecisionRuntime` adopts a new plan, clear `materialization_bindings` since old hypothetical IDs are no longer valid.

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add `MaterializationBindings` field)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — wire `CommitOutcome` → bindings, pass bindings to resolution)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — accept bindings for ref resolution)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — add `expected_materializations` to `PlannedStep`)
- `crates/worldwake-ai/src/lib.rs` (modify — export new types)

## Out of Scope

- Producing `CommitOutcome` with actual materializations in handlers (HARHYPENTIDE-006 for `pick_up`)
- Planning state identity model (HARHYPENTIDE-002)
- Carry-capacity beliefs (HARHYPENTIDE-003)
- Changes to `worldwake-sim` beyond consuming `CommitOutcome`
- Changes to `worldwake-systems`
- Changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. `MaterializationBindings::bind` stores and `resolve` retrieves correctly.
2. `MaterializationBindings::clear` empties all bindings.
3. Binding wiring: `CommitOutcome` with `SplitOffLot` materialization → correct hypothetical ID bound to authoritative entity.
4. Count mismatch between expected and actual materializations → replan triggered.
5. Unresolved hypothetical ref in next step → revalidation fails → replan.
6. Bindings cleared on goal switch or replan.
7. Existing suite: `cargo test --workspace`
8. Existing lint: `cargo clippy --workspace`

### Invariants

1. `MaterializationBindings` is never stored as authoritative world state.
2. Binding is deterministic: same `CommitOutcome` + same expectations → same bindings.
3. Unresolved hypothetical refs always trigger replan, never silent fallback.
4. Bindings are cleared when adopting a new plan (no stale mappings across plans).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` — `MaterializationBindings` CRUD and lifecycle tests.
2. `crates/worldwake-ai/src/agent_tick.rs` — binding wiring from `CommitOutcome`, mismatch handling.
3. `crates/worldwake-ai/src/plan_revalidation.rs` — revalidation with binding resolution.

### Commands

1. `cargo test -p worldwake-ai decision_runtime`
2. `cargo test -p worldwake-ai plan_revalidation`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
