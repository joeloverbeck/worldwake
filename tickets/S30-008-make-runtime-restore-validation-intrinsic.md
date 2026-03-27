# S30-008: Make AI runtime restore validation intrinsic to the persistence contract

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `SaveableRuntime` restore contract, AI runtime restore surface, CLI load path, focused tests
**Deps**: archive/tickets/S30-006-remove-driver-reset-workaround.md, specs/S30-ai-runtime-save-load-parity.md

## Problem

The current save/load architecture still permits an invalid restore sequence at the trait boundary. `SaveableRuntime::restore_runtime_state()` in [crates/worldwake-sim/src/saveable_runtime.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/saveable_runtime.rs) only deserializes bytes, while `AgentTickDriver::post_load_validate()` in [crates/worldwake-ai/src/agent_tick/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs) must be called afterward to prune dead references and reset derived fields. `S30-006` added `AgentTickDriver::from_saved_runtime(...)` and routed the known callers through it, but the underlying contract still allows future callers to bypass validation by using the low-level trait method directly.

That is not the ideal end-state under Principle 11. A save/load boundary should not rely on caller discipline to preserve world meaning. The validated post-load state should be the only legal restored state.

## Assumption Reassessment (2026-03-27)

1. The only active implementation ticket in `tickets/` besides this proposed work is [tickets/S30-007-increase-exhaustion-ttl.md](/home/joeloverbeck/projects/worldwake/tickets/S30-007-increase-exhaustion-ttl.md). Its scope is `EXHAUSTION_SKIP_TTL` tuning in `planning.rs`; it does not name the `SaveableRuntime` contract, `AgentTickDriver::post_load_validate()`, or the restore boundary as a change area.
2. The live shared abstraction boundary under audit is the persistence seam between `SaveableRuntime::restore_runtime_state()` in [crates/worldwake-sim/src/saveable_runtime.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/saveable_runtime.rs) and `AgentTickDriver::post_load_validate()` / `AgentTickDriver::from_saved_runtime(...)` in [crates/worldwake-ai/src/agent_tick/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs).
3. `SaveableRuntime::restore_runtime_state()` currently accepts only `bytes: &[u8]` and cannot validate against the loaded world. The validation step lives outside the trait contract.
4. The current AI implementation already proves that validated restore is semantically required, not optional. `post_load_validate()` prunes dead agents, prunes dead entity references from `exhaustion_cache` and `materialization_bindings`, clears invalid snapshot anchors, resets dirty bits, clears derived ranking diagnostics, and clears the semantics cache.
5. The current callers in [crates/worldwake-cli/src/handlers/persistence.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/persistence.rs) and [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) already route through `AgentTickDriver::from_saved_runtime(...)`. The remaining issue is architectural: the low-level trait still exposes an unlawful intermediate state.
6. This ticket manipulates runtime restore conditions, so retained runtime intent must be assessed explicitly. The current architecture lawfully retains saved `current_plan`, `current_step_index`, `step_in_flight`, and related runtime state across load, as proved by `golden_save_load_round_trip_under_ai`, `golden_save_load_preserves_promoted_commitments`, and `from_saved_runtime_restores_and_validates_driver_state` in the current `worldwake-ai` test suite.
7. Existing focused coverage already names the relevant proof surfaces: `saveable_runtime_roundtrip_restores_persisted_driver_state` proves raw serialization/deserialization, `post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty` proves validation, and `from_saved_runtime_restores_and_validates_driver_state` proves the current higher-level helper. Golden coverage exists in [crates/worldwake-ai/tests/golden_determinism.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs).
8. `specs/S30-ai-runtime-save-load-parity.md` still describes the restore boundary as “caller MUST call post_load_validate() afterward” and still frames the contract around separate `restore_runtime_state()` and `post_load_validate()` steps. That spec narrative no longer matches the ideal hardened architecture.
9. This is a mixed-layer ticket but not a planner-goal ticket. No `GoalKind` or candidate-generation surface changes are in scope.
10. Adjacent contradiction classification: the public existence of `AgentTickDriver::from_saved_runtime(...)` is not itself a problem; it is a partial improvement. The real contradiction is that the lower trait boundary still permits bypassing validation. Fixing the contract, not just adding more helper wrappers, is the required consequence.
11. Mismatch + correction: no remaining active ticket is scheduled to make validated restore intrinsic to the persistence contract. A new ticket is required.

## Architecture Check

1. Folding validation into the restore contract is cleaner than relying on a public helper plus a weaker low-level trait beneath it. The boundary itself should guarantee legal restored state, not merely offer a safer convenience wrapper.
2. Replacing the split `restore_runtime_state()` + caller-side validation choreography with a single validated restore contract aligns with Principle 11 (boundaries do not change world meaning), Principle 24 (clear state-mediated boundaries), and the repo’s no-workaround rule in [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md).
3. No backwards-compatibility aliasing/shims should be introduced. The old split contract should be replaced, not kept alongside a second “better” path.

## Verification Layers

1. Raw runtime bytes still deserialize into persisted AI state -> focused unit/runtime test in [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs)
2. Restored runtime cannot expose dead references or stale derived fields -> focused unit/runtime test in [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs)
3. Production load path uses only the intrinsic validated restore contract -> focused CLI runtime test in [crates/worldwake-cli/src/handlers/persistence.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/persistence.rs)
4. Save/load parity and commitment continuity still hold after the contract change -> golden determinism tests in [crates/worldwake-ai/tests/golden_determinism.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs)
5. This ticket does not change action lifecycle or planning behavior, so action-trace and decision-trace proof surfaces are not the primary verification boundary.

## What to Change

### 1. Make validated restore the only restore contract

Change the persistence trait/API so restoring runtime state cannot happen without access to the loaded world and the required validation step. The recommended end-state is:

- `SaveableRuntime::restore_runtime_state(...)` accepts the loaded `World` (or equivalent world view) as part of the contract and leaves the runtime fully validated on success.
- `AgentTickDriver::post_load_validate()` becomes an internal implementation detail or is removed if no longer needed as a separate symbol.
- `AgentTickDriver::from_saved_runtime(...)` becomes unnecessary and can be removed once callers use the intrinsic contract directly.

### 2. Update all restore call sites to the new canonical contract

Update the CLI load path, golden harness helpers, and any focused tests that still model restore as a two-step operation so there is exactly one lawful post-load restore path.

### 3. Update S30 documentation/test narrative to match the hardened contract

Update the active S30 save/load parity documentation or dependent ticket narratives if they still describe validation as a caller obligation rather than an intrinsic restore property.

## Files to Touch

- `crates/worldwake-sim/src/saveable_runtime.rs` (modify — replace split restore contract)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — fold validation into restore implementation, remove duplicate surface if possible)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — focused restore-contract coverage)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — use the intrinsic contract)
- `crates/worldwake-cli/src/handlers/persistence.rs` (modify — use the intrinsic contract)
- `specs/S30-ai-runtime-save-load-parity.md` (modify — correct the contract narrative if still active when implemented)

## Out of Scope

- Save format version changes or payload layout changes
- Any planner, candidate-generation, ranking, or action-semantics changes
- Changing `EXHAUSTION_SKIP_TTL` or S31 invalidation semantics
- Generalizing a multi-runtime plugin system beyond the single current AI driver use case

## Acceptance Criteria

### Tests That Must Pass

1. No public caller must manually sequence “restore, then validate” for AI runtime state.
2. Focused restore-contract tests pass in `worldwake-ai`.
3. CLI save/load tests pass using the intrinsic validated restore contract.
4. Save/load golden determinism tests still pass.
5. `cargo clippy --workspace` and `cargo test --workspace` pass.

### Invariants

1. A successful runtime restore returns a post-load-valid runtime state; there is no lawful public intermediate state with dead references or stale derived fields.
2. Save/load boundaries preserve AI commitments and search history without caller-side fixups.
3. No compatibility alias path remains for the old split restore contract.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — replace split restore/validation coverage with tests that prove the intrinsic restore contract returns already-validated runtime state.
2. `crates/worldwake-cli/src/handlers/persistence.rs` — keep the load-path test pinned to the canonical production restore surface.
3. `crates/worldwake-ai/tests/golden_determinism.rs` — keep save/load parity coverage passing after the contract change; update wording only if the contract description changes.

### Commands

1. `cargo test -p worldwake-ai saveable_runtime_roundtrip_restores_persisted_driver_state -- --nocapture`
2. `cargo test -p worldwake-ai from_saved_runtime_restores_and_validates_driver_state -- --nocapture`
3. `cargo test -p worldwake-ai golden_save_load -- --nocapture`
4. `cargo test -p worldwake-cli test_save_load_roundtrip -- --nocapture`
5. `cargo clippy --workspace`
6. `cargo test --workspace`
