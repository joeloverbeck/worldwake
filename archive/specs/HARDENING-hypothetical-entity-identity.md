# Hardening Spec: Hypothetical Entity Identity & Materialization Binding

**Status**: ✅ COMPLETED

## Summary

The planner currently reasons over hypothetical state by overlaying mutations onto existing authoritative entities. That is sufficient for travel, need relief, and other transitions that only mutate existing entities. It is not sufficient for action families whose authoritative commit path creates new entities.

The clearest current failure is `pick_up` with partial carry fit:

- authoritative transport semantics leave the original lot on the ground with reduced quantity
- authoritative transport semantics create a new carried lot with a new `EntityId`
- the planner cannot currently represent that new future lot, so it cannot model exact partial pickup faithfully across later steps

This spec defines a full system for hypothetical entity identity, exact materialization-aware planning, and execution-time rebinding from hypothetical planner entities to authoritative post-commit entities. It also explicitly requires reworking the current `pick_up` planner semantics to use that system so partial pickup can be represented perfectly rather than approximately.

## Phase

Pre-E14 hardening of the E01-E13 foundation

## Crates

- `worldwake-ai`
- `worldwake-sim`
- `worldwake-systems`
- `worldwake-core` only if a shared non-component identity type is required there

## Problem

### Current Limitation

`PlanningState` is an overlay on top of `PlanningSnapshot`. It can:

- move existing entities
- change possession/container relationships for existing entities
- adjust commodity quantities on existing entities
- remove existing entities from the hypothetical state

It cannot:

- create a new hypothetical entity with stable identity
- let later hypothetical steps target that new entity
- rebind that hypothetical entity to the authoritative entity produced when the real action commits

### Why This Breaks Exact Partial Pickup

Consider:

- ground lot `L10`: `Water x 10`
- actor remaining carry capacity: `LoadUnits(8)`
- `Water` load per unit: `LoadUnits(2)`

Authoritative `pick_up` behavior:

1. `L10` remains on ground as `Water x 6`
2. new lot `L27` is created in actor possession as `Water x 4`

The current planner has no way to express `L27` before it exists. If search wants to plan:

1. partial `pick_up`
2. `travel`
3. `put_down` the carried partial lot

it needs a stable target identity for the carried lot created at step 1. Without that identity, the planner either lies about what happened or becomes unable to express later steps exactly.

### Architectural Root Cause

The planner currently conflates:

- authoritative entity identity
- hypothetical entity identity
- executable step targeting

These must be separated cleanly.

## Goals

1. Allow the planner to create hypothetical entities with deterministic, stable identities during search.
2. Allow later hypothetical steps to target those hypothetical entities directly.
3. Rebind hypothetical entities to authoritative entities when the real action commit creates them.
4. Make partial `pick_up` exact, including split-off lot identity and quantity.
5. Design the mechanism so it naturally extends to future materializing transitions (partial loot, craft outputs, harvest outputs) when those features are implemented.
6. Preserve Principle 12: systems interact through state, not direct cross-system calls.
7. Preserve Principle 13: no compatibility shims, no alias API retained alongside the new identity model.

## Non-Goals

1. This spec does not add new action families.
2. This spec does not add global planner omniscience.
3. This spec does not introduce abstract cargo scores or probabilistic pickup approximations.
4. This spec does not preserve the old raw-`EntityId` planned-target model as a parallel compatibility path.
5. This spec does not implement materializing transitions for harvest, craft, trade, or loot — those will be added when their respective systems need them.

## Design Overview

The implementation has four pillars:

1. hypothetical entity identity in planning
2. target references that can point to authoritative or hypothetical entities
3. exact planner transition semantics that can create hypothetical entities
4. execution-time materialization binding from hypothetical IDs to authoritative IDs

## Core Data Model

### 1. HypotheticalEntityId

Introduce a planner-local stable ID:

```rust
pub struct HypotheticalEntityId(pub u32);
```

Requirements:

- deterministic allocation order
- monotonically increasing within a search root
- clone-safe within `PlanningState`
- serializable if stored in plans/runtime

This ID is not an ECS `EntityId` and must never be used as one.

### 2. PlanningEntityRef

Replace planner targeting of bare `EntityId` with an explicit sum type:

```rust
pub enum PlanningEntityRef {
    Authoritative(EntityId),
    Hypothetical(HypotheticalEntityId),
}
```

All planner-internal targeting and all persisted `PlannedStep` targets must use this type.

This is the central architectural change. It makes identity honest.

### 3. HypotheticalEntityMeta (Minimal Registry)

`PlanningState` must own a minimal registry of hypothetical entity base data — only information that cannot be expressed as an override in the existing override maps:

```rust
pub struct HypotheticalEntityMeta {
    pub kind: EntityKind,
    pub item_lot_commodity: Option<CommodityKind>,
}
```

All other hypothetical entity data — placement, possession, quantity, container relationships — lives in the same override maps that already exist for authoritative entity overrides, re-keyed from `EntityId` to `PlanningEntityRef`.

**Rationale**: A unified key architecture means one code path per query. Check the override map; fall back to the snapshot ONLY for `PlanningEntityRef::Authoritative` refs. Hypothetical entities have no snapshot to fall back to — all their data is in overrides.

### 4. PlanningState Unified Maps

`PlanningState` must evolve from "snapshot plus `EntityId`-keyed overrides" into unified `PlanningEntityRef`-keyed maps:

- All existing override maps (`position_overrides`, `quantity_overrides`, `possession_overrides`, `removed_entities`, etc.) are re-keyed from `EntityId` to `PlanningEntityRef`
- `hypothetical_registry: BTreeMap<HypotheticalEntityId, HypotheticalEntityMeta>` — NOT a full entity table, just the minimal base data per hypothetical entity
- `next_hypothetical_id: u32` — deterministic counter, part of `PlanningState`, cloned correctly during search branching

Required operations:

- `spawn_hypothetical_lot(kind: EntityKind, commodity: CommodityKind) -> HypotheticalEntityId` — allocates from `next_hypothetical_id`, registers in `hypothetical_registry`, returns the new ID
- Query methods (`effective_place`, `commodity_quantity`, `direct_possessions`, etc.) take `PlanningEntityRef` instead of `EntityId` — check override map first, fall back to snapshot only for `Authoritative` refs
- `move_entity_ref(...)`, `set_possessor_ref(...)`, `set_quantity_ref(...)`, `mark_removed_ref(...)` — operate on `PlanningEntityRef` keys

## Belief Surface Extensions Needed For Exact Pickup

Exact partial pickup requires exact carry math. The planner cannot fake this.

### Required Belief Data

Extend the planner-visible concrete state so it can compute exact carry fit:

- `carry_capacity(agent) -> Option<LoadUnits>` — delegates to `Container.capacity` on the agent's body container (the `Container` component in `worldwake-core/src/items.rs`)
- Enough item/load data to compute exact current carried load recursively

One acceptable shape:

```rust
fn carry_capacity(&self, entity: PlanningEntityRef) -> Option<LoadUnits>;
fn load_of_entity(&self, entity: PlanningEntityRef) -> Option<LoadUnits>;
```

These should reference the existing load accounting system in `worldwake-core/src/load.rs`: `remaining_container_capacity()`, `load_per_unit()`, `load_of_lot()`.

Alternative shapes are acceptable if they remain concrete and deterministic.

### Why This Is Required

Without this data, the planner cannot know whether `pick_up`:

- fully moves the lot
- partially splits the lot
- fails because nothing fits

This spec forbids rough planner approximations for that branch.

## Planned Step Model

### 1. PlannedStep Targets

`PlannedStep.targets` must become:

```rust
pub targets: Vec<PlanningEntityRef>
```

### 2. No Compatibility Path

Do not introduce:

- `legacy_targets: Vec<EntityId>`
- dual raw/typed target storage
- implicit "hypothetical IDs stuffed into `EntityId`"

The old path must be replaced, not preserved.

### 3. Payload Resolution at Execution Boundary

`ActionPayload` variants (`Harvest`, `Craft`, `Trade`, `Combat`, `Loot`) remain `EntityId`-based. They are part of the execution layer and must not be contaminated with planning types.

Instead: resolve all `PlanningEntityRef` targets to authoritative `EntityId`s BEFORE constructing payloads. This resolution happens at the planning-to-execution boundary in `agent_tick.rs`, not inside the payload types.

## Planner Transition Semantics

### 1. Transition Ownership

Hypothetical transitions remain owned by planner semantics, not by search.

This spec builds on the recent ownership cleanup and extends it to materializing transitions.

### 2. PlannerTransitionKind

Extend the existing `PlannerTransitionKind` enum (currently in `planner_ops.rs`):

```rust
pub enum PlannerTransitionKind {
    GoalModelFallback,
    PickUpGroundLot,       // existing — preserved name; "Ground" qualifier is meaningful
    PutDownGroundLot,      // new — needed for putting down hypothetical lots
}
```

Only transitions for currently implemented features are included. The enum is designed to grow — when harvest, craft, trade, or loot need materializing transitions, new variants will be added alongside their implementations.

### 3. Exact Pickup Transition

The planner-owned `PickUpGroundLot` transition (currently `apply_pick_up_transition` in `planner_ops.rs`) must be reworked to mirror authoritative pickup semantics exactly:

1. validate co-location and target shape
2. compute exact remaining carry capacity from concrete planner-visible state
3. if the full lot fits:
   - move the authoritative lot into actor possession (current behavior)
4. if only a partial quantity fits:
   - reduce the original authoritative lot quantity in overrides
   - call `spawn_hypothetical_lot` to create a hypothetical lot with the moved quantity and same commodity kind
   - place that hypothetical lot in actor possession via override maps
5. if nothing fits:
   - transition is invalid

This is the precise rework required for the current pickup issue.

### 4. PutDownGroundLot Transition

The new `PutDownGroundLot` transition handles putting down lots (including hypothetical ones):

- If target is `PlanningEntityRef::Hypothetical(...)`: move the hypothetical entity from actor possession to ground at the actor's current (authoritative) place
- If target is `PlanningEntityRef::Authoritative(...)`: same as current `GoalModelFallback` behavior for put-down

### 5. Exact Representation Example

Input state:

- `Authoritative(L10)` = `Water x 10`
- actor remaining capacity fits `Water x 4`

After hypothetical partial pickup:

- `Authoritative(L10)` = `Water x 6` (quantity override)
- `Hypothetical(H1)` = `Water x 4`, possessed by actor (registry entry + override entries)

If the next step is `put_down(H1)`, the planner must target `Hypothetical(H1)` directly.

### 6. Constraint: Hypothetical Entities at Authoritative Places Only

Hypothetical lots are always placed at authoritative places (the actor's current place). No hypothetical places exist in any current or foreseeable use case. This simplifies placement logic — the place ref in override maps for hypothetical entities is always `PlanningEntityRef::Authoritative(place_id)`.

## Execution-Time Materialization Binding

### 1. Why Binding Is Required

Search may produce plans containing hypothetical target refs created by earlier steps. After the real world executes those earlier steps, later steps must resolve those hypothetical refs to real authoritative entities.

### 2. Binding Table

Introduce a deterministic runtime binding table:

```rust
pub struct MaterializationBindings {
    pub hypothetical_to_authoritative: BTreeMap<HypotheticalEntityId, EntityId>,
}
```

This belongs in plan execution runtime state (in `AgentDecisionRuntime`), not in authoritative world ECS.

### 3. CommitOutcome Return Type

Actions whose commit path creates new authoritative entities must report those creations. The current `ActionCommitFn` signature:

```rust
pub type ActionCommitFn = for<'w> fn(
    &ActionDef, &ActionInstance, &mut DeterministicRng, &mut WorldTxn<'w>,
) -> Result<(), ActionError>;
```

must change to:

```rust
pub type ActionCommitFn = for<'w> fn(
    &ActionDef, &ActionInstance, &mut DeterministicRng, &mut WorldTxn<'w>,
) -> Result<CommitOutcome, ActionError>;
```

where:

```rust
pub struct CommitOutcome {
    pub materializations: Vec<Materialization>,
}

pub struct Materialization {
    pub tag: MaterializationTag,
    pub entity: EntityId,
}

pub enum MaterializationTag {
    SplitOffLot,
}

impl CommitOutcome {
    pub fn empty() -> Self {
        Self { materializations: Vec::new() }
    }
}
```

**`MaterializationTag` is designed to grow** — when harvest, craft, trade, or loot need materialization tracking, new variants will be added alongside their implementations.

All existing action handlers that currently return `Ok(())` must be updated to return `Ok(CommitOutcome::empty())`. This is a cross-cutting change to `worldwake-sim/src/action_handler.rs` and all handler implementations in `worldwake-systems`.

### 4. Planner Expectation Metadata

Planner steps that create hypothetical entities must carry enough semantics to bind outputs deterministically.

Example:

- pickup partial split step expects one materialized output with tag `SplitOffLot`
- runtime binds `H1 -> L27`

### 5. Target Resolution Before Enqueue

Binding resolution happens in `agent_tick.rs` BETWEEN plan step retrieval and input queue submission:

1. Retrieve the next `PlannedStep` (which has `Vec<PlanningEntityRef>` targets)
2. Resolve all `PlanningEntityRef` targets through the binding table:
   - `Authoritative(id)` -> `id` (passthrough)
   - `Hypothetical(hid)` -> look up in `MaterializationBindings` -> `EntityId`
3. Any unresolved hypothetical ref -> replan (don't enqueue)
4. Construct `InputKind::RequestAction` with resolved `Vec<EntityId>` targets

This keeps `InputKind::RequestAction` and the entire scheduler/execution layer using `EntityId` only.

### 6. Revalidation with Hypothetical Refs

`revalidate_next_step()` (in `plan_revalidation.rs`) compares planned targets against affordance targets. Affordances always have `Vec<EntityId>` (real world entities).

For steps with hypothetical targets:

1. Resolve through binding table first
2. Then compare against affordance `Vec<EntityId>`
3. Unresolved hypothetical refs -> revalidation fails -> replan

## BeliefView Trait Interaction

- `BeliefView` remains `EntityId`-based — it serves authoritative world queries
- `PlanningState` continues to implement `BeliefView` for authoritative entity queries (wrapping `Authoritative(id)` internally)
- Hypothetical entity queries go through `PlanningState`'s own `PlanningEntityRef`-based methods
- Search and transition logic use `PlanningEntityRef` methods directly, not `BeliefView`

## Runtime Execution Flow

### Step 1: Search

- search produces `PlannedPlan`
- plan may contain `PlanningEntityRef::Hypothetical(...)` in later steps

### Step 2: Execute First Step

- step targets are resolved through binding table (Authoritative refs pass through, Hypothetical refs must be bound)
- any unresolved hypothetical ref -> replan instead of enqueuing
- resolved `EntityId` targets are used to construct `InputKind::RequestAction` for the scheduler
- if step commit creates new authoritative entities, runtime receives `CommitOutcome` with explicit materializations

### Step 3: Bind

- runtime updates `MaterializationBindings`
- hypothetical refs created by the step are mapped to authoritative entities

### Step 4: Revalidate Next Step

- next step target refs are resolved through bindings
- affordance matching uses resolved authoritative IDs

### Step 5: Continue Or Replan

- unresolved binding, target loss, or semantic mismatch triggers replanning

## Component Registration

No new authoritative ECS components are required by default.

Explicit requirements:

- `MaterializationBindings` is planner/runtime state, not a world component
- hypothetical entity registry and override maps are planner state, not world components
- `CommitOutcome`, `Materialization`, `MaterializationTag` belong in `worldwake-sim` alongside the action handler types
- `ActionCommitFn` signature change is a cross-cutting modification to `worldwake-sim/src/action_handler.rs` — all existing handlers in `worldwake-systems` must update their return type

If implementation discovers a genuine need for authoritative component storage, that must be justified in a revision to this spec before coding.

## SystemFn Integration

### worldwake-ai

Reads:

- `PlanningSnapshot`
- concrete cargo/load/capacity beliefs (referencing `load.rs` accounting)
- planner transition semantics
- binding table during revalidation/execution

Writes:

- hypothetical entity registry and overrides in `PlanningState`
- `PlannedPlan` with `PlanningEntityRef` targets
- runtime materialization bindings

### worldwake-sim

Reads:

- planned step target refs after binding resolution
- action execution context

Writes:

- `CommitOutcome` from action handlers that create entities

### worldwake-systems

Reads:

- authoritative world state through normal action execution

Writes:

- normal authoritative entity creation / mutation
- `CommitOutcome` with `Materialization` entries for handlers that create entities (currently: `pick_up` split path in `transport_actions.rs`)

## Cross-System Interactions (Principle 12)

- `worldwake-systems` does not call planner logic directly.
- `worldwake-ai` does not call transport internals directly.
- interaction occurs through:
  - shared action definitions
  - authoritative world mutations
  - `CommitOutcome` results emitted by action execution

No system-to-system shortcut is allowed.

## FND-01 Section H

### Information-Path Analysis

- Exact pickup planning depends on information already local to the actor:
  - visible target lot
  - local containment/possession tree
  - actor carry capacity (via `Container` component)
  - commodity physical load profile (via `load_per_unit()` in `load.rs`)
- No global query should answer "can this fit?" on behalf of the planner beyond the planner's own local belief-derived state.
- Materialization bindings are runtime-local plan execution facts, not world knowledge. They do not violate locality because they are not exposed as global world truth to agents.

### Positive-Feedback Analysis

- More exact hypothetical planning can improve action chaining success, which can increase planner confidence in multi-step cargo plans.
- Better cargo-plan success can expose more cases where entity-creating transitions are useful, causing more planner reliance on the system.

### Concrete Dampeners

The physical world dampener is **finite carry capacity and concrete lot quantities** — the physical world limits how much cargo can be moved, which naturally bounds the utility of multi-step cargo plans. An agent cannot plan infinite cargo chains because the world's concrete load constraints cap what is achievable.

Additionally, the planner has process constraints that limit planning scope:

- planner depth budget (limits plan length)
- beam width budget (limits search breadth)
- revalidation on every step (forces replanning when world diverges)
- explicit binding failure forcing replanning

These are execution process limits, not physical world dampeners per se, but they bound computational cost.

### Stored State vs Derived Read-Model

**Stored (authoritative):**

- world entities and their real `EntityId`s
- item lot quantities
- carry capacity components (`Container.capacity`)
- `CommitOutcome` returned from action handlers (consumed immediately by binding logic, not persisted)

**Stored (planner/runtime but non-authoritative):**

- hypothetical entity registry (`BTreeMap<HypotheticalEntityId, HypotheticalEntityMeta>`)
- `PlanningEntityRef`-keyed override maps
- plan target refs (`Vec<PlanningEntityRef>`)
- materialization binding table

**Derived (transient):**

- remaining carry capacity (computed from container capacity and current carried load)
- whether a lot fully fits (computed from remaining capacity and lot load)
- which hypothetical ref resolves to which authoritative ref at the current execution point

## Invariants

1. Every hypothetical entity has exactly one stable `HypotheticalEntityId` within a plan search.
2. No `HypotheticalEntityId` is ever treated as an authoritative `EntityId`.
3. Every persisted planner target is explicitly typed as authoritative or hypothetical.
4. Exact partial pickup leaves the source lot reduced and creates a distinct carried lot.
5. Later steps may target that distinct carried lot without ambiguity.
6. Hypothetical-to-authoritative binding is explicit, typed, and deterministic.
7. Unresolved hypothetical refs fail revalidation rather than degrading silently.
8. No backward-compatibility alias path remains after migration.
9. Hypothetical entities are always placed at authoritative places — no hypothetical places exist.

## Implementation Sections

### Section A: Planning Identity Model

Implement:

1. `HypotheticalEntityId`
2. `PlanningEntityRef`
3. `HypotheticalEntityMeta` (minimal registry struct)
4. `PlanningState` unified `PlanningEntityRef`-keyed override maps with `next_hypothetical_id` counter

### Section B: Belief & Snapshot Exact Cargo Support

Implement:

1. carry-capacity planner visibility (referencing `Container` component and `load.rs`)
2. exact planner load computation
3. any snapshot fields required for exact recursive load accounting

### Section C: Planned Step & Revalidation Migration

Implement:

1. `PlannedStep.targets` migration to `Vec<PlanningEntityRef>`
2. target resolution in `agent_tick.rs` before enqueue (resolve `PlanningEntityRef` -> `EntityId` via binding table)
3. revalidation resolution through binding table (resolve then compare against affordance `Vec<EntityId>`)

### Section D: CommitOutcome Contract

Implement:

1. `CommitOutcome`, `Materialization`, `MaterializationTag` types in `worldwake-sim`
2. `ActionCommitFn` signature change from `Result<(), ActionError>` to `Result<CommitOutcome, ActionError>`
3. update all existing handlers in `worldwake-systems` to return `CommitOutcome::empty()`
4. runtime binding application in `AgentDecisionRuntime`
5. deterministic failure behavior on mismatch

### Section E: Exact Pickup Rework

Implement:

1. `PutDownGroundLot` transition variant
2. exact partial pickup hypothetical transition (rework `apply_pick_up_transition`)
3. `pick_up` handler in `transport_actions.rs` returns `CommitOutcome` with `SplitOffLot` materialization on split path
4. exact post-commit binding for split-off carried lot
5. regression coverage for multi-step exact cargo planning

## Acceptance Criteria

- [ ] `PlannedStep` no longer stores raw `EntityId` targets
- [ ] planner can create hypothetical lots with stable identity
- [ ] planner can represent exact partial pickup without approximation
- [ ] planner can target the split-off carried lot in a later step
- [ ] runtime binds hypothetical split-off lot identity to the authoritative entity created by commit
- [ ] unresolved hypothetical refs fail revalidation cleanly
- [ ] no compatibility alias path remains
- [ ] `pick_up` planner semantics are reworked to use the exact identity system
- [ ] `ActionCommitFn` returns `CommitOutcome` across all handlers
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` passes

## Tests

### Unit Tests

- [ ] `PlanningState` can spawn hypothetical item lots deterministically via `spawn_hypothetical_lot`
- [ ] `PlanningEntityRef`-keyed override maps correctly distinguish authoritative vs hypothetical refs
- [ ] hypothetical entity data is queryable through the same methods as authoritative data (unified code path)
- [ ] exact carry-fit math matches authoritative transport semantics for full fit
- [ ] exact carry-fit math matches authoritative transport semantics for partial fit
- [ ] exact carry-fit math rejects zero-fit pickup
- [ ] binding table resolves hypothetical refs after commit
- [ ] `CommitOutcome` materialization flow: handler returns `SplitOffLot` -> runtime binds `H1 -> real EntityId`
- [ ] unresolved hypothetical refs fail revalidation

### Search / Planner Tests

- [ ] planner can produce a plan with partial `pick_up` followed by `travel`
- [ ] planner can produce a plan with partial `pick_up` followed by `put_down` of the split-off lot
- [ ] generic non-materializing actions still work with no behavior regression

### Transport / Action Tests

- [ ] authoritative partial `pick_up` returns `CommitOutcome` with `SplitOffLot` materialization for the split-off lot
- [ ] full-fit `pick_up` returns `CommitOutcome::empty()`
- [ ] `put_down` after exact partial pickup resolves against the bound authoritative entity

### End-to-End Tests

- [ ] golden-style scenario where an actor partially picks up cargo, travels, and delivers it exactly
- [ ] deterministic replay of that scenario yields identical results for identical seeds

## Suggested Ticket Breakdown

1. `HYPID-001` - `CommitOutcome` type and `ActionCommitFn` signature change (Section D.1-D.3)
2. `HYPID-002` - Planning identity model: `HypotheticalEntityId`, `PlanningEntityRef`, `HypotheticalEntityMeta`, unified maps (Section A)
3. `HYPID-003` - Belief/snapshot carry-capacity and exact load support (Section B)
4. `HYPID-004` - `PlannedStep` target migration and resolution in `agent_tick.rs` (Section C)
5. `HYPID-005` - Revalidation and execution binding runtime (Section C.3, D.4-D.5)
6. `HYPID-006` - Exact partial pickup planner rework + `PutDownGroundLot` transition (Section E)
7. `HYPID-007` - Exact follow-up cargo planning tests and golden scenario (Section E.5)

## Cross-References

- [HARDENING-PRE-E14.md](/home/joeloverbeck/projects/worldwake/archive/specs/HARDENING-PRE-E14.md)
- [E10-production-transport.md](/home/joeloverbeck/projects/worldwake/archive/specs/E10-production-transport.md)
- [HARPREE14-015-authoritative-hypothetical-action-transitions.md](/home/joeloverbeck/projects/worldwake/archive/tickets/HARPREE14-015-authoritative-hypothetical-action-transitions.md)
- [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md)

## Outcome

- Completed: 2026-03-12
- What changed: the planning/runtime stack now uses explicit hypothetical identity and binding primitives, including `HypotheticalEntityId`, `PlanningEntityRef`, `MaterializationBindings`, `CommitOutcome`, `Materialization`, `MaterializationTag`, `PutDownGroundLot`, and exact partial-pickup planner semantics with split-off lot binding. `PlannedStep.targets` now carries planning refs instead of raw `EntityId`s, and the split `pick_up` commit path reports `SplitOffLot` materializations for runtime rebinding.
- Deviations from original plan: no authoritative ECS components were added; the work landed as planner/runtime state plus the shared action-handler contract. The implementation was delivered across the HARHYPENTIDE ticket series rather than as one monolithic change.
- Verification results: repository tests now cover hypothetical entity spawning, typed planning targets, binding resolution, exact carry-fit/partial pickup behavior, `CommitOutcome` materialization flow, and exact cargo planning follow-up scenarios in `worldwake-ai`, `worldwake-sim`, and `worldwake-systems`.
