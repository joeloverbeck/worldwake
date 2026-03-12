# Hardening Spec: Hypothetical Entity Identity & Materialization Binding

**Status**: DRAFT

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
5. Generalize the same mechanism for future materializing transitions:
   - partial loot
   - partial trade
   - craft outputs
   - harvest outputs
   - future container split/merge flows
6. Preserve Principle 12: systems interact through state, not direct cross-system calls.
7. Preserve Principle 13: no compatibility shims, no alias API retained alongside the new identity model.

## Non-Goals

1. This spec does not add new action families.
2. This spec does not add global planner omniscience.
3. This spec does not introduce abstract cargo scores or probabilistic pickup approximations.
4. This spec does not preserve the old raw-`EntityId` planned-target model as a parallel compatibility path.

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

### 3. HypotheticalEntity Data

`PlanningState` must own a deterministic table of hypothetical entities:

```rust
pub struct HypotheticalEntity {
    pub kind: EntityKind,
    pub effective_place: Option<PlanningEntityRef>,
    pub direct_container: Option<PlanningEntityRef>,
    pub direct_possessor: Option<PlanningEntityRef>,
    pub direct_possessions: BTreeSet<PlanningEntityRef>,
    pub item_lot_commodity: Option<CommodityKind>,
    pub item_lot_consumable_profile: Option<CommodityConsumableProfile>,
    pub commodity_quantities: BTreeMap<CommodityKind, Quantity>,
    pub owner: Option<PlanningEntityRef>,
    pub lifecycle: SnapshotLifecycle,
}
```

The exact stored fields may be refined, but the resulting hypothetical entity must contain all concrete information required for exact planner reasoning. No abstract "future inventory score" substitutes are allowed.

### 4. PlanningState Entity Arena

`PlanningState` must evolve from "snapshot plus overrides" into:

- authoritative snapshot-backed entities
- override tables for authoritative entities
- hypothetical entity table
- deterministic `next_hypothetical_id`

Required operations:

- `spawn_hypothetical_lot(...) -> HypotheticalEntityId`
- `resolve_entity_ref(...)`
- `move_entity_ref(...)`
- `set_possessor_ref(...)`
- `set_container_ref(...)`
- `set_quantity_ref(...)`
- `mark_removed_ref(...)`

All existing planning queries must work against both authoritative and hypothetical entities through one unified belief surface.

## Belief Surface Extensions Needed For Exact Pickup

Exact partial pickup requires exact carry math. The planner cannot fake this.

### Required Belief Data

Extend the planner-visible concrete state so it can compute exact carry fit:

- `carry_capacity(agent) -> Option<LoadUnits>`
- `container_capacity(entity) -> Option<LoadUnits>` where relevant
- enough item/load data to compute exact current carried load recursively

One acceptable shape:

```rust
fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits>;
fn container_capacity(&self, entity: EntityId) -> Option<LoadUnits>;
fn load_of_entity(&self, entity: PlanningEntityRef) -> Option<LoadUnits>;
```

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

### 3. Payloads

If payloads contain entity references, they must also be migrated to use `PlanningEntityRef` or a structurally equivalent target-reference type. This applies to:

- loot payloads
- combat payloads if they ever target hypothetical entities in future
- any future transport/trade payload referencing split outputs

## Planner Transition Semantics

### 1. Transition Ownership

Hypothetical transitions remain owned by planner semantics, not by search.

This spec builds on the recent ownership cleanup and extends it to materializing transitions.

### 2. Materializing Transition Kinds

Introduce planner transition kinds that may create hypothetical entities, for example:

```rust
pub enum PlannerTransitionKind {
    GoalModelFallback,
    PickUpLot,
    PutDownLot,
    HarvestOutput,
    CraftOutput,
    TradeTransfer,
    LootTransfer,
}
```

Only the transitions actually implemented need to be active initially, but the system design must support this class of operation cleanly.

### 3. Exact Pickup Transition

The planner-owned `pick_up` transition must mirror authoritative pickup semantics exactly:

1. validate co-location and target shape
2. compute exact remaining carry capacity from concrete planner-visible state
3. if the full lot fits:
   - move the authoritative lot into actor possession
4. if only a partial quantity fits:
   - reduce the original authoritative lot quantity
   - create a hypothetical lot with the moved quantity
   - place that hypothetical lot in actor possession
5. if nothing fits:
   - transition is invalid

This is the precise rework required for the current pickup issue.

### 4. Exact Representation Example

Input state:

- `Authoritative(L10)` = `Water x 10`
- actor remaining capacity fits `Water x 4`

After hypothetical partial pickup:

- `Authoritative(L10)` = `Water x 6`
- `Hypothetical(H1)` = `Water x 4`, possessed by actor

If the next step is `put_down(H1)`, the planner must target `Hypothetical(H1)` directly.

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

This belongs in plan execution runtime state, not in authoritative world ECS.

### 3. Action Materialization Contract

Actions whose commit path creates new authoritative entities must publish explicit materialization results.

One acceptable shape:

```rust
pub struct ActionMaterialization {
    pub role: MaterializationRole,
    pub entity: EntityId,
}

pub enum MaterializationRole {
    SplitOffCarryLot,
    CraftOutputLot { output_index: u16 },
    HarvestOutputLot { output_index: u16 },
    TradeReceivedLot,
    LootTransferredLot,
}
```

The exact API may differ, but the contract must be explicit and typed. It must not rely on parsing ad hoc event tags or guessing from all newly touched entities.

### 4. Planner Expectation Metadata

Planner steps that create hypothetical entities must carry enough semantics to bind outputs deterministically.

Example:

- pickup partial split step expects one materialized output with role `SplitOffCarryLot`
- runtime binds `H1 -> L27`

### 5. Revalidation Path

Plan revalidation must:

1. resolve each `PlanningEntityRef`
2. authoritative refs use the current world directly
3. hypothetical refs must resolve through the binding table
4. unresolved hypothetical refs fail revalidation

This gives exact failure behavior rather than silent degradation.

## Runtime Execution Flow

### Step 1: Search

- search produces `PlannedPlan`
- plan may contain `PlanningEntityRef::Hypothetical(...)` in later steps

### Step 2: Execute First Step

- step targets resolve
- if step commit creates new authoritative entities, runtime receives explicit materialization results

### Step 3: Bind

- runtime updates `MaterializationBindings`
- hypothetical refs created by the step are mapped to authoritative entities

### Step 4: Revalidate Next Step

- next step target refs are resolved through bindings
- affordance matching uses resolved authoritative IDs

### Step 5: Continue Or Replan

- unresolved binding, target loss, or semantic mismatch triggers replanning

## Pick Up Rework Requirements

This spec explicitly requires reworking the current pickup issue, not merely supporting it in theory.

### Required Code-Level Rework Areas

- `worldwake-ai/src/planning_state.rs`
- `worldwake-ai/src/planning_snapshot.rs`
- `worldwake-ai/src/planner_ops.rs`
- `worldwake-ai/src/search.rs`
- `worldwake-ai/src/plan_revalidation.rs`
- `worldwake-ai/src/agent_tick.rs`
- `worldwake-sim` action execution APIs where materialization results are surfaced
- `worldwake-systems/src/transport_actions.rs`

### Required Behavioral End State

The planner must be able to represent exactly:

1. partial pickup of a lot
2. travel while holding the split-off lot
3. later actions targeting that exact split-off lot

### Forbidden End State

The implementation fails this spec if it:

- still treats partial pickup as full pickup in planner state
- stores only aggregate quantity deltas without entity identity
- invents a compatibility alias path from hypothetical entities to old raw `EntityId` targeting
- infers bindings by heuristic best effort instead of explicit typed materialization contracts

## Component Registration

No new authoritative ECS components are required by default.

Explicit requirements:

- `MaterializationBindings` is planner/runtime state, not a world component
- hypothetical entity arenas are planner state, not world components
- if any shared identity type is moved into `worldwake-core`, it must be a plain value type, not a registered component

If implementation discovers a genuine need for authoritative component storage, that must be justified in a revision to this spec before coding.

## SystemFn Integration

### worldwake-ai

Reads:

- `PlanningSnapshot`
- concrete cargo/load/capacity beliefs
- planner transition semantics
- binding table during revalidation/execution

Writes:

- hypothetical entity arena in `PlanningState`
- `PlannedPlan` with `PlanningEntityRef` targets
- runtime materialization bindings

### worldwake-sim

Reads:

- planned step target refs after binding resolution
- action execution context

Writes:

- explicit action materialization outputs for committing actions that create entities

### worldwake-systems

Reads:

- authoritative world state through normal action execution

Writes:

- normal authoritative entity creation / mutation
- typed materialization outputs for runtime binding

## Cross-System Interactions (Principle 12)

- `worldwake-systems` does not call planner logic directly.
- `worldwake-ai` does not call transport internals directly.
- interaction occurs through:
  - shared action definitions
  - authoritative world mutations
  - typed materialization results emitted by action execution

No system-to-system shortcut is allowed.

## FND-01 Section H

### Information-Path Analysis

- Exact pickup planning depends on information already local to the actor:
  - visible target lot
  - local containment/possession tree
  - actor carry capacity
  - commodity physical load profile
- No global query should answer "can this fit?" on behalf of the planner beyond the planner's own local belief-derived state.
- Materialization bindings are runtime-local plan execution facts, not world knowledge. They do not violate locality because they are not exposed as global world truth to agents.

### Positive-Feedback Analysis

- More exact hypothetical planning can improve action chaining success, which can increase planner confidence in multi-step cargo plans.
- Better cargo-plan success can expose more cases where entity-creating transitions are useful, causing more planner reliance on the system.

### Concrete Dampeners

- Planner depth budget
- beam width budget
- revalidation on every step
- explicit binding failure forcing replanning
- finite carry capacity and concrete lot quantities

These are concrete process limits, not arbitrary score clamps.

### Stored State vs Derived Read-Model

**Stored (authoritative):**

- world entities and their real `EntityId`s
- item lot quantities
- carry capacity components
- action commit outputs / materialization records

**Stored (planner/runtime but non-authoritative):**

- hypothetical entity arena
- hypothetical entity IDs
- plan target refs
- materialization binding table

**Derived (transient):**

- remaining carry capacity
- whether a lot fully fits
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

## Implementation Sections

### Section A: Planning Identity Model

Implement:

1. `HypotheticalEntityId`
2. `PlanningEntityRef`
3. `HypotheticalEntity`
4. `PlanningState` arena support

### Section B: Belief & Snapshot Exact Cargo Support

Implement:

1. carry-capacity planner visibility
2. exact planner load computation
3. any snapshot fields required for exact recursive load accounting

### Section C: Planned Step & Revalidation Migration

Implement:

1. `PlannedStep.targets` migration to typed refs
2. payload reference migration where needed
3. revalidation resolution through binding table

### Section D: Action Materialization Contract

Implement:

1. typed materialization outputs from action execution
2. runtime binding application
3. deterministic failure behavior on mismatch

### Section E: Exact Pickup Rework

Implement:

1. exact partial pickup hypothetical transition
2. exact post-commit binding for split-off carried lot
3. regression coverage for multi-step exact cargo planning

## Acceptance Criteria

- [ ] `PlannedStep` no longer stores raw `EntityId` targets
- [ ] planner can create hypothetical lots with stable identity
- [ ] planner can represent exact partial pickup without approximation
- [ ] planner can target the split-off carried lot in a later step
- [ ] runtime binds hypothetical split-off lot identity to the authoritative entity created by commit
- [ ] unresolved hypothetical refs fail revalidation cleanly
- [ ] no compatibility alias path remains
- [ ] `pick_up` planner semantics are reworked to use the exact identity system
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` passes

## Tests

### Unit Tests

- [ ] `PlanningState` can spawn hypothetical item lots deterministically
- [ ] `PlanningEntityRef` resolution distinguishes authoritative vs hypothetical refs correctly
- [ ] exact carry-fit math matches authoritative transport semantics for full fit
- [ ] exact carry-fit math matches authoritative transport semantics for partial fit
- [ ] exact carry-fit math rejects zero-fit pickup
- [ ] binding table resolves hypothetical refs after commit
- [ ] unresolved hypothetical refs fail revalidation

### Search / Planner Tests

- [ ] planner can produce a plan with partial `pick_up` followed by `travel`
- [ ] planner can produce a plan with partial `pick_up` followed by `put_down` of the split-off lot
- [ ] planner can produce a plan that consumes or trades a split-off lot when that is the correct next action
- [ ] generic non-materializing actions still work with no behavior regression

### Transport / Action Tests

- [ ] authoritative partial `pick_up` emits typed materialization output for the split-off lot
- [ ] full-fit `pick_up` emits no split-off hypothetical binding expectation
- [ ] `put_down` after exact partial pickup resolves against the bound authoritative entity

### End-to-End Tests

- [ ] golden-style scenario where an actor partially picks up cargo, travels, and delivers it exactly
- [ ] deterministic replay of that scenario yields identical results for identical seeds

## Suggested Ticket Breakdown

1. `HYPID-001` - Planning entity reference migration
2. `HYPID-002` - Hypothetical entity arena in `PlanningState`
3. `HYPID-003` - Belief/snapshot carry-capacity and exact load support
4. `HYPID-004` - Action materialization output contract
5. `HYPID-005` - Revalidation and execution binding runtime
6. `HYPID-006` - Exact partial pickup planner rework
7. `HYPID-007` - Exact follow-up cargo planning tests and golden scenario

## Cross-References

- [HARDENING-PRE-E14.md](/home/joeloverbeck/projects/worldwake/specs/HARDENING-PRE-E14.md)
- [E10-production-transport.md](/home/joeloverbeck/projects/worldwake/archive/specs/E10-production-transport.md)
- [HARPREE14-015-authoritative-hypothetical-action-transitions.md](/home/joeloverbeck/projects/worldwake/archive/tickets/HARPREE14-015-authoritative-hypothetical-action-transitions.md)
- [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md)
