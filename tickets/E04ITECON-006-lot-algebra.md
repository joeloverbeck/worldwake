# E04ITECON-006: Lot algebra — split and merge operations

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None (uses existing World API)
**Deps**: E04ITECON-003 (ItemLot component registered in ECS)

## Problem

Lots must support split and merge with exact conservation and provenance tracking. Without these operations, trade, consumption, and transfer cannot divide or combine commodity stacks — the core economic loop is blocked.

## Assumption Reassessment (2026-03-09)

1. `ItemLot` component is registered in ECS after E04ITECON-003 — dependency
2. `World::create_item_lot` factory exists after E04ITECON-003 — dependency
3. `ProvenanceEntry` and `LotOperation` exist after E04ITECON-002 — dependency
4. `WorldError::InsufficientQuantity` variant already exists in `error.rs` — confirmed

## Architecture Check

1. Operations are methods on `World` (not free functions) so they can create/archive entities atomically
2. Split creates a new lot entity and reduces the source; merge archives one lot and increases the other
3. Zero-quantity lots are archived immediately after split (spec rule: no live zero-quantity lots)

## What to Change

### 1. Add `World::split_lot` method

```rust
pub fn split_lot(
    &mut self,
    lot_id: EntityId,
    amount: u32,
    tick: Tick,
    event_id: Option<EventId>,
) -> Result<(EntityId, EntityId), WorldError>
```

- Validates `lot_id` is alive with an `ItemLot` component
- Validates `amount > 0` and `amount < lot.quantity` (splitting the full amount is just a move, not a split)
- Reduces source lot quantity by `amount`
- Creates a new `ItemLot` entity with `quantity = amount` and `commodity` matching source
- Appends `ProvenanceEntry { operation: Split, source_lot: Some(lot_id), amount, tick, event_id }` to both lots
- Returns `(source_lot_id, new_lot_id)`

### 2. Add `World::merge_lots` method

```rust
pub fn merge_lots(
    &mut self,
    target_id: EntityId,
    source_id: EntityId,
    tick: Tick,
    event_id: Option<EventId>,
) -> Result<EntityId, WorldError>
```

- Validates both lots are alive with `ItemLot` components
- Validates both have the same `CommodityKind` (different commodities cannot merge)
- Adds source quantity to target
- Appends `ProvenanceEntry { operation: Merge, source_lot: Some(source_id), amount: source_quantity, tick, event_id }` to target
- Archives the source lot
- Returns `target_id`

### 3. Zero-quantity enforcement

If any operation would produce a zero-quantity lot, that lot must be archived. This ticket enforces this for split edge cases. (Note: `split_lot` already prevents `amount == lot.quantity` which would zero the source, so this is a guard rather than a common path.)

## Files to Touch

- `crates/worldwake-core/src/world.rs` (modify — add `split_lot`, `merge_lots`)

## Out of Scope

- Load accounting during split/merge (E04ITECON-007)
- Container capacity checks during split/merge (E04ITECON-007 + E05)
- Physical placement validation ("same container" rule deferred to E05)
- Consumption / production / spoilage operations (future systems epics)
- Event log emission (E06)

## Acceptance Criteria

### Tests That Must Pass

1. `split_lot(lot, 3)` on a lot with quantity 10 produces two lots: source has 7, new has 3
2. Total quantity is preserved exactly: `source.quantity + new.quantity == original`
3. Both lots have the same `CommodityKind` after split
4. Split appends a `ProvenanceEntry` with `LotOperation::Split` to both lots
5. `split_lot` with `amount == 0` returns error
6. `split_lot` with `amount >= lot.quantity` returns `WorldError::InsufficientQuantity`
7. `split_lot` on a non-ItemLot entity returns error
8. `merge_lots(apple_lot, grain_lot)` returns error (different commodities)
9. `merge_lots(a, b)` adds b's quantity to a and archives b
10. Merge appends `ProvenanceEntry` with `LotOperation::Merge` to the target
11. After merge, source lot is archived (`world.is_archived(source)`)
12. Provenance is preserved through split then merge: chain is traceable
13. `Waste` lots can be split and merged like any other commodity
14. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. **Conservation**: split and merge preserve total quantity exactly (spec 9.5)
2. **No negative stocks**: no operation produces negative quantity (spec 9.6)
3. **No live zero-quantity lots**: zero-quantity lots are archived immediately
4. **Provenance is append-only**: entries are only added, never removed or mutated

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` (extend test module) — split conservation, merge conservation, error cases, provenance chain, Waste handling

### Commands

1. `cargo test -p worldwake-core split`
2. `cargo test -p worldwake-core merge`
3. `cargo clippy --workspace && cargo test --workspace`
