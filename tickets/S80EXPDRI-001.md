# S80EXPDRI-001: Core types and component registration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component type, new GoalKind variant, component schema registration
**Deps**: S80 spec (reassessed)

## Problem

No exploration-related types exist in the codebase. All downstream tickets (belief view accessor, dispatch, candidate generation, golden tests) depend on `ExplorationProfile` and `GoalKind::ExploreLocation` existing and being registered in the ECS.

## Assumption Reassessment (2026-04-10)

1. `GoalKind` enum at `crates/worldwake-core/src/goal.rs:17` has 31 variants with derives `Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`. New variant fields (`EntityId`, `HomeostaticNeedId`) are both `Copy` — compatible.
2. `component_schema.rs` at `crates/worldwake-core/src/component_schema.rs` registers agent components with `EntityKind::Agent` filter. Macro expansion sites confirmed at `delta.rs`, `world.rs`, `component_tables.rs`.
3. Shared abstraction boundary: `EntityKind::Agent` component registration pattern (universal profile with Default impl, `set_component_*`/`get_component_*` accessors).
4. `Permille` at `crates/worldwake-core/src/numerics.rs:20` derives `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize`. ExplorationProfile uses `Permille` for two fields — compatible.
5. `HomeostaticNeedId` at `crates/worldwake-core/src/needs.rs:18` derives `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`. Used as field in ExploreLocation variant — compatible.

## Architecture Check

1. ExplorationProfile follows the established universal profile pattern (like `PerceptionProfile`, `CognitiveProfile`): struct with per-agent parameters, Default impl, registered on `EntityKind::Agent`. GoalKind::ExploreLocation follows the established single-place-target goal pattern (like `Patrol { place }`), adding a `motivating_need` field for invalidation semantics.
2. No backward-compatibility shims. These are new types with no legacy paths.

## Verification Layers

1. ExplorationProfile registered as component → focused unit test: insert and retrieve from ECS store
2. GoalKind::ExploreLocation round-trips through serde → focused unit test: serialize/deserialize
3. Default impl produces expected values → focused unit test: check field defaults
4. Single-layer ticket (core type definitions); additional layer mapping not applicable.

## What to Change

### 1. Add ExplorationProfile to worldwake-core

In `crates/worldwake-core/src/` (new module or added to an existing profiles module):

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExplorationProfile {
    pub curiosity_weight: Permille,
    pub need_activation_threshold: Permille,
    pub max_consecutive_explorations: u8,
    pub visit_lookback_ticks: u32,
    pub consecutive_exploration_count: u8,
}
```

Implement `Default` with: `curiosity_weight: Permille(500)`, `need_activation_threshold: Permille(400)`, `max_consecutive_explorations: 3`, `visit_lookback_ticks: 200`, `consecutive_exploration_count: 0`.

Implement `Component` for `ExplorationProfile`.

Export from `crates/worldwake-core/src/lib.rs`.

### 2. Add GoalKind::ExploreLocation

In `crates/worldwake-core/src/goal.rs`, add variant:

```rust
ExploreLocation {
    target_place: EntityId,
    motivating_need: HomeostaticNeedId,
},
```

### 3. Register ExplorationProfile in component schema

In `crates/worldwake-core/src/component_schema.rs`:
- Add entry for `ExplorationProfile` with `EntityKind::Agent` filter
- Add `set_component_exploration_profile` / `get_component_exploration_profile` accessors

Ensure macro expansion sites import the new type:
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/component_tables.rs`

## Files to Touch

- `crates/worldwake-core/src/exploration.rs` (new — ExplorationProfile struct, Default, Component impl)
- `crates/worldwake-core/src/goal.rs` (modify — add ExploreLocation variant)
- `crates/worldwake-core/src/component_schema.rs` (modify — register ExplorationProfile)
- `crates/worldwake-core/src/delta.rs` (modify — import ExplorationProfile for macro expansion)
- `crates/worldwake-core/src/world.rs` (modify — import ExplorationProfile for macro expansion)
- `crates/worldwake-core/src/component_tables.rs` (modify — import ExplorationProfile for macro expansion)
- `crates/worldwake-core/src/lib.rs` (modify — declare and export exploration module)

## Out of Scope

- GoalBeliefView accessor (ticket 002)
- AgentDef / spawn_agent scenario wiring (ticket 002)
- Goal dispatch and planner integration (ticket 003)
- Candidate generation logic (ticket 004)
- Golden E2E tests (ticket 005)
- Systematic cartography or map-building mechanics
- Explicit "scout" or "explorer" role

## Acceptance Criteria

### Tests That Must Pass

1. ExplorationProfile inserts into and retrieves from ECS store correctly
2. ExplorationProfile::default() returns expected field values
3. GoalKind::ExploreLocation serializes and deserializes correctly
4. Existing suite: `cargo test -p worldwake-core`
5. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. GoalKind remains `Copy` — all ExploreLocation fields must be Copy
2. ExplorationProfile is registered on `EntityKind::Agent` only
3. Universal profile contract: Default impl exists, component always present on agents

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/exploration.rs` (or test module) — ExplorationProfile default values, serde round-trip
2. `crates/worldwake-core/src/goal.rs` (test module) — ExploreLocation variant serde round-trip, Copy check

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo build --workspace`
