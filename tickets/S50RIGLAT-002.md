# S50RIGLAT-002: Migrate OfficeData.jurisdiction to BTreeSet

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — OfficeData field type change, SAVE_FORMAT_VERSION bump
**Deps**: S50RIGLAT-001

## Problem

`OfficeData.jurisdiction` is a single `EntityId`, but FOUNDATIONS P23 says "a jurisdiction can stop at the town gate" — implying multi-place jurisdiction is the correct model. All existing callers use equality checks (`== place`) that need to become containment checks (`.contains(&place)`). This ticket migrates the field type and all its consumers.

## Assumption Reassessment (2026-04-05)

1. `OfficeData.jurisdiction: EntityId` at `crates/worldwake-core/src/offices.rs:9`. Verified this session.
2. `offices_with_jurisdiction(place, world)` at `crates/worldwake-systems/src/offices.rs:121` uses `office_data.jurisdiction == place`. Verified.
3. `office_actions.rs` has 4 jurisdiction equality checks: lines 445, 479, 992, 994-995. Verified via grep.
4. `world_txn.rs` constructs `OfficeData` with single `EntityId` jurisdiction at lines 1757, 1797, 2045, 2193. Verified.
5. `SAVE_FORMAT_VERSION` is currently 19 at `crates/worldwake-sim/src/save_load.rs:6`. Bump to 20.
6. `OfficeData` derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. `BTreeSet<EntityId>` satisfies all these bounds.
7. Scenario RON files and test fixtures create offices with single-place jurisdiction. These wrap in `BTreeSet::from([entity])`.
8. No other specs in `specs/` reference `OfficeData.jurisdiction`. No cross-spec impact.

## Architecture Check

1. Direct field migration per P28 — no shim, no compatibility layer. The old single-place representation is replaced everywhere.
2. `BTreeSet<EntityId>` is deterministic (BTree ordering) per project invariants. No `HashSet`.
3. The migration is purely structural — no behavioral change when offices have a single jurisdiction place. Multi-place jurisdiction becomes available for future scenario definitions without further code changes.

## Verification Layers

1. `offices_with_jurisdiction()` returns correct offices for multi-place jurisdiction → focused unit test
2. Office action locality checks reject actors outside jurisdiction → existing golden political tests (must still pass)
3. Save/load round-trip preserves multi-place jurisdiction → save/load test
4. All existing golden tests pass unchanged (single-place offices wrapped in BTreeSet behave identically)

## What to Change

### 1. Migrate OfficeData.jurisdiction field

In `crates/worldwake-core/src/offices.rs`:
```rust
// Before
pub jurisdiction: EntityId,
// After
pub jurisdiction: BTreeSet<EntityId>,
```

Update `Default` impl if one exists. Update any `OfficeData` constructors.

### 2. Update offices_with_jurisdiction()

In `crates/worldwake-systems/src/offices.rs`:
```rust
// Before
(office_data.jurisdiction == place).then_some(office)
// After
office_data.jurisdiction.contains(&place).then_some(office)
```

### 3. Update office_actions.rs locality checks

In `crates/worldwake-systems/src/office_actions.rs`, update 4 sites:
- `office_data.jurisdiction != actor_place` → `!office_data.jurisdiction.contains(&actor_place)`
- `office_data.jurisdiction == actor_place` → `office_data.jurisdiction.contains(&actor_place)`
- `world.effective_place(actor) != Some(office_data.jurisdiction)` → `!world.effective_place(actor).map_or(false, |p| office_data.jurisdiction.contains(&p))`

### 4. Update world_txn.rs office creation

In `crates/worldwake-core/src/world_txn.rs`, update all `OfficeData` constructions to wrap jurisdiction in `BTreeSet::from([place])`.

### 5. Update scenario/test fixtures

Grep for all `jurisdiction: entity(` patterns in test code and scenario files. Wrap each in `BTreeSet::from([entity(N)])`.

### 6. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs`:
```rust
pub const SAVE_FORMAT_VERSION: u32 = 20;
```

### 7. Update effective_rights() JurisdictionalAuthority check

In `crates/worldwake-core/src/world/ownership.rs`, the `effective_rights()` function (from ticket 001) can now implement `JurisdictionalAuthority` fully: check if actor holds any office whose `jurisdiction.contains(entity_place)`.

## Files to Touch

- `crates/worldwake-core/src/offices.rs` (modify — field type change)
- `crates/worldwake-core/src/world_txn.rs` (modify — office construction sites)
- `crates/worldwake-core/src/world/ownership.rs` (modify — JurisdictionalAuthority in effective_rights)
- `crates/worldwake-core/src/delta.rs` (modify — if OfficeData appears in delta test fixtures)
- `crates/worldwake-core/src/component_tables.rs` (modify — if OfficeData test fixtures exist)
- `crates/worldwake-systems/src/offices.rs` (modify — offices_with_jurisdiction)
- `crates/worldwake-systems/src/office_actions.rs` (modify — 4 locality checks)
- `crates/worldwake-sim/src/save_load.rs` (modify — SAVE_FORMAT_VERSION bump)
- Test/scenario files with OfficeData fixtures (modify — wrap in BTreeSet)

## Out of Scope

- Adding belief-facing rights queries (ticket 003)
- Justice candidate generation changes (ticket 004)
- Changing `can_exercise_control()` signature
- Multi-place jurisdiction scenario content (this ticket enables it; scenarios are future work)

## Acceptance Criteria

### Tests That Must Pass

1. `offices_with_jurisdiction()` returns office when place is in jurisdiction set
2. `offices_with_jurisdiction()` returns empty when place is NOT in jurisdiction set
3. Office actions reject actors outside jurisdiction (existing golden behavior preserved)
4. Save/load round-trip preserves `BTreeSet<EntityId>` jurisdiction
5. All existing golden tests: `cargo test -p worldwake-ai`
6. Existing suite: `cargo test --workspace`

### Invariants

1. `OfficeData.jurisdiction` is a `BTreeSet<EntityId>` — deterministic ordering, no HashSet
2. Every `offices_with_jurisdiction(place, world)` call uses `.contains()`, never `==`
3. `SAVE_FORMAT_VERSION` is 20 after this ticket

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs` (test module) — test `offices_with_jurisdiction` with multi-place jurisdiction
2. `crates/worldwake-core/src/world/ownership.rs` (test module) — test `JurisdictionalAuthority` in `effective_rights()` with multi-place office

### Commands

1. `cargo test -p worldwake-core -- jurisdiction`
2. `cargo test -p worldwake-systems -- jurisdiction`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`
