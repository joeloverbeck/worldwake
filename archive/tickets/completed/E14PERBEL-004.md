# E14PERBEL-004: Implement PerAgentBeliefView

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new BeliefView implementation in worldwake-sim
**Deps**: E14PERBEL-003 (components must be registered for World access)

## Problem

The AI crate currently queries world state through `OmniscientBeliefView`, which gives agents perfect knowledge. E14 still needs a `PerAgentBeliefView` adapter, but the current `BeliefView` contract is broader than the belief data modeled today. This ticket must therefore implement the new adapter truthfully: belief-backed for the subjective data actually stored in `AgentBeliefStore`, authoritative for self/topology/public-structure queries, and explicit about the remaining trait methods that cannot yet be answered from belief snapshots alone.

## Assumption Reassessment (2026-03-14)

1. `BeliefView` in `crates/worldwake-sim/src/belief_view.rs` is already the planning boundary, but it mixes subjective planning reads with authoritative execution-style reads such as reservation, facility queue, control, and resource-source queries.
2. Several `BeliefView` methods return concrete defaults rather than `Option`, for example `is_alive -> bool`, `commodity_quantity -> Quantity`, and `direct_possessions -> Vec<EntityId>`. Unknown-state behavior must therefore be expressed as conservative defaults, not optional absence.
3. `OmniscientBeliefView` holds `&World` plus optional runtime data and computes many answers from live world structure, not from stored snapshots alone.
4. `AgentBeliefStore` currently stores only `BelievedEntityState { last_known_place, last_known_inventory, alive, wounds, observed_tick, source }` plus `social_observations`.
5. `AgentBeliefStore` does not currently model entity kind, transit state, direct container/possessor, control, reservations, facility queues, workstation/resource metadata, travel disposition, or profile snapshots for other entities.
6. Because of (4) and (5), a full trait-wide "belief only for every non-self query" implementation is not possible without either broadening the belief schema or splitting `BeliefView` into subjective and authoritative sub-interfaces. That larger architectural change is not this ticket.
7. The spec requirement to stop using `OmniscientBeliefView` in agent reasoning remains correct, but actual call-site migration and omniscient-view deletion remain owned by `E14PERBEL-006`.
8. `estimate_duration()` still needs active action state; the new adapter needs a runtime carrier equivalent to `OmniscientBeliefRuntime`.

## Architecture Check

1. `PerAgentBeliefView` should hold the agent `EntityId`, `&World`, `&AgentBeliefStore`, and optional runtime data.
2. This ticket should not change `BeliefView`. Changing the trait now would cascade into a broad planner refactor and overlap with later migration work.
3. The new adapter should draw a hard line in code between:
   - self-authoritative queries,
   - topology/public-infrastructure queries,
   - belief-backed subjective queries,
   - temporary authoritative fallbacks for trait methods whose subjective representation does not exist yet.
4. The ideal long-term architecture is to split planner-facing subjective knowledge from executor/authoritative helpers once E14 migration is complete. This ticket should not fake that split by pretending unsupported subjective fields already exist in `AgentBeliefStore`.

## What to Change

### 1. Create `crates/worldwake-sim/src/per_agent_belief_view.rs`

Define:

```rust
pub struct PerAgentBeliefView<'w> {
    agent: EntityId,
    world: &'w World,
    belief_store: &'w AgentBeliefStore,
    runtime: Option<PerAgentBeliefRuntime<'w>>,
}

pub struct PerAgentBeliefRuntime<'a> {
    pub active_actions: &'a BTreeMap<ActionInstanceId, ActionInstance>,
    pub action_defs: &'a ActionDefRegistry,
}
```

### 2. Implement `BeliefView` for `PerAgentBeliefView`

Route methods according to the current schema, not the aspirational one:

**Self-authoritative queries (from `World`):**
- own needs, wounds, profiles, recipes, carried load, inventory totals, demand memory, effective place, transit state, and other self-state queries
- runtime-sensitive queries such as `estimate_duration()` and `current_attackers_of()`

**Topology/public-structure queries (from `World`):**
- `adjacent_places(place)` → World
- `adjacent_places_with_travel_ticks(place)` → World
- `place_has_tag(place, tag)` → World
- `workstation_tag(entity)` → World
- `matching_workstations_at(place, tag)` → World
- `resource_sources_at(place, commodity)` → World
- facility queue, reservation, and resource-source helpers remain authoritative until the subjective model exists for them

**Belief-backed subjective queries (from `AgentBeliefStore`):**
- `effective_place(id)` for non-self
- `is_alive(id)` for non-self
- `is_dead(id)` for non-self
- `commodity_quantity(id, kind)` for non-self, from `last_known_inventory`
- `wounds(id)` for non-self
- `has_wounds(id)` for non-self
- `entities_at(place)` from believed placements, plus self when authoritative self-location matches
- `corpse_entities_at(place)` from believed dead entities at that believed place

**Known-entity filtered helpers:**
- `agents_selling_at(place, commodity)` may be built from the believed-at-place entity set, then filtered by live world metadata that is not yet snapshot-modeled (for example `MerchandiseProfile`)
- `entity_kind()` and `merchandise_profile()` may return answers only for self or entities already present in the belief store; unknown entities must not become discoverable through these helpers

**Unknown entities:**
- Queries for entities absent from `known_entities` must return conservative trait-shaped defaults: `None`, `false`, `Quantity(0)`, or an empty collection as appropriate.
- This ticket must not silently "help" unknown-entity queries by falling back to full world scans.

### 3. Register module in `crates/worldwake-sim/src/lib.rs`

Add `pub mod per_agent_belief_view;` and re-exports.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add module declaration and re-exports)

## Out of Scope

- Modifying the `BeliefView` trait itself (no trait changes needed)
- Deleting `OmniscientBeliefView` (that's E14PERBEL-006)
- Migrating `agent_tick.rs` call sites (that's E14PERBEL-006)
- Implementing `perception_system()` (that's E14PERBEL-005)
- Expanding `BelievedEntityState` to cover every field in `BeliefView`
- Splitting `BeliefView` into subjective and authoritative sub-traits
- Removing all authoritative fallbacks for methods that have no subjective representation yet
- Adding confidence derivation beyond `PerceptionSource` + staleness (E15 scope)
- Report/rumor perception sources (E15 scope — E14 only uses `DirectObservation`)

## Acceptance Criteria

### Tests That Must Pass

1. Self-query: `view.homeostatic_needs(self_agent)` returns authoritative world data.
2. Self-query: `view.effective_place(self_agent)` and `view.commodity_quantity(self_agent, kind)` remain authoritative.
3. Topology/public-structure queries such as `adjacent_places()`, `adjacent_places_with_travel_ticks()`, and `place_has_tag()` remain authoritative.
4. Observed other: `view.effective_place(other)` returns the believed place when the entity exists in `AgentBeliefStore`.
5. Observed other: `view.is_alive(other)`, `view.is_dead(other)`, `view.wounds(other)`, and `view.commodity_quantity(other, kind)` return believed values.
6. Unknown other: `view.effective_place(unknown)` returns `None`.
7. Unknown other: `view.is_alive(unknown)` returns `false`, `view.commodity_quantity(unknown, kind)` returns `Quantity(0)`, and collection-returning methods return empty collections.
8. `view.entities_at(place)` only returns self plus entities the agent currently believes are at that place.
9. `view.agents_selling_at(place, commodity)` is filtered by believed presence first; unknown merchants do not appear.
10. `view.estimate_duration()` still works when runtime data is supplied.
11. Stale belief data remains stale; the adapter does not auto-refresh from `World`.
12. `PerAgentBeliefView` implements `BeliefView` and compiles as `&dyn BeliefView`.
13. `cargo test -p worldwake-sim`
14. `cargo clippy --workspace`

### Invariants

1. Unknown entities do not become discoverable through `PerAgentBeliefView`.
2. `BeliefView` remains unchanged; `PerAgentBeliefView` is a drop-in implementation at the trait boundary.
3. Belief-backed methods never auto-refresh from authoritative world state.
4. Authoritative fallbacks are explicit and limited to self/topology/public-structure/runtime queries that the current belief schema does not model yet.
5. `DemandMemory` continues as a separate component; `demand_memory()` stays self-authoritative.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — unit tests covering self-authoritative, topology-authoritative, belief-backed, and unknown-entity behavior
2. Tests verifying `&dyn BeliefView` trait-object use with `PerAgentBeliefView`
3. Tests covering stale belief behavior and runtime-backed `estimate_duration()`

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- Completed: 2026-03-14
- Added `PerAgentBeliefView` and `PerAgentBeliefRuntime` in `crates/worldwake-sim/src/per_agent_belief_view.rs`, with explicit routing between self-authoritative reads, topology/public-structure reads, belief-backed reads, and documented authoritative fallbacks for trait methods the current belief schema does not yet model.
- Added focused unit coverage for:
  - trait implementation conformance
  - authoritative self queries vs belief-backed non-self queries
  - unknown-entity hiding
  - stale-belief preservation
  - runtime-backed attacker visibility and duration estimation
- Updated `crates/worldwake-sim/src/lib.rs` to export the new belief-view module and types.
- Deviation from original plan: the original ticket assumed the existing `BeliefView` trait could be answered belief-only for all non-self queries. That assumption was false in the current codebase because `AgentBeliefStore` does not yet model entity kind, transit state, direct possession graphs, reservations, facility queues, resource metadata, or profile snapshots for others. The implemented adapter therefore does not pretend those subjective fields exist; it uses conservative defaults or explicit authoritative fallbacks where the current architecture still requires them.
- Verification:
  - `cargo test -p worldwake-sim`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
