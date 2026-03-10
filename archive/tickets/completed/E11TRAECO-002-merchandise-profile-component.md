# E11TRAECO-002: Add `MerchandiseProfile` Component

**Status**: COMPLETED

**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new trade-domain authoritative component in `worldwake-core`
**Deps**: E11TRAECO-001 complete; E11 trade schema work in progress

## Problem

E11 needs a first-class way to represent merchant sale intent as concrete authoritative state. Without `MerchandiseProfile`, later trade valuation, unmet-demand memory, and merchant restock logic would either infer merchant behavior from ad hoc inventory snapshots or introduce hidden trade-role heuristics. That would be weaker than the current architecture goal of explicit, inspectable state.

## Assumption Reassessment (2026-03-11)

1. No `MerchandiseProfile` type or `trade.rs` core module exists yet — confirmed.
2. The original ticket understated the integration pattern for new authoritative components. In this codebase, new components are added through `component_schema.rs`, and that schema then drives generated APIs across `ComponentTables`, `World`, `WorldTxn`, and `ComponentKind`/`ComponentValue`.
3. Because of that schema-driven architecture, this ticket does **not** need bespoke edits in `delta.rs`, `world_txn.rs`, or hand-written world APIs unless tests reveal a genuine gap.
4. Kind restrictions are enforced by the generated `World::insert_component_*` API, not by `ComponentTables`.
5. Existing test coverage patterns for new authoritative components are stronger than the original ticket described:
   - module-local trait/serialization tests in the owning domain module
   - `component_tables.rs` CRUD coverage
   - `world.rs` roundtrip and wrong-kind rejection coverage
6. The original "new `trade.rs` module" assumption is still a good architectural fit, but the reason is domain cohesion, not because `components.rs` is technically incapable of holding the type.

## Architecture Check

1. A dedicated `crates/worldwake-core/src/trade.rs` module is better than extending `components.rs`. Trade is becoming its own shared schema area in E11 (`MerchandiseProfile`, `DemandMemory`, `TradeDispositionProfile`, `SubstitutePreferences`), and scattering those types across generic component files would make the architecture less coherent over time.
2. `MerchandiseProfile` should remain narrowly about merchant intent:
   - `sale_kinds` is the concrete set of commodities this actor aims to stock and sell.
   - `home_market` is an optional anchor for later restock/travel reasoning.
   This is cleaner than embedding pricing, thresholds, or demand heuristics into the component.
3. The current schema-driven component architecture is already stronger than the ticket assumed. The robust move here is to plug `MerchandiseProfile` into that existing path, not to bypass it with ad hoc storage or helper aliases.
4. A broader architectural improvement may become worthwhile later if E11 accumulates repeated ownership-transfer logic, but that is not this ticket. No compatibility shims or speculative abstractions are justified here.

## Scope Correction

This ticket should:

1. Add `MerchandiseProfile` in a dedicated `trade.rs` module in `worldwake-core`.
2. Register it as an agent-only authoritative component through `component_schema.rs`.
3. Re-export it from `worldwake-core`.
4. Add focused tests at the module, component-table, and world API levels.

This ticket should not:

1. Add `DemandMemory`, `TradeDispositionProfile`, or `SubstitutePreferences`.
2. Implement valuation, negotiation, restock planning, or trade actions.
3. Add compatibility aliases, role flags, or implicit merchant heuristics.
4. Refactor unrelated core component infrastructure.

## What to Change

### 1. Add `crates/worldwake-core/src/trade.rs`

Define:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MerchandiseProfile {
    pub sale_kinds: BTreeSet<CommodityKind>,
    pub home_market: Option<EntityId>,
}

impl Component for MerchandiseProfile {}
```

Include focused module-local tests for component bounds and bincode roundtrip.

### 2. Export the trade module from `crates/worldwake-core/src/lib.rs`

- add `pub mod trade;`
- re-export `MerchandiseProfile`

### 3. Register the component in `crates/worldwake-core/src/component_schema.rs`

Add a schema entry for `MerchandiseProfile` restricted to `EntityKind::Agent`.

### 4. Wire typed storage in `crates/worldwake-core/src/component_tables.rs`

Import `MerchandiseProfile` so the schema-generated tables compile, then add/extend focused table CRUD tests.

### 5. Extend world-level tests in `crates/worldwake-core/src/world.rs`

Add:

- agent roundtrip coverage for `MerchandiseProfile`
- wrong-kind rejection coverage for non-agent entities

## Files to Touch

- `crates/worldwake-core/src/trade.rs` (new)
- `crates/worldwake-core/src/lib.rs`
- `crates/worldwake-core/src/component_schema.rs`
- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-core/src/world.rs`

## Out of Scope

- `ActionPayload` changes
- belief/valuation logic
- demand-memory recording or aging
- merchant restock goal selection
- any `worldwake-sim` or `worldwake-systems` trade behavior

## Acceptance Criteria

### Tests That Must Pass

1. `MerchandiseProfile` satisfies `Clone + Eq + Debug + Serialize + DeserializeOwned + Component`.
2. `MerchandiseProfile` round-trips through bincode.
3. `ComponentTables` supports insert/get/remove/has for `MerchandiseProfile`.
4. `World` accepts `MerchandiseProfile` on `EntityKind::Agent`, exposes generated get/has/remove/query APIs, and rejects insertion on non-agent kinds.
5. `sale_kinds` uses `BTreeSet<CommodityKind>`.
6. `cargo test -p worldwake-core`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `cargo test --workspace`

### Invariants

1. `MerchandiseProfile` is authoritative stored state, not a derived score.
2. The component is valid only on `EntityKind::Agent`.
3. No `HashMap`/`HashSet`, floats, or compatibility aliases are introduced.
4. Trade intent remains separate from valuation formulas and market-wide state.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/trade.rs` — trait bounds and bincode roundtrip for `MerchandiseProfile`
2. `crates/worldwake-core/src/component_tables.rs` — CRUD coverage for `MerchandiseProfile`
3. `crates/worldwake-core/src/world.rs` — agent roundtrip and wrong-kind rejection for `MerchandiseProfile`

### Commands

1. `cargo test -p worldwake-core merchandise_profile`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Added `crates/worldwake-core/src/trade.rs` with the new `MerchandiseProfile` component and focused module-local tests.
  - Registered `MerchandiseProfile` as an agent-only authoritative component through `component_schema.rs`.
  - Re-exported `MerchandiseProfile` from `worldwake-core`.
  - Extended `component_tables.rs`, `world.rs`, and `delta.rs` so the new component participates in generated typed storage/APIs and authoritative component inventories.
  - Updated downstream workspace verification to account for the expanded `ComponentKind::ALL` inventory.
- Deviations from original plan:
  - The ticket was corrected first because the original version overstated manual integration work in some files and understated the existing schema-driven architecture.
  - `delta.rs` needed explicit test updates even though runtime wiring came from `component_schema.rs`; without that, the new authoritative component would have left canonical component-inventory assertions stale.
  - Two pre-existing clippy pedantic failures outside the immediate ticket scope were resolved with targeted `#[allow(clippy::too_many_lines)]` annotations so the repository-wide lint gate could pass unchanged in behavior.
- Verification results:
  - `cargo test -p worldwake-core merchandise_profile` passed.
  - `cargo test -p worldwake-core` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
