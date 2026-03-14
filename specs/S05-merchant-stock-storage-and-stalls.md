**Status**: PENDING

# Merchant Stock Storage and Stall Custody

## Summary
Design explicit merchant stock storage and market stall custody for Worldwake so merchant sale inventory is no longer modeled as "goods the merchant happens to be carrying at the market." This spec introduces concrete stock containers, stall containers, and lawful transfer paths between them so ownership, custody, sale visibility, theft, audit, and institutional stock control remain distinct.

This spec extends:
- [S01-production-output-ownership-claims.md](/home/joeloverbeck/projects/worldwake/specs/S01-production-output-ownership-claims.md)
- [S04-merchant-selling-market-presence.md](/home/joeloverbeck/projects/worldwake/specs/S04-merchant-selling-market-presence.md)

It is intentionally forward-looking and must not be scheduled ahead of the active phase gates in [IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md) without explicit reprioritization.

## Why This Exists
Current and near-future merchant logistics still stop one layer short of the cleanest architecture:
- current Phase 2 cargo semantics consider destination-local controlled stock sufficient for `MoveCargo`
- the merchant-selling draft makes sale visibility concrete through `SaleListing`
- but listed lots are still directly possessed by the seller

That design is coherent for a first pass, but it leaves several long-term architectural weaknesses:

1. It conflates "merchant is physically present" with "merchant is personally carrying all market stock."
2. It makes store inventory, displayed inventory, and carried travel inventory the same custody state.
3. It weakens theft, audit, and office/faction asset modeling because goods never have to pass through explicit market/storage locations.
4. It makes future delegation harder:
   - carriers cannot meaningfully deliver to a merchant-owned stock room without also becoming the active seller
   - shop stock cannot persist cleanly while the merchant steps away
   - institutional stores cannot distinguish treasury stock, shelf stock, and personal carried goods

The cleaner architecture is:
- stock exists in explicit storage/container entities at concrete places
- movement between carried, stored, and displayed states is performed by real actions
- sale visibility derives from explicitly staged/displayed stock, not from the seller carrying everything

This aligns directly with the foundations:
- Principle 4: persistent identity and explicit transfer
- Principle 7: locality of interaction and information
- Principle 15: expectation-based discovery
- Principle 22: ownership, custody, access, obligation, and jurisdiction are distinct
- Principle 23: social/economic artifacts should be world state, not controller abstractions

## Phase
Phase 4+: Economy Deepening, Step 14

## Crates
- `worldwake-core`
- `worldwake-sim`
- `worldwake-systems`
- `worldwake-ai`

## Dependencies
- S04
- S01

## Design Goals
1. Separate ownership from custody and from sale visibility.
2. Give merchants explicit stock locations at markets and shops.
3. Allow sale-visible stock to persist as world state without requiring the merchant to personally carry every lot.
4. Make delivery, shelving, display, audit, theft, and confiscation all operate on the same concrete objects.
5. Keep trade buyer discovery concrete and local.
6. Support institutional and faction-owned stores with no special singleton market manager.
7. Avoid backward-compatibility shims. When implemented, replace the direct-possession market model cleanly.

## Deliverables

### 1. `StockStoragePolicy` Component
Attach stock-storage rules to facilities or offices that function as a merchant's base of trade.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct StockStoragePolicy {
    stock_container: EntityId,
    display_container: Option<EntityId>,
}

impl Component for StockStoragePolicy {}
```

Interpretation:
- `stock_container`: long-lived storage for local market/shop inventory
- `display_container`: optional seller-facing display/stall location used for buyer-visible sale stock

This component belongs on entities such as:
- shops
- market stalls
- merchant-controlled store facilities
- office/faction trade facilities

No global market object is introduced. Storage remains attached to ordinary world entities.

### 2. Explicit Storage/Display Containers
Containers used for merchant stock must be ordinary `Container` entities with explicit placement and ownership.

Required properties:
- concrete entity identity
- deterministic capacity
- location at a real place
- normal ownership/custody relations

Examples:
- stockroom chest in General Store
- market stall crate in Village Square
- guild pantry in Bakers' Hall

The point is not decorative realism. The point is consequence carriers:
- stock can be counted
- stock can be moved
- stock can be stolen
- stock can be missing
- stock can remain after a merchant leaves

### 3. `StockAssignment` Component on Lots
Record whether a lot is ordinary storage stock or active sale stock.

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum StockAssignmentKind {
    Stored,
    Displayed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct StockAssignment {
    facility: EntityId,
    kind: StockAssignmentKind,
}

impl Component for StockAssignment {}
```

This component belongs on item lots.

Interpretation:
- `Stored`: local stock counted for inventory/audit but not automatically sale-visible
- `Displayed`: local stock staged for active sale visibility

This avoids abusing possession alone to represent sale readiness.

### 4. Replace Direct-Possession Sale Visibility With Displayed Stock
The merchant-selling draft should evolve from:
- listed lots directly possessed by seller

To:
- listed lots contained in the facility's `display_container`
- listing validity derived from facility, container, ownership/control, and local presence

Buyer discovery becomes:

```rust
fn listed_sale_lots_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
fn sale_facility_for_lot(&self, lot: EntityId) -> Option<EntityId>;
fn authorized_seller_for_sale_lot(&self, lot: EntityId) -> Option<EntityId>;
```

`authorized_seller_for_sale_lot` derives the active seller from current facility control rather than from direct possession of the lot.

This is the key architectural shift:
- sale stock can exist at the market without being hand-carried
- the seller still must be locally present and authorized
- theft, audit, and confiscation now act on facility stock, not imaginary market intent

### 5. New Transport/Stock Actions
Add explicit stock-handling actions rather than overloading generic `put_down`.

#### `store_stock`
- domain: `Transport`
- moves directly possessed eligible lots into the facility's `stock_container`
- lawful only if actor can exercise control over the facility stock

#### `stage_stock_for_sale`
- domain: `Transport` or `Trade`
- moves eligible lots from `stock_container` to `display_container`
- assigns `StockAssignmentKind::Displayed`
- may add or refresh `SaleListing`

#### `unstage_stock`
- domain: `Transport` or `Trade`
- moves displayed lots back into `stock_container`
- clears `SaleListing`
- preserves ownership

#### `collect_display_stock`
- domain: `Transport`
- moves displayed or stored lots back into direct possession when the actor is authorized

These are intentionally explicit because:
- "put down" only means relinquish direct possession to local ground/container context
- storage/display logistics have stronger semantics than generic dropping
- explicit verbs make later theft and audit traces clearer

### 6. `MoveCargo` Evolves to Destination Storage, Not Mere Arrival
For merchant logistics using a facility with `StockStoragePolicy`, the clean terminal condition should become:
- destination facility stock gap resolved by lots stored in `stock_container` or `display_container`

Not merely:
- merchant is carrying the right commodity at destination place

This is the main architectural improvement over the current runtime.

New cargo interpretation:
- ordinary personal `MoveCargo` can still target destination-local control
- merchant/facility restock `MoveCargo` should target explicit local stock custody at the destination facility

This should be expressed through goal identity and evidence, not through compatibility aliases.

### 7. `SellCommodity` Uses Displayed Stock
`SellCommodity { commodity }` from the merchant-selling draft should be satisfied when:
- the actor is at the relevant market/facility
- at least one eligible lot of that commodity is in the facility's `display_container`
- that lot has a valid `SaleListing`

This preserves seller-side active presence while removing the unrealistic "all sale stock must remain personally carried" requirement.

### 8. Facility Control Determines Lawful Access
Lawful access to stock and display containers must remain derived from ordinary control/ownership relations.

Examples:
- merchant owns shop -> can store, stage, unstage, audit
- guild quartermaster controls guild store -> can manipulate guild stock
- outsider cannot lawfully remove displayed goods just because they are visible

This is where the architecture becomes robust for E17:
- lawful stock handling and unlawful theft are no longer ambiguous

### 9. Inventory Audit Hooks
Facilities with `StockStoragePolicy` become valid audit targets.

Audit semantics should compare:
- expected facility-owned or controlled stock
- observed stock in storage/display containers

This produces the clean expectation path needed for:
- discovered shortage
- suspected theft
- spoilage/misplacement distinctions later

### 10. Institutional Compatibility
This model must support:
- one-person merchant shop
- stall vendor with portable display crate
- office-controlled treasury market
- guild-owned workshop/store pair
- carrier delivering stock into an institution's storage without becoming the seller

That last case is the key extensibility gain. Delivery and sale presence become separate roles.

## Component Registration
Register in authoritative schema:
- `StockStoragePolicy` on facility-like entities
- `StockAssignment` on `EntityKind::ItemLot`

No aggregate "shop stock count" component is permitted as authoritative truth.

## SystemFn Integration

### `worldwake-core`
- add `StockStoragePolicy`
- add `StockAssignment`
- preserve ordinary ownership/control/container helpers
- add any needed helpers for querying a facility's stock/display containers

### `worldwake-systems`
- add `store_stock`, `stage_stock_for_sale`, `unstage_stock`, and `collect_display_stock`
- update trade listing validity to operate on displayed lots rather than directly possessed lots
- update lawful transfer checks so displayed/store stock requires authorized facility control
- keep generic `put_down` as a primitive, but do not overload it to mean "merchant delivered stock correctly"

### `worldwake-sim`
- extend affordance enumeration and belief queries for facility stock/display visibility
- ensure deterministic container-based discovery and action binding
- preserve state-mediated legality and event emission for storage/display transfers

### `worldwake-ai`
- update merchant restock and selling planning:
  - `MoveCargo` for merchant restock targets explicit facility stock custody
  - `SellCommodity` targets display/listing readiness
- allow delivery roles and seller roles to differ while sharing the same storage state
- use blocked-intent memory for repeated failed storage/staging attempts rather than hidden cooldown logic

## Cross-System Interactions (Principle 12)
- E10 transport writes lot location/custody changes into stock/display containers
- merchant selling reads displayed lots and listing state
- trade consumes displayed lots through ordinary lot transfer
- E15 discovery reads stock containers and display containers during audits
- E16 offices/factions express authority through ownership/control of facilities and containers
- E17 theft operates on the same stored/displayed lots when access is unlawful

No direct system-to-system command path is introduced. Influence travels only through:
- containers
- lot containment
- stock assignment state
- ownership/control relations
- listing state
- emitted events

## FND-01 Section H

### Information-Path Analysis
- stock exists at concrete places in concrete containers
- buyers only discover displayed/listed stock at their local place
- auditors only know stock is missing after observing the relevant facility containers
- merchants only manipulate stock they can physically reach and lawfully control
- institutions know about stock through their records/audits/agents, not through global inventory queries

### Positive-Feedback Analysis
- successful sale cycles can increase coin, which funds more procurement and more stock
- well-stocked facilities can attract more buyers, which can justify more staging behavior
- explicit storage can also increase theft targets, making profitable stores more vulnerable

### Concrete Dampeners
- container capacity limits how much stock can be stored or displayed
- travel time and carry capacity limit replenishment
- seller presence is still required for active sale visibility
- displayed stock is theft-exposed and can be removed or contested
- blocked-intent memory suppresses repeated failed staging/storage loops
- finite stock and finite demand remain the primary economic dampeners

### Stored vs Derived State
Stored authoritative state:
- `StockStoragePolicy`
- `StockAssignment`
- container relationships
- ownership/control relations
- `SaleListing`
- item lots
- stock transfer and trade events

Derived transient read-model:
- current display-visible stock at a place
- current authorized seller for a displayed lot
- whether a facility stock gap is resolved
- whether a shortage implies likely theft, sale, or ordinary depletion

## Invariants
- merchant stock, displayed stock, and carried stock are distinct custody states
- ownership and possession remain distinct from facility storage assignment
- sale visibility never comes from `MerchandiseProfile` alone
- facility restock is satisfied by explicit destination stock custody, not by mere arrival while carrying
- unauthorized removal of stored/displayed stock is not a lawful transport path
- no compatibility layer preserves both direct-possession selling and storage/display selling as equal first-class paths

## Tests
- [ ] merchant restock can end by storing goods into facility stock without requiring the merchant to keep carrying them
- [ ] displayed stock is buyer-visible locally while undisplayed stored stock is not
- [ ] carrier can deliver stock into merchant facility storage without becoming the active seller
- [ ] seller can stage stock from storage into display and unstage it later
- [ ] trade consumes displayed stock and preserves exact quantity conservation
- [ ] audit can distinguish displayed+stored stock from absent stock
- [ ] unlawful removal of displayed or stored owned stock requires theft, not lawful pickup
- [ ] displayed stock can remain in place when merchant briefly leaves, but listing validity drops until authorized seller presence returns
- [ ] institutional facility ownership authorizes the correct actors and rejects outsiders
- [ ] deterministic replay preserves storage/display transitions and trade outcomes

## Acceptance Criteria
- merchant restock, sale visibility, and audit semantics all operate on explicit facility stock state
- carried goods are no longer the hidden source of all merchant availability
- the architecture cleanly supports carriers, merchants, and institutions as distinct actors around the same stock
- theft, discovery, and trade share one concrete stock model
- no world singleton, stock scalar, or backward-compatibility alias is introduced

## References
- [FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md)
- [IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md)
- [S01-production-output-ownership-claims.md](/home/joeloverbeck/projects/worldwake/specs/S01-production-output-ownership-claims.md)
- [S04-merchant-selling-market-presence.md](/home/joeloverbeck/projects/worldwake/specs/S04-merchant-selling-market-presence.md)
- [E15-rumor-witness-discovery.md](/home/joeloverbeck/projects/worldwake/specs/E15-rumor-witness-discovery.md)
- [E16-offices-succession-factions.md](/home/joeloverbeck/projects/worldwake/specs/E16-offices-succession-factions.md)
- [E17-crime-theft-justice.md](/home/joeloverbeck/projects/worldwake/specs/E17-crime-theft-justice.md)
