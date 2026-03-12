**Status**: ✅ COMPLETED

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

### `spawn_scenario(def: ScenarioDef) -> Result<SpawnedSimulation, ScenarioError>`

Returns a `SpawnedSimulation` struct that bundles persistent `SimulationState` with transient runtime artifacts:
```rust
pub struct SpawnedSimulation {
    pub state: SimulationState,
    pub action_registries: ActionRegistries,
    pub dispatch_table: SystemDispatchTable,
}
```
Rationale: `SimulationState` is serializable persistent state (save/load). Action registries and dispatch tables are derived runtime state rebuilt from the recipe registry — they don't belong in `SimulationState` (Principle 3: no derived state stored as authoritative). Bundling them in a wrapper keeps initialization in one place while preserving the boundary.

Bootstrap sequence (per spec lines 36–48):
1. Build `Topology` from `def.places` + `def.edges`:
   - Assign synthetic `EntityId { slot: N, generation: 0 }` for each place (topology is built before World)
   - Create `Place` for each `PlaceDef` (with name, tags)
   - Create `TravelEdge` for each `EdgeDef` (resolve place names → `EntityId`)
   - If `bidirectional`, create reverse edge too
2. `World::new(topology)`
3. Spawn agents via `WorldTxn`:
   - `txn.create_agent(name, control_source)` — sets `Name`, `AgentData` automatically
   - Set `HomeostaticNeeds` (default or overrides) via `txn.set_component_homeostatic_needs()`
   - Set optional components via macro-generated setters: `set_component_combat_profile()`, `set_component_utility_profile()`, `set_component_merchandise_profile()`, `set_component_trade_disposition_profile()`
   - Place agent at named location via `txn.set_ground_location()`
4. Spawn items via `WorldTxn`:
   - `txn.create_item_lot(commodity, quantity)` — creates `ItemLot` entity
   - Place at named location (place or agent) via `txn.set_ground_location()` or `txn.set_possessor()`
5. Spawn facilities via `WorldTxn`:
   - `txn.create_entity(EntityKind::Facility)` + `txn.set_component_workstation_marker()` + `txn.set_ground_location()`
6. Spawn resource sources via `WorldTxn`:
   - `txn.create_entity(EntityKind::Facility)` + `txn.set_component_resource_source()` + `txn.set_ground_location()`
7. Create `RecipeRegistry::new()` (empty — recipes registered separately)
8. Build action registries: `build_full_action_registries(&recipes)`
9. Build dispatch table: `worldwake_systems::dispatch_table()`
10. Create `Scheduler::new_with_tick(Tick(0), SystemManifest::canonical())`
11. Create `ControllerState` — find agent with `ControlSource::Human`, set as controlled
12. Create `DeterministicRng::new(Seed(...))`
13. Create `ReplayState::new(initial_hash, seed, Tick(0), ReplayRecordingConfig::disabled())`
14. Assemble `SimulationState::new(...)` and wrap in `SpawnedSimulation`

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

## Outcome

- **Completion date**: 2026-03-12
- **What changed**: Implemented `spawn_scenario()`, `load_scenario_file()`, `ScenarioError`, and `SpawnedSimulation` in `crates/worldwake-cli/src/scenario/mod.rs`. The function was split into 5 helpers (`build_topology`, `spawn_entities`, `spawn_agent`, `spawn_item`, `assemble_state`) to satisfy clippy's `too_many_lines` lint.
- **Deviations**: `spawn_scenario()` takes `&ScenarioDef` (reference) instead of owned `ScenarioDef` per clippy `needless_pass_by_value`. Dead-code warnings remain for public API items not yet consumed by `main.rs` (E21CLIHUMCON-003 scope).
- **Verification**: 16/16 tests pass, cargo clippy passes with no errors (only expected dead_code warnings for items not yet called from main).
