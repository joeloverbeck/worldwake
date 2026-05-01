# S130SURRECFRO-004: SurveyMemory registration + GoalBeliefView accessor

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `SurveyMemory` ECS registration, `create_agent`/`spawn_agent` default insertion, `GoalBeliefView::survey_memory()` accessor, `SAVE_FORMAT_VERSION` bump
**Deps**: 002, spec `specs/S130-survey-records-frontier-disconfirmation.md` D4, D8

## Problem

`SurveyMemory` (defined in ticket 002) needs to be wired through the ECS storage stack and the AI-layer belief-view read surface so downstream tickets (006 ranking, 007 perception, 008 decay) can read and write it. This ticket registers the component with the schema macro, seeds defaults at agent creation, and adds the `GoalBeliefView::survey_memory()` accessor in `worldwake-sim`. It also bumps `SAVE_FORMAT_VERSION` again since `SurveyMemory` joins the saved-agent-state surface.

## Assumption Reassessment (2026-05-02)

1. `with_component_schema_entries!` macro at `crates/worldwake-core/src/component_schema.rs:3-6` generates the storage table, `World` accessors (`get_component_*`, `insert_component_*`, etc.), `WorldTxn` accessors (`set_component_*`, `clear_component_*`), and query helpers. The `WoundList` registration at `component_schema.rs:82-106` is the canonical universal-on-Agent precedent (predicate `|kind| kind == EntityKind::Agent`).
2. `World::create_agent` at `crates/worldwake-core/src/world.rs:183-231` seeds 19 universal Agent components via `insert_component_*(entity, Default::default())?` — `SurveyMemory::default()` insertion follows the same pattern. The test `create_agent_attaches_belief_store_perception_profile_and_tell_profile` at `world.rs:1308` may need an additional assertion for `SurveyMemory` presence after the new insertion lands.
3. `spawn_agent` at `crates/worldwake-cli/src/scenario/mod.rs` mirrors `create_agent` for scenario-loaded agents; spec D4 mandates universal `SurveyMemory::default()` insertion there too. No `AgentDef` field — `SurveyMemory` is runtime-generated state per spec-drafting-rules.md Section 5 (analogous to `WoundList`).
4. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs` is the standard belief-view-mediated read surface for the AI crate. Sibling memory accessors (`discrepancy_memory`, `blocker_memory`, `repair_memory`, `learned_opportunity_memory`) at `belief_view.rs:301-313` follow the pattern `fn <name>(&self, agent: EntityId) -> Option<&<Type>>`. The `RuntimeBeliefView` impl at `belief_view.rs:1084-1096` forwards each via `world.get_component_*(agent)`. `impl_goal_belief_view!` macro / blanket impl propagates the accessor.
5. `SAVE_FORMAT_VERSION` at `crates/worldwake-sim/src/save_load.rs:6` is now `59` after ticket 001's persisted profile-field additions. Registering a new universal Agent component (`SurveyMemory`) modifies saved agent component state, so this ticket owns the next bump from `59` to `60`.
6. The macro-generated identifiers expected by ticket 008 (decay) and tickets 006/007 (read sites) are `get_component_survey_memory`, `set_component_survey_memory`, `entities_with_survey_memory`, `query_survey_memory` (mirroring the WoundList macro entry).

## Architecture Check

1. Co-locating ECS registration and the belief-view accessor in one ticket is justified because the accessor (in `worldwake-sim`) directly forwards to the `worldwake-core` `get_component_survey_memory` accessor that the registration generates. Splitting them would create an intermediate state where `worldwake-sim` cannot compile (the accessor body refers to a function that doesn't exist).
2. Universal-on-Agent classification — every agent always has a `SurveyMemory` component (default empty) so ranking damping reads and perception writes can always assume the component is present (agnostic to whether the agent has surveyed anything yet). Aligns with FND-22A: explicit-state acquisition starts at empty, not absent.
3. Ticket 001 already bumped `SAVE_FORMAT_VERSION` for profile-field persisted shape. This ticket still needs a new bump because registering `SurveyMemory` changes saved agent component state.
4. No backward-compat shim — the registration is net-new; older saves without `SurveyMemory` components are post-bump-incompatible. Per FND-28, save migration is a boundary concern (out of scope for the live authority path).

## Verification Layers

1. `SurveyMemory` is universally present on agents → focused unit test extending `create_agent_attaches_belief_store_perception_profile_and_tell_profile` (or sibling) to assert `world.get_component_survey_memory(agent)` returns `Some(&SurveyMemory::default())` after `create_agent`.
2. Scenario-spawned agents also receive `SurveyMemory::default()` → focused unit test in `scenario/mod.rs` `#[cfg(test)]` block, or extension of an existing `spawn_agent` smoke test.
3. `GoalBeliefView::survey_memory()` returns `Some(&SurveyMemory)` for known agents → focused unit test on `RuntimeBeliefView` constructed over a world with a known agent.
4. `GoalBeliefView::survey_memory()` default impl returns `None` → focused unit test on a minimal mock that doesn't override the default.
5. `SAVE_FORMAT_VERSION` is `60` and round-trips → existing `save_load` test updated to assert the new value.
6. Single-cross-system layer (component schema + read accessor) — no decision-trace, action-trace, or event-log emission in this ticket.

## What to Change

### 1. Component schema registration

In `crates/worldwake-core/src/component_schema.rs`, add a `SurveyMemory` entry to the `with_component_schema_entries!` macro invocation, mirroring the `WoundList` entry at `component_schema.rs:82-106`. Predicate is `|kind| kind == EntityKind::Agent`. This generates `get_component_survey_memory`, `set_component_survey_memory`, `entities_with_survey_memory`, and the standard query/iter accessors.

### 2. `create_agent` default insertion

In `crates/worldwake-core/src/world.rs`, add to the `create_agent` body (around line 207, alongside other universal insertions):

```rust
world.insert_component_survey_memory(entity, SurveyMemory::default())?;
```

Update the existing test `create_agent_attaches_belief_store_perception_profile_and_tell_profile` at `world.rs:1308` (or a sibling test) to assert `SurveyMemory::default()` presence after `create_agent`.

### 3. `spawn_agent` default insertion

In `crates/worldwake-cli/src/scenario/mod.rs::spawn_agent`, insert `SurveyMemory::default()` for every spawned agent via `txn.set_component_survey_memory(agent_id, SurveyMemory::default())?;` (universal — no `AgentDef` field; runtime-generated state).

### 4. `GoalBeliefView::survey_memory()` accessor

In `crates/worldwake-sim/src/belief_view.rs`, add to the `GoalBeliefView` trait alongside existing memory accessors (around line 301-313):

```rust
fn survey_memory(&self, agent: EntityId) -> Option<&SurveyMemory> {
    None
}
```

Add to the `RuntimeBeliefView` impl (around line 1084-1096):

```rust
fn survey_memory(&self, agent: EntityId) -> Option<&SurveyMemory> {
    self.world.get_component_survey_memory(agent)
}
```

Add the new method to the `impl_goal_belief_view!` macro / blanket impl forwarding so derived view backends pick it up automatically.

### 5. `SAVE_FORMAT_VERSION` bump

In `crates/worldwake-sim/src/save_load.rs`, change `pub const SAVE_FORMAT_VERSION: u32 = 59;` to `60`. Update the matching `SAVE_FORMAT_VERSION` assertion to `60`. Verify the version-mismatch test still produces a meaningful failure under the new constant.

## Files to Touch

- `crates/worldwake-core/src/component_schema.rs` (modify — macro entry)
- `crates/worldwake-core/src/world.rs` (modify — `create_agent` insertion + test assertion)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — `spawn_agent` insertion)
- `crates/worldwake-sim/src/belief_view.rs` (modify — trait method + impls)
- `crates/worldwake-sim/src/save_load.rs` (modify — `SAVE_FORMAT_VERSION` bump + assertion)

## Out of Scope

- Calling `SurveyMemory::record` from perception (ticket 007)
- Reading `SurveyMemory` for ranking damping (ticket 006)
- Calling `SurveyMemory::enforce_limits` from `evidence_decay_system` (ticket 008)
- `AgentDef` field for `SurveyMemory` — explicitly rejected by spec D4 (FND-22-exempt runtime-generated state, analogous to `WoundList`)
- Save-format migration tooling — boundary concern, out of scope per FND-28

## Acceptance Criteria

### Tests That Must Pass

1. New or extended: `create_agent_attaches_survey_memory` — asserts `world.get_component_survey_memory(agent)` returns `Some(&SurveyMemory::default())` after `create_agent`.
2. New: `spawn_agent_attaches_survey_memory` — asserts scenario-spawned agents receive `SurveyMemory::default()`.
3. New: `runtime_belief_view_survey_memory_returns_component` — asserts `RuntimeBeliefView::survey_memory(agent)` forwards to `world.get_component_survey_memory`.
4. New: `goal_belief_view_default_impl_survey_memory_returns_none` — asserts the default trait method returns `None` on a minimal mock.
5. Existing: `cargo test -p worldwake-sim save_load` — version-bump assertion updated to `60`.
6. Existing suite: `cargo test --workspace`.

### Invariants

1. Every agent created via `World::create_agent` or `spawn_agent` has `SurveyMemory` present (default empty) — ranking and perception code can `expect()` the component on known agents.
2. `SAVE_FORMAT_VERSION` increments for this registration from the ticket-001 baseline (`59→60`) — ticket 002 deliberately did not bump because the type definitions alone do not change save shape until registration lands.
3. `GoalBeliefView::survey_memory()` is the only AI-layer read path for `SurveyMemory` — perception writes go through `world.get_component_survey_memory_mut` / `txn.set_component_survey_memory` directly (no AI-layer write path).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` (`#[cfg(test)]` block) — extend `create_agent_attaches_belief_store_perception_profile_and_tell_profile` or add `create_agent_attaches_survey_memory`.
2. `crates/worldwake-cli/src/scenario/mod.rs` (`#[cfg(test)]` block) — 1 new unit test: `spawn_agent_attaches_survey_memory`.
3. `crates/worldwake-sim/src/belief_view.rs` (`#[cfg(test)]` block) — 2 new unit tests: `runtime_belief_view_survey_memory_returns_component` and `goal_belief_view_default_impl_survey_memory_returns_none`.
4. `crates/worldwake-sim/src/save_load.rs` (`#[cfg(test)]` block) — existing `SAVE_FORMAT_VERSION` assertion updated to `60`.

### Commands

1. `cargo test -p worldwake-core world::tests::create_agent`
2. `cargo test -p worldwake-cli scenario::tests::spawn_agent`
3. `cargo test -p worldwake-sim belief_view::tests`
4. `cargo test -p worldwake-sim save_load::tests`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

Merge note: ticket 001 already bumped `SAVE_FORMAT_VERSION 58→59` for persisted profile-field additions. This ticket bumps `59→60` for `SurveyMemory` registration. Sibling runtime mutations from ticket 007 and `enforce_limits` calls from ticket 008 are changes to an already-registered component, not new schema changes.
