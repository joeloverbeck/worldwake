# S04MERSELMAR-001: Add `SaleListing` component and extend `TradeDispositionProfile`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component type, extended existing component
**Deps**: None

## Problem

There is no concrete world state to mark a lot as actively offered for sale. Seller visibility is inferred from `MerchandiseProfile` alone, conflating enterprise intent with instantaneous availability. A new `SaleListing` component on `ItemLot` entities is needed to make sale availability an explicit, inspectable world fact. Additionally, `TradeDispositionProfile` lacks a `market_presence_ticks` field to control how long a merchant maintains a sale stance.

## Assumption Reassessment (2026-03-31)

1. `TradeDispositionProfile` in `crates/worldwake-core/src/trade.rs:27-32` has exactly 4 fields: `negotiation_round_ticks`, `initial_offer_bias`, `concession_rate`, `demand_memory_retention_ticks`. Confirmed — no `market_presence_ticks` exists yet.
2. Component registration macro in `crates/worldwake-core/src/component_tables.rs` and schema in `crates/worldwake-core/src/component_schema.rs` handle typed storage generation. Confirmed via grep — `MerchandiseProfile`, `DemandMemory`, `TradeDispositionProfile` all registered there.
3. `EntityKind::ItemLot` exists in `crates/worldwake-core/src/entity.rs`. Components are registered per-kind in `component_schema.rs`.
4. `SaleListing` does not exist anywhere in the codebase (only in spec files). Confirmed via grep.
5. S04 spec Section 1 defines `SaleListing { listed_at: Tick }` with `impl Component`. Section 4 defines `market_presence_ticks: NonZeroU32` addition to `TradeDispositionProfile`.
6. No adjacent contradictions found.

## Architecture Check

1. Adding a simple component with a single `Tick` field follows the existing pattern for domain components (e.g., `DemandMemory`, `CombatProfile`). No derived state is stored — seller, commodity, and place are all derived from authoritative relations.
2. No backwards-compatibility shims. `TradeDispositionProfile` gains a new field; all construction sites must be updated.

## Verification Layers

1. `SaleListing` registered on `ItemLot` -> component_schema.rs registration test + direct `World` attach/detach round-trip
2. `TradeDispositionProfile` extended -> existing focused tests updated, serialization round-trip verified
3. Single-layer ticket (core types only) — no cross-system verification needed

## What to Change

### 1. Add `SaleListing` struct in `crates/worldwake-core/src/trade.rs`

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SaleListing {
    pub listed_at: Tick,
}

impl Component for SaleListing {}
```

### 2. Register `SaleListing` in component tables and schema

- Add `SaleListing` to the `define_component_tables!` macro invocation in `component_tables.rs`
- Register `SaleListing` on `EntityKind::ItemLot` in `component_schema.rs`
- Add `SaleListing` to `ComponentDelta` enum in `delta.rs`
- Add `SaleListing` accessors to `World` and `WorldTxn`

### 3. Extend `TradeDispositionProfile`

Add `market_presence_ticks: NonZeroU32` field. Update all construction sites:
- `crates/worldwake-core/src/test_utils.rs`
- `crates/worldwake-cli/src/scenario/types.rs` and `scenario/mod.rs`
- `scenarios/default.ron`
- Any test files that construct `TradeDispositionProfile`

### 4. Re-export from `crates/worldwake-core/src/lib.rs`

Ensure `SaleListing` is publicly exported.

## Files to Touch

- `crates/worldwake-core/src/trade.rs` (modify — add `SaleListing`, extend `TradeDispositionProfile`)
- `crates/worldwake-core/src/component_tables.rs` (modify — register `SaleListing`)
- `crates/worldwake-core/src/component_schema.rs` (modify — register on `ItemLot`)
- `crates/worldwake-core/src/delta.rs` (modify — add `SaleListing` variant to `ComponentDelta`)
- `crates/worldwake-core/src/world.rs` (modify — add accessors)
- `crates/worldwake-core/src/world_txn.rs` (modify — add accessors)
- `crates/worldwake-core/src/lib.rs` (modify — re-export)
- `crates/worldwake-core/src/test_utils.rs` (modify — update `TradeDispositionProfile` construction)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — update construction)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — update construction)
- `scenarios/default.ron` (modify — add `market_presence_ticks` field)

## Out of Scope

- `staff_market` action definition or handler (ticket 003)
- Belief view changes (ticket 004)
- Listing cleanup logic (ticket 005)
- Any AI planner changes
- Serialization format migration (no existing save files depend on these types yet)

## Acceptance Criteria

### Tests That Must Pass

1. `SaleListing` can be attached to and detached from an `ItemLot` entity via `World` and `WorldTxn`
2. `SaleListing` round-trips through serde (bincode) correctly
3. `TradeDispositionProfile` with `market_presence_ticks` round-trips through serde correctly
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `SaleListing` is only registerable on `EntityKind::ItemLot` — attaching to other kinds must fail
2. `market_presence_ticks` is `NonZeroU32` — zero is not representable
3. No derived state stored in `SaleListing` — only `listed_at: Tick`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/trade.rs` — unit test for `SaleListing` serde round-trip
2. `crates/worldwake-core/src/trade.rs` — unit test for extended `TradeDispositionProfile` serde round-trip
3. Component schema test verifying `SaleListing` registration on `ItemLot`

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace && cargo test --workspace`
