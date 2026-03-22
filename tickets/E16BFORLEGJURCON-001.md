# E16BFORLEGJURCON-001: Add OfficeForceProfile and OfficeForceState components

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — core component types, component tables, component schema
**Deps**: E16 (offices exist), E16c (institutional infra exists)

## Problem

Force-succession offices lack explicit per-office timing parameters and temporal continuity tracking. The spec requires `OfficeForceProfile` (policy) and `OfficeForceState` (mutable continuity) as separate components on `EntityKind::Office`.

## Assumption Reassessment (2026-03-22)

1. `OfficeData` and `SuccessionLaw::Force` exist in `crates/worldwake-core/src/offices.rs`. `OfficeForceProfile` and `OfficeForceState` do not exist anywhere in the codebase.
2. Component registration follows the `with_component_schema_entries!` macro in `component_schema.rs` and typed storage in `component_tables.rs`.
3. Not a planner or golden ticket — pure data-layer addition.
4. N/A — not an AI regression ticket.
5. N/A — no ordering dependency.
6. N/A — no heuristic removal.
7. N/A — not a start-failure ticket.
8. N/A — not a political office-claim closure ticket (that's later tickets).
9. N/A — no ControlSource manipulation.
10. N/A — no golden scenario.
11. No mismatches found.
12. N/A — no cumulative arithmetic.

## Architecture Check

1. Two separate components (profile = immutable policy, state = mutable tracking) follow the existing pattern of `OfficeData` + mutable state separation. Controller identity lives in a relation (ticket -002), not in this component, avoiding dual authoritative sources (Principle 26).
2. No backward-compatibility shims. These are net-new types.

## Verification Layers

1. `OfficeForceProfile` attached only to `EntityKind::Office` → focused unit test on component schema predicate
2. `OfficeForceState` attached only to `EntityKind::Office` → focused unit test on component schema predicate
3. Round-trip serde for both types → focused unit test
4. Single-layer ticket (data types + registration). No additional layer mapping needed.

## What to Change

### 1. Define types in `offices.rs`

Add `OfficeForceProfile` and `OfficeForceState` structs as specified:

```rust
pub struct OfficeForceProfile {
    pub uncontested_hold_ticks: NonZeroU32,
    pub vacancy_claim_grace_ticks: NonZeroU32,
    pub challenger_presence_grace_ticks: NonZeroU32,
}

pub struct OfficeForceState {
    pub control_since: Option<Tick>,
    pub contested_since: Option<Tick>,
    pub last_uncontested_tick: Option<Tick>,
}
```

Both must derive `Clone, Debug, Serialize, Deserialize` and implement the `Component` trait.

### 2. Register in component tables

Add `office_force_profile` and `office_force_state` storage fields to `ComponentTables` in `component_tables.rs`, following the existing pattern for `OfficeData`.

### 3. Register in component schema

Add entries to `with_component_schema_entries!` with predicate `kind == EntityKind::Office`.

### 4. Expose via World

Add getter/setter methods on `World` for the new components, following the existing `office_data()` / `set_office_data()` pattern.

## Files to Touch

- `crates/worldwake-core/src/offices.rs` (modify — add types)
- `crates/worldwake-core/src/component_tables.rs` (modify — add storage fields)
- `crates/worldwake-core/src/component_schema.rs` (modify — register components)
- `crates/worldwake-core/src/world.rs` or relevant world submodule (modify — add accessors)

## Out of Scope

- Relations (`contests_office`, `office_controller`) — that's E16BFORLEGJURCON-002
- WorldTxn helpers — that's E16BFORLEGJURCON-002
- Action payloads/handlers — later tickets
- AI integration — later tickets
- Removing `resolve_force_succession` — that's E16BFORLEGJURCON-005

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `OfficeForceProfile` can be attached to an `EntityKind::Office` entity and read back
2. Unit test: `OfficeForceState` can be attached to an `EntityKind::Office` entity and read back
3. Unit test: attempting to attach either component to a non-Office entity is rejected by schema
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `OfficeForceProfile` and `OfficeForceState` are only attachable to `EntityKind::Office`
2. All values use integer/newtype types (`NonZeroU32`, `Tick`) — no floats
3. `OfficeForceState` contains only temporal continuity data, never controller identity (Principle 26)
4. No existing tests break

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/offices.rs` (or test module) — schema predicate and round-trip tests for both new components

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
