# S107PRODIV-001: Core types — DiversificationProfile, PlaceVisitRecord, ExplorationMotivation, LastProactiveExplorationTick

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new types and component registrations in worldwake-core
**Deps**: None

## Problem

S107 introduces proactive diversification exploration requiring four new types in worldwake-core: a per-agent profile component, a visit-tracking record stored in the belief store, an enum to distinguish exploration motivations, and a runtime tick-tracking component. These are foundational types that all subsequent S107 tickets depend on.

## Assumption Reassessment (2026-04-17)

1. `DiversificationProfile` does not exist in the codebase — confirmed via grep. `ExplorationMotivation` does not exist. `PlaceVisitRecord` does not exist. `LastProactiveExplorationTick` does not exist.
2. `component_schema.rs` uses `with_component_schema_entries!` macro for registration. Currently 51 Agent components. Pattern: `(TypeName, type_snake, |kind| kind == EntityKind::Agent)`.
3. `GoalKind` in `crates/worldwake-core/src/goal.rs:115` currently has `ExploreLocation { target_place: EntityId, motivating_need: HomeostaticNeedId }`. `GoalKind` derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`.
4. `Permille` derives `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize` — all field types in DiversificationProfile satisfy these bounds.
5. `Tick` is `pub struct Tick(pub u64)` — derives the same traits. `PlaceVisitRecord` fields (`u32`, `Tick`, `u16`) all satisfy Copy/Serialize/Ord bounds.
6. Macro expansion sites for component registration: `delta.rs`, `world.rs`, `component_tables.rs` — new types must be imported at each site.
7. Because `PlaceVisitRecord` lands on serialized `AgentBeliefStore`, the live owned surface also includes omitted-field serde compatibility plus `BeliefStoreDiff`/manual full-literal fallout for `AgentBeliefStore` across `worldwake-core`; CI-matching clippy also exposes one downstream full literal in `worldwake-ai/src/candidate_generation.rs`.

## Architecture Check

1. Four small, focused types with clear responsibilities. DiversificationProfile is role-specific (agents without it behave exactly as today). PlaceVisitRecord is concrete state (FND-3), not a derived score. ExplorationMotivation cleanly separates need-driven from proactive exploration drives.
2. No backward-compatibility shims. ExplorationMotivation replaces HomeostaticNeedId in ExploreLocation — the migration happens in ticket 002.

## Verification Layers

1. `DiversificationProfile` component registration → focused unit test: insert/get/has round-trip on Agent entity
2. `LastProactiveExplorationTick` component registration → focused unit test: insert/get round-trip
3. `ExplorationMotivation` derive compatibility → compilation (GoalKind derives Copy, ExplorationMotivation must too)
4. Single-layer ticket: types and registration only, no behavioral changes

## What to Change

### 1. Add ExplorationMotivation enum

In `crates/worldwake-core/src/goal.rs`, add:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ExplorationMotivation {
    NeedDriven(HomeostaticNeedId),
    Proactive,
}
```

Do NOT change `ExploreLocation.motivating_need` type yet — that is ticket 002.

### 2. Add DiversificationProfile

New file `crates/worldwake-core/src/diversification.rs` with the struct, Default impl, and `impl Component for DiversificationProfile {}`.

### 3. Add PlaceVisitRecord

In `crates/worldwake-core/src/belief.rs`, add the `PlaceVisitRecord` struct near the top (it's a belief-layer type, not a component), add `place_visits` to `AgentBeliefStore` with omitted-field serde compatibility, and extend `BeliefStoreDiff` plus manual full-literal fixtures for the new persisted field.

### 4. Add LastProactiveExplorationTick

In `crates/worldwake-core/src/diversification.rs` alongside DiversificationProfile:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct LastProactiveExplorationTick(pub Option<Tick>);
impl Component for LastProactiveExplorationTick {}
```

### 5. Register components in component_schema.rs

Add `DiversificationProfile` and `LastProactiveExplorationTick` entries to `with_component_schema_entries!` macro, both guarded by `|kind| kind == EntityKind::Agent`.

### 6. Export from crate root

Add `pub mod diversification;` to `crates/worldwake-core/src/lib.rs` and re-export types. Ensure imports at macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`).

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify) — add ExplorationMotivation enum
- `crates/worldwake-core/src/diversification.rs` (new) — DiversificationProfile, LastProactiveExplorationTick
- `crates/worldwake-core/src/belief.rs` (modify) — add PlaceVisitRecord struct
- `crates/worldwake-core/src/component_schema.rs` (modify) — register 2 new components
- `crates/worldwake-core/src/lib.rs` (modify) — add module + re-exports
- `crates/worldwake-core/src/delta.rs` (modify) — import new types for macro expansion
- `crates/worldwake-core/src/world.rs` (modify) — import new types for macro expansion
- `crates/worldwake-core/src/component_tables.rs` (modify) — import new types for macro expansion
- `crates/worldwake-ai/src/candidate_generation.rs` (modify) — shared `AgentBeliefStore` full-literal fallout from the new `place_visits` field

## Out of Scope

- Changing ExploreLocation.motivating_need type (ticket 002)
- GoalBeliefView accessors (ticket 003)
- GoalBeliefView-side diversification accessors and downstream AI read wiring for `place_visits` (ticket 003)
- CLI/scenario wiring (ticket 005)
- Any behavioral logic

## Acceptance Criteria

### Tests That Must Pass

1. Component round-trip: insert DiversificationProfile on Agent, get it back, verify fields match
2. Component round-trip: insert LastProactiveExplorationTick on Agent, get it back
3. DiversificationProfile::default() produces expected values (base_curiosity=400, comfort_threshold=450, etc.)
4. ExplorationMotivation derives Copy and can be used in GoalKind (compilation test)
5. Existing suite: `cargo test -p worldwake-core`
6. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. DiversificationProfile is role-specific — no seeding in `World::create_agent()`
2. LastProactiveExplorationTick is runtime-generated — no seeding in `World::create_agent()`
3. All new types derive Serialize/Deserialize for save/load compatibility
4. All new types satisfy GoalKind's Copy bound (for ExplorationMotivation)

## Outcome

Completed on 2026-04-17.

- Added `DiversificationProfile` and `LastProactiveExplorationTick` in new `crates/worldwake-core/src/diversification.rs`, exported them from `worldwake-core`, and registered both as Agent-only components without `World::create_agent()` default seeding.
- Added `ExplorationMotivation` to `goal.rs` without changing `GoalKind::ExploreLocation` yet, preserving ticket 002 ownership.
- Added `PlaceVisitRecord` plus `AgentBeliefStore.place_visits` in `belief.rs` with `#[serde(default)]` omitted-field compatibility, `BeliefStoreDiff` support, and focused diff coverage.
- Absorbed the real shared-field fallout in `delta.rs`, `component_tables.rs`, `world.rs`, and one downstream `worldwake-ai` full `AgentBeliefStore` literal in `candidate_generation.rs` exposed by CI-matching clippy.
- Archival handoff corrected `Out of Scope` to reflect that `place_visits` storage landed here while ticket 003 now owns only GoalBeliefView-side read wiring.

## Verification Result

- Passed `cargo test -p worldwake-core --lib diversification::tests::diversification_profile_default_matches_spec_defaults -- --exact`
- Passed `cargo test -p worldwake-core --lib diversification::tests::last_proactive_exploration_tick_registers_for_agents_without_default_seeding -- --exact`
- Passed `cargo test -p worldwake-core --lib goal::tests::exploration_motivation_roundtrips_through_bincode -- --exact`
- Passed `cargo test -p worldwake-core --lib belief::tests::belief_store_diff_roundtrip_place_visits -- --exact`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-ai --lib --no-run`
- Passed `cargo build --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/diversification.rs` — unit tests for Default impl, component registration round-trips
2. `crates/worldwake-core/src/goal.rs` — compilation test that ExplorationMotivation satisfies GoalKind derive bounds
3. `crates/worldwake-core/src/belief.rs` — focused diff coverage for `place_visits` on `AgentBeliefStore`

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo build --workspace`
