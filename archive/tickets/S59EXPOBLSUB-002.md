# S59EXPOBLSUB-002: Component registration for ExpectationStore and LastSeenMemory

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — ECS component schema, delta, world, world_txn, component_tables
**Deps**: S59EXPOBLSUB-001

## Problem

The expectation and last-seen types from ticket 001 need to be registered as ECS components so they can be stored on agents, participate in save/load, and be accessed through the standard component API (`world.component_*`, `txn.set_component_*`).

## Assumption Reassessment (2026-04-06)

1. Component registration uses the `with_component_schema_entries!` macro in `crates/worldwake-core/src/component_schema.rs:3`. Confirmed 2026-04-06.
2. Macro expansion sites that need imports: `delta.rs:19`, `world.rs:20`, `component_tables.rs:11`, `world_txn.rs:6`. Per ticket README item 13.
3. Both components are classified as Universal on `EntityKind::Agent` with `Default` impl.
4. `ExpectationStore` default: empty `BTreeMap`, `next_expectation_id: ExpectationId(0)`.
5. `LastSeenMemory` default: empty `BTreeMap`, `capacity: 20`.
6. Component implementations follow the `impl Component for T` pattern seen in `violation.rs:66`.
7. User-supplied `specs/S59-expectation*` glob resolves to `specs/S59-expectation-obligation-substrate.md`. Safe mechanical correction.
8. Ticket says macro expansion sites only need imports; live code also requires crate-root re-exports in `crates/worldwake-core/src/lib.rs` so downstream crates can name the new components. Safe fallout correction because ticket 001 already introduced the module and this ticket is the first one making the component types workspace-visible.
9. Ticket says registration is sufficient for Universal agent components; live `World::create_agent()` seeds other universal defaults directly (`AgentBeliefStore`, `PerceptionProfile`, `TellProfile`, etc.) in `crates/worldwake-core/src/world.rs:150-175`. To keep the universal-component contract honest for non-scenario agent creation, this ticket must seed `ExpectationStore::default()` and `LastSeenMemory::default()` there as well.
10. Ticket says save/load proof can rely on a version bump; live `crates/worldwake-sim/src/save_load.rs:298-641` already has a focused non-default roundtrip proof for persisted agent state. Safe correction: extend that proof with non-default expectation/last-seen component data instead of treating the version bump as sufficient evidence by itself.

## Architecture Check

1. Universal components with Default follow the established pattern (e.g., `ViolationMemory`, `AgentBeliefStore`). No new architectural patterns introduced.
2. No backward compatibility shims.

## Verification Layers

1. Components registered in schema → compilation success (macro expansion at all sites)
2. Component get/set works → focused unit test via World/WorldTxn API
3. Save/load roundtrip → existing save_load tests pass after SAVE_FORMAT_VERSION bump

## What to Change

### 1. Define component structs

In `crates/worldwake-core/src/expectation.rs` (or the module created in 001), add:

```rust
pub struct ExpectationStore {
    pub records: BTreeMap<ExpectationId, ExpectationRecord>,
    next_expectation_id: ExpectationId,
}

pub struct LastSeenMemory {
    pub records: BTreeMap<EntityId, LastSeenRecord>,
    pub capacity: u16,
}
```

With `Default` impls and `impl Component for ExpectationStore` / `impl Component for LastSeenMemory`.

### 2. Register in component_schema.rs

Add entries to the `with_component_schema_entries!` macro for both components, with entity kind predicate `|kind| kind == EntityKind::Agent`.

### 3. Add imports at macro expansion sites

Add `ExpectationStore` and `LastSeenMemory` to the import lists in:
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-core/src/world_txn.rs`

Also re-export both components from `crates/worldwake-core/src/lib.rs` so downstream crates can use the generated component API with the concrete types.

### 4. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs:6`, increment from 28 to 29.

### 5. Seed default components on agent creation

In `crates/worldwake-core/src/world.rs`, extend `World::create_agent()` to insert `ExpectationStore::default()` and `LastSeenMemory::default()` alongside the other universal default components so ordinary agent creation matches the universal-component contract before scenario wiring lands in ticket 003.

## Files to Touch

- `crates/worldwake-core/src/expectation.rs` (modify — add component structs + Default + Component impls)
- `crates/worldwake-core/src/component_schema.rs` (modify — add macro entries)
- `crates/worldwake-core/src/delta.rs` (modify — add imports)
- `crates/worldwake-core/src/world.rs` (modify — add imports + default component seeding in `create_agent()`)
- `crates/worldwake-core/src/component_tables.rs` (modify — add imports)
- `crates/worldwake-core/src/world_txn.rs` (modify — add imports)
- `crates/worldwake-core/src/lib.rs` (modify — re-export component types)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump SAVE_FORMAT_VERSION)

## Out of Scope

- Scenario integration (AgentDef fields, spawn_agent) — ticket 003
- GoalBeliefView methods — ticket 004
- SystemFn for overdue detection — ticket 006

## Acceptance Criteria

### Tests That Must Pass

1. `world.component_expectation_store(agent)` returns `Some` after `txn.set_component_expectation_store(agent, store)`
2. `world.component_last_seen_memory(agent)` returns `Some` after set
3. Default values are correct (empty records, capacity 20)
4. Agents created through `World::create_agent()` and `WorldTxn::create_agent()` start with default `ExpectationStore` and `LastSeenMemory`
5. Save/load roundtrip preserves non-default component data
6. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Both components registered on `EntityKind::Agent` only
2. `BTreeMap` used for deterministic iteration (no HashMap)
3. SAVE_FORMAT_VERSION bumped to 29

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/expectation.rs` — component defaults, agent default seeding, and component get/set roundtrip tests
2. `crates/worldwake-sim/src/save_load.rs` — persisted non-default expectation/last-seen component roundtrip

### Commands

1. `cargo test -p worldwake-core expectation`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

Completed on 2026-04-06.

- Added `ExpectationStore` and `LastSeenMemory` to `crates/worldwake-core/src/expectation.rs` with `Default`, `Component`, and focused component/default tests.
- Registered both components in the authoritative schema, exported them from `crates/worldwake-core/src/lib.rs`, and wired the generated component API through the core manifest surfaces.
- Updated `World::create_agent()` to seed default expectation and last-seen components so the universal-agent contract is true for ordinary agent creation.
- Extended `crates/worldwake-sim/src/save_load.rs` to serialize and verify non-default expectation and last-seen component data, and bumped `SAVE_FORMAT_VERSION` from 28 to 29.

## Verification Result

- Passed `cargo test -p worldwake-core expectation`
- Passed `cargo test -p worldwake-core`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
