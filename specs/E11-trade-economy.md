# E11: Trade, Exchange & Merchant Restock

## Epic Summary
Implement co-located exchange, deterministic negotiation over concrete bundles, merchant restock grounded in actual stock absence and observed unmet demand, and substitute demand. Prices must emerge from bilateral valuation of an exchange, not from lookup tables, global formulas, or hidden “market state.”

## Phase
Phase 2: Survival & Logistics

## Crate
`worldwake-systems`

## Dependencies
- E08 (actions and scheduler)
- E10 (physical procurement / transport is required for merchant restock acceptance criteria)

## Foundations Alignment Changes
This revision makes four critical corrections:

1. **Negotiation is no longer described as a vague willingness formula.** The system must compare concrete post-trade states for each side.
2. **Restock is no longer driven by a naked reorder threshold.** That was an abstract trigger detached from observed demand and actual sale intent.
3. **Local demand becomes a concrete information carrier.** Missed sales and buyer inquiries create merchant memory / evidence.
4. **No hidden market object exists.** There is no location price index, no equilibrium price table, and no global “scarcity score.”

## Deliverables

### MerchandiseProfile Component
Concrete merchant sale intent.

```rust
struct MerchandiseProfile {
    sale_kinds: Vec<CommodityKind>,
    home_market: Option<EntityId>,
}
```

This is not a price table. It is a statement of what this agent is trying to carry and sell.

An agent without `MerchandiseProfile` may still trade opportunistically, but merchant-style restock behavior depends on having explicit sale intent.

### DemandMemory Component
Local memory of concrete missed demand and sale opportunities.

```rust
struct DemandObservation {
    commodity: CommodityKind,
    quantity: Quantity,
    place: EntityId,
    tick: u64,
    counterparty: Option<EntityId>,
    reason: DemandObservationReason,
}
```

`DemandObservationReason` includes:
- `WantedToBuyButNoSeller`
- `WantedToBuyButSellerOutOfStock`
- `WantedToBuyButTooExpensive`
- `WantedToSellButNoBuyer`

These records age out naturally, but while present they are real carriers of consequence. They are how a merchant can remember that people in Town wanted bread yesterday.

### TradeDispositionProfile Component
Per-agent negotiation style / time cost.

- `negotiation_round_ticks: NonZeroU32`
- `initial_offer_bias: Permille`
- `concession_rate: Permille`

These parameters shape *offer sequencing*, not a hidden global price formula.

### Negotiation / Exchange Action
A trade action is a scheduled co-located interaction between a buyer and seller.

#### Physical Preconditions
- buyer and seller co-located
- seller controls the good
- buyer controls the offered coin / barter good
- both are alive and capable of acting

#### Negotiation Procedure
Trade must be resolved through deterministic evaluation of candidate exchange bundles.

For each candidate bundle:
- seller gives `goods`
- buyer gives `coin` (Phase 2 may remain coin-only; barter extension is allowed later)

Each side independently evaluates:
- current state utility from their own beliefs
- post-trade state utility if the candidate bundle executes

The trade succeeds only if the bundle improves or at least meets each side’s acceptance rule.

This replaces the earlier hand-wavy “willingness-to-pay / willingness-to-accept formula.” The system must actually compare concrete ownership outcomes.

#### Candidate Bundle Search
The search order must be deterministic and bounded.

A compliant Phase 2 implementation may:
1. choose a commodity lot and quantity
2. enumerate affordable coin amounts in a deterministic order
3. let each side accept / reject based on post-trade evaluation
4. settle on the first mutually acceptable amount or terminate with no deal

No global base price is permitted.

### Valuation Inputs
Valuation is derived from concrete believed state, not from a price table.

Examples of allowed inputs:
- current physiological needs (food / water more valuable when needed)
- current wounds and pain
- current coin holdings
- visible alternative sellers / buyers at the same place
- own inventory abundance / shortage
- recent `DemandMemory`
- `MerchandiseProfile.sale_kinds`
- believed procurement difficulty (travel + source availability known through beliefs)

The valuation helper must live in `worldwake-core` or `worldwake-sim`, not in `worldwake-ai`, so E11 does not depend on E13.

### Merchant Restock Planning Inputs
Merchant restock becomes a grounded candidate goal when all of the following hold:

1. the agent has `MerchandiseProfile`
2. a sale kind in that profile is absent from saleable stock
3. the agent either:
   - has recent `DemandMemory` for that kind, or
   - has explicit sale intent to keep that kind in circulation

This is the key correction: restock is no longer “stock < threshold -> restock.”  
It is “I am a merchant for this kind, I currently lack stock, and I have concrete reason to believe obtaining it is worthwhile.”

### Procurement Paths
Restock is satisfied only through physical procurement:

- travel to source and buy
- travel to source and harvest / craft
- receive delivery from another carrier

There is no magical refill and no stock respawn.

### Substitute Demand
When a preferred good is unavailable or a negotiation fails:

- the buyer inspects other available goods at the current place
- they rank substitutes using per-agent preference data
- they may buy the substitute if the exchange is beneficial

Preference ordering must be agent-specific and can live in a simple profile or commodity preference table.

### Trade Events
Every negotiation attempt must emit an event with:
- participants
- commodity
- quantity
- offered amount(s)
- success / failure
- failure reason if any

This supports event-trace causality and demand memory updates.

## Component Registration
New components to register in `component_schema.rs`:

- `MerchandiseProfile` — on merchant-like `EntityKind::Agent`
- `DemandMemory` — on `EntityKind::Agent`
- `TradeDispositionProfile` — on `EntityKind::Agent`

Trade still uses existing coin / inventory ownership components from `worldwake-core`.

## SystemFn Integration
- Implements the trade handler in `SystemDispatch`
- Runs once per tick for active negotiation / exchange actions
- Reads:
  - inventories / ownership
  - coin holdings
  - `HomeostaticNeeds`
  - `WoundList`
  - co-located agents
  - `MerchandiseProfile`
  - `DemandMemory`
  - local visible alternatives
- Writes:
  - item / coin transfers
  - trade attempt events
  - `DemandMemory` updates

## Cross-System Interactions (Principle 12)
- **E09 → E11**: needs affect valuation of food / water / rest-related goods
- **E10 → E11**: production and transport create actual procurement paths for restock
- **E11 → E13**: trade failures, demand memories, sale intent, and visible exchange affordances become candidate-goal evidence
- **E12 → E11**: wounds / incapacity alter what an agent values and what negotiations they can physically complete

## FND-01 Section H

### Information-Path Analysis
- Trade requires co-location
- Observed alternatives are local to the place
- Demand memories come from specific observed interactions
- Restock planning is based on believed sources and remembered unmet demand, not omniscient global stock checks

### Positive-Feedback Analysis
- **Successful selling → more coin → more procurement ability → more selling**
- **Scarcity at a place → more failed purchase attempts → stronger merchant incentive to restock**

### Concrete Dampeners
- **Merchant expansion dampeners**:
  - travel time
  - carry capacity
  - limited source stock
  - competition from co-located sellers
- **Demand-memory dampener**:
  - observations age out
  - if no one keeps asking, the signal fades naturally rather than living forever in an abstract market score

### Stored State vs. Derived Read-Model
**Stored (authoritative)**:
- inventories / ownership
- coin holdings
- `MerchandiseProfile`
- `DemandMemory`
- trade attempt events
- `TradeDispositionProfile`

**Derived (transient read-model)**:
- accepted transaction price
- candidate exchange bundles
- marginal value of a unit of goods or coin
- “does this merchant need to restock now?”

## Invariants Enforced
- 9.5: Conservation through trade
- 9.6: No negative stocks
- 9.7: Ownership transfer requires a valid possession chain
- Principle 2: no base-price table, no scarcity multiplier table, no global price formula
- Principle 7: all negotiation information is local or explicitly remembered

## Tests
- [ ] Negotiation succeeds only when a candidate bundle is mutually acceptable
- [ ] Successful trade transfers both goods and coin with conservation
- [ ] Cannot sell goods not in possession
- [ ] Higher physiological need can make a buyer accept a worse offer for survival goods
- [ ] Additional co-located alternative sellers can change accepted offers or cause the buyer to reject a bad deal
- [ ] Additional co-located alternative buyers can change seller acceptance outcomes
- [ ] Failed purchase due to empty stock records `DemandMemory`
- [ ] Merchant restock candidate appears from sale intent + stock absence + observed demand, not a naked reorder threshold
- [ ] No magical restock path exists
- [ ] Substitute demand activates when preferred good is unavailable or rejected
- [ ] Trade requires co-location
- [ ] No base-price table, no threshold-multiplier scarcity table, no global market score
- [ ] Negotiation duration is derived from `TradeDispositionProfile`

## Acceptance Criteria
- Pricing emerges from bilateral bundle valuation
- Merchant restock is grounded in sale intent plus observed unmet demand
- No hidden global market state exists
- All trades conserve goods and coin
- Substitute demand lets agents adapt to scarcity
- Physical procurement is the only restock path

## FND-01 Route Presence Note
E10 now provides explicit `InTransitOnEdge`, so future route-risk logic has a concrete carrier to build on.  
This epic still does **not** introduce ambush, interception, or trade-route danger scoring.

## Spec References
- Section 4.5 (trade and pricing)
- Section 7.2 (economic propagation)
- Section 8 (no magical merchant restock)
- Section 9.5 (conservation)
- Section 9.7 (ownership consistency)
- `docs/FOUNDATIONS.md` Principles 2, 3, 6, 7, 11, 12