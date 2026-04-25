# T01: Debug Visualizer

## Summary

A new workspace crate `worldwake-visualizer` that runs any `.ron` scenario live
as an interactive desktop application. It auto-lays out places as a
force-directed graph, draws agents on places or lerped along travel edges at
fractional tick-progress, and exposes per-agent state via hover tooltips and
click-to-open tabbed modals. The visualizer is an **observer** (FND-29
Debuggability; the tooling-boundary pattern already established by
`crates/worldwake-cli/src/bin/observer.rs`): it advances the simulation via the
existing `step_tick()` API but never mutates world meaning.

## Phase

Developer Tooling (not phase-gated; independent of engine phase work).

## Status

Draft.

## Crates

- **New**: `worldwake-visualizer` (binary crate with an `eframe` app entry
  point).
- **Depends on** (as library deps): `worldwake-core`, `worldwake-sim`,
  `worldwake-systems`, `worldwake-ai`, `worldwake-cli` (for `load_scenario_file`
  / `spawn_scenario` / `spawn_scenario_ignoring_lints` / `entity_display_name`
  reuse).
- **New external deps**: `eframe`, `egui` (MIT OR Apache-2.0); `rfd` (MIT, for
  the native "Load scenario…" file dialog).
- **Reused existing deps**: `rand_chacha` (workspace), `serde`,
  `ron`-derived scenario loader (via `worldwake-cli`).

## Dependencies

No spec-level dependencies. The visualizer only consumes already-landed public
APIs: scenario spawning, `step_tick`, topology/`World` read surface, agent
ECS-component accessors, scheduler `active_actions`, `AgentBeliefStore` (and
sibling belief components), `AgentTickDriver`/`AgentDecisionRuntime`,
action/decision trace sinks.

Three completed (archived) specs supply types this spec reads:
- [archive/specs/S81-golden-gaps-simulation-remediation.md](../archive/specs/S81-golden-gaps-simulation-remediation.md) — `DeadAt { tick, cause }` (`crates/worldwake-core/src/combat.rs`).
- [archive/specs/S111-scenario-homogeneity-lints.md](../archive/specs/S111-scenario-homogeneity-lints.md) — scenario-load lints surfaced via `--ignore-lints`.
- [archive/specs/S114-plan-step-guards.md](../archive/specs/S114-plan-step-guards.md) — `PlanGuard` and `PlanExpectation` attached to `PlannedStep`.

## Design Goals

- **Zero simulation coupling**: the visualizer is a library-level client of the
  same APIs `observer.rs` already uses. No engine code changes to land this
  spec.
- **Works for any scenario**: reads the existing `.ron` schema; adds no
  scenario-side requirements.
- **Readable topology**: places auto-positioned via force-directed layout;
  dashed thick edges carry tick-count labels.
- **Transit legibility**: agents mid-travel render at the correct fractional
  position on the edge (`k/n` of the way from origin to destination).
- **Rich per-agent inspection**: hover surfaces colored need bars + compact
  status; click opens a tabbed modal with beliefs, inventory, goals, plan, and
  a scoped trace ring buffer.
- **Deterministic observer**: the visualizer cannot change simulation outcomes.
  Tick rate only affects when the engine computes, never what it computes.
- **No big-framework lock-in**: egui is immediate mode and does not impose a
  reactive architecture; the visualizer owns its own `main`, its own loop, and
  its own draw calls.

## Non-Goals

- **Replay / timeline scrubbing**: explicitly out of scope for v1. If the user
  wants to rewind, Reset re-runs the scenario from tick 0 (bit-identical via
  seed).
- **Edit-and-refresh scenario hot-reload**: v1 reloads on explicit Reset.
- **Human control of agents from the visualizer**: inspection only. Human
  control lives in `worldwake-cli` (REPL). Not re-implemented here.
- **Golden E2E coverage for visualizer output**: no new golden scenarios.
  Layout and snapshot builders are unit-tested; visual correctness is manual.
- **Cross-scenario diffing or comparison**: one scenario per app instance.
- **Performance tuning for >30 places**: the force-directed layout is tuned for
  the current scenario catalog (4–8 places). It degrades gracefully but is not
  optimized for city-scale graphs.

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P12 (Performance compresses computation, never causality) | Play/pause/step/speed change *when* ticks compute, never what they mean. The sim is deterministic from seed + state; observed outcomes match `observer.rs` on the same scenario and seed. Pausing doesn't freeze "world time" — world time IS the tick count, and paused = "no ticks computed yet." |
| P14 / P19 (Belief vs world; UI omniscience carve-out) | FND-19 explicitly permits debug/authoring/replay tools to surface ground-truth belief/inventory/goal state. This spec is that debug tool; surfacing beliefs directly from authoritative state is the entire point of a belief-inspection tab. Not a violation of P14 because no agent is shown fact via this surface — the *developer* is. |
| P26 (Systems interact through state, not through each other) | `worldwake-visualizer` is a new crate that depends on the public read-surfaces of `core`/`sim`/`systems`/`ai`/`cli`. It writes nothing to any system. No engine system imports the visualizer. |
| P27 (Derived summaries are caches, never truth) | Frame snapshot (`places`, `edges`, `agents` views) is rebuilt every frame from authoritative state. FR layout positions are cached, keyed on topology identity, invalidated on scenario reload. Neither is ever promoted to authoritative state. FR layout determinism is within-platform only — `f32` `sqrt` and division are hardware-dependent — but layout positions are UI-only and never authoritative, so cross-platform variation is acceptable per the CLAUDE.md "no floats in authoritative state" invariant. |
| P28 (No backward compat in live authority) | N/A — new code, no legacy path. |
| P29 (Debuggability is a product feature) | This spec directly instantiates P29. It answers: "why did this agent do that?" (Plan + Traces tabs), "what does this agent know?" (Beliefs tab), "where did this item come from?" (Inventory tab + Traces tab cross-reference), "who is where and how did they get there?" (canvas + transit lerp). |
| P29A (Causal history is authoritative, append-only, queryable) | The visualizer reads the append-only event log but does not depend on event-log mutations to function. Reset re-runs from tick 0; the replay contract is unchanged. |

## Deliverables

### 1. New Crate `worldwake-visualizer`

`crates/worldwake-visualizer/Cargo.toml` declares:

```toml
[package]
name = "worldwake-visualizer"
version = "0.1.0"
edition = "2021"

[dependencies]
worldwake-core = { path = "../worldwake-core" }
worldwake-sim = { path = "../worldwake-sim" }
worldwake-systems = { path = "../worldwake-systems" }
worldwake-ai = { path = "../worldwake-ai" }
worldwake-cli = { path = "../worldwake-cli" }
eframe = { version = "0.34", default-features = false, features = ["default_fonts", "glow", "wayland", "x11"] }
egui = "0.34"
rand_chacha = "0.3"
rfd = { version = "0.14", default-features = false, features = ["xdg-portal", "tokio"] }
clap = { version = "4", features = ["derive"] }
```

The workspace `Cargo.toml` `members` array gains the new entry:

```toml
members = [
    "crates/worldwake-core",
    "crates/worldwake-sim",
    "crates/worldwake-systems",
    "crates/worldwake-ai",
    "crates/worldwake-cli",
    "crates/worldwake-visualizer",  # new
]
```

### 2. Entry Point

```rust
// crates/worldwake-visualizer/src/main.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = VisualizerCli::parse();
    let app = VisualizerApp::new(cli)?;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("Worldwake Visualizer"),
        ..Default::default()
    };
    eframe::run_native(
        "worldwake-visualizer",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    )?;
    Ok(())
}
```

The CLI accepts an optional scenario path (matching `observer.rs`'s ergonomics).
`clap`'s derive auto-generates `--help`:

```
worldwake-visualizer [--ignore-lints] [scenarios/<name>.ron]
```

When `--ignore-lints` is set, the visualizer calls
`worldwake_cli::scenario::spawn_scenario_ignoring_lints` (`crates/worldwake-cli/src/scenario/mod.rs:134`);
otherwise it calls `spawn_scenario` (`scenario/mod.rs:125`). This mirrors
observer.rs's selection at `crates/worldwake-cli/src/bin/observer.rs:3631-3635`.

If no scenario is provided at startup, the app opens with a "Load scenario…"
empty state.

### 3. `VisualizerApp` Struct

The host pattern mirrors `observer.rs` exactly — separate persistent fields
for the simulation pieces, with `TickStepServices` and
`AutonomousControllerRuntime` constructed *fresh each tick* because both are
lifetime-bound types. `TickStepServices` holds
`Option<&'a mut ActionTraceSink>` and similar mutable borrows
(`crates/worldwake-sim/src/tick_step.rs:19`); `AutonomousControllerRuntime` is
`pub struct AutonomousControllerRuntime<'a> { controllers: Vec<&'a mut dyn AutonomousController> }`
(`crates/worldwake-sim/src/autonomous_controller.rs:32`). Neither can be stored
as an owned struct field.

```rust
struct VisualizerApp {
    // Simulation host (unpacked SpawnedSimulation, exactly as observer.rs):
    sim: SimulationState,                    // owns world, event_log, scheduler, controller, rng, recipe_registry
    action_registries: ActionRegistries,     // .defs + .handlers
    dispatch_table: SystemDispatchTable,
    driver: AgentTickDriver,                 // persistent owner of AgentDecisionRuntime per agent
    scenario_path: PathBuf,

    // Trace sinks (owned here, borrowed into TickStepServices each tick):
    action_trace: ActionTraceSink,
    perception_trace: PerceptionTraceSink,
    request_resolution_trace: RequestResolutionTraceSink,
    politics_trace: PoliticalTraceSink,
    institutional_knowledge_trace: InstitutionalKnowledgeTraceSink,

    // Visualizer state:
    layout: PlaceLayout,
    play_state: PlayState,
    speed: TicksPerSecond,
    tick_carry: f32,
    selected_agent: Option<EntityId>,
    hovered_agent: Option<EntityId>,
    trace_buffers: AgentTraceBuffers,
    ui_settings: UiSettings,
}

enum PlayState {
    Paused,
    Playing,
}

struct TicksPerSecond(f32); // 0.5..=50.0
```

The simulation `RecipeRegistry` and `DeterministicRng` already live inside
`SimulationState` (`crates/worldwake-sim/src/simulation_state.rs:6-14`); they
are reached via `sim.recipe_registry()` and `sim.tick_parts_mut()`.

`update()` body per frame:
1. Handle key input: `Space` → single-step; `P` → toggle play; `Ctrl+R` →
   reset; `Esc` → deselect.
2. If `Playing`, accumulate `tick_carry += dt * speed`; while
   `tick_carry >= 1.0`, call the per-tick step routine (below) and decrement.
   Cap at `MAX_TICKS_PER_FRAME = 100`.
3. Build `FrameSnapshot` from authoritative state (see §5).
4. Draw top bar (scenario name, tick, play controls, speed slider).
5. Draw canvas inside `egui::Scene` (pan + zoom) with places + edges + agents.
6. Handle hover / click hit-tests.
7. If `selected_agent.is_some()`, render detail modal.

Per-tick step routine (mirrors `observer.rs:3702-3719`):

```rust
fn step_one_tick(&mut self) {
    let mut controllers = AutonomousControllerRuntime::new(vec![&mut self.driver]);
    let (world, event_log, scheduler, controller, rng, recipe_registry) =
        self.sim.tick_parts_mut();

    let _result = step_tick(
        world,
        event_log,
        scheduler,
        controller,
        rng,
        TickStepServices {
            action_defs: &self.action_registries.defs,
            action_handlers: &self.action_registries.handlers,
            recipe_registry,
            systems: &self.dispatch_table,
            input_producer: Some(&mut controllers),
            action_trace: Some(&mut self.action_trace),
            // …other sinks borrowed here…
        },
    );
}
```

### 4. Canvas Rendering

All canvas drawing uses `egui::Painter` primitives inside an
`egui::Scene::new().show(ui, &mut scene_rect, ...)` container (built-in
pan/zoom on the pinned egui 0.34 API). The mutable `scene_rect` lives in
visualizer app state so pan/zoom persists across frames:

- **Places**: `Shape::Rect` with 8px rounded corners, stroke width 1.5,
  stroke color derived from dominant `PlaceTag`, fill `Color32::from_rgb(36,
  36, 42)`; place name rendered with `TextStyle::Body` at top-left of rect;
  tag pills below name.
- **Edges**: `Shape::dashed_line` with `stroke = Stroke::new(3.0,
  Color32::GRAY)` and `dash_length = 8.0, gap_length = 4.0`; label
  `"{travel_ticks} ticks"` rendered at edge midpoint on a background-filled
  rounded-rect for legibility.
- **Agents at place**: circle radius 10, fan-out around place center at
  ring-radius 22, stable angle = `(stable_index * TAU / agent_count)` where
  `stable_index` is determined by BTreeMap-ordered `EntityId`.
- **Agents in transit**: circle at
  `lerp(place_pos[origin], place_pos[destination], progress_ratio)`; a small
  directional chevron glyph at the leading edge of the circle.
- **Agent color**: control source encodes color
  (`ControlSource::Ai = blue-400`, `Human = gold`, `None = grey`); dead
  agents get a striped grey fill.
- **Agent label**: name in 11pt text anchored 14px below the circle; elided
  to 16 chars.

### 5. Frame Snapshot

The snapshot is a plain-old-data view rebuilt every frame. It is a cache per
P27, never authoritative.

```rust
struct FrameSnapshot {
    tick: Tick,
    places: BTreeMap<EntityId, PlaceView>,
    edges: Vec<EdgeView>,
    agents: BTreeMap<EntityId, AgentView>,
}

struct PlaceView {
    name: String,
    tags: Vec<PlaceTag>,
    position: egui::Pos2,
}

struct EdgeView {
    from: EntityId,
    to: EntityId,
    travel_ticks: u32,
}

struct AgentView {
    name: String,
    control: ControlSource,
    position: AgentPosition,
    alive: bool,
    active_action: Option<ActiveActionView>,
    active_goal: Option<CommittedGoalView>,
    needs: HomeostaticNeeds,
    derived_pressures: DerivedDrivePressures,
    drive_thresholds: DriveThresholds,
}

struct DerivedDrivePressures {
    pain: Permille,
    danger: Permille,
}

enum AgentPosition {
    AtPlace(EntityId),
    InTransit {
        from: EntityId,
        to: EntityId,
        progress: Permille, // (current_tick - departure) / (arrival - departure)
        k_of_n: (u32, u32),
    },
}
```

Snapshot construction reads only through the existing public API surface.
Two distinct data sources are used: ECS state (via `World`) and runtime state
(via `Scheduler` and `AgentTickDriver`).

**Places and edges (topology read surface):**
- `world.topology().place_ids()` for places (`crates/worldwake-core/src/topology.rs:285`).
- `world.topology().place(id) -> Option<&Place>` for the place's name and tags
  (`topology.rs:281`).
- `world.topology().outgoing_edges(place: EntityId) -> &[TravelEdgeId]` to
  enumerate edges out of a place (`topology.rs:293`); each edge ID resolves
  via `world.topology().edge(id) -> Option<&TravelEdge>` (`topology.rs:289`)
  to read `from()`, `to()`, `travel_time_ticks()` (`topology.rs:208-216`).

**Agents (ECS read surface):**
- `world.entities_with_name_and_agent_data()` to enumerate agent entities
  (`crates/worldwake-core/src/world.rs:521`) — same enumerator `observer.rs`
  uses (`observer.rs:3649-3652`).
- Display name via `worldwake_cli::display::entity_display_name(world, id)`
  (`crates/worldwake-cli/src/display.rs:37`), which falls back to a formatted
  entity ID when no `Name` component is set; or directly via
  `world.get_component_name(id)` (macro-generated at
  `crates/worldwake-core/src/component_schema.rs:17`).
- Location via `world.effective_place(agent) -> Option<EntityId>`
  (`crates/worldwake-core/src/world/placement.rs:10`), inherited by `WorldTxn`
  via `Deref`.
- Carry data via `world.possessions_of(agent) -> Vec<EntityId>`
  (`crates/worldwake-core/src/world/ownership.rs:50`).
- Needs via `world.get_component_homeostatic_needs(agent)` returning
  `&HomeostaticNeeds` (`needs.rs:9`; macro-generated accessor at
  `component_schema.rs:1242`).
- Derived pain/danger pressures via `worldwake_ai::derive_pain_pressure` and
  `derive_danger_pressure` over the agent-scoped `PerAgentBeliefView`; these
  are cached in the visualizer snapshot only and are not authoritative state.
- Drive thresholds via `world.get_component_drive_thresholds(agent)`
  (`crates/worldwake-core/src/drives.rs:58`; accessor at
  `component_schema.rs:1217`).
- Active intention via `world.get_component_intention_frame(agent)`
  (`crates/worldwake-core/src/intention_frame.rs`; accessor at
  `component_schema.rs:1777`).
- Belief surfaces (separate components) — see §7 Beliefs tab for the full list.

**In-transit agents (scheduler read surface):**
- Active actions live on the scheduler:
  `sim.scheduler().active_actions() -> &BTreeMap<ActionInstanceId, ActionInstance>`
  (`crates/worldwake-sim/src/scheduler.rs:122`). Filter the map by
  `instance.actor == agent_id`.
- For each matched `ActionInstance`, inspect `local_state: Option<ActionState>`
  (`crates/worldwake-sim/src/action_instance.rs:16`). When the variant is
  `ActionState::Travel { edge_id, origin, destination, departure_tick, arrival_tick }`
  (`crates/worldwake-sim/src/action_state.rs:21-27`), the snapshot computes
  `progress = (current_tick - departure_tick) / (arrival_tick - departure_tick)`
  as `Permille`, and `k_of_n = (current_tick - departure_tick, arrival_tick - departure_tick)`.

**Agent decision runtime (driver read surface):**
- `AgendaState` (committed/pending/suspended goals) lives on
  `AgentDecisionRuntime`, owned by the persistent `AgentTickDriver`. Read via
  `self.driver.runtime(agent) -> Option<&AgentDecisionRuntime>`
  (`crates/worldwake-ai/src/agent_tick/mod.rs:121`), then `.agenda_state`
  (`crates/worldwake-ai/src/decision_runtime.rs:181`).
- Active plan via the same runtime: `runtime.current_plan: Option<PlannedPlan>`
  (`decision_runtime.rs:152`).

### 6. Tooltip (Hover)

`egui::Response::on_hover_ui` for the hit-rect of each agent circle. Tooltip
width ~240px; layout:

- **Row 1**: bold agent name · control-source badge · alive/dead glyph.
- **Row 2**: location or transit string (`"@ Riverside Camp"` /
  `"→ Fertile Fields (3/7)"`).
- **Row 3**: active action (`"travel [2/4]"` / `"eat"` / `"—"`).
- **Row 4**: active goal — `GoalKind` debug-name and `motive_score` from
  `AgendaState.committed`, or `"no goal"` when `committed.is_none()`.
- **Need bars** (the visually distinct feature): vertical stack of labeled
  rows for the five core needs (`Hunger`, `Thirst`, `Fatigue`, `Bladder`,
  `Dirtiness`). Each row:
  - Label on left (fixed 72px).
  - Horizontal bar of fixed length 140px, filled proportionally to
    `need_value / 1000`.
  - **Zone-colored fill**, derived from `DriveThresholds` for that need:
    - `< low` → green (`Color32::from_rgb(90, 180, 90)`)
    - `low..medium` → yellow-green (`Color32::from_rgb(170, 200, 80)`)
    - `medium..high` → amber (`Color32::from_rgb(230, 180, 60)`)
    - `high..critical` → orange-red (`Color32::from_rgb(230, 110, 60)`)
    - `>= critical` → red (`Color32::from_rgb(220, 60, 60)`) with a subtle
      1px outline pulse (`alpha = 0.6 + 0.4 * sin(time * 4.0)`).
  - Threshold tick marks as thin 1px vertical lines on the bar at the
    `low / medium / high / critical` positions.
  - Numeric value right-aligned after the bar (`"{val}"` without unit).
- (`Pain`, `Danger` rows shown only when non-zero to reduce clutter.)

### 7. Detail Modal (Click)

`egui::Modal` anchored centered, resizable, initial size 820x640. A
`egui::CollapsingHeader`-based tab strip across the top; selected tab renders
below.

Tabs:

1. **Overview**: Tooltip content expanded, plus `AgendaState` entries —
   `committed: Option<AgendaEntry>`, `pending: BTreeMap<…, AgendaEntry>`,
   `suspended: BTreeMap<…, AgendaEntry>` (`crates/worldwake-ai/src/agenda_types.rs:13`).
   Each entry shows its `goal_kind`, `motive_score: u32`
   (`agenda_types.rs:30`), and `provenance: Option<RankedGoalProvenance>`.
2. **Needs**: Full-width need bars (same widget, larger), plus
   `MetabolismProfile` decay rates (`crates/worldwake-core/src/needs.rs:117`)
   and `DriveEscalationProfile`
   (`crates/worldwake-core/src/drive_escalation_profile.rs:38`) if registered.
3. **Beliefs**: Each belief surface lives in a *distinct ECS component*. The
   tab dedicates a collapsible section per source:
   - `AgentBeliefStore` (`crates/worldwake-core/src/belief.rs:45`), accessed
     via `world.get_component_agent_belief_store(agent)`
     (`component_schema.rs:792`). Sub-sections render its top-level fields:
     `entity_claims: BTreeMap<EntityId, Vec<EntityBeliefClaim>>` (with
     `aspect: EntityBeliefAspect`, `confidence: Permille`, freshness
     computed as `current_tick - acquired_tick`); `known_entities`;
     `social_observations`; `told_beliefs`; `heard_beliefs`;
     `asked_witnesses`; `place_visits: BTreeMap<EntityId, PlaceVisitRecord>`;
     `institutional_beliefs`.
   - `LastSeenMemory` (separate component;
     `crates/worldwake-core/src/expectation.rs:136`).
   - `ExpectationStore` (separate component; `expectation.rs:80`).
   - `SourceReliability` (separate component;
     `crates/worldwake-core/src/experience.rs:84`).
   Within each section, entries are sorted by staleness (freshest first).
4. **Inventory**: Carried items via `world.possessions_of(agent)`
   (`crates/worldwake-core/src/world/ownership.rs:50`). Lots are themselves
   entities (`EntityKind::ItemLot`); identity is `EntityId`. Columns:
   `CommodityKind | Quantity | LotEntity (EntityId) | GroundSince (if any)`;
   totals at the bottom.
5. **Plan**: Active `IntentionFrame` (or `"no active intention"`) read via
   `world.get_component_intention_frame(agent)`. Plan step list rendered from
   `driver.runtime(agent).map(|r| &r.current_plan)` (`PlannedPlan` of
   `PlannedStep`s; `crates/worldwake-ai/src/planner_ops.rs:830`); current step
   highlighted. Each step displays its `guard: Option<PlanGuard>` and
   `expectations: Vec<PlanExpectation>` (fields at `planner_ops.rs:826-827`;
   types at `crates/worldwake-ai/src/plan_guard.rs:8,53`). Last replan reason
   sources from the agent's scoped `decision_trace` ring buffer (last
   `AgentDecisionTrace` whose outcome is a replan); `ReplanReason` and
   `PlanInvalidationReason` are the two enums to extract
   (`crates/worldwake-core/src/decision_event_payload.rs`).
6. **Traces**: Last 50 entries from the agent's scoped ring buffer. Two
   sub-columns: `Decision` (filtered `AgentDecisionTrace` entries;
   `crates/worldwake-ai/src/decision_trace.rs:89`) and `Action` (filtered
   `ActionTraceEvent` entries;
   `crates/worldwake-sim/src/action_trace.rs:20`). Newest first.

### 8. Per-Agent Trace Ring Buffers

The visualizer installs its own trace sinks (mirroring `observer.rs`'s
pattern). Each sink keeps a per-agent `VecDeque` capped at 50 entries:

```rust
struct AgentTraceBuffers {
    decisions: BTreeMap<EntityId, VecDeque<AgentDecisionTrace>>,
    actions: BTreeMap<EntityId, VecDeque<ActionTraceEvent>>,
    capacity: usize, // 50
}
```

`ActionTraceSink` is owned by `VisualizerApp` (see §3) and borrowed into
`TickStepServices` at the start of each `step_one_tick` call. Decision traces
are collected by the tracing sink owned by `AgentTickDriver`. After each tick,
the visualizer drains both carriers into `AgentTraceBuffers` and clears the
per-tick sinks so entries are not duplicated. They add no runtime cost when no
agent is selected — they always record, selection just controls display.

### 9. Force-Directed Layout (FR)

A hand-rolled Fruchterman-Reingold implementation in
`crates/worldwake-visualizer/src/layout.rs`. Target: ~80 lines. API:

```rust
pub struct PlaceLayout {
    positions: BTreeMap<EntityId, egui::Pos2>,
    topology_fingerprint: u64, // xxhash of sorted (place_ids, edges)
}

impl PlaceLayout {
    pub fn compute(
        places: &[EntityId],
        edges: &[(EntityId, EntityId, u32 /* travel_ticks */)],
        seed: u64,
    ) -> Self { /* ... */ }
}
```

Algorithm (standard FR with weighted ideal length):

1. Seed initial positions with `ChaCha8Rng::seed_from_u64(seed)` inside a
   fixed `[0, 1000] × [0, 1000]` box. This `ChaCha8Rng` is layout-local; it
   does not interact with `SimulationState.rng_state` (which remains the
   simulation's authoritative `DeterministicRng`,
   `crates/worldwake-sim/src/deterministic_rng.rs:13`).
2. Compute `k_base = sqrt(area / n)` where `area = 1e6`, `n = num_places`.
3. Per-edge ideal length `k_e = k_base * travel_ticks_e`.
4. For 200 iterations:
   - Compute pairwise repulsive forces: `F_rep = k_base^2 / |d|` along `d`.
   - Compute per-edge attractive forces: `F_att = |d|^2 / k_e` along edge.
   - Cool: `t_i = t_0 * (1 - i / iterations)`.
   - Clamp displacement magnitude per node to `t_i`.
5. Final centering: translate positions so the bounding-box center is at
   `(500, 500)`.
6. Compute `topology_fingerprint` for cache-invalidation.

Iteration order is `BTreeMap`-sorted for determinism. All arithmetic is
`f32`; summation order is fixed (nodes sorted by ID). Determinism is
within-platform only; cross-platform `f32` `sqrt`/division reproducibility is
not guaranteed and not required, since layout positions are UI-only and
never authoritative state.

Cache invalidation: on scenario reload, `topology_fingerprint` changes
(different place IDs), triggering recompute. Within a run, topology is
immutable, so layout is computed exactly once.

### 10. Step Controls & Play State

Input handling in the top bar and global keybinds:

- **Space**: step exactly one tick (works in both Playing and Paused states;
  in Playing it inserts an extra tick beyond the normal cadence).
- **P**: toggle play/pause.
- **Ctrl+R**: confirm dialog → reload scenario from disk, re-spawn, tick = 0.
- **Esc**: clear `selected_agent`.
- **Mouse wheel on canvas**: zoom (handled by `egui::Scene`).
- **Middle-drag on canvas**: pan (handled by `egui::Scene`).
- **Left-click on agent**: select (open modal).
- **Left-click on canvas background**: deselect.
- **Speed slider**: logarithmic 0.5x–50x, default 5x (ticks/sec).

Top-bar layout: `[Scenario name] [Tick: 247] [⏸/▶] [⏭ Step] [🔄 Reset] [Speed:
—•—•— 5.0 t/s] [Load scenario…]`.

### 11. Edge Cases

- **Dead agents**: drawn striped-grey at last known `effective_place(agent)`;
  still inspectable via click; Needs tab shows final values and `DeadAt.cause`
  from `crates/worldwake-core/src/combat.rs:77` (delivered by S81).
- **Multi-agent same place**: circles fan out around the place center at
  ring-radius 22, evenly spaced by BTreeMap-ordered ID. Label elision
  prevents text overlap.
- **Self-loops or parallel edges**: not present in current scenarios.
  Self-loops render as a small arc above the place; parallel edges offset
  perpendicularly by 6px per duplicate.
- **Agent in transit at tick 0**: never happens (scenario spawns always
  place agents). Defensive: assert-fail with a clear message if a snapshot
  builder encounters this.
- **Scenario file not found / parse error**: app remains in "empty" state
  with a visible toast error; does not crash.
- **Scenario reload with different topology**: layout recomputes from scratch
  (new fingerprint).
- **Transit-progress division by zero**: `arrival_tick == departure_tick`
  implies zero-duration travel; the snapshot builder treats this as
  `progress = Permille::new_unchecked(1000)` (the agent has already
  arrived); defensive clamp prevents NaN.
- **Max-speed frame budget**: at `speed = 50.0`, ~50 ticks/sec requested. At
  60fps, that's ~0.83 ticks/frame, well below the
  `MAX_TICKS_PER_FRAME = 100` cap. Cap only kicks in under severe frame
  stalls.
- **`PlaceTag` rendering**: `PlaceTag` is a closed enum
  (`crates/worldwake-core/src/topology.rs:11`, 14 variants, not
  `#[non_exhaustive]`). Every variant must have an explicit color/style
  mapping; adding a new variant fails compilation at the rendering site,
  which is desirable.
- **Agents with no active goal**: shown as `"no goal"` in tooltip; Overview
  tab shows `"idle"`.

### 12. Testing Strategy

- **Unit — layout determinism**: `fr_layout_is_deterministic` — same
  `(places, edges, seed)` → bit-identical `Vec<(EntityId, Pos2)>` across two
  calls *on the same platform*. Core regression guard.
- **Unit — transit-progress computation**: construct a mock snapshot input
  with `departure_tick = 100, arrival_tick = 107, current_tick = 103`;
  assert `progress = Permille::new(429)` (≈3/7 = 0.4286 → 429‰) and
  `k_of_n = (3, 7)`.
- **Unit — need-bar zone classification**: parameterized test covering each
  of the 5 zones against a known `DriveThresholds`.
- **Unit — topology-fingerprint stability**: same
  `(place_set, edge_set)` → same fingerprint regardless of input vector
  order.
- **Smoke — scenario load**: for each `.ron` in `scenarios/`, spawn the
  scenario, build an initial `FrameSnapshot`, assert no panic and all
  agents have resolved positions (either `AtPlace` or `InTransit`).
- **Smoke — stepped advance**: spawn `survival-baseline.ron`, step 100
  ticks, assert the tick counter advances correctly and the snapshot
  remains consistent after each step.
- **No new golden E2E**: the visualizer reads existing goldens' state
  without altering it; does not need its own golden coverage.

### 13. Manual QA Checklist

Documented in `crates/worldwake-visualizer/README.md`. For each landed
scenario under `scenarios/`:

1. `cargo run -p worldwake-visualizer -- scenarios/<name>.ron` opens window
   within 2s.
2. `cargo run -p worldwake-visualizer -- --help` prints the clap-derived
   usage and exits.
3. Places render without overlap; graph fits in window via auto-fit on
   first frame.
4. Dashed edges render with tick-count labels at midpoints.
5. Pan (middle-drag) and zoom (wheel) work on the canvas — confirms
   `egui::Scene` is reachable on the pinned `egui` version. If
   `egui::Scene` is unavailable on the resolved version, fall back to a
   hand-rolled pan/zoom container before continuing.
6. Space advances exactly one tick (tick counter in header increments by
   1).
7. Play + speed slider: tick counter advances at approximately the
   configured rate.
8. Reset returns tick to 0 and places agents at their initial locations.
9. Hover agent → tooltip with zone-colored need bars; bars match numeric
   values.
10. Click agent → modal opens; all 6 tabs render without panic.
11. Traces tab populates with entries after several ticks.
12. Beliefs tab shows entries from `AgentBeliefStore`, `LastSeenMemory`,
    `ExpectationStore`, and `SourceReliability` after the agent has
    observed something.
13. Transit: for `survival-scattered.ron`, an agent on a multi-tick edge
    is visibly lerped across ticks.

## Explicit Opt-Outs from `docs/spec-drafting-rules.md`

This is a tooling-boundary spec: it introduces no new simulation state,
laws, or agent behavior. The following spec-drafting rules are documented
as N/A with explicit reasoning:

| Rule | Status | Reason |
|------|--------|--------|
| FND-01 Section H causal-hooks analysis | N/A | The tool introduces zero new simulation entities, relations, actions, information paths, conserved quantities, scarce capacities, feedback loops, lifecycle states, or boundary conditions. Section H analyzes a proposed *world-system*; there is no world-system here. |
| `Permille` for [0,1] or [0,1000] range values | Partial | The one ratio the tool computes semantically (`transit progress = (current - departure) / (arrival - departure)`) correctly uses `Permille`. UI pixel coordinates and layout-force scalars are screen-space `f32`, which is not a "[0,1]/[0,1000] simulation value" within the scope of the rule. |
| Profile-driven parameters | N/A | The tool has no per-agent behavior tunables. UI constants (step rate cap, dash length, ring-fan-out radius, layout iterations) are app-level config, not world parameters. |
| SystemFn integration | N/A | The visualizer is not a simulation system and registers no `SystemFn`. It is a top-level binary that calls `step_tick` from its event loop. |
| Component registration | N/A | No new ECS components are defined. |
| Cross-system interactions (via FND-26) | N/A | No cross-system interactions. The visualizer is a client of read APIs only. |

## Module Layout

```
crates/worldwake-visualizer/
├── Cargo.toml
├── README.md                     # Manual QA checklist, screenshots TBD
└── src/
    ├── main.rs                   # CLI + eframe entry
    ├── app.rs                    # VisualizerApp struct + update loop + step_one_tick
    ├── snapshot.rs               # FrameSnapshot + builders (topology + scheduler + driver reads)
    ├── layout.rs                 # Fruchterman-Reingold (~80 LOC)
    ├── canvas.rs                 # Painter calls for places/edges/agents
    ├── tooltip.rs                # Hover tooltip with need bars
    ├── modal.rs                  # Detail modal + tab routing
    ├── tabs/
    │   ├── overview.rs
    │   ├── needs.rs
    │   ├── beliefs.rs
    │   ├── inventory.rs
    │   ├── plan.rs
    │   └── traces.rs
    ├── need_bar.rs               # Reusable colored need-bar widget
    ├── trace_buffers.rs          # Per-agent ring-buffer sinks
    └── controls.rs               # Top bar + keybinds
```

Total expected LOC: ~2000–2500 across the crate.

## Open Questions (non-blocking)

1. **`egui_graphs` reuse**: the `egui_graphs` crate bundles FR layout and
   interactive graph rendering. We chose hand-rolled to keep deps minimal and
   determinism guaranteed; if an author later finds the hand-rolled version
   constraining, switching to `egui_graphs` (with a seeded RNG override) is a
   bounded refactor inside `layout.rs` and `canvas.rs`.
2. **Persistent UI settings**: speed, panel open/closed, selected tab could
   be persisted via `eframe::Storage`. Not v1.
3. **Screenshot / canvas export**: `egui` can dump the framebuffer to PNG.
   Useful for bug reports. Deferred.
4. **Scenario-level lint surface**: the existing scenario lints (S111) emit
   warnings at load time. The visualizer surfaces them as a toast overlay
   and respects `--ignore-lints`.

## Rollout

Single wave; no phase gate.

1. Add `crates/worldwake-visualizer/` and register in workspace
   `Cargo.toml`.
2. Implement `layout.rs`, `snapshot.rs`, and their unit tests first (the
   testable substrate).
3. Implement `app.rs` (with `step_one_tick` mirroring `observer.rs:3702-3719`)
   + `canvas.rs` + `controls.rs` (visual substrate).
4. Implement `tooltip.rs` + `need_bar.rs` + `modal.rs` + tab modules.
5. Implement `trace_buffers.rs` and wire sinks into per-tick
   `TickStepServices` construction.
6. Document Manual QA checklist in `README.md`.
7. Verify on each landed scenario in `scenarios/`.
