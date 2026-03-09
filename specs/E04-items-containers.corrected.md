# E04: Goods, Lots, Unique Items & Container Capacity

## Epic Summary
Implement the prototype’s hybrid identity inventory model:
- stackable lots for bulk commodities
- unique entities for singular equipment / documents
- containers with deterministic capacity rules
- provenance on lot creation, split, and merge

This epic deliberately narrows scope to item identity, lot arithmetic, and capacity accounting. Physical placement and ownership semantics are completed in E05.

## Phase
Phase 1: World Legality

## Crate
`worldwake-core`

## Dependencies
- E03 (typed world model and entity lifecycle)

## Why this revision exists
The original version made `Weapon` a stackable `GoodType` and used a `HashMap<String, String>` for unique-item metadata. Both conflict with the spec’s hybrid-identity direction and with deterministic save/load.

Spec 3.6 says weapons are unique entities. The inventory model has to respect that now, not “later”.

## Deliverables

### CommodityKind
Define the stackable commodity taxonomy for Phase 1 bulk lots.

Required stackable commodities:
- `Apple`
- `Grain`
- `Bread`
- `Water`
- `Firewood`
- `Medicine`
- `Coin`
- `Waste`

Rules:
- all Phase 1 commodities are conserved unless an explicit world rule emits production, destruction, spoilage, or transformation
- `Waste` is conserved too; it is a material consequence, not a free sink

### TradeCategory (Optional but Recommended)
Because spec 4.3 mentions simple tools and weapons in the economic catalog, provide a lightweight category layer that can span both bulk and unique items.

Examples:
- `Food`
- `Water`
- `Fuel`
- `Medicine`
- `Coin`
- `SimpleTool`
- `Weapon`
- `Waste`

This lets later trade logic price unique tools / weapons without pretending they are stackable lots.

### ItemLot Component
`ItemLot`:
- `commodity: CommodityKind`
- `quantity: Quantity`
- `provenance: Vec<ProvenanceEntry>`

Rules:
- live lots always have `quantity > 0`
- zero-quantity lots are removed / archived, not kept around
- provenance entries are append-only

### ProvenanceEntry
Record enough information to preserve lot lineage:
- `tick: Tick`
- `event_id: Option<EventId>`
- `operation: LotOperation`
- `source_lot: Option<EntityId>`
- `amount: Quantity`

`LotOperation` cases:
- `Created`
- `Split`
- `Merge`
- `Produced`
- `Consumed`
- `Destroyed`
- `Spoiled`
- `Transformed`

### Lot Algebra
Provide pure and world-level helpers for:
- `split_lot(lot_id, amount: Quantity) -> Result<(EntityId, EntityId)>`
- `merge_lots(a, b) -> Result<EntityId>`

Rules:
- split and merge preserve total quantity exactly
- merge is allowed only for the same commodity and compatible world context
- “compatible world context” means at minimum the same effective place, same container / holder, and same legal state once E05 lands
- provenance must record the operation, not erase it

### UniqueItemKind
Define unique-item identity for singular objects.

Required minimum kinds for Phase 1:
- `SimpleTool`
- `Weapon`
- `Contract`
- `Artifact`
- `OfficeInsignia` (optional but useful)
- `Misc`

### UniqueItem Component
`UniqueItem`:
- `kind: UniqueItemKind`
- `name: Option<String>`
- `metadata: BTreeMap<String, String>`

Rules:
- metadata uses `BTreeMap`, never `HashMap`
- unique items are indivisible
- weapons are represented here, not in `ItemLot`

### Container Component
`Container`:
- `capacity: LoadUnits`
- `allowed_commodities: Option<BTreeSet<CommodityKind>>`
- `allows_unique_items: bool`
- `allows_nested_containers: bool`

Rules:
- capacity is measured in `LoadUnits`, not raw quantity
- capacity checks must account for both lots and unique items
- nested containers are allowed only if this flag is true

### Load Accounting
Provide deterministic helpers:
- `load_of_lot(lot: &ItemLot) -> LoadUnits`
- `load_of_unique_item(item: &UniqueItem) -> LoadUnits`
- `load_of_entity(world, entity_id) -> LoadUnits`
- `current_container_load(world, container_id) -> LoadUnits`
- `remaining_container_capacity(world, container_id) -> LoadUnits`

Rule:
- if a container is nested inside another container, its carried load is counted recursively exactly once

### Conservation Helper
Provide:
- `total_commodity_quantity(world, commodity) -> u64`
- `verify_conservation(world, commodity, expected_total) -> Result<()>`

This helper is intentionally global and does not care where the lots are located.
Lot-local quantities stay strongly typed as `Quantity`; the global helper widens to `u64` only at aggregation boundaries to avoid overflow when summing many lots.

## Invariants Enforced
- Spec 3.6: hybrid identity is explicit
- Spec 9.5: conserved quantities change only through explicit operations
- Spec 9.6: no negative quantity
- container capacity is deterministic and type-safe

Note:
- physical placement and containment-cycle invariants are finalized in E05

## Tests
- [ ] Split preserves total quantity
- [ ] Merge preserves total quantity
- [ ] Splitting more than available fails
- [ ] Merging different commodities fails
- [ ] Zero-quantity live lots are impossible
- [ ] Provenance is preserved through split / merge
- [ ] `Waste` is treated as conserved
- [ ] Weapons are represented as `UniqueItem`, not `ItemLot`
- [ ] Container load calculations include unique items and nested containers correctly
- [ ] `metadata` for unique items serializes deterministically

## Acceptance Criteria
- stackable and unique inventory identities are clearly separated
- all commodity operations preserve conservation
- no operation can create negative stock
- capacity accounting is defined in `LoadUnits`, not vague ad hoc quantities
- the item model is ready for E05 relation semantics

## Spec References
- Section 3.6 (hybrid identity)
- Section 4.3 (minimum goods catalog)
- Section 7.1 (material propagation includes waste)
- Section 9.5 (conservation)
- Section 9.6 (no negative stocks)
