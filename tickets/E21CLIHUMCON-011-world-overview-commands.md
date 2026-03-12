# E21CLIHUMCON-011: World Overview Commands (world, places, agents, goods)

## Summary

Implement the world overview commands for global state inspection: `world` (summary), `places` (with connections), `agents` (with locations), `goods` (commodity totals).

## Depends On

- E21CLIHUMCON-003 (REPL loop)
- E21CLIHUMCON-004 (command enum)
- E21CLIHUMCON-005 (display helpers)

## Files to Touch

- `crates/worldwake-cli/src/handlers/world_overview.rs` — **create**: `handle_world()`, `handle_places()`, `handle_agents()`, `handle_goods()`
- `crates/worldwake-cli/src/handlers/mod.rs` — **modify**: wire `World`, `Places`, `Agents`, `Goods` variants

## Out of Scope

- Other command handlers (006–010, 012)
- Modifying any core types or systems
- Changes to any crate other than `worldwake-cli`
- Per-agent belief views (E14 — not implemented yet)

## Deliverables

### `handle_world(sim: &SimulationState)`
Summary of all places with population:
```
=== World Overview (tick 15) ===
  Market Square: 3 agents, 12 items
  Forest Clearing: 1 agent, 5 items
  Mountain Pass: 0 agents, 0 items
```
- Iterate all places in topology
- Count agents and item lots at each place via placement relations

### `handle_places(sim: &SimulationState)`
List places with travel connections:
```
Places:
  Market Square [market, settlement]
    → Forest Clearing (3 ticks)
    → Mountain Pass (5 ticks)
  Forest Clearing [forest]
    → Market Square (3 ticks)
```
- Show place name and tags
- For each place, list outgoing `TravelEdge` connections with travel time

### `handle_agents(sim: &SimulationState)`
List all living agents:
```
Agents:
  Kael [human] at Market Square — idle
  Merchant Vara [ai] at Market Square — trading (2 ticks left)
  Guard Theron [ai] at Forest Clearing — patrolling
```
- Show name, control source, location, current action (if any) with remaining ticks
- Only show living agents (filter by allocator liveness)

### `handle_goods(sim: &SimulationState)`
Global goods summary:
```
Goods:
  Grain: 45 total (Market Square: 30, Forest Clearing: 15)
  Water: 20 total (Market Square: 20)
  Iron Ore: 8 total (Mountain Pass: 8)
```
- Iterate all `ItemLot` entities
- Aggregate `Quantity` by `CommodityKind`
- Sub-aggregate by location (place)
- Use `BTreeMap` for deterministic ordering

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-cli`:
  - `test_world_shows_all_places`: world lists every place in topology
  - `test_world_shows_population`: world shows correct agent count per place
  - `test_places_shows_connections`: places shows travel edges with tick durations
  - `test_places_shows_tags`: places shows place tags
  - `test_agents_lists_all_living`: agents lists all alive agents
  - `test_agents_shows_location`: each agent shows their current place
  - `test_agents_shows_control_source`: each agent shows [human]/[ai]/[none]
  - `test_goods_aggregates_by_commodity`: goods totals match actual item quantities
  - `test_goods_empty_world`: world with no items → "no goods" message

### Invariants That Must Remain True
- All handlers are read-only — zero world mutation
- Deterministic output ordering (BTreeMap-based iteration)
- Counts match actual entity/component state (no caching that could go stale)
- `cargo clippy -p worldwake-cli` passes with no warnings
