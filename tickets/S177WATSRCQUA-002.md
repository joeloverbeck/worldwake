# S177WATSRCQUA-002: `ItemLot.quality` propagation at extraction commit

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-core/items` (field addition to `ItemLot`), `worldwake-systems/production_actions` (extraction commit writes source quality onto produced lot), `worldwake-sim/save_load` (SAVE_FORMAT_VERSION bump)
**Deps**: S177WATSRCQUA-001

## Problem

The spec's D3 deliverable propagates a water source's `quality` onto the item lot produced by extraction, so that Drink (ticket 005) can read the lot's intrinsic quality at consumption time. Without this propagation, lot quality is lost between extraction and drink, breaking the source/sink causal chain (FND-4) and forcing Drink to re-discover quality from the source — which would violate locality once the lot has been moved through inventory.

## Assumption Reassessment (2026-05-31)

1. `ItemLot` at `crates/worldwake-core/src/items.rs:317` carries `commodity, quantity, provenance`. Derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. `Option<WaterQuality>` is derive-compatible.
2. ItemLot construction sites are 5 in worldwake-core (`world.rs:628, 4880, 5831` and `component_tables.rs:452, 1265`); test-only sites also touch ItemLot. Construction-site count is informational — `Option<WaterQuality>::None` is meaningful for non-water lots.
3. Extraction commit lives at `crates/worldwake-systems/src/production_actions.rs::apply_harvest_resource` (lines 956-1067). The function already reads the source's `available_quantity` and writes the consumed quantity back; adding a `source.quality` read at the same site and writing it onto the produced `ItemLot` is structurally identical to the existing read pattern.
4. Shared abstraction boundary: the `ItemLot` component's serialized shape. Like `ResourceSource`, adding a field requires `#[serde(default)]` for RON/save-load tolerance plus the cascade bump (111→112).
5. The water-extraction-to-item-lot path is established by S79 (archived). Per S79's Outcome: "water harvest via Well workstation is the explicit contract." This ticket extends that contract with quality propagation.

## Architecture Check

1. Source/sink propagation (FND-4) — the lot inherits the source's quality at extraction commit, so the quality fact has explicit provenance back to the source entity. FND-29A inspectability is preserved.
2. `Option<WaterQuality>` (vs. a sibling component on the lot entity) is the FND-26 state-cohesion choice — the lot is a single ECS component already; adding a separate quality component on every produced lot is wasteful state and creates a second read site for every consumer.
3. Per FND-28, no parallel "untyped water lot" path is preserved — the field is added, not aliased. Non-water lots carry `quality: None` permanently.

## Verification Layers

1. Field addition compiles — full workspace build.
2. ItemLot serialization roundtrip — focused test in `items.rs` covers `Some(quality)` and `None` variants.
3. Extraction commit propagates quality — focused integration test seeds a `ResourceSource { quality: Some(Muddy), … }`, runs the harvest action commit, and asserts the produced `ItemLot.quality == Some(Muddy)`.
4. Non-water extraction sets `quality: None` — focused integration test seeds an apple/grain `ResourceSource { quality: None, … }` and asserts the produced lot's quality is `None`.

## What to Change

### 1. Add `quality` field to `ItemLot`

`crates/worldwake-core/src/items.rs:317`:

```rust
pub struct ItemLot {
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub provenance: Vec<ProvenanceEntry>,
    #[serde(default)]
    pub quality: Option<WaterQuality>,
}
```

### 2. Propagate source quality at extraction commit

`crates/worldwake-systems/src/production_actions.rs::apply_harvest_resource` (lines 956-1067) — at the point where the produced `ItemLot` is constructed (the new-lot creation block after the source's `available_quantity` is decremented), read `source.quality` and write it onto the lot:

```rust
// existing: let lot = ItemLot { commodity, quantity: actual, provenance, … };
let lot = ItemLot {
    commodity: source.commodity,
    quantity: actual,
    provenance,
    quality: source.quality,
};
```

If a single extraction merges into an existing `ItemLot` on the actor (the typical inventory-merge pattern), the merge semantics must preserve `quality`: if the existing lot's `quality` matches the source's, merge; if they differ, the merge is illegal (an actor cannot blend clean and muddy water into one quantity). For now (out-of-scope for this ticket beyond a flagged assertion), assume the actor's inventory has at most one water lot at a time; document this as a known constraint that ticket 005 (Drink) or a future spec must address if mixed-quality lots become possible.

### 3. Update existing ItemLot construction sites

Each of the 5 sites in `worldwake-core/src/world.rs` and `component_tables.rs` adds `quality: None` (or relies on spread syntax if already in use).

### 4. Bump `SAVE_FORMAT_VERSION`

`crates/worldwake-sim/src/save_load.rs:7`: change `111` to `112`.

## Files to Touch

- `crates/worldwake-core/src/items.rs` (modify — add `quality` field to `ItemLot` + roundtrip test)
- `crates/worldwake-core/src/world.rs` (modify — 3 sites at 628, 4880, 5831)
- `crates/worldwake-core/src/component_tables.rs` (modify — 2 sites at 452, 1265)
- `crates/worldwake-systems/src/production_actions.rs` (modify — extraction commit writes source.quality onto produced lot; new focused test)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump `SAVE_FORMAT_VERSION` 111→112)

## Out of Scope

- Drink action reading `ItemLot.quality` — owned by ticket 005.
- Trade/exchange propagation of `quality` between agents — out of scope; the trade path already moves whole lots, so quality rides along automatically. Document as a confirmation rather than a behavioral change.
- Mixed-quality lot merging — flagged as a known constraint in What to Change §2; deferred unless a future spec requires it.
- Adding quality to non-`ItemLot` carriers (UniqueItem, Container) — out of scope; water is exclusively a stackable commodity.

## Acceptance Criteria

### Tests That Must Pass

1. New: `item_lot_with_quality_roundtrip` in `crates/worldwake-core/src/items.rs` test module — `ItemLot { …, quality: Some(WaterQuality::Stale) }` and `quality: None` both roundtrip through bincode.
2. New: `harvest_propagates_source_quality_to_produced_lot` in `crates/worldwake-systems/src/production_actions.rs` test module — seed `ResourceSource { quality: Some(Muddy), … }`, run harvest action, assert `lot.quality == Some(Muddy)`.
3. New: `harvest_non_water_source_produces_lot_with_none_quality` — seed apple `ResourceSource { quality: None, … }`, assert produced lot `quality == None`.
4. Existing: `cargo test --workspace` passes.

### Invariants

1. Every `ItemLot` produced by extraction has `quality` equal to the source's `quality` at commit time.
2. Non-water `ItemLot`s always have `quality: None`.
3. `SAVE_FORMAT_VERSION` is now 112.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/items.rs` (test module extension) — roundtrip with quality variants.
2. `crates/worldwake-systems/src/production_actions.rs` (test module extension) — water+quality and non-water+None paths.

### Commands

1. `cargo test -p worldwake-core item_lot` — targeted ItemLot tests.
2. `cargo test -p worldwake-systems harvest_propagates_source_quality` — targeted extraction tests.
3. `./scripts/verify.sh` — full workspace.

See Merge-Order Constraints in Step 6 summary — SAVE_FORMAT_VERSION cascade 110→111→112→113→114→115 must land in dependency order.
