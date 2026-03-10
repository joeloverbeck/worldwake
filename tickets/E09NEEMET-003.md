# E09NEEMET-003: HomeostaticNeeds, DeprivationExposure, and MetabolismProfile components

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new core types, component registration
**Deps**: E09NEEMET-002 (DriveThresholds must exist for deprivation threshold references)

## Problem

Agents need concrete physiological body state tracked as authoritative components. This ticket adds the three core physiology components: `HomeostaticNeeds` (current body state), `DeprivationExposure` (sustained critical-pressure counters), and `MetabolismProfile` (per-agent physiological parameters). These are the data foundation for the metabolism system.

## Assumption Reassessment (2026-03-10)

1. `Permille` exists in `numerics.rs` with validated range 0..=1000 — confirmed.
2. Component registration pattern requires 15 method names per component — confirmed.
3. `NonZeroU32` is available from `std::num` — no external dep needed.
4. The spec requires `MetabolismProfile` to use `NonZeroU32` for tolerance/duration fields to prevent zero-division.

## Architecture Check

1. All three components go on `EntityKind::Agent` — consistent with existing patterns.
2. `HomeostaticNeeds` is mutable body state (updated each tick). `MetabolismProfile` is effectively static per-agent config seeded at creation. `DeprivationExposure` is mutable counter state.
3. Placing in `worldwake-core` ensures systems crate can read/write them without circular deps.

## What to Change

### 1. New module `crates/worldwake-core/src/needs.rs`

```rust
pub struct HomeostaticNeeds {
    pub hunger: Permille,     // 0=sated, 1000=starving
    pub thirst: Permille,     // 0=hydrated, 1000=dehydrated
    pub fatigue: Permille,    // 0=rested, 1000=exhausted
    pub bladder: Permille,    // 0=empty, 1000=desperate
    pub dirtiness: Permille,  // 0=clean, 1000=filthy
}
impl Component for HomeostaticNeeds {}

pub struct DeprivationExposure {
    pub hunger_critical_ticks: u32,
    pub thirst_critical_ticks: u32,
    pub fatigue_critical_ticks: u32,
    pub bladder_critical_ticks: u32,
}
impl Component for DeprivationExposure {}

pub struct MetabolismProfile {
    // Basal rates per tick
    pub hunger_rate: Permille,
    pub thirst_rate: Permille,
    pub fatigue_rate: Permille,
    pub bladder_rate: Permille,
    pub dirtiness_rate: Permille,
    // Recovery / tolerance
    pub rest_efficiency: Permille,
    pub starvation_tolerance_ticks: NonZeroU32,
    pub dehydration_tolerance_ticks: NonZeroU32,
    pub exhaustion_collapse_ticks: NonZeroU32,
    pub bladder_accident_tolerance_ticks: NonZeroU32,
    pub toilet_ticks: NonZeroU32,
    pub wash_ticks: NonZeroU32,
}
impl Component for MetabolismProfile {}
```

Include `HomeostaticNeeds::new_sated()` (all zeros) and `MetabolismProfile::default_human()` constructors.

### 2. Register all three in `component_schema.rs`

Add macro blocks for `HomeostaticNeeds`, `DeprivationExposure`, and `MetabolismProfile`, all restricted to `EntityKind::Agent`.

### 3. Export from `lib.rs`

## Files to Touch

- `crates/worldwake-core/src/needs.rs` (new)
- `crates/worldwake-core/src/component_schema.rs` (modify — add 3 component registrations)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- The metabolism system tick logic (E09NEEMET-005)
- Deprivation consequence logic (E09NEEMET-006)
- Consumable profiles on commodities (E09NEEMET-004)
- DriveThresholds (E09NEEMET-002)
- WoundList (E09NEEMET-001)

## Acceptance Criteria

### Tests That Must Pass

1. `HomeostaticNeeds` can be inserted/retrieved/removed on Agent entities.
2. `HomeostaticNeeds` insertion rejected for non-Agent kinds.
3. `DeprivationExposure` can be inserted/retrieved/removed on Agent entities.
4. `MetabolismProfile` can be inserted/retrieved/removed on Agent entities.
5. `HomeostaticNeeds::new_sated()` has all fields at `Permille(0)`.
6. All three types round-trip through bincode serialization.
7. `MetabolismProfile` fields using `NonZeroU32` cannot be zero.
8. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. All `Permille` fields stay within 0..=1000 range (enforced by type).
2. `MetabolismProfile` tolerance fields are `NonZeroU32` — no division by zero possible.
3. Components restricted to `EntityKind::Agent`.
4. No floating-point types used.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/needs.rs` (unit tests) — construction, defaults, serialization, trait bounds
2. Component table integration tests for all three components

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
