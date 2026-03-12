# E21CLIHUMCON-007: Inspection Commands (look, inspect, inventory, needs, relations)

## Summary

Implement the state inspection command handlers: `look`, `inspect <entity>`, `inventory [entity]`, `needs [entity]`, `relations <entity>`.

## Depends On

- E21CLIHUMCON-003 (REPL loop)
- E21CLIHUMCON-004 (command enum)
- E21CLIHUMCON-005 (display helpers — entity resolution, formatting)

## Files to Touch

- `crates/worldwake-cli/src/handlers/inspect.rs` — **create**: all inspection handler functions
- `crates/worldwake-cli/src/handlers/mod.rs` — **modify**: wire `Look`, `Inspect`, `Inventory`, `Needs`, `Relations` variants

## Out of Scope

- Other command handlers (006, 008–012)
- Display helper functions (defined in 005)
- World mutation — all handlers are read-only
- Changes to any crate other than `worldwake-cli`

## Deliverables

### `handle_look(sim: &SimulationState)`
- Requires controlled agent (error if observer mode)
- Get agent's current place via placement relation
- Print place name and tags
- List all entities at the same place (agents, items, facilities) with names
- Show travel connections (adjacent places via `TravelEdge`)

### `handle_inspect(sim: &SimulationState, entity_input: &str)`
- Resolve entity via `resolve_entity()`
- Print entity kind and name
- Print all components on the entity (iterate component tables)
- Format each component's Debug output in a readable way

### `handle_inventory(sim: &SimulationState, entity_input: Option<&str>)`
- Default to controlled agent if no argument (error if observer mode and no argument)
- Resolve entity if argument provided
- List all items owned/carried by the entity via ownership relations
- Show `CommodityKind`, `Quantity` for each item lot via `format_quantity()`
- Show total load vs capacity if applicable

### `handle_needs(sim: &SimulationState, entity_input: Option<&str>)`
- Default to controlled agent if no argument
- Resolve entity if argument provided
- Entity must be an agent (error otherwise)
- Show all 5 homeostatic needs (hunger, thirst, fatigue, bladder, dirtiness) via `format_needs_bar()`
- Use agent's `ThresholdBand` for urgency classification (or a default band)

### `handle_relations(sim: &SimulationState, entity_input: &str)`
- Resolve entity
- Query `RelationTables` for all relations involving the entity
- Print each relation: type, related entity name, direction (e.g., "placed at Market Square", "owns 5× Grain")

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-cli`:
  - `test_look_shows_place_name`: look output includes current place name
  - `test_look_shows_colocated_entities`: look lists other entities at same place
  - `test_look_shows_travel_connections`: look shows adjacent places
  - `test_inspect_shows_components`: inspect an agent → output includes AgentData, Name
  - `test_inspect_unknown_entity`: inspect nonexistent entity → error with suggestion
  - `test_inventory_controlled_agent`: inventory with no arg → shows controlled agent's items
  - `test_inventory_named_entity`: inventory with entity name → shows that entity's items
  - `test_inventory_empty`: entity with no items → "no items" message
  - `test_needs_shows_all_five`: needs output includes hunger, thirst, fatigue, bladder, dirtiness
  - `test_needs_non_agent`: needs on a non-agent entity → error
  - `test_relations_shows_placement`: relations shows where entity is placed

### Invariants That Must Remain True
- All handlers are read-only — zero world mutation
- Entity resolution follows the spec protocol (numeric → exact → prefix → error)
- `cargo clippy -p worldwake-cli` passes with no warnings
