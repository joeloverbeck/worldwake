# E13DECARC-005: BeliefView trait extensions and omniscient belief runtime support

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — trait extension in worldwake-sim, omniscient adapter runtime plumbing, stub-belief updates
**Deps**: E13DECARC-001

## Problem

The AI decision architecture needs to query homeostatic needs, drive thresholds, wounds, local hostiles, current attackers, sellers, recipes, workstations, resource sources, demand observations, merchandise profiles, corpses, transit state, adjacent places with travel ticks, and duration estimates through `BeliefView`. The current trait has 23 methods; E13 still needs 15 more. `OmniscientBeliefView` remains the temporary scaffolding until E14, but some of the required answers are not world-only facts and need read-only runtime support.

## Assumption Reassessment (2026-03-11)

1. `BeliefView` still has exactly 23 methods in `crates/worldwake-sim/src/belief_view.rs` — confirmed.
2. `OmniscientBeliefView` still implements only those 23 in `crates/worldwake-sim/src/omniscient_belief_view.rs` — confirmed.
3. The required types exist, but two of the ticket's original assumptions were too loose:
   - wounds are stored as `WoundList { wounds: Vec<Wound> }`, not as a standalone per-wound component
   - trade memory is stored as `DemandMemory { observations: Vec<DemandObservation> }`, so belief methods should expose observations while reading from that component
4. `NonZeroU32` from `std` is available.
5. `World` does **not** have everything needed for the original `current_attackers_of()` definition. Active action instances live in `Scheduler`, and combat-domain classification lives in `ActionDefRegistry`.
6. `DurationExpr::resolve_for()` already exists on the authoritative side and is the right semantic baseline for the new belief-side `estimate_duration()` API.

## Architecture Check

1. All new methods should continue using types already defined in `worldwake-core` or `worldwake-sim`; do not introduce duplicate AI-only mirrors.
2. `visible_hostiles_for()` and `current_attackers_of()` must be local in semantics: only entities at the agent's effective place, or agents sharing the same `InTransitOnEdge`, may count. This is required by Principle 7.
3. `visible_hostiles_for()` should stay actor-specific and threat-specific. It should not return every combat participant at a place. The useful set for E13 is:
   - entities the agent is hostile toward, filtered locally
   - entities hostile toward the agent, filtered locally
   - runtime-visible current attackers of the agent
4. `current_attackers_of()` is not a pure world query. The clean implementation is a composite omniscient adapter over:
   - `&World` for authoritative components and relations
   - read-only scheduler active actions
   - read-only `ActionDefRegistry` so only true combat attack actions count
5. `estimate_duration()` replaces direct planner access to `DurationExpr::resolve_for(&World, ...)`; the planner should only go through beliefs, while the omniscient scaffolding may delegate to the same underlying authoritative semantics.
6. Extending the trait will also require updating the existing `StubBeliefView` implementations used by `affordance_query.rs` and `trade_valuation.rs`.

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

Notes:

- `wounds()` reads from `WoundList` and returns a cloned `Vec<Wound>`.
- `demand_memory()` reads from `DemandMemory` and returns a cloned `Vec<DemandObservation>`.
- `estimate_duration()` should take `&DurationExpr`, matching the E13 spec and avoiding unnecessary copies.

### 2. Add read-only runtime support to `OmniscientBeliefView`

Introduce a small runtime context for the omniscient adapter so combat-awareness queries can stay truthful without moving scheduler state into `World`.

Expected shape:

```rust
pub struct OmniscientBeliefRuntime<'a> {
    pub active_actions: &'a BTreeMap<ActionInstanceId, ActionInstance>,
    pub action_defs: &'a ActionDefRegistry,
}
```

`OmniscientBeliefView` should accept an optional runtime context. World-backed calls continue to work without it, but E13 AI integration must pass runtime when it needs exact attacker visibility.

### 3. Implement the new methods in `OmniscientBeliefView`

Key locality constraints for the omniscient adapter:

- `visible_hostiles_for(agent)`: filter to local entities at the agent's place (or same transit edge) that are hostile to the agent or targeted by the agent's hostility relation, plus any local entities returned by `current_attackers_of(agent)`. Must NOT return global hostiles or unrelated local combatants.
- `current_attackers_of(agent)`: inspect runtime active actions, keeping only combat-domain `"attack"` actions whose bound targets include this agent and whose actor is local to the agent.
- `agents_selling_at(place, commodity)`: filter to agents at `place` with a `MerchandiseProfile` containing `commodity`.
- `known_recipes(agent)`: return deterministic recipe ids from the `KnownRecipes` component.
- `matching_workstations_at(place, tag)`: filter local workstation entities by `WorkstationMarker`.
- `resource_sources_at(place, commodity)`: filter local entities with a matching `ResourceSource`.
- `demand_memory(agent)`: return the `DemandObservation` list from `DemandMemory`.
- `merchandise_profile(agent)`: clone the `MerchandiseProfile` component.
- `corpse_entities_at(place)`: filter to dead agents at `place`.
- `in_transit_state(entity)`: clone the `InTransitOnEdge` component.
- `adjacent_places_with_travel_ticks(place)`: use `topology().edges_from(place)` to get travel times.
- `estimate_duration()`: delegate to the authoritative duration semantics through the omniscient adapter and return `None` on resolution failure.

### 4. Update imports in `belief_view.rs`

Add imports for the new types used in method signatures.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — add 15 methods)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` (modify — add runtime support and implement 15 methods)
- `crates/worldwake-sim/src/lib.rs` (modify — export any new omniscient runtime helper types)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — update `StubBeliefView` for the expanded trait)
- `crates/worldwake-sim/src/trade_valuation.rs` (modify — update `StubBeliefView` for the expanded trait)

## Out of Scope

- Per-agent belief stores (E14)
- `PlanningState`'s implementation of `BeliefView` — E13DECARC-011
- Changing authoritative `World` storage to embed scheduler or active action state
- Changing existing 23 methods

## Acceptance Criteria

### Tests That Must Pass

1. `OmniscientBeliefView` still implements `BeliefView` (existing trait bound test still passes)
2. `homeostatic_needs()` returns `Some` for an agent with `HomeostaticNeeds` component, `None` without
3. `drive_thresholds()` returns `Some` for an agent with `DriveThresholds` component
4. `wounds()` returns the wound list for an agent with `WoundList`
5. `visible_hostiles_for()` returns only local actor-relevant hostiles — NOT global hostiles and NOT unrelated local combatants
6. `current_attackers_of()` returns only entities currently executing a local attack action targeting the agent when runtime context is present
7. `agents_selling_at()` returns only agents at the place with matching `MerchandiseProfile`
8. `known_recipes()` returns the agent's known recipe IDs
9. `demand_memory()` returns the current observation list from `DemandMemory`
10. `merchandise_profile()` clones the current `MerchandiseProfile`
11. `matching_workstations_at()` and `resource_sources_at()` stay local to the queried place
12. `in_transit_state()` clones the current `InTransitOnEdge`
9. `corpse_entities_at()` returns only dead agents at the place
13. `adjacent_places_with_travel_ticks()` returns `(place, ticks)` pairs from topology
14. `estimate_duration()` returns `Some` for fixed, travel, metabolism, trade, combat-weapon, and treatment durations when resolvable; returns `None` when the authoritative resolver would error
15. All existing `BeliefView` / `OmniscientBeliefView` tests remain green
16. Existing suite: `cargo test --workspace`

### Invariants

1. `visible_hostiles_for()` is local — never returns hostiles at remote places
2. `current_attackers_of()` never reads or stores scheduler state in `World`
3. All new methods go through `&World` reads only (no mutation)
4. No new `HashMap`/`HashSet` usage
5. `BeliefView` remains object-safe (`&dyn BeliefView`)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/omniscient_belief_view.rs` — new tests for each added method, including locality and scheduler-backed attacker visibility
2. `crates/worldwake-sim/src/affordance_query.rs` — update stub implementation used by existing affordance tests
3. `crates/worldwake-sim/src/trade_valuation.rs` — update stub implementation used by valuation tests

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Extended `BeliefView` with the 15 E13 belief-side queries described here.
  - Added `OmniscientBeliefRuntime` so `OmniscientBeliefView` can answer runtime-backed combat queries without pushing scheduler state into `World`.
  - Implemented the new omniscient queries with local filtering for hostiles, sellers, corpses, workstations, and sources.
  - Added belief-side duration estimation by delegating omnisciently to the authoritative `DurationExpr::resolve_for()` semantics.
  - Updated the existing `StubBeliefView` implementations in `affordance_query.rs` and `trade_valuation.rs` to match the expanded trait.
  - Added focused tests for the extended trait surface, locality rules, runtime-backed attacker visibility, transit-state reads, and duration estimation.
- Deviations from original plan:
  - The original ticket assumed `World` alone could answer `current_attackers_of()`. That was incorrect; the completed work adds a read-only runtime helper instead.
  - The original ticket treated local hostility too broadly. The completed implementation keeps `visible_hostiles_for()` actor-specific rather than returning unrelated local combatants.
  - `agents_selling_at()` lawfully includes any colocated agent whose `MerchandiseProfile` includes the commodity, including the querying actor when applicable.
- Verification results:
  - `cargo test -p worldwake-sim` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
