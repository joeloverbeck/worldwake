# E18BANDYN-001: Add BanditCamp and BanditCampProfile components

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-core authoritative component surface, typed delta/value surface, transaction setters
**Deps**: E16 (faction system — completed), E12 (combat — completed)

## Problem

E18 requires two new components on Place entities: `BanditCamp` (marks a place as a bandit camp with its communal supply container reference) and `BanditCampProfile` (per-camp behavioral thresholds and rally point). These are foundational types that later E18 tickets need for actions, abandonment checks, and AI integration.

## Assumption Reassessment (2026-03-29)

1. `PlaceTag::Camp` and `PlaceTag::Forest` already exist in [`crates/worldwake-core/src/topology.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/topology.rs), and the prototype topology already defines `PrototypePlace::BanditCamp` with both tags. No topology or prototype-world changes are required in this ticket.
2. `EntityKind::Place` already participates in the authoritative component schema through `with_component_schema_entries!()` in [`crates/worldwake-core/src/component_schema.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_schema.rs). The live extension pattern is broader than the original ticket claimed: adding an authoritative component also feeds `ComponentTables`, `World` component CRUD/query methods, `ComponentKind`/`ComponentValue`, and `WorldTxn` simple set/clear setters via shared macros.
3. The live faction boundary is `bandit membership -> MemberOf relation -> faction entity`, not `camp -> faction field`. `World::members_of()` already exposes the canonical living-member query in [`crates/worldwake-core/src/world/social.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/social.rs). Storing faction again on `BanditCamp` would duplicate authority and make later camp/faction divergence possible.
4. `Permille` and `NonZeroU32` are already the established value types for profile thresholds and durations across `worldwake-core`; `EntityId` is the standard cross-entity pointer type for `supplies` and `rally_place`.
5. No `BanditCamp` or `BanditCampProfile` component currently exists anywhere in `crates/`. This is still net-new work.
6. Existing focused tests already prove the relevant live patterns:
   - place-scoped component roundtrips and kind restrictions in [`crates/worldwake-core/src/world.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs) for `ResourceSource` and `ProductionOutputOwnershipPolicy`
   - component table roundtrips in [`crates/worldwake-core/src/component_tables.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_tables.rs)
   - typed delta/value completeness in [`crates/worldwake-core/src/delta.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/delta.rs)
   - transaction manifest parity in [`crates/worldwake-core/src/world_txn.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs)
7. Mismatch + correction: the ticket’s original `BanditCamp { faction, supplies }` shape conflicts with the live spec in [`specs/E18-bandit-dynamics.md`](/home/joeloverbeck/projects/worldwake/specs/E18-bandit-dynamics.md), which deliberately keeps faction membership canonical in relations and stores only `supplies` on `BanditCamp`. This ticket is corrected to follow the spec and current architecture.
8. Mismatch + correction: the original files-to-touch list was incomplete for this repo. Because authoritative components participate in typed deltas and transaction setters, [`crates/worldwake-core/src/delta.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/delta.rs) and focused tests in [`crates/worldwake-core/src/world_txn.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs) are in scope if needed to keep the authoritative surface coherent.

## Architecture Check

1. Two small Place-scoped components match the current authoritative component architecture and keep camp identity explicit without inventing a new entity kind. The clean design is:
   - `BanditCamp` stores only the camp’s communal supply container reference
   - `BanditCampProfile` stores profile parameters and rally-point knowledge
   - faction membership remains canonical in `MemberOf`
   This is cleaner than storing redundant faction authority on the camp component because later systems can derive member count and living membership from one source of truth instead of reconciling two.
2. Wiring the full authoritative component surface now is more robust than a partial schema/table-only addition. It keeps event-log deltas, verification, and transaction mutation APIs in sync with the new components from day one.
3. No backwards-compatibility aliasing or shim fields. Net-new types only.

## Verification Layers

1. Place-only attachment and CRUD semantics -> focused `worldwake-core` unit tests against `World` authoritative component APIs
2. Component-table storage wiring -> focused `component_tables` unit tests
3. Typed delta/value completeness for the new authoritative components -> focused `delta` unit tests
4. Transaction setter parity for the new simple-set components -> focused `world_txn` unit tests
5. Serialization/regression coverage for the authoritative world surface -> `cargo test -p worldwake-core`
6. Additional AI/action trace mapping is not applicable yet because this ticket only adds core component types, not planning or runtime behavior

## What to Change

### 1. Define `BanditCamp` component

In a new file `crates/worldwake-core/src/bandit_camp.rs`:

```rust
/// Marks a Place entity as a bandit camp. Minimal stored state;
/// faction membership stays canonical in MemberOf relations,
/// while the camp stores only its communal supplies container.
pub struct BanditCamp {
    /// Container entity holding the camp's communal supplies.
    pub supplies: EntityId,
}
```

### 2. Define `BanditCampProfile` component

In the same file:

```rust
/// Per-camp profile controlling bandit behavior thresholds.
/// All thresholds are Permille (0-1000) to comply with spec drafting rules.
pub struct BanditCampProfile {
    /// Minimum living faction members needed to establish a new camp.
    pub min_regroup_count: u8,
    /// Ticks required to establish a new camp via EstablishCamp action.
    pub establishment_duration_ticks: NonZeroU32,
    /// Wound-load threshold (as fraction of capacity) above which
    /// a bandit prioritizes fleeing over fighting.
    pub flee_wound_threshold: Permille,
    /// Known rally place where faction members should regroup after
    /// camp destruction. Observable state — members learn through perception.
    pub rally_place: Option<EntityId>,
}
```

Do not add `abandonment_grace_ticks` here in this ticket. The live spec currently defines that value in the abandonment-system narrative but not in the component contract. If a later ticket needs that profile field, add it there with the corresponding spec update instead of silently extending this foundational type now.

### 3. Register in component schema

Add both components to `with_component_schema_entries!()` in `crates/worldwake-core/src/component_schema.rs`, restricted to `EntityKind::Place`.

### 4. Wire the full authoritative component surface

Ensure the new components participate in the same authoritative surfaces as existing simple-set components:

- `ComponentTables` storage and tests
- `ComponentKind` / `ComponentValue` coverage and tests
- `World` component CRUD/query methods through the shared schema macros
- `WorldTxn` simple set/clear surface and focused tests as needed

### 5. Re-export from lib.rs

Add `pub mod bandit_camp;` and re-export types from `crates/worldwake-core/src/lib.rs`.

## Files to Touch

- `crates/worldwake-core/src/bandit_camp.rs` (new — component definitions)
- `crates/worldwake-core/src/component_schema.rs` (modify — register both components)
- `crates/worldwake-core/src/component_tables.rs` (modify — add storage fields)
- `crates/worldwake-core/src/delta.rs` (modify — typed component delta/value coverage)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)
- `crates/worldwake-core/src/world.rs` (modify — focused authoritative component tests)
- `crates/worldwake-core/src/world_txn.rs` (modify — focused transaction-surface tests if required by manifest parity)

## Out of Scope

- Raid action definition or handler (E18BANDYN-003)
- EstablishCamp action definition or handler (E18BANDYN-004)
- `bandit_camp_system()` abandonment logic (E18BANDYN-005)
- AI candidate generation for bandit goals (E18BANDYN-006)
- GoalKind variants (E18BANDYN-002)
- Route threat estimation (E18BANDYN-008)
- Golden test T22 (E18BANDYN-009)
- Any changes to `Topology` or `build_prototype_world` — prototype world already has `BanditCamp` place
- Adding redundant faction aliases or compatibility fields to `BanditCamp`
- Extending the spec-owned profile contract beyond what `specs/E18-bandit-dynamics.md` currently declares

## Acceptance Criteria

### Tests That Must Pass

1. Insert `BanditCamp` on a Place entity, retrieve it, verify fields match
2. Insert `BanditCampProfile` on a Place entity, retrieve it, verify all fields
3. Attempting to insert either component on a non-Place entity fails appropriately
4. Remove each component from a Place entity succeeds
5. The new components are represented in the typed component delta/value surface
6. The new components participate in the `WorldTxn` simple set manifest without breaking parity checks
7. Existing suite: `cargo test -p worldwake-core`
8. Existing suite: `cargo clippy --workspace`
9. Existing suite: `cargo build --workspace`

### Invariants

1. `BanditCamp` is only attachable to `EntityKind::Place` entities
2. `BanditCampProfile` is only attachable to `EntityKind::Place` entities
3. No `f32`/`f64` anywhere — `Permille` for fractional values, `NonZeroU32` for tick counts
4. `BanditCamp` does not duplicate faction authority already carried by `MemberOf`
5. Existing conservation invariants remain unaffected (`verify_live_lot_conservation`, `verify_authoritative_conservation`)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/component_tables.rs` — table roundtrip coverage for the two new component storages
2. `crates/worldwake-core/src/world.rs` — authoritative CRUD and Place-only kind restriction coverage
3. `crates/worldwake-core/src/delta.rs` — typed component kind/value coverage for the new components
4. `crates/worldwake-core/src/world_txn.rs` — simple set/clear delta coverage if the new components are transaction-managed simple-set components

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo build --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed:
  - Added `BanditCamp` and `BanditCampProfile` as new `worldwake-core` components in `crates/worldwake-core/src/bandit_camp.rs`
  - Registered both as Place-only authoritative components in the shared component schema
  - Wired them through `ComponentTables`, typed `ComponentKind` / `ComponentValue`, `World` component CRUD, and `WorldTxn` simple set/clear mutation surfaces
  - Added focused core tests for component bounds/serde, table storage, place-only world CRUD, typed delta coverage, and transaction delta recording
- Deviations from original plan:
  - Removed the ticket's original `BanditCamp.faction` field because it duplicated canonical faction membership already carried by `MemberOf`; this would have been weaker architecture than the live spec
  - Did not add `abandonment_grace_ticks` to `BanditCampProfile` because the current E18 spec narrative mentions abandonment timing but the live component contract does not yet declare that field; adding it here would have silently expanded spec scope
  - Expanded implementation beyond schema/table registration to include the full authoritative component surface this repo requires (`delta.rs`, `world_txn.rs`, and focused tests)
- Verification results:
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace` passed
  - `cargo build --workspace` passed
