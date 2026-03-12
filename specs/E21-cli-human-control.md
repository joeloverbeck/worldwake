# E21: CLI & Human Control

## Epic Summary
Implement the text-based CLI for human interaction: scenario loading, affordance-based action menus, agent switching, event log viewer, state inspector, and AI integration. Serves as the manual testing tool for the Phase 2 prototype.

## Phase
Originally Phase 4 — pulled forward post-Phase 2 as the primary manual testing interface for the working simulation (needs, production, trade, combat, GOAP AI).

## Crate
`worldwake-cli`

## Dependencies
- E13 (affordance query for action menus, GOAP AI)
- `worldwake-sim` (SimulationState, step_tick, save/load, affordance query, scheduler)
- `worldwake-systems` (action registries, system dispatch table)
- `worldwake-ai` (AgentTickDriver as AutonomousController)

### External Dependencies
- `clap` (CLI argument parsing with derive macros)
- `rustyline` (REPL line editing with history)
- `ron` (scenario file deserialization)
- `serde` (derive Deserialize for scenario types)

## Deliverables

### Scenario System (RON-Based World Initialization)
Data-driven world initialization replacing hardcoded prototypes:
- RON file format defining places, edges, agents, items, workstations, resource sources
- `ScenarioDef` struct hierarchy with serde Deserialize
- `spawn_scenario()` function: ScenarioDef → fully initialized SimulationState
- Default scenario file (`scenarios/default.ron`) for testing

Bootstrap sequence:
1. Parse CLI args (scenario file path) via clap
2. Load & parse RON scenario file → ScenarioDef
3. Build Topology from ScenarioDef places + edges
4. `World::new(topology)`
5. Spawn entities from ScenarioDef via WorldTxn (agents, items, facilities)
6. `RecipeRegistry::new()` (empty for now, recipes registered separately)
7. `build_full_action_registries(&recipes)` → ActionRegistries { defs, handlers }
8. `dispatch_table()` → SystemDispatchTable
9. `Scheduler::new_with_tick(Tick(0), SystemManifest::canonical())`
10. `ControllerState::with_entity(human_agent_id)` or `ControllerState::new()`
11. `DeterministicRng::new(Seed(seed_bytes))`
12. `ReplayState::new(initial_hash, seed, Tick(0), ReplayRecordingConfig::disabled())`
13. `SimulationState::new(world, event_log, scheduler, recipe_registry, replay_state, controller, rng)`
14. `AgentTickDriver::new(PlanningBudget::default())` — AI controller
15. Enter REPL loop

### AI Integration
- `AgentTickDriver` implements `AutonomousController` trait
- Wrapped in `AutonomousControllerRuntime` as `TickInputProducer`
- Runs each tick alongside human control via `step_tick()`
- AI controls all agents with `ControlSource::Ai`; human controls one agent with `ControlSource::Human`

### Text Command Interface
- Interactive REPL loop via rustyline with history
- Prompt shows current tick, controlled agent name, and location
- Commands parsed via clap derive subcommands
- Entity resolution: try numeric ID → exact name match → prefix match → error with suggestions

### Core Commands

| Command | Description | Handler |
|---------|-------------|---------|
| `tick [n]` | Advance simulation n ticks (default 1). AI runs each tick. | `tick.rs` |
| `status` | Show controlled agent's status (needs, location, current action, wounds) | `tick.rs` |
| `look` | Describe current location, entities present, visible items | `inspect.rs` |
| `actions` | List available actions via `get_affordances()` | `actions.rs` |
| `do <n>` | Execute action by menu number from last `actions` output | `actions.rs` |
| `cancel` | Cancel current action | `actions.rs` |
| `inventory [entity]` | Show carried items with quantities | `inspect.rs` |
| `needs [entity]` | Show homeostatic need levels with urgency bands | `inspect.rs` |
| `inspect <entity>` | Show all components on an entity | `inspect.rs` |
| `relations <entity>` | Show all relations involving entity | `inspect.rs` |
| `events [n]` | Show last n events (default 10) | `events.rs` |
| `event <id>` | Show event details including state deltas | `events.rs` |
| `trace <id>` | Show causal chain walking `EventRecord.cause` (CauseRef) backward to root | `events.rs` |
| `switch <name>` | Switch human control to named agent | `control.rs` |
| `observe` | Enter observer mode (no controlled agent) | `control.rs` |
| `world` | Summary of all places with population | `world_overview.rs` |
| `places` | List places with travel connections | `world_overview.rs` |
| `agents` | List all living agents with location and control source | `world_overview.rs` |
| `goods` | Global goods summary (total quantities by commodity) | `world_overview.rs` |
| `save <path>` | Save simulation state to file | `persistence.rs` |
| `load <path>` | Load simulation state from file | `persistence.rs` |
| `help` | List commands (clap auto-generates) | built-in |
| `quit` | Exit CLI | `repl.rs` |

### Removed Commands (depend on unimplemented systems)
- ~~`beliefs`~~ — requires E14 per-agent belief system
- ~~`order`~~ — "public order per place" has no backing system
- ~~`wait [duration]`~~ — complex event-driven interruption; `tick n` suffices

### Affordance-Based Action Menu
Per spec section 6.4:
- `actions` command queries affordances for controlled agent via `get_affordances()`
- Shows only legal actions from agent's perceived context
- Each action shows: number, name, targets, estimated duration
- `do <n>` selects from menu → creates `InputEvent(RequestAction)` → enqueued in `InputQueue`
- Same affordance query as AI agents (no special player actions)

### Agent Switching
- `switch <name>`: resolve name → EntityId, then:
  - Update `ControllerState` to track new agent
  - Set old agent's `AgentData.control_source` to `ControlSource::Ai`
  - Set new agent's `AgentData.control_source` to `ControlSource::Human`
  - World simulation continues without reset
  - New agent's affordances immediately available
- `observe`: clear controlled entity, set current agent to `ControlSource::Ai`
- Target must be alive and present in world

### Event Log Viewer
- `events [n]`: show last n events from `EventLog` (default 10)
- `event <id>`: show full `EventRecord` details including state deltas
- `trace <id>`: walk `EventRecord.cause` (`CauseRef`) backward to root cause (not forward via effects)

### State Inspector
- `inspect <entity>`: show all components on an entity
- `needs <entity>`: show detailed need levels with urgency bands (via `ThresholdBand`)
- `inventory <entity>`: show all carried items with quantities
- `relations <entity>`: show all relations involving entity

### World Overview
- `world`: show summary of all places with population and notable state
- `places`: list all places with travel connections
- `agents`: list all living agents with location and control source
- `goods`: show global goods summary (total quantities by type)

### Persistence
- `save <path>`: delegate to `worldwake_sim::save()` (bincode format)
- `load <path>`: delegate to `worldwake_sim::load()`, replace current SimulationState

## Module Structure

```
crates/worldwake-cli/src/
  main.rs              — clap CLI args (scenario path), bootstrap, REPL entry
  repl.rs              — rustyline REPL loop + command dispatch
  commands.rs          — CliCommand enum (clap-derived subcommands)
  display.rs           — Formatting helpers (entity_display_name, needs_bar, etc.)
  scenario/
    mod.rs             — ScenarioDef loading + spawn_scenario()
    types.rs           — AgentDef, PlaceDef, EdgeDef, ItemDef, etc.
  handlers/
    mod.rs             — dispatch_command() router
    tick.rs            — tick, status commands
    inspect.rs         — look, inspect, inventory, needs, relations commands
    actions.rs         — actions, do, cancel commands
    events.rs          — events, event, trace commands
    control.rs         — switch, observe commands
    world_overview.rs  — world, places, agents, goods commands
    persistence.rs     — save, load commands
```

## Invariants Enforced
- 9.1: Simulation authority — CLI only reads state and requests actions via InputQueue, never mutates World directly
- 9.12: Player symmetry — no special player actions, same affordance query as AI
- Determinism — all mutations go through the action framework and event log

## Tests
- [ ] T24: Player replacement — switch control to any living agent, world continues
- [ ] T27: Controlled agent death — world continues, control can transfer
- [ ] T12: No player branching — switching to merchant/guard shows appropriate affordances (no special player actions)
- [ ] Action menu shows only legal actions via get_affordances()
- [ ] CLI commands don't mutate world state directly (only through InputQueue)
- [ ] Event trace walks CauseRef backward correctly
- [ ] Agent switching preserves world state
- [ ] Observer mode: simulation runs without controlled agent
- [ ] Scenario loading: RON file → spawn → verify entity counts and component values
- [ ] Integration: load default scenario, tick, verify AI runs, verify affordances, verify events

## Acceptance Criteria
- Text interface for full simulation interaction
- RON-based scenario loading (data-driven world initialization)
- AI agents run autonomously each tick via AgentTickDriver
- Actions only through affordance query (no cheats)
- Agent switching works for any living agent (updates both ControllerState and AgentData.control_source)
- Event log navigable with causal tracing (backward via CauseRef)
- State inspection for debugging and play
- Save/load via existing worldwake-sim persistence
- UI is read-only + action requests (simulation authority)

## Spec References
- Section 3.2 (ControlSource: Human | AI | None, any agent switchable)
- Section 6.4 (human control uses same action query pipeline)
- Section 9.1 (simulation authority: UI only reads or requests)
- Section 9.12 (player symmetry)
- Section 9.21 (controlled-agent mortality)
