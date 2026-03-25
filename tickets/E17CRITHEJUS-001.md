# E17CRITHEJUS-001: Core crime types in worldwake-core

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component types, component registration
**Deps**: None (pure type additions)

## Problem

No per-agent crime disposition types exist. E17 requires `TheftDispositionProfile`, `JusticeDispositionProfile`, and `PunishmentKind` before any action or AI work can begin.

## Assumption Reassessment (2026-03-25)

1. `component_schema.rs` and `component_tables.rs` use a macro-generated registration pattern for Agent-only components (e.g., `CombatProfile`, `UtilityProfile`, `ViolationDispositionProfile`). New profiles follow the same pattern.
2. `Permille` newtype exists in `crates/worldwake-core/src/numerics.rs` and is used across all profile types.
3. `CommodityKind` and `Quantity` exist in `crates/worldwake-core/src/items.rs`.
4. Not an AI ticket — pure type definitions.
5. N/A — no ordering dependencies.
6. N/A — no heuristic changes.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatches found.
12. N/A.

## Architecture Check

1. A new `crime.rs` module in worldwake-core follows the established pattern of domain-specific type modules (`combat.rs`, `trade.rs`, `needs.rs`). Profiles are Agent-only components registered in the schema, consistent with `CombatProfile`, `ViolationDispositionProfile`, etc.
2. No backwards-compatibility aliasing introduced.

## Verification Layers

1. `TheftDispositionProfile` round-trips through serde -> focused unit test in `crime.rs`
2. `JusticeDispositionProfile` round-trips through serde -> focused unit test in `crime.rs`
3. `PunishmentKind` round-trips through serde -> focused unit test in `crime.rs`
4. Component registration accepts both profiles on Agent entities and rejects on non-Agent entities -> focused unit test in `component_schema.rs`
5. Single-crate ticket; no cross-layer mapping needed.

## What to Change

### 1. New `crime.rs` module in worldwake-core

Create `crates/worldwake-core/src/crime.rs` with:

- `TheftDispositionProfile` struct: `steal_duration_ticks: NonZeroU32`, `theft_motive_weight: Permille`, `witness_risk_penalty: Permille`. Derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Implement `Component` trait.
- `JusticeDispositionProfile` struct: `accusation_motive_weight: Permille`, `fine_severity: Permille`. Same derives + `Component`.
- `PunishmentKind` enum: `Fine { commodity: CommodityKind, amount: Quantity }`, `Exile { from_faction: EntityId }`. Derive `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`.

### 2. Register components in component_schema.rs and component_tables.rs

Add `TheftDispositionProfile` and `JusticeDispositionProfile` as Agent-only components following the existing registration macro pattern.

### 3. Export from lib.rs

Add `pub mod crime;` and re-export the three types.

## Files to Touch

- `crates/worldwake-core/src/crime.rs` (new)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify)

## Out of Scope

- ViolationKind extensions (E17CRITHEJUS-002)
- InstitutionalClaim extensions (E17CRITHEJUS-003)
- GoalKind extensions (E17CRITHEJUS-004)
- Any worldwake-sim, worldwake-systems, or worldwake-ai changes
- Action definitions or handlers
- AI candidate generation or planner integration
- Golden tests

## Acceptance Criteria

### Tests That Must Pass

1. `TheftDispositionProfile` serde round-trip preserves all fields
2. `JusticeDispositionProfile` serde round-trip preserves all fields
3. `PunishmentKind::Fine` and `PunishmentKind::Exile` serde round-trip
4. Both profiles accepted as components on `EntityKind::Agent`
5. Both profiles rejected on non-Agent entity kinds
6. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. All new types derive the full standard set (`Clone, Debug, Eq, PartialEq, Serialize, Deserialize`)
2. `PunishmentKind` additionally derives `Copy, Ord, PartialOrd` (used in `GoalKind` discriminant ordering)
3. No `HashMap`/`HashSet` introduced (determinism invariant)
4. No floats introduced (determinism invariant)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/crime.rs` — unit tests for construction and serde round-trip
2. `crates/worldwake-core/src/component_schema.rs` — registration test for both profiles on Agent vs non-Agent

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy -p worldwake-core`
