# E09NEEMET-003: HomeostaticNeeds, DeprivationExposure, and MetabolismProfile components

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new core types, component registration
**Deps**: E09NEEMET-002 (DriveThresholds must exist for deprivation threshold references)

## Problem

Agents need concrete physiological body state tracked as authoritative components. This ticket adds the three core physiology components: `HomeostaticNeeds` (current body state), `DeprivationExposure` (sustained critical-pressure counters), and `MetabolismProfile` (per-agent physiological parameters). These are the data foundation for the metabolism system.

## Assumption Reassessment (2026-03-10)

1. `Permille` exists in `numerics.rs` with validated range 0..=1000 — confirmed.
2. `DriveThresholds` already exists in `crates/worldwake-core/src/drives.rs` and is already registered as an agent-only authoritative component. This ticket must integrate with that existing shared schema rather than assume physiology starts from an empty core surface.
3. `worldwake-core` authoritative components are schema-driven, not local-only additions. Adding a new component requires updating the `with_authoritative_components!` inventory in `component_schema.rs`, which then fans out into generated APIs in `component_tables.rs`, `world.rs`, and `delta.rs`; the consuming files must import the new types and extend test coverage accordingly.
4. `NonZeroU32` is available from `std::num` — no external dep needed.
5. Kind restrictions are enforced by `World`'s generated typed insertion API, not by raw `ComponentTables`. Wrong-kind rejection tests therefore belong in `crates/worldwake-core/src/world.rs`.
6. The spec requires `MetabolismProfile` tolerance/duration fields to use `NonZeroU32`. In practice this protects against impossible zero-duration / zero-tolerance states and keeps downstream logic free of sentinel zero handling.
7. The current core test pattern for new authoritative components is:
   - value-type construction / serialization tests in the module that defines the types
   - table CRUD coverage in `component_tables.rs`
   - kind-restricted world API coverage in `world.rs`
   - authoritative component inventory coverage in `delta.rs`

## Architecture Check

1. All three components go on `EntityKind::Agent` — consistent with existing patterns.
2. `HomeostaticNeeds` is mutable body state (updated each tick). `MetabolismProfile` is effectively static per-agent config seeded at creation. `DeprivationExposure` is mutable counter state.
3. Placing in `worldwake-core` ensures systems crate can read/write them without circular deps.
4. A dedicated `needs.rs` module is cleaner than expanding `components.rs`. The crate already groups domain types into focused modules (`drives.rs`, `wounds.rs`, `items.rs`, `topology.rs`) rather than a single catch-all component file.
5. The more robust baseline API is:
   - `HomeostaticNeeds::new_sated()` for explicit zeroed body state
   - `Default` for `HomeostaticNeeds` and `DeprivationExposure`, since all-zero state is canonical
   - `MetabolismProfile::new(...)` for explicit construction
   - `Default` for the canonical baseline metabolism profile
   This is cleaner than introducing a bespoke `default_human()` alias.
6. No compatibility aliases or parallel physiology schemas are justified. These components should enter the authoritative schema as first-class types immediately.

## Scope Correction

This ticket should:

1. Add the three physiology component types in `worldwake-core`.
2. Register them as agent-only authoritative components through the existing schema macro.
3. Extend the generated authoritative component surface so these types participate in world APIs, component tables, and delta typing the same way existing components do.
4. Add focused tests that match the current `worldwake-core` testing pattern.

This ticket should not:

1. Implement metabolism ticking, deprivation effects, or consumption logic.
2. Introduce compatibility constructors, alias types, or temporary duplicate schemas.
3. Refactor the broader authoritative-component architecture beyond the narrow schema fanout already required by existing patterns.

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

Include `HomeostaticNeeds::new_sated()` (all zeros), derive or implement `Default` for `HomeostaticNeeds` and `DeprivationExposure`, and provide `MetabolismProfile::new(...)` plus `Default` for the canonical baseline profile.

### 2. Register all three in `component_schema.rs`

Add macro blocks for `HomeostaticNeeds`, `DeprivationExposure`, and `MetabolismProfile`, all restricted to `EntityKind::Agent`.

### 3. Export from `lib.rs`

### 4. Extend the existing schema fanout imports and coverage

Because `component_tables.rs`, `world.rs`, and `delta.rs` consume the schema-generated component inventory, this ticket also needs the minimal import and test updates required for those files to compile and exercise the new components correctly.

## Files to Touch

- `crates/worldwake-core/src/needs.rs` (new)
- `crates/worldwake-core/src/component_schema.rs` (modify — add 3 component registrations)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)
- `crates/worldwake-core/src/component_tables.rs` (modify — imports/tests for schema fanout)
- `crates/worldwake-core/src/world.rs` (modify — imports/tests for generated world API coverage)
- `crates/worldwake-core/src/delta.rs` (modify — imports/component inventory coverage)

## Out of Scope

- The metabolism system tick logic (E09NEEMET-005)
- Deprivation consequence logic (E09NEEMET-006)
- Consumable profiles on commodities (E09NEEMET-004)
- DriveThresholds (E09NEEMET-002)
- WoundList (E09NEEMET-001)

## Acceptance Criteria

### Tests That Must Pass

1. `HomeostaticNeeds`, `DeprivationExposure`, and `MetabolismProfile` satisfy the required component/value trait bounds and round-trip through bincode.
2. `HomeostaticNeeds::new_sated()` produces all-zero body state.
3. `HomeostaticNeeds`, `DeprivationExposure`, and `MetabolismProfile` can be inserted/retrieved/removed on Agent entities through the `World` API.
4. Insertion of each of the three components is rejected for non-Agent kinds through the `World` API.
5. `HomeostaticNeeds::new_sated()` has all fields at `Permille(0)`.
6. `ComponentKind::ALL` and `ComponentValue` coverage include the three new authoritative components.
7. `MetabolismProfile` fields using `NonZeroU32` cannot be zero.
8. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. All `Permille` fields stay within 0..=1000 range (enforced by type).
2. `MetabolismProfile` tolerance and duration fields are `NonZeroU32` — no zero-duration / zero-tolerance states.
3. Components restricted to `EntityKind::Agent`.
4. No floating-point types used.
5. The three components are first-class authoritative state and participate in generated world/component/delta inventories; they are not hidden side tables.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/needs.rs` — construction, defaults, serialization, trait bounds
2. `crates/worldwake-core/src/component_tables.rs` — table CRUD coverage for all three components
3. `crates/worldwake-core/src/world.rs` — agent-only insertion/query/remove coverage and wrong-kind rejection
4. `crates/worldwake-core/src/delta.rs` — authoritative component inventory coverage updated for all three components

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

Completion date: 2026-03-10

What actually changed:

1. Added `crates/worldwake-core/src/needs.rs` with `HomeostaticNeeds`, `DeprivationExposure`, and `MetabolismProfile`.
2. Registered all three as agent-only authoritative components in `component_schema.rs` and re-exported them from `worldwake-core`.
3. Extended the existing schema fanout so the new physiology components participate in `ComponentTables`, generated `World` APIs, and `ComponentKind` / `ComponentValue`.
4. Added focused coverage for value construction/defaults/serialization, component-table CRUD, world-level kind restrictions, and authoritative component inventory coverage.

Differences from the original plan:

1. The ticket was corrected before implementation because the original assumptions understated the integration surface of a new authoritative component in this codebase.
2. `MetabolismProfile::default_human()` was not introduced. The cleaner long-term API here is `MetabolismProfile::new(...)` plus a canonical `Default` baseline, matching the direction already established for shared Phase 2 schema.
3. No broader component-architecture refactor was needed because the repo already uses schema-driven fanout for `delta.rs`, `world.rs`, `world_txn.rs`, and `verification.rs`.

Verification results:

1. `cargo test -p worldwake-core` passed.
2. `cargo clippy --workspace --all-targets -- -D warnings` passed.
3. `cargo test --workspace` passed.
