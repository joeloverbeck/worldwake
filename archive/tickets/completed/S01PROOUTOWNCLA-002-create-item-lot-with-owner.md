# S01PROOUTOWNCLA-002: Add WorldTxn::create_item_lot_with_owner() helper

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — WorldTxn API
**Deps**: S01PROOUTOWNCLA-001 (types must exist, though this helper only uses existing relation API)

## Problem

Currently, creating an owned item lot requires three separate calls: `create_item_lot()`, `set_ground_location()`, `set_owner()`. This opens a failure mode where ownership assignment is forgotten. A single atomic helper prevents this gap.

## Assumption Reassessment (2026-03-15)

1. `WorldTxn::create_item_lot()` exists at `world_txn.rs:240-250` — confirmed
2. `WorldTxn::set_ground_location()` exists at `world_txn.rs:278-284` — confirmed
3. `set_owner()` is available on `World` at `ownership.rs:96-110` and via `WorldTxn` — confirmed
4. `RelationDelta::Added { relation_kind: RelationKind::OwnedBy, ... }` is how ownership is recorded in deltas — confirmed
5. `WorldTxn` already provides `set_possessor()` pattern to follow — confirmed

## Architecture Check

1. Wrapping three calls into one atomic helper is the minimal correct change — no new abstractions
2. The helper does NOT auto-possess the lot (preserving custody/ownership separation)
3. `None` owner is a valid argument, producing an unowned lot (same as current behavior)

## What to Change

### 1. Add `create_item_lot_with_owner()` to `WorldTxn`

```rust
pub fn create_item_lot_with_owner(
    &mut self,
    commodity: CommodityKind,
    quantity: Quantity,
    place: EntityId,
    owner: Option<EntityId>,
) -> Result<EntityId, WorldError>
```

Implementation:
1. Call `self.create_item_lot(commodity, quantity)?`
2. Call `self.set_ground_location(lot, place)?`
3. If `owner.is_some()`, call `self.set_owner(lot, owner.unwrap())?`
4. Return `Ok(lot)`

The `set_owner` call within the transaction produces the standard `RelationDelta` in the committed event's delta list.

## Files to Touch

- `crates/worldwake-core/src/world_txn.rs` (modify — add method)

## Out of Scope

- Resolving `ProductionOutputOwnershipPolicy` to an owner (that's the handler's job in S01PROOUTOWNCLA-004, -005)
- Changing existing `create_item_lot()` callers (S01PROOUTOWNCLA-004, -005)
- Any validation of the owner entity (that's `set_owner()`'s responsibility)

## Acceptance Criteria

### Tests That Must Pass

1. `create_item_lot_with_owner()` with `Some(owner)` creates lot + sets ground location + sets owner atomically
2. `create_item_lot_with_owner()` with `None` owner creates unowned lot at the specified place
3. Ownership assignment produces `RelationDelta` with `OwnedBy` in committed event deltas
4. Lot is unpossessed after creation (custody separate from ownership)
5. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. The lot exists at the specified place after creation
2. The lot is unpossessed (no auto-possession)
3. If owner is provided, `owner_of(lot) == Some(owner)`
4. If owner is None, `owner_of(lot) == None`
5. Event log traceability: ownership assignment is recorded in deltas

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world_txn.rs` test module — tests for both `Some` and `None` owner paths, delta verification

### Commands

1. `cargo test -p worldwake-core create_item_lot_with_owner`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-15
- **What changed**: Added `WorldTxn::create_item_lot_with_owner()` at `world_txn.rs:553-571`. Atomic helper wrapping `create_item_lot()` + `set_ground_location()` + optional `set_owner()`. Five tests added covering both `Some`/`None` owner paths, delta traceability, and custody/ownership separation.
- **Deviations**: None. Implementation matches ticket spec exactly.
- **Verification**: `cargo test -p worldwake-core` (662 tests pass), `cargo clippy --workspace` clean.
