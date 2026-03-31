# S04MERSELMAR-008: `AcquireCommodity` listed-lot evidence and `TradeActionPayload` migration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — buyer-side evidence generation, trade payload struct change, affordance assembly
**Deps**: S04MERSELMAR-004

## Problem

Buyer-side `AcquireCommodity` candidate generation currently discovers sellers through `agents_selling_at` (now replaced by ticket 004). The candidate generation, evidence assembly, and trade affordance/payload must be migrated to use listed sale lots. Additionally, `TradeActionPayload` must gain a `sale_lot` field so trades operate against concrete listed lots rather than abstract commodity buckets.

## Assumption Reassessment (2026-03-31)

1. `TradeActionPayload` at `crates/worldwake-sim/src/action_payload.rs:266-272` currently has: `counterparty`, `offered_commodity`, `offered_quantity`, `requested_commodity`, `requested_quantity`. No `sale_lot` field. Confirmed.
2. Trade affordance generation in `crates/worldwake-sim/src/affordance_query.rs` uses `agents_selling_at` (now being replaced). The affordance must be updated to enumerate listed sale lots.
3. Candidate generation for `AcquireCommodity` in `crates/worldwake-ai/src/candidate_generation.rs` inspects sellers at place for evidence. After ticket 004, it must use `listed_sale_lots_at` + `seller_for_sale_lot`.
4. The spec (Section 10) adds `sale_lot: EntityId` to `TradeActionPayload` and derives `requested_commodity` from `ItemLot.commodity` at commit time instead of storing it in the payload.
5. Trade action handler in `crates/worldwake-systems/src/trade_actions.rs` currently reads `requested_commodity` from the payload. It must be updated to derive it from the `sale_lot`.
6. Search transition for `Trade` op in `crates/worldwake-ai/src/search/transition.rs` assembles trade payloads. Must be updated to include `sale_lot`.
7. `PayloadEntityRole` entries for `Trade` payload must include `sale_lot`.
8. No adjacent contradictions found.

## Architecture Check

1. Adding `sale_lot` to `TradeActionPayload` and deriving `requested_commodity` from it is architecturally cleaner — trades operate on concrete supply rather than abstract commodity names. This aligns with Principle 3 (concrete state over abstract scores) and Principle 4 (persistent identity).
2. Removing `requested_commodity` from the payload prevents stale data — the lot's commodity is the authoritative source.
3. No backwards-compatibility shims. `TradeActionPayload` changes cleanly; all construction sites are updated.

## Verification Layers

1. `AcquireCommodity` evidence includes listed sale lots -> focused unit test in candidate_generation.rs
2. `TradeActionPayload` includes `sale_lot` -> compilation + focused unit test
3. Trade affordance enumerates listed lots -> focused unit test in affordance_query.rs
4. Search transition assembles payload with `sale_lot` -> focused unit test
5. `requested_commodity` derived from `sale_lot` at commit time -> trade handler focused test

## What to Change

### 1. Migrate `TradeActionPayload` in `action_payload.rs`

Replace:
```rust
pub struct TradeActionPayload {
    pub counterparty: EntityId,
    pub offered_commodity: CommodityKind,
    pub offered_quantity: Quantity,
    pub requested_commodity: CommodityKind,
    pub requested_quantity: Quantity,
}
```
With:
```rust
pub struct TradeActionPayload {
    pub counterparty: EntityId,
    pub sale_lot: EntityId,
    pub offered_commodity: CommodityKind,
    pub offered_quantity: Quantity,
    pub requested_quantity: Quantity,
}
```

`requested_commodity` is derived from `ItemLot.commodity` of `sale_lot` at commit time.

### 2. Update all `TradeActionPayload` construction sites

- `crates/worldwake-sim/src/affordance_query.rs` — trade affordance assembly
- `crates/worldwake-ai/src/search/transition.rs` — plan search trade payload
- `crates/worldwake-ai/src/search/candidates.rs` — if trade candidates are assembled here
- `crates/worldwake-systems/src/trade_actions.rs` — handler reads and test construction
- `crates/worldwake-sim/src/action_payload.rs` — test `sample_trade_payload()`

### 3. Update `AcquireCommodity` candidate generation

In `candidate_generation.rs`, when generating `AcquireCommodity` candidates for the "trade with seller" path:
- Use `listed_sale_lots_at(place, commodity)` to find available lots
- Use `seller_for_sale_lot(lot)` to identify the counterparty
- Include sale lot entity and seller entity in evidence

### 4. Update trade affordance enumeration in `affordance_query.rs`

When enumerating trade affordances for an agent:
- Use `listed_sale_lots_at` to find available listed lots at the agent's place
- For each listed lot + seller pair, generate a trade affordance with `sale_lot` in the payload

### 5. Update `PayloadEntityRole` for trade

Add `sale_lot` to the `PayloadEntityRole` entries so the action framework tracks the lot entity in payload validation.

### 6. Update trade action handler to derive `requested_commodity`

In `trade_actions.rs`, the commit handler should derive `requested_commodity` from `world.item_lot(payload.sale_lot).commodity` instead of reading it from the payload.

## Files to Touch

- `crates/worldwake-sim/src/action_payload.rs` (modify — add `sale_lot`, remove `requested_commodity`)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — trade affordance uses listed lots)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — AcquireCommodity evidence uses listed lots)
- `crates/worldwake-ai/src/search/transition.rs` (modify — trade transition assembles sale_lot payload)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — if trade candidate expansion here)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — handler derives requested_commodity from lot)
- `crates/worldwake-sim/src/start_gate.rs` (modify — if payload entity validation updated)

## Out of Scope

- Trade commit validation rules (ticket 009)
- Seller-side `SellCommodity` candidate generation (ticket 007)
- `staff_market` action (ticket 003)
- Listing cleanup (ticket 005)
- DemandMemory ranking (ticket 010)

## Acceptance Criteria

### Tests That Must Pass

1. `AcquireCommodity` candidate generation discovers listed sale lots, not profile-inferred sellers
2. Trade affordance includes `sale_lot` pointing to a concrete listed lot
3. `TradeActionPayload` compiles with `sale_lot` field and without `requested_commodity`
4. Trade handler correctly derives `requested_commodity` from `sale_lot` entity
5. Unlisted merchant stock at the same place does NOT appear as a trade affordance
6. Existing suite: `cargo test --workspace`

### Invariants

1. Buyer discovery depends on `SaleListing` state, not `MerchandiseProfile` alone
2. Trade payloads reference concrete lot entities, not abstract commodity buckets
3. `requested_commodity` is derived, not stored in payload (prevents stale data)
4. No backward-compatibility preserving old `requested_commodity` field

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused test: AcquireCommodity discovers listed lots
2. `crates/worldwake-sim/src/affordance_query.rs` — focused test: trade affordance enumerates listed lots only
3. `crates/worldwake-systems/src/trade_actions.rs` — focused test: handler derives requested_commodity from sale_lot
4. `crates/worldwake-sim/src/action_payload.rs` — update `sample_trade_payload()` test

### Commands

1. `cargo test -p worldwake-sim -- affordance`
2. `cargo test -p worldwake-ai -- candidate_generation`
3. `cargo test -p worldwake-systems -- trade_action`
4. `cargo clippy --workspace && cargo test --workspace`
