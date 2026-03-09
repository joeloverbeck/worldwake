# E03ENTSTO-007: World Serialization and No-Any Policy Enforcement

**Status**: TODO
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E03ENTSTO-006 (World struct fully assembled with factories)

## Problem

The spec requires that `World` serializes and deserializes correctly (for save/load and replay), and that no `TypeId`, `Any`, or trait-object component storage exists in authoritative world code.

This ticket adds:
1. `Serialize`/`Deserialize` derives on `World` and all internal structs.
2. A bincode round-trip test for a populated World.
3. A policy test that scans source files for forbidden patterns in non-test code.

## What to Change

### 1. Ensure Serialize/Deserialize on World and its parts

- `World` — derive or implement `Serialize + Deserialize`.
- `EntityAllocator` — derive `Serialize + Deserialize`.
- `ComponentTables` — already derives (from E03ENTSTO-003).
- All component types — already derive (from E03ENTSTO-003).

### 2. Add World serialization round-trip test

Create a test that:
1. Builds a `World` with a small topology.
2. Creates several entities (agent, place, office) via factory helpers.
3. Serializes to bincode bytes.
4. Deserializes back.
5. Verifies all entities, components, and metadata match.

### 3. Add policy enforcement test

Add a test (in `crates/worldwake-core/tests/` or inline) that scans all `.rs` files under `crates/worldwake-core/src/` for forbidden patterns in non-test, non-comment lines:
- `TypeId`
- `Box<dyn Any>`
- `dyn Any`
- `HashMap<` or `HashSet<` (in non-test code)

Note: There may already be a policy test in `crates/worldwake-core/tests/policy.rs` — extend it rather than duplicating.

## Files to Touch

- `crates/worldwake-core/src/world.rs` (modify — add Serialize/Deserialize if not already present)
- `crates/worldwake-core/src/allocator.rs` (modify — add Serialize/Deserialize if not already present)
- `crates/worldwake-core/tests/policy.rs` (modify — extend with E03-relevant checks, if this file exists)

## Out of Scope

- Stable hashing of World state — that's E08 (replay determinism).
- Save/load with event log — E06/E08.
- Snapshot diffing — E06.
- Performance optimization of serialization.

## Acceptance Criteria

### Tests That Must Pass

1. **World bincode round-trip**: a populated World (with agents, places, offices) serializes and deserializes with all data intact.
2. **Empty World round-trip**: `World::new(topology)` with no entities round-trips correctly.
3. **Policy: no TypeId/Any**: scanning `worldwake-core/src/` non-test code finds zero instances of `TypeId`, `dyn Any`, `Box<dyn Any>`.
4. **Policy: no HashMap/HashSet**: scanning `worldwake-core/src/` non-test code finds zero instances of `HashMap<` or `HashSet<` in authoritative state (comment/test exclusions allowed).
5. **Deserialized World is functional**: after deserializing, queries and CRUD operations work correctly on the restored World.
6. **Existing suite**: `cargo test -p worldwake-core` continues green.

### Invariants

1. Authoritative state is deterministic and serializable.
2. No runtime-erased `Any` store exists in authoritative world code.
3. No `HashMap`/`HashSet` in authoritative state (deterministic data policy).

## Test Plan

### New Tests

In `crates/worldwake-core/src/world.rs`:
- `world_bincode_roundtrip_populated`
- `world_bincode_roundtrip_empty`
- `deserialized_world_queries_work`

In `crates/worldwake-core/tests/policy.rs` (or new integration test):
- `no_typeid_any_in_authoritative_code` (extend existing if present)
- `no_hashmap_hashset_in_authoritative_code` (extend existing if present)

### Commands

```bash
cargo test -p worldwake-core world
cargo test -p worldwake-core --test policy
cargo clippy --workspace && cargo test --workspace
```
