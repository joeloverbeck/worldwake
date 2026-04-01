# S05MERSTOSTALL-002: Add stock container facility creation helpers

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new entity creation helpers in worldwake-core
**Deps**: S05MERSTOSTALL-001

## Problem

Tests and scenario setup need a way to create merchant facilities with stock and display containers. Currently there are no helpers for creating `EntityKind::Facility` entities with associated `EntityKind::Container` children and `StockStoragePolicy`.

## Assumption Reassessment (2026-04-01)

1. `StockStoragePolicy` exists in `trade.rs` and is registered on `EntityKind::Facility` — confirmed via S05MERSTOSTALL-001 outcome.
2. `EntityKind::Facility` and `EntityKind::Container` variants exist in `entity.rs` — confirmed.
3. `CarryCapacity` exists at `production.rs:67` — check whether it works for containers or if a separate `ContainerCapacity` is needed.
4. Entity creation helpers follow patterns in `world.rs` or `world/` module — confirmed existing helpers for other entity kinds.
5. No facility creation helper currently exists — confirmed via grep.

## Architecture Check

1. Follows the existing entity creation helper pattern — single function creates Facility + Container children + attaches StockStoragePolicy. No new abstractions or traits needed.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Facility creation produces valid `StockStoragePolicy` → authoritative world state (focused unit test)
2. Containers created at correct place → authoritative world state (focused unit test)
3. Containment relationship between facility and containers works → focused unit test
4. Single-layer ticket — additional layer mapping not applicable beyond authoritative state verification.

## What to Change

### 1. Add facility creation helper

In `crates/worldwake-core/src/world.rs` (or `world/` module), add a helper that:
- Creates a `EntityKind::Facility` entity at a given place
- Creates `EntityKind::Container` entities for stock and (optionally) display
- Attaches `StockStoragePolicy` to the facility referencing the containers
- Returns the facility ID and container IDs

### 2. Verify CarryCapacity applicability

Check whether `CarryCapacity` (production.rs:67) works for containers. If not, determine whether `ContainerCapacity` is needed or if containers are unbounded by default.

### 3. Re-export and update lib.rs if needed

Ensure the helper is accessible from `worldwake-core` public API. Update `component_schema.rs` and `lib.rs` if any new types are introduced.

## Files to Touch

- `crates/worldwake-core/src/world.rs` or `crates/worldwake-core/src/world/` (modify)
- `crates/worldwake-core/src/component_schema.rs` (modify — if ContainerCapacity needed)
- `crates/worldwake-core/src/lib.rs` (modify — re-export helper)

## Out of Scope

- Action handlers for store/collect/stage/unstage (003/004)
- Belief view queries for facility stock (005)
- AI planning changes (007)
- MoveCargo evolution (006)

## Acceptance Criteria

### Tests That Must Pass

1. Facility creation produces a valid `StockStoragePolicy` with correct container references
2. Stock and display containers are placed at the same location as the facility
3. Containment relationships work correctly between facility and containers
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Every entity exists in exactly one place — containers share the facility's place
2. `StockStoragePolicy` always references valid container entities

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` (or `world/` test module) — facility creation helper produces valid policy, containers at correct place
2. `crates/worldwake-core/src/world.rs` — containment relationship between facility and containers

### Commands

1. `cargo test -p worldwake-core -- facility`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
