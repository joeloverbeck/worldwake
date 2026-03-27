**Status**: COMPLETED

# S30-005: Post-load validation for AgentTickDriver

**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — already delivered in `AgentTickDriver::post_load_validate()`; 2026-03-27 reassessment added one missing harness regression test and corrected this ticket’s scope.
**Deps**: S30-004, `specs/S30-ai-runtime-save-load-parity.md`

## Problem

After deserializing AI runtime state, serialized entity references can outlive the authoritative world they point at. Save/load parity requires restoring the AI runtime without preserving dead references, stale observation anchors, or derived caches that should be recomputed from the loaded world.

## Assumption Reassessment (2026-03-27)

1. The original ticket was stale. `AgentTickDriver::post_load_validate(&World)` already exists in [crates/worldwake-ai/src/agent_tick/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs) and is already invoked by the golden harness in [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs).
2. The live implementation is broader, and cleaner, than the original ticket text claimed. It prunes:
   - dead agent entries from `runtime_by_agent`
   - dead `GoalKey.entity` / `GoalKey.place` references from `exhaustion_cache`
   - dead authoritative bindings from `materialization_bindings`
   - dead `last_effective_place` anchors
   - dead entries from `last_facility_access_signature`
3. The live implementation also resets derived state after load:
   - `dirty = DirtySet::STRUCTURAL_MASK | DirtySet::SNAPSHOT_MASK | DirtySet::FRAME_MASK`
   - `last_priority_class = None`
   - `last_frame_clear_reason = None`
   - `semantics_cache = None`
4. The original `DirtySet::all()` assumption was incorrect for current code. The concrete contract is the union of structural, snapshot, and frame masks, which is the meaningful “full re-evaluation” surface in the live runtime.
5. The save/load boundary is already versioned and wired as expected. `SAVE_FORMAT_VERSION` is already `6` in [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs), `SaveableRuntime` already exists in [crates/worldwake-sim/src/autonomous_controller.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/autonomous_controller.rs), and the golden harness already restores runtime bytes before validating against the loaded world.
6. Existing focused coverage already proved the driver-level behavior in [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs). The real remaining gap was proof that the golden harness roundtrip path actually executes the restore-plus-validate sequence, so this ticket’s scope was corrected to strengthen that boundary test rather than re-implement already-landed engine code.

## Architecture Check

1. The current split between `restore_runtime_state(bytes)` and `post_load_validate(world)` is the right architecture. Deserialization must remain world-agnostic to preserve the `worldwake-sim` -> opaque-bytes -> `worldwake-ai` boundary from `specs/S30-ai-runtime-save-load-parity.md`.
2. Folding validation into raw deserialization would be worse. It would either require `worldwake-sim` to know AI internals or require the AI layer to deserialize against a world it does not yet own. The current two-phase rehydration keeps crate boundaries clean and explicit.
3. The live implementation is better than the original ticket plan because it validates the entire observation-anchor surface, not just the obvious entity maps. Pruning `last_effective_place` and `last_facility_access_signature` keeps the first post-load observation refresh coherent and avoids retaining stale locality assumptions.
4. No backward-compatibility shim or alias path is needed here. The architecture already does the right thing: restore authoritative runtime bytes, then rehydrate derived AI state from the loaded world.
5. Ideal future direction, if this area grows, would be a single higher-level “rehydrate runtime from save” helper that still preserves the current crate boundary and still performs world-aware validation only after `SimulationState` is loaded. That is not needed for this ticket because the current call path is already explicit and robust.

## Verification Layers

1. Runtime serialization contract -> [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) `saveable_runtime_roundtrip_restores_persisted_driver_state`
2. Invalid runtime bytes rejection -> [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) `restore_runtime_state_rejects_invalid_bytes`
3. Driver-level stale-reference pruning and derived-state reset -> [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) `post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
4. Golden harness restore path invokes world-aware validation -> [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) `save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation`
5. End-to-end save/load determinism under AI -> [crates/worldwake-ai/tests/golden_determinism.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs) `golden_save_load_round_trip_under_ai` and companion save/load parity tests

## What Changed

1. Confirmed that the intended engine work was already implemented in the current architecture.
2. Added a missing focused harness regression test proving that the public save/load roundtrip path executes restore plus post-load validation, not just the direct unit-tested driver method.
3. Corrected the ticket’s assumptions and archived it as completed instead of leaving it as a stale pending implementation ticket.

## Out of Scope

1. Any change to the runtime payload format or `SaveableRuntime` contract
2. Any change to AI decision behavior beyond post-load runtime hygiene
3. Any new compatibility path for older runtime layouts
4. Any refactor that hides the current explicit restore/validate sequencing behind a shim without architectural benefit

## Acceptance Criteria

1. `AgentTickDriver::post_load_validate()` remains the single world-aware runtime cleanup step after restore.
2. Save/load roundtrip continues to prune dead runtime references and clear derived caches before the next AI tick.
3. Focused driver tests and harness roundtrip coverage both pass.
4. `cargo test -p worldwake-ai`, `cargo clippy --workspace`, and `cargo test --workspace` pass.

## Tests

### New/Modified Tests

1. [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) `save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation`
Rationale: proves the real public save/load boundary restores runtime bytes and then applies world-aware validation before resumed execution, covering the exact harness path the original ticket wanted to wire.

### Existing Relevant Tests

1. [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) `saveable_runtime_roundtrip_restores_persisted_driver_state`
Rationale: proves persisted runtime fields survive serialize/deserialize and derived fields stay reset until post-load validation.
2. [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) `post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
Rationale: proves direct driver-level pruning and derived-state reset for stale runtime references.
3. [crates/worldwake-ai/tests/golden_determinism.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs) `golden_save_load_round_trip_under_ai`
Rationale: proves end-to-end resumed execution remains behaviorally aligned with uninterrupted execution.

### Commands

1. `cargo test -p worldwake-ai save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation -- --exact`
2. `cargo test -p worldwake-ai post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty -- --exact`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-27
- What actually changed: reassessed the ticket against live code, confirmed the engine work was already implemented, added one focused golden harness regression test for the restore-plus-validate boundary, and corrected the ticket to match the delivered architecture.
- Deviations from original plan: no new `post_load_validate()` implementation or harness wiring was needed because both were already present; the only code change in this pass was the missing harness proof.
- Verification results: `cargo test -p worldwake-ai save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation -- --exact`, `cargo test -p worldwake-ai post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty -- --exact`, `cargo test -p worldwake-ai`, `cargo clippy --workspace`, and `cargo test --workspace` all passed on 2026-03-27.
