# E14PERBEL-004: Implement PerAgentBeliefView

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new BeliefView implementation in worldwake-sim
**Deps**: E14PERBEL-003 (components must be registered for World access)

## Problem

The AI crate currently queries world state through `OmniscientBeliefView`, which gives agents perfect knowledge. E14 requires a `PerAgentBeliefView` struct that implements the `BeliefView` trait by reading from the agent's `AgentBeliefStore` for observed entities, while providing authoritative answers for self-queries and topology queries. This is the core of the world/belief separation.

## Assumption Reassessment (2026-03-14)

1. `BeliefView` trait has ~50 methods in `crates/worldwake-sim/src/belief_view.rs` — confirmed.
2. The trait is already used as `&dyn BeliefView` in affordance queries and planning — confirmed.
3. `OmniscientBeliefView` holds `&World` and optional `OmniscientBeliefRuntime` (active_actions + action_defs) — confirmed.
4. Self-queries (homeostatic_needs, wounds, combat_profile for self, own inventory) must remain authoritative from World — per spec.
5. Topology queries (adjacent_places, place_has_tag, workstation_tag, etc.) must remain authoritative — per spec, place graph is public infrastructure.
6. Observed entity queries (effective_place, is_alive, commodity_quantity for others, etc.) must read from `AgentBeliefStore` — per spec.
7. Unknown entities return `None`/empty/default — per spec's Default Ignorance Policy.
8. `estimate_duration()` needs active action state — currently provided by `OmniscientBeliefRuntime`.

## Architecture Check

1. `PerAgentBeliefView` holds: agent `EntityId`, `&World` (for self-queries and topology), `&AgentBeliefStore` (for observed entities). Active action data passed similarly to `OmniscientBeliefRuntime`.
2. No `BeliefView` trait changes required — the trait is already correctly abstracted. Only the implementation changes.
3. The view does not hold `&World` for observed-entity queries — it reads belief store. This enforces the separation.

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

Route each of the ~50 methods to the correct data source:

**Self-queries (authoritative from World):**
- `homeostatic_needs(id)` where `id == self.agent` → World
- `wounds(id)` where `id == self.agent` → World
- `combat_profile(id)` where `id == self.agent` → World
- `trade_disposition_profile(id)` where `id == self.agent` → World
- `metabolism_profile(id)` where `id == self.agent` → World
- `drive_thresholds(id)` where `id == self.agent` → World
- `known_recipes(id)` where `id == self.agent` → World
- `merchandise_profile(id)` where `id == self.agent` → World
- `carry_capacity(id)` where `id == self.agent` → World
- `load_of_entity(id)` where `id == self.agent` → World
- `direct_possessions(id)` where `id == self.agent` → World (agent knows own inventory)
- `commodity_quantity(id, kind)` where `id == self.agent` → World
- `effective_place(id)` where `id == self.agent` → World
- `is_alive(id)` where `id == self.agent` → World (always true if querying)
- `demand_memory(id)` where `id == self.agent` → World (DemandMemory is agent's own domain memory)

**Topology queries (authoritative from World):**
- `adjacent_places(place)` → World
- `adjacent_places_with_travel_ticks(place)` → World
- `place_has_tag(place, tag)` → World
- `workstation_tag(entity)` → World
- `matching_workstations_at(place, tag)` → World
- `resource_sources_at(place)` → World

**Observed entity queries (from AgentBeliefStore):**
- `effective_place(id)` where `id != self.agent` → belief_store.get_entity(id).last_known_place
- `is_alive(id)` where `id != self.agent` → belief_store.get_entity(id).alive
- `commodity_quantity(id, kind)` where `id != self.agent` → belief_store inventory
- `direct_possessions(id)` where `id != self.agent` → belief_store inventory
- `wounds(id)` where `id != self.agent` → belief_store wounds
- `agents_selling_at(place)` → filter known entities with merchandise profiles at believed place
- `corpse_entities_at(place)` → filter known dead entities at believed place
- `visible_hostiles_for(id)` → only hostiles the agent has perceived
- Profile queries for others → from belief store if observed, None otherwise

**Unknown entities:**
- `entities_at(place)` → only returns entities the agent believes are at that place
- Any query about an entity not in `known_entities` → `None`/empty/default

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
- Handling `DemandMemory` — it remains a separate component, `BeliefView::demand_memory()` continues reading it directly
- Adding confidence derivation beyond `PerceptionSource` + staleness (E15 scope)
- Report/rumor perception sources (E15 scope — E14 only uses `DirectObservation`)

## Acceptance Criteria

### Tests That Must Pass

1. Self-query: `view.homeostatic_needs(self_agent)` returns authoritative data from World
2. Self-query: `view.effective_place(self_agent)` returns authoritative place from World
3. Self-query: `view.direct_possessions(self_agent)` returns authoritative inventory from World
4. Topology: `view.adjacent_places(place)` returns authoritative topology from World
5. Topology: `view.place_has_tag(place, tag)` returns authoritative tag from World
6. Observed other: `view.effective_place(other)` returns believed place when other is known
7. Observed other: `view.is_alive(other)` returns believed alive status
8. Observed other: `view.commodity_quantity(other, kind)` returns believed quantity
9. Unknown other: `view.effective_place(unknown)` returns `None`
10. Unknown other: `view.is_alive(unknown)` returns `None` or `false`
11. Unknown other: `view.entities_at(place)` does not include unknown entities
12. `view.agents_selling_at(place)` only includes entities the agent believes are merchants at that place
13. `view.visible_hostiles_for(agent)` only includes perceived hostiles
14. `view.estimate_duration()` works with runtime active actions
15. Stale belief: after many ticks, belief store still returns old data (not auto-updated)
16. `PerAgentBeliefView` implements `BeliefView` — compiles as `&dyn BeliefView`
17. `cargo test -p worldwake-sim`
18. `cargo clippy --workspace`

### Invariants

1. `PerAgentBeliefView` does NOT hold `&World` for observed-entity queries — only for self and topology
2. Agent never learns about entities it hasn't observed (Default Ignorance Policy)
3. `BeliefView` trait is unchanged — `PerAgentBeliefView` is a drop-in replacement
4. Stale beliefs are returned as-is — no auto-refresh from World
5. `DemandMemory` continues as separate component (no change to `demand_memory()` routing)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — comprehensive unit tests for all 3 query categories
2. Tests verifying `&dyn BeliefView` trait object works with `PerAgentBeliefView`

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
