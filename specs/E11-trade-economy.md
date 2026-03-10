# E11: Trade & Economy

## Epic Summary
Implement merchant buy/sell actions with negotiation-based pricing, restock planning, and substitute demand. Prices emerge from agent-to-agent negotiation, not lookup tables (Principle 2).

## Phase
Phase 2: Survival & Logistics

## Crate
`worldwake-systems`

## Dependencies
- E08 (actions and scheduler)

## Deliverables

### Trade Actions

- **Negotiate**: buyer and seller negotiate a price for goods
  - Precondition: buyer and seller at same place, seller has goods, buyer has coin
  - Duration: derived from agent profiles (negotiation speed parameter), not hardcoded
  - Effect: if agreement reached, transfer goods from seller to buyer, transfer coin from buyer to seller
  - Both parties must be co-located (Principle 7)

### Negotiation-Based Pricing
Prices emerge from the `Negotiate` action between co-located buyer and seller. There are NO base prices, NO threshold-multiplier tables, NO global price formulas. Per `docs/FOUNDATIONS.md` Principle 2, shortcutting with lookup tables what should emerge from agent interactions violates the No Magic Numbers principle.

**Buyer's willingness-to-pay** is derived from:
- Need urgency for the good (from `HomeostaticNeeds` / `AgentCondition` via `BeliefView`)
- Current coin holdings (can the buyer afford it)
- Number of alternative sellers at the current location (competition drives price down)
- Agent's `UtilityProfile` weights (E13) — how much they value this good relative to others

**Seller's willingness-to-accept** is derived from:
- Remaining stock of the good (fewer units → higher asking price)
- Expected resupply cost (how hard is it to replace this inventory)
- Number of alternative buyers at the current location (competition drives price up)
- Agent's `UtilityProfile` weights (E13) — risk tolerance, greed weight

**Agreement**: trade occurs when buyer's willingness-to-pay ≥ seller's willingness-to-accept. The transaction price is determined by the overlap (e.g., midpoint, or biased by negotiation skill).

**Price is NOT stored** — it is a transient outcome of each negotiation. Different buyer-seller pairs may agree on different prices for the same good at the same location.

### Merchant Restock Planning
- When merchant stock of a good depletes:
  - Generate restock goal (feeds into E13 decision architecture)
  - Plan: identify source → arrange transport → purchase/produce → stock
  - Restock occurs through physical procurement, not magical creation
- Restock triggers when stock falls below a per-agent reorder threshold (from agent profile, not global constant)

### Substitute Demand
- When preferred good unavailable:
  - Agent checks available goods at current location
  - Falls back to per-agent preference ordering (a derived read-model, not stored state)
  - E.g., agent prefers bread but settles for grain if no bread available
  - Preference ordering is part of agent's profile (Principle 11 — different agents prefer different substitutes)
- Demand shifts visible in purchase patterns (observable via event log)

## Component Registration
No new components — trade uses existing coin/item components from worldwake-core.

## SystemFn Integration
- Implements the `SystemId::Trade` handler registered in `SystemDispatch`
- Runs once per tick for all active trade/negotiation actions
- Reads: agent inventories, `HomeostaticNeeds`, coin holdings, co-located agents
- Writes: item/coin transfers, trade events to event log

## Cross-System Interactions (Principle 12)
- **E09 → E11**: Need urgency (from `HomeostaticNeeds`) influences buyer willingness-to-pay
- **E10 → E11**: Production creates goods that become available for trade
- **E11 → E13**: Restock goals feed into decision architecture for merchant planning
- **E11 → E09**: Acquiring food/water via trade enables consumption actions

## FND-01 Section H

### Information-Path Analysis
- Trade requires co-location: buyer and seller must be at the same place (Principle 7)
- Price information: each agent knows only their own willingness-to-pay/accept. They do not know the other party's reservation price.
- Stock information: a seller's remaining stock is observable by co-located agents (they can see what's for sale). Remote stock is unknown.
- Restock planning: merchants plan based on their beliefs about source locations (may be outdated — correct per Principle 10)

### Positive-Feedback Analysis
- **Scarcity → high prices → merchants flock → oversupply → low prices → merchants leave → scarcity**: classic cobweb/boom-bust cycle
- **Monopoly → high prices → wealth accumulation → buying out competitors → stronger monopoly**: a single seller could theoretically corner a market

### Concrete Dampeners
- **Boom-bust cycle**: travel time is the physical dampener. Merchants must physically travel to restock, which takes ticks. By the time a merchant arrives with goods, the scarcity signal is stale — other merchants may have already delivered. Geographic distance and travel time create natural delay that prevents instant market equilibrium (and prevents instant bubbles).
- **Monopoly**: agents have finite carry capacity (`LoadUnits`) and finite coin. A monopolist cannot buy infinite stock. Additionally, production takes time and requires co-location with facilities — a single agent cannot produce everywhere simultaneously. Physical limits on carrying, storage, and production time are the dampeners.

### Stored State vs. Derived Read-Model
**Stored (authoritative)**:
- Item lots (goods in inventory)
- Coin holdings (per-agent)
- Trade events in event log

**Derived (transient read-model)**:
- Price (outcome of each negotiation — never stored as component)
- Willingness-to-pay / willingness-to-accept (computed per negotiation from needs, holdings, alternatives)
- Substitute preference ordering (per-agent profile data, not a separate stored component)
- Available sellers at location (query of co-located agents with inventory)
- Whether restock is needed (current stock vs. per-agent reorder threshold)

## Invariants Enforced
- 9.5: Conservation through trade (goods + coin balanced)
- 9.6: No negative stocks (can't sell what you don't have)
- 9.7: Ownership transfer requires valid possession chain
- Principle 2: No magic numbers — prices emerge from negotiation, not formulas

## Tests
- [ ] Negotiation between co-located buyer and seller produces a trade
- [ ] Trade transfers both goods and coin (conservation)
- [ ] Cannot sell goods not in possession
- [ ] Buyer willingness-to-pay increases with need urgency
- [ ] Seller willingness-to-accept increases as stock decreases
- [ ] Multiple sellers at location drive buyer's willingness-to-pay down
- [ ] Multiple buyers at location drive seller's willingness-to-accept up
- [ ] No trade occurs when willingness-to-pay < willingness-to-accept
- [ ] Merchant generates restock goal when stock depletes
- [ ] No magical restock (goods must arrive physically per spec 8)
- [ ] Substitute demand activates when preferred good unavailable
- [ ] Trade requires both parties at same place
- [ ] No base_price, no threshold-multiplier table, no global price formula
- [ ] Negotiation duration from agent profile, not hardcoded

## Acceptance Criteria
- Pricing emerges from agent-to-agent negotiation (no lookup tables)
- Merchants restock through physical procurement
- No magical creation of goods or money
- Conservation maintained through all trades
- Substitute demand allows agents to adapt to scarcity

## FND-01 Section D — Route Presence Gate

**GATE**: Any route-based trade risk, merchant ambush, caravan interception, or trade-route danger logic in this epic MUST NOT proceed until a concrete route presence model exists in the codebase. This model must support:

- Determining which entities are physically on a route or route segment
- Determining which travelers can physically encounter each other
- Determining which agents can witness route events locally

It is **forbidden** to introduce stored route danger or visibility scores to compensate for missing route presence. All route risk/danger must be derived from concrete entity presence, never from stored abstract scores (Principle 3, `docs/FOUNDATIONS.md`).

See `specs/FND-01-phase1-foundations-alignment.md` Section D for full context.

## Spec References
- Section 4.5 (trade and pricing)
- Section 7.2 (economic propagation: stock, scarcity)
- Section 8 (no magical merchant restock)
- Section 9.5 (conservation)
- Section 9.7 (ownership consistency)
- `docs/FOUNDATIONS.md` Principles 2, 3, 6, 7, 11
- `specs/FND-01-phase1-foundations-alignment.md` Section D (route presence gate)
