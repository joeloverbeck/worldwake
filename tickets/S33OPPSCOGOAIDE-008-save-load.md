# S33OPPSCOGOAIDE-008: Save/load support for OpportunityKey and dead-entity pruning

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — SAVE_FORMAT_VERSION bump, dead-entity pruning
**Deps**: S33OPPSCOGOAIDE-001, S33OPPSCOGOAIDE-004, S33OPPSCOGOAIDE-006

## Problem

`OpportunityKey` and `OpportunityAnchor` are now part of persisted runtime state (`exhaustion_cache`, `PlannedPlan`). Save/load must serialize and deserialize these types correctly. `SAVE_FORMAT_VERSION` must bump. Post-load pruning must remove exhaustion entries whose anchor references dead entities.

## Assumption Reassessment (2026-03-28)

1. `SAVE_FORMAT_VERSION` at `crates/worldwake-sim/src/save_load.rs:6` is currently `9`. Confirmed.
2. `OpportunityAnchor` and `OpportunityKey` (S33OPPSCOGOAIDE-001) derive `Serialize, Deserialize`. They will serialize/deserialize via bincode automatically.
3. `ExhaustionEntry` already derives `Serialize, Deserialize`. Changing the cache key from `GoalKey` to `OpportunityKey` changes the serialized layout.
4. `PlannedPlan` already derives `Serialize, Deserialize`. Adding the `opportunity` field changes the serialized layout.
5. `AgentDecisionRuntime` is serialized as part of the AI driver state. Changes to its fields affect the save format.
6. Dead-entity pruning: the allocator at `crates/worldwake-core/src/allocator.rs` provides `is_alive(EntityId)` checks. Post-load, any `OpportunityAnchor::Place(id)` or `OpportunityAnchor::Entity(id)` where `id` is dead should be pruned from the exhaustion cache.

## Architecture Check

1. Bumping `SAVE_FORMAT_VERSION` is the correct approach. The alternative — maintaining backward compatibility with version 9 — violates P26 (no backward compatibility in live authority paths).
2. Dead-entity pruning on load is necessary because entities may have been despawned between save and load. Without pruning, stale exhaustion entries could suppress valid opportunities for dead-anchor goals.
3. No backward-compatibility shims.

## Verification Layers

1. Round-trip preservation → focused unit test: save state with OpportunityKey entries, load, verify entries match.
2. Dead-entity pruning → focused unit test: save with alive entity anchor, kill entity, load, verify entry pruned.
3. Single-layer ticket (serialization boundary).

## What to Change

### 1. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs`, change `SAVE_FORMAT_VERSION` from `9` to `10`.

### 2. Add post-load pruning for dead-entity anchors

After loading `AgentDecisionRuntime` state, iterate `exhaustion_cache` and remove entries where:
- `anchor` is `OpportunityAnchor::Place(id)` and `!world.is_alive(id)`
- `anchor` is `OpportunityAnchor::Entity(id)` and `!world.is_alive(id)`
- `anchor` is `OpportunityAnchor::None` — never pruned.

### 3. Verify PlannedPlan serialization

The new `opportunity` field on `PlannedPlan` is automatically handled by serde derives, but verify with a round-trip test.

## Files to Touch

- `crates/worldwake-sim/src/save_load.rs` (modify — bump version, add dead-entity pruning)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add `prune_dead_entity_exhaustion()` method if needed)

## Out of Scope

- `OpportunityAnchor`/`OpportunityKey` type definitions (S33OPPSCOGOAIDE-001)
- Exhaustion cache re-keying (S33OPPSCOGOAIDE-004)
- `PlannedPlan` field addition (S33OPPSCOGOAIDE-006)
- Replay changes (replay re-derives state from inputs; no structural change needed beyond save/load)
- Migration from version 9 saves (P26 — old saves are not forward-compatible)

## Acceptance Criteria

### Tests That Must Pass

1. Save/load round-trip preserves `OpportunityKey` in exhaustion cache.
2. Save/load round-trip preserves `PlannedPlan.opportunity` field.
3. Post-load pruning removes exhaustion entries with dead-entity anchors.
4. Post-load pruning leaves `OpportunityAnchor::None` entries intact.
5. `SAVE_FORMAT_VERSION` is `10`.
6. Existing suite: `cargo test -p worldwake-sim -- save`
7. Existing suite: `cargo test --workspace`

### Invariants

1. `SAVE_FORMAT_VERSION` bump prevents loading version 9 saves with version 10 code (mismatch detected at load time).
2. No dead-entity references survive in exhaustion cache after load.
3. `OpportunityAnchor::None` entries are never pruned.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/save_load.rs` — `test_opportunity_key_roundtrip` — save and load with OpportunityKey entries.
2. `crates/worldwake-sim/src/save_load.rs` or `decision_runtime.rs` — `test_dead_entity_pruning` — entity dies between save and load, entry pruned.
3. Existing save/load tests updated for version 10.

### Commands

1. `cargo test -p worldwake-sim -- save`
2. `cargo test -p worldwake-ai -- decision_runtime`
3. `cargo clippy --workspace && cargo test --workspace`
