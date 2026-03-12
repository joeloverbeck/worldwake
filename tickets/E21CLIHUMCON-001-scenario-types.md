# E21CLIHUMCON-001: Scenario Types (RON Structs)

## Summary

Define the `ScenarioDef` struct hierarchy with serde `Deserialize` for RON-based world initialization. These are pure data types with no logic — just the schema for scenario files.

## Depends On

None.

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` — **create**: all scenario definition structs
- `crates/worldwake-cli/src/scenario/mod.rs` — **create**: module declaration, re-exports

## Out of Scope

- Spawning logic (`spawn_scenario()`) — that's E21CLIHUMCON-002
- RON file loading/parsing — that's E21CLIHUMCON-002
- Default scenario file — that's E21CLIHUMCON-013
- Any changes to `worldwake-core`, `worldwake-sim`, `worldwake-systems`, or `worldwake-ai`
- CLI args, REPL, or any command handling

## Deliverables

### `ScenarioDef` (top-level)
```rust
pub struct ScenarioDef {
    pub seed: u64,
    pub places: Vec<PlaceDef>,
    pub edges: Vec<EdgeDef>,
    pub agents: Vec<AgentDef>,
    pub items: Vec<ItemDef>,
    pub facilities: Vec<FacilityDef>,
    pub resource_sources: Vec<ResourceSourceDef>,
}
```

### Sub-structs
- `PlaceDef` — name, tags (`Vec<PlaceTag>`)
- `EdgeDef` — from place name, to place name, travel ticks, bidirectional flag
- `AgentDef` — name, location (place name), control source (`"Human"` / `"Ai"` / `"None"`), optional needs overrides, optional combat profile, optional utility profile, optional merchandise profile, optional trade disposition profile
- `ItemDef` — commodity kind, quantity, location (place name or agent name), optional container flag
- `FacilityDef` — workstation tag, location (place name)
- `ResourceSourceDef` — commodity kind, location (place name), regeneration rate, capacity

### Design Notes
- All location references use **string names** (resolved to `EntityId` during spawning in 002)
- Control source as a string enum (`"Human"`, `"Ai"`, `"None"`) for RON readability
- Use `#[serde(default)]` for optional fields where sensible defaults exist
- All structs derive `Debug, Clone, serde::Deserialize`
- No `Serialize` needed (scenario files are read-only input)

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-cli` — unit tests in `scenario/types.rs`:
  - `test_scenario_def_deserialize_minimal`: a minimal RON string with 1 place, 0 edges, 1 agent deserializes correctly
  - `test_scenario_def_deserialize_full`: a RON string with all fields populated deserializes correctly
  - `test_agent_def_default_optional_fields`: agent with only name/location/control deserializes, optional fields are `None`
  - `test_edge_def_bidirectional_default`: edge without explicit bidirectional field defaults to `true`

### Invariants That Must Remain True
- All types are `Deserialize` — RON parsing works for valid input
- No dependency on runtime state (pure data types)
- `cargo clippy -p worldwake-cli` passes with no warnings
