# E13DECARC-002: UtilityProfile component and registration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component in worldwake-core, schema registration
**Deps**: E13DECARC-001

## Problem

Agents need per-agent temperament weights (Principle 11) to produce diverse behavior. `UtilityProfile` stores how much each agent cares about hunger vs. thirst vs. danger vs. enterprise, etc. Without it, all agents have identical priorities.

## Assumption Reassessment (2026-03-11)

1. `Permille` exists in `worldwake-core::numerics` — confirmed.
2. `Component` trait exists in `worldwake-core::traits` — confirmed.
3. Authoritative component registration is schema-driven through `with_component_schema_entries` in `component_schema.rs`, and that schema generates the typed APIs in `ComponentTables`, `World`, `WorldTxn`, and `ComponentKind` / `ComponentValue` — confirmed.
4. Because of that schema-driven path, this ticket does not need hand-written storage or setter boilerplate unless a specific compile/test gap appears. The required manual edits are the owning module, schema entry, imports / re-exports, and the explicit tests that enumerate authoritative component inventories.
5. Entity kind validation is enforced at the generated `World::insert_component_*` layer, not in `ComponentTables` itself — confirmed.
6. Current coverage patterns for new authoritative components are broader than the original ticket described:
   - module-local trait / serialization tests in the owning module
   - `component_tables.rs` CRUD coverage
   - `world.rs` roundtrip, query/count, and wrong-kind rejection coverage
   - `world_txn.rs` delta-recording coverage for generated setters / clearers
   - `delta.rs` authoritative component inventory assertions when `ComponentKind::ALL` changes

## Architecture Check

1. `UtilityProfile` is authoritative per-agent state, not runtime planner state, so it belongs in `worldwake-core` as an `EntityKind::Agent` component.
2. Storing only stable multiplicative weights is cleaner than storing urgency, danger, or enterprise scores. Those pressures remain derived from concrete belief-visible state at decision time, which keeps causality inspectable and avoids stale duplicated state.
3. A dedicated `utility_profile.rs` module is a reasonable near-term fit because only this E13 component lands in this ticket. If E13 later adds multiple shared decision-schema components in core (`BlockedIntentMemory`, future agent-decision state carriers), a grouped domain module may become cleaner, but that broader reorganization is not justified by this ticket alone.
4. No `fear_weight`, `greed_weight`, or `sociability_weight` per spec corrections.

## Scope Correction

This ticket should:

1. Add `UtilityProfile` as a new authoritative core component with focused module-local tests.
2. Register it through `component_schema.rs` as agent-only authoritative state.
3. Re-export it from `worldwake-core`.
4. Extend the focused tests that must acknowledge a new authoritative component in `component_tables.rs`, `world.rs`, `world_txn.rs`, and `delta.rs`.

This ticket should not:

1. Implement any AI logic that reads `UtilityProfile`.
2. Add `BlockedIntentMemory` or any other E13 components.
3. Introduce stored urgency / pressure scores, compatibility aliases, or fallback fields.
4. Refactor unrelated component infrastructure or pre-emptively group future E13 schema into a larger module.

## What to Change

### 1. Define `UtilityProfile` in `worldwake-core`

Create `crates/worldwake-core/src/utility_profile.rs`:

```rust
pub struct UtilityProfile {
    pub hunger_weight: Permille,
    pub thirst_weight: Permille,
    pub fatigue_weight: Permille,
    pub bladder_weight: Permille,
    pub dirtiness_weight: Permille,
    pub pain_weight: Permille,
    pub danger_weight: Permille,
    pub enterprise_weight: Permille,
}
```

Implement `Component`, `Default` (all weights at `Permille(500)` as a balanced baseline), `Clone`, `Debug`, `Eq`, `PartialEq`, `Serialize`, `Deserialize`.

### 2. Register in component schema

Add `UtilityProfile` to `with_component_schema_entries` in `component_schema.rs` with entity-kind guard: `Agent` only.

### 3. Wire imports / re-exports and schema-driven tests

- add the module and re-export from `worldwake-core/src/lib.rs`
- import the new type where schema-generated code or explicit test inventories require it
- extend focused tests in `component_tables.rs`, `world.rs`, `world_txn.rs`, and `delta.rs`

## Files to Touch

- `crates/worldwake-core/src/utility_profile.rs` (new)
- `crates/worldwake-core/src/lib.rs`
- `crates/worldwake-core/src/component_schema.rs`
- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/world_txn.rs`
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/test_utils.rs` (only if shared fixtures reduce duplication)

## Out of Scope

- `BlockedIntentMemory` — separate ticket
- Any AI logic that reads `UtilityProfile` — later tickets
- Derived pressure computation — later ticket
- `enterprise_weight` usage / opportunity_signal derivation — later ticket

## Acceptance Criteria

### Tests That Must Pass

1. `UtilityProfile` implements `Component` (trait bound test)
2. `UtilityProfile` round-trips through bincode serialization
3. `ComponentTables` supports insert/get/remove/has for `UtilityProfile`
4. `World` accepts `UtilityProfile` on `EntityKind::Agent`, exposes generated query/count APIs, and rejects insertion on non-agent kinds
5. `UtilityProfile::default()` produces a valid instance with all weights at `Permille(500)`
6. `WorldTxn` setter / clearer coverage records the expected `ComponentDelta`
7. `delta.rs` authoritative component inventories remain complete after adding `UtilityProfile`
8. Existing suite: `cargo test --workspace`

### Invariants

1. `UtilityProfile` is only registerable on `EntityKind::Agent`
2. No stored abstract scores — only multiplicative weights
3. No `fear_weight`, `greed_weight`, or `sociability_weight` fields

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/utility_profile.rs` — module-level trait bounds, default baseline, and bincode roundtrip
2. `crates/worldwake-core/src/component_tables.rs` — CRUD coverage for `UtilityProfile`
3. `crates/worldwake-core/src/world.rs` — agent roundtrip, query/count, and wrong-kind rejection for `UtilityProfile`
4. `crates/worldwake-core/src/world_txn.rs` — setter / clearer delta coverage for `UtilityProfile`
5. `crates/worldwake-core/src/delta.rs` — authoritative component inventory coverage updated for `UtilityProfile`

### Commands

1. `cargo test -p worldwake-core utility_profile`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Added `crates/worldwake-core/src/utility_profile.rs` with the new `UtilityProfile` authoritative component, balanced `Default`, and focused module-local tests.
  - Registered `UtilityProfile` as an agent-only authoritative component through `component_schema.rs`.
  - Re-exported `UtilityProfile` from `worldwake-core`.
  - Extended `component_tables.rs`, `world.rs`, `world_txn.rs`, and `delta.rs` so the new component participates in generated storage/APIs and authoritative component inventories.
  - Added a shared deterministic fixture in `test_utils.rs`.
  - Updated `crates/worldwake-systems/tests/e09_needs_integration.rs` because that integration test hard-codes the authoritative component inventory.
- Deviations from original plan:
  - `component_tables.rs` did not require bespoke storage wiring beyond schema-driven integration; the real manual work was in explicit tests and imports.
  - `world_txn.rs`, `delta.rs`, and the E09 systems integration test all needed updates because they contain explicit authoritative-component expectations that the original ticket did not account for.
  - Workspace clippy required one narrow `#[allow(clippy::too_many_lines)]` on the existing `delta.rs` test helper after the authoritative component sample inventory grew by one entry.
- Verification results:
  - `cargo test -p worldwake-core utility_profile` passed.
  - `cargo test -p worldwake-core` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
