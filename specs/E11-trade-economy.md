# E11: Trade, Exchange & Merchant Restock

## Epic Summary
Implement co-located exchange, deterministic negotiation over concrete bundles, merchant restock grounded in actual stock absence and observed unmet demand, and substitute demand. Prices must emerge from bilateral valuation of an exchange, not from lookup tables, global formulas, or hidden "market state."

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
4. **No hidden market object exists.** There is no location price index, no equilibrium price table, and no global "scarcity score."

## Deliverables

### MerchandiseProfile Component
Concrete merchant sale intent.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct MerchandiseProfile {
    sale_kinds: BTreeSet<CommodityKind>,
    home_market: Option<EntityId>,
}

impl Component for MerchandiseProfile {}
```

`sale_kinds` uses `BTreeSet` (not `Vec`) because iteration order must be deterministic and membership queries are the primary operation. No duplicate entries are meaningful.

This is not a price table. It is a statement of what this agent is trying to carry and sell.

An agent without `MerchandiseProfile` may still trade opportunistically, but merchant-style restock behavior depends on having explicit sale intent.

### DemandMemory Component
Local memory of concrete missed demand and sale opportunities.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct DemandMemory {
    observations: Vec<DemandObservation>,
}

impl Component for DemandMemory {}
```

The `Vec` is acceptable here because append-order is meaningful (chronological observation sequence). New observations are pushed to the end; aging prunes from the front.

### DemandObservation

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct DemandObservation {
    commodity: CommodityKind,
    quantity: Quantity,
    place: EntityId,
    tick: Tick,
    counterparty: Option<EntityId>,
    reason: DemandObservationReason,
}
```

### DemandObservationReason

```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
enum DemandObservationReason {
    WantedToBuyButNoSeller,
    WantedToBuyButSellerOutOfStock,
    WantedToBuyButTooExpensive,
    WantedToSellButNoBuyer,
}
```

### DemandMemory Aging Mechanism

During each trade system tick, for each agent with `DemandMemory` and `TradeDispositionProfile`, drop all `DemandObservation` entries where `current_tick.0 - observation.tick.0 > agent.trade_disposition_profile.demand_memory_retention_ticks as u64`. This per-agent parameter is the concrete dampener for the demand-memory feedback loop (Principle 8) and supports agent diversity (Principle 11).

### TradeDispositionProfile Component
Per-agent negotiation style / time cost.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct TradeDispositionProfile {
    negotiation_round_ticks: NonZeroU32,
    initial_offer_bias: Permille,
    concession_rate: Permille,
    demand_memory_retention_ticks: u32,
}

impl Component for TradeDispositionProfile {}
```

- `negotiation_round_ticks`: Duration of a negotiation action in ticks.
- `initial_offer_bias`: How aggressively the agent opens (0 = generous, 1000 = maximally aggressive).
- `concession_rate`: How quickly the agent concedes per negotiation round (Permille per round).
- `demand_memory_retention_ticks`: Tick duration after which `DemandObservation` entries are pruned. This is a duration, not an absolute tick. Supports agent diversity: a cautious merchant might retain demand memory for 500 ticks, while a forgetful one only 100.

These parameters shape *offer sequencing*, not a hidden global price formula.

### SubstitutePreferences Component

Per-agent ordering of which commodities can substitute for others within a trade category.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct SubstitutePreferences {
    preferences: BTreeMap<TradeCategory, Vec<CommodityKind>>,
}

impl Component for SubstitutePreferences {}
```

`BTreeMap` keyed by `TradeCategory` for deterministic iteration. The `Vec<CommodityKind>` value is ordered by preference (index 0 = most preferred substitute). When a preferred good is unavailable, the agent walks this list to find acceptable alternatives.

Registered on `EntityKind::Agent` in Component Registration.

### Negotiation / Exchange Action

A trade action is a multi-tick scheduled co-located interaction between a buyer and seller.

#### Action Model

- Negotiation is a multi-tick action with `DurationExpr::Fixed(negotiation_round_ticks)` where `negotiation_round_ticks` comes from the initiator's `TradeDispositionProfile`.
- Candidate bundle search and acceptance evaluation happen at commit time (when the action completes), like craft/harvest actions.
- `body_cost_per_tick`: Negotiation consumes time/attention (Principle 6). The specific cost is defined in the action definition.
- `interruptibility: FreelyInterruptible` — if either party leaves the place, negotiation fails.
- `commit_conditions`: both parties still co-located, seller still possesses goods, buyer still possesses payment.
- `visibility: VisibilitySpec::SamePlace` — co-located agents can observe trade attempts.

#### TradeActionPayload

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct TradeActionPayload {
    counterparty: EntityId,
    offered_commodity: CommodityKind,
    offered_quantity: Quantity,
    requested_commodity: CommodityKind,
    requested_quantity: Quantity,
}
```

A `Trade(TradeActionPayload)` variant must be added to the `ActionPayload` enum in `worldwake-sim/src/action_payload.rs`.

#### Co-Location Precondition

Co-location with the counterparty is enforced by specifying the counterparty as `TargetSpec::SpecificEntity(counterparty_id)` in the action's targets, with `Precondition::TargetAtActorPlace(0)` as a precondition. No new constraint variants are needed.

#### Physical Preconditions
- buyer and seller co-located (via `Precondition::TargetAtActorPlace(0)`)
- seller controls the good
- buyer controls the offered coin / barter good
- both are alive and capable of acting (`Precondition::ActorAlive`)

#### Negotiation Procedure
Trade must be resolved through deterministic evaluation of candidate exchange bundles.

For each candidate bundle:
- seller gives `goods`
- buyer gives `coin` (Phase 2 may remain coin-only; barter extension is allowed later)

Each side independently evaluates:
- current state utility from their own beliefs
- post-trade state utility if the candidate bundle executes

The trade succeeds only if the bundle improves or at least meets each side's acceptance rule.

This replaces the earlier hand-wavy "willingness-to-pay / willingness-to-accept formula." The system must actually compare concrete ownership outcomes.

#### Candidate Bundle Search
The search order must be deterministic and bounded.

A compliant Phase 2 implementation may:
1. choose a commodity lot and quantity
2. enumerate affordable coin amounts in a deterministic order
3. let each side accept / reject based on post-trade evaluation
4. settle on the first mutually acceptable amount or terminate with no deal

No global base price is permitted.

### Conservation Mechanism

Trade operates through ownership relation changes. The `possessed_by` and `owned_by` relations of existing lot entities are transferred from seller to buyer (for goods) and buyer to seller (for payment). For partial lot trades, `World::split_lot` is called first, then the split-off portion's relations are transferred. No lots are created or destroyed by trade itself. This is inherently conserving because `split_lot` maintains the conservation invariant.

### LotOperation::Traded

A `Traded` variant must be added to the `LotOperation` enum in `worldwake-core/src/items.rs`. A `ProvenanceEntry` with this operation is appended to each lot that changes ownership through trade. This enables provenance tracking for goods that have passed through trade (e.g., "this bread was traded from merchant X to buyer Y at tick 150").

### Valuation Inputs
Valuation is derived from concrete believed state, not from a price table.

The valuation helper lives in `worldwake-sim` because it needs `&dyn BeliefView` which is defined in that crate. It must not live in `worldwake-ai` so E11 does not depend on E13.

```rust
pub fn evaluate_trade_bundle(
    actor: EntityId,
    belief: &dyn BeliefView,
    needs: Option<&HomeostaticNeeds>,
    wounds: Option<&WoundList>,
    current_coin: Quantity,
    offered: &[(CommodityKind, Quantity)],
    received: &[(CommodityKind, Quantity)],
    local_alternatives: &[(EntityId, CommodityKind, Quantity)],
    demand_memory: Option<&DemandMemory>,
) -> TradeAcceptance
```

`Option` wrappers for `needs` and `wounds` handle non-biological entities. If an agent lacks these components (non-biological entity or E09 not yet complete), the valuation function treats the agent as having no physiological urgency, making them harder to extract value from in negotiation.

Examples of allowed inputs:
- current physiological needs (food / water more valuable when needed)
- current wounds and pain
- current coin holdings
- visible alternative sellers / buyers at the same place
- own inventory abundance / shortage
- recent `DemandMemory`
- `MerchandiseProfile.sale_kinds`
- believed procurement difficulty (travel + source availability known through beliefs)

### E09 Dependency Note

E11 reads the following E09 components for valuation: `HomeostaticNeeds`, `WoundList`. If an agent lacks these components (non-biological entity or E09 not complete), the valuation function treats the agent as having no physiological urgency, making them harder to extract value from in negotiation. Both components are defined in `worldwake-core` (in `needs.rs` and `wounds.rs` respectively), so no cross-system import is needed.

### Trade System Tick Responsibilities

The trade system tick function (replacing noop at dispatch table index 2) has two responsibilities:

1. **Age DemandMemory**: For each agent with both `DemandMemory` and `TradeDispositionProfile`, prune observations older than the retention window (`current_tick.0 - observation.tick.0 > trade_disposition_profile.demand_memory_retention_ticks as u64`).
2. **Record unmet demand**: For agents who attempted to find a trade counterparty but failed (no seller present, no buyer present), append a `DemandObservation` to their `DemandMemory`.

### Merchant Restock Planning Inputs
Merchant restock becomes a grounded candidate goal when all of the following hold:

1. the agent has `MerchandiseProfile`
2. a sale kind in that profile is absent from saleable stock
3. the agent either:
   - has recent `DemandMemory` for that kind, or
   - has explicit sale intent to keep that kind in circulation

This is the key correction: restock is no longer "stock < threshold -> restock."
It is "I am a merchant for this kind, I currently lack stock, and I have concrete reason to believe obtaining it is worthwhile."

### Procurement Paths
Restock is satisfied only through physical procurement:

- travel to source and buy
- travel to source and harvest / craft
- receive delivery from another carrier

There is no magical refill and no stock respawn.

### Substitute Demand
When a preferred good is unavailable or a negotiation fails:

- the buyer inspects other available goods at the current place
- they rank substitutes using their `SubstitutePreferences` component (a `BTreeMap<TradeCategory, Vec<CommodityKind>>` ordered by preference)
- they may buy the substitute if the exchange is beneficial

An agent without `SubstitutePreferences` does not consider substitutes.

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
- `SubstitutePreferences` — on `EntityKind::Agent`

Trade still uses existing coin / inventory ownership components from `worldwake-core`.

## SystemFn Integration
- Implements the trade handler in `SystemDispatch`
- Runs once per tick for active negotiation / exchange actions
- Reads:
  - inventories / ownership
  - coin holdings
  - `HomeostaticNeeds` (from E09, `Option`-wrapped)
  - `WoundList` (from E09, `Option`-wrapped)
  - co-located agents
  - `MerchandiseProfile`
  - `DemandMemory`
  - `TradeDispositionProfile`
  - `SubstitutePreferences`
  - local visible alternatives
- Writes:
  - item / coin ownership relation transfers
  - `LotOperation::Traded` provenance entries
  - trade attempt events
  - `DemandMemory` updates (aging + new observations)

## Cross-System Interactions (Principle 12)
- **E09 -> E11**: needs affect valuation of food / water / rest-related goods (read `HomeostaticNeeds`, `WoundList` components)
- **E10 -> E11**: production and transport create actual procurement paths for restock
- **E11 -> E13**: trade failures, demand memories, sale intent, and visible exchange affordances become candidate-goal evidence
- **E12 -> E11**: wounds / incapacity alter what an agent values and what negotiations they can physically complete

## FND-01 Section H

### Information-Path Analysis
- Trade requires co-location (Principle 7)
- Observed alternatives are local to the place
- Demand memories come from specific observed interactions at specific places and ticks
- Restock planning is based on believed sources and remembered unmet demand, not omniscient global stock checks
- Valuation inputs are read from the agent's own components and `BeliefView`, never from global state

### Positive-Feedback Analysis
- **Successful selling -> more coin -> more procurement ability -> more selling**
- **Scarcity at a place -> more failed purchase attempts -> stronger merchant incentive to restock**

### Concrete Dampeners
- **Coin conservation (primary dampener)**: Total coin in the world is fixed by the conservation invariant. A merchant accumulating coin means other agents have less purchasing power, naturally limiting the merchant's future sales velocity. This is the strongest dampener because it emerges from the conservation invariant itself.
- **Merchant expansion dampeners**:
  - travel time (physical cost to procure goods — Principle 6)
  - carry capacity (load limits restrict how much stock a merchant can transport)
  - limited source stock (production output is bounded by E10 recipe throughput)
  - competition from co-located sellers (alternative sellers reduce any single merchant's sales)
- **Demand-memory dampener**:
  - observations age out based on per-agent `demand_memory_retention_ticks` (TradeDispositionProfile)
  - if no one keeps asking, the signal fades naturally rather than living forever in an abstract market score
  - the aging threshold is a per-agent parameter supporting agent diversity (Principle 11)

### Stored State vs. Derived Read-Model
**Stored (authoritative)**:
- inventories / ownership relations
- coin holdings (as lot quantities)
- `MerchandiseProfile`
- `DemandMemory`
- `TradeDispositionProfile`
- `SubstitutePreferences`
- trade attempt events (in event log)
- `LotOperation::Traded` provenance entries

**Derived (transient read-model)**:
- accepted transaction price (computed at commit time)
- candidate exchange bundles (enumerated during negotiation)
- marginal value of a unit of goods or coin (output of `evaluate_trade_bundle`)
- "does this merchant need to restock now?" (derived from MerchandiseProfile + inventory + DemandMemory)
- `TradeAcceptance` result (output of valuation helper)

## Invariants Enforced
- 9.5: Conservation through trade (ownership transfer, not creation/destruction)
- 9.6: No negative stocks
- 9.7: Ownership transfer requires a valid possession chain
- Principle 2: no base-price table, no scarcity multiplier table, no global price formula
- Principle 6: negotiation consumes time/attention (body_cost_per_tick)
- Principle 7: all negotiation information is local or explicitly remembered
- Principle 8: every positive-feedback loop has a concrete physical dampener
- Principle 11: per-agent TradeDispositionProfile parameters support diversity

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
- [ ] DemandMemory aging prunes observations older than per-agent retention threshold
- [ ] Trade appends `LotOperation::Traded` provenance entry to transferred lots
- [ ] Partial lot trade uses `split_lot` and conserves total quantity
- [ ] Agent without `HomeostaticNeeds` can still trade (no physiological urgency factor)
- [ ] Agent without `SubstitutePreferences` does not consider substitutes

## Acceptance Criteria
- Pricing emerges from bilateral bundle valuation
- Merchant restock is grounded in sale intent plus observed unmet demand
- No hidden global market state exists
- All trades conserve goods and coin
- Substitute demand lets agents adapt to scarcity
- Physical procurement is the only restock path
- All struct types use proper derives and `impl Component`
- All tick/quantity fields use newtype wrappers (`Tick`, `Quantity`, `Permille`)
- Deterministic collections (`BTreeSet`, `BTreeMap`) used where iteration order matters
- Valuation helper lives in `worldwake-sim` with `&dyn BeliefView`

## FND-01 Route Presence Note
E10 now provides explicit `InTransitOnEdge`, so future route-risk logic has a concrete carrier to build on.
This epic still does **not** introduce ambush, interception, or trade-route danger scoring.

## Spec References
- Section 4.5 (trade and pricing)
- Section 7.2 (economic propagation)
- Section 8 (no magical merchant restock)
- Section 9.5 (conservation)
- Section 9.7 (ownership consistency)
- `docs/FOUNDATIONS.md` Principles 2, 3, 6, 7, 8, 11, 12
