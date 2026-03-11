# E13DECARC-011: PlanningSnapshot, PlanningState, and BeliefView implementation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — AI-layer types
**Deps**: E13DECARC-005, E13DECARC-010

## Problem

Plan search needs a lightweight state representation that avoids cloning the whole world per search node. `PlanningSnapshot` is an immutable candidate-specific extract of relevant beliefs. `PlanningState` is a per-node delta overlay on top of the snapshot. `PlanningState` must implement `BeliefView` so `get_affordances()` can serve as the successor generator for hypothetical states.

## Assumption Reassessment (2026-03-11)

1. `BeliefView` trait has 38 methods after E13DECARC-005.
2. `get_affordances(view: &dyn BeliefView, actor, registry)` returns `Vec<Affordance>` — confirmed.
3. `Affordance` has `def_id`, `actor`, `bound_targets`, `payload_override` — confirmed.
4. Search nodes must NOT clone `Vec<Wound>` or `Vec<DemandObservation>` per node — spec requirement.
5. `HomeostaticNeeds`, `DriveThresholds`, `Wound`, `InTransitOnEdge`, `MerchandiseProfile`, `DemandObservation`, `ResourceSource` all exist in `worldwake-core`.

## Architecture Check

1. One immutable snapshot + compact per-node delta overlay — efficient, no heavy cloning.
2. Snapshot includes only entities/places relevant to the current candidate — no full world clone.
3. Per-node deltas: actor place override, possession overrides, resource quantity overrides, reservation shadows, drive summary updates, target availability overrides.
4. `PlanningState` implementing `BeliefView` means `get_affordances()` works unchanged as the successor generator.

## What to Change

### 1. Define `PlanningSnapshot` in `worldwake-ai/src/planning_snapshot.rs`

Contains only what's needed for the current candidate's plan search:

```rust
pub struct PlanningSnapshot {
    pub actor: EntityId,
    pub actor_place: Option<EntityId>,
    pub actor_needs: HomeostaticNeeds,
    pub actor_thresholds: DriveThresholds,
    pub actor_wounds_summary: Permille,  // pain summary, not cloned Vec<Wound>
    pub actor_alive: bool,
    pub actor_incapacitated: bool,
    pub reachable_places: BTreeMap<EntityId, PlaceSnapshot>,
    pub actor_possessions: BTreeSet<EntityId>,
    pub actor_recipes: Vec<RecipeId>,
    pub actor_demand_memory: Vec<DemandObservation>,  // cloned once in snapshot
    pub actor_merchandise: Option<MerchandiseProfile>,
    pub travel_edges: BTreeMap<(EntityId, EntityId), NonZeroU32>,
    // ... other fields as needed for referenced entities
}

pub struct PlaceSnapshot {
    pub entities: BTreeSet<EntityId>,
    pub sellers: BTreeMap<CommodityKind, Vec<EntityId>>,
    pub corpses: Vec<EntityId>,
    pub workstations: BTreeMap<WorkstationTag, Vec<EntityId>>,
    pub resource_sources: BTreeMap<CommodityKind, Vec<EntityId>>,
}
```

### 2. Define `PlanningState` in `worldwake-ai/src/planning_state.rs`

```rust
pub struct PlanningState<'s> {
    snapshot: &'s PlanningSnapshot,
    actor_place_override: Option<EntityId>,
    possession_overrides: BTreeMap<EntityId, Option<EntityId>>,
    resource_quantity_overrides: BTreeMap<EntityId, Quantity>,
    reservation_shadows: BTreeSet<EntityId>,
    drive_overrides: Option<HomeostaticNeeds>,
    pain_override: Option<Permille>,
    target_gone: BTreeSet<EntityId>,
}
```

### 3. Implement `BeliefView` for `PlanningState`

Each method must check the delta overlay first, then fall back to the snapshot. Key behaviors:

- `effective_place(actor)` -> check `actor_place_override`, else snapshot
- `direct_possessions(actor)` -> apply `possession_overrides` on top of snapshot
- `resource_source(entity)` -> check `resource_quantity_overrides`, else snapshot
- `homeostatic_needs(actor)` -> check `drive_overrides`, else snapshot
- `is_dead(entity)` -> check `target_gone`, else snapshot

### 4. Build snapshot from `BeliefView`

```rust
pub fn build_planning_snapshot(
    view: &dyn BeliefView,
    agent: EntityId,
    evidence_entities: &BTreeSet<EntityId>,
    evidence_places: &BTreeSet<EntityId>,
    travel_horizon: u8,
) -> PlanningSnapshot
```

### 5. PlanningState delta application

```rust
impl PlanningState {
    pub fn apply_op(&self, step: &PlannedStep, semantics: &PlannerOpSemantics) -> Self { ... }
}
```

Creates a new `PlanningState` with updated deltas based on the planner op kind.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — was empty stub)
- `crates/worldwake-ai/src/planning_state.rs` (modify — was empty stub)

## Out of Scope

- Plan search algorithm — E13DECARC-012
- Goal satisfaction predicates — implemented here for `is_satisfied` / `is_progress_barrier` on `PlanningState`
- Actual `get_affordances()` logic — it already exists in `worldwake-sim`, we just implement `BeliefView`

## Acceptance Criteria

### Tests That Must Pass

1. `PlanningState` implements `BeliefView` (trait bound test)
2. `PlanningState` with no overrides returns same values as snapshot
3. `actor_place_override` correctly overrides `effective_place()` for the actor
4. `possession_overrides` correctly affect `direct_possessions()`
5. `resource_quantity_overrides` correctly affect `resource_source()`
6. `drive_overrides` correctly affect `homeostatic_needs()`
7. `target_gone` correctly makes entities appear dead
8. `build_planning_snapshot()` includes only places within travel horizon
9. `build_planning_snapshot()` includes entities referenced by evidence
10. Search nodes do NOT clone `Vec<Wound>` per node (uses `Permille` pain summary instead)
11. `get_affordances(&planning_state, actor, registry)` returns valid affordances for hypothetical state
12. `apply_op()` for Travel updates actor place
13. `apply_op()` for Consume updates drive overrides
14. Existing suite: `cargo test --workspace`

### Invariants

1. `PlanningSnapshot` is immutable once built
2. No full world clone in snapshot or state
3. `Vec<Wound>` and `Vec<DemandObservation>` cloned once in snapshot, never per node
4. `PlanningState` is transient — never stored as authoritative state
5. All maps are `BTreeMap`, sets are `BTreeSet`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` — snapshot construction tests
2. `crates/worldwake-ai/src/planning_state.rs` — BeliefView implementation tests, delta application tests

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
