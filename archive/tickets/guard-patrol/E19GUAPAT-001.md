# E19GUAPAT-001: Add PatrolRoute and PatrolProfile components to worldwake-core

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component types, component schema registration
**Deps**: E16 (offices), E17 (violation memory) — both delivered

## Problem

Guards need per-agent patrol configuration (route waypoints, vigilance, sensitivity) and persistent patrol progress. No patrol domain types exist yet in the codebase.

## Assumption Reassessment (2026-03-30)

1. `crates/worldwake-core/src/patrol.rs` does not exist — confirmed by glob search.
2. The exact shared abstraction boundary under audit is the authoritative component manifest in [`crates/worldwake-core/src/component_schema.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_schema.rs). Adding a component there fans out into `ComponentTables`, `World`, `WorldTxn`, and `ComponentKind`/`ComponentValue` in `delta.rs`; this ticket must cover that whole manifest projection, not only raw storage.
3. `ComponentTables` in [`crates/worldwake-core/src/component_tables.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_tables.rs) is still generated from the manifest into `BTreeMap<EntityId, T>` fields and methods, so the determinism assumption is correct.
4. `World` component APIs in [`crates/worldwake-core/src/world.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs) are generated from the same manifest and already provide the focused proof surface used by other core component tickets: round-trip insertion, query/count visibility, and kind rejection.
5. `WorldTxn` simple set/clear APIs in [`crates/worldwake-core/src/world_txn.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs) are also generated from the manifest for `txn_simple_set` components, and existing per-component tests assert emitted `ComponentDelta` values. Deferring patrol `WorldTxn` coverage to a later ticket would leave the core authoritative mutation surface under-specified.
6. `ComponentKind` and `ComponentValue` in [`crates/worldwake-core/src/delta.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/delta.rs) are manifest-derived. Adding patrol components changes the event-log delta vocabulary inside `worldwake-core` even though no new action handler ships in this ticket.
7. `Permille` exists in `crates/worldwake-core/src/numerics.rs` and remains the correct type for vigilance, adaptation sensitivity, and motive-weight fields.
8. `EntityKind::Agent` remains the correct kind predicate for both patrol components, consistent with nearby per-agent config/state components such as `CombatProfile`, `UtilityProfile`, and `ViolationMemory`.
9. `ViolationMemory` already exists in [`crates/worldwake-core/src/violation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/violation.rs); this ticket does not couple patrol data to violation storage and should keep the two concerns separate.
10. Mismatch corrected: the original ticket said `WorldTxn` integration could be deferred until the patrol action ticket. In the live architecture, once a component is in the authoritative manifest and marked `txn_simple_set`, its `WorldTxn` and delta surfaces are part of the same core contract and belong in scope now.
11. No adjacent architectural contradiction was found that requires broadening beyond the core patrol data contract.

## Architecture Check

1. Two separate components are still the cleaner architecture. `PatrolRoute` is authoritative mutable world state that patrol actions and future reassignment systems will mutate; `PatrolProfile` is durable per-agent configuration that shapes patrol cadence and urgency without entangling it with route progress.
2. Registering patrol data through the existing authoritative component manifest is better than introducing patrol-specific side tables or AI-runtime-only storage. Patrol route progress is world state, not planner scratch data, and future action/event-log mutations should use the same typed component pathway as the rest of the engine.
3. No backwards-compatibility aliasing or shims. New types enter the canonical manifest directly and all generated core surfaces update in place.

## Verification Layers

1. Patrol component value semantics and serialization -> focused unit tests in `patrol.rs`
2. Authoritative world API registration, query/count visibility, and kind enforcement -> focused runtime tests in `world.rs`
3. Authoritative mutation and event-log delta vocabulary for patrol components -> focused runtime tests in `world_txn.rs`
4. Single-crate ticket: all proofs stay inside `worldwake-core`; no AI/system/golden mapping is required yet because no patrol behavior ships here.

## What to Change

### 1. New file: `crates/worldwake-core/src/patrol.rs`

Define two structs:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatrolRoute {
    pub assigned_places: Vec<EntityId>,
    pub current_index: usize,
}
impl Component for PatrolRoute {}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatrolProfile {
    pub base_patrol_interval: u32,
    pub vigilance: Permille,
    pub route_adaptation_sensitivity: Permille,
    pub patrol_motive_weight: Permille,
}
impl Component for PatrolProfile {}
```

### 2. Register components in `component_schema.rs`

Add two new entries to `with_component_schema_entries!` following the existing pattern. Kind predicate: `|kind| kind == EntityKind::Agent`.

These entries must participate in the standard manifest fan-out:
- typed storage in `ComponentTables`
- authoritative read APIs in `World`
- `ComponentKind` / `ComponentValue` support in `delta.rs`
- `WorldTxn` simple set/clear projection via `txn_simple_set`

### 3. Wire generated core surfaces

Update the `use` surfaces and generated projections that depend on the manifest:
- `component_tables.rs`
- `world.rs`
- `world_txn.rs`
- `delta.rs`

No bespoke patrol side-channel APIs. Patrol data must flow only through the existing authoritative component machinery.

### 4. Declare module and re-exports in `lib.rs`

Add `pub mod patrol;` and re-export `PatrolRoute`, `PatrolProfile`.

## Files to Touch

- `crates/worldwake-core/src/patrol.rs` (new)
- `crates/worldwake-core/src/component_schema.rs` (modify — add 2 entries)
- `crates/worldwake-core/src/component_tables.rs` (modify — add patrol imports via manifest usage)
- `crates/worldwake-core/src/delta.rs` (modify — add patrol imports via manifest usage)
- `crates/worldwake-core/src/lib.rs` (modify — add module declaration and re-exports)
- `crates/worldwake-core/src/world.rs` (modify — add patrol imports via manifest-generated API usage)
- `crates/worldwake-core/src/world_txn.rs` (modify — add patrol imports via manifest-generated setter/delta usage)

## Out of Scope

- Patrol action definition or handler (E19GUAPAT-003)
- GoalKind::Patrol or PlannerOpKind::Patrol (E19GUAPAT-002)
- Route adaptation logic (E19GUAPAT-006)
- Any AI candidate generation
- Any changes to worldwake-systems, worldwake-sim, or worldwake-ai crates
- Any patrol-specific behavior layer beyond the core authoritative data contract

## Acceptance Criteria

### Tests That Must Pass

1. `PatrolRoute` and `PatrolProfile` satisfy the same trait/serialization bounds as neighboring core component types.
2. `PatrolRoute` bincode round-trip preserves `assigned_places` and `current_index`.
3. `PatrolProfile` bincode round-trip preserves all four fields.
4. `World` round-trip tests prove insert/get/query/count/remove behavior for both patrol components on `EntityKind::Agent`.
5. `World` rejects both patrol components on at least one non-agent entity kind.
6. `WorldTxn` set/clear tests prove patrol components emit the correct `ComponentDelta` variants and update authoritative world state on commit.
7. Existing suite: `cargo test -p worldwake-core`
8. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Components use `BTreeMap` storage (determinism invariant — no `HashMap`)
2. `Permille` used for all [0,1000] range values (spec drafting rule)
3. No `f32`/`f64` in any field (determinism invariant)
4. `current_index` is authoritative stored state, not derived
5. Patrol component mutation uses the canonical authoritative component/delta path; no parallel patrol-specific mutation mechanism is introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/patrol.rs` — new unit tests for trait bounds and bincode round-trips, proving the raw component value contract.
2. `crates/worldwake-core/src/world.rs` — new focused tests for patrol route/profile round-trips and non-agent rejection, proving the generated authoritative world API.
3. `crates/worldwake-core/src/world_txn.rs` — new focused tests for patrol route/profile set/clear delta emission and commit behavior, proving the canonical mutation surface.

### Commands

1. `cargo test -p worldwake-core patrol::tests`
2. `cargo test -p worldwake-core patrol_route`
3. `cargo test -p worldwake-core patrol_profile`
4. `cargo test -p worldwake-core set_component_patrol`
5. `cargo test -p worldwake-core clear_component_patrol`
6. `cargo test -p worldwake-core`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-30
- What actually changed:
  - Added `PatrolRoute` and `PatrolProfile` in `crates/worldwake-core/src/patrol.rs`.
  - Registered both components in the authoritative component manifest so they now project through `ComponentTables`, `World`, `WorldTxn`, and `ComponentKind`/`ComponentValue`.
  - Re-exported both types from `worldwake-core`.
  - Added focused tests for raw component value bounds/serialization, `World` round-trips and kind rejection, `WorldTxn` delta emission and commit behavior, plus direct `ComponentTables` storage coverage.
- Deviations from original plan:
  - The ticket was corrected before implementation because the original scope understated the real core boundary. In the live architecture, manifest registration necessarily includes `delta.rs`, `world.rs`, and `world_txn.rs`; leaving those as an implicit later concern would have been weaker than the existing component architecture.
  - `ComponentTables` focused tests were added as extra proof even though the ticket’s minimum acceptance criteria only required `World` and `WorldTxn`.
- Verification results:
  - `cargo fmt --all` ✅
  - `cargo test -p worldwake-core patrol` ✅
  - `cargo test -p worldwake-core` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
