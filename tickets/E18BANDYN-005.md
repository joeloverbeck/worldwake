# E18BANDYN-005: Implement bandit_camp_system() abandonment detection

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-sim (SystemId, SystemManifest, SystemDispatch), worldwake-systems (new system)
**Deps**: archive/tickets/completed/E18BANDYN-004.md, E18BANDYN-010

## Problem

When all living faction members leave or die at a bandit camp, the camp should eventually be marked abandoned. This requires a lightweight per-tick system that checks `BanditCamp` places for member presence, respects a grace period, and removes the `BanditCamp` component when the grace period expires. The system must be registered in the tick execution order between Combat and FacilityQueue.

## Assumption Reassessment (2026-03-29)

1. `SystemId` enum in `crates/worldwake-sim/src/system_manifest.rs` currently has 7 variants (Needs, Production, Trade, Combat, FacilityQueue, Politics, Perception). Adding `BanditCamp` between Combat and FacilityQueue requires inserting a new variant and updating the ordinal-based dispatch.
2. `SystemDispatchTable` in `crates/worldwake-sim/src/system_dispatch.rs` uses a fixed-size array of `SystemFn` function pointers indexed by `SystemId` ordinal. Adding a new system requires updating the array size and registration.
3. `SystemManifest` in `crates/worldwake-sim/src/system_manifest.rs` defines the execution order. The new system must be inserted after `Combat` (so combat deaths are processed first) and before `FacilityQueue` (so abandonment is visible to downstream systems).
4. System functions have the signature matching the `SystemContext` pattern used in `crates/worldwake-systems/src/`. Each system receives `&mut World`, `&mut EventLog`, `&mut DeterministicRng`, tick, and other context.
5. `members_of(faction_id)` on `RelationTables` returns member entity IDs. Checking "living members at this place" requires: (a) no `DeadAt` component, (b) `located_in` matches the camp's place.
6. The live code has no `abandonment_grace_ticks` field yet. The canonical place for that missing input should be `BanditFactionPolicy` from `E18BANDYN-010`, not a revived place-backed `BanditCampProfile`.
7. `CampAbandoned` event needs an `EventTag`. `EventTag::WorldMutation` is appropriate since it modifies the place's component state.
8. Camp supplies container remains at the place after abandonment — lootable by anyone. The container is NOT removed.
9. Faction entity is NOT archived — surviving members still reference it.
10. Adjacent contradiction exposed during reassessment: this ticket previously normalized a nonexistent `BanditCampProfile.abandonment_grace_ticks` field. That policy input should come from `E18BANDYN-010` as `BanditFactionPolicy.abandonment_grace_ticks`; this ticket should consume that canonical faction-scoped contract instead of recreating a place-level alias.

## Architecture Check

1. A dedicated per-tick system is the correct approach because: (a) abandonment detection is a world-state check, not an action with duration/cost, (b) it must run every tick to track the grace period, (c) it's analogous to how `needs_system()` checks deprivation each tick. Alternative: making abandonment an action would violate the principle that camps aren't agents with intentions — camps are places that become abandoned through emergent absence.
2. Placement after Combat and before FacilityQueue ensures: combat deaths are counted before member presence check (correctness), and abandonment events are visible to Perception in the same tick (FND-7 locality).
3. No backwards-compatibility shims. New system, new SystemId variant.

## Verification Layers

1. Camp abandoned when zero living members for grace period → authoritative world state: `BanditCamp` component removed from Place
2. Grace period respected → focused unit test: camp persists during grace period even with zero members
3. Members returning during grace resets timer → focused unit test: member arrival prevents abandonment
4. Supply container persists after abandonment → authoritative world state: Container entity still at place
5. Faction not archived → authoritative world state: FactionData still present on faction entity
6. System runs after Combat → structural: `SystemId::BanditCamp` ordinal is between Combat and FacilityQueue
7. CampAbandoned event emitted → event-log delta: event with `WorldMutation` tag

## What to Change

### 1. Add SystemId::BanditCamp variant

In `crates/worldwake-sim/src/system_manifest.rs`, insert `BanditCamp` between `Combat` (ordinal 3) and `FacilityQueue` (currently ordinal 4). This shifts FacilityQueue, Politics, and Perception ordinals up by 1.

### 2. Update SystemManifest execution order

Update the system ordering list to include `BanditCamp` in the correct position.

### 3. Update SystemDispatchTable

In `crates/worldwake-sim/src/system_dispatch.rs`, increase the array size and register the new system function at the `BanditCamp` ordinal.

### 4. Add abandonment tracking to BanditCamp

The `BanditCamp` component (from E18BANDYN-001) needs a field to track when members were last present:

```rust
pub struct BanditCamp {
    pub faction: EntityId,
    pub supplies: EntityId,
    /// Tick when a living faction member was last present at this place.
    /// Used by bandit_camp_system for grace-period abandonment.
    pub last_member_present_tick: Tick,
}
```

Note: this tracking remains active-camp state, so it still belongs on `BanditCamp`. Faction policy inputs such as grace duration belong to the faction-scoped contract from `E18BANDYN-010`, not to `BanditCamp`.

### 5. Implement bandit_camp_system()

In `crates/worldwake-systems/src/bandit_camp.rs`:

```rust
pub fn bandit_camp_system(ctx: &mut SystemContext) {
    // For each place with BanditCamp component:
    //   1. Get camp's faction from BanditCamp.faction
    //   2. Query members_of(faction) for living members located at this place
    //   3. If any living member present: update last_member_present_tick to current tick
    //   4. If zero present and (current_tick - last_member_present_tick) >= abandonment_grace_ticks:
    //      a. Remove BanditCamp component from place
    //      b. Emit CampAbandoned event with WorldMutation tag
    //      c. Do NOT remove supply container
    //      d. Do NOT archive faction entity
}
```

### 6. Register system in dispatch

Wire `bandit_camp_system` into `SystemDispatchTable` at the `BanditCamp` ordinal.

## Files to Touch

- `crates/worldwake-sim/src/system_manifest.rs` (modify — add `SystemId::BanditCamp`, update ordering)
- `crates/worldwake-sim/src/system_dispatch.rs` (modify — update array size, register new system)
- `crates/worldwake-systems/src/bandit_camp.rs` (new — system implementation)
- `crates/worldwake-systems/src/lib.rs` (modify — add `pub mod bandit_camp;`)
- `crates/worldwake-core/src/bandit_camp.rs` (modify — add `last_member_present_tick` field if not in E18BANDYN-001)
- Any files with exhaustive `match` on `SystemId` (modify — add arm)

## Out of Scope

- EstablishCamp action (E18BANDYN-004) — the system only detects abandonment, not creation
- Raid action (E18BANDYN-003)
- AI candidate generation (E18BANDYN-006)
- Route threat estimation (E18BANDYN-008)
- Golden test T22 (E18BANDYN-009)
- Supply container cleanup — containers persist indefinitely (FND-4)
- Faction entity archival — faction persists for surviving members

## Acceptance Criteria

### Tests That Must Pass

1. Camp with living members present: `BanditCamp` component remains, `last_member_present_tick` updated
2. Camp with zero living members: `BanditCamp` persists during grace period
3. Camp with zero living members past grace period: `BanditCamp` component removed
4. Members returning during grace period resets the timer
5. Supply container remains at place after abandonment
6. Faction entity not archived after camp abandonment
7. `CampAbandoned` event emitted with `WorldMutation` tag on abandonment
8. System runs after Combat and before FacilityQueue in tick order
9. Dead members (with `DeadAt`) do not count as present
10. Members in transit (`InTransitOnEdge`) do not count as present at camp
11. Existing suite: `cargo test -p worldwake-systems`
12. Existing suite: `cargo test -p worldwake-sim`
13. Existing suite: `cargo clippy --workspace`

### Invariants

1. `BanditCamp` removal only through this system (not through direct component deletion elsewhere)
2. Grace period is policy-driven from `BanditFactionPolicy` introduced by `E18BANDYN-010`, not hardcoded
3. Conservation: system does not create or destroy any entities or items
4. System ordering: Combat < BanditCamp < FacilityQueue (load-bearing)
5. No global queries — system iterates only Place entities with `BanditCamp` component

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/bandit_camp.rs` — focused tests for abandonment detection, grace period, member presence tracking
2. `crates/worldwake-sim/src/system_manifest.rs` — verify system ordering includes BanditCamp at correct position

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace`
4. `cargo build --workspace`
