# S33OPPSCOGOAIDE-008: Save/load coverage for opportunity-scoped runtime identity

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None beyond focused AI runtime post-load validation coverage
**Deps**: S33OPPSCOGOAIDE-004, S33OPPSCOGOAIDE-006

## Problem

The original ticket assumed the S33 opportunity-scoped runtime identity work had not yet landed. In live code, the runtime model, serialization shape, and post-load pruning path already exist. The remaining gap is to verify that the current architecture preserves the intended save/load invariants for all anchor variants without moving AI-specific validation into the generic simulation save layer.

The shared abstraction boundary under audit is:

- persisted snapshot container format in `worldwake-sim`
- AI runtime ownership and post-load validation in `worldwake-ai::agent_tick::AgentTickDriver`

## Assumption Reassessment (2026-03-28)

1. `OpportunityAnchor` and `OpportunityKey` are already live in [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs), including focused bincode coverage for `OpportunityKey`.
2. `AgentDecisionRuntime.exhaustion_cache` is already keyed by `OpportunityKey` in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs), so the original ticket’s “re-key the save format” premise is stale.
3. `PlannedPlan` already carries `opportunity`, and runtime save coverage already asserts that field survives serialization in [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs).
4. Post-load pruning already exists in `AgentTickDriver::post_load_validate()` in [`crates/worldwake-ai/src/agent_tick/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs). This is the correct ownership boundary because the AI runtime, not `worldwake-sim`, knows which cached runtime references are stale and how to invalidate them.
5. `worldwake-sim` save/load remains a generic byte container in [`crates/worldwake-sim/src/save_load.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs). Moving AI-runtime pruning there would couple a generic persistence layer to AI-specific runtime semantics, which is architecturally worse than the current design.
6. `SAVE_FORMAT_VERSION` is already `9` in [`crates/worldwake-sim/src/save_load.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs). Because the opportunity-scoped runtime layout is already the committed live format, this ticket should not require a further bump unless this ticket itself changes serialized bytes again.
7. Current focused coverage already proves three important pieces:
   - runtime save serialization preserves `PlannedPlan.opportunity` and `OpportunityKey` exhaustion entries via `agent_tick::tests::save_runtime_state_serializes_persisted_driver_state`
   - runtime restore prunes dead entity-linked state via `agent_tick::tests::from_saved_runtime_restores_and_validates_driver_state`
   - full save/load round-trip re-applies post-load validation via `golden_harness::tests::save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation`
8. The remaining undercovered edge from the original ticket is anchor completeness: the focused tests cover dead `OpportunityAnchor::Entity` pruning, but they do not explicitly prove that dead `OpportunityAnchor::Place` entries are pruned while `OpportunityAnchor::None` entries survive.
9. Mismatch + correction: the original ticket assigned version bumping and pruning work to `worldwake-sim/src/save_load.rs`. In the live architecture, the correct scope is narrower: keep the container format unchanged, keep pruning in `worldwake-ai`, and harden tests around the already-landed post-load validation behavior.

## Architecture Check

1. Keeping post-load pruning in `AgentTickDriver::post_load_validate()` is cleaner than pushing it into `worldwake-sim` because ownership stays with the runtime that defines these caches and their invariants.
2. Preserving the current save container and only strengthening coverage is more robust than bumping the save format again without an actual wire-format change. A gratuitous version bump would create churn without buying a cleaner architecture.
3. No backward-compatibility aliasing or shim path is introduced. The live opportunity-scoped runtime layout remains canonical.

## Verification Layers

1. `OpportunityKey` exhaustion entries and `PlannedPlan.opportunity` survive runtime serialization -> focused `agent_tick` runtime serialization tests.
2. Dead runtime references are pruned on restore, including anchor-specific exhaustion entries -> focused `agent_tick` post-load validation tests.
3. Post-load validation is re-applied across a full save/load round-trip -> golden harness save/load round-trip test.
4. Generic save container remains valid and stable -> existing `worldwake-sim` save/load unit tests.
5. This is not a mixed-layer ordering ticket; action-trace and event-log ordering surfaces are not the contract under audit here.

## What to Change

### 1. Correct the ticket scope

Rewrite this ticket so it matches the already-landed architecture:

- no mandatory `SAVE_FORMAT_VERSION` bump unless serialized bytes change again
- no migration of AI-runtime pruning logic into `worldwake-sim`
- focus remaining work on missing anchor-coverage tests in `worldwake-ai`

### 2. Harden anchor-completeness coverage

Add or strengthen focused tests proving:

- `OpportunityAnchor::Place(dead_place)` exhaustion entries are pruned on restore
- `OpportunityAnchor::None` exhaustion entries survive post-load validation

## Files to Touch

- `tickets/S33OPPSCOGOAIDE-008-save-load.md` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)

## Out of Scope

- Any further `SAVE_FORMAT_VERSION` bump without a new serialized layout change
- Moving runtime pruning into `worldwake-sim`
- Reworking the already-landed opportunity-scoped runtime model
- Replay changes

## Acceptance Criteria

### Tests That Must Pass

1. Focused runtime serialization still preserves `PlannedPlan.opportunity` and `OpportunityKey` exhaustion entries.
2. Focused restore/post-load validation prunes dead `OpportunityAnchor::Entity` and dead `OpportunityAnchor::Place` entries.
3. Focused restore/post-load validation preserves `OpportunityAnchor::None` entries.
4. Existing suite: `cargo test -p worldwake-ai agent_tick::tests::save_runtime_state_serializes_persisted_driver_state`
5. Existing suite: `cargo test -p worldwake-ai agent_tick::tests::from_saved_runtime_restores_and_validates_driver_state`
6. Existing suite: `cargo test -p worldwake-ai agent_tick::tests::post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
7. Existing suite: `cargo test -p worldwake-ai golden_harness::tests::save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation`
8. Existing suite: `cargo test -p worldwake-sim -- save_load`
9. Existing suite: `cargo clippy --workspace`
10. Existing suite: `cargo test --workspace`

### Invariants

1. AI-runtime stale-reference pruning remains owned by `worldwake-ai`, not duplicated in `worldwake-sim`.
2. Dead `OpportunityAnchor::Place` and `OpportunityAnchor::Entity` exhaustion entries do not survive restore.
3. `OpportunityAnchor::None` exhaustion entries are never pruned solely because they lack an external anchor.
4. No backward-compatibility shim is introduced for an obsolete pre-opportunity runtime layout.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — extend focused restore/post-load validation coverage to include dead place-anchor pruning and `OpportunityAnchor::None` retention.

### Commands

1. `cargo test -p worldwake-ai agent_tick::tests::save_runtime_state_serializes_persisted_driver_state`
2. `cargo test -p worldwake-ai agent_tick::tests::from_saved_runtime_restores_and_validates_driver_state`
3. `cargo test -p worldwake-ai agent_tick::tests::post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
4. `cargo test -p worldwake-ai golden_harness::tests::save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation`
5. `cargo test -p worldwake-sim -- save_load`
6. `cargo clippy --workspace`
7. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-28
- What actually changed: corrected the ticket to match the already-landed S33 architecture, kept save/load ownership split as-is, and strengthened `worldwake-ai` save/load coverage for dead place-anchor pruning plus `OpportunityAnchor::None` retention in both focused and golden-harness tests.
- Deviations from original plan: did not bump `SAVE_FORMAT_VERSION`, did not move pruning into `worldwake-sim`, and did not change production save/load code because that architecture was already cleaner and already implemented in the correct owner.
- Verification results:
  - `cargo test -p worldwake-ai agent_tick::tests::save_runtime_state_serializes_persisted_driver_state`
  - `cargo test -p worldwake-ai agent_tick::tests::from_saved_runtime_restores_and_validates_driver_state`
  - `cargo test -p worldwake-ai agent_tick::tests::post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
  - `cargo test -p worldwake-ai golden_harness::tests::save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation`
  - `cargo test -p worldwake-sim -- save_load`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
