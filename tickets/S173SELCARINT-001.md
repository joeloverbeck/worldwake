# S173SELCARINT-001: `SelfCareOccupancy` component + `SelfCareUseKind` enum + ECS registration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new core-resident ECS component on `EntityKind::Facility | EntityKind::Place`; SAVE_FORMAT_VERSION bump
**Deps**: `specs/S173-self-care-interruption-occupancy.md` (D1)

## Problem

Self-care actions (`wash`, `toilet`) have no facility reservation today: two dirty agents at the same `WashBasin`-tagged `Facility` cannot lawfully contend for it because neither one ever reserves it (`crates/worldwake-systems/src/needs_actions.rs:48-52` registers `wash` with `reservation_requirements: Vec::new()`; same shape for `toilet`). The contention substrate `S44` exists, but without a concrete occupancy carrier on the facility/place entity there is nothing to release on abort and nothing to gate concurrent attempts. This ticket introduces the authoritative state carrier `SelfCareOccupancy` and its discriminator `SelfCareUseKind` so downstream tickets can wire start-gate reservation, commit/abort lifecycle, and trace-detail attribution against a real component, not a forward-declared symbol.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `WashBasin` is a `WorkstationTag` variant (`crates/worldwake-core/src/production.rs:15`), NOT an `EntityKind` variant. `WashBasinState` (`crates/worldwake-core/src/place_dirtiness.rs:44-52`) is registered on `EntityKind::Facility` (`component_schema.rs:2034`, filter `|kind| kind == EntityKind::Facility`). Latrine-tagged places carry `PlaceTag::Latrine` (`crates/worldwake-core/src/topology.rs:18`). Therefore `SelfCareOccupancy` registers on the union filter `|kind| kind == EntityKind::Facility || kind == EntityKind::Place`; the Wash handler writes only on Facility carrying `WorkstationTag::WashBasin`, the Toilet handler only on Place carrying `PlaceTag::Latrine`.
2. `GoalKey` exists in core (`crates/worldwake-core/src/goal.rs`); used in the `goal_key: GoalKey` field on `SelfCareOccupancy`. `EntityId` and `Tick` are core (`crates/worldwake-core/src/ids.rs:44, 57`). All field types resolve to core — no Core-Side Mirror Enum pattern required.
3. Shared abstraction boundary: ECS component registration via the `with_component_schema_entries!` macro (`crates/worldwake-core/src/component_schema.rs:3-31`). Macro expansion sites that need `SelfCareOccupancy` in scope: `crates/worldwake-core/src/delta.rs:29`, `crates/worldwake-core/src/world.rs:29`, `crates/worldwake-core/src/component_tables.rs:12` (each imports `WoundList` + others via `crate::` paths or `pub use` re-exports — `SelfCareOccupancy` follows the same pattern as `SleepEpisode` at `lib.rs:316`).
4. Precedent for this lifecycle (runtime-managed, absent by default, written at action start, removed on commit/abort/abandon): `SleepEpisode` registration at `component_schema.rs:2169-2191`. Use the same accessor naming pattern (`insert_self_care_occupancy`, `get_self_care_occupancy`, `iter_self_care_occupancies`, `entities_with_self_care_occupancy`, etc.).
5. `SelfCareUseKind` carries 5 variants per spec D2 note option (i): `Wash`, `LatrineRelief`, `Eat`, `Drink`, `WildernessRelief`. The two occupancy-bearing variants (`Wash`, `LatrineRelief`) are used by `SelfCareOccupancy.use_kind`; the three non-occupancy variants are used only by `ActionTraceDetail::SelfCareInterrupted` (ticket 002) and the atomic-action abort handlers (ticket 005). Defining all 5 in core here simplifies downstream tickets and avoids a future enum-extension ticket.
6. SAVE_FORMAT_VERSION is currently `106` (`crates/worldwake-sim/src/save_load.rs:7`). Adding a new ECS component to the authoritative world state is a save-format-breaking change; bump to `107`. The two `assert_eq!(SAVE_FORMAT_VERSION, 106)` sites in `save_load.rs:1372,1383` must be updated to `107`.
7. No new agent-side component. No `AgentDef`/`PlaceDef`/`FacilityDef` field is added — `SelfCareOccupancy` is runtime-managed and scenario-exempt per the `docs/spec-drafting-rules.md §Agent Profile Scenario Contract` exemption clause (mirrors `SleepEpisode`'s scenario exemption).

## Architecture Check

1. The component lives in `worldwake-core` because the `with_component_schema_entries!` macro at `crates/worldwake-core/src/component_schema.rs:3` references types via `crate::TypeName`. Components defined in higher crates cannot be registered through this macro. The core-residence constraint is satisfied because all four field types (`EntityId`, `SelfCareUseKind`, `Tick`, `GoalKey`) are core types.
2. Component lifecycle is runtime-managed (absent by default, written at action start, removed on commit/abort/abandon) mirroring `SleepEpisode`. This avoids scenario-authoring overhead and matches the spec's intent that occupancy is a real-time mechanic, not a scenario-configurable knob.
3. No backwards-compatibility aliasing: this is the first carrier of self-care facility occupancy. No prior shim exists; this ticket introduces the canonical surface.

## Verification Layers

1. Component registration → focused unit/runtime test (`worldwake-core`'s component-schema test pattern): asserts `SelfCareOccupancy` is registerable on a `Facility` entity and a `Place` entity, and that accessors `insert_*`, `get_*`, `remove_*`, `has_*`, `entities_with_*`, `iter_*` are generated.
2. Save format compatibility → focused unit test in `save_load.rs`: round-trip serialize/deserialize a `World` containing a `SelfCareOccupancy` instance; assert the saved bytes deserialize cleanly under `SAVE_FORMAT_VERSION = 107`.
3. Single-layer ticket (component definition + registration only): higher-layer invariants (start-gate reservation, abort lifecycle, candidate-emitter filtering) are proven by tickets 004 and 006. This ticket's contract is solely that the component exists, registers, and round-trips.

## What to Change

### 1. Define `SelfCareOccupancy` and `SelfCareUseKind` in core

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

### 2. Register the component via `with_component_schema_entries!`

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

### 3. Re-export from `crates/worldwake-core/src/lib.rs`

Add to the `pub use` block (alongside the existing `SleepEpisode` re-export at line 316):

```rust
pub use self_care_occupancy::{SelfCareOccupancy, SelfCareUseKind};
```

And register the module: `mod self_care_occupancy;` at the appropriate location in `lib.rs`.

### 4. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs:7`, change `pub const SAVE_FORMAT_VERSION: u32 = 106;` to `107`. Update the two `assert_eq!(SAVE_FORMAT_VERSION, 106)` sites at lines 1372 and 1383 to `107`.

## Files to Touch

- `crates/worldwake-core/src/self_care_occupancy.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — `mod` declaration and `pub use`)
- `crates/worldwake-core/src/component_schema.rs` (modify — registration block)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump version constant and assertions)

The `with_component_schema_entries!` macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`) already import via `crate::TypeName` paths and pick up new components automatically once the `pub use` in `lib.rs` lands — no per-site import additions needed. Per `tickets/README.md` check 13, verify this assumption at implementation time.

## Out of Scope

- Writing `SelfCareOccupancy` from any action handler — tickets 004 (wash, toilet start/commit/abort) and 005 (atomic-action abort trace detail) own that.
- `ActionTraceDetail::SelfCareInterrupted` variant — owned by ticket 002.
- `PromotableContentionKind` extension — owned by ticket 003 (independent of this ticket).
- Belief-view accessor for `SelfCareOccupancy` — per spec D5 "no new accessor on `GoalBeliefView` is required" claim; ticket 006 verifies whether the consumer can compose without one, or whether a thin `facility_self_care_occupancy_observed` accessor is needed.
- Scenario authoring — `SelfCareOccupancy` is runtime-managed, no `*Def` wrapper.

## Acceptance Criteria

### Tests That Must Pass

1. New unit test: `self_care_occupancy_round_trips_on_facility_entity` — write a `SelfCareOccupancy` to a Facility, serialize world, deserialize, assert presence and equality.
2. New unit test: `self_care_occupancy_round_trips_on_place_entity` — same but on a Place.
3. Existing suite: `cargo test -p worldwake-core` and `cargo test -p worldwake-sim --test save_load` (or equivalent — verified at implementation time per `cargo test -p worldwake-sim -- --list`).
4. Workspace builds: `cargo build --workspace`.

### Invariants

1. `SelfCareOccupancy` registers on `EntityKind::Facility | EntityKind::Place` only — attempts to register on `Agent`, `ItemLot`, or other kinds via the macro fail or are filtered out at insert time.
2. `SAVE_FORMAT_VERSION` is `107` and a save bytestream emitted at `107` round-trips losslessly when the world contains zero, one, or many `SelfCareOccupancy` instances.
3. `SelfCareUseKind` derives `Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize` — required by use in `SelfCareOccupancy` (which derives `Eq` and is `Serialize`-able) and by ticket 002's `ActionTraceDetail::SelfCareInterrupted` variant payload.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/self_care_occupancy.rs` (inline `#[cfg(test)] mod tests`) — basic construction + accessor sanity.
2. `crates/worldwake-sim/src/save_load.rs` (extend existing save-load round-trip tests) — `SAVE_FORMAT_VERSION == 107` assertion update + new round-trip case for `SelfCareOccupancy`-bearing world.
3. Existing component-schema lint / registration tests in `worldwake-core` — confirm the new entry parses correctly through `with_component_schema_entries!`.

### Commands

1. `cargo test -p worldwake-core self_care_occupancy`
2. `cargo test -p worldwake-sim --lib save_load` (verify exact test target name at implementation time)
3. `cargo build --workspace`
4. `./scripts/verify.sh` before commit.

Merge note: Ticket 001 bumps SAVE_FORMAT_VERSION 106→107. Sibling tickets 002 (trace-detail variant), 003 (PromotableContentionKind variants — crate-private, not in save state), 005 (abort handlers, no new world-state shape), 006 (candidate emitter, no save-state changes) deliberately avoid a second bump because they touch no serialized `SimulationState` surface. Ticket 004 (wash/toilet handlers) writes `SelfCareOccupancy` instances but rides the bump in 001.
