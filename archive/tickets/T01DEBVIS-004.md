# T01DEBVIS-004: VisualizerApp host + step controls + scenario reload

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: [T01DEBVIS-001](T01DEBVIS-001.md)

## Problem

The shell from T01DEBVIS-001 opens a window but does not advance the simulation. This ticket replaces the shell with the full `VisualizerApp` from spec T01 §D3 and the step controls from §D10: a per-tick step routine that mirrors `crates/worldwake-cli/src/bin/observer.rs:3702-3719` exactly, plus a top bar with play/pause, single-step, reset, and a logarithmic 0.5×–50× speed slider. Distributed edge cases from §D11 land here: scenario file not found (toast, no crash), max-speed frame budget cap (`MAX_TICKS_PER_FRAME = 100`), and scenario reload (re-call `spawn_scenario`, recompute layout if topology fingerprint changed).

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The observer reference pattern at `crates/worldwake-cli/src/bin/observer.rs:3702-3719` constructs `AutonomousControllerRuntime::new(vec![&mut driver])` and `TickStepServices { ... }` fresh each tick using `sim.tick_parts_mut()`. Reassessment 2026-04-25 confirmed both types are lifetime-bound and cannot be stored as plain fields:
   - `TickStepServices<'a>` at `crates/worldwake-sim/src/tick_step.rs:19` carries `Option<&'a mut ActionTraceSink>` plus other mutable borrows.
   - `AutonomousControllerRuntime<'a>` at `crates/worldwake-sim/src/autonomous_controller.rs:32` is `pub struct AutonomousControllerRuntime<'a> { controllers: Vec<&'a mut dyn AutonomousController> }`.
2. `SimulationState::tick_parts_mut` at `crates/worldwake-sim/src/simulation_state.rs:148` returns the split-borrow tuple `(&mut World, &mut EventLog, &mut Scheduler, &mut ControllerState, &mut DeterministicRng, &RecipeRegistry)` — the visualizer follows this exact split-borrow pattern.
3. `SpawnedSimulation { state: SimulationState, action_registries: ActionRegistries, dispatch_table: SystemDispatchTable }` at `crates/worldwake-cli/src/scenario/mod.rs:40-44` is the bundle returned by `spawn_scenario` / `spawn_scenario_ignoring_lints`. The visualizer unpacks all three as separate persistent fields.
4. `AgentTickDriver` at `crates/worldwake-ai/src/agent_tick/mod.rs:75` is the persistent owner of `AgentDecisionRuntime` per agent. It must live on `VisualizerApp` and be borrowed mutably into `AutonomousControllerRuntime::new(vec![&mut self.driver])` each tick.
5. Tooling-only ticket — the visualizer writes nothing to authoritative engine state. Per template item 11: `step_one_tick` does not manipulate `ControlSource` or queued inputs beyond the standard tick; retained intent and runtime-driver state are unaffected.

## Architecture Check

1. Mirroring observer.rs's exact pattern (separate persistent fields + per-tick `TickStepServices`/`AutonomousControllerRuntime` construction) keeps the visualizer aligned with the existing tooling-binary precedent. No second host abstraction is introduced (`VisualizerHost` was considered and rejected at reassessment time per Q1).
2. The `--ignore-lints` selection is wired by choosing between `spawn_scenario` and `spawn_scenario_ignoring_lints` — no parallel lint-bypass path. Scenario reload uses the same selection.
3. `MAX_TICKS_PER_FRAME = 100` is a UI frame-budget cap, not a simulation rate cap. Per FOUNDATIONS Alignment P12 (T01): play/pause/speed change *when* ticks compute, never what they mean. The sim is deterministic from seed + state regardless of cadence.

## Verification Layers

1. Per-tick step correctness → focused runtime test (`step_one_tick_advances_scheduler_tick`) loading `survival-baseline.ron`, calling `step_one_tick` once, and asserting `sim.scheduler().current_tick()` advanced by exactly 1 (action trace and decision trace are populated by sinks borrowed each tick; full trace integration is T01DEBVIS-009).
2. Scenario reload determinism → focused unit test (`reset_reloads_at_tick_zero`) calling reload after N ticks, asserting `sim.scheduler().current_tick() == Tick(0)` post-reset.
3. Startup load failure handling → focused unit test (`missing_startup_scenario_opens_empty_with_toast`) attempts to load a missing `.ron` file and asserts the app remains empty with an in-app error toast.
4. Per template item 6: layout/snapshot/canvas surfaces are downstream and are tested by their own tickets (002, 003, 005). Action-trace and decision-trace assertions are deferred to T01DEBVIS-009 where sinks are fully wired.

## What to Change

### 1. Replace `VisualizerApp` shell with full struct (D3)

`crates/worldwake-visualizer/src/app.rs` — replace the shell from T01DEBVIS-001:

```rust
pub struct VisualizerApp {
    sim: SimulationState,
    action_registries: ActionRegistries,
    dispatch_table: SystemDispatchTable,
    driver: AgentTickDriver,
    scenario_path: PathBuf,
    ignore_lints: bool,

    action_trace: ActionTraceSink,
    decision_trace: DecisionTraceSink,
    perception_trace: PerceptionTraceSink,
    request_resolution_trace: RequestResolutionTraceSink,
    politics_trace: PoliticalTraceSink,
    institutional_knowledge_trace: InstitutionalKnowledgeTraceSink,

    layout: PlaceLayout,
    play_state: PlayState,
    speed: TicksPerSecond,
    tick_carry: f32,
    selected_agent: Option<EntityId>,
    hovered_agent: Option<EntityId>,
    ui_settings: UiSettings,
}

enum PlayState { Paused, Playing }
struct TicksPerSecond(f32); // 0.5..=50.0
```

`new(cli)` loads the scenario, unpacks `SpawnedSimulation`, computes initial `PlaceLayout`, default-constructs sinks and driver. Driver retention is required so `AgentDecisionRuntime` per agent persists across ticks.

### 2. `step_one_tick()` mirroring observer.rs:3702-3719

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
            // …other sinks… (full set wired in T01DEBVIS-009)
        },
    );
}
```

Sink fields beyond `action_trace` are constructed in T01DEBVIS-001 / this ticket but their per-frame drain into per-agent ring buffers is T01DEBVIS-009's responsibility.

### 3. `update()` body per spec T01 §D3

1. Handle key input: `Space` → `step_one_tick()`; `P` → toggle play; `Ctrl+R` → confirm-reset; `Esc` → clear `selected_agent`.
2. If `Playing`, accumulate `tick_carry += dt * speed`; while `tick_carry >= 1.0`, call `step_one_tick()` and decrement. Cap loop at `MAX_TICKS_PER_FRAME = 100`.
3. Draw top bar via `controls::draw_top_bar`.
4. Placeholder canvas area (canvas rendering lands in T01DEBVIS-005 — show a "canvas placeholder" panel here).
5. Tooltip / modal hooks are stubs (filled in T01DEBVIS-006 / -007).

### 4. Top bar (D10) — `controls.rs`

Create `crates/worldwake-visualizer/src/controls.rs`:

- Top bar layout: `[Scenario name] [Tick: N] [⏸/▶] [⏭ Step] [🔄 Reset] [Speed: slider] [Load scenario…]`.
- Speed slider: logarithmic 0.5×–50×, default 5×.
- Reset: `egui::Modal` confirm dialog → on accept, reload scenario from disk, re-spawn (respecting `--ignore-lints`), recompute layout if `topology_fingerprint` changed, reset `tick_carry = 0`, drop driver and create a fresh one.
- Load scenario: `rfd::FileDialog::new().pick_file()`; on selection, replace `scenario_path` and trigger reload.

### 5. Distributed D11 sub-cases

- **Scenario file not found / parse error**: `new(cli)` retains its live `Result<Self, ScenarioError>` signature for `main` compatibility, but startup scenario load failures are captured inside the returned app as an empty state with a visible toast carrying the error message. No panic.
- **Max-speed frame budget**: the `while tick_carry >= 1.0` loop in `update()` is capped at `MAX_TICKS_PER_FRAME = 100` per spec §D11 to bound worst-case frame time.

### 6. Wire `controls` module

Add `pub mod controls;` to `crates/worldwake-visualizer/src/lib.rs`.

## Files to Touch

- `crates/worldwake-visualizer/src/app.rs` (modify — replace shell with full implementation)
- `crates/worldwake-visualizer/src/controls.rs` (new)
- `crates/worldwake-visualizer/src/lib.rs` (modify — add `controls` module declaration)

## Out of Scope

- Canvas drawing of places/edges/agents (T01DEBVIS-005) — `update()` here renders only the top bar and a placeholder canvas region.
- Tooltip on hover (T01DEBVIS-006).
- Detail modal on click (T01DEBVIS-007 / -008).
- Per-agent trace ring buffer drain after each tick (T01DEBVIS-009) — sinks are owned and borrowed into `TickStepServices`, but their drained contents are not yet routed into per-agent buffers.
- Self-loop / parallel edge rendering (T01DEBVIS-005).
- Multi-agent fan-out and dead-agent striping (T01DEBVIS-005).

## Acceptance Criteria

### Tests That Must Pass

1. `step_one_tick_advances_scheduler_tick` — load `survival-baseline.ron` via `spawn_scenario_ignoring_lints`, instantiate `VisualizerApp`, call `step_one_tick` once, assert `sim.scheduler().current_tick()` advanced by 1.
2. `step_one_tick_advances_100_ticks_without_panic` — call `step_one_tick` 100 times, assert tick advanced by 100 and no panic occurred.
3. `reset_reloads_at_tick_zero` — after stepping N ticks, call the reset routine, assert `sim.scheduler().current_tick() == Tick(0)`.
4. Existing suite: `cargo test -p worldwake-visualizer` passes.

### Invariants

1. `step_one_tick` constructs `TickStepServices` and `AutonomousControllerRuntime` fresh each call; neither type is stored as a `VisualizerApp` field.
2. The sim is deterministic from seed + state; tick rate (Paused/Playing/speed) only affects *when* ticks compute, never *what* they compute (FND-12 / FOUNDATIONS Alignment P12).
3. Reset re-runs from `Tick(0)`; topology changes trigger layout recomputation (fingerprint check from T01DEBVIS-002).
4. The visualizer never writes to authoritative engine state directly; every state change flows through `step_tick`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-visualizer/src/app.rs` (`#[cfg(test)] mod tests`) — `step_one_tick_advances_scheduler_tick`, `step_one_tick_advances_100_ticks_without_panic`, `reset_reloads_at_tick_zero`, `missing_startup_scenario_opens_empty_with_toast`. Smoke tests use `survival-baseline.ron`.

### Commands

1. `cargo test -p worldwake-visualizer app::`
2. `cargo test -p worldwake-visualizer`
3. `./scripts/verify.sh`

## Outcome

Completed on 2026-04-25.

- Replaced the visualizer shell with an operational host that owns the spawned `SimulationState`, `ActionRegistries`, `SystemDispatchTable`, persistent `AgentTickDriver`, trace sinks, layout cache, play state, speed, tick carry, selection state, reset modal state, and toast state.
- Implemented `step_one_tick` using the observer-style split borrow: `AutonomousControllerRuntime::new(vec![&mut self.driver])`, `sim.tick_parts_mut()`, and a fresh `TickStepServices` per tick. The visualizer does not directly mutate authoritative world state outside `step_tick`.
- Added `controls.rs` and wired `pub mod controls;`. The top bar shows scenario name, tick, play/pause, step, reset, logarithmic 0.5x-50x speed slider, and load-scenario control.
- Implemented play cadence with `MAX_TICKS_PER_FRAME = 100`, reset/reload with fresh driver and trace sinks, topology-fingerprint layout reuse/recompute, and missing startup scenario handling through an in-app toast.
- Added focused app-host tests for one-tick advance, 100-tick advance, reset to `Tick(0)`, and missing startup scenario behavior.

## Deviations

- `new(cli)` keeps the live `Result<Self, ScenarioError>` return type instead of introducing a separate `VisualizerError`; startup load errors are converted into app state so `main` still opens the empty visualizer.
- `decision_trace` is retained as a staged visualizer-owned sink field for the later per-agent ring-buffer ticket, while the active AI decision trace producer remains the persistent `AgentTickDriver` tracing sink.

## Verification Result

- Passed `cargo test -p worldwake-visualizer --lib -- --list`
- Passed `cargo test -p worldwake-visualizer --lib app::tests::step_one_tick_advances_scheduler_tick -- --exact`
- Passed `cargo test -p worldwake-visualizer --lib app::tests::step_one_tick_advances_100_ticks_without_panic -- --exact`
- Passed `cargo test -p worldwake-visualizer --lib app::tests::reset_reloads_at_tick_zero -- --exact`
- Passed `cargo test -p worldwake-visualizer --lib app::tests::missing_startup_scenario_opens_empty_with_toast -- --exact`
- Passed `cargo test -p worldwake-visualizer`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `bash scripts/verify.sh` (`cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p worldwake-cli --bin scenario-coverage -- --check`)
- After the final top-bar glyph-label edit, passed `cargo test -p worldwake-visualizer` and `cargo clippy -p worldwake-visualizer --all-targets -- -D warnings`
