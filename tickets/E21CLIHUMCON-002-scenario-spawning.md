# E21CLIHUMCON-002: Scenario Spawning (`spawn_scenario()`)

## Summary

Implement `spawn_scenario()`: takes a `ScenarioDef`, builds a `Topology`, creates a `World`, spawns all entities via `WorldTxn`, and returns a fully initialized `SimulationState`. Also implement RON file loading.

## Depends On

- E21CLIHUMCON-001 (scenario types)

## Files to Touch

- `crates/worldwake-cli/src/scenario/mod.rs` — **modify**: add `load_scenario_file()` and `spawn_scenario()` functions
- `crates/worldwake-cli/Cargo.toml` — **verify**: `ron` and `serde` deps already present (no change expected)

## Out of Scope

- Scenario type definitions — done in E21CLIHUMCON-001
- CLI arg parsing — that's E21CLIHUMCON-003
- REPL loop — that's E21CLIHUMCON-003
- Default scenario RON file — that's E21CLIHUMCON-013
- Modifying any types in `worldwake-core` or `worldwake-sim` (use existing APIs only)

## Deliverables

### `load_scenario_file(path: &Path) -> Result<ScenarioDef, ScenarioError>`
- Read file to string
- Parse via `ron::from_str()`
- Return typed error on I/O or parse failure

### `spawn_scenario(def: ScenarioDef) -> Result<SimulationState, ScenarioError>`

Bootstrap sequence (per spec lines 36–48):
1. Build `Topology` from `def.places` + `def.edges`:
   - Create `Place` for each `PlaceDef` (with name, tags)
   - Create `TravelEdge` for each `EdgeDef` (resolve place names → `EntityId`)
   - If `bidirectional`, create reverse edge too
2. `World::new(topology)`
3. Spawn agents via `WorldTxn`:
   - Allocate entity with `EntityKind::Agent`
   - Set `Name`, `AgentData` (with `ControlSource`), `HomeostaticNeeds` (default or overrides)
   - Set optional components: `CombatProfile`, `UtilityProfile`, `MerchandiseProfile`, `TradeDispositionProfile`
   - Place agent at named location via relation
4. Spawn items via `WorldTxn`:
   - Allocate `ItemLot` entities with `CommodityKind`, `Quantity`
   - Place at named location (place or agent) via relation
5. Spawn facilities via `WorldTxn`:
   - Create entities with `WorkstationTag` component at named place
6. Spawn resource sources via `WorldTxn`:
   - Create entities with `ResourceSource` component at named place
7. Build action registries: `build_full_action_registries(&recipes)`
8. Build dispatch table: `dispatch_table()`
9. Create `Scheduler::new_with_tick(Tick(0), SystemManifest::canonical())`
10. Create `ControllerState` — find agent with `ControlSource::Human`, set as controlled
11. Create `DeterministicRng::new(Seed(...))`
12. Create `ReplayState::new(...)`
13. Assemble `SimulationState::new(...)`

### `ScenarioError` enum
- `Io(std::io::Error)`
- `Parse(ron::error::SpannedError)`
- `Validation(String)` — e.g., "place 'Foo' referenced by agent 'Bar' not found"

### Name Resolution
- Build a `BTreeMap<String, EntityId>` as entities are spawned
- Validate all name references resolve before spawning dependent entities
- Order: places first → agents → items (items can reference agents or places)

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-cli` — unit/integration tests:
  - `test_spawn_minimal_scenario`: 1 place, 1 agent → world has 1 place, 1 agent at that place
  - `test_spawn_agents_at_places`: 2 places, 2 agents each at different place → verify placement relations
  - `test_spawn_items_at_place`: items spawned at place → verify ownership/placement
  - `test_spawn_items_on_agent`: items spawned on agent → verify carried relation
  - `test_spawn_with_edges`: edges create travel connections → verify topology adjacency
  - `test_spawn_bidirectional_edge`: bidirectional edge creates both directions
  - `test_spawn_human_control`: agent with `ControlSource::Human` → `ControllerState` tracks it
  - `test_spawn_invalid_place_ref`: agent references nonexistent place → `ScenarioError::Validation`
  - `test_spawn_facilities_and_sources`: workstations and resource sources spawned at correct places

### Invariants That Must Remain True
- `verify_completeness()` passes on the spawned world (every entity has required components)
- Conservation invariants hold (all items accounted for)
- Determinism: same `ScenarioDef` with same seed → identical `SimulationState`
- No direct world mutation outside `WorldTxn`
- `cargo clippy -p worldwake-cli` passes with no warnings
