# E11TRAECO-005: Add `SubstitutePreferences` Component

**Status**: COMPLETED

**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new trade-domain authoritative component in `worldwake-core`
**Deps**: E11TRAECO-002, E11TRAECO-003, and E11TRAECO-004 complete

## Problem

E11 needs explicit per-agent substitute ordering so later trade selection can react to scarcity through concrete agent state instead of hidden fallback heuristics. Without a first-class `SubstitutePreferences`, substitute demand in E11TRAECO-010 would either infer behavior from global commodity categories alone or embed ad hoc preference rules inside trade logic. That would be weaker than the current architecture goal of explicit, inspectable, per-agent state.

## Assumption Reassessment (2026-03-11)

1. `crates/worldwake-core/src/trade.rs` already exists and currently contains `MerchandiseProfile`, `DemandMemory`, and `TradeDispositionProfile`.
2. The original ticket incorrectly assumed this was mainly a manual `trade.rs` plus `component_tables.rs` change. In this repo, authoritative components are declared in `component_schema.rs`, and that schema fans out into generated APIs for `ComponentTables`, `World`, `WorldTxn`, and `delta` component inventories.
3. Because of that schema-driven architecture, the real blast radius includes `delta.rs`, `world.rs`, shared test fixtures, and any downstream tests that pin the authoritative component inventory.
4. Kind restrictions are enforced by generated `World::insert_component_*` methods, not by `ComponentTables`, so world-level wrong-kind rejection tests are part of the real acceptance surface.
5. Existing coverage patterns for new authoritative components are stronger than the original ticket described:
   - module-local trait and serialization tests in the owning domain module
   - `component_tables.rs` CRUD coverage
   - `world.rs` roundtrip and wrong-kind rejection coverage
   - `delta.rs` authoritative component inventory coverage
   - downstream schema assertions where `ComponentKind::ALL` is intentionally pinned
6. No `SubstitutePreferences` type or substitute-preference fixture exists yet in the Rust crates.
7. The original dependency note `E11TRAECO-002 (trade.rs module must exist)` is stale. The live dependency is the already-completed trade-domain schema work from E11TRAECO-002 through E11TRAECO-004.

## Architecture Check

1. Adding `SubstitutePreferences` to `crates/worldwake-core/src/trade.rs` is better than introducing substitute logic directly in a later trade handler. Preference ordering is agent state, not system-local policy.
2. The proposed shape remains architecturally sound for Phase 2:
   - `BTreeMap<TradeCategory, Vec<CommodityKind>>` keeps deterministic iteration.
   - `TradeCategory` keys make substitution local to a concrete commodity grouping already defined in `items.rs`.
   - ordered `Vec<CommodityKind>` expresses ranking without introducing hidden scoring formulas.
3. This is cleaner than deriving substitute order from `CommodityKind::ALL`, category membership, or hardcoded fallback tables in `worldwake-systems`. Those alternatives would centralize behavior that should vary per agent.
4. The main long-term architectural risk is not this component itself, but whether some substitutions eventually need need-specific or context-specific reasoning beyond one category-level ordering. That is a future trade-evaluation concern, not a reason to avoid storing the baseline preference state now.
5. No broader refactor is justified here. The current trade-domain module plus schema-driven component registration is already the robust long-term path; adding aliases, wrappers, or parallel storage would make it worse.

## Scope Correction

This ticket should:

1. Add `SubstitutePreferences` to `crates/worldwake-core/src/trade.rs`.
2. Register it as an agent-only authoritative component through `component_schema.rs`.
3. Re-export it from `worldwake-core`.
4. Add a shared sample fixture in `crates/worldwake-core/src/test_utils.rs`.
5. Extend focused tests at the module, component-table, world API, and authoritative component inventory levels.
6. Update any downstream schema assertion tests that intentionally enumerate `ComponentKind::ALL`.

This ticket should not:

1. Implement substitute-demand behavior or trade fallback selection. That remains E11TRAECO-010.
2. Add valuation formulas, demand logic, or negotiation behavior.
3. Introduce compatibility aliases, default global substitute tables, or hidden fallback heuristics.
4. Refactor unrelated component infrastructure.

## What to Change

### 1. Extend `crates/worldwake-core/src/trade.rs`

Define:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubstitutePreferences {
    pub preferences: BTreeMap<TradeCategory, Vec<CommodityKind>>,
}

impl Component for SubstitutePreferences {}
```

Include focused module-local tests for:

- `Clone + Eq + Debug + Serialize + DeserializeOwned + Component`
- bincode roundtrip
- deterministic `BTreeMap` serialization/iteration behavior with non-sorted insertion

### 2. Export the new type from `crates/worldwake-core/src/lib.rs`

Re-export `SubstitutePreferences`.

### 3. Register `SubstitutePreferences` in `crates/worldwake-core/src/component_schema.rs`

Add a schema entry for `SubstitutePreferences` restricted to `EntityKind::Agent`.

### 4. Wire the schema-generated storage and APIs

Update imports, fixtures, and tests where required so the schema expansion compiles and remains covered:

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

- substitute-demand runtime behavior
- valuation or negotiation bundle search
- merchant restock planning
- any `worldwake-sim` or `worldwake-systems` runtime trade behavior beyond schema assertions

## Acceptance Criteria

### Tests That Must Pass

1. `SubstitutePreferences` satisfies `Clone + Eq + Debug + Serialize + DeserializeOwned + Component`.
2. `SubstitutePreferences` round-trips through bincode.
3. `SubstitutePreferences` preserves deterministic ordering for category-keyed preference storage.
4. `ComponentTables` supports insert/get/remove/has for `SubstitutePreferences`.
5. `World` accepts `SubstitutePreferences` on `EntityKind::Agent`, exposes generated get/has/remove/query APIs, and rejects insertion on non-agent kinds.
6. `ComponentKind::ALL` and `ComponentValue` inventory coverage remain accurate after adding `SubstitutePreferences`.
7. `cargo test -p worldwake-core`
8. `cargo clippy --workspace --all-targets -- -D warnings`
9. `cargo test --workspace`

### Invariants

1. `SubstitutePreferences` is authoritative stored state, not derived substitute scoring.
2. The component is valid only on `EntityKind::Agent`.
3. `preferences` uses `BTreeMap`, not `HashMap`.
4. Preference ordering is explicit in `Vec<CommodityKind>`, not implied through aliases or hidden fallback tables.
5. No floats, compatibility shims, or global substitute registries are introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/trade.rs` — trait bounds, bincode roundtrip, and deterministic map-order coverage for `SubstitutePreferences`
2. `crates/worldwake-core/src/component_tables.rs` — CRUD coverage for `SubstitutePreferences`
3. `crates/worldwake-core/src/world.rs` — agent roundtrip and wrong-kind rejection for `SubstitutePreferences`
4. `crates/worldwake-core/src/delta.rs` — authoritative component inventory assertions updated for the new component kind/value
5. `crates/worldwake-systems/tests/e09_needs_integration.rs` — expected authoritative schema inventory updated for the new trade component

### Commands

1. `cargo test -p worldwake-core substitute`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Added `SubstitutePreferences` to `crates/worldwake-core/src/trade.rs` with focused module-local trait-bound, bincode, and deterministic-order tests.
  - Added a shared `sample_substitute_preferences()` fixture in `crates/worldwake-core/src/test_utils.rs`.
  - Registered `SubstitutePreferences` as an agent-only authoritative component through `component_schema.rs`.
  - Re-exported `SubstitutePreferences` from `worldwake-core`.
  - Extended `component_tables.rs`, `world.rs`, and `delta.rs` so the schema-generated storage, world APIs, and authoritative component inventories include `SubstitutePreferences`.
  - Updated the downstream authoritative-schema assertion in `crates/worldwake-systems/tests/e09_needs_integration.rs`.
- Deviations from original plan:
  - The ticket was corrected before implementation because the original version understated the schema-driven integration path and omitted `delta.rs`, `world.rs`, shared fixtures, and downstream schema assertions from the real blast radius.
  - `component_tables.rs` did not require manual storage wiring beyond imports and tests; the schema entry generated the concrete APIs and storage fields.
  - `world_txn.rs` did not require bespoke changes; the schema entry propagated the generated transaction APIs as expected.
- Verification results:
  - `cargo test -p worldwake-core substitute` passed.
  - `cargo test -p worldwake-core` passed.
  - `cargo fmt --all --check` passed after formatting.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
