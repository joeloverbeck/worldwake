# E11TRAECO-004: Add `TradeDispositionProfile` Component

**Status**: COMPLETED

**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new trade-domain authoritative component in `worldwake-core`
**Deps**: E11TRAECO-002 and E11TRAECO-003 complete

## Problem

E11 needs explicit per-agent trade behavior parameters for negotiation pacing, opening stance, concession behavior, and demand-memory retention. Without a first-class `TradeDispositionProfile`, later trade handling and demand-memory aging would either hardcode cross-agent behavior, hide configuration in system logic, or couple memory dampening to non-trade components. That would be weaker than the current architecture goal of concrete, inspectable, per-agent state.

## Assumption Reassessment (2026-03-11)

1. `crates/worldwake-core/src/trade.rs` already exists and currently contains `MerchandiseProfile`, `DemandMemory`, `DemandObservation`, and `DemandObservationReason`.
2. The original ticket incorrectly treated this as mostly a four-file change. In this codebase, authoritative components are declared once in `component_schema.rs`, and that schema fans out into generated APIs for `ComponentTables`, `World`, `WorldTxn`, and `delta` component inventories.
3. Because of that schema-driven architecture, this ticket should extend the existing trade-domain module and schema path rather than add bespoke storage or wrapper APIs.
4. Kind restrictions are enforced by generated `World::insert_component_*` methods, not by `ComponentTables`, so world-level wrong-kind rejection tests are part of the real acceptance surface.
5. Existing coverage patterns for new authoritative components are stronger than the original ticket described:
   - module-local trait and serialization tests in the owning domain module
   - `component_tables.rs` CRUD coverage
   - `world.rs` roundtrip and wrong-kind rejection coverage
   - `delta.rs` authoritative component inventory coverage
   - downstream schema assertions where `ComponentKind::ALL` is pinned
6. No `TradeDispositionProfile` or `SubstitutePreferences` type exists yet anywhere in the Rust crates.
7. The original dependency note "trade.rs module must exist" is stale. The real live dependencies are the archived E11TRAECO-002 and E11TRAECO-003 results plus the existing schema-generated component architecture.

## Architecture Check

1. Adding `TradeDispositionProfile` to `crates/worldwake-core/src/trade.rs` is architecturally stronger than introducing a generic profile component elsewhere. E11 is building a coherent trade-domain authoritative schema, and negotiation pacing plus memory retention belongs with that domain.
2. The proposed shape is better than pushing these values into system-local constants. It preserves Principle 3 by storing concrete agent parameters instead of embedding hidden negotiation heuristics inside the trade system.
3. `demand_memory_retention_ticks` belongs in the trade disposition profile, not in `DemandMemory`, because it is an agent-specific policy knob for how this actor behaves, not part of the observed demand facts themselves.
4. The current schema-driven component architecture is already the cleaner long-term design. The robust move is to extend that path and keep generated world/storage/value inventories aligned, not to bypass it with aliases or one-off helper layers.
5. A possible future refinement, if E11 adds more trade-domain component fixtures, is to keep expanding shared test fixtures in `test_utils.rs` rather than duplicating literal profiles across tests. That improves test cohesion without changing runtime architecture.

## Scope Correction

This ticket should:

1. Add `TradeDispositionProfile` to `crates/worldwake-core/src/trade.rs`.
2. Register it as an agent-only authoritative component through `component_schema.rs`.
3. Re-export it from `worldwake-core`.
4. Extend focused tests at the module, component-table, world API, and authoritative component inventory levels.
5. Update any downstream schema assertion tests that intentionally enumerate `ComponentKind::ALL`.

This ticket should not:

1. Implement negotiation logic, valuation, or trade action handling.
2. Implement demand-memory aging behavior itself. That remains E11TRAECO-009.
3. Add compatibility aliases, fallback profile defaults, or hidden system-local constants.
4. Introduce `SubstitutePreferences` or any restock logic.
5. Refactor unrelated component infrastructure.

## What to Change

### 1. Extend `crates/worldwake-core/src/trade.rs`

Define:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TradeDispositionProfile {
    pub negotiation_round_ticks: NonZeroU32,
    pub initial_offer_bias: Permille,
    pub concession_rate: Permille,
    pub demand_memory_retention_ticks: u32,
}

impl Component for TradeDispositionProfile {}
```

Include focused module-local tests for:

- `Clone + Eq + Debug + Serialize + DeserializeOwned + Component`
- bincode roundtrip

### 2. Export the new type from `crates/worldwake-core/src/lib.rs`

Re-export `TradeDispositionProfile`.

### 3. Register `TradeDispositionProfile` in `crates/worldwake-core/src/component_schema.rs`

Add a schema entry for `TradeDispositionProfile` restricted to `EntityKind::Agent`.

### 4. Wire the schema-generated storage and APIs

Update imports and tests where required so the schema expansion compiles and remains covered:

- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/test_utils.rs`

`world_txn.rs` should not need bespoke logic if the schema entry is correct.

### 5. Update downstream schema assertions if they pin authoritative component inventory

Expected file:

- `crates/worldwake-systems/tests/e09_needs_integration.rs`

## Files to Touch

- `crates/worldwake-core/src/trade.rs`
- `crates/worldwake-core/src/lib.rs`
- `crates/worldwake-core/src/component_schema.rs`
- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/test_utils.rs`
- `crates/worldwake-systems/tests/e09_needs_integration.rs`

## Out of Scope

- trade-system tick behavior
- valuation or negotiation bundle search
- merchant restock planning
- substitute demand
- any `worldwake-sim` runtime trade behavior

## Acceptance Criteria

### Tests That Must Pass

1. `TradeDispositionProfile` satisfies `Clone + Eq + Debug + Serialize + DeserializeOwned + Component`.
2. `TradeDispositionProfile` round-trips through bincode.
3. `ComponentTables` supports insert/get/remove/has for `TradeDispositionProfile`.
4. `World` accepts `TradeDispositionProfile` on `EntityKind::Agent`, exposes generated get/has/remove/query APIs, and rejects insertion on non-agent kinds.
5. `ComponentKind::ALL` and `ComponentValue` inventory coverage remain accurate after adding `TradeDispositionProfile`.
6. `cargo test -p worldwake-core`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `cargo test --workspace`

### Invariants

1. `TradeDispositionProfile` is authoritative stored state, not a derived score.
2. The component is valid only on `EntityKind::Agent`.
3. Ratio fields use `Permille`, not floats.
4. `negotiation_round_ticks` uses `NonZeroU32`, not an ad hoc runtime check.
5. No `HashMap`/`HashSet`, compatibility aliases, or hidden trade heuristics are introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/trade.rs` — trait bounds and bincode roundtrip for `TradeDispositionProfile`
2. `crates/worldwake-core/src/component_tables.rs` — CRUD coverage for `TradeDispositionProfile`
3. `crates/worldwake-core/src/world.rs` — agent roundtrip and wrong-kind rejection for `TradeDispositionProfile`
4. `crates/worldwake-core/src/delta.rs` — authoritative component inventory assertions updated for the new component kind/value
5. `crates/worldwake-systems/tests/e09_needs_integration.rs` — expected authoritative schema inventory updated for the new trade component

### Commands

1. `cargo test -p worldwake-core trade_disposition`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Added `TradeDispositionProfile` to `crates/worldwake-core/src/trade.rs` with focused module-local trait-bound and bincode tests.
  - Registered `TradeDispositionProfile` as an agent-only authoritative component through `component_schema.rs`.
  - Re-exported `TradeDispositionProfile` from `worldwake-core`.
  - Added a shared `sample_trade_disposition_profile()` fixture in `crates/worldwake-core/src/test_utils.rs`.
  - Extended `component_tables.rs`, `world.rs`, and `delta.rs` so the schema-generated storage, world APIs, and authoritative component inventories include `TradeDispositionProfile`.
  - Updated the downstream authoritative-schema assertion in `crates/worldwake-systems/tests/e09_needs_integration.rs`.
- Deviations from original plan:
  - The ticket was corrected before implementation because the original version understated the schema-driven integration path and omitted `delta.rs`, `world.rs`, shared fixtures, and downstream schema assertions from the real blast radius.
  - `world_txn.rs` did not require bespoke changes; the schema entry propagated the generated transaction APIs as expected.
  - No broader architectural rewrite was justified. The current trade-domain module plus schema-driven component registration is the cleaner long-term design for this change.
- Verification results:
  - `cargo fmt --all --check` passed after formatting.
  - `cargo test -p worldwake-core trade_disposition` passed.
  - `cargo test -p worldwake-core` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
