# S102FROAWAEXP-002: AcquisitionExhaustionTracker component + VARIANT_COUNT

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new ECS component (worldwake-core), component_schema registration, create_agent bootstrap
**Deps**: S102 spec

## Problem

No mechanism exists to track per-need budget exhaustion counts. The exploration gate (ticket 004) needs this stored state to distinguish "planner believes a path exists" from "planner has repeatedly failed to execute any path for this need."

## Assumption Reassessment (2026-04-14)

1. `HomeostaticNeedId` at `crates/worldwake-core/src/needs.rs:19-25` has exactly 5 variants: Hunger, Thirst, Fatigue, Bladder, Dirtiness. Derives: Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize. No existing `VARIANT_COUNT` constant.
2. `component_schema.rs` at `crates/worldwake-core/src/component_schema.rs` uses `with_component_schema_entries!` macro for registration. ExplorationProfile registered at lines 833-856 with `|kind| kind == EntityKind::Agent`.
3. `create_agent()` at `crates/worldwake-core/src/world.rs:152-191` seeds default components for every new agent. ExplorationProfile seeded at line 171. New component must follow the same pattern. `world_txn.rs` `create_agent()` at lines 231-241 wraps the call.
4. Per `tickets/README.md` check #13: macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`) must import the new type.
5. Existing per-goal exhaustion: `AgentDecisionRuntime.exhaustion_cache: BTreeMap<OpportunityKey, ExhaustionEntry>` in `crates/worldwake-ai/src/decision_runtime.rs:166`. This tracks per-goal `consecutive_failures` for retry backoff. The new tracker complements it at per-need granularity — different purpose.

## Architecture Check

1. Per-need aggregation as an ECS component is cleaner than adding per-need tracking to the per-goal `exhaustion_cache`, which would conflate retry backoff (per-goal) with exploration triggering (per-need). Separate components keep concerns independent (FND-26).
2. No backward-compatibility shims. New component, new constant — no existing code affected until downstream tickets consume them.

## Verification Layers

1. `AcquisitionExhaustionTracker::default()` is all-zeros → focused unit test
2. `increment` / `reset` / `count` API correctness → focused unit tests
3. Component registered on Agent → compilation of `create_agent()` + `component_schema` entry
4. Single-layer ticket (core types) — no cross-system verification needed

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
- `crates/worldwake-core/src/exploration.rs` (modify — new struct)
- `crates/worldwake-core/src/component_schema.rs` (modify — registration)
- `crates/worldwake-core/src/world.rs` (modify — create_agent seeding)
- `crates/worldwake-core/src/delta.rs` (modify — macro import if needed)
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
6. Workspace builds cleanly: `cargo build --workspace`
7. Existing suite: `cargo test --workspace`

### Invariants

1. `AcquisitionExhaustionTracker` derives Copy — must remain a value type
2. Component registered only on `EntityKind::Agent`
3. Every agent created via `create_agent()` gets a default tracker

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/exploration.rs` — unit tests for tracker API (increment, reset, count, saturation)
2. `crates/worldwake-core/src/needs.rs` — assert `HomeostaticNeedId::VARIANT_COUNT == 5`

### Commands

1. `cargo test -p worldwake-core -- exploration`
2. `cargo test -p worldwake-core -- needs`
3. `cargo build --workspace && cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
