# S38LRNPREF-001: Core types, Component impls, and component schema registration

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new ECS components in worldwake-core, SAVE_FORMAT_VERSION bump in worldwake-sim
**Deps**: S38 spec (all dependencies satisfied: E14 ✅, S35 ✅, S33 ✅)

## Problem

No infrastructure exists for per-agent experience memories. The three new components (`RouteExperience`, `SourceReliability`, `PreferenceProfile`) and their supporting types must be defined and registered in the ECS before any other S38 ticket can proceed.

## Assumption Reassessment (2026-04-02)

1. `Component` trait requires `'static + Send + Sync + Clone + Debug + Serialize + DeserializeOwned` — verified at `crates/worldwake-core/src/traits.rs:15`.
2. `component_schema.rs` uses `with_component_schema_entries!` macro — verified at `crates/worldwake-core/src/component_schema.rs:3`. Bare-type macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`) must import the new types. `world_txn.rs` uses the crate-qualified `select_txn_simple_set_components` selector, so it will pick up new simple-setters without a matching top-level import unless the file starts using the types directly.
3. `TravelEdgeId` exists at `crates/worldwake-core/src/ids.rs:149` as `struct TravelEdgeId(u32)` with `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize`.
4. `EntityId` exists at `crates/worldwake-core/src/ids.rs:39` as `struct EntityId { slot: u32, generation: u32 }` with same derives.
5. `CommodityKind` exists at `crates/worldwake-core/src/items.rs:9` as enum with 10 variants including `Ord` derive.
6. `Tick` exists at `crates/worldwake-core/src/ids.rs:55` as `struct Tick(pub u64)` with `Copy, Clone, Eq, PartialEq, Ord, PartialOrd`.
7. `Permille` exists at `crates/worldwake-core/src/numerics.rs:20` as `struct Permille(u16)`.
8. `PerceptionProfile` at `crates/worldwake-core/src/belief.rs:1294` uses `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize` and `memory_capacity: u32`, `memory_retention_ticks: u64` — our `PreferenceProfile` follows this pattern.
9. `SAVE_FORMAT_VERSION` is currently 12 at `crates/worldwake-sim/src/save_load.rs:6`.
10. No existing `RouteExperience`, `SourceReliability`, or `PreferenceProfile` types in the codebase.

## Architecture Check

1. Three new components follow the established PerceptionProfile pattern: per-agent, agent-only, Copy-able profile + Default-able stores. Struct shapes and derives match existing conventions.
2. Component-registration fallout includes the hardcoded authoritative component manifest in `delta.rs` (`ComponentKind::ALL`, `component_samples()`, and related tests), not just schema macro expansion sites.
3. No backward-compatibility shims. These are net-new types with no legacy predecessors.

## Verification Layers

1. Component trait bounds satisfied → compile-time verification (derive macros + `impl Component`)
2. Component schema registration and manifest mirrors correct → focused unit tests: construct world, insert/get/remove each component; `delta.rs` authoritative component inventory still matches the live schema
3. SAVE_FORMAT_VERSION bump → existing save/load round-trip tests catch version mismatch
4. Single-layer ticket (worldwake-core types + schema); no cross-system verification needed.

## What to Change

### 1. New types module in worldwake-core

Create `crates/worldwake-core/src/experience.rs` containing:

- `EdgeExperience` struct: `safe_trips: u16`, `hostile_encounters: u16`, `last_travel_tick: Tick`. Derives: `Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize`.
- `RouteExperience` struct: `edges: BTreeMap<TravelEdgeId, EdgeExperience>`. Derives: `Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize`. Impl `Component`.
- `SourceKey` struct: `entity: EntityId`, `commodity: CommodityKind`. Derives: `Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize`.
- `ReliabilityRecord` struct: `successful_acquisitions: u16`, `failed_attempts: u16`, `last_attempt_tick: Tick`. Derives: `Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize`.
- `SourceReliability` struct: `sources: BTreeMap<SourceKey, ReliabilityRecord>`. Derives: `Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize`. Impl `Component`.
- `PreferenceProfile` struct: `route_caution_weight: Permille`, `source_trust_weight: Permille`, `route_memory_capacity: u32`, `source_memory_capacity: u32`, `memory_retention_ticks: u64`. Derives: `Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize`. Impl `Component`.

### 2. Component schema registration

Add entries for `RouteExperience`, `SourceReliability`, and `PreferenceProfile` to `with_component_schema_entries!` in `component_schema.rs`, gated to `EntityKind::Agent`.

### 3. Macro expansion site imports and component-manifest updates

Add `use crate::{RouteExperience, SourceReliability, PreferenceProfile};` (or bare type names if already in scope via `crate::*`) at the bare-type macro expansion sites: `delta.rs`, `world.rs`, `component_tables.rs`.

Update `delta.rs` to keep the authoritative component manifest honest:
- add the new `ComponentKind` variants to the explicit `ComponentKind::ALL` expectation test
- add representative `ComponentValue` samples for the new components so `component_value_reports_matching_component_kind` still proves the full schema mirror surface

### 4. Module declaration and re-exports

Add `pub mod experience;` to `crates/worldwake-core/src/lib.rs` and re-export the public types.

### 5. SAVE_FORMAT_VERSION bump

Bump `SAVE_FORMAT_VERSION` from 12 to 13 in `crates/worldwake-sim/src/save_load.rs`.

## Files to Touch

- `crates/worldwake-core/src/experience.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — module declaration + re-exports)
- `crates/worldwake-core/src/component_schema.rs` (modify — 3 new entries)
- `crates/worldwake-core/src/delta.rs` (modify — imports + component manifest/tests)
- `crates/worldwake-core/src/world.rs` (modify — imports)
- `crates/worldwake-core/src/component_tables.rs` (modify — imports)
- `crates/worldwake-sim/src/save_load.rs` (modify — version bump)

## Out of Scope

- Memory eviction logic (S38LRNPREF-002)
- GoalBeliefView extension (S38LRNPREF-003)
- Action handler modifications (S38LRNPREF-004, 005)
- Ranking adjustments (S38LRNPREF-006, 007)
- Golden tests (S38LRNPREF-008)

## Acceptance Criteria

### Tests That Must Pass

1. Insert `RouteExperience` on an agent entity, retrieve it, verify fields match
2. Insert `SourceReliability` on an agent entity, retrieve it, verify fields match
3. Insert `PreferenceProfile` on an agent entity, retrieve it, verify fields match
4. Component schema rejects insertion on non-Agent entity kinds
5. Save/load round-trip preserves all three components
6. Existing suite: `cargo test --workspace`

### Invariants

1. All three components implement `Component` trait (compile-time enforced)
2. `SourceKey` is `Ord` (required for `BTreeMap` key)
3. `RouteExperience` and `SourceReliability` implement `Default` (agents start with no experience)
4. SAVE_FORMAT_VERSION is exactly 13 after this ticket

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/experience.rs` (new, `#[cfg(test)]` module) — component insert/get/remove round-trips, Default behavior, SourceKey ordering, wrong-kind rejection
2. `crates/worldwake-core/src/delta.rs` (existing tests updated) — authoritative component inventory and representative `ComponentValue` sample coverage
3. `crates/worldwake-sim/src/save_load.rs` (existing tests run) — save/load round-trip with new components

### Commands

1. `cargo test -p worldwake-core experience`
2. `cargo test -p worldwake-sim save_load`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

- **Completed**: 2026-04-02
- **What changed**:
  - added `crates/worldwake-core/src/experience.rs` with `EdgeExperience`, `RouteExperience`, `SourceKey`, `ReliabilityRecord`, `SourceReliability`, and `PreferenceProfile`
  - registered the new agent-only components in `crates/worldwake-core/src/component_schema.rs`
  - re-exported the new types from `crates/worldwake-core/src/lib.rs`
  - updated `crates/worldwake-core/src/delta.rs` so the authoritative component manifest and representative `ComponentValue` samples include the new schema entries
  - updated `crates/worldwake-core/src/component_tables.rs` and `crates/worldwake-core/src/world.rs` for the new bare-type component surfaces
  - added representative fixtures in `crates/worldwake-core/src/test_utils.rs`
  - bumped `SAVE_FORMAT_VERSION` from `12` to `13` in `crates/worldwake-sim/src/save_load.rs`
  - expanded the existing save/load fixture builder so the broad roundtrip tests actually serialize and deserialize the new components
- **Deviations from original plan**:
  - `world_txn.rs` did not require a new top-level import because its simple-setter generation path is crate-qualified through `select_txn_simple_set_components`
  - the schema-registration blast radius included `delta.rs` manifest/test updates and save/load fixture expansion, not just raw macro-site imports
- **Verification**:
  - `cargo test -p worldwake-core experience`
  - `cargo test -p worldwake-core component_kind_variants_match_authoritative_components`
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-sim save_load`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
