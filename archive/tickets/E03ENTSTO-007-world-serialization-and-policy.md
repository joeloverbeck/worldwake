# E03ENTSTO-007: World Serialization and Policy Coverage

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E03ENTSTO-006 (World struct fully assembled with factories)

## Reassessed Baseline

The original ticket was written against an older snapshot of E03.

Current code already provides:
1. `EntityAllocator: Serialize + Deserialize`.
2. `ComponentTables: Serialize + Deserialize`.
3. Component/value serialization across the current authoritative E03 types.
4. A repository policy integration test in `crates/worldwake-core/tests/policy.rs` that already enforces the no-`Player`, no-`HashMap`, no-`HashSet`, no-`TypeId`, and no-`Box<dyn Any>` rules.

Current architecture also matters:
1. `World::new(topology)` returns `Result<World, WorldError>`, not `World`.
2. Topology-owned `Place` entities are registered into the allocator during world construction; they are not created through a `World::create_place(...)` helper.
3. E03 factory coverage currently centers on `create_agent`, `create_office`, and `create_faction`.

## Problem

E03 still requires the authoritative `World` boundary itself to round-trip through serialization and remain operational after restore. That is the remaining architectural gap in this ticket.

The policy test also needs to reflect the intent more explicitly:
1. keep using the existing integration test rather than adding duplicate scanners;
2. cover `dyn Any` directly, not just `Box<dyn Any>`;
3. preserve the deterministic-data policy without weakening the current checks.

## Scope

This ticket should:
1. make `World` serializable/deserializable;
2. add focused `World` round-trip tests against the real E03 architecture;
3. harden the existing policy integration test only where it is missing explicit E03 coverage.

This ticket should not:
1. introduce a generic ECS abstraction;
2. change topology/place ownership semantics;
3. widen `World` mutability or expose component tables publicly.

## What to Change

### 1. Serialize the `World` boundary

- `World` should derive or implement `Serialize + Deserialize`.
- Keep the existing private-field, typed-table architecture intact.
- Do not add aliasing layers or compatibility shims.

### 2. Add `World` round-trip tests

Add tests around the actual public surface:
1. empty world round-trip using `World::new(Topology::new())?`;
2. populated world round-trip using topology-owned places plus factory-created agent/office/faction entities;
3. post-deserialization behavioral checks proving queries and CRUD still work on the restored world.

The populated test should verify at least:
1. topology-owned place entities remain registered and alive;
2. factory-created entities keep their kinds, metadata, and components;
3. entity ordering and query results remain deterministic after restore.

### 3. Extend the existing policy integration test

Modify `crates/worldwake-core/tests/policy.rs` instead of creating a second scanner.

Ensure authoritative source scanning explicitly rejects:
- `TypeId`
- `dyn Any`
- `Box<dyn Any>`
- `HashMap`
- `HashSet`

Comment lines and dedicated test infrastructure can remain excluded as they are today.

## Files to Touch

- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/tests/policy.rs`

`crates/worldwake-core/src/allocator.rs` and `crates/worldwake-core/src/component_tables.rs` are already in the desired state unless test-driven work proves otherwise.

## Architectural Rationale

The proposed changes are beneficial because they complete the current explicit-typed architecture rather than competing with it:
1. serializing `World` directly makes save/load and later event-log integration compose naturally with the existing typed tables;
2. round-trip tests protect the narrow mutation/query surface that E06 will journal later;
3. strengthening the existing policy test is preferable to introducing new parallel policy infrastructure.

No broader architectural rewrite is warranted here. The current `World + EntityAllocator + ComponentTables + Topology` split is clean, explicit, and extensible for Phase 1.

## Out of Scope

- Stable hashing of world state beyond existing deterministic structures.
- Event-log snapshotting or replay integration.
- Save/load API design outside plain serde/bincode round-trips.
- Refactoring topology ownership of places.
- Broad policy-test parser improvements unless required by the added tests.

## Acceptance Criteria

### Tests That Must Pass

1. `World` round-trips through bincode when empty.
2. `World` round-trips through bincode when populated with topology-owned places and factory-created entities.
3. A deserialized world still supports typed queries and controlled CRUD correctly.
4. The existing policy integration suite explicitly rejects `TypeId`, `dyn Any`, `Box<dyn Any>`, `HashMap`, and `HashSet` in authoritative source.
5. `cargo test -p worldwake-core` passes.
6. `cargo clippy --workspace` and `cargo test --workspace` pass.

### Invariants

1. Authoritative world state remains deterministic and serializable.
2. No runtime-erased `Any` storage exists in authoritative world code.
3. Topology-owned places remain topology-owned after serialization round-trip.
4. `World` retains its private-field, narrow-mutation architecture.

## Test Plan

### New or Modified Tests

In `crates/worldwake-core/src/world.rs`:
- `world_bincode_roundtrip_empty`
- `world_bincode_roundtrip_populated`
- `deserialized_world_remains_operational`

In `crates/worldwake-core/tests/policy.rs`:
- extend the existing policy checks to cover `dyn Any` explicitly

### Commands

```bash
cargo test -p worldwake-core world_bincode_roundtrip
cargo test -p worldwake-core deserialized_world_remains_operational
cargo test -p worldwake-core --test policy
cargo clippy --workspace
cargo test --workspace
```

## Outcome

Implemented:
1. `World` now derives `Serialize + Deserialize`, completing the serialization boundary implied by E03's typed authoritative-state architecture.
2. Added focused `World` round-trip coverage for empty and populated worlds, including a post-deserialization behavior test proving the restored world still supports typed queries and controlled mutation.
3. Extended the existing policy integration suite with an explicit `dyn Any` check instead of introducing a second scanner.

Changed from the original ticket plan:
1. No changes were needed in `EntityAllocator` or `ComponentTables`; both were already in the correct serialized form.
2. The populated-world test was aligned to the real architecture: topology-owned places plus factory-created agent, office, and faction entities. No `create_place(...)` helper was added because that would cut across the intended topology ownership model.
