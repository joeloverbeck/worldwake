# E18BANDYN-001: Add BanditCamp and BanditCampProfile components

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-core component tables, component schema
**Deps**: E16 (faction system — completed), E12 (combat — completed)

## Problem

E18 requires two new components on Place entities: `BanditCamp` (marks a place as a bandit camp with a supply container reference) and `BanditCampProfile` (per-camp behavioral thresholds and rally point). These are foundational types that all other E18 tickets depend on.

## Assumption Reassessment (2026-03-29)

1. `PlaceTag::Camp` and `PlaceTag::Forest` already exist in `crates/worldwake-core/src/topology.rs`. The prototype world already includes a `BanditCamp` prototype place. No new `PlaceTag` variants needed.
2. `EntityKind::Place` exists. Component schema in `crates/worldwake-core/src/component_schema.rs` uses `with_component_schema_entries!()` macro with kind-checker closures to restrict which entity kinds accept which components. Pattern is well-established with 47 existing components.
3. `Permille` type exists in `crates/worldwake-core/src/numerics.rs` — used for all [0,1000] range values per spec drafting rules.
4. `EntityId` is the standard reference type for cross-entity pointers (e.g., `rally_place: Option<EntityId>`).
5. `NonZeroU32` is used for duration ticks throughout the codebase (e.g., `OfficeForceProfile`).
6. `MemberOf` relation exists in `crates/worldwake-core/src/relations.rs` — `members_of(faction_id)` query is available.
7. No existing `BanditCamp` or `BanditCampProfile` component exists in the codebase — this is net-new work.

## Architecture Check

1. Two small, focused components on Place entities follows the existing pattern (e.g., `WorkstationMarker` on Place, `ResourceSource` on Place). No new entity kinds needed. No new relation types needed. Profile-driven thresholds replace all magic numbers (FND-2). `BanditCamp` stores only the supply container reference — member count, combat strength, and supply level are all derived (FND-3, FND-25).
2. No backwards-compatibility shims. Net-new types only.

## Verification Layers

1. Component registration correctness → focused unit test: insert/get/remove `BanditCamp` and `BanditCampProfile` on Place entities
2. Kind restriction enforcement → focused unit test: inserting on non-Place entity panics/errors
3. Serialization roundtrip → existing `save_load` tests pass after adding new components to component tables
4. `abandonment_grace_ticks` is profile-driven (not hardcoded) → structural: field exists on `BanditCampProfile`

## What to Change

### 1. Define `BanditCamp` component

In a new file `crates/worldwake-core/src/bandit_camp.rs`:

```rust
/// Marks a Place entity as a bandit camp. Minimal stored state;
/// membership tracked via MemberOf relations to the camp's faction,
/// combat capability via per-agent CombatProfile, survival state
/// via WoundList and HomeostaticNeeds.
pub struct BanditCamp {
    /// The faction entity that owns this camp.
    pub faction: EntityId,
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
    /// Ticks with zero living faction members present before camp is
    /// marked abandoned.
    pub abandonment_grace_ticks: NonZeroU32,
}
```

### 3. Register in component schema

Add both components to `with_component_schema_entries!()` in `crates/worldwake-core/src/component_schema.rs`, restricted to `EntityKind::Place`.

### 4. Register in component tables

Add storage fields to `ComponentTables` in `crates/worldwake-core/src/component_tables.rs` following the existing macro pattern.

### 5. Re-export from lib.rs

Add `pub mod bandit_camp;` and re-export types from `crates/worldwake-core/src/lib.rs`.

## Files to Touch

- `crates/worldwake-core/src/bandit_camp.rs` (new — component definitions)
- `crates/worldwake-core/src/component_schema.rs` (modify — register both components)
- `crates/worldwake-core/src/component_tables.rs` (modify — add storage fields)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Raid action definition or handler (E18BANDYN-003)
- EstablishCamp action definition or handler (E18BANDYN-004)
- `bandit_camp_system()` abandonment logic (E18BANDYN-005)
- AI candidate generation for bandit goals (E18BANDYN-006)
- GoalKind variants (E18BANDYN-002)
- Route threat estimation (E18BANDYN-008)
- Golden test T22 (E18BANDYN-009)
- Any changes to `Topology` or `build_prototype_world` — prototype world already has `BanditCamp` place

## Acceptance Criteria

### Tests That Must Pass

1. Insert `BanditCamp` on a Place entity, retrieve it, verify fields match
2. Insert `BanditCampProfile` on a Place entity, retrieve it, verify all fields
3. Attempting to insert `BanditCamp` on a non-Place entity fails appropriately
4. Remove `BanditCamp` from a Place entity succeeds
5. Existing suite: `cargo test -p worldwake-core`
6. Existing suite: `cargo clippy --workspace`

### Invariants

1. `BanditCamp` is only attachable to `EntityKind::Place` entities
2. `BanditCampProfile` is only attachable to `EntityKind::Place` entities
3. No `f32`/`f64` anywhere — `Permille` for fractional values, `NonZeroU32` for tick counts
4. No magic numbers — all behavioral thresholds are fields on `BanditCampProfile`
5. Existing conservation invariants unaffected (`verify_live_lot_conservation`, `verify_authoritative_conservation`)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/bandit_camp.rs` (or test module) — unit tests for component CRUD on Place entities
2. `crates/worldwake-core/src/component_schema.rs` tests — verify kind restrictions

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo build --workspace`
