# S102FROAWAEXP-002: AcquisitionExhaustionTracker component + VARIANT_COUNT

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new ECS component (worldwake-core), component_schema registration, create_agent bootstrap
**Deps**: archive/tickets/S102FROAWAEXP-001.md, S102 spec

## Problem

No mechanism exists to track per-need budget exhaustion counts. The exploration gate (ticket 004) needs this stored state to distinguish "planner believes a path exists" from "planner has repeatedly failed to execute any path for this need."

## Assumption Reassessment (2026-04-14)

1. `HomeostaticNeedId` at `crates/worldwake-core/src/needs.rs` still has exactly 5 variants: Hunger, Thirst, Fatigue, Bladder, Dirtiness. There was no existing `VARIANT_COUNT` constant, so the fixed array width remains lawful and explicit.
2. `component_schema.rs` still registers agent-only components through `with_component_schema_entries!`. Adding a new agent component also requires its generated symbols to compile across `delta.rs`, `world.rs`, and `component_tables.rs`.
3. `World::create_agent()` is the canonical bootstrap for universal agent components. Because `world_txn.rs` asserts the exact component delta sequence from `create_agent()`, this ticket also needs the new tracker reflected in that test surface.
4. `S102FROAWAEXP-001` has already landed, so `ExplorationProfile` is now the established adjacent exploration substrate rather than a planned dependency.
5. Existing per-goal exhaustion in `worldwake-ai` remains separate from this work. The new tracker stores authoritative per-need failure counts and does not replace planner retry-backoff state.

## Architecture Check

1. Per-need aggregation as an ECS component is cleaner than adding per-need tracking to the per-goal `exhaustion_cache`, which would conflate retry backoff (per-goal) with exploration triggering (per-need). Separate components keep concerns independent (FND-26).
2. No backward-compatibility shims. New component, new constant — no existing code affected until downstream tickets consume them.

## Verification Layers

1. `AcquisitionExhaustionTracker::default()` is all-zeros → focused unit test
2. `increment` / `reset` / `count` API correctness → focused unit tests
3. Component registered on Agent → focused unit test plus `create_agent()` delta assertion
4. Shared-type plumbing remains coherent through crate, workspace, and CI-equivalent lint verification

## What to Change

### 1. Add HomeostaticNeedId::VARIANT_COUNT

In `crates/worldwake-core/src/needs.rs`, add:

```rust
impl HomeostaticNeedId {
    pub const VARIANT_COUNT: usize = 5;
}
```

### 2. Add AcquisitionExhaustionTracker struct

In `crates/worldwake-core/src/exploration.rs`, add the struct with `counts: [u8; HomeostaticNeedId::VARIANT_COUNT]`, derive block matching spec (Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize), and `increment`, `reset`, `count` methods.

### 3. Register in component_schema.rs

Add `AcquisitionExhaustionTracker` entry in `with_component_schema_entries!` macro, restricted to `EntityKind::Agent`. Follow the ExplorationProfile registration pattern.

### 4. Seed in create_agent()

In `crates/worldwake-core/src/world.rs`, add `world.insert_component_acquisition_exhaustion_tracker(entity, AcquisitionExhaustionTracker::default())?;` alongside other universal component seeding.

### 5. Ensure macro expansion imports

Verify `delta.rs`, `world.rs`, and `component_tables.rs` can resolve `AcquisitionExhaustionTracker`. Add `use` imports where the macro expansion sites require bare type names.

### 6. Re-export from lib.rs

Add `AcquisitionExhaustionTracker` to `crates/worldwake-core/src/lib.rs` public exports.

## Files to Touch

- `crates/worldwake-core/src/needs.rs` (modify — VARIANT_COUNT)
- `crates/worldwake-core/src/exploration.rs` (modify — new struct and tests)
- `crates/worldwake-core/src/component_schema.rs` (modify — registration)
- `crates/worldwake-core/src/world.rs` (modify — create_agent seeding)
- `crates/worldwake-core/src/delta.rs` (modify — macro imports and component coverage)
- `crates/worldwake-core/src/component_tables.rs` (modify — macro import)
- `crates/worldwake-core/src/world_txn.rs` (modify — create_agent delta expectation)
- `crates/worldwake-core/src/lib.rs` (modify — re-export)

## Out of Scope

- GoalBeliefView accessor (ticket 003)
- Planner failure tracking / gate logic (ticket 004)
- Scenario exposure — this is runtime-only, not in AgentDef or RON
- Modifying existing per-goal exhaustion_cache in AgentDecisionRuntime

## Acceptance Criteria

### Tests That Must Pass

1. `AcquisitionExhaustionTracker::default()` has all counts at 0
2. `increment(Hunger)` increments Hunger count, leaves others at 0
3. `reset(Hunger)` resets Hunger to 0 after increment
4. `count(need)` returns correct value for each HomeostaticNeedId variant
5. Saturating behavior: 255 increments don't wrap
6. `World::create_agent()` seeds the tracker as a universal agent component
7. Workspace builds cleanly: `cargo build --workspace`
8. Existing suite: `cargo test --workspace`

### Invariants

1. `AcquisitionExhaustionTracker` derives Copy — must remain a value type
2. Component registered only on `EntityKind::Agent`
3. Every agent created via `create_agent()` gets a default tracker

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/exploration.rs` — unit tests for tracker API, defaulting, saturation, and agent-only registration
2. `crates/worldwake-core/src/needs.rs` — assert `HomeostaticNeedId::VARIANT_COUNT == 5`
3. `crates/worldwake-core/src/world_txn.rs` — assert `create_agent()` emits the tracker component delta

### Commands

1. `cargo test -p worldwake-core --lib exploration::tests::acquisition_exhaustion_tracker_default_is_zeroed -- --exact`
2. `cargo test -p worldwake-core --lib exploration::tests::acquisition_exhaustion_tracker_increment_and_reset_are_need_specific -- --exact`
3. `cargo test -p worldwake-core --lib exploration::tests::acquisition_exhaustion_tracker_saturates_at_u8_max -- --exact`
4. `cargo test -p worldwake-core --lib exploration::tests::acquisition_exhaustion_tracker_registers_for_agents -- --exact`
5. `cargo test -p worldwake-core --lib needs::tests::homeostatic_need_id_variant_count_matches_enum -- --exact`
6. `cargo test -p worldwake-core --lib world_txn::tests::create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through -- --exact`
7. `cargo test -p worldwake-core`
8. `cargo build --workspace`
9. `cargo test --workspace`
10. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed: 2026-04-14

Added `HomeostaticNeedId::VARIANT_COUNT` and the new `AcquisitionExhaustionTracker` value component in `worldwake-core`, with need-specific `count`, `increment`, `reset`, and saturating semantics. The component is now registered as an agent-only ECS component, re-exported from the crate root, seeded automatically by `World::create_agent()`, and reflected in delta/sample coverage so the authoritative schema stays internally consistent.

Deviations from original plan:
- The implementation surface was slightly wider than the initial draft because macro-generated component plumbing and the exact `world_txn` delta assertion both had to be updated for the new shared type to compile and remain provable.
- `component_tables.rs` and `world_txn.rs` became owned fallout even though the initial ticket body did not call them out as primary implementation sites.

## Verification Result

- Passed: `cargo test -p worldwake-core --lib exploration::tests::acquisition_exhaustion_tracker_default_is_zeroed -- --exact`
- Passed: `cargo test -p worldwake-core --lib exploration::tests::acquisition_exhaustion_tracker_increment_and_reset_are_need_specific -- --exact`
- Passed: `cargo test -p worldwake-core --lib exploration::tests::acquisition_exhaustion_tracker_saturates_at_u8_max -- --exact`
- Passed: `cargo test -p worldwake-core --lib exploration::tests::acquisition_exhaustion_tracker_registers_for_agents -- --exact`
- Passed: `cargo test -p worldwake-core --lib needs::tests::homeostatic_need_id_variant_count_matches_enum -- --exact`
- Passed: `cargo test -p worldwake-core --lib world_txn::tests::create_agent_records_entity_component_and_in_transit_deltas_and_supports_read_through -- --exact`
- Passed: `cargo test -p worldwake-core`
- Passed: `cargo build --workspace`
- Passed: `cargo test --workspace`
- Passed: `cargo clippy --workspace --all-targets -- -D warnings`
