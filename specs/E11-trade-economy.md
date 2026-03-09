# E11: Trade & Economy

## Epic Summary
Implement merchant buy/sell actions, scarcity-driven pricing, restock planning, debt/contracts, and substitute demand.

## Phase
Phase 2: Survival & Logistics

## Crate
`worldwake-systems`

## Dependencies
- E08 (actions and scheduler)

## Deliverables

### Trade Actions

- **Buy**: buyer agent acquires goods from seller
  - Precondition: seller has goods, buyer has sufficient coin
  - Effect: transfer goods from seller to buyer, transfer coin from buyer to seller
  - Duration: 2-5 ticks (negotiation)
  - Both parties must be at same place

- **Sell**: seller agent offers goods to buyer
  - Mirror of buy from seller's perspective
  - Same mechanics, different initiator

### Scarcity-Driven Pricing
- Base price per good type
- Price modifier based on stock level:
  - Abundant (>150% typical stock): price * 0.7
  - Normal (50-150%): base price
  - Scarce (<50%): price * 1.5
  - Critical (<10%): price * 3.0
- Price also affected by demand (number of recent purchase attempts)
- Price stored per merchant, not globally

### Merchant Restock Planning
- When merchant stock of a good depletes:
  - Generate restock goal (feeds into E13 decision architecture)
  - Plan: identify source → arrange transport → purchase/produce → stock
  - Restock occurs through physical procurement, not magical creation
- Restock triggers when stock falls below reorder threshold

### Debt & Contract System
- `Contract` entity with:
  - parties: Vec<EntityId>
  - terms: ContractTerms (deliver X goods by tick Y for Z coin)
  - status: Active | Fulfilled | Breached | Cancelled
- IOU creation: when buyer lacks full payment
- Delivery contracts: promise to deliver goods by deadline
- Contract tracking: check fulfillment conditions each tick

### Insolvency Detection
- Agent is insolvent when: debts > assets + expected income
- Insolvency triggers: cannot take new debt, reputation decrease
- Does not magically create money

### Substitute Demand
- When preferred good unavailable:
  - Agent seeks substitutes (e.g., grain instead of bread, apples instead of grain)
  - Substitute mapping per good type with preference ordering
  - Demand shifts visible in purchase patterns

## Invariants Enforced
- 9.5: Conservation through trade (goods + coin balanced)
- 9.6: No negative stocks (can't sell what you don't have)
- 9.7: Ownership transfer requires valid possession chain

## Tests
- [ ] Price increases when stock is low
- [ ] Price decreases when stock is abundant
- [ ] Merchant generates restock plan when stock depletes
- [ ] No magical restock (goods must arrive physically per spec 8)
- [ ] Trade transfers both goods and coin (conservation)
- [ ] Cannot sell goods not in possession
- [ ] Contracts track fulfillment correctly
- [ ] Insolvency detected when debts exceed assets
- [ ] Substitute demand activates when preferred good unavailable
- [ ] Trade requires both parties at same place

## Acceptance Criteria
- Dynamic pricing responds to real supply/demand
- Merchants restock through physical procurement
- Debt system tracks obligations
- No magical creation of goods or money

## Spec References
- Section 4.5 (trade and pricing)
- Section 7.2 (economic propagation: stock, scarcity, prices, debt)
- Section 8 (no magical merchant restock)
- Section 9.5 (conservation)
- Section 9.7 (ownership consistency)
