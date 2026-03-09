# E05RELOWN-003: Social relation storage in RelationTables

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — extends `RelationTables` with social relation maps
**Deps**: E05RELOWN-002 (RelationTables struct exists)

## Problem

The spec requires deterministic storage for six social/knowledge relations: `MemberOf`, `LoyalTo`, `HoldsOffice`, `HostileTo`, `KnowsFact`, `BelievesFact`. These are needed now for storage and API surface even though propagation behavior lands in later epics.

## Assumption Reassessment (2026-03-09)

1. `RelationTables` exists after E05RELOWN-002 with physical relation maps — assumed
2. `FactId` exists after E05RELOWN-001 — assumed
3. `EntityKind::Faction` and `EntityKind::Office` exist in `entity.rs` — confirmed
4. `HoldsOffice` must enforce at most one holder per office (spec 9.13) — storage-level uniqueness via `BTreeMap<EntityId, EntityId>` (office → holder) naturally enforces this
5. Social relations are many-to-many except `HoldsOffice` (one holder per office) — confirmed from spec

## Architecture Check

1. Social relations are additional fields in `RelationTables`, not a separate struct
2. `HoldsOffice` uses `BTreeMap<EntityId, EntityId>` (office → holder) + reverse index (holder → offices) — the forward map naturally enforces one-holder-per-office
3. Many-to-many relations use `BTreeMap<EntityId, BTreeSet<EntityId>>` for both directions
4. Knowledge/belief relations use `BTreeMap<EntityId, BTreeSet<FactId>>` since facts are identified by `FactId`

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

## Files to Touch

- `crates/worldwake-core/src/relations.rs` (modify)

## Out of Scope

- Social relation mutation APIs (E05RELOWN-008)
- Office uniqueness enforcement logic (E05RELOWN-008)
- Knowledge/belief propagation semantics (future epic E14+)
- Physical relation storage (already in E05RELOWN-002)
- Any `World`-level API for social relations

## Acceptance Criteria

### Tests That Must Pass

1. `RelationTables::default()` initializes all social relation maps as empty
2. `RelationTables` with populated social relations round-trips through bincode
3. Social relation maps use `BTreeMap`/`BTreeSet` (no HashMap/HashSet)
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `office_holder` map naturally enforces one holder per office at the storage level
2. All social relation storage is deterministic and serializable
3. No behavioral logic — storage only

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/relations.rs` (extend inline `#[cfg(test)]`) — social relation storage construction, bincode round-trip with populated social data

### Commands

1. `cargo test -p worldwake-core relations`
2. `cargo clippy --workspace && cargo test --workspace`
