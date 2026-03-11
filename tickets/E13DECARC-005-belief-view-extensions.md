# E13DECARC-005: BeliefView trait extensions and OmniscientBeliefView implementations

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — trait extension in worldwake-sim, new imports in worldwake-core
**Deps**: E13DECARC-001

## Problem

The AI decision architecture needs to query homeostatic needs, drive thresholds, wounds, hostiles, attackers, sellers, recipes, workstations, resource sources, demand memory, merchandise profiles, corpses, transit state, adjacent places with travel ticks, and duration estimates through `BeliefView`. The current trait has 23 methods; E13 adds 15 more. `OmniscientBeliefView` must implement all of them as temporary scaffolding.

## Assumption Reassessment (2026-03-11)

1. `BeliefView` has exactly 23 methods in `crates/worldwake-sim/src/belief_view.rs` — confirmed.
2. `OmniscientBeliefView` implements all 23 in `omniscient_belief_view.rs` — confirmed.
3. Types needed: `HomeostaticNeeds`, `DriveThresholds`, `Wound`, `DemandObservation`, `MerchandiseProfile`, `InTransitOnEdge`, `WorkstationTag`, `RecipeId`, `CommodityKind`, `ActionPayload`, `ActionDuration`, `DurationExpr` — all exist in `worldwake-core` or `worldwake-sim`.
4. `NonZeroU32` from std — available.
5. `World` has access to all component tables needed for omniscient implementations — confirmed.

## Architecture Check

1. All new methods use types already defined in `worldwake-core` or `worldwake-sim`.
2. `visible_hostiles_for()` and `current_attackers_of()` must be LOCAL in semantics — only return threats at the agent's effective place or sharing the same transit edge. This is critical for Principle 7.
3. `estimate_duration()` replaces what would otherwise be `DurationExpr::resolve_for(&World)` — it goes through beliefs instead.
4. `OmniscientBeliefView` implementations must use local filtering, not global queries.

## What to Change

### 1. Extend `BeliefView` trait

Add 15 new methods to the trait in `belief_view.rs`:

```rust
fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds>;
fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds>;
fn wounds(&self, agent: EntityId) -> Vec<Wound>;
fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId>;
fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId>;
fn agents_selling_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
fn known_recipes(&self, agent: EntityId) -> Vec<RecipeId>;
fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId>;
fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation>;
fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile>;
fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId>;
fn in_transit_state(&self, entity: EntityId) -> Option<InTransitOnEdge>;
fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)>;
fn estimate_duration(
    &self,
    actor: EntityId,
    duration: &DurationExpr,
    targets: &[EntityId],
    payload: &ActionPayload,
) -> Option<ActionDuration>;
```

### 2. Implement in `OmniscientBeliefView`

Key locality constraints for the omniscient adapter:

- `visible_hostiles_for(agent)`: filter to agents at agent's effective place (or same transit edge) that have hostile social relation or are in active combat with anyone at that place. Must NOT return all hostiles globally.
- `current_attackers_of(agent)`: filter to agents currently executing an attack action targeting this agent. Must be local.
- `agents_selling_at(place, commodity)`: filter to agents at `place` with a `MerchandiseProfile` containing `commodity`.
- `corpse_entities_at(place)`: filter to dead agents at `place`.
- `adjacent_places_with_travel_ticks(place)`: use `topology().edges_from(place)` to get travel times.
- `estimate_duration()`: resolve `DurationExpr` using belief data instead of `&World`.

### 3. Update imports in `belief_view.rs`

Add imports for the new types used in method signatures.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — add 15 methods)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` (modify — implement 15 methods)

## Out of Scope

- Per-agent belief stores (E14)
- `PlanningState`'s implementation of `BeliefView` — E13DECARC-011
- Social relation system for hostility — use existing hostile relation or combat state
- Changing existing 23 methods

## Acceptance Criteria

### Tests That Must Pass

1. `OmniscientBeliefView` still implements `BeliefView` (existing trait bound test still passes)
2. `homeostatic_needs()` returns `Some` for an agent with `HomeostaticNeeds` component, `None` without
3. `drive_thresholds()` returns `Some` for an agent with `DriveThresholds` component
4. `wounds()` returns the wound list for an agent with `WoundList`
5. `visible_hostiles_for()` returns only entities at the agent's place with hostile relations or active attacks — NOT global hostiles
6. `current_attackers_of()` returns only entities currently executing an attack action targeting the agent
7. `agents_selling_at()` returns only agents at the place with matching `MerchandiseProfile`
8. `known_recipes()` returns the agent's known recipe IDs
9. `corpse_entities_at()` returns only dead agents at the place
10. `adjacent_places_with_travel_ticks()` returns `(place, ticks)` pairs from topology
11. `estimate_duration()` returns `Some` for fixed and travel durations, `None` for indefinite
12. All existing `BeliefView` / `OmniscientBeliefView` tests remain green
13. Existing suite: `cargo test --workspace`

### Invariants

1. `visible_hostiles_for()` is local — never returns hostiles at remote places
2. `current_attackers_of()` is local — only direct combat opponents
3. All new methods go through `&World` reads only (no mutation)
4. No new `HashMap`/`HashSet` usage
5. `BeliefView` remains object-safe (`&dyn BeliefView`)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/omniscient_belief_view.rs` — new tests for each of the 15 methods with locality assertions

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
