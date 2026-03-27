# S30-008: Make validated AI runtime restore the only public restore path

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `SaveableRuntime` trait surface, AI runtime restore surface, CLI load path, focused tests, S30 spec narrative
**Deps**: archive/tickets/S30-006-remove-driver-reset-workaround.md, specs/S30-ai-runtime-save-load-parity.md

## Problem

The current save/load architecture still exposes an unlawful public raw-restore path for AI runtime state. `SaveableRuntime` in [crates/worldwake-sim/src/saveable_runtime.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/saveable_runtime.rs) includes `restore_runtime_state()`, but the sim crate never uses that method. The real validated restore path already lives in `AgentTickDriver::from_saved_runtime(...)` plus `post_load_validate()` in [crates/worldwake-ai/src/agent_tick/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs).

That means the current generic boundary is weaker than the real architectural boundary. Future callers can still deserialize AI runtime through the public trait method and bypass validation even though validation is semantically required to produce a lawful post-load driver state.

## Assumption Reassessment (2026-03-27)

1. The only other active S30 implementation ticket in `tickets/` is [tickets/S30-007-increase-exhaustion-ttl.md](/home/joeloverbeck/projects/worldwake/tickets/S30-007-increase-exhaustion-ttl.md). Its scope is exhaustion TTL tuning in `planning.rs`; it does not touch the runtime restore boundary.
2. The live shared abstraction boundary under audit is the runtime persistence seam between `worldwake-sim` save/load payload transport in [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs), the public `SaveableRuntime` trait in [crates/worldwake-sim/src/saveable_runtime.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/saveable_runtime.rs), and the AI-layer validated restore entry point `AgentTickDriver::from_saved_runtime(...)` / `post_load_validate()` in [crates/worldwake-ai/src/agent_tick/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs).
3. `worldwake-sim` uses `SaveableRuntime` only for saving. `save_to_bytes()` calls `save_runtime_state()`, while `load_from_bytes()` returns opaque runtime bytes and never invokes `restore_runtime_state()`. The restore half of the trait is therefore not part of the sim crate's real transport responsibility.
4. The current AI implementation already proves that validated restore is semantically required, not optional. `post_load_validate()` prunes dead agents, prunes dead entity references from `exhaustion_cache` and `materialization_bindings`, clears invalid snapshot anchors, resets dirty bits, clears derived ranking diagnostics, and clears the semantics cache.
5. The current production restore callers in [crates/worldwake-cli/src/handlers/persistence.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/persistence.rs) and [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) already route through `AgentTickDriver::from_saved_runtime(...)`. The remaining architectural problem is the extra public raw-restore API that bypasses that validated path.
6. This ticket manipulates runtime restore conditions, so retained runtime intent must be assessed explicitly. The current architecture lawfully retains saved `current_plan`, `current_step_index`, `step_in_flight`, and related runtime state across load, as proved by `agent_tick::tests::from_saved_runtime_restores_and_validates_driver_state`, `golden_save_load_round_trip_under_ai`, and `golden_save_load_preserves_promoted_commitments`.
7. Existing focused coverage names the relevant proof surfaces: `agent_tick::tests::saveable_runtime_roundtrip_restores_persisted_driver_state` proves raw persisted bytes round-trip, `agent_tick::tests::post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty` proves validation semantics, and `agent_tick::tests::from_saved_runtime_restores_and_validates_driver_state` proves the current validated constructor path. CLI coverage exists in `handlers::persistence::tests::test_save_load_roundtrip`, and golden harness coverage exists in `golden_harness::tests::save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation`.
8. `specs/S30-ai-runtime-save-load-parity.md` still describes the restore contract as part of `SaveableRuntime` and says callers must invoke `post_load_validate()` afterward. That no longer matches the cleaner boundary. The validated constructor should be the only public restore path, while the sim trait should remain save-only.
9. This is a mixed-layer ticket but not a planner-goal ticket. No `GoalKind`, candidate-generation, plan-search, or action-start behavior changes are in scope.
10. Adjacent contradiction classification: `AgentTickDriver::from_saved_runtime(...)` is not the problem. The misplaced `restore_runtime_state()` method on the generic sim trait is the contradiction. Removing or privatizing the raw restore surface is the required consequence; widening the sim trait to take `World` would be separate architectural churn, not a requirement.
11. Mismatch + correction: the original ticket proposed making validated restore intrinsic to the generic persistence trait. Reassessment shows that would make the architecture worse by teaching `worldwake-sim` about restore-time world validation semantics it does not own. The corrected scope is to make validated restore intrinsic to the AI restore boundary and remove the unused generic raw-restore surface.

## Architecture Check

1. The clean boundary is: `worldwake-sim` transports opaque runtime bytes, and `worldwake-ai` owns the only restore operation that can interpret and validate those bytes against a `World`. That keeps responsibilities aligned with crate ownership instead of smearing AI restore semantics into the generic sim trait.
2. Removing the restore half from `SaveableRuntime` is cleaner than widening it to accept `World`. The sim crate never performs restore itself, so a world-aware restore trait there would be extra coupling without any new invariant enforcement.
3. No backwards-compatibility aliasing/shims should be introduced. The public raw-restore path should be removed or made internal, and callers should keep using a single validated restore entry point.

## Verification Layers

1. Persisted runtime bytes still round-trip losslessly through the save payload -> focused runtime test in [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs)
2. The only public restore entry point returns already-validated AI runtime state -> focused runtime test in [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs)
3. Production load path still restores runtime only through the validated AI constructor -> focused CLI runtime test in [crates/worldwake-cli/src/handlers/persistence.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/persistence.rs)
4. Save/load parity and stale-reference pruning still hold across the golden harness and determinism coverage -> [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) and [crates/worldwake-ai/tests/golden_determinism.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs)
5. This ticket does not change planning, candidate generation, action lifecycle, or authoritative mutation ordering, so decision-trace and action-trace assertions are not the primary proof surface here.

## What to Change

### 1. Remove the misplaced generic raw-restore surface

Update `SaveableRuntime` so it only exposes the save concern that `worldwake-sim` actually owns. The raw restore method should no longer be publicly available through the generic sim trait.

### 2. Keep validated restore at the AI boundary

Retain a single public AI-layer restore entry point that takes runtime bytes plus the loaded `World` and returns already-validated driver state. `post_load_validate()` may remain as a private/helper implementation detail if the focused tests still need to prove its semantics indirectly through the constructor.

### 3. Update all restore call sites to the canonical AI restore contract

Update the CLI load path, golden harness helpers, and focused tests so there is exactly one lawful public post-load restore path.

### 4. Update S30 documentation/test narrative to match the hardened contract

Update the active S30 save/load parity documentation or dependent ticket narratives if they still describe raw trait-based restore or caller-sequenced validation as the intended architecture.

## Files to Touch

- `crates/worldwake-sim/src/saveable_runtime.rs` (modify — remove the generic restore surface)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — keep validated restore as the only public restore entry point)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — focused restore-boundary coverage)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify if needed — use only the canonical AI restore surface)
- `crates/worldwake-cli/src/handlers/persistence.rs` (modify if needed — keep using only the canonical AI restore surface)
- `specs/S30-ai-runtime-save-load-parity.md` (modify — correct the contract narrative)

## Out of Scope

- Save format or payload layout changes
- Any planner, candidate-generation, ranking, or action-semantics changes
- Changing `EXHAUSTION_SKIP_TTL` or S31 invalidation semantics
- Generalizing a multi-runtime plugin system beyond the single current AI driver use case

## Acceptance Criteria

### Tests That Must Pass

1. No public caller can deserialize AI runtime state through a generic raw-restore surface.
2. Focused restore-contract tests pass in `worldwake-ai`.
3. CLI save/load tests pass using the intrinsic validated restore contract.
4. Save/load golden determinism tests still pass.
5. `cargo clippy --workspace` and `cargo test --workspace` pass.

### Invariants

1. A successful public AI runtime restore returns a post-load-valid driver state; there is no lawful public intermediate state with dead references or stale derived fields.
2. Save/load boundaries preserve AI commitments and search history without caller-side fixups.
3. No generic compatibility alias path remains for raw public restore through `SaveableRuntime`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — replace raw trait-restore coverage with tests that prove persisted bytes round-trip and the public AI restore constructor returns already-validated runtime state.
2. `crates/worldwake-cli/src/handlers/persistence.rs` — keep the load-path test pinned to the canonical production restore surface.
3. `crates/worldwake-ai/tests/golden_harness/mod.rs` — keep save/load stale-reference pruning coverage pinned to the canonical restore surface.
4. `crates/worldwake-ai/tests/golden_determinism.rs` — keep save/load parity coverage passing after the boundary cleanup.

### Commands

1. `cargo test -p worldwake-ai save_runtime_state_serializes_persisted_driver_state -- --nocapture`
2. `cargo test -p worldwake-ai from_saved_runtime_restores_and_validates_driver_state -- --nocapture`
3. `cargo test -p worldwake-ai golden_harness::tests::save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation -- --nocapture`
4. `cargo test -p worldwake-ai golden_save_load -- --nocapture`
5. `cargo test -p worldwake-cli handlers::persistence::tests::test_save_load_roundtrip -- --nocapture`
6. `cargo clippy --workspace`
7. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-27
- What actually changed:
  - Removed the public raw restore method from `worldwake-sim`'s `SaveableRuntime` trait so the generic sim boundary is save-only.
  - Kept raw AI deserialization private inside `AgentTickDriver` and preserved `AgentTickDriver::from_saved_runtime(...)` as the only public validated restore entry point.
  - Updated focused AI tests, the golden harness restore setup, and the active S30 spec narrative to match that boundary.
- Deviations from original plan:
  - Did not widen `SaveableRuntime` to accept `World` during restore. Reassessment showed that would couple `worldwake-sim` to AI restore semantics it does not own.
  - `AgentTickDriver::from_saved_runtime(...)` was kept as the canonical public restore path instead of being removed.
- Verification results:
  - Targeted tests passed:
    - `cargo test -p worldwake-ai save_runtime_state_serializes_persisted_driver_state -- --nocapture`
    - `cargo test -p worldwake-ai from_saved_runtime_restores_and_validates_driver_state -- --nocapture`
    - `cargo test -p worldwake-ai golden_harness::tests::save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation -- --nocapture`
    - `cargo test -p worldwake-ai golden_save_load -- --nocapture`
    - `cargo test -p worldwake-cli handlers::persistence::tests::test_save_load_roundtrip -- --nocapture`
  - Broad verification passed:
    - `cargo clippy --workspace`
    - `cargo test --workspace`
