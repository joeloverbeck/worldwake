# S40REMPUR-001: Add PursuitProfile component to worldwake-core

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component type, component table entry, schema registration
**Deps**: None (foundational for S40REMPUR-002..007)

## Problem

Agents have no per-agent parameters governing whether or when to attempt remote pursuit. The spec requires a `PursuitProfile` component with `min_location_confidence: Permille` and `max_pursuit_travel_ticks: NonZeroU32` so pursuit willingness is profile-driven (Principle 2, Principle 22) rather than hardcoded.

## Assumption Reassessment (2026-03-30)

1. `PursuitProfile` does not exist anywhere in the codebase (`grep -r PursuitProfile` finds only `specs/S40-remote-pursuit.md` and `specs/S42-per-agent-reasoning-style.md`).
2. Component registration uses the `with_component_schema_entries!` macro in `crates/worldwake-core/src/component_schema.rs`, forwarded through `forward_authoritative_components` in `component_tables.rs:112-118`.
3. `EntityKind::Agent` already accepts many components (e.g., `CombatProfile`, `UtilityProfile`, `BlockedIntentMemory`). Adding `PursuitProfile` follows the identical pattern.
4. `Permille` is the project-standard [0,1000] newtype (`crates/worldwake-core/src/numerics.rs`). `NonZeroU32` is from `std::num`.
5. The component must derive `Serialize, Deserialize` for save/load and `Component` via the project's trait.
6. No adjacent contradictions exposed.

## Architecture Check

1. A dedicated component is cleaner than embedding pursuit fields in `CombatProfile` or `UtilityProfile`, because pursuit is domain-neutral (used by both combat and justice pursuit). Separate component respects single responsibility.
2. No backwards-compatibility shims. This is a new addition.

## Verification Layers

1. Component registration completeness → focused unit test: round-trip set/get on `World`
2. Schema registration → compile-time: `World::get::<PursuitProfile>(agent)` must compile
3. Save/load survival → existing `save_load` infrastructure covers any registered component; no special test needed beyond round-trip
4. Single-layer ticket (new type + registration); no cross-system mapping needed.

## What to Change

### 1. Define `PursuitProfile` struct

In a new or existing module in `worldwake-core` (likely `crates/worldwake-core/src/pursuit.rs` or inline in `crates/worldwake-core/src/components.rs` — follow existing pattern):

```rust
use crate::{Component, Permille};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PursuitProfile {
    pub min_location_confidence: Permille,
    pub max_pursuit_travel_ticks: NonZeroU32,
}
```

Implement `Component` trait for `PursuitProfile`.

### 2. Register in component schema

Add `PursuitProfile` to the `with_component_schema_entries!` macro invocation in `component_schema.rs`, associated with `EntityKind::Agent`.

### 3. Register in component tables

Add `PursuitProfile` to the `forward_authoritative_components` macro list in `component_tables.rs` so typed storage is generated.

### 4. Re-export from `lib.rs`

Ensure `PursuitProfile` is publicly exported from `worldwake_core`.

## Files to Touch

- `crates/worldwake-core/src/pursuit.rs` (new) — or inline in existing components module
- `crates/worldwake-core/src/component_schema.rs` (modify) — register on Agent
- `crates/worldwake-core/src/component_tables.rs` (modify) — typed storage entry
- `crates/worldwake-core/src/lib.rs` (modify) — re-export

## Out of Scope

- Any AI-side logic that reads `PursuitProfile` (that is S40REMPUR-002+)
- Candidate generation changes
- Belief-view integration for exposing the profile to the AI crate
- Default profile values for existing agent setups (tests will set explicit values)
- Guard/justice-specific pursuit parameters (same `PursuitProfile` shape serves both domains per spec)

## Acceptance Criteria

### Tests That Must Pass

1. Round-trip test: set `PursuitProfile` on an Agent entity, retrieve it, assert fields match.
2. Compile-time: `World::get::<PursuitProfile>(agent_id)` compiles and returns `Option<&PursuitProfile>`.
3. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `PursuitProfile` is only registerable on `EntityKind::Agent` (schema enforcement).
2. `PursuitProfile` survives save/load round-trip (serde derive).
3. No stored confidence value — `min_location_confidence` is a threshold, not a cached score.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/pursuit.rs` (or relevant test module) — `test_pursuit_profile_round_trip`: create World, spawn Agent, set PursuitProfile, get it back, assert equality.

### Commands

1. `cargo test -p worldwake-core test_pursuit_profile`
2. `cargo clippy -p worldwake-core && cargo test -p worldwake-core`
