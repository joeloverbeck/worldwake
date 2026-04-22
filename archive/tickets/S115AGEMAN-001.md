# S115AGEMAN-001: AgendaProfile component + scenario wiring

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — adds universal `AgendaProfile` ECS component on `EntityKind::Agent`
**Deps**: [specs/S115-agenda-manager.md](../../specs/S115-agenda-manager.md) D6

## Problem

S115 requires per-agent capacity parameters (`pending_capacity`, `suspended_capacity`, `revive_cooldown_ticks`) that the agenda manager consults when bounding memory use and enforcing revival cooldowns. These must be scenario-authorable (so different agent populations can plateau at different rates per FND-22 Agent Diversity) and universally present (so every agent has the substrate). This ticket adds the component before any agenda flow consumes it — keeping every downstream ticket's runtime references compilable without speculative "assume default" fallback paths.

## Assumption Reassessment (2026-04-22)

1. `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs:111-184` is the authoritative scenario-authoring surface for agent components. Field pattern for universal profiles: `#[serde(default)] pub <name>: Option<<Type>>` (e.g., `needs` at line 117, `cognitive_profile` at line 133). Seven universal profiles follow this pattern today.
2. `World::create_agent()` at `crates/worldwake-core/src/world.rs:165` seeds defaults for every universal profile via `world.insert_component_<name>(entity, <Type>::default())?`. Lines 172-206 show the seeding sequence. A new universal component must join this seeding block — omitting it means agents created through non-scenario paths (runtime spawns) lack the profile and `expect()` reads fail.
3. The shared boundary is the ECS component-schema registration macro `with_component_schema_entries!` at `crates/worldwake-core/src/component_schema.rs:3`. The macro generates per-component accessors from a shared descriptor (name, filter, type reference). Per `tickets/README.md` check #13 and `references/worldwake-validation-patterns.md` item 7 (this session's skill-audit follow-up), the component struct MUST live in `worldwake-core` because the macro references types via `crate::TypeName`.
4. No existing component named `AgendaProfile` exists; grep of `worldwake-core` returns zero hits. This is a pure addition, not a rename or migration.
5. Live fallout is slightly broader than the drafted file list: adding `agenda_profile: Option<AgendaProfile>` to shared `AgentDef` requires exhaustive same-crate helper/test literals in CLI modules (`display.rs`, `handlers/*`, `scenario/lints.rs`) to stay buildable. No production scenario `.ron` files needed changes because the field is `#[serde(default)]` and `spawn_agent()` uses `unwrap_or_default()`.

## Architecture Check

1. Defining `AgendaProfile` in `worldwake-core` (alongside `CognitiveProfile`, `ExplorationProfile`, and other universal profiles) matches the established universal-profile pattern. Placing it in a higher crate would prevent schema registration (see Assumption 3). The component is registered on `EntityKind::Agent` only, keeping non-agent entities unaffected.
2. No backwards-compatibility shim: no prior `AgendaProfile` exists, so there is no alias or deprecated path to preserve. Defaults (16/8/4) are embedded in `impl Default` per spec-drafting-rules §5(4).

## Verification Layers

1. Component registration — macro-expansion and delta inventory proof in `component_tables.rs`, `delta.rs`, and the focused `agenda_profile` registration test. The live branch does not have a standalone schema-count test to update.
2. Scenario authoring — `AgentDef` deserialization round-trip test asserting `agenda_profile: Some(AgendaProfile { pending_capacity: 20, .. })` survives RON parse and reaches the spawned agent's component.
3. Bootstrap seeding — `world_txn.rs` create-agent delta assertion updated; `World::create_agent()` unit test reads the component back with `AgendaProfile::default()`.
4. Single-layer ticket — no mixed-layer proof surface needed; this is a pure ECS addition.

## What to Change

### 1. Define `AgendaProfile` in `worldwake-core`

Create `crates/worldwake-core/src/agenda_profile.rs`:

```rust
use crate::traits::Component;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgendaProfile {
    pub pending_capacity: u32,
    pub suspended_capacity: u32,
    pub revive_cooldown_ticks: u32,
}

impl Default for AgendaProfile {
    fn default() -> Self {
        Self { pending_capacity: 16, suspended_capacity: 8, revive_cooldown_ticks: 4 }
    }
}

impl Component for AgendaProfile {}
```

Re-export from `crates/worldwake-core/src/lib.rs` alongside other profile types.

### 2. Register in component_schema.rs

Add a `with_component_schema_entries!` block following the `ExplorationProfile` pattern (agent-only filter, universal-profile accessor naming). Verify the macro expansion sites named in `tickets/README.md` check #13 (`delta.rs`, `world.rs`, `component_tables.rs`) pick up the new component.

### 3. Seed default in `World::create_agent()`

Add `world.insert_component_agenda_profile(entity, AgendaProfile::default())?;` to the seeding block at `crates/worldwake-core/src/world.rs:165`, placing it alongside other universal-profile seeds. Update `world_txn.rs` create_agent delta-count assertion if one exists.

### 4. Add `agenda_profile` field to `AgentDef`

In `crates/worldwake-cli/src/scenario/types.rs`, add:

```rust
#[serde(default)]
pub agenda_profile: Option<AgendaProfile>,
```

alongside the other universal-profile fields (AgendaProfile has no `EntityId` references, so no `*Def` wrapper needed).

### 5. Wire in `spawn_agent()`

In `crates/worldwake-cli/src/scenario/mod.rs`, add:

```rust
txn.set_component_agenda_profile(agent_id, agenda_profile.unwrap_or_default())?;
```

following the pattern for other universal profiles (e.g., near the `HomeostaticNeeds` seeding around line 374).

## Files to Touch

- `crates/worldwake-core/src/agenda_profile.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — re-export)
- `crates/worldwake-core/src/component_schema.rs` (modify — add registration block)
- `crates/worldwake-core/src/world.rs` (modify — `create_agent()` seed)
- `crates/worldwake-core/src/world_txn.rs` (modify — create_agent delta assertion if present)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — `AgentDef` field)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — `spawn_agent()` wiring)
- `crates/worldwake-cli/src/scenario/lints.rs` (modify — exhaustive `AgentDef` helper literal)
- `crates/worldwake-cli/src/display.rs` (modify — exhaustive `AgentDef` test literals)
- `crates/worldwake-cli/src/handlers/actions.rs` (modify — exhaustive `AgentDef` test literals)
- `crates/worldwake-cli/src/handlers/control.rs` (modify — exhaustive `AgentDef` test literals)
- `crates/worldwake-cli/src/handlers/events.rs` (modify — exhaustive `AgentDef` test literals)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify — exhaustive `AgentDef` test literals)
- `crates/worldwake-cli/src/handlers/tick.rs` (modify — exhaustive `AgentDef` test literals)
- `crates/worldwake-cli/src/handlers/world_overview.rs` (modify — exhaustive `AgentDef` test literals)

## Out of Scope

- `AgendaState` / `AgendaEntry` runtime state (ticket 002)
- `tick_agenda` flow consuming capacity/cooldown (ticket 003)
- Actual eviction logic and cooldown enforcement (ticket 003)
- Per-agent profile overrides in existing scenarios (`.ron` files need no change — `unwrap_or_default()` covers them)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core -- agenda_profile` covers Default values (16/8/4), Component bound, bincode round-trip.
2. `cargo test -p worldwake-core` passes with the new component reflected in macro-generated accessors and `delta.rs` sample inventories.
3. Existing suite: `cargo test --workspace` passes unchanged.

### Invariants

1. Every agent created through `World::create_agent()` or `spawn_agent()` has an `AgendaProfile` component. Runtime `get_component_agenda_profile(agent).expect(...)` never panics.
2. `AgendaProfile` is `Copy + Serialize + Deserialize + Eq`; satisfies `Component` trait bounds from `worldwake-core/src/traits.rs:15`.
3. Default values match the spec's Profile-Driven Parameters table (16/8/4).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/agenda_profile.rs` (new) — inline `#[cfg(test)]` block covering Default, Component bounds, bincode round-trip.
2. `crates/worldwake-core/src/world.rs` + `crates/worldwake-core/src/world_txn.rs` — bootstrap/default-seeding and create-agent delta assertions cover the universal-profile contract.
3. `crates/worldwake-cli/src/scenario/types.rs` + `crates/worldwake-cli/src/scenario/mod.rs` — verify an `AgentDef` with an explicit `agenda_profile: Some(AgendaProfile { pending_capacity: 20, suspended_capacity: 4, revive_cooldown_ticks: 2 })` survives RON deserialize → spawn → component-read round-trip.

### Commands

1. `cargo test -p worldwake-core --lib agenda_profile::tests::agenda_profile_default_matches_spec_defaults -- --exact`
2. `cargo test -p worldwake-core --lib agenda_profile::tests::agenda_profile_roundtrips_through_bincode -- --exact`
3. `cargo test -p worldwake-core --lib world::tests::create_agent_attaches_belief_store_perception_profile_and_tell_profile -- --exact`
4. `cargo test -p worldwake-core --lib world_txn::tests::create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through -- --exact`
5. `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_cognitive_profile_missing_new_field_uses_default -- --exact`
6. `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_agent_applies_authored_agenda_profile -- --exact`
7. `cargo test --workspace --no-run`
8. `cargo test -p worldwake-core`
9. `cargo test -p worldwake-cli`
10. `cargo test --workspace`
11. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-22.

- Added `AgendaProfile` in `worldwake-core`, re-exported it, registered it through the shared component schema, seeded it in `World::create_agent()`, and updated `WorldTxn` delta expectations.
- Added optional scenario authoring for `agenda_profile` on `AgentDef` and wired `spawn_agent()` to apply authored values or the default profile.
- Absorbed the real shared-field fallout in CLI helper/test literals that construct `AgentDef` directly so the new scenario field is exhaustive everywhere it is instantiated.

## Deviations

- The live branch did not have a standalone `component_schema.rs` registration-count test to update. Registration proof landed through the new component module test plus the existing macro-expansion and `delta.rs` inventory surfaces.
- The drafted file list was too narrow for the shared `AgentDef` addition. Same-crate CLI helper/test literals in `display.rs`, `handlers/*`, and `scenario/lints.rs` required `agenda_profile: None` for exhaustive construction.

## Verification Result

- Passed `cargo test -p worldwake-core --lib agenda_profile::tests::agenda_profile_default_matches_spec_defaults -- --exact`
- Passed `cargo test -p worldwake-core --lib agenda_profile::tests::agenda_profile_roundtrips_through_bincode -- --exact`
- Passed `cargo test -p worldwake-core --lib world::tests::create_agent_attaches_belief_store_perception_profile_and_tell_profile -- --exact`
- Passed `cargo test -p worldwake-core --lib world_txn::tests::create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through -- --exact`
- Passed `cargo test -p worldwake-cli --lib scenario::types::tests::test_scenario_def_cognitive_profile_missing_new_field_uses_default -- --exact`
- Passed `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_agent_applies_authored_agenda_profile -- --exact`
- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
