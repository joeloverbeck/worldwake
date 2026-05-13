# S143STABELVIE-002: New belief-view traits — `LocalPhysicalObservationView`, `BelievedAuthorityView`, `DebugWorldView`

**Status**: COMPLETED
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

## Verified Layers

1. Trait surface compiled and stayed reachable as free-standing imports through `cargo test --workspace --no-run`, `cargo test --workspace`, and CI-matching clippy.
2. `LocalPhysicalObservationView::colocated_entities` on `PerAgentBeliefView` returned FND-14A-legal physical observations matching the legacy `locally_observed_entities_at` behavior — focused unit test.
3. `BelievedAuthorityView::believed_owner_of` and `believed_office_holder` default impls returned `BeliefRead::Unknown` (real reads remain ticket 003 work) — focused unit test.
4. `DebugWorldView` impl on `&World` produced consistent `EntityState` snapshots — cfg-gated focused unit test.
5. Single-layer ticket (trait surface introduction); downstream behavioral mapping remains in tickets 003/004.

## Landed Changes

### 1. Trait definitions in `crates/worldwake-sim/src/belief_view.rs`

Added three traits (free-standing, not yet supertraits of `RuntimeBeliefView`):

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

Added `impl LocalPhysicalObservationView for PerAgentBeliefView` with real co-located observation behavior:
- `colocated_entities(actor)` — same authoritative-read path as legacy `SpatialBeliefView::locally_observed_entities_at(actor, current_place)`, wrapped in `ObservedRead { source: CoLocatedSameTick }`.
- `observed_item_lot_quantity(lot)` — read `ItemLot` quantity via `World::get_component_item_lot` when actor is co-located with the lot; `None` otherwise.
- `observed_workstation_tag(entity)` — wrap `World::get_component_workstation_marker` for co-located entities.
- `observed_resource_source(entity)` — wrap `World::get_component_resource_source` for co-located entities.
- `observed_container_contents(container)` — wrap inventory accessor for co-located container.
- `observed_entity_kind(entity)` — wrap `World::entity_kind` for co-located entities.

Added `impl BelievedAuthorityView for PerAgentBeliefView` — all 5 methods take the default `BeliefRead::Unknown` in this ticket. Ticket 003 overrides each with the real belief-store read (for migrated methods `believed_owner_of` and `believed_office_holder`) or wires net-new methods (`believed_holder_of`, `believed_access_right`, `believed_jurisdiction`) to belief-store entries.

Added cfg-gated `impl DebugWorldView for &World` with methods wrapping existing `&World` accessors:
- `world_entity_state(e)` — assembles an `EntityState` from `World::entity_kind`, `World::effective_place`, `World::is_alive`, `World::direct_container`, `World::possessor_of`.
- `world_owner_of(e)` — wraps the canonical `World::owner_of` accessor.
- `world_location_of(e)` — wraps `World::effective_place`.
- `world_inventory_of(e)` — wraps `World::possessions_of`.

### 3. Re-exports in `crates/worldwake-sim/src/lib.rs`

Extended the existing `pub use belief_view::{ … };` to include `LocalPhysicalObservationView, BelievedAuthorityView`. Added `#[cfg(any(debug_assertions, test))] pub use belief_view::DebugWorldView;`.

## Landed Files

- `crates/worldwake-sim/src/belief_view.rs` (modify — add three trait definitions and the cfg-gate)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — add canonical impls; the file already contains all other sub-trait impls)
- `crates/worldwake-sim/src/lib.rs` (modify — extend re-export list)

## Out of Scope

- Adding new traits as supertraits of `RuntimeBeliefView` — deferred to tickets 003 (`BelievedAuthorityView`) and 004 (`LocalPhysicalObservationView`). Each migration ticket promotes its own supertrait alongside the method moves.
- Migrating existing methods (`believed_owner_of`, `believed_office_holder`, `locally_observed_entities_at`) — tickets 003 and 004.
- Updating consumer call sites — tickets 003 and 004.
- CI lint (D7) — ticket 005.
- Golden coverage (D8) — ticket 006.

## Acceptance Result

### Tests That Must Pass

1. Focused test added: `BelievedAuthorityView` default impls return `BeliefRead::Unknown` for all 5 methods when called on a minimal test struct that only `impl BelievedAuthorityView for _ {}`.
2. Focused test added: `LocalPhysicalObservationView::colocated_entities` on `PerAgentBeliefView` returns `ObservedRead` with `source == CoLocatedSameTick` and `value` matching the legacy `locally_observed_entities_at` result for the same agent/place pair.
3. Cfg-gated focused test added: `DebugWorldView::world_entity_state` on `&World` returns an `EntityState` consistent with `World`'s authoritative state for a small scenario.
4. Existing suite passed: `cargo test --workspace`.

### Invariants

1. The three introduced traits are free-standing — `RuntimeBeliefView`'s supertrait chain does not reference them.
2. `BelievedAuthorityView` canonical impl on `PerAgentBeliefView` reads only from belief-store data (or returns `Unknown`); never reads authoritative world state.
3. `LocalPhysicalObservationView` canonical impl enforces FND-14A co-location at read time — returns `ObservedRead { source: CoLocatedSameTick, … }` only when the actor is co-located with the subject; returns the empty/None default value otherwise.
4. `DebugWorldView` symbol is unreachable from release builds of any `worldwake-ai/src/**.rs` source file (cfg-gate enforcement; CI lint added in ticket 005 closes the loop).

## Test Plan Result

### Focused Tests

1. `crates/worldwake-sim/src/belief_view.rs` `#[cfg(test)]` — defaults exercise for the three introduced traits.
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` `#[cfg(test)]` — canonical impl smoke tests for `LocalPhysicalObservationView::colocated_entities` and the cfg-gated `DebugWorldView` impl.

### Commands Passed

1. `cargo test -p worldwake-sim belief_view`
2. `cargo test -p worldwake-sim per_agent_belief_view`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-13.

- Added free-standing `LocalPhysicalObservationView`, `BelievedAuthorityView`, and cfg-gated `DebugWorldView` in `crates/worldwake-sim/src/belief_view.rs`.
- Implemented `LocalPhysicalObservationView` for `PerAgentBeliefView` with co-location-gated physical reads and current-tick `ObservedRead` metadata.
- Implemented `BelievedAuthorityView` for `PerAgentBeliefView` through the default `BeliefRead::Unknown` methods, leaving real belief-store migration to tickets 003/004 as planned.
- Implemented cfg-gated `DebugWorldView` for `&World` and re-exported the new trait surfaces from `worldwake-sim`.
- Added focused unit coverage for trait defaults, `PerAgentBeliefView` co-located observation reads, `BelievedAuthorityView` default behavior on the canonical view, and `DebugWorldView` authoritative snapshots.

## Deviations

- `world_owner_of` landed on the existing `World::owner_of` accessor rather than an inferred `possessions_of` reverse lookup.
- The compile sweep exposed one internal ambiguity after adding `BelievedAuthorityView::believed_office_holder`; `visible_reward_encumbrance` now explicitly calls `PoliticalBeliefView::believed_office_holder`, preserving the existing behavior.
- The three new traits remain free-standing; `RuntimeBeliefView` still does not list them as supertraits in this ticket.

## Verification Result

- Passed `cargo test -p worldwake-sim --lib belief_view::tests::believed_authority_view_defaults_return_unknown -- --exact`.
- Passed `cargo test -p worldwake-sim --lib belief_view::tests::local_physical_observation_view_defaults_return_empty_same_tick_reads -- --exact`.
- Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::believed_authority_view_on_per_agent_view_starts_unknown -- --exact`.
- Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::local_physical_observation_view_colocated_entities_matches_legacy_read -- --exact`.
- Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::local_physical_observation_view_gates_subject_reads_by_colocation -- --exact`.
- Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::debug_world_view_reports_authoritative_entity_state -- --exact`.
- Passed `cargo test --workspace --no-run`.
- Passed `cargo test -p worldwake-sim belief_view`.
- Passed `cargo test -p worldwake-sim per_agent_belief_view`.
- Passed `cargo test --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `python3 .codex/skills/implement-ticket/scripts/check_closeout.py tickets/S143STABELVIE-002.md`.
- Passed `git diff --check`.
