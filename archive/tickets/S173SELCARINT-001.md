# S173SELCARINT-001: `SelfCareOccupancy` component + `SelfCareUseKind` enum + ECS registration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new core-resident ECS component on `EntityKind::Facility | EntityKind::Place`; SAVE_FORMAT_VERSION bump
**Deps**: `archive/specs/S173-self-care-interruption-occupancy.md` (D1)

## Problem

Before this ticket, self-care actions (`wash`, `toilet`) had no facility reservation: two dirty agents at the same `WashBasin`-tagged `Facility` could not lawfully contend for it because neither one ever reserved it (`crates/worldwake-systems/src/needs_actions.rs` registered `wash` with `reservation_requirements: Vec::new()`; same shape for `toilet`). The contention substrate `S44` existed, but without a concrete occupancy carrier on the facility/place entity there was nothing to release on abort and nothing to gate concurrent attempts. This ticket introduced the authoritative state carrier `SelfCareOccupancy` and its discriminator `SelfCareUseKind` so downstream tickets can wire start-gate reservation, commit/abort lifecycle, and trace-detail attribution against a real component, not a forward-declared symbol.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `WashBasin` is a `WorkstationTag` variant (`crates/worldwake-core/src/production.rs:15`), NOT an `EntityKind` variant. `WashBasinState` (`crates/worldwake-core/src/place_dirtiness.rs:44-52`) is registered on `EntityKind::Facility` (`component_schema.rs:2034`, filter `|kind| kind == EntityKind::Facility`). Latrine-tagged places carry `PlaceTag::Latrine` (`crates/worldwake-core/src/topology.rs:18`). Therefore `SelfCareOccupancy` registers on the union filter `|kind| kind == EntityKind::Facility || kind == EntityKind::Place`; the Wash handler writes only on Facility carrying `WorkstationTag::WashBasin`, the Toilet handler only on Place carrying `PlaceTag::Latrine`.
2. `GoalKey` exists in core (`crates/worldwake-core/src/goal.rs`); used in the `goal_key: GoalKey` field on `SelfCareOccupancy`. `EntityId` and `Tick` are core (`crates/worldwake-core/src/ids.rs:44, 57`). All field types resolve to core — no Core-Side Mirror Enum pattern required.
3. Shared abstraction boundary: ECS component registration via the `with_component_schema_entries!` macro (`crates/worldwake-core/src/component_schema.rs:3-31`). Macro expansion sites that need `SelfCareOccupancy` in scope: `crates/worldwake-core/src/delta.rs`, `crates/worldwake-core/src/world.rs`, and `crates/worldwake-core/src/component_tables.rs`. Live implementation confirmed these sites require explicit import fallout; the ticket absorbed that schema-expansion work alongside the component registration.
4. Precedent for this lifecycle (runtime-managed, absent by default, written at action start, removed on commit/abort/abandon): `SleepEpisode` registration at `component_schema.rs:2169-2191`. Use the same accessor naming pattern (`insert_self_care_occupancy`, `get_self_care_occupancy`, `iter_self_care_occupancies`, `entities_with_self_care_occupancy`, etc.).
5. `SelfCareUseKind` initially carried 5 variants per spec D2 note option (i): `Wash`, `LatrineRelief`, `Eat`, `Drink`, `WildernessRelief`. The two occupancy-bearing variants (`Wash`, `LatrineRelief`) are used by `SelfCareOccupancy.use_kind`; the non-occupancy variants are used only by `ActionTraceDetail::SelfCareInterrupted` (ticket 002) and the later action-trace mapping. `archive/tickets/S173SELCARINT-005.md` added `Sleep` as the sixth trace-only discriminator.
6. Before this ticket, `SAVE_FORMAT_VERSION` was `106` (`crates/worldwake-sim/src/save_load.rs`). Adding a new ECS component to the authoritative world state was a save-format-breaking change; this ticket bumped it to `107` and updated the version assertions.
7. No new agent-side component. No `AgentDef`/`PlaceDef`/`FacilityDef` field is added — `SelfCareOccupancy` is runtime-managed and scenario-exempt per the `docs/spec-drafting-rules.md §Agent Profile Scenario Contract` exemption clause (mirrors `SleepEpisode`'s scenario exemption).

## Architecture Check

1. The component lives in `worldwake-core` because the `with_component_schema_entries!` macro at `crates/worldwake-core/src/component_schema.rs:3` references types via `crate::TypeName`. Components defined in higher crates cannot be registered through this macro. The core-residence constraint is satisfied because all four field types (`EntityId`, `SelfCareUseKind`, `Tick`, `GoalKey`) are core types.
2. Component lifecycle is runtime-managed (absent by default, written at action start, removed on commit/abort/abandon) mirroring `SleepEpisode`. This avoids scenario-authoring overhead and matches the spec's intent that occupancy is a real-time mechanic, not a scenario-configurable knob.
3. No backwards-compatibility aliasing: this is the first carrier of self-care facility occupancy. No prior shim exists; this ticket introduces the canonical surface.

## Verified Layers

1. Component registration → passed focused unit/runtime coverage in `worldwake-core`'s component-schema test pattern: `SelfCareOccupancy` is registerable on a `Facility` entity and a `Place` entity, rejects `Agent`, and exposes generated accessors.
2. Save format compatibility → passed focused unit coverage in `save_load.rs`: a full `SimulationState` containing both place and facility `SelfCareOccupancy` instances round-trips under `SAVE_FORMAT_VERSION = 107`.
3. Single-layer ticket (component definition + registration only): higher-layer invariants (start-gate reservation, abort lifecycle, candidate-emitter filtering) remain owned by tickets 004 and 006. This ticket's landed contract is solely that the component exists, registers, appears in schema inventories, and round-trips.

## Landed Changes

### 1. Defined `SelfCareOccupancy` and `SelfCareUseKind` in core

Create `crates/worldwake-core/src/self_care_occupancy.rs` with:

```rust
use crate::{Component, EntityId, goal::GoalKey, ids::Tick};
use serde::{Deserialize, Serialize};

/// Authoritative world state. Attached to the facility entity (a `WashBasin`-tagged
/// `Facility`, or a `Latrine`-tagged `Place`) while a self-care action is mid-flight.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelfCareOccupancy {
    pub occupant: EntityId,
    pub use_kind: SelfCareUseKind,
    pub started_tick: Tick,
    /// The `GoalKey` that the occupant was pursuing when occupancy was written.
    /// Records why the occupancy started; consumed by decision-trace queries.
    pub goal_key: GoalKey,
}

impl Component for SelfCareOccupancy {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelfCareUseKind {
    /// Wash at a `WashBasin`-tagged `Facility` (occupancy-bearing).
    Wash,
    /// Toilet at a `Latrine`-tagged `Place` (occupancy-bearing).
    LatrineRelief,
    /// Eat — trace-detail discriminator only; no occupancy is written for `eat`.
    Eat,
    /// Drink — trace-detail discriminator only; no occupancy is written for `drink`.
    Drink,
    /// Wilderness relief — trace-detail discriminator only; location-flexible.
    WildernessRelief,
}
```

### 2. Registered the component via `with_component_schema_entries!`

In `crates/worldwake-core/src/component_schema.rs`, add a registration block following the `SleepEpisode` pattern (line 2169-2191) but with the dual-kind filter:

```rust
{
    self_care_occupancies,
    SelfCareOccupancy,
    insert_self_care_occupancy,
    get_self_care_occupancy,
    get_self_care_occupancy_mut,
    remove_self_care_occupancy,
    has_self_care_occupancy,
    iter_self_care_occupancies,
    insert_component_self_care_occupancy,
    get_component_self_care_occupancy,
    get_component_self_care_occupancy_mut,
    remove_component_self_care_occupancy,
    has_component_self_care_occupancy,
    entities_with_self_care_occupancy,
    query_self_care_occupancy,
    count_with_self_care_occupancy,
    "SelfCareOccupancy",
    |kind| kind == EntityKind::Facility || kind == EntityKind::Place,
    SelfCareOccupancy,
    crate::SelfCareOccupancy,
    set_component_self_care_occupancy,
    clear_component_self_care_occupancy,
    txn_simple_set
}
```

### 3. Re-exported from `crates/worldwake-core/src/lib.rs`

Add to the `pub use` block (alongside the existing `SleepEpisode` re-export at line 316):

```rust
pub use self_care_occupancy::{SelfCareOccupancy, SelfCareUseKind};
```

And register the module: `mod self_care_occupancy;` at the appropriate location in `lib.rs`.

### 4. Bumped `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs`, `SAVE_FORMAT_VERSION` now equals `107`; the version assertions and the full non-default save fixture were updated with that new persisted state shape.

## Landed Files

- `crates/worldwake-core/src/self_care_occupancy.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modified — `mod` declaration and `pub use`)
- `crates/worldwake-core/src/component_schema.rs` (modified — registration block and focused registration test)
- `crates/worldwake-core/src/component_tables.rs` (modified — macro import fallout)
- `crates/worldwake-core/src/world.rs` (modified — macro import fallout)
- `crates/worldwake-core/src/delta.rs` (modified — macro import fallout plus `ComponentKind`/`ComponentValue` inventory coverage)
- `crates/worldwake-sim/src/save_load.rs` (modified — bump version constant, assertions, and non-default save fixture)

Implementation corrected the draft assumption about macro expansion sites: `delta.rs`, `world.rs`, and `component_tables.rs` required explicit `SelfCareOccupancy` imports, and `delta.rs` also required hand-maintained inventory/sample coverage.

## Out of Scope

- Writing `SelfCareOccupancy` from any action handler — tickets 004 (wash, toilet start/commit/abort) and 005 (atomic-action abort trace detail) own that.
- `ActionTraceDetail::SelfCareInterrupted` variant — owned by ticket 002.
- `PromotableContentionKind` extension — owned by the now-archived `archive/tickets/S173SELCARINT-003.md` (independent of this ticket).
- Belief-view accessor for `SelfCareOccupancy` — per spec D5 "no new accessor on `GoalBeliefView` is required" claim; ticket 006 verifies whether the consumer can compose without one, or whether a thin `facility_self_care_occupancy_observed` accessor is needed.
- Scenario authoring — `SelfCareOccupancy` is runtime-managed, no `*Def` wrapper.

## Acceptance Result

### Tests Passed Or Deferred

1. Passed via `component_schema::tests::self_care_occupancy_is_registered_for_facilities_and_places_only` — writes `SelfCareOccupancy` to a `Facility` and a `Place`, validates generated accessors, and rejects `Agent`.
2. Passed via `save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state` — round-trips a full saved `SimulationState` with both a `LatrineRelief` place occupancy and a `Wash` facility occupancy.
3. Passed `cargo test -p worldwake-core` and `cargo test -p worldwake-sim --lib save_load`.
4. Passed `cargo build --workspace`.

### Invariants

1. `SelfCareOccupancy` registers on `EntityKind::Facility | EntityKind::Place` only — attempts to register on `Agent`, `ItemLot`, or other kinds via the macro fail or are filtered out at insert time.
2. `SAVE_FORMAT_VERSION` is `107` and a save bytestream emitted at `107` round-trips losslessly when the world contains zero, one, or many `SelfCareOccupancy` instances.
3. `SelfCareUseKind` derives `Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize` — required by use in `SelfCareOccupancy` (which derives `Eq` and is `Serialize`-able) and by ticket 002's `ActionTraceDetail::SelfCareInterrupted` variant payload.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/self_care_occupancy.rs` (inline `#[cfg(test)] mod tests`) — basic construction + accessor sanity.
2. `crates/worldwake-sim/src/save_load.rs` (extend existing save-load round-trip tests) — `SAVE_FORMAT_VERSION == 107` assertion update + new round-trip case for `SelfCareOccupancy`-bearing world.
3. Existing component-schema lint / registration tests in `worldwake-core` — confirm the new entry parses correctly through `with_component_schema_entries!`.

### Commands Run

1. Passed `cargo test -p worldwake-core self_care_occupancy`
2. Passed `cargo test -p worldwake-sim --lib save_format_version_is_107_after_self_care_occupancy`
3. Passed `cargo test -p worldwake-sim --lib save_to_bytes_roundtrip_preserves_full_nondefault_state`
4. Passed `cargo test -p worldwake-sim --lib save_load`
5. Passed `cargo test -p worldwake-core`
6. Passed `cargo build --workspace`
7. Waived `./scripts/verify.sh` for this ticket closeout because the `implement-spec-tickets` harness owns the full pre-push verification gate after the S173 family lands.

Merge note: Ticket 001 bumps SAVE_FORMAT_VERSION 106→107. Sibling tickets 002 (trace-detail variant), 003 (PromotableContentionKind variants — crate-private, not in save state), 005 (abort handlers, no new world-state shape), 006 (candidate emitter, no save-state changes) deliberately avoid a second bump because they touch no serialized `SimulationState` surface. Ticket 004 (wash/toilet handlers) writes `SelfCareOccupancy` instances but rides the bump in 001.

## Outcome

Completed on 2026-05-26.

- Added the core-resident `SelfCareOccupancy` component and `SelfCareUseKind` enum.
- Registered `SelfCareOccupancy` on `EntityKind::Facility | EntityKind::Place` with generated world, transaction, delta, and component-table surfaces.
- Bumped `SAVE_FORMAT_VERSION` from `106` to `107` and extended the full non-default save fixture so saved worlds round-trip both occupancy-bearing self-care variants.
- Updated the hand-maintained `ComponentKind`/`ComponentValue` inventory in `delta.rs` so the new authoritative component is part of schema parity coverage.

## Deviations

- The draft said `delta.rs`, `world.rs`, and `component_tables.rs` would pick up the component automatically after the crate-root re-export. Live compilation showed those macro expansion sites require explicit imports; this ticket absorbed that fallout because it is part of the component registration contract.
- The focused core selector used the shared `self_care_occupancy` substring and ran the new component tests plus zero matching integration tests. The full `worldwake-core` suite then proved the entire core component/schema surface.

## Verification Result

- Passed `cargo test -p worldwake-core self_care_occupancy`
- Passed `cargo test -p worldwake-sim --lib save_format_version_is_107_after_self_care_occupancy`
- Passed `cargo test -p worldwake-sim --lib save_to_bytes_roundtrip_preserves_full_nondefault_state`
- Passed `cargo test -p worldwake-sim --lib save_load`
- Passed `cargo test -p worldwake-core`
- Passed `cargo build --workspace`
- Waived `./scripts/verify.sh` for this ticket iteration; the S173 harness finalization still owns the full pre-push gate.
