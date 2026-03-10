# E11TRAECO-003: Add `DemandMemory`, `DemandObservation`, and `DemandObservationReason`

**Status**: COMPLETED

**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new trade-domain authoritative component and value types in `worldwake-core`
**Deps**: E11TRAECO-002 complete; E11 trade schema work in progress

## Problem

E11 needs concrete, inspectable state for unmet local demand and missed sale opportunities. Without a first-class `DemandMemory`, later merchant restock, valuation, and unmet-demand recording would have to infer intent from transient world snapshots or introduce hidden market-state shortcuts. That would be weaker than the current architecture goal of explicit state carried forward through time.

## Assumption Reassessment (2026-03-11)

1. `crates/worldwake-core/src/trade.rs` already exists from E11TRAECO-002 and currently contains `MerchandiseProfile` plus module-local tests.
2. The original ticket understated how authoritative components are integrated in this repo. `component_schema.rs` is the source of truth, and that schema fans out into generated APIs for `ComponentTables`, `World`, `WorldTxn`, and `delta` component inventories.
3. Because of that schema-driven architecture, `DemandMemory` should be added by extending the existing schema path, not by introducing bespoke storage or parallel registration code.
4. Kind restrictions are enforced by generated `World::insert_component_*` methods, not by `ComponentTables`, so world-level tests are required to verify agent-only registration.
5. Existing coverage patterns for new authoritative components are stronger than the original ticket described:
   - module-local trait and serialization tests in the owning domain module
   - `component_tables.rs` CRUD coverage
   - `world.rs` roundtrip and wrong-kind rejection coverage
   - `delta.rs` authoritative component inventory coverage when a new component expands `ComponentKind`
6. The dependency on E11TRAECO-002 remains correct, but the live reference is now the archived completed ticket under `archive/tickets/completed/`.
7. `DemandObservation` and `DemandObservationReason` are plain trade-domain value types, not components; only `DemandMemory` participates in authoritative component registration.

## Architecture Check

1. Keeping `DemandMemory`, `DemandObservation`, and `DemandObservationReason` in `crates/worldwake-core/src/trade.rs` is cleaner than spreading them across generic component files. E11 is building a coherent trade-domain schema, and the module should continue to own that domain surface.
2. `DemandMemory { observations: Vec<DemandObservation> }` is still the right shape for Phase 2. Observation order is chronological state, not a derived score, and later pruning can operate on concrete entries without introducing a parallel index or summary cache.
3. The current schema-driven component architecture is stronger than the ticket originally assumed. The robust move is to extend that path and let generated APIs stay consistent across storage, world access, transactions, and delta typing.
4. There is no architectural benefit in prematurely abstracting demand memory into a separate subsystem, alias type, or helper registry at this stage. That would add indirection without improving extensibility.
5. A broader future improvement may be to centralize trade-domain sample fixtures used across `trade.rs`, `world.rs`, and `delta.rs` tests if E11 adds repeated literals, but this ticket should stay narrowly scoped unless duplication becomes material.

## Scope Correction

This ticket should:

1. Add `DemandMemory`, `DemandObservation`, and `DemandObservationReason` to `crates/worldwake-core/src/trade.rs`.
2. Register `DemandMemory` as an agent-only authoritative component through `component_schema.rs`.
3. Re-export the new types from `worldwake-core`.
4. Extend focused tests at the module, component-table, world API, and authoritative component inventory levels.

This ticket should not:

1. Implement demand-memory aging or pruning logic. That belongs to E11TRAECO-009.
2. Record observations from trade failures or unmet demand. That also belongs to E11TRAECO-009 and later handler work.
3. Add `TradeDispositionProfile` or `SubstitutePreferences`. Those remain separate tickets.
4. Introduce aggregate demand scores, hidden market-state caches, compatibility aliases, or speculative abstractions.

## What to Change

### 1. Extend `crates/worldwake-core/src/trade.rs`

Define:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DemandMemory {
    pub observations: Vec<DemandObservation>,
}

impl Component for DemandMemory {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DemandObservation {
    pub commodity: CommodityKind,
    pub quantity: Quantity,
    pub place: EntityId,
    pub tick: Tick,
    pub counterparty: Option<EntityId>,
    pub reason: DemandObservationReason,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum DemandObservationReason {
    WantedToBuyButNoSeller,
    WantedToBuyButSellerOutOfStock,
    WantedToBuyButTooExpensive,
    WantedToSellButNoBuyer,
}
```

Include focused module-local tests for:

- trait bounds on `DemandMemory` and `DemandObservationReason`
- bincode roundtrip for `DemandObservation`

### 2. Export the new types from `crates/worldwake-core/src/lib.rs`

Re-export:

- `DemandMemory`
- `DemandObservation`
- `DemandObservationReason`

### 3. Register `DemandMemory` in `crates/worldwake-core/src/component_schema.rs`

Add a schema entry for `DemandMemory` restricted to `EntityKind::Agent`.

### 4. Wire the schema-generated storage and APIs

Update imports and tests where required so the schema expansion compiles and remains covered:

- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/delta.rs`

`world_txn.rs` should not need bespoke logic if the schema entry is correct.

## Files to Touch

- `crates/worldwake-core/src/trade.rs`
- `crates/worldwake-core/src/lib.rs`
- `crates/worldwake-core/src/component_schema.rs`
- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/delta.rs`

## Out of Scope

- trade-system tick behavior
- valuation or negotiation logic
- merchant restock planning
- substitute demand
- any `worldwake-sim` or `worldwake-systems` behavior changes

## Acceptance Criteria

### Tests That Must Pass

1. `DemandMemory` satisfies `Clone + Eq + Debug + Serialize + DeserializeOwned + Component`.
2. `DemandObservation` round-trips through bincode.
3. `DemandObservationReason` satisfies `Copy + Clone + Eq + Ord + Hash + Debug + Serialize + DeserializeOwned`.
4. `ComponentTables` supports insert/get/remove/has for `DemandMemory`.
5. `World` accepts `DemandMemory` on `EntityKind::Agent`, exposes generated get/has/remove/query APIs, and rejects insertion on non-agent kinds.
6. `ComponentKind::ALL` and `ComponentValue` inventory coverage remain accurate after adding `DemandMemory`.
7. `cargo test -p worldwake-core`
8. `cargo clippy --workspace --all-targets -- -D warnings`
9. `cargo test --workspace`

### Invariants

1. `DemandMemory` is authoritative stored state, not a derived market score.
2. Only `DemandMemory` is an authoritative component; `DemandObservation` and `DemandObservationReason` remain plain value types.
3. `DemandMemory` is valid only on `EntityKind::Agent`.
4. No `HashMap`/`HashSet`, floats, compatibility aliases, or hidden market-state abstractions are introduced.
5. Observation order remains concrete chronological state via `Vec`, not an abstract aggregate.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/trade.rs` — trait bounds and bincode roundtrip for the new demand-memory types
2. `crates/worldwake-core/src/component_tables.rs` — CRUD coverage for `DemandMemory`
3. `crates/worldwake-core/src/world.rs` — agent roundtrip and wrong-kind rejection for `DemandMemory`
4. `crates/worldwake-core/src/delta.rs` — authoritative component inventory assertions updated for the new component kind

### Commands

1. `cargo test -p worldwake-core demand`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Added `DemandMemory`, `DemandObservation`, and `DemandObservationReason` to `crates/worldwake-core/src/trade.rs`.
  - Registered `DemandMemory` as an agent-only authoritative component through `component_schema.rs`.
  - Re-exported the new trade-domain types from `worldwake-core`.
  - Extended `component_tables.rs`, `world.rs`, and `delta.rs` so the schema-generated storage, world APIs, and authoritative component inventories include `DemandMemory`.
  - Updated the downstream schema assertion in `crates/worldwake-systems/tests/e09_needs_integration.rs` to reflect the legitimate addition of a new authoritative component kind.
- Deviations from original plan:
  - The ticket was corrected before implementation because the original version understated the schema-driven component architecture and omitted the authoritative component inventory impact in `delta.rs`.
  - `world_txn.rs` did not require bespoke changes; the schema entry propagated the generated transaction APIs as expected.
  - Workspace verification exposed a downstream hard-coded component inventory assertion outside `worldwake-core`; that test needed updating even though the ticket originally scoped only core files.
- Verification results:
  - `cargo test -p worldwake-core demand` passed.
  - `cargo test -p worldwake-core merchandise_profile` passed.
  - `cargo test -p worldwake-core` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
