# E04ITECON-006: Lot algebra — split and merge operations

**Status**: COMPLETED
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
4. `WorldError::InsufficientQuantity` already exists in `error.rs` as a struct variant carrying `{ entity, requested, available }` with raw `u32` payloads — confirmed
5. `world.rs` already owns the public item/container factory API and carries the inline unit tests for world-facing component lifecycle behavior; this ticket should extend that test module rather than introduce a new test harness — confirmed
6. `World::create_item_lot` currently seeds canonical `Created` provenance with `event_id: None`; split-created lots therefore need either a lower-level lot-construction helper or an equivalent direct initialization path so split lineage is recorded cleanly without abusing the public factory semantics — confirmed

## Architecture Check

1. Operations are methods on `World` (not free functions) so they can create/archive entities atomically
2. Split creates a new lot entity and reduces the source; merge archives one lot and increases the other
3. A private world helper for constructing `ItemLot` entities is preferable to duplicating lot initialization logic or routing split-created lots through the public `create_item_lot` API, because split-born lots need canonical lineage setup that differs from exogenous creation
4. Zero-quantity lots must never remain live; in practice `split_lot` should reject `amount >= source.quantity`, and `merge_lots` should archive the consumed source lot immediately
5. Split/merge amounts should use `Quantity` end-to-end so conservation-sensitive APIs stay semantically typed
6. Provenance payloads should link to the other participating lot via a relationship-oriented field so split and merge history stay symmetric and extensible

## What to Change

### 1. Add `World::split_lot` method

```rust
pub fn split_lot(
    &mut self,
    lot_id: EntityId,
    amount: Quantity,
    tick: Tick,
    event_id: Option<EventId>,
) -> Result<(EntityId, EntityId), WorldError>
```

- Validates `lot_id` is alive with an `ItemLot` component
- Validates `amount > Quantity(0)` and `amount < lot.quantity` (splitting the full amount is just a move, not a split)
- Reduces source lot quantity by `amount`
- Creates a new `ItemLot` entity with `quantity = amount` and `commodity` matching source
- Appends `ProvenanceEntry { operation: Split, related_lot: Some(new_lot_id), amount, tick, event_id }` to the source lot
- Seeds the new lot with canonical `Created` provenance for its own entity creation, then appends `ProvenanceEntry { operation: Split, related_lot: Some(lot_id), amount, tick, event_id }`
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
- Validates `target_id != source_id`
- Validates both have the same `CommodityKind` (different commodities cannot merge)
- Adds source quantity to target
- Appends `ProvenanceEntry { operation: Merge, related_lot: Some(source_id), amount: source_quantity, tick, event_id }` to target
- Appends `ProvenanceEntry { operation: Merge, related_lot: Some(target_id), amount: source_quantity, tick, event_id }` to source before archival
- Archives the source lot
- Returns `target_id`

### 3. Zero-quantity enforcement

If any operation would produce a zero-quantity lot, that lot must be archived or the operation must be rejected before it can leave the world in that state. This ticket enforces that by rejecting `split_lot(amount >= source.quantity)` and by archiving the fully consumed source in `merge_lots`.

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

1. `split_lot(lot, Quantity(3))` on a lot with quantity `Quantity(10)` produces two lots: source has `Quantity(7)`, new has `Quantity(3)`
2. Total quantity is preserved exactly: `source.quantity + new.quantity == original`
3. Both lots have the same `CommodityKind` after split
4. Split appends a `ProvenanceEntry` with `LotOperation::Split` to both lots
5. `split_lot` with `amount == Quantity(0)` returns error
6. `split_lot` with `amount >= lot.quantity` returns `WorldError::InsufficientQuantity`
7. `split_lot` on a non-ItemLot entity returns error
8. `merge_lots(apple_lot, grain_lot)` returns error (different commodities)
9. `merge_lots(a, b)` adds b's quantity to a and archives b
10. Merge appends `ProvenanceEntry` with `LotOperation::Merge` to the target
11. After merge, source lot is archived (`world.is_archived(source)`)
12. Provenance is preserved through split then merge: chain is traceable
13. `Waste` lots can be split and merged like any other commodity
14. `merge_lots(a, a)` returns error
15. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. **Conservation**: split and merge preserve total quantity exactly (spec 9.5)
2. **No negative stocks**: no operation produces negative quantity (spec 9.6)
3. **No live zero-quantity lots**: zero-quantity lots are archived immediately
4. **Provenance is append-only**: entries are only added, never removed or mutated
5. Conserved operation amounts stay typed as `Quantity`
6. Lot-to-lot provenance points at the other participating lot, not an ambiguously named one-way source field

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` (extend test module) — split conservation, merge conservation, error cases, provenance chain, Waste handling

### Commands

1. `cargo test -p worldwake-core split_lot`
2. `cargo test -p worldwake-core merge_lots`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Outcome amended: 2026-03-09
- Completion date: 2026-03-09
- What actually changed:
  - Added `World::split_lot` and `World::merge_lots` in `crates/worldwake-core/src/world.rs`
  - Added a private `create_item_lot_with_provenance` helper so canonical lot construction stays centralized while split-created lots can carry lineage-specific provenance cleanly
  - Added focused world tests for split conservation, merge conservation, non-`ItemLot` rejection, same-entity merge rejection, archived-source merge provenance, and `Waste` handling
  - Upgraded lot provenance from a one-way `source_lot` link to a symmetric `related_lot` link so split and merge histories can point to the other lot involved without overloading “source”
- Deviations from original plan:
  - Tightened the ticket before implementation to reflect the real `WorldError::InsufficientQuantity` shape and the existing inline `world.rs` test architecture
  - Replaced the interim asymmetric provenance interpretation with a stronger relationship-oriented field and symmetric split/merge linkage once the limitation was reviewed explicitly
- Verification results:
  - `cargo test -p worldwake-core items` passed
  - `cargo test -p worldwake-core split_lot` passed
  - `cargo test -p worldwake-core merge_lots` passed
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
