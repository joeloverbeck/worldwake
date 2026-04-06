# S56PEREXP-003: Add `ObservationContext` type and `PlaceVisibilityProfile` component

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new component in ECS, new type in core
**Deps**: S56PEREXP-001

## Problem

S56 requires a struct to compute effective observation fidelity from multiple modifiers, and a place-level component to model concealment. Neither exists yet.

## Assumption Reassessment (2026-04-06)

1. `Permille` at `numerics.rs:25` derives `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize`. New types using `Permille` fields inherit these requirements.
2. Component registration uses `with_component_schema_entries!` macro in `component_schema.rs`. Place-specific components use predicate `|kind| kind == EntityKind::Place`. Existing examples: `BanditCamp` (line 1185), `SceneEvidence` (line 1210).
3. Per tickets/README.md item 13: macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`) must import the new type.
4. `ObservationContext` is a transient derived struct (not stored as a component) — it lives in `worldwake-core` as a pure data type with `Permille` arithmetic.
5. `PlaceVisibilityProfile` must derive `Clone, Debug, Serialize, Deserialize` at minimum to satisfy the component macro requirements.

## Architecture Check

1. `ObservationContext` in `worldwake-core` is correct: pure data + `Permille` arithmetic, no system dependencies. Accessible from all crates.
2. `PlaceVisibilityProfile` as a stored component on `EntityKind::Place` follows the existing pattern for place-specific data. No backward-compatibility shims.

## Verification Layers

1. `effective_fidelity()` arithmetic correctness -> focused unit tests
2. `PlaceVisibilityProfile` registered on `EntityKind::Place` -> compilation + component schema test
3. `ObservationContext` all-zero penalties -> effective equals base -> unit test
4. Single-layer ticket (type definitions) — no decision/action trace needed.

## What to Change

### 1. Add `ObservationContext` to `worldwake-core`

Create a new file `crates/worldwake-core/src/observation_context.rs` or add to an existing perception-related module:

```rust
use crate::Permille;

#[derive(Copy, Clone, Debug)]
pub struct ObservationContext {
    pub base_fidelity: Permille,
    pub fatigue_penalty: Permille,
    pub occupancy_penalty: Permille,
    pub place_concealment: Permille,
    pub entity_concealment: Permille,
}

impl ObservationContext {
    pub fn effective_fidelity(&self) -> Permille {
        let mut f = u32::from(self.base_fidelity.value());
        f = f * (1000 - u32::from(self.fatigue_penalty.value())) / 1000;
        f = f * (1000 - u32::from(self.occupancy_penalty.value())) / 1000;
        let concealment = u32::from(self.place_concealment.value())
            .max(u32::from(self.entity_concealment.value()));
        f = f * (1000 - concealment) / 1000;
        Permille::new_unchecked(f.min(1000) as u16)
    }
}
```

Export from `crates/worldwake-core/src/lib.rs`.

### 2. Add `PlaceVisibilityProfile` component

Define the struct (in a new or existing module in `worldwake-core`):

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlaceVisibilityProfile {
    pub base_concealment: Permille,
}
```

Export from `crates/worldwake-core/src/lib.rs`.

### 3. Register `PlaceVisibilityProfile` in component schema

Add a registration block in `crates/worldwake-core/src/component_schema.rs` with predicate `|kind| kind == EntityKind::Place`, following the `BanditCamp` pattern.

### 4. Add imports at macro expansion sites

Ensure `PlaceVisibilityProfile` is imported (or use path) in:
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/component_tables.rs`

### 5. Unit tests for `effective_fidelity()`

- All penalties zero -> effective == base
- Single penalty (fatigue 300) -> correct multiplication
- All penalties max (1000) -> effective == 0
- Zero base fidelity -> effective == 0 regardless
- Multiplicative stacking: fidelity 800, fatigue penalty 120, occupancy 400, concealment 400 -> expected value
- Place concealment vs entity concealment: max is used

## Files to Touch

- `crates/worldwake-core/src/observation_context.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — export new types)
- `crates/worldwake-core/src/component_schema.rs` (modify — register PlaceVisibilityProfile)
- `crates/worldwake-core/src/delta.rs` (modify — import)
- `crates/worldwake-core/src/world.rs` (modify — import)
- `crates/worldwake-core/src/component_tables.rs` (modify — import)

## Out of Scope

- Perception system integration (S56PEREXP-004)
- Scenario integration for `PlaceVisibilityProfile` (S56PEREXP-005)
- Fatigue penalty function (S56PEREXP-004)

## Acceptance Criteria

### Tests That Must Pass

1. `effective_fidelity()` returns correct values for all penalty combinations
2. `PlaceVisibilityProfile` can be set/get on Place entities via WorldTxn
3. `cargo build --workspace` compiles
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `effective_fidelity() <= base_fidelity` for all inputs (penalties can only reduce)
2. Zero base fidelity -> zero effective fidelity regardless of other factors
3. `PlaceVisibilityProfile` is only settable on `EntityKind::Place` entities

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/observation_context.rs` — unit tests for `effective_fidelity()` arithmetic
2. Component schema tests (existing pattern) — verify PlaceVisibilityProfile registration

### Commands

1. `cargo test -p worldwake-core -- observation_context`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
