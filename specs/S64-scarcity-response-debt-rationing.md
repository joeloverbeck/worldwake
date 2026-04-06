# S64: Scarcity Response — Debt, Rationing, and Substitution

## Summary

Add downstream human responses to scarcity: substitution, rationing, debt/credit, and prioritized distribution. Currently stockout produces empty shelves and failed purchase attempts but no deeper behavioral adaptation. This spec turns existing logistics pressure into social pressure — agents borrow, substitute, ration, hoard, and refuse service based on concrete stock state and social relationships.

## Phase

Phase 7: Consequence Carriers

## Status

Draft

## Crates

- `worldwake-core` (debt, ration, commitment types)
- `worldwake-systems` (scarcity actions, rationing system)
- `worldwake-ai` (scarcity-driven goal generation)

## Dependencies

- E09 (needs/metabolism) — completed
- E10 (production/transport) — completed
- S04 (merchant selling) — completed
- S10 (bilateral trade) — completed
- S62 (boundary processes) — provides the upstream shortage pressure that makes scarcity responses necessary

## Design Goals

- Scarcity response emerges from concrete stock levels and demand, not a "scarcity event" trigger
- Debt is a social artifact with creditor, debtor, amount, and due date — not an abstract score
- Rationing is an institutional action with priority lists, not a global distribution algorithm
- Substitution uses existing `SubstitutePreferences` (already implemented) extended with action support
- Hoarding and refusal are rational responses to scarcity, driven by agent beliefs about future supply

## Non-Goals

- Price dynamics or market clearing mechanisms — prices remain concrete ask/bid in trade negotiation
- Currency or monetary system — goods are bartered or traded directly
- Formal banking or lending institutions — debt is interpersonal or institutional, not financial
- Insurance or futures contracts — deferred
- Global price simulation — explicitly forbidden (P3)

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P1 (Emergence) | Scarcity responses emerge from actual stock depletion, not scripted shortage events |
| P3 (Concrete State) | Debt records have creditor, debtor, commodity, quantity, due tick — not abstract credit scores |
| P5 (Carriers of Consequence) | Debt creates future behavioral pressure; rationing reshapes daily routines; hoarding tightens supply for others |
| P7 (Locality) | Rationing is local to the rationing authority. Agents discover shortage by failing to buy, not by global notification |
| P8 (Preconditions) | Borrowing requires a willing creditor. Rationing requires institutional authority. Substitution requires substitute availability |
| P10 (Aftermath) | Unpaid debts create social tension. Hoarding creates further shortage. Rationing creates resentment and priority disputes |
| P17 (Violated Expectation) | Expected availability → empty shelf → substitution/borrowing cascade |
| P25 (Social Artifacts) | Debt records and ration orders are social artifacts with lifecycle |

## Deliverables

### 1. Debt Record

```rust
/// A concrete debt obligation between entities.
/// Stored as a social artifact (S45 pattern) or in RecordData.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DebtRecord {
    pub debt_id: DebtId,
    pub creditor: EntityId,
    pub debtor: EntityId,
    pub commodity: CommodityKind,
    pub quantity: u32,
    /// When the debt was incurred.
    pub incurred_tick: Tick,
    /// When repayment is expected. None = open-ended.
    pub due_tick: Option<Tick>,
    /// Why the debt exists.
    pub basis: DebtBasis,
    pub state: DebtState,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct DebtId(pub u64);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DebtBasis {
    /// Borrowed goods during shortage.
    Loan,
    /// Received goods or service on credit.
    Credit,
    /// Aid received during emergency.
    AidReceived,
    /// Institutional obligation (tax, levy, tithe).
    Obligation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DebtState {
    /// Debt is active and repayment expected.
    Outstanding,
    /// Partially repaid.
    PartiallyRepaid { amount_repaid: u32 },
    /// Fully repaid.
    Repaid { repaid_tick: Tick },
    /// Creditor forgave the debt.
    Forgiven { forgiven_tick: Tick },
    /// Debt overdue and unpaid.
    Defaulted { since_tick: Tick },
}
```

### 2. Ration Order

```rust
/// An institutional order to ration a commodity.
/// Issued by an office-holder, applies to entities under their jurisdiction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RationOrder {
    pub order_id: RationOrderId,
    /// The office issuing the ration order.
    pub issuing_office: EntityId,
    /// What commodity is being rationed.
    pub commodity: CommodityKind,
    /// Maximum quantity per recipient per distribution period.
    pub ration_amount: u32,
    /// How often distribution occurs (in ticks).
    pub distribution_period_ticks: u32,
    /// Priority ordering for distribution.
    pub priority_list: Vec<RationPriority>,
    pub effective_tick: Tick,
    /// When the ration order expires. None = indefinite.
    pub expires_tick: Option<Tick>,
    pub state: RationOrderState,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct RationOrderId(pub u64);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RationPriority {
    pub category: PriorityCategory,
    /// Higher rank = served first.
    pub rank: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PriorityCategory {
    /// Guards and patrol agents.
    Guards,
    /// Sick and wounded.
    SickAndWounded,
    /// Children and elderly (if modeled).
    Vulnerable,
    /// Office-holders.
    Officials,
    /// General population.
    General,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RationOrderState {
    Active,
    Suspended,
    Expired,
}
```

### 3. Scarcity Response Profile

```rust
/// Per-agent profile governing scarcity behavior.
/// Registered on EntityKind::Agent. Universal with defaults.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScarcityResponseProfile {
    /// Willingness to lend goods to others (0 = never, 1000 = freely).
    pub lending_willingness: Permille,
    /// Willingness to borrow (some agents prefer going without).
    pub borrowing_willingness: Permille,
    /// At what stock level the agent begins hoarding (fraction of normal consumption).
    pub hoard_threshold: Permille,
    /// Willingness to accept substitutes.
    pub substitution_tolerance: Permille,
}

impl Default for ScarcityResponseProfile {
    fn default() -> Self {
        Self {
            lending_willingness: Permille::new(500),
            borrowing_willingness: Permille::new(500),
            hoard_threshold: Permille::new(300),
            substitution_tolerance: Permille::new(700),
        }
    }
}
```

### 4. New Actions

#### `borrow`
- **Preconditions**: Actor needs a commodity and cannot buy it. A potential creditor is co-located and has stock. Creditor's `lending_willingness` exceeds threshold.
- **Duration**: Short (negotiation).
- **Effect**: Transfers commodity from creditor to debtor. Creates `DebtRecord` with basis `Loan`. Both parties update beliefs.
- **Domain**: `ActionDomain::Trade`

#### `repay`
- **Preconditions**: Actor has an outstanding `DebtRecord`. Actor has sufficient quantity of the owed commodity. Creditor is co-located or at a reachable place.
- **Duration**: Short (transfer).
- **Effect**: Transfers commodity to creditor. Updates `DebtState` to `Repaid` or `PartiallyRepaid`. Debt record superseded in institutional records.
- **Domain**: `ActionDomain::Trade`

#### `substitute_purchase`
- **Preconditions**: Actor's preferred commodity is unavailable. Actor has `SubstitutePreferences` listing alternatives. Substitute is available at the current market.
- **Duration**: Same as regular purchase (uses existing trade action with substitute commodity).
- **Effect**: Purchases substitute. Actor's need is partially satisfied (substitutes may have lower satisfaction value). Updates demand memory.
- **Domain**: `ActionDomain::Trade`

#### `issue_ration_order`
- **Preconditions**: Actor holds an office with authority over resource distribution. Stock of a commodity is below a critical threshold observable by the actor.
- **Duration**: Short (institutional action).
- **Effect**: Creates `RationOrder` as an institutional record. Posted at the office for agent observation.
- **Domain**: `ActionDomain::Social`

#### `distribute_rations`
- **Preconditions**: An active `RationOrder` exists. Actor holds the relevant office or is designated distributor. Stock is available for distribution.
- **Duration**: Medium (distribution process).
- **Effect**: Distributes commodity in priority order up to `ration_amount` per recipient. Creates transfer records. Updates stock.
- **Domain**: `ActionDomain::Trade`

#### `refuse_sale`
- **Preconditions**: Actor is a merchant with low stock. Buyer is requesting purchase. Actor believes stock will not be replenished soon.
- **Duration**: Short.
- **Effect**: Rejects the purchase. Buyer's `DemandMemory` records `WantedToBuyButSellerRefused`. May trigger buyer to seek alternatives.
- **Domain**: `ActionDomain::Trade`

#### `hoard`
- **Preconditions**: Actor observes or believes shortage is imminent. Actor has access to a commodity source. Actor's stock is below `hoard_threshold`.
- **Duration**: Same as regular purchase but actor buys above normal consumption needs.
- **Effect**: Purchases extra quantity. Reduces available stock for others. Agent stores excess.
- **Domain**: `ActionDomain::Trade`

#### `request_aid`
- **Preconditions**: Actor's need is critical (hunger/thirst at dangerous levels). Actor cannot buy, borrow, or substitute. An institution or wealthy agent is co-located.
- **Duration**: Short (social request).
- **Effect**: Target may respond with `distribute_rations`, `borrow` (creating a debt), or refuse. Outcome depends on target's profile and beliefs.
- **Domain**: `ActionDomain::Social`

### 5. Goal Kinds

```rust
GoalKind::BorrowCommodity { commodity: CommodityKind, from: Option<EntityId> }
GoalKind::RepayDebt { debt_id: DebtId }
GoalKind::SubstitutePurchase { original: CommodityKind, substitute: CommodityKind }
GoalKind::IssueRationOrder { commodity: CommodityKind }
GoalKind::DistributeRations { order_id: RationOrderId }
GoalKind::Hoard { commodity: CommodityKind }
GoalKind::RequestAid
```

**Candidate generation**: When `DemandMemory` records repeated purchase failures (`WantedToBuyButSellerOutOfStock`), candidate generation produces `SubstitutePurchase`, `BorrowCommodity`, and `RequestAid` goals in priority order based on `ScarcityResponseProfile`. Office-holders generate `IssueRationOrder` when they observe critical stock levels. Agents with outstanding debts generate `RepayDebt` goals. Agents with low stock and high scarcity beliefs generate `Hoard`.

### 6. DemandMemory Extension

Add a new `DemandObservation` variant:

```rust
pub enum DemandObservation {
    // ... existing variants ...
    /// Seller refused the sale (hoarding or prioritizing others).
    WantedToBuyButSellerRefused {
        seller: EntityId,
        place: EntityId,
        commodity: CommodityKind,
        tick: Tick,
    },
}
```

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: Stockout currently produces failed purchases and nothing more. No debt, no rationing, no behavioral adaptation, no social pressure from shortage.

2. **New entities/relations/records**: `DebtRecord`, `RationOrder`, `ScarcityResponseProfile` (component), `DemandObservation::WantedToBuyButSellerRefused`.

3. **Actions that mutate them**: `borrow` (creates debt + transfers goods), `repay` (resolves debt), `substitute_purchase` (uses existing trade with substitute), `issue_ration_order` (creates order), `distribute_rations` (transfers goods in priority order), `refuse_sale` (rejects purchase), `hoard` (over-purchases), `request_aid` (social request).

4. **Information production and travel**: Ration orders are posted at offices — locally observable. Debt records are known to creditor and debtor. Shortage beliefs propagate through observation and tell. No global shortage notification.

5. **Conserved quantities**: All commodity transfers (borrow, repay, ration, hoard) follow existing item conservation. Debt records are informational state tracking an obligation, not creating goods.

6. **Scarce capacities and contention**: Rationed commodities are distributed in priority order — lower-priority agents may receive nothing. Multiple borrowers may compete for the same creditor's stock. Hoarding reduces supply for others.

7. **Partial failures and aftermath**: Borrow attempt refused → try substitution or request aid. Ration distribution runs out before reaching general population. Debt defaults → social tension (feeds into S65 grudges). Hoard depletes stock → others face shortage.

8. **Positive feedback loops**: Shortage → hoarding → deeper shortage → more hoarding. Dampener: finite agent purchasing power, finite goods to hoard, rationing limits individual consumption, social pressure against hoarding (if observed).

9. **Physical dampeners**: Agent carrying capacity limits hoarding. Merchant stock is finite. Rationing limits per-agent consumption. Debt repayment eventually returns goods to circulation. Alternative supply through boundary channels (S62).

10. **Agent learning**: `DemandMemory` records purchase failures and seller refusals. Agents update shortage beliefs from repeated failures. Institutions observe stock levels to trigger rationing.

11. **How agents can be wrong**: Agent hoards based on false shortage rumor. Agent borrows from unreliable creditor. Ration order issued when supply is about to recover. Substitution preference outdated.

12. **Lifecycle states**: DebtState: Outstanding → PartiallyRepaid → Repaid / Forgiven / Defaulted. RationOrderState: Active → Suspended → Expired.

13. **Temporal resolution**: Scarcity actions are agent-driven (no per-tick scarcity system). Ration distribution occurs at `distribution_period_ticks` intervals when the distributor acts. Debt due dates are tick-based.

14. **Boundary conditions**: Boundary inflow failure (S62) creates the upstream shortage pressure. Internal scarcity response does not interact directly with boundary systems — it responds to local stock state.

15. **Derived views**: None. Debt records, ration orders, and stock levels are authoritative.

16. **Causal records**: Borrow/repay events logged. Ration distribution logged (who received, how much). Sale refusal logged. Hoard purchases logged as regular trade events.

17. **Target patterns**: Expected bread inflow fails → baker shifts to barley substitute → some households buy, some borrow, some steal. Office treasury rations grain to guards and sick first. Merchant prefers trusted debtors under shortage.

18. **Save/load and replay**: All components and records are standard ECS. Deterministic.

## SystemFn Integration

No new system tick function. All scarcity responses are action-driven through the agent decision pipeline. Ration distribution timing is governed by the distributor agent acting on their `RationOrder` obligation.

Debt default detection (checking `due_tick` against current tick) can be handled in the overdue-expectations check from S59, or as a simple component-scan during the belief-update phase.

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `ScarcityResponseProfile` | Agent | Universal | `Default` — all agents respond to scarcity with individual variation |

`ScarcityResponseProfile` added to `AgentDef` with `unwrap_or_default()` in `spawn_agent()`.

`DebtRecord` and `RationOrder` are runtime-generated state stored in `RecordData` on institutional entities, not scenario-configured.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Trade (E10, S04, S10) | Substitution and refusal modify trade behavior; borrow/repay are trade-adjacent transfers | State-mediated |
| Needs (E09) | Hunger/thirst pressure drives scarcity urgency; substitution partially satisfies needs | State-mediated |
| Boundary (S62) | Inflow failure creates the shortage that triggers scarcity responses | State-mediated |
| Institutions (E16) | Office-holders issue ration orders; institutional stock used for distribution | State-mediated |
| Expectations (S59) | Debt repayment creates expectations; default triggers obligation violation | State-mediated |
| Crime (E17) | Theft becomes more likely under scarcity pressure (existing theft motivation + heightened need) | State-mediated |

## Profile-Driven Parameters

`ScarcityResponseProfile` is per-agent (scenario-configurable):
- `lending_willingness`: generous agents lend freely, selfish agents refuse
- `borrowing_willingness`: proud agents prefer going without
- `hoard_threshold`: anxious agents hoard early, trusting agents wait longer
- `substitution_tolerance`: picky agents refuse substitutes, pragmatic agents accept readily

Existing `SubstitutePreferences` (per-agent, already implemented) controls which commodities substitute for others.
