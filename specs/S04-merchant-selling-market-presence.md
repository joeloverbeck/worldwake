**Status**: PENDING

# Merchant Selling Market Presence

## Summary
Design proactive seller-side merchant behavior for Worldwake without adding abstract market state, price tables, or mirrored buyer/seller logic. Replace passive seller discovery from `MerchandiseProfile` alone with explicit concrete sale listings on item lots, and give merchants a real `SellCommodity` behavior that creates and maintains those listings at a market for a bounded time.

This spec is intentionally forward-looking. It extends archived E11 trade/economy and the E13 decision architecture, but it is not part of the active E14-E22 implementation sequence. Do not schedule implementation ahead of the current phase gates in `specs/IMPLEMENTATION-ORDER.md`.

Note: this spec intentionally models a first seller-side market-presence pass where sale lots remain directly possessed by the seller. If the project wants a cleaner longer-term distinction between carried stock, stored stock, and displayed sale stock, see [S05-merchant-stock-storage-and-stalls.md](/home/joeloverbeck/projects/worldwake/specs/S05-merchant-stock-storage-and-stalls.md). That follow-on spec upgrades this design from direct-possession sale readiness to explicit facility stock and display custody.

## Why This Exists
Current trade is buyer-driven:
- a buyer generates `AcquireCommodity`
- the planner discovers a local seller through `MerchandiseProfile`
- the buyer executes the trade action

That architecture is coherent for passive exchange, but it leaves a gap in merchant behavior:
- `MerchandiseProfile` means both "this agent trades in this commodity" and "this agent is actively offering stock right now"
- sellers do not choose when to occupy a market and make inventory available
- `RestockCommodity` and `MoveCargo` can prepare stock, but nothing converts prepared stock into an explicit market-facing sale state
- buyer discovery is too abstract because it queries "agents selling at place" instead of concrete lots being offered

This spec fixes those gaps by making sale availability concrete and giving merchant selling a real action path.

## Phase
Phase 4+: Economy Deepening, Step 14

## Crates
- `worldwake-core`
- `worldwake-sim`
- `worldwake-systems`
- `worldwake-ai`

## Dependencies
- E14

E14 is not strictly required for the first implementation pass because local sale listings are directly observable, but the design must remain compatible with belief-only planning once richer market information propagation exists.

## Design Goals
1. Keep buyer-side `AcquireCommodity` intact as the demand-side driver.
2. Make seller availability concrete through lot state, not through profile inference.
3. Give merchants a real proactive seller behavior with explicit time cost.
4. Preserve locality: buyers only discover listed lots they can actually observe.
5. Avoid hidden market objects, price indices, and threshold multiplier formulas.
6. Do not add compatibility shims. Replace the old seller-discovery path cleanly.

## Deliverables

### 1. `SaleListing` Component
Concrete world state that marks a specific lot as currently offered for sale.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct SaleListing {
    listed_at: Tick,
}

impl Component for SaleListing {}
```

This component belongs on `EntityKind::ItemLot`.

Only `listed_at` is stored. The following are derived from authoritative state instead of duplicated:
- seller: current direct possessor of the lot
- commodity: `ItemLot.commodity`
- place: lot effective place

This keeps the model concrete and avoids stale duplicate state.

### 2. Replace Seller Discovery With Lot Discovery
Buyer planning and trade affordance enumeration must stop discovering sellers from `MerchandiseProfile` alone.

Replace the conceptual query:
- `agents_selling_at(place, commodity)`

With concrete lot-oriented queries:

```rust
fn listed_sale_lots_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
fn seller_for_sale_lot(&self, lot: EntityId) -> Option<EntityId>;
```

`listed_sale_lots_at` returns lots where all of the following hold:
- lot has `SaleListing`
- lot effective place is `place`
- lot commodity matches
- lot has a direct possessor who is alive, capable, and effectively at the same place

`seller_for_sale_lot` derives the seller from the lot's direct possessor. If the lot is not possessed, contained, or colocated with a capable seller, the listing is invalid and should not be surfaced.

This is the central architectural cleanup. Buyers discover actual offered lots, not profile-declared merchant intent.

### 3. `SellCommodity` Goal Becomes Real
`SellCommodity { commodity }` becomes an active enterprise goal instead of a deferred enum variant.

The goal means:
- "I want to establish and maintain a concrete sale presence for this commodity at my market."

It does not mean:
- "I am guaranteed to accept any trade."

Trade acceptance remains bilateral and bundle-based per E11.

### 4. Extend `TradeDispositionProfile`
Add a seller-side market presence parameter.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct TradeDispositionProfile {
    negotiation_round_ticks: NonZeroU32,
    initial_offer_bias: Permille,
    concession_rate: Permille,
    demand_memory_retention_ticks: u32,
    market_presence_ticks: NonZeroU32,
}
```

`market_presence_ticks` is how long a merchant will spend maintaining a sale stance before reevaluating. This is a concrete dampener against infinite market camping and supports agent diversity.

### 5. New Trade Action: `staff_market`
Add a seller-side trade-domain action that lists local eligible lots for sale and keeps the seller in place for a bounded time.

Action shape:
- domain: `Trade`
- name: `staff_market`
- duration: `DurationExpr::Finite(market_presence_ticks)`
- interruptibility: `FreelyInterruptible`
- visibility: `VisibilitySpec::SamePlace`
- body cost: non-zero attention/time cost

Preconditions:
- actor alive
- actor not in transit
- actor effectively at `home_market`
- actor has `MerchandiseProfile`
- actor controls at least one local lot of the target commodity

Start behavior:
- add `SaleListing { listed_at: current_tick }` to all eligible local controlled lots of the target commodity not already listed

Tick behavior:
- no special mutation beyond remaining active

Commit / abort behavior:
- remove `SaleListing` from any still-controlled local lots of that commodity that were listed by this market-presence cycle
- lots already traded away naturally disappear from the actor's controlled inventory and therefore are not unlisted from the actor

The action is not the trade itself. It is the market-presence action that makes lots available for buyer discovery.

### 6. Listing Eligibility
A lot may be listed only if:
- it is directly possessed by the seller
- it is local to the seller's current place
- its commodity is in `MerchandiseProfile.sale_kinds`
- the seller is at `MerchandiseProfile.home_market`

This spec intentionally does not introduce a separate "display container" or stall entity. Those can be added later if the world grows a concrete market-facility layer. The planned direction for that deeper architecture now lives in [S05-merchant-stock-storage-and-stalls.md](/home/joeloverbeck/projects/worldwake/specs/S05-merchant-stock-storage-and-stalls.md).

### 7. `SellCommodity` Candidate Generation
Emit `SellCommodity { commodity }` when all of the following hold:
- actor has `MerchandiseProfile`
- `commodity` is in `sale_kinds`
- actor is at `home_market`
- actor controls at least one local lot of `commodity`
- no local controlled lot of `commodity` is currently listed for sale

Optional strengthening signal:
- recent `DemandMemory` for the same market and commodity should raise ranking, not gate candidate existence

This avoids bootstrapping deadlock. Merchants can choose to establish a market presence even before demand memory exists.

### 8. `SellCommodity` Planning Semantics
Relevant op kinds:
- `Travel`
- `MoveCargo`
- `Trade`

Interpretation:
- `Travel` gets the merchant to `home_market`
- `MoveCargo` gets stock to `home_market`
- `Trade` via `staff_market` establishes explicit sale presence

`SellCommodity` is satisfied when:
- actor is at `home_market`
- at least one local controlled lot of `commodity` has `SaleListing`

This makes seller-side readiness a concrete state change rather than an eternal background desire.

### 9. `AcquireCommodity` Uses Listed Lots
Buyer candidate generation for `AcquireCommodity` must inspect:
- listed sale lots at the current place
- local unpossessed lots
- local sources
- local corpses
- known recipes

For a listed lot path, evidence must include:
- the sale lot entity
- the derived seller entity
- the place

Trade payload selection must target a concrete lot, not just an abstract commodity.

### 10. Update `TradeActionPayload`
Trade should operate against an identified listed lot.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct TradeActionPayload {
    counterparty: EntityId,
    sale_lot: EntityId,
    offered_commodity: CommodityKind,
    offered_quantity: Quantity,
    requested_quantity: Quantity,
}
```

Derived at commit time:
- requested commodity = `ItemLot.commodity` of `sale_lot`

Benefits:
- trade acts on concrete supply
- partial-lot trades remain explicit
- buyer and seller both negotiate over the same object, not an abstract commodity bucket

### 11. Trade Commit Rules
Trade succeeds only if, at commit time:
- buyer and seller are still co-located
- `sale_lot` still exists
- `sale_lot` still has `SaleListing`
- seller still directly possesses `sale_lot`
- seller still has enough quantity in the lot
- buyer still controls enough payment
- bilateral bundle valuation accepts the trade

On successful trade:
- transfer or split the listed lot as needed
- transfer payment
- append traded provenance
- remove `SaleListing` from the transferred lot portion
- if the original seller-retained remainder still exists and remains possessed locally, it may remain listed

### 12. Listing Cleanup System Responsibility
The trade system tick must prune invalid listings when:
- the lot no longer exists
- the lot is no longer possessed
- the possessor is dead or incapacitated
- the seller left the place
- the lot commodity is no longer part of the seller's `MerchandiseProfile.sale_kinds`

This keeps listing validity state-mediated and robust without requiring every consumer to re-implement cleanup logic.

### 13. DemandMemory Role
Demand memory remains concrete information about unmet demand and failed trade attempts.

In this design it does three things:
- boosts the ranking of `SellCommodity`
- boosts the ranking of `RestockCommodity`
- helps valuation for holding sale stock that has local remembered demand

It does not directly make a seller visible. Visibility comes from `SaleListing`.

### 14. No Compatibility Layer
When this spec is implemented:
- remove buyer discovery that treats `MerchandiseProfile` alone as active sell availability
- update planning snapshot, planning state, belief-view, candidate generation, goal model, and affordance code to use listed sale lots
- do not preserve dual semantics where both raw profile sellers and listed lots count as active offers

## Component Registration
Register in `component_schema.rs`:
- `SaleListing` on `EntityKind::ItemLot`

Update registration / serialization for:
- `TradeDispositionProfile` with `market_presence_ticks`

No new market singleton, market object, or price table component is permitted.

## SystemFn Integration

### `worldwake-systems`
- add `staff_market` trade action definition and handler
- extend trade-system tick to prune invalid `SaleListing` state
- extend trade commit logic to require `sale_lot` and listing validity

### `worldwake-ai`
- emit `SellCommodity`
- plan `SellCommodity` through travel / cargo / `staff_market`
- update `AcquireCommodity` evidence and payload generation to use listed lots
- update ranking so remembered local demand can increase sell-goal motive without overpowering self-care
- use existing blocked-intent memory to avoid infinite immediate relisting after an unproductive market-presence cycle

### `worldwake-sim`
- replace seller discovery belief queries with listed-lot belief queries
- expose listed sale lots in affordance enumeration and planning state
- keep valuation helper bilateral and bundle-based, but source commodity usefulness from `S06-commodity-opportunity-valuation.md` rather than direct-use-only heuristics

### `worldwake-core`
- add `SaleListing`
- extend `TradeDispositionProfile`
- keep `MerchandiseProfile` as enterprise intent, not sale visibility

## Cross-System Interactions (Principle 12)
- E11 trade state influences E13 planning through `SaleListing`, `DemandMemory`, and `MerchandiseProfile`
- E10 transport influences seller readiness by moving actual sale stock to `home_market`
- E09 needs influence whether merchants are willing to keep sale stock versus consume it themselves through trade valuation and ranking
- `S06-commodity-opportunity-valuation.md` determines whether held sale stock also has local indirect recipe utility for the same actor
- E14/E15 can later feed richer demand-memory updates through witnessed requests and local reports, but no direct system-to-system calls are added here

All interactions remain state-mediated:
- listed lots
- inventory possession
- demand memory
- trade events
- blocked intent memory

## FND-01 Section H

### Information-Path Analysis
- Buyers only discover listed lots at their current place.
- Sellers only list lots they physically possess at their current market.
- Market presence consumes ticks, so sale availability persists only while the seller is actually there.
- Demand memory comes from concrete failed or completed exchanges and ages out locally.
- No agent can query global merchant stock or global prices.

### Positive-Feedback Analysis
- successful sales -> more coin -> more procurement ability -> more sale stock
- remembered unmet demand -> stronger desire to staff market -> more chances to sell

### Concrete Dampeners
- `market_presence_ticks`: sale posture consumes bounded time and attention
- blocked-intent memory: repeated unproductive `SellCommodity` cycles are temporarily suppressed without adding a special merchant cooldown system
- conservation of coin and goods: success redistributes finite resources, limiting runaway expansion
- travel time and carry capacity: stock must be physically brought to market
- listing invalidation: sellers who leave or lose control of stock stop being sale-visible immediately
- demand-memory retention window: remembered demand naturally decays

### Stored vs Derived State
Stored authoritative state:
- `MerchandiseProfile`
- `DemandMemory`
- `TradeDispositionProfile`
- `SaleListing`
- inventory lots and possession
- trade and listing events

Derived transient read-model:
- current active seller for a listed lot
- whether a commodity is locally sellable
- whether `SellCommodity` is currently satisfiable
- whether a listed lot is still valid for commit
- relative motive boost from remembered demand

## Invariants
- listed sale availability is concrete lot state, never inferred from profile alone
- no lot can be sold unless it is physically present and possessed
- trades conserve quantity and coin exactly as before
- no hidden market object or location-wide stock scalar is introduced
- seller-side market presence has physical time cost
- all visibility and discovery remain local
- no backward compatibility path preserves the old abstract seller discovery model

## Tests
- [ ] buyer-side acquire generation discovers local listed sale lots and not unlisted merchant stock
- [ ] merchant emits `SellCommodity` when at `home_market` with unlisted sale stock
- [ ] `staff_market` lists eligible lots on start and removes stale listings on completion / abort
- [ ] buyer can trade against a listed lot and receives the correct split-off quantity
- [ ] unlisted merchant stock at the same place does not count as sellable
- [ ] seller leaving the place invalidates listing visibility immediately
- [ ] dead or incapacitated seller invalidates listing visibility immediately
- [ ] repeated failed market-presence cycles are damped through blocked-intent memory rather than a hidden merchant cooldown table
- [ ] `MoveCargo` followed by `SellCommodity` yields listed stock at `home_market`
- [ ] remembered demand raises sell-goal ranking without overpowering critical self-care
- [ ] planning snapshot and planning state preserve listed-lot visibility deterministically
- [ ] deterministic replay preserves listing/open-market behavior

## Acceptance Criteria
- `SellCommodity` is a real proactive seller behavior, not a placeholder enum variant
- buyer discovery uses concrete listed lots instead of profile inference
- merchants can establish explicit market presence with bounded time cost
- `MerchandiseProfile` remains enterprise intent, not instantaneous sell visibility
- trade payloads operate on concrete listed lots
- no compatibility shim keeps the old discovery semantics alive
- all new tick, quantity, and duration fields use proper newtypes (`Tick`, `Quantity`, `Permille`, `NonZeroU32`)
- all authoritative iteration remains deterministic

## References
- [E11-trade-economy.md](/home/joeloverbeck/projects/worldwake/archive/specs/E11-trade-economy.md)
- [E13-decision-architecture.md](/home/joeloverbeck/projects/worldwake/archive/specs/E13-decision-architecture.md)
- [S05-merchant-stock-storage-and-stalls.md](/home/joeloverbeck/projects/worldwake/specs/S05-merchant-stock-storage-and-stalls.md)
- [IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md)
- [FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md)
