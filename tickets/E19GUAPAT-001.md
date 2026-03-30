# E19GUAPAT-001: Add PatrolRoute and PatrolProfile components to worldwake-core

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component types, component schema registration
**Deps**: E16 (offices), E17 (violation memory) — both delivered

## Problem

Guards need per-agent patrol configuration (route waypoints, vigilance, sensitivity) and persistent patrol progress. No patrol domain types exist yet in the codebase.

## Assumption Reassessment (2026-03-30)

1. `crates/worldwake-core/src/patrol.rs` does not exist — confirmed by glob search.
2. Component registration follows the `with_component_schema_entries!` macro in `component_schema.rs` (lines 3–31). Each entry requires 15+ accessor names plus a kind predicate.
3. `ComponentTables` struct in `component_tables.rs` uses `define_component_tables_struct!` and `define_component_table_impls!` macros with `BTreeMap<EntityId, T>` storage.
4. `Permille` type exists in `crates/worldwake-core/src/numerics.rs` — used throughout for [0,1000] range values.
5. `EntityKind::Agent` is the kind predicate for agent-specific components (e.g., `AgentData`, `CombatProfile`, `UtilityProfile`).
6. `ViolationMemory` exists in `crates/worldwake-core/src/violation.rs` (line 60) — E19 will read from it, not modify it.
7. No adjacent contradictions found.

## Architecture Check

1. Two separate components (`PatrolRoute` for mutable route state, `PatrolProfile` for per-agent configuration parameters) follow the existing pattern of separating mutable state from configuration (cf. `HomeostaticNeeds` vs `ThresholdBand`, `CombatProfile` as config). This is cleaner than a single monolithic struct because route state changes every patrol cycle while profile rarely changes.
2. No backwards-compatibility shims introduced. New types only.

## Verification Layers

1. Component registration correctness → focused unit test: insert/get/remove round-trip on `World`
2. Kind predicate enforcement → focused unit test: inserting on non-Agent entity returns error
3. Serialization round-trip → focused unit test: serde serialize/deserialize preserves all fields
4. Single-layer ticket (core domain types only) — no cross-layer mapping needed.

## What to Change

### 1. New file: `crates/worldwake-core/src/patrol.rs`

Define two structs:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatrolRoute {
    pub assigned_places: Vec<EntityId>,
    pub current_index: usize,
}
impl Component for PatrolRoute {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatrolProfile {
    pub base_patrol_interval: u32,
    pub vigilance: Permille,
    pub route_adaptation_sensitivity: Permille,
    pub patrol_motive_weight: Permille,
}
impl Component for PatrolProfile {}
```

### 2. Register components in `component_schema.rs`

Add two new entries to `with_component_schema_entries!` following the existing pattern. Kind predicate: `|kind| kind == EntityKind::Agent`.

### 3. Add storage in `component_tables.rs`

Add `patrol_routes: BTreeMap<EntityId, PatrolRoute>` and `patrol_profiles: BTreeMap<EntityId, PatrolProfile>` fields via the macro system.

### 4. Declare module and re-exports in `lib.rs`

Add `pub mod patrol;` and re-export `PatrolRoute`, `PatrolProfile`.

## Files to Touch

- `crates/worldwake-core/src/patrol.rs` (new)
- `crates/worldwake-core/src/component_schema.rs` (modify — add 2 entries)
- `crates/worldwake-core/src/component_tables.rs` (modify — add 2 fields)
- `crates/worldwake-core/src/lib.rs` (modify — add module declaration and re-exports)

## Out of Scope

- Patrol action definition or handler (E19GUAPAT-003)
- GoalKind::Patrol or PlannerOpKind::Patrol (E19GUAPAT-002)
- Route adaptation logic (E19GUAPAT-006)
- Any AI candidate generation
- Any changes to worldwake-systems, worldwake-sim, or worldwake-ai crates
- `WorldTxn` integration for patrol mutations (deferred to E19GUAPAT-003 where it's needed)

## Acceptance Criteria

### Tests That Must Pass

1. `PatrolRoute` insert/get/remove round-trip on a `World` with `EntityKind::Agent`
2. `PatrolProfile` insert/get/remove round-trip on a `World` with `EntityKind::Agent`
3. Insert on non-Agent entity is rejected by kind predicate
4. `PatrolRoute` serde round-trip preserves `assigned_places` and `current_index`
5. `PatrolProfile` serde round-trip preserves all four fields
6. Existing suite: `cargo test -p worldwake-core`
7. `cargo clippy --workspace`

### Invariants

1. Components use `BTreeMap` storage (determinism invariant — no `HashMap`)
2. `Permille` used for all [0,1000] range values (spec drafting rule)
3. No `f32`/`f64` in any field (determinism invariant)
4. `current_index` is authoritative stored state, not derived

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/patrol.rs` — unit tests for component round-trips and serde
2. Integration with existing `component_schema` test infrastructure if applicable

### Commands

1. `cargo test -p worldwake-core -- patrol`
2. `cargo clippy --workspace && cargo test --workspace`
