# S10BILTRANEG-002: Reservation price derivation functions

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-systems` (new pure functions in trade_actions.rs)
**Deps**: S10BILTRANEG-001

## Problem

The bilateral trade negotiation protocol requires each agent to compute a walkaway price — the maximum a buyer will pay or the minimum a seller will accept. Currently no such computation exists; the trade system uses hardcoded `Quantity(1)` for all offers. Without reservation prices, the negotiation protocol (ticket 005) and affordance generation (ticket 004) cannot compute meaningful offers.

## Assumption Reassessment (2026-04-02)

1. `HomeostaticNeeds` at `crates/worldwake-core/src/needs.rs:8` has fields `hunger`, `thirst`, `fatigue`, `bladder`, `dirtiness` — all `Permille`. These are the urgency inputs for buyer reservation.
2. `WoundList` at `crates/worldwake-core/src/wounds.rs:64` wraps `Vec<Wound>`. Available via `txn.get_component_wound_list(actor)` during execution and via `wounds_for(view, actor)` helper during affordance generation (returns `Option<WoundList>`).
3. `DemandMemory` at `crates/worldwake-core/src/trade.rs:63` wraps `Vec<DemandObservation>`. Available via `txn.get_component_demand_memory(actor)` and `demand_memory_for(view, actor)`.
4. `CommodityKind` at `crates/worldwake-core/src/items.rs:9-21` includes `Apple`, `Grain`, `Bread`, `Water`, `Firewood`, `Sword`, `Bow`, `Medicine`, `Coin`, `Waste`.
5. `Quantity(pub u32)` supports basic arithmetic. `Permille` has `.value() -> u16`, `saturating_add`, `saturating_sub`. No `Quantity * Permille` operation exists, but integer arithmetic via `.0` and `.value()` is the established pattern.
6. `local_trade_alternatives` at `trade_actions.rs:356-383` returns `Vec<(EntityId, CommodityKind, Quantity)>` — this is the existing pattern for counting alternatives. The spec proposes a simpler `count_local_alternatives` that returns `u32`.
7. The functions are pure (no mutation), so they can be tested independently of the action lifecycle.

## Architecture Check

1. Pure functions that derive price from concrete state (Principle 3) — no stored "price" component, no abstract scores. Reservation prices are recomputed every time they are needed, from the agent's current needs, inventory, wounds, alternatives, and demand memory.
2. No backward-compatibility shims. These are new functions added alongside existing trade infrastructure.

## Verification Layers

1. Buyer reservation scales with urgency -> focused unit test (needs → reservation mapping)
2. Buyer reservation capped by coin balance -> focused unit test
3. Buyer reservation decreases with alternatives -> focused unit test
4. Seller reservation increases with scarcity -> focused unit test
5. Seller reservation increases with demand pressure -> focused unit test
6. Single-layer ticket (pure function tests, no runtime integration).

## What to Change

### 1. Implement `buyer_reservation_price` in `trade_actions.rs`

```rust
fn buyer_reservation_price(
    needs: Option<&HomeostaticNeeds>,
    wounds: Option<&WoundList>,
    commodity: CommodityKind,
    current_coin: Quantity,
    local_alternatives: u32,
) -> Quantity
```

Logic:
- Compute marginal relief: map `commodity` to the need it satisfies (Apple/Bread → hunger, Water → thirst, Medicine → wounds). Use the current need level as urgency.
- Scarcity adjustment: if `local_alternatives == 0`, multiply base by ~2. Each alternative reduces willingness proportionally.
- Budget cap: result ≤ `current_coin`.
- Floor: if no urgency, return `Quantity(1)` (will buy at minimum).
- All arithmetic uses `u32` and `Permille::value()` — no floats.

### 2. Implement `seller_reservation_price` in `trade_actions.rs`

```rust
fn seller_reservation_price(
    needs: Option<&HomeostaticNeeds>,
    commodity: CommodityKind,
    current_stock: Quantity,
    demand_memory: Option<&DemandMemory>,
) -> Quantity
```

Logic:
- Base: `Quantity(1)` (selling is always worth at least 1 coin).
- Self-need: if the commodity relieves the seller's own needs, raise reservation proportionally to the need level.
- Stock scarcity: fewer units → higher reservation (e.g., `base * (4 / current_stock.0.max(1))`).
- Demand pressure: count recent `DemandObservation` entries for this commodity. More demand → higher reservation.
- All arithmetic uses `u32` — no floats, no magic numbers that aren't derivable from concrete state.

### 3. Add `count_local_alternatives` helper

```rust
fn count_local_alternatives(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    excluded_counterparty: EntityId,
    place: EntityId,
    commodity: CommodityKind,
) -> u32
```

Counts how many other sellers at `place` (excluding `actor` and `excluded_counterparty`) offer `commodity`. Built on the existing `local_trade_alternatives` pattern but returns only a count.

## Files to Touch

- `crates/worldwake-systems/src/trade_actions.rs` (modify — add `buyer_reservation_price`, `seller_reservation_price`, `count_local_alternatives`)

## Out of Scope

- Concession curve / offer generation (ticket 003)
- Integration with `enumerate_trade_payloads` (ticket 004)
- Integration with `tick_trade` negotiation rounds (ticket 005)
- `rejection_count_for` helper (ticket 004, used by affordance generation)

## Acceptance Criteria

### Tests That Must Pass

1. `buyer_reservation_price` returns higher values for higher hunger when commodity is Apple
2. `buyer_reservation_price` never exceeds `current_coin`
3. `buyer_reservation_price` returns lower values with more alternatives
4. `buyer_reservation_price` returns `Quantity(1)` when no needs and no wounds
5. `seller_reservation_price` returns higher values with fewer stock units
6. `seller_reservation_price` returns higher values with more demand observations
7. `seller_reservation_price` returns at least `Quantity(1)` always
8. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Both functions are pure — given identical inputs, they produce identical outputs (determinism).
2. `buyer_reservation_price` ≤ `current_coin` always.
3. `seller_reservation_price` ≥ `Quantity(1)` always.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/trade_actions.rs` (new `#[cfg(test)] mod reservation_tests`) — unit tests for both reservation functions and count_local_alternatives

### Commands

1. `cargo test -p worldwake-systems -- reservation` — targeted reservation tests
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite
