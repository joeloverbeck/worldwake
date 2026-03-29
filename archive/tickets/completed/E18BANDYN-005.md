# E18BANDYN-005: Implement bandit_camp_system() abandonment detection

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-sim (SystemId, SystemManifest, SystemDispatch), worldwake-systems (new system)
**Deps**: archive/tickets/completed/E18BANDYN-004.md, E18BANDYN-010

## Problem

When all living faction members leave or die at a bandit camp, the camp should eventually be marked abandoned. This requires a lightweight per-tick system that checks `BanditCamp` places for member presence, respects a grace period, and removes the `BanditCamp` component when the grace period expires. The system must be registered in the tick execution order between Combat and FacilityQueue.

## Assumption Reassessment (2026-03-29)

Shared abstraction boundary under audit: active camp state on `Place` (`worldwake_core::BanditCamp`), faction-scoped abandonment policy on `Faction` (`worldwake_core::BanditFactionPolicy`), and authoritative per-tick scheduling through `worldwake_sim::SystemId` / `SystemManifest` / `SystemDispatchTable`.

1. `BanditCamp` in [`crates/worldwake-core/src/bandit_camp.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs) already stores both `faction: EntityId` and `supplies: EntityId`. The ticket's earlier assumption that camp-faction identity was still missing is stale. The only additional camp state this ticket needs is abandonment timing.
2. `BanditFactionPolicy` already exists in [`crates/worldwake-core/src/bandit_camp.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/bandit_camp.rs), and it already carries `abandonment_grace_ticks`. This ticket must consume that faction-scoped policy directly; reviving a place-backed alias would regress the architecture corrected by archived [`E18BANDYN-010`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/E18BANDYN-010.md).
3. `SystemId` in [`crates/worldwake-sim/src/system_manifest.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/system_manifest.rs) currently has 7 closed variants (`Needs`, `Production`, `Trade`, `Combat`, `FacilityQueue`, `Politics`, `Perception`). Adding `BanditCamp` requires extending the closed enum, updating display/tests, and updating the canonical manifest order. The cleaner live fix preserves existing ordinals for preexisting systems instead of renumbering `SystemId::ALL`.
4. `SystemDispatchTable` in [`crates/worldwake-sim/src/system_dispatch.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/system_dispatch.rs) is a fixed-size handler array indexed by `SystemId::ordinal()`. Adding a new system requires updating the canonical worldwake-systems dispatch table, but not changing the dispatch-table architecture itself.
5. Live system functions use `worldwake_sim::SystemExecutionContext`, not a `SystemContext` type. The new system should follow the existing pattern used in [`crates/worldwake-systems/src/needs.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs) and [`crates/worldwake-systems/src/production.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production.rs): inspect authoritative world state, stage mutations through `WorldTxn`, and emit normal tagged system events.
6. `members_of(faction)` already exists on `World` / `WorldTxn`. For this ticket, "living faction member present at camp" should mean: the member is alive and `effective_place(member) == Some(camp_place)`. That deliberately excludes dead members and transit cases without adding a parallel location test path.
7. `EstablishCamp` already enforces one active `BanditCamp` per faction and already creates/reuses the supply container in [`crates/worldwake-systems/src/bandit_camp_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/bandit_camp_actions.rs). Abandonment should therefore remove only the active-camp marker. It must not delete the supply container or archive the faction entity.
8. There is no dedicated `CampAbandoned` event type or payload in the live event model. The lawful event surface here is a normal system `WorldTxn` commit with `EventTag::System` and `EventTag::WorldMutation`, plus a `ComponentDelta::Removed` for `BanditCamp` on the place entity.
9. The current focused test surface does not yet cover abandonment. Verified from `cargo test -p worldwake-systems -- --list` and `cargo test -p worldwake-sim -- --list`: there are existing manifest/dispatch tests, existing `bandit_camp_actions` tests, and no live `bandit_camp_system` tests yet.
10. Adjacent contradiction exposed during reassessment: the active E18 spec still claims the `BanditCamp` component is "minimal stored state," but the live architecture now already treats camp identity as active place state and faction policy as faction state. The ticket's older `last_member_present_tick` draft would also force a component write every occupied tick. The cleaner active-camp state is `empty_since_tick: Option<Tick>`: `None` while occupied, set once when the camp first becomes empty, cleared once on reoccupation, and consumed on abandonment. This is a required correction to the proposed implementation, not a separate cleanup ticket.

## Architecture Check

1. A dedicated per-tick system is still the correct architecture. Camp abandonment is an authoritative world-state consequence of absence, not an intention or action owned by an agent. Modeling it as an action would invent agency for places and would weaken the causal story.
2. Storing `empty_since_tick: Option<Tick>` on `BanditCamp` is more beneficial than both the current architecture and the ticket's older `last_member_present_tick` draft. The abandonment timer is active camp state, not faction doctrine, but it should only mutate on occupancy transitions. `empty_since_tick` keeps one fact on one carrier without introducing per-tick write churn.
3. Placement after `Combat` and before `FacilityQueue` remains load-bearing. Fatal combat outcomes should affect abandonment immediately, and downstream same-tick systems should see whether the camp marker still exists.
4. No backwards-compatibility shims or parallel alias paths. Update the closed system set directly, add the timing field directly to `BanditCamp`, and fix the affected tests/callers.

## Verification Layers

1. Camp abandonment removes only the active-camp marker after the grace period -> authoritative world state plus event-log delta proving `ComponentDelta::Removed { component_kind: BanditCamp }`
2. Grace period is policy-driven and resets on renewed lawful occupancy -> focused systems unit coverage on `bandit_camp_system`
3. Dead members and members not effectively at the camp do not keep it active -> focused systems unit coverage on authoritative presence filtering
4. Supply container and faction entity survive abandonment -> authoritative world-state assertions in focused systems coverage
5. System scheduling remains `Combat < BanditCamp < FacilityQueue` -> `SystemManifest` and dispatch/manifest-focused unit coverage
6. This is an authoritative state/scheduling ticket, not a planner or golden ticket; decision traces are not the primary proof surface here

## What to Change

### 1. Add SystemId::BanditCamp variant

In `crates/worldwake-sim/src/system_manifest.rs`, add `BanditCamp` to the closed system set while preserving existing ordinals for preexisting systems. The canonical manifest order must still run `BanditCamp` between `Combat` and `FacilityQueue`, but later systems should not be renumbered just to add a new phase.

### 2. Update SystemManifest execution order

Update the system ordering list to include `BanditCamp` in the correct position.

### 3. Update SystemDispatchTable

In `crates/worldwake-sim/src/system_dispatch.rs`, increase the array size and register the new system function at the `BanditCamp` ordinal.

### 4. Add abandonment tracking to BanditCamp

The `BanditCamp` component (from E18BANDYN-001) needs a field to track when the camp first became empty:

```rust
pub struct BanditCamp {
    pub faction: EntityId,
    pub supplies: EntityId,
    /// Tick when the camp first had zero living faction members present.
    /// `None` means the camp is currently occupied.
    pub empty_since_tick: Option<Tick>,
}
```

Note: this timing remains active-camp state, so it belongs on `BanditCamp`. Faction policy inputs such as grace duration remain on `BanditFactionPolicy` and must not be duplicated.

### 5. Implement bandit_camp_system()

In `crates/worldwake-systems/src/bandit_camp.rs`:

```rust
pub fn bandit_camp_system(ctx: SystemExecutionContext<'_>) {
    // For each place with BanditCamp component:
    //   1. Get camp's faction from BanditCamp.faction
    //   2. Query members_of(faction) for living members located at this place
    //   3. If any living member present: clear empty_since_tick back to None
    //   4. If zero present and empty_since_tick is None: set it to current tick
    //   5. If zero present and empty_since_tick is Some(t0)
    //      and (current_tick - t0) >= abandonment_grace_ticks:
    //      a. Remove BanditCamp component from place
    //      b. Emit the normal system world-mutation event through WorldTxn
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
- `crates/worldwake-core/src/bandit_camp.rs` (modify — add `empty_since_tick` field)
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

1. Camp with living members present: `BanditCamp` component remains and `empty_since_tick` stays `None`
2. Camp with zero living members: `BanditCamp` persists during grace period
3. Camp with zero living members past grace period: `BanditCamp` component removed
4. Members returning during grace period clear `empty_since_tick` and reset abandonment timing
5. Supply container remains at place after abandonment
6. Faction entity not archived after camp abandonment
7. Abandonment emits the normal system mutation event with a `BanditCamp` removal delta and `WorldMutation` tag
8. System runs after Combat and before FacilityQueue in tick order
9. Dead members (with `DeadAt`) do not count as present
10. Members not effectively at the camp place, including transit cases, do not count as present
11. Existing suite: `cargo test -p worldwake-systems`
12. Existing suite: `cargo test -p worldwake-sim`
13. Existing suite: `cargo clippy --workspace`

### Invariants

1. `BanditCamp` removal only through this system (not through direct component deletion elsewhere)
2. Grace period is policy-driven from `BanditFactionPolicy` introduced by `E18BANDYN-010`, not hardcoded
3. Conservation: system does not create or destroy any entities or items
4. System ordering: Combat < BanditCamp < FacilityQueue (load-bearing)
5. No duplicate policy path — grace-period duration comes only from `BanditFactionPolicy` on the faction entity
6. System iterates only place entities with `BanditCamp` and inspects faction membership through existing authoritative queries

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/bandit_camp.rs` — extend component roundtrip/contract coverage for the new `empty_since_tick` field
Rationale: proves the active-camp timing state is part of the canonical component contract rather than an ad-hoc sidecar.

2. `crates/worldwake-systems/src/bandit_camp.rs` — focused tests for grace-period abandonment, timer reset on return, dead/transit exclusion, supply/faction persistence, and emitted mutation-event shape
Rationale: proves the authoritative abandonment contract directly at the system layer that owns it.

3. `crates/worldwake-sim/src/system_manifest.rs` — verify canonical order includes `BanditCamp` between `Combat` and `FacilityQueue`
Rationale: proves the closed scheduler contract changed at the correct ordering layer.

4. `crates/worldwake-systems/src/lib.rs` or `crates/worldwake-systems/src/bandit_camp.rs` — dispatch-table coverage for the new `SystemId::BanditCamp` slot
Rationale: proves the live worldwake-systems canonical dispatch table actually wires the real system into the new manifest slot.

### Commands

1. `cargo test -p worldwake-core bandit_camp`
2. `cargo test -p worldwake-sim system_manifest`
3. `cargo test -p worldwake-systems bandit_camp`
4. `cargo test -p worldwake-systems`
5. `cargo test -p worldwake-sim`
6. `cargo clippy --workspace`
7. `cargo build --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed:
  - Added a new `BanditCamp` system phase and wired it into the canonical manifest between `Combat` and `FacilityQueue`
  - Implemented `bandit_camp_system()` to arm abandonment when a camp first becomes empty, clear that timer on reoccupation, and remove `BanditCamp` after the faction policy grace period expires
  - Added `BanditCamp.empty_since_tick: Option<Tick>` as the active-camp abandonment timer
  - Preserved the existing ordinals for preexisting systems; only the manifest execution order changed
  - Added focused coverage for abandonment timing, dead/transit exclusion, persistence of supplies/faction, dispatch wiring, and manifest ordering
- Deviations from original plan:
  - Replaced the ticket's older `last_member_present_tick` proposal with `empty_since_tick` to avoid per-tick mutation churn while camps remain occupied
  - Preserved old `SystemId` ordinals instead of shifting later systems, because renumbering broke unrelated deterministic AI goldens
  - Used the existing tagged `WorldTxn` event contract rather than introducing a dedicated `CampAbandoned` event type
- Verification results:
  - `cargo test -p worldwake-core bandit_camp`
  - `cargo test -p worldwake-sim system_manifest`
  - `cargo test -p worldwake-systems bandit_camp`
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-systems`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
  - `cargo build --workspace`
