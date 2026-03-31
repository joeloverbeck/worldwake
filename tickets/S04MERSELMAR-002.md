# S04MERSELMAR-002: Add `StaffMarketPayload` and `ActionPayload::StaffMarket` variant

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new action payload variant
**Deps**: None

## Problem

The action framework has no payload type for the seller-side market-presence action. A new `StaffMarketPayload { commodity }` and corresponding `ActionPayload::StaffMarket` variant are needed before the `staff_market` action definition and handler can be implemented.

## Assumption Reassessment (2026-03-31)

1. `ActionPayload` enum in `crates/worldwake-sim/src/action_payload.rs` currently has variants: `None`, `Harvest`, `Craft`, `Trade`, `Combat`, `Loot`, `Transport`, `Travel`, `Tell`, `QueueForFacilityUse`, `YieldForceClaim`, `Patrol`, `Accuse`, `PunishAccused`, `ConsultRecord`, `Investigate`, `EstablishBanditCamp`, `RaidTarget`. Confirmed via read at line 24+.
2. `TradeActionPayload` at line 266 has fields: `counterparty`, `offered_commodity`, `offered_quantity`, `requested_commodity`, `requested_quantity`. No `sale_lot` field yet (that is ticket 008).
3. All payload structs derive `Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`. Confirmed.
4. Payload variants have corresponding `as_*` accessor methods on `ActionPayload`. Confirmed at line 106+.
5. No `StaffMarket` variant or `StaffMarketPayload` exists. Confirmed.
6. No adjacent contradictions found.

## Architecture Check

1. Follows the exact same pattern as every other payload variant (e.g., `LootActionPayload`, `PatrolActionPayload`). One struct, one enum variant, one accessor.
2. No backwards-compatibility shims.

## Verification Layers

1. `StaffMarketPayload` serde round-trip -> focused unit test
2. `ActionPayload::StaffMarket` accessor -> focused unit test
3. Single-layer ticket (sim types only)

## What to Change

### 1. Add `StaffMarketPayload` in `action_payload.rs`

```rust
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct StaffMarketPayload {
    pub commodity: CommodityKind,
}
```

### 2. Add `ActionPayload::StaffMarket(StaffMarketPayload)` variant

Add to the `ActionPayload` enum. Add `as_staff_market()` accessor method following the pattern of `as_trade()`, `as_combat()`, etc.

### 3. Update serialization assertion test

The `action_payload.rs` module has a `assert_traits` test at line ~534. Add `StaffMarketPayload` to it.

### 4. Re-export from `crates/worldwake-sim/src/lib.rs`

Ensure `StaffMarketPayload` is publicly exported.

## Files to Touch

- `crates/worldwake-sim/src/action_payload.rs` (modify — add struct, variant, accessor)
- `crates/worldwake-sim/src/lib.rs` (modify — re-export)

## Out of Scope

- `staff_market` action definition or handler (ticket 003)
- Changes to `TradeActionPayload` (ticket 008)
- Planner op registration (ticket 006)

## Acceptance Criteria

### Tests That Must Pass

1. `StaffMarketPayload` round-trips through serde (bincode) correctly
2. `ActionPayload::StaffMarket` accessor returns the correct inner value
3. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. `StaffMarketPayload` contains only `commodity: CommodityKind` — no derived state
2. Follows identical derive and trait pattern as all other payload types

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_payload.rs` — `sample_staff_market_payload()` + serde round-trip test following existing pattern
2. `crates/worldwake-sim/src/action_payload.rs` — add `StaffMarketPayload` to `assert_traits` test

### Commands

1. `cargo test -p worldwake-sim -- action_payload`
2. `cargo clippy --workspace && cargo test --workspace`
