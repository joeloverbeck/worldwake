# E13DECARC-002: UtilityProfile component and registration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component in worldwake-core, schema registration
**Deps**: E13DECARC-001

## Problem

Agents need per-agent temperament weights (Principle 11) to produce diverse behavior. `UtilityProfile` stores how much each agent cares about hunger vs. thirst vs. danger vs. enterprise, etc. Without it, all agents have identical priorities.

## Assumption Reassessment (2026-03-11)

1. `Permille` exists in `worldwake-core::numerics` — confirmed.
2. `Component` trait exists in `worldwake-core::traits` — confirmed.
3. Component registration uses the `with_component_schema_entries` macro in `component_schema.rs` — confirmed.
4. `ComponentTables` is generated from the macro in `component_tables.rs` — confirmed.
5. Entity kind validation restricts components to appropriate entity kinds — confirmed.

## Architecture Check

1. `UtilityProfile` is authoritative per-agent state (not derived), so it belongs as a registered component on `EntityKind::Agent`.
2. No abstract scores stored — only multiplicative weights. Derived pressures are computed at decision time.
3. No `fear_weight`, `greed_weight`, or `sociability_weight` per spec corrections.

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

Add `UtilityProfile` to the `with_component_schema_entries` macro in `component_schema.rs` with entity-kind guard: `Agent` only.

### 3. Export from `worldwake-core/src/lib.rs`

Add `pub mod utility_profile;` and re-export `UtilityProfile`.

## Files to Touch

- `crates/worldwake-core/src/utility_profile.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-export)
- `crates/worldwake-core/src/component_schema.rs` (modify — add entry)
- `crates/worldwake-core/src/component_tables.rs` (modify — add storage field)

## Out of Scope

- `BlockedIntentMemory` — separate ticket
- Any AI logic that reads `UtilityProfile` — later tickets
- Derived pressure computation — later ticket
- `enterprise_weight` usage / opportunity_signal derivation — later ticket

## Acceptance Criteria

### Tests That Must Pass

1. `UtilityProfile` implements `Component` (trait bound test)
2. `UtilityProfile` round-trips through bincode serialization
3. `UtilityProfile` can be inserted on an `Agent` entity via `WorldTxn`
4. `UtilityProfile` insertion on a non-Agent entity returns an error
5. `UtilityProfile::default()` produces a valid instance with all weights at `Permille(500)`
6. Existing suite: `cargo test --workspace`

### Invariants

1. `UtilityProfile` is only registerable on `EntityKind::Agent`
2. No stored abstract scores — only multiplicative weights
3. No `fear_weight`, `greed_weight`, or `sociability_weight` fields

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/utility_profile.rs` — module-level tests for bounds, default, bincode roundtrip
2. `crates/worldwake-core/src/component_schema.rs` (or test module) — schema registration test for `UtilityProfile` on Agent

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
