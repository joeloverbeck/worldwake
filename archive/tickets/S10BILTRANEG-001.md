# S10BILTRANEG-001: Core type additions for bilateral trade negotiation

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-core` (TradeRole, DemandObservationReason variant, TradeDispositionProfile field), `worldwake-sim` (ActionState variant)
**Deps**: specs/S10-bilateral-trade-negotiation.md

## Problem

The bilateral trade negotiation protocol (S10) requires several new types and type extensions before any behavioral code can be written. Without `TradeRole`, `ActionState::Trade`, `DemandObservationReason::TradeAgreed`, and `rejection_escalation_rate`, the downstream tickets (002-006) cannot compile. This ticket delivers all type-level scaffolding with no behavioral changes.

## Assumption Reassessment (2026-04-02)

1. `TradeRole` does not exist anywhere in the codebase — confirmed via grep for `enum TradeRole` (zero matches).
2. `DemandObservationReason` at `crates/worldwake-core/src/trade.rs:102-107` has 4 variants: `WantedToBuyButNoSeller`, `WantedToBuyButSellerOutOfStock`, `WantedToBuyButTooExpensive`, `WantedToSellButNoBuyer`. No `TradeAgreed` variant exists.
3. `TradeDispositionProfile` at `crates/worldwake-core/src/trade.rs:72-78` has 5 fields: `negotiation_round_ticks`, `initial_offer_bias`, `concession_rate`, `demand_memory_retention_ticks`, `market_presence_ticks`. No `rejection_escalation_rate` field exists.
4. `ActionState` at `crates/worldwake-sim/src/action_state.rs:7-26` derives `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Serialize, Deserialize`. Has 4 variants: `Empty`, `Heal`, `Investigate`, `Travel`. All fields in all variants are `Copy`. The new `Trade` variant must maintain this constraint.
5. `Quantity(pub u32)` at `crates/worldwake-core/src/numerics.rs` is `Copy`. `Option<Quantity>` is `Copy`. `u32` is `Copy`. All proposed `Trade` variant fields satisfy the `Copy` constraint.
6. The existing bincode roundtrip test at `action_state.rs:58-93` covers all current variants and must be extended for `Trade`.
7. `TradeDispositionProfile { ... }` construction sites are broader than originally listed. Current grep shows live construction in `worldwake-core`, `worldwake-sim`, `worldwake-systems`, and many `worldwake-ai` tests/helpers, including `golden_supply_chain.rs`, `golden_trade.rs`, `golden_merchant_selling.rs`, `golden_integration.rs`, `golden_harness/soak_world.rs`, `golden_production.rs`, `golden_determinism.rs`, `golden_t22_bandit_camp_destruction.rs`, `golden_ai_decisions.rs`, and `planner_conformance.rs`. This ticket must update all of them because the new field is authoritative type scaffolding, not later behavioral work.

## Architecture Check

1. Placing `TradeRole` in `worldwake-core/src/trade.rs` alongside `TradeDispositionProfile` and `DemandObservationReason` is the natural location — all trade-related types colocate. Adding the `ActionState::Trade` variant in `worldwake-sim/src/action_state.rs` follows the existing pattern (`Heal`, `Investigate`, `Travel`).
2. No backward-compatibility shims. The new `rejection_escalation_rate` field on `TradeDispositionProfile` is a struct addition — all construction sites must be updated (compile error enforces this).

## Verification Layers

1. `TradeRole` satisfies required trait bounds -> focused unit test (trait assertion)
2. `ActionState::Trade` roundtrips through bincode -> focused unit test (serialization)
3. `TradeDispositionProfile` construction sites compile with new field -> compiler plus focused crate tests at the changed type-definition boundaries
4. Single-layer ticket (type additions only, no runtime behavior change).

## What to Change

### 1. Add `TradeRole` enum to `worldwake-core/src/trade.rs`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum TradeRole {
    Buyer,
    Seller,
}
```

Ensure it is exported from the crate's public API (`lib.rs`).

### 2. Add `TradeAgreed` variant to `DemandObservationReason`

Add to the existing enum:

```rust
pub enum DemandObservationReason {
    WantedToBuyButNoSeller,
    WantedToBuyButSellerOutOfStock,
    WantedToBuyButTooExpensive,
    WantedToSellButNoBuyer,
    TradeAgreed,  // NEW
}
```

### 3. Add `rejection_escalation_rate: Permille` to `TradeDispositionProfile`

Add the field to the struct. Update every construction site in the codebase (grep for `TradeDispositionProfile {` to find all). This includes:
- `crates/worldwake-ai/tests/golden_supply_chain.rs` — `default_trade_disposition()` and `enterprise_trade_disposition()`
- `crates/worldwake-ai/tests/golden_trade.rs` — current trade disposition helpers used by the live merchant/trade goldens
- any other production or test construction site found via grep across `worldwake-core`, `worldwake-sim`, `worldwake-systems`, and `worldwake-ai`

Use a sensible default value in test helpers: `pm(200)` (20% of reservation per rejection — the value the spec recommends).

### 4. Add `ActionState::Trade` variant to `worldwake-sim/src/action_state.rs`

```rust
pub enum ActionState {
    // ... existing variants ...
    Trade {
        round: u32,
        initiator_role: TradeRole,
        initiator_last_offer: Option<Quantity>,
        responder_last_offer: Option<Quantity>,
        agreed_price: Option<Quantity>,
    },
}
```

Import `TradeRole` and `Quantity` from `worldwake_core`. Extend the bincode roundtrip test to include a `Trade` variant with representative field values.

## Files to Touch

- `crates/worldwake-core/src/trade.rs` (modify — add `TradeRole`, `TradeAgreed`, `rejection_escalation_rate`)
- `crates/worldwake-core/src/lib.rs` (modify — export `TradeRole`)
- `crates/worldwake-sim/src/action_state.rs` (modify — add `Trade` variant, extend roundtrip test)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify — add `rejection_escalation_rate` to disposition helpers)
- `crates/worldwake-core/src/test_utils.rs` (modify — representative disposition fixture gains the new field)
- any other `TradeDispositionProfile` construction sites found via grep in `worldwake-core`, `worldwake-sim`, `worldwake-systems`, and `worldwake-ai` (modify)

## Out of Scope

- Behavioral changes to trade action handlers (ticket 005)
- Reservation price functions (ticket 002)
- Concession curve logic (ticket 003)
- Affordance generation changes (ticket 004)
- Golden test creation (ticket 006)

## Acceptance Criteria

### Tests That Must Pass

1. `TradeRole` satisfies `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + DeserializeOwned` (trait assertion test)
2. `ActionState::Trade` bincode roundtrip produces identical output
3. Updated construction sites compile cleanly across touched crates
4. All existing golden tests pass: `cargo test -p worldwake-ai`
5. Full suite: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `ActionState` remains `Copy` — the derive macro enforces this at compile time for all variants.
2. All `TradeDispositionProfile` construction sites compile with the new field — no default fallback or `..Default::default()` spread.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_state.rs` (in `mod tests`) — extend `action_state_bincode_roundtrip_covers_every_variant` with `ActionState::Trade` variant
2. `crates/worldwake-core/src/trade.rs` (new test) — trait assertion for `TradeRole`
3. touched crate compile/test surfaces for the new `TradeDispositionProfile` field, including current `worldwake-ai` golden helpers and updated core/systems fixtures

### Commands

1. `cargo test -p worldwake-sim -- action_state` — targeted ActionState tests
2. `cargo test -p worldwake-core -- trade` — targeted trade type tests
3. `cargo test -p worldwake-ai` — current golden/helper compile surface for trade dispositions
4. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — full suite

## Outcome

- Completed: 2026-04-02
- What changed:
  - added `TradeRole`, `DemandObservationReason::TradeAgreed`, and `TradeDispositionProfile.rejection_escalation_rate` in `crates/worldwake-core/src/trade.rs`, and exported `TradeRole` from `crates/worldwake-core/src/lib.rs`
  - added `ActionState::Trade` plus roundtrip coverage in `crates/worldwake-sim/src/action_state.rs`
  - updated all live `TradeDispositionProfile` construction sites found during reassessment across `worldwake-core`, `worldwake-sim`, `worldwake-systems`, `worldwake-ai` tests/helpers, CLI scenario types, and the bundled `scenarios/default.ron`
  - fixed enum-expansion fallout by updating the exhaustive `ActionState` handling in `crates/worldwake-systems/src/travel_actions.rs`
- Deviations from original plan:
  - the ticket stayed type-scaffolding only, but the actual blast radius was wider than the initial file list because serialized fixtures and bundled scenario data also had to adopt the new `TradeDispositionProfile` shape
  - no additional runtime negotiation behavior was added in this ticket; the implementation remained limited to shared type surfaces and compile/serialization fallout
- Verification results:
  - `cargo test -p worldwake-core -- trade --nocapture`
  - `cargo test -p worldwake-sim -- action_state --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-cli --test integration`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
