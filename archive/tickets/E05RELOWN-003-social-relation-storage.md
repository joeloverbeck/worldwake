# E05RELOWN-003: Social relation storage in RelationTables

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — extends `RelationTables` with social relation maps
**Deps**: Archived ticket `E05RELOWN-002` is already implemented; no remaining code dependency beyond current `main`

## Problem

The spec requires deterministic storage for six social/knowledge relations: `MemberOf`, `LoyalTo`, `HoldsOffice`, `HostileTo`, `KnowsFact`, `BelievesFact`. These are needed now for storage and API surface even though propagation behavior lands in later epics.

## Assumption Reassessment (2026-03-09)

1. `RelationTables` already exists in `crates/worldwake-core/src/relations.rs` with physical and reservation storage plus `remove_all` cleanup used by `World::purge_entity` — confirmed
2. `FactId` already exists in `ids.rs` and already satisfies deterministic ordering + serde requirements — confirmed
3. `EntityKind::Faction` and `EntityKind::Office` exist in `entity.rs` — confirmed
4. `HoldsOffice` must enforce at most one holder per office (spec 9.13) — storage-level uniqueness via `BTreeMap<EntityId, EntityId>` (office → holder) is the right representation, but reverse-index cleanup for purged holders/offices must land in the same change
5. `World::purge_entity` already delegates relation teardown to `RelationTables::remove_all` — confirmed; if this ticket adds social storage without extending cleanup, purging entities would leave stale authoritative relation rows
6. Existing tests already cover relation-table defaults, bincode round-trip, and purge cleanup for physical relations — confirmed; this ticket should extend those tests rather than invent a new test harness

## Architecture Check

1. Social relations are additional fields in `RelationTables`, not a separate struct
2. `HoldsOffice` uses `BTreeMap<EntityId, EntityId>` (office → holder) + reverse index (holder → offices) — the forward map naturally enforces one-holder-per-office while the reverse index keeps later queries efficient
3. Many-to-many entity-to-entity relations use `BTreeMap<EntityId, BTreeSet<EntityId>>` for both directions
4. Knowledge/belief relations use `BTreeMap<EntityId, BTreeSet<FactId>>` keyed only by agent for now; adding speculative fact → agents indices before any query or invariant requires them would add maintenance cost without architectural benefit
5. This ticket is not just passive storage. Because the relation layer is authoritative and already participates in purge, social storage must integrate with teardown immediately to keep the world internally consistent

## Proposed Architecture Rationale

This change is more beneficial than the current architecture.

1. The current architecture has no authoritative storage for social, office, or fact relations, so later APIs in `E05RELOWN-008` would either need to bolt on ad hoc maps or widen their scope to introduce storage and behavior at once
2. Extending `RelationTables` preserves the existing design direction from `E05RELOWN-002`: explicit typed ordered tables, no untyped relation bag, no compatibility aliases
3. Keeping fact storage one-way for now is the cleaner architecture than preemptively mirroring it. Reverse fact indices should appear only when a real query or invariant needs them
4. The only meaningful architectural correction to the original ticket is scope: storage without purge cleanup is not robust enough for authoritative state, so cleanup belongs here rather than in a later bug-fix pass

## What to Change

### 1. Add social relation fields to `RelationTables` in `relations.rs`

```rust
// Faction membership (many-to-many)
pub(crate) member_of: BTreeMap<EntityId, BTreeSet<EntityId>>,    // member → factions
pub(crate) members_of: BTreeMap<EntityId, BTreeSet<EntityId>>,   // faction → members

// Loyalty (many-to-many)
pub(crate) loyal_to: BTreeMap<EntityId, BTreeSet<EntityId>>,     // subject → targets
pub(crate) loyalty_from: BTreeMap<EntityId, BTreeSet<EntityId>>, // target → subjects

// Office holding (one holder per office)
pub(crate) office_holder: BTreeMap<EntityId, EntityId>,          // office → holder
pub(crate) offices_held: BTreeMap<EntityId, BTreeSet<EntityId>>, // holder → offices

// Hostility (many-to-many)
pub(crate) hostile_to: BTreeMap<EntityId, BTreeSet<EntityId>>,   // subject → targets
pub(crate) hostility_from: BTreeMap<EntityId, BTreeSet<EntityId>>, // target → subjects

// Knowledge (agent → facts)
pub(crate) knows_fact: BTreeMap<EntityId, BTreeSet<FactId>>,
pub(crate) believes_fact: BTreeMap<EntityId, BTreeSet<FactId>>,
```

### 2. Update `Default` impl

Extend the `Default` implementation to initialize all new maps as empty.

### 3. Extend relation teardown

Update `RelationTables::remove_all` so purging an entity removes:

- entity-as-source and entity-as-target rows for social many-to-many relations
- entity-as-office and entity-as-holder rows for office holding
- entity-scoped knowledge/belief rows

This is required because `World::purge_entity` already relies on `RelationTables::remove_all` for authoritative cleanup.

## Files to Touch

- `crates/worldwake-core/src/relations.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify tests only if needed to prove purge cleanup covers social storage)

## Out of Scope

- Social relation mutation APIs (E05RELOWN-008)
- Office assignment/vacate behavior (E05RELOWN-008)
- Knowledge/belief propagation semantics (future epic E14+)
- Physical relation storage (already in E05RELOWN-002)
- Any `World`-level API for social relations

## Acceptance Criteria

### Tests That Must Pass

1. `RelationTables::default()` initializes all social relation maps as empty
2. `RelationTables` with populated social relations round-trips through bincode
3. Social relation maps use `BTreeMap`/`BTreeSet` (no HashMap/HashSet)
4. Purging an entity cannot leave stale social/office/knowledge rows behind
5. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `office_holder` map naturally enforces one holder per office at the storage level
2. All social relation storage is deterministic and serializable
3. No mutation/query behavior is introduced yet, but authoritative teardown remains complete

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/relations.rs` (extend inline `#[cfg(test)]`) — social relation storage construction, bincode round-trip with populated social data
2. `crates/worldwake-core/src/world.rs` (extend inline `#[cfg(test)]`) — purge cleanup covers social relation rows reachable through `World::purge_entity`

### Commands

1. `cargo test -p worldwake-core relations`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- Actual changes:
  - Extended `RelationTables` with deterministic storage for faction membership, loyalty, office holding, hostility, known facts, and believed facts
  - Extended `RelationTables::remove_all` so purge teardown also removes social, office, and fact rows
  - Strengthened relation-storage and world-purge tests to cover the new maps and teardown paths
- Deviations from original plan:
  - The ticket was corrected first because `RelationTables`, `FactId`, and existing relation cleanup tests were already present
  - Scope was widened slightly to include purge cleanup because storage-only integration would have left stale authoritative rows after `World::purge_entity`
  - Knowledge/belief storage remained agent → facts only; no reverse fact index was added because no current query or invariant needs it
- Verification results:
  - `cargo test -p worldwake-core relations`
  - `cargo test -p worldwake-core`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo fmt --check`
