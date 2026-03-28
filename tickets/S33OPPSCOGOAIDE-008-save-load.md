# S33OPPSCOGOAIDE-008: Save/load support for opportunity-scoped runtime identity

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — save format change and post-load pruning
**Deps**: S33OPPSCOGOAIDE-004, S33OPPSCOGOAIDE-006

## Problem

Once exhaustion is keyed by `OpportunityKey` and `PlannedPlan` carries explicit `opportunity`, the save format must change with it. Save/load also needs a post-load pruning pass so stale anchors to dead entities do not survive in persisted AI runtime state.

The shared abstraction boundary under audit is:

- runtime AI state in `worldwake-ai`
- persisted snapshot format in `worldwake-sim`

## Assumption Reassessment (2026-03-28)

1. `SAVE_FORMAT_VERSION` in `crates/worldwake-sim/src/save_load.rs` must be treated as stale once `S33OPPSCOGOAIDE-004` and `S33OPPSCOGOAIDE-006` land. This ticket should not assume the bump is needed before those structural changes exist.
2. `OpportunityKey`/`OpportunityAnchor` already exist in live code. The remaining persistence work depends on new runtime ownership of those types, not on type definition work.
3. The serialized layout change comes from two concrete runtime changes: exhaustion cache keys becoming `OpportunityKey`, and `PlannedPlan` gaining `opportunity`.
4. Dead-anchor pruning is still required after load because persisted opportunity identity can outlive an entity or place reference.

## Architecture Check

1. Bumping `SAVE_FORMAT_VERSION` is the correct approach. Maintaining compatibility with the old layout would violate the repository rule against backward-compatibility layers.
2. Dead-anchor pruning is not optional cleanup; it preserves locality and concrete-state correctness by ensuring persisted suppression does not reference nonexistent world objects.
3. No backward-compatibility shims.

## Verification Layers

1. Round-trip preservation -> focused save/load test once runtime structures change.
2. Dead-anchor pruning -> focused save/load test.
3. Full workspace regression -> required because save/load spans crate boundaries.

## What to Change

### 1. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs`, bump the format version to the next integer after the currently committed value when this ticket lands.

### 2. Add post-load pruning for dead anchors

After loading `AgentDecisionRuntime` state, iterate `exhaustion_cache` and remove entries where:
- `anchor` is `OpportunityAnchor::Place(id)` and `!world.is_alive(id)`
- `anchor` is `OpportunityAnchor::Entity(id)` and `!world.is_alive(id)`
- `anchor` is `OpportunityAnchor::None` — never pruned.

### 3. Verify PlannedPlan serialization

The new `opportunity` field on `PlannedPlan` is automatically handled by serde derives, but verify with a round-trip test.

## Files to Touch

- `crates/worldwake-sim/src/save_load.rs` (modify — version bump and load-time pruning)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify if pruning logic belongs on runtime state helpers)

## Out of Scope

- Exhaustion cache re-keying (S33OPPSCOGOAIDE-004)
- `PlannedPlan` field addition (S33OPPSCOGOAIDE-006)
- Replay changes (replay re-derives state from inputs; no structural change needed beyond save/load)
- Migration from version 9 saves (P26 — old saves are not forward-compatible)

## Acceptance Criteria

### Tests That Must Pass

1. Save/load round-trip preserves `OpportunityKey` in exhaustion cache.
2. Save/load round-trip preserves `PlannedPlan.opportunity`.
3. Post-load pruning removes exhaustion entries with dead entity or dead place anchors.
4. `OpportunityAnchor::None` entries remain intact.
5. `SAVE_FORMAT_VERSION` is bumped at the landing commit.
6. Existing suite: `cargo test -p worldwake-sim -- save`
7. Existing suite: `cargo test --workspace`

### Invariants

1. No compatibility shim exists for the old save layout.
2. No dead-anchor exhaustion entry survives load.
3. `OpportunityAnchor::None` entries are never pruned.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/save_load.rs` — `opportunity_runtime_roundtrip`
2. `crates/worldwake-sim/src/save_load.rs` or runtime helper tests — `dead_anchor_exhaustion_entries_are_pruned_on_load`
3. Existing save/load tests updated for the new format version

### Commands

1. `cargo test -p worldwake-sim -- save`
2. `cargo test -p worldwake-ai -- decision_runtime`
3. `cargo clippy --workspace`
4. `cargo test --workspace`
