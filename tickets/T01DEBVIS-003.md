# T01DEBVIS-003: Frame snapshot module + transit-progress

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: None

## Problem

Each frame, the visualizer rebuilds a plain-old-data `FrameSnapshot` from authoritative state to drive rendering. Spec T01 §D5 specifies the snapshot's shape (`places`, `edges`, `agents` views) and the read sources: topology accessors on `World`, scheduler accessors on `SimulationState`, and per-agent runtime from `AgentTickDriver`. The snapshot is a cache per FND-27, never authoritative — it is recomputed every frame from the same state observer.rs reads. Spec T01 §D11 also requires defensive handling of `arrival_tick == departure_tick` (transit-progress division-by-zero) and an assert for the impossible "agent in transit at tick 0" case.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. All read accessors named in spec T01 §D5 were verified by `/reassess-spec` post-apply on 2026-04-25:
   - `World::entities_with_name_and_agent_data` at `crates/worldwake-core/src/world.rs:521`.
   - `Topology::place_ids` at `crates/worldwake-core/src/topology.rs:285`, `place(id)` at line 281, `outgoing_edges(place)` at line 293, `edge(id)` at line 289.
   - `TravelEdge::from`, `to`, `travel_time_ticks` at `topology.rs:208,212,216`.
   - `World::effective_place(entity)` at `crates/worldwake-core/src/world/placement.rs:10` (inherited via `Deref` on `WorldTxn`).
   - `World::get_component_name(id)` macro-generated at `crates/worldwake-core/src/component_schema.rs:17`; high-level helper `worldwake_cli::display::entity_display_name(world, id) -> String` at `crates/worldwake-cli/src/display.rs:37`.
   - `World::possessions_of(holder)` at `crates/worldwake-core/src/world/ownership.rs:50`.
   - `World::get_component_homeostatic_needs(agent)` (`HomeostaticNeeds` at `crates/worldwake-core/src/needs.rs:9`) and `get_component_drive_thresholds(agent)` (`DriveThresholds` at `crates/worldwake-core/src/drives.rs:58`).
   - `Scheduler::active_actions() -> &BTreeMap<ActionInstanceId, ActionInstance>` at `crates/worldwake-sim/src/scheduler.rs:122`.
   - `ActionInstance.actor: EntityId` and `local_state: Option<ActionState>` at `crates/worldwake-sim/src/action_instance.rs:6-21`.
   - `ActionState::Travel { edge_id, origin, destination, departure_tick, arrival_tick }` at `crates/worldwake-sim/src/action_state.rs:21-27`.
   - `AgentTickDriver::runtime(agent) -> Option<&AgentDecisionRuntime>` at `crates/worldwake-ai/src/agent_tick/mod.rs:121`; `agenda_state: AgendaState` at `crates/worldwake-ai/src/decision_runtime.rs:181`.
2. `Permille` newtype lives at `crates/worldwake-core/src/numerics.rs:25` and supports `Permille::MAX` per spec convention. Edge-case "transit at tick 0" never happens under scenario spawn invariants — defensive assert with clear message rather than silent fallback.
3. Tooling-only ticket — the snapshot reads through the same surfaces observer.rs uses; no new abstraction boundary or simulation contract is introduced.

## Architecture Check

1. The snapshot is derived state, recomputed every frame, never written back. This matches FND-27's "derived summaries are caches, never truth" — replacing the cache with a fresh recomputation is the correctness check.
2. Read sources are exactly those spec T01 §D5 names; the snapshot does not introduce parallel transport paths or alias accessors. `txn.entities_by_kind` / `txn.edges_out` / `txn.location_of` / `txn.active_action_of` are explicitly absent — the snapshot routes through the actual public APIs.

## Verification Layers

1. Snapshot construction correctness for transit-progress arithmetic → focused unit test (`transit_progress_three_of_seven`) asserting `Permille::new(429)` and `k_of_n = (3, 7)` for `(dep=100, arr=107, cur=103)`.
2. Snapshot defensive clamp for zero-duration travel → focused unit test (`transit_progress_zero_duration_clamps_to_max`) asserting no NaN and `progress == Permille::MAX`.
3. Snapshot reads consistent state → focused unit test loading `survival-baseline.ron`, building one snapshot, asserting (a) every place ID in `places` resolves to a `topology().place(id)`, (b) every agent has either `AtPlace` or `InTransit` (no third state).
4. Per template item 6: additional decision/action/event-log layer mapping is not applicable. The snapshot is a read-only cache; verification is at the construction-correctness layer.

## What to Change

### 1. Define `FrameSnapshot` and view types

Create `crates/worldwake-visualizer/src/snapshot.rs`. POD types per T01 §D5:

- `FrameSnapshot { tick: Tick, places: BTreeMap<EntityId, PlaceView>, edges: Vec<EdgeView>, agents: BTreeMap<EntityId, AgentView> }`.
- `PlaceView { name: String, tags: Vec<PlaceTag>, position: egui::Pos2 }`.
- `EdgeView { from: EntityId, to: EntityId, travel_ticks: u32 }`.
- `AgentView { name: String, control: ControlSource, position: AgentPosition, alive: bool, active_action: Option<ActiveActionView>, active_goal: Option<ActiveGoalView>, needs: HomeostaticNeeds, drive_thresholds: DriveThresholds }`.
- `AgentPosition { AtPlace(EntityId), InTransit { from, to, progress: Permille, k_of_n: (u32, u32) } }`.
- `ActiveActionView { action_def_id: ActionDefId, ticks_in: u32, ticks_total: u32 }` (or equivalent compact representation surfaceable in the tooltip).
- `ActiveGoalView { goal_kind: GoalKind, motive_score: u32, provenance: Option<RankedGoalProvenance> }`.

### 2. Implement `build_snapshot`

`pub fn build_snapshot(world: &World, scheduler: &Scheduler, driver: &AgentTickDriver, layout: &PlaceLayout, current_tick: Tick) -> FrameSnapshot`.

Algorithm:

1. Places: iterate `world.topology().place_ids()`; for each, look up `topology().place(id)` for `name` + `tags`; pull `position` from `layout.positions[&id]`.
2. Edges: for each place, iterate `topology().outgoing_edges(place)`; resolve each `TravelEdgeId` via `topology().edge(id)`; record `(from, to, travel_time_ticks)`.
3. Agents: iterate `world.entities_with_name_and_agent_data()`. For each agent:
   - `name = entity_display_name(world, id)`.
   - `control = world.get_component_control_source(id)` (or analogous accessor) — clarify exact accessor name during implementation; the spec assumes `ControlSource` is reachable per agent.
   - `position`: query `scheduler.active_actions()` filtered by `instance.actor == id`. If a matched instance has `local_state == Some(ActionState::Travel { edge_id, origin, destination, departure_tick, arrival_tick })`, build `InTransit`; otherwise `AtPlace(world.effective_place(id).expect("agent must have a location"))`.
   - `needs`/`drive_thresholds` via component getters.
   - `active_action` summary if present.
   - `active_goal` from `driver.runtime(id).map(|r| &r.agenda_state.committed)`.
   - `alive` derived from absence of `DeadAt` or analogous component (`crates/worldwake-core/src/combat.rs:77`).

### 3. Transit-progress arithmetic

For `InTransit`:

```rust
let elapsed = current_tick.0 - departure_tick.0;
let total = arrival_tick.0 - departure_tick.0;
let progress = if total == 0 {
    Permille::MAX  // defensive clamp; instant arrival
} else {
    Permille::new(((elapsed * 1000) / total).min(1000) as u16)
};
let k_of_n = (elapsed as u32, total as u32);
```

### 4. Defensive asserts (from T01 §D11)

- Transit at `current_tick == 0` (impossible under scenario spawn invariants) → `debug_assert!` with message naming the invariant. Production code does not panic, but tests catch unexpected reachability.

### 5. Wire module into lib.rs

Add `pub mod snapshot;` to `crates/worldwake-visualizer/src/lib.rs`.

## Files to Touch

- `crates/worldwake-visualizer/src/snapshot.rs` (new)
- `crates/worldwake-visualizer/src/lib.rs` (modify — add module declaration)

## Out of Scope

- App integration / per-tick snapshot rebuild (T01DEBVIS-004 owns calling `build_snapshot` from `update()`).
- Canvas rendering (T01DEBVIS-005).
- Trace ring buffer reads (T01DEBVIS-009 — Traces tab).
- Belief-store rendering (T01DEBVIS-008 — Beliefs tab; the Beliefs tab reads `AgentBeliefStore` and sibling components directly, not via this snapshot).

## Acceptance Criteria

### Tests That Must Pass

1. `transit_progress_three_of_seven` — `dep=100, arr=107, cur=103` produces `progress = Permille::new(429)` and `k_of_n = (3, 7)`.
2. `transit_progress_zero_duration_clamps_to_max` — `dep=arr=100, cur=100` produces `progress = Permille::MAX`, no panic.
3. `snapshot_baseline_scenario_smoke` — load `survival-baseline.ron` via `worldwake_cli::scenario::spawn_scenario_ignoring_lints`, build one snapshot, assert (a) `places` non-empty, (b) every agent in `agents` has either `AtPlace` or `InTransit`.
4. Existing suite: `cargo test -p worldwake-visualizer` passes.

### Invariants

1. `FrameSnapshot` is recomputed every frame from authoritative state (`World`, `Scheduler`, `AgentTickDriver`); the snapshot is never mutated and never promoted to authoritative state (FND-27).
2. The snapshot reads through the public accessors named in §D5 and never bypasses them.
3. Transit progress is `Permille`-bounded; division-by-zero is clamped, not propagated as NaN.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-visualizer/src/snapshot.rs` (`#[cfg(test)] mod tests`) — three unit tests above; transit-progress tests use synthetic inputs without spawning a scenario; the smoke test spawns `survival-baseline.ron` from `scenarios/`.

### Commands

1. `cargo test -p worldwake-visualizer snapshot::`
2. `cargo test -p worldwake-visualizer`
3. `./scripts/verify.sh`
