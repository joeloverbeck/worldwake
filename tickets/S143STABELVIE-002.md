# S143STABELVIE-002: New belief-view traits — `LocalPhysicalObservationView`, `BelievedAuthorityView`, `DebugWorldView`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new trait surfaces in `worldwake-sim/src/belief_view.rs`; canonical impls on `PerAgentBeliefView`. No method migrations or supertrait changes in this ticket.
**Deps**: archive/tickets/S143STABELVIE-001.md

## Problem

S143's three new trait surfaces must exist before any method can migrate to them. Defining them free-standing (NOT yet supertraits of `RuntimeBeliefView`) keeps this ticket compile-safe and isolates the trait-shape decisions from the workspace-wide migration cascade in tickets 003/004. The canonical impl on `PerAgentBeliefView` provides the production behavior; default trait impls absorb mock-impl cascades from `TestBeliefView` sites when tickets 003/004 promote the new traits into the `RuntimeBeliefView` supertrait list.

## Assumption Reassessment (2026-05-13)

1. `PerAgentBeliefView` lives at `crates/worldwake-sim/src/per_agent_belief_view.rs` and is the canonical impl of `RuntimeBeliefView` (the file holds scattered `impl <SubTrait> for PerAgentBeliefView` blocks for each of the 11 sub-traits). New trait impls land alongside the existing impl blocks. FND-14A enforcement comments + runtime co-location assertions live in this file (e.g., lines 617, 1959, 1968, 1978).
2. Default impls returning empty/Unknown values follow the existing convention — e.g., `EntityBeliefView::is_dead` has a default forwarding to `!is_alive(entity)` (`belief_view.rs:804`), and several methods on `EntityBeliefView`, `SpatialBeliefView`, `GoalSpatialBeliefView` use the `let _ = (…); None` / `let _ = (…); Vec::new()` placeholder pattern. The new traits adopt the same style so future ticket-003/004 mock cascades can absorb defaults without explicit overrides.
3. Net-new methods on `BelievedAuthorityView` (`believed_holder_of`, `believed_access_right`, `believed_jurisdiction`) have no current data source in `AgentBeliefStore` for absent-domain cases. Canonical impl returns `BeliefRead::Unknown` for these methods initially; ticket 003 wires the methods to real belief-store reads as part of the BelievedAuthorityView migration.
4. Adjacent contradiction (was item 13): spec D4's `DebugWorldView` value-prop is *labeling* discipline, not type-enforced firewall. `worldwake-ai` can already reach `&World` accessors directly (via `worldwake-core` dependency); the cfg-gate + ticket-005's CI lint together provide enforcement, not the trait itself. Classification: required consequence — the trait's existence is the parking surface for future debug accessors; ticket 005 closes the enforcement loop.

## Architecture Check

1. Free-standing traits avoid the workspace-wide cascade that adding them to `RuntimeBeliefView`'s supertrait list would force. This isolates trait-shape from migration logic, keeping each ticket independently reviewable.
2. Default impls (returning `Unknown`/empty) follow existing belief-view conventions — no architectural novelty.
3. `DebugWorldView` is `#[cfg(any(debug_assertions, test))]`-gated at the trait, `&World` impl, and `pub use` levels — release builds of `worldwake-ai` cannot resolve the symbol.
4. FND-28-clean: no temporary alias paths. The canonical `PerAgentBeliefView` impl provides real reads for `LocalPhysicalObservationView` (FND-14A-legal authoritative reads) and `BeliefRead::Unknown` defaults for `BelievedAuthorityView` (until ticket 003 wires real reads).

## Verification Layers

1. Trait surface compiles and is reachable as a free-standing import — `cargo build --workspace`.
2. `LocalPhysicalObservationView::colocated_entities` on `PerAgentBeliefView` returns FND-14A-legal physical observations matching the legacy `locally_observed_entities_at` behavior — focused unit test.
3. `BelievedAuthorityView::believed_owner_of` and `believed_office_holder` default impls return `BeliefRead::Unknown` (real reads land in ticket 003) — focused unit test.
4. `DebugWorldView` impl on `&World` produces consistent `EntityState` snapshots — cfg-gated focused unit test.
5. Single-layer ticket (trait surface introduction); downstream behavioral mapping happens in tickets 003/004.

## What to Change

### 1. Trait definitions in `crates/worldwake-sim/src/belief_view.rs`

Add three new traits (free-standing, not yet supertraits of `RuntimeBeliefView`):

```rust
pub trait LocalPhysicalObservationView {
    fn colocated_entities(&self, actor: EntityId) -> ObservedRead<Vec<EntityId>> {
        let _ = actor;
        ObservedRead { value: Vec::new(), observed_tick: Tick(0), source: ObservationSource::CoLocatedSameTick }
    }
    fn observed_item_lot_quantity(&self, lot: EntityId) -> ObservedRead<Option<Quantity>> { … }
    fn observed_workstation_tag(&self, entity: EntityId) -> ObservedRead<Option<WorkstationTag>> { … }
    fn observed_resource_source(&self, entity: EntityId) -> ObservedRead<Option<ResourceSource>> { … }
    fn observed_container_contents(&self, container: EntityId) -> ObservedRead<Vec<EntityId>> { … }
    fn observed_entity_kind(&self, entity: EntityId) -> ObservedRead<Option<EntityKind>> { … }
}

pub trait BelievedAuthorityView {
    fn believed_owner_of(&self, entity: EntityId) -> BeliefRead<EntityId> { let _ = entity; BeliefRead::Unknown }
    fn believed_holder_of(&self, entity: EntityId) -> BeliefRead<EntityId> { let _ = entity; BeliefRead::Unknown }
    fn believed_access_right(&self, actor: EntityId, target: EntityId) -> BeliefRead<EffectiveRight> { let _ = (actor, target); BeliefRead::Unknown }
    fn believed_jurisdiction(&self, place: EntityId) -> BeliefRead<EntityId> { let _ = place; BeliefRead::Unknown }
    fn believed_office_holder(&self, office: EntityId) -> BeliefRead<EntityId> { let _ = office; BeliefRead::Unknown }
}

#[cfg(any(debug_assertions, test))]
pub trait DebugWorldView {
    fn world_entity_state(&self, entity: EntityId) -> EntityState;
    fn world_owner_of(&self, entity: EntityId) -> Option<EntityId>;
    fn world_location_of(&self, entity: EntityId) -> Option<EntityId>;
    fn world_inventory_of(&self, entity: EntityId) -> Vec<EntityId>;
}
```

### 2. Canonical impls on `PerAgentBeliefView` (`crates/worldwake-sim/src/per_agent_belief_view.rs`)

Add `impl LocalPhysicalObservationView for PerAgentBeliefView` with real co-located observation behavior:
- `colocated_entities(actor)` — same authoritative-read path as today's `SpatialBeliefView::locally_observed_entities_at(actor, current_place)`, wrapped in `ObservedRead { source: CoLocatedSameTick }`.
- `observed_item_lot_quantity(lot)` — read `ItemLot` quantity via `World::get_component_item_lot` when actor is co-located with the lot; `None` otherwise.
- `observed_workstation_tag(entity)` — wrap `World::get_component_workstation_marker` for co-located entities.
- `observed_resource_source(entity)` — wrap `World::get_component_resource_source` for co-located entities.
- `observed_container_contents(container)` — wrap inventory accessor for co-located container.
- `observed_entity_kind(entity)` — wrap `World::entity_kind` for co-located entities.

Add `impl BelievedAuthorityView for PerAgentBeliefView` — all 5 methods take the default `BeliefRead::Unknown` for now. Ticket 003 overrides each with the real belief-store read (for migrated methods `believed_owner_of` and `believed_office_holder`) or wires net-new methods (`believed_holder_of`, `believed_access_right`, `believed_jurisdiction`) to belief-store entries.

Add cfg-gated `impl DebugWorldView for &World` with methods wrapping existing `&World` accessors:
- `world_entity_state(e)` — assembles an `EntityState` from `World::entity_kind`, `World::effective_place`, `World::is_alive`, `World::direct_container`, `World::direct_possessor`.
- `world_owner_of(e)` — wraps the existing `World::possessions_of` reverse-lookup or the canonical ownership accessor.
- `world_location_of(e)` — wraps `World::effective_place`.
- `world_inventory_of(e)` — wraps `World::possessions_of`.

### 3. Re-exports in `crates/worldwake-sim/src/lib.rs`

Extend the existing `pub use belief_view::{ … };` to include `LocalPhysicalObservationView, BelievedAuthorityView`. Add `#[cfg(any(debug_assertions, test))] pub use belief_view::DebugWorldView;`.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — add three trait definitions and the cfg-gate)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — add canonical impls; the file already contains all other sub-trait impls)
- `crates/worldwake-sim/src/lib.rs` (modify — extend re-export list)

## Out of Scope

- Adding new traits as supertraits of `RuntimeBeliefView` — deferred to tickets 003 (`BelievedAuthorityView`) and 004 (`LocalPhysicalObservationView`). Each migration ticket promotes its own supertrait alongside the method moves.
- Migrating existing methods (`believed_owner_of`, `believed_office_holder`, `locally_observed_entities_at`) — tickets 003 and 004.
- Updating consumer call sites — tickets 003 and 004.
- CI lint (D7) — ticket 005.
- Golden coverage (D8) — ticket 006.

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: `BelievedAuthorityView` default impls return `BeliefRead::Unknown` for all 5 methods when called on a minimal test struct that only `impl BelievedAuthorityView for _ {}`.
2. New focused test: `LocalPhysicalObservationView::colocated_entities` on `PerAgentBeliefView` returns `ObservedRead` with `source == CoLocatedSameTick` and `value` matching the legacy `locally_observed_entities_at` result for the same agent/place pair.
3. New cfg-gated focused test: `DebugWorldView::world_entity_state` on `&World` returns an `EntityState` consistent with `World`'s authoritative state for a small scenario.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. The three new traits are free-standing — `RuntimeBeliefView`'s supertrait chain does not yet reference them.
2. `BelievedAuthorityView` canonical impl on `PerAgentBeliefView` reads only from belief-store data (or returns `Unknown`); never reads authoritative world state.
3. `LocalPhysicalObservationView` canonical impl enforces FND-14A co-location at read time — returns `ObservedRead { source: CoLocatedSameTick, … }` only when the actor is co-located with the subject; returns the empty/None default value otherwise.
4. `DebugWorldView` symbol is unreachable from release builds of any `worldwake-ai/src/**.rs` source file (cfg-gate enforcement; CI lint added in ticket 005 closes the loop).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` `#[cfg(test)]` — defaults exercise for the three new traits (3–4 focused tests).
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` `#[cfg(test)]` — canonical impl smoke tests for `LocalPhysicalObservationView::colocated_entities` and the cfg-gated `DebugWorldView` impl.

### Commands

1. `cargo test -p worldwake-sim belief_view`
2. `cargo test -p worldwake-sim per_agent_belief_view`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
