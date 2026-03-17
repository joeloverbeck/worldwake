**Status**: DRAFT

# S10: Bilateral Trade Negotiation

## Summary

Replace the fixed 1:1 price trade offers with a belief-driven, multi-round bilateral negotiation protocol. Agents derive reservation prices from concrete state (needs, inventory, wounds, alternatives, demand memory), generate variable-price offers using their existing `TradeDispositionProfile` parameters (`initial_offer_bias`, `concession_rate`), and learn from trade outcomes through `DemandMemory` observations. This activates two currently-unused profile fields, requires no new components or systems, and makes the full 3-agent supply chain golden test pass.

## Why This Exists

### The Problem

`enumerate_trade_payloads` (trade_actions.rs:104-110) hardcodes every trade offer as `offered_quantity: Quantity(1), requested_quantity: Quantity(1)` — one coin for one unit. The `evaluate_trade_bundle` valuation function correctly rejects trades where the post-trade snapshot does not dominate the pre-trade snapshot. When a merchant has high `enterprise_weight`, few units of stock, and existing coins, 1 coin per apple is legitimately insufficient payment.

The consumer retries the same 1-coin offer every 4 ticks (the `negotiation_round_ticks` duration), the merchant rejects every time with `InsufficientPayment`, and the loop never terminates. The consumer cannot bid higher because the system generates no alternative price points.

### Evidence

Diagnosed via instrumentation of the full supply chain golden test (`golden_supply_chain.rs`):

1. **Perception works correctly**: Consumer observes merchant's return with apples at tick ~32 via passive local observation (E14 perception system).
2. **Planning works correctly**: Consumer generates `AcquireCommodity(Apple, SelfConsume)` and finds a trade plan.
3. **Trade initiates correctly**: Consumer starts a trade action targeting the merchant.
4. **Trade is rejected at commit**: `evaluate_for_participant` returns `Reject { reason: InsufficientPayment }` from the merchant's perspective. The merchant values 1 apple more than 1 coin given `enterprise_weight: pm(900)`, `DemandMemory` showing demand for apples, and only 2 units of stock.
5. **Infinite retry loop**: Consumer replans, offers 1 coin again, rejected again. 500 ticks of identical rejections.

### Structural Gap

Two `TradeDispositionProfile` fields exist but are never read in production code:

- **`initial_offer_bias: Permille`** — declared, initialized in tests and profiles, never accessed via `.initial_offer_bias` anywhere.
- **`concession_rate: Permille`** — declared, initialized in tests and profiles, never accessed via `.concession_rate` anywhere.

These were designed for exactly this purpose — parameterizing negotiation behavior — but the implementation was never completed. The trade action skips negotiation entirely: `tick_trade` is a no-op that returns `ActionProgress::Continue`, and `commit_trade` evaluates a single fixed-price bundle at completion.

### Why a Test Hack Is Not the Fix

Adjusting the test setup (giving the merchant fewer coins, removing enterprise weight, starting with more apples) would mask the architectural gap. The same failure will recur whenever:
- A merchant with enterprise focus has scarce stock
- A buyer offers the minimum price for a commodity the seller values
- Any agent has asymmetric urgency relative to the fixed 1:1 price

The fix belongs in the trade action lifecycle, not in test configuration.

## Phase

Post-Phase-2 hardening. No dependency on E14+ epics (perception already implemented and working). Depends on the existing trade action infrastructure in `worldwake-systems/src/trade_actions.rs` and `worldwake-sim/src/trade_valuation.rs`.

## Crates

- `worldwake-systems` (primary — trade action handler modifications)
- `worldwake-sim` (minor — `TradeActionPayload` extension, possible `NegotiationState` in `ActionState`)
- `worldwake-core` (minor — `DemandObservationReason` variant addition)
- `worldwake-ai` (test-only — golden test un-ignore and budget change)

## Design Principles

1. **Concrete state over abstract prices (Principle 3)** — Reservation prices are derived at query time from needs, inventory, wounds, alternatives, and demand memory. No stored "price" component exists.
2. **Actions have duration and cost (Principle 8)** — Each negotiation round occupies one tick. Multi-round negotiation costs real time, during which both agents are occupied and cannot do other things.
3. **Granular outcomes leave aftermath (Principle 9)** — Failed negotiations produce `DemandObservation` records with rejection reasons, which inform future opening offers.
4. **Agent diversity through concrete variation (Principle 20)** — Per-agent `TradeDispositionProfile` parameters (`initial_offer_bias`, `concession_rate`) create distinct bargaining styles (aggressive/patient vs cooperative/quick).
5. **Systems interact through state (Principle 24)** — The trade action writes `DemandObservation` to the agent's memory. Future affordance generation reads it. No cross-system calls.
6. **Belief-only planning (Principle 12)** — Each agent evaluates offers against its own belief view. Neither agent accesses the other's internal valuation.
7. **Locality (Principle 7)** — Negotiation is strictly bilateral and co-located. No global price signals, no market aggregation.

## Architecture

### Overview

The negotiation protocol is **alternating offers with monotonic concession constraint**, implemented within the existing trade action lifecycle (start → tick × N → commit). The key change: `tick_trade` becomes an active negotiation round instead of a no-op, and `commit_trade` executes the agreed price rather than evaluating a fixed one.

### Component 1: Reservation Price Derivation

A pure function that computes an agent's walkaway price from concrete state. No new stored state — this is a derived computation (Principle 3, 25).

```rust
/// Buyer's maximum willingness to pay (in coins) for one unit.
/// Derived from: need urgency, wound severity, local alternatives, coin budget.
fn buyer_reservation_price(
    needs: Option<&HomeostaticNeeds>,
    wounds: Option<&WoundList>,
    commodity: CommodityKind,
    current_coin: Quantity,
    local_alternatives: u32,
) -> Quantity

/// Seller's minimum acceptable price (in coins) for one unit.
/// Derived from: own need for the commodity, current stock, remembered demand.
fn seller_reservation_price(
    needs: Option<&HomeostaticNeeds>,
    commodity: CommodityKind,
    current_stock: Quantity,
    demand_memory: Option<&DemandMemory>,
) -> Quantity
```

**Buyer reservation logic:**
- Base: marginal relief the commodity provides for the agent's most urgent need (hunger/thirst/wound)
- Scarcity adjustment: fewer local alternatives → higher willingness to pay (concrete count, not abstract weight)
- Budget cap: never exceeds `current_coin`
- Zero urgency → reservation of 1 (will buy at minimum, but not more)

**Seller reservation logic:**
- Base: 1 coin (floor — selling is always worth at least 1 coin)
- Self-need adjustment: if the commodity relieves the seller's own needs, reservation rises
- Stock scarcity: fewer units in stock → higher reservation (each unit is more valuable)
- Demand pressure: more remembered demand observations → higher reservation (scarce good, hold for better price)

All inputs are `Permille` or `Quantity` values already available on the agent's belief view. No new components.

### Component 2: Offer Generation via Faratin Concession Curve

Uses the existing `TradeDispositionProfile` fields that are currently unused:

- **`initial_offer_bias: Permille`** → `kappa`: Where in the [reservation, counterparty_reservation] range the agent opens. `pm(0)` = open at own reservation (generous). `pm(1000)` = open at maximum distance from reservation (aggressive).
- **`concession_rate: Permille`** → `beta`: Shape of the concession curve. `pm(0)-pm(499)` = Boulware (slow initial concession, rapid near deadline). `pm(500)` = Linear. `pm(501)-pm(1000)` = Conceder (rapid initial concession, slow near deadline).
- **`negotiation_round_ticks: NonZeroU32`** → base patience (maximum rounds before walking away). Modulated by urgency.

```rust
/// Generate the agent's offer at negotiation round `t`.
/// Uses the Faratin time-dependent concession function.
fn generate_offer(
    role: TradeRole,           // Buyer or Seller
    reservation: Quantity,     // own walkaway price
    opening: Quantity,         // initial offer (from bias + rejection memory)
    round: u32,                // current round (0-indexed)
    deadline: u32,             // effective max rounds (urgency-modulated)
    concession_rate: Permille, // beta: curve shape
) -> Quantity
```

**Urgency modulation**: The effective deadline shrinks with need urgency. A merchant with `negotiation_round_ticks: 8` at hunger `pm(0)` has 8 rounds of patience. The same merchant at hunger `pm(800)` has ~2 rounds. This means the same `TradeDispositionProfile` produces different bargaining behavior depending on concrete physiological state — a naturally patient agent becomes desperate when starving.

**Monotonic concession constraint**: Each successive offer must be weakly more favorable to the counterparty than the previous one. Buyers can only bid up. Sellers can only ask less. This guarantees convergence in finite rounds and prevents oscillation.

### Component 3: Negotiation State in Trade Action

The trade action's `ActionState` stores the negotiation progress:

```rust
/// Stored in ActionState during trade negotiation.
pub struct NegotiationState {
    pub round: u32,
    pub initiator_role: TradeRole,  // Buyer or Seller
    pub initiator_last_offer: Option<Quantity>,
    pub responder_last_offer: Option<Quantity>,
    pub agreed_price: Option<Quantity>,
}

pub enum TradeRole {
    Buyer,
    Seller,
}
```

This is transient action state — it lives only for the duration of the trade action instance and is not a stored component. It is serializable for replay determinism.

### Component 4: Modified Trade Action Lifecycle

#### `enumerate_trade_payloads` (affordance generation)

Currently generates a single 1:1 payload filtered by mutual acceptance. Changed to:

1. Compute buyer's reservation price from belief state.
2. Compute seller's reservation price from belief state (using consumer's belief about seller's state).
3. If buyer's reservation ≥ seller's reservation (zone of possible agreement exists), emit a trade payload with the buyer's opening offer as `offered_quantity`.
4. The payload now represents an *initial* offer, not a final price.

```rust
fn enumerate_trade_payloads(
    _def: &ActionDef,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn RuntimeBeliefView,
) -> Vec<ActionPayload> {
    // ... existing counterparty/place/merchandise validation ...

    let buyer_reservation = buyer_reservation_price(
        view.homeostatic_needs(actor).as_ref(),
        view.wound_list(actor).as_ref(),
        requested_commodity,
        view.commodity_quantity(actor, CommodityKind::Coin),
        count_local_alternatives(view, actor, counterparty, place, requested_commodity),
    );

    // Check zone of possible agreement exists using belief-estimated
    // seller reservation (conservative: assume seller wants at least 1)
    if buyer_reservation < Quantity(1) {
        return Vec::new();  // cannot afford anything
    }

    let disposition = view.trade_disposition_profile(actor)?;
    let opening_offer = derive_opening_offer(
        TradeRole::Buyer,
        buyer_reservation,
        disposition.initial_offer_bias,
        // Shift opening up based on prior rejections with this counterparty
        rejection_count_for(view, actor, counterparty, requested_commodity),
    );

    // Emit single payload with the computed opening offer
    vec![ActionPayload::Trade(TradeActionPayload {
        counterparty,
        offered_commodity: CommodityKind::Coin,
        offered_quantity: opening_offer,
        requested_commodity,
        requested_quantity: Quantity(1),
    })]
}
```

#### `start_trade` (action start)

Initialize `NegotiationState` with round 0, the initiator's opening offer from the payload, and no responder offer yet.

#### `tick_trade` (per-tick negotiation round)

Currently a no-op. Changed to execute one negotiation round:

```
1. Determine whose turn it is (alternating: even rounds = initiator, odd = responder).
2. The responding agent evaluates the current offer:
   a. Compute own reservation price from WorldTxn state.
   b. If offer meets or exceeds reservation → set agreed_price, return Continue.
   c. If own deadline reached → walk away (return error → abort).
   d. Otherwise → compute counter-offer using concession curve, store in state.
3. The proposing agent evaluates the counter-offer on the next tick.
4. Repeat until agreement or deadline.
```

Each round is one tick. The action's total duration is no longer fixed at `negotiation_round_ticks` — it ends early on agreement or walkaway. The `negotiation_round_ticks` value becomes the *maximum* rounds (the agent's base patience).

#### `commit_trade` (trade execution)

Currently evaluates a fixed bundle. Changed to:

1. Check `NegotiationState.agreed_price` exists (both sides accepted).
2. If no agreement → abort (both walked away or deadline).
3. Execute transfer at the agreed price: buyer transfers `agreed_price` coins, seller transfers 1 unit of commodity.
4. Record negotiation outcome in both agents' `DemandMemory`.

### Component 5: Post-Negotiation Learning via DemandMemory

After each negotiation (success or failure), both agents record the outcome:

**New `DemandObservationReason` variant:**
```rust
pub enum DemandObservationReason {
    WantedToBuyButNoSeller,
    WantedToBuyButSellerOutOfStock,
    WantedToBuyButTooExpensive,
    WantedToSellButNoBuyer,
    TradeAgreed,  // NEW: record agreed price as reference
}
```

**On successful trade:**
- Buyer records `TradeAgreed` with `quantity` = agreed price. Future opening offers for this commodity at this place start near the last agreed price.
- Seller records `TradeAgreed` with `quantity` = agreed price.

**On failed negotiation (walkaway/deadline):**
- Buyer records `WantedToBuyButTooExpensive` with `counterparty` = seller. Future opening offers to this seller start higher.
- Seller records `WantedToSellButNoBuyer`. Future reservation prices may soften.

These observations age out via the existing `trade_system_tick` which prunes observations older than `demand_memory_retention_ticks`. No new aging logic needed.

**Rejection-count heuristic for opening offers:**
```rust
fn derive_opening_offer(
    role: TradeRole,
    reservation: Quantity,
    initial_offer_bias: Permille,
    prior_rejections: u32,
) -> Quantity {
    // Base opening from reservation + bias
    let base = /* reservation adjusted by initial_offer_bias */;

    // Each prior rejection shifts opening toward reservation
    // Buyer: bid higher after rejections. Seller: ask less.
    let shift_per_rejection = reservation.0 / 5;  // 20% per rejection
    let total_shift = shift_per_rejection * prior_rejections.min(4);

    match role {
        TradeRole::Buyer => Quantity((base.0 + total_shift).min(reservation.0)),
        TradeRole::Seller => Quantity(base.0.saturating_sub(total_shift).max(1)),
    }
}
```

This means an agent who was rejected 3 times by the same seller will open at a substantially higher price on attempt 4, breaking the infinite-retry loop that is the root cause of the test failure.

### Component 6: Emergent Price Properties

The architecture produces these emergent properties with zero special-case code:

| Scenario | Emergent Behavior |
|----------|-------------------|
| Hungry buyer, surplus seller | Buyer's urgency collapses deadline → rapid concession → seller captures surplus |
| Patient merchant, desperate consumer | Merchant's Boulware curve holds; consumer's urgency forces acceptance |
| Two patient agents, fair value | Slow convergence toward midpoint of reservation prices |
| Monopoly seller, no alternatives | Buyer's reservation rises (scarcity) → willing to pay more |
| Competing sellers at same place | Buyer's reservation falls (alternatives) → harder to extract high price |
| Repeated rejection by same seller | Buyer's opening offer ratchets up via rejection memory |
| Merchant restocks abundantly | Seller reservation falls (more stock) → cheaper prices |

All of these emerge from the intersection of concrete agent state — no pricing rules, no market maker, no equilibrium computation.

## FND-01 Section H Analysis

### H.1: Information-Path Analysis

**How does price information reach agents?**

1. Each agent's reservation price is derived locally from its own belief view (needs, inventory, wounds, demand memory). No information travels.
2. The counterparty's offers arrive through the negotiation protocol — direct co-located communication (Principle 7).
3. Trade outcomes are recorded in `DemandMemory` as observations. These are local to the agent and age out.
4. No global price signals exist. An agent at Place A does not learn what prices were agreed at Place B unless it travels there and observes or is told (future E15 rumor system).

**Information path for "what price should I offer?":**
```
Agent's own needs (authoritative) → marginal relief computation
Agent's own inventory (authoritative) → coin budget
Agent's belief about local sellers (belief store) → alternative count
Agent's demand memory observations (belief store) → rejection history
→ All local, all belief-based, no global queries
```

### H.2: Positive-Feedback Analysis

**Loop 1: Rejection → Higher Offer → Acceptance → Higher Reference Price → Higher Future Offers**
- If a buyer pays 3 coins for an apple, it records this. Next time it opens at ~3 coins. If the seller's reservation hasn't changed, this is fine. But if many buyers record high prices, future offers start high, potentially inflating prices.

**Loop 2: Seller Demand Memory → Higher Reservation → Higher Rejection Rate → More "TooExpensive" Observations → Even Higher Reservations**
- A seller who rejects many buyers accumulates `WantedToBuyButTooExpensive` observations (from buyers). If the seller's reservation depends on demand observations, this could spiral.

### H.3: Concrete Dampeners

**Dampener for Loop 1 (price inflation):**
- `demand_memory_retention_ticks` causes old price observations to age out. High prices from a scarcity period are forgotten, returning opening offers to state-derived levels.
- Buyer's reservation price is capped by `current_coin` (finite resource). A buyer cannot pay more than it has.
- New sellers entering the place create alternatives, lowering buyer reservation prices.
- The concession curve converges toward reservation, not away from it. Monotonic concession prevents upward price spirals within a single negotiation.

**Dampener for Loop 2 (seller overpricing):**
- Seller's `demand_memory_retention_ticks` ages out old demand observations, decaying the demand pressure.
- Buyers walking away means the seller's stock sits unsold. If the commodity is needed for the seller's own survival (e.g., the merchant gets hungry), the seller's reservation drops.
- Alternative sellers attract buyers, reducing demand observations for the overpricing seller.
- The seller's reservation depends on `current_stock` — as stock accumulates without sales, the surplus grows and reservation price drops.

All dampeners are physical world processes (resource depletion, memory decay, alternative competition, physiological need), not numeric clamps.

### H.4: Stored State vs. Derived Read-Model

**Authoritative stored state:**
- `TradeDispositionProfile` (per-agent component) — negotiation personality parameters
- `DemandMemory.observations` (per-agent component) — historical trade observations
- `HomeostaticNeeds` (per-agent component) — current physiological state
- `NegotiationState` (transient `ActionState` during trade action) — current round, offers
- `TradeActionPayload` (action instance data) — the initial offer

**Derived computations (never stored):**
- Reservation prices — always recomputed from current state
- Opening offers — derived from reservation + bias + rejection count
- Concession curve values — derived from round number + profile parameters
- Effective deadline — derived from `negotiation_round_ticks` × urgency factor

No derived value is promoted to authoritative state.

## SystemFn Integration

No new system function. The negotiation protocol operates entirely within the existing trade action handler lifecycle (`start_trade`, `tick_trade`, `commit_trade`, `abort_trade`). The perception system and trade system continue to run unchanged.

**System execution order (unchanged):**
```
Tick N:
  1. Drain input queue (AI decisions, human input)
  2. Progress active actions (tick_trade runs negotiation rounds)
  3. Run systems: needs → production → trade_system_tick → combat → perception
  4. AI decision cycle (generates new trade actions if needed)
```

## Component Registration

**Modified components (no new registration):**
- `DemandObservationReason` — add `TradeAgreed` variant
- `ActionState` — extend to support `NegotiationState` serialization
- `TradeActionPayload` — semantics change (offered_quantity is opening offer, not final price)

**No new components.** All state either exists already or is transient action state.

## Cross-System Interactions (Principle 24)

All interactions are state-mediated:

| Producer System | State Written | Consumer System | State Read |
|----------------|---------------|-----------------|------------|
| Trade action (commit) | `DemandObservation(TradeAgreed)` | Future affordance generation | Opening offer derivation |
| Trade action (abort) | `DemandObservation(TooExpensive)` | Future affordance generation | Rejection count for higher opening |
| Needs system | `HomeostaticNeeds` | Trade action (tick) | Urgency modulation of deadline |
| Perception system | `AgentBeliefStore` | Trade affordance enumeration | Belief about seller inventory/alternatives |
| Trade system tick | Pruned `DemandMemory` | Trade action (tick) | Aged-out observations no longer affect offers |

No system calls another system's logic. The trade action reads world state to make local decisions.

## Acceptance Criteria

### Functional

1. Agents generate variable-price trade offers based on concrete state (needs, inventory, alternatives, demand memory).
2. `TradeDispositionProfile.initial_offer_bias` and `.concession_rate` are actively read and affect offer generation.
3. Multi-round negotiation converges to agreement when a zone of possible agreement exists.
4. Agents walk away when no zone of possible agreement exists (buyer's reservation < seller's reservation).
5. Failed negotiations record `DemandObservation` outcomes that inform future opening offers.
6. Observations age out via existing `trade_system_tick` — no new aging logic.

### Golden Test Gate

7. `test_full_supply_chain` passes with `PlanningBudget::default()` (512 expansions, beam width 8) — the consumer successfully trades with the merchant at a mutually acceptable price.
8. `test_full_supply_chain_replay` passes — deterministic replay is preserved.
9. All existing golden tests pass unchanged: `cargo test -p worldwake-ai` — no regressions. The segment tests (`test_consumer_trade_with_traces`, `test_merchant_restock_with_traces`) continue to pass because 1:1 trades remain acceptable when the seller has low enterprise weight and surplus stock.
10. `cargo test --workspace && cargo clippy --workspace` — clean.

### Invariants

11. `PlanningBudget::default()` is not modified.
12. Deterministic replay holds — same seed, same inputs produce identical event logs and negotiation outcomes.
13. Conservation invariants hold — coins and commodities are neither created nor destroyed by negotiation.
14. No stored "price" or "market rate" component exists — all prices are derived at query time.
15. The negotiation protocol is symmetric under agent swap — a human-controlled agent using the same `TradeDispositionProfile` would negotiate identically to an AI-controlled one (Principle 17).

## Files to Touch

- `crates/worldwake-systems/src/trade_actions.rs` (major — negotiation protocol in tick_trade, reservation price functions, modified enumerate_trade_payloads, post-negotiation learning)
- `crates/worldwake-sim/src/trade_valuation.rs` (minor — expose helper functions for reservation price derivation, or keep self-contained in trade_actions)
- `crates/worldwake-sim/src/action_payload.rs` (minor — `TradeActionPayload` semantics documentation)
- `crates/worldwake-sim/src/action_state.rs` (minor — `NegotiationState` serialization support)
- `crates/worldwake-core/src/trade.rs` (minor — add `TradeAgreed` to `DemandObservationReason`)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` (test — remove `#[ignore]`, switch to `PlanningBudget::default()`, update comments)

## Out of Scope

- Centralized market/auction mechanisms
- Global price indexes or equilibrium computation
- Multi-commodity bundle negotiation (future extension — currently 1 commodity per trade)
- Barter (commodity-for-commodity without coin intermediary)
- `S06-commodity-opportunity-valuation` integration (future — indirect commodity utility)
- Changes to `PlanningBudget` defaults
- Changes to the perception system
- New golden tests beyond un-ignoring the existing full supply chain test

## Test Plan

### Unit Tests (in trade_actions.rs)

1. `buyer_reservation_price` returns higher values for higher need urgency
2. `buyer_reservation_price` is capped by available coins
3. `buyer_reservation_price` decreases with more local alternatives
4. `seller_reservation_price` increases with fewer stock units
5. `seller_reservation_price` increases with more demand observations
6. `generate_offer` with Boulware rate concedes slowly then rapidly
7. `generate_offer` with Conceder rate concedes rapidly then slowly
8. `generate_offer` with Linear rate concedes uniformly
9. Monotonic concession constraint: buyer offers never decrease, seller asks never increase
10. `derive_opening_offer` shifts with rejection count
11. Urgency modulation: effective deadline shrinks with higher need
12. Negotiation converges when buyer reservation > seller reservation
13. Negotiation fails (walkaway) when buyer reservation < seller reservation
14. Post-negotiation DemandObservation is recorded with correct reason

### Integration Tests (golden tests)

15. `test_full_supply_chain` — un-ignored, passes with default budget
16. `test_full_supply_chain_replay` — un-ignored, deterministic
17. All existing golden tests pass unchanged (no regressions)

### Commands

```bash
cargo test -p worldwake-systems -- trade   # unit tests
cargo test -p worldwake-ai --test golden_supply_chain  # golden tests
cargo test -p worldwake-ai  # all AI golden tests
cargo test --workspace && cargo clippy --workspace  # full suite
```

## References

- Faratin, P., Sierra, C., & Jennings, N. (1998). "Negotiation Decision Functions for Autonomous Agents" — time-dependent concession tactics, Boulware/Conceder/Linear strategies
- Rubinstein, A. (1982). "Perfect Equilibrium in a Bargaining Model" — alternating offers protocol
- Rosenschein, J. & Zlotkin, G. (1994). *Rules of Encounter* — monotonic concession protocol
- Zeng, D. & Sycara, K. (1998). "Bayesian Learning in Negotiation" — rejection-driven belief updates
