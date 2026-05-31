# S177WATSRCQUA-002: `ItemLot.quality` propagation at extraction commit

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-core/items`, `worldwake-core/world`, `worldwake-core/world_txn`, `worldwake-systems/production_actions`, `worldwake-sim/save_load`
**Deps**: `archive/tickets/S177WATSRCQUA-001.md`

## Problem

Before this ticket, the spec's D3 deliverable was missing: a water source's `quality` was not propagated onto the item lot produced by extraction. Drink in ticket 005 needs that lot-local quality at consumption time; otherwise the quality fact would be lost between extraction and drink, breaking the FND-4 source/sink chain and encouraging later consumers to rediscover quality from a source that may no longer be local.

## Assumption Reassessment (2026-05-31)

1. Before this ticket, `ItemLot` at `crates/worldwake-core/src/items.rs` carried `commodity, quantity, provenance`. Derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. `Option<WaterQuality>` is derive-compatible.
2. The live constructor sweep found explicit `ItemLot` literals in `world.rs`, `component_tables.rs`, `delta.rs`, `load.rs`, and `items.rs` tests, plus the canonical `World` / `WorldTxn` item-lot creation path. All explicit literals now set `quality: None` and the canonical path defaults to `None` unless the caller supplies a source quality.
3. Extraction commit lives at `crates/worldwake-systems/src/production_actions.rs::apply_harvest_resource`. The function already reads the source's `available_quantity` and writes the consumed quantity back; adding a `source.quality` read at the same site and writing it onto the produced `ItemLot` is the same authoritative read/write boundary.
4. Shared abstraction boundary: the `ItemLot` component's serialized shape. Like `ResourceSource`, adding a field requires `#[serde(default)]` plus the cascade bump from save format 111 to 112.
5. The water-extraction-to-item-lot path is established by S79 (archived). Per S79's Outcome: "water harvest via Well workstation is the explicit contract." This ticket extends that contract with quality propagation.

## Architecture Check

1. Source/sink propagation (FND-4) — the lot inherits the source's quality at extraction commit, so the quality fact has explicit provenance back to the source entity. FND-29A inspectability is preserved.
2. `Option<WaterQuality>` on `ItemLot` is the FND-26 state-cohesion choice: the lot is already the ECS component carrying stackable commodity state. A sibling quality component would duplicate the consumer read path.
3. Per FND-28, no parallel "untyped water lot" path was introduced. Non-water lots carry `quality: None`; extraction uses the quality-aware creation path when source quality exists.

## Verified Layers

1. Field addition compiled under `cargo test --workspace`.
2. ItemLot serialization roundtrip is covered by `item_lot_with_quality_roundtrips_through_bincode`, including `Some(WaterQuality::Stale)` and `None`.
3. Extraction commit propagation is covered by `harvest_propagates_source_quality_to_produced_lot`, which seeds `ResourceSource { quality: Some(Muddy), ... }` and asserts the produced lot carries `Some(Muddy)`.
4. Non-water extraction is covered by `harvest_non_water_source_produces_lot_with_none_quality`, which asserts produced apple lots carry `quality: None`.

## Landed Changes

### 1. Added `quality` field to `ItemLot`

`crates/worldwake-core/src/items.rs` now stores:

```rust
pub struct ItemLot {
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub provenance: Vec<ProvenanceEntry>,
    #[serde(default)]
    pub quality: Option<WaterQuality>,
}
```

### 2. Propagated source quality at extraction commit

`crates/worldwake-systems/src/production_actions.rs::apply_harvest_resource` now captures `source.quality` before writing the drained source back, then creates the produced lot through `WorldTxn::create_item_lot_with_owner_and_quality`.

The landed harvest path creates a new lot rather than merging into existing actor inventory. Mixed-quality merging remains out of scope; whole-lot trade/transport continues to move the lot's stored quality with the lot.

### 3. Updated existing ItemLot construction sites

Explicit `ItemLot` literals in `world.rs`, `component_tables.rs`, `delta.rs`, `load.rs`, and `items.rs` tests now set `quality: None`.

The canonical `World` / `WorldTxn` creation path gained quality-aware helpers. Existing generic lot creation delegates to `quality: None`; extraction uses the quality-aware owner helper. `World::split_lot` preserves the source lot's quality on the split-off lot.

### 4. Bumped `SAVE_FORMAT_VERSION`

`crates/worldwake-sim/src/save_load.rs` now sets `SAVE_FORMAT_VERSION` to 112 and the save/load version tests assert 112.

## Landed Files

- `crates/worldwake-core/src/items.rs` — added `quality` field to `ItemLot` and roundtrip test.
- `crates/worldwake-core/src/world.rs` — added quality-aware canonical creation path, split preservation, and explicit literal updates.
- `crates/worldwake-core/src/world_txn.rs` — added quality-aware transaction creation helpers.
- `crates/worldwake-core/src/component_tables.rs` — updated explicit literals.
- `crates/worldwake-core/src/delta.rs` — updated explicit sample `ComponentValue::ItemLot` literal.
- `crates/worldwake-core/src/load.rs` — updated explicit load-test literals.
- `crates/worldwake-systems/src/production_actions.rs` — harvest commit writes source quality onto produced lots and adds focused tests.
- `crates/worldwake-sim/src/save_load.rs` — bumped `SAVE_FORMAT_VERSION` 111 to 112 and updated tests.

## Out of Scope

- Drink action reading `ItemLot.quality` — owned by ticket 005.
- Trade/exchange propagation of `quality` between agents — out of scope; the trade path already moves whole lots, so quality rides along automatically.
- Mixed-quality lot merging — deferred unless a future spec requires it.
- Adding quality to non-`ItemLot` carriers (`UniqueItem`, `Container`) — out of scope; water is exclusively a stackable commodity.

## Acceptance Result

### Passed Tests

1. `item_lot_with_quality_roundtrips_through_bincode` in `crates/worldwake-core/src/items.rs` covers `Some(WaterQuality::Stale)` and `None`.
2. `harvest_propagates_source_quality_to_produced_lot` in `crates/worldwake-systems/src/production_actions.rs` covers water extraction from a muddy source.
3. `harvest_non_water_source_produces_lot_with_none_quality` covers non-water extraction.
4. `cargo test --workspace` passed.

### Verified Invariants

1. Every `ItemLot` produced by extraction has `quality` equal to the source's `quality` at commit time.
2. Non-water `ItemLot`s created outside quality-aware extraction have `quality: None`.
3. `SAVE_FORMAT_VERSION` is now 112.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/items.rs` — roundtrip with quality variants.
2. `crates/worldwake-systems/src/production_actions.rs` — water+quality and non-water+None paths.
3. `crates/worldwake-sim/src/save_load.rs` — current save format version assertion now names 112.

### Commands Run

1. `cargo test -p worldwake-core item_lot`.
2. `cargo test -p worldwake-systems harvest_propagates_source_quality`.
3. `cargo test -p worldwake-systems harvest_non_water_source_produces_lot_with_none_quality`.
4. `cargo test -p worldwake-sim --lib`.
5. `cargo test --workspace`.

`./scripts/verify.sh` is waived for this per-ticket closeout because the `implement-spec-tickets` harness owns the final pre-push gate after the full S177 ticket queue lands.

## Outcome

Completed on 2026-05-31.

- Added `ItemLot.quality: Option<WaterQuality>` with `#[serde(default)]`.
- Added quality-aware `World` / `WorldTxn` lot creation helpers and preserved quality when splitting lots.
- Updated harvest commit to propagate `ResourceSource.quality` onto the produced lot.
- Updated explicit `ItemLot` literals and bumped `SAVE_FORMAT_VERSION` to 112.

## Deviations

- The live constructor sweep was broader than the ticket's initial 5-site estimate; `delta.rs`, `load.rs`, and canonical `World` / `WorldTxn` creation helpers were current-ticket shared-shape fallout.
- No mixed-quality merge rule landed because harvest creates a new lot rather than merging into existing inventory.

## Verification Result

- Passed `cargo test -p worldwake-core item_lot`.
- Passed `cargo test -p worldwake-systems harvest_propagates_source_quality`.
- Passed `cargo test -p worldwake-systems harvest_non_water_source_produces_lot_with_none_quality`.
- Passed `cargo test -p worldwake-sim --lib`.
- Passed `cargo test --workspace`.
- Waived `./scripts/verify.sh` because this harness reserves the final pre-push verification gate for the completed S177 ticket family.
