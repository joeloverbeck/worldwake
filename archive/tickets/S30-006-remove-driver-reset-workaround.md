# S30-006: Remove golden_determinism driver reset workaround

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AI runtime restore surface, golden harness, CLI load path
**Deps**: S30-004 (runtime save/restore wired), S30-005 (post-load validation wired)

## Problem

The save/load parity bug itself is already fixed: `GoldenHarness::save_load_roundtrip()` saves `Some(&self.driver)`, loads `(SimulationState, Option<Vec<u8>>)`, restores the serialized AI payload into a fresh `AgentTickDriver`, and then calls `post_load_validate()`. The stale part is the restore architecture and the ticket narrative. Runtime restoration is currently open-coded at multiple call sites (`crates/worldwake-ai/tests/golden_harness/mod.rs`, `crates/worldwake-cli/src/handlers/persistence.rs`) as `AgentTickDriver::new(...)` followed by `restore_runtime_state()` and then `post_load_validate()`. That split surface is easy to misuse and leaves the ticket incorrectly framed as a test-only deletion.

## Assumption Reassessment (2026-03-27)

1. `GoldenHarness::save_load_roundtrip()` in [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) already preserves AI runtime payloads by calling `save_to_bytes(&self.snapshot_state(), Some(&self.driver))`, then `load_from_bytes(...)`, then `restore_runtime_state(&bytes)`, then `post_load_validate(&restored.world)`. The runtime is not being silently dropped anymore.
2. `GoldenHarness::from_simulation_state()` in [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) still constructs `AgentTickDriver::new(PlanningBudget::default())`, but that fresh driver is only a restore target. The architectural issue is not “state lost at boundary”; it is that restore correctness depends on every caller remembering the full `new -> restore -> post_load_validate` sequence.
3. The same split restore contract exists in production CLI load handling in [crates/worldwake-cli/src/handlers/persistence.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/persistence.rs). This ticket therefore cannot remain `Engine Changes: None` or “test-only”; the clean fix belongs in the AI runtime restore surface and should be reused by tests and CLI loading.
4. The exact shared boundary under audit is the AI runtime persistence seam formed by `SaveableRuntime::restore_runtime_state()` in [crates/worldwake-sim/src/saveable_runtime.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/saveable_runtime.rs) plus `AgentTickDriver::post_load_validate()` in [crates/worldwake-ai/src/agent_tick/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs). The current seam is two-step and mutation-based; the desired seam is a validated restore entrypoint that makes those steps inseparable.
5. Existing coverage is broader than the original ticket claims. Focused/unit coverage already exists for raw runtime round-trip and post-load pruning in [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) (`saveable_runtime_roundtrip_restores_persisted_driver_state`, `post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`). Golden coverage already exists in [crates/worldwake-ai/tests/golden_determinism.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs) (`golden_save_load_round_trip_under_ai`, `golden_save_load_preserves_promoted_commitments`, `golden_save_load_preserves_promoted_commitments_replays_deterministically`). The gap is a focused proof that the higher-level restore entrypoint itself preserves those guarantees.
6. `cargo test -p worldwake-ai -- --list` confirms the live golden names above. The original command `cargo test -p worldwake-ai golden_save_load_round_trip_under_ai golden_save_load_preserves_promoted_commitments -- --nocapture` is invalid on the current Cargo CLI because `cargo test` accepts a single positional filter.
7. `golden_save_load_round_trip_under_ai` is still the intended save/load determinism invariant, but its helper assertions in [crates/worldwake-ai/tests/golden_determinism.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs) still narrate the resumed path as “fresh AI runtime,” which is no longer architecturally precise.
8. Mismatch + correction: this ticket is not about proving that runtime bytes survive the boundary; that is already true. The corrected scope is to replace duplicated open-coded restore choreography with one validated AI-layer restore helper, update callers to use it, and strengthen focused/runtime coverage around that canonical restore path.
9. Adjacent contradiction classification: the stale “fresh runtime” wording in golden assertions is an in-scope documentation/test precision cleanup because it describes the exact behavior under test. No planner or save-format changes are in scope.

## Architecture Check

1. A validated `AgentTickDriver` restore helper is cleaner than pushing `new + restore + post_load_validate` into each caller. It makes the authoritative post-load contract explicit at the AI layer, prevents future call sites from forgetting validation, and improves both test harness and production CLI loading through one surface.
2. Keeping `restore_runtime_state()` as the low-level trait implementation but routing callers through a higher-level validated constructor/factory is cleaner than changing save/load format or adding alias paths. This removes duplicated choreography without introducing backward-compatibility shims.

## Verification Layers

1. Validated runtime restore preserves serialized driver state -> focused unit/runtime test in [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs)
2. Validated runtime restore prunes dead references and resets derived fields -> focused unit/runtime test in [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs)
3. Golden harness save/load resumes identically after AI restore -> golden determinism tests in [crates/worldwake-ai/tests/golden_determinism.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs)
4. Production CLI load path uses the same canonical restore surface -> focused CLI runtime test in [crates/worldwake-cli/src/handlers/persistence.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/persistence.rs)
5. This ticket does not change candidate generation, planning search, or action lifecycle semantics, so additional decision-trace/action-trace layer mapping is not the relevant proof surface here.

## What to Change

### 1. Add a validated AI restore entrypoint

Add a helper on `AgentTickDriver` that takes serialized runtime bytes plus the loaded world, performs `restore_runtime_state()`, immediately runs `post_load_validate()`, and returns a ready-to-use driver. This becomes the canonical restore surface.

### 2. Route harness and CLI loading through the canonical restore surface

Update `GoldenHarness::save_load_roundtrip()` and CLI `handle_load()` to use the validated helper instead of open-coding `new + restore + post_load_validate`.

### 3. Tighten focused/runtime coverage and wording

Add or strengthen focused tests so the canonical restore path itself is covered, and update stale “fresh AI runtime” wording in the determinism helper assertions to match the actual restored-runtime behavior.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — add validated restore helper)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — focused coverage for validated restore helper)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — use validated restore helper)
- `crates/worldwake-ai/tests/golden_determinism.rs` (modify — precision wording if needed)
- `crates/worldwake-cli/src/handlers/persistence.rs` (modify — use validated restore helper and strengthen load test)

## Out of Scope

- Changing `EXHAUSTION_SKIP_TTL` value (S30-007)
- Any candidate generation, ranking, or planner-search behavior changes
- Any save format changes or `SaveableRuntime` trait redesign
- Broad test refactors unrelated to the canonical restore path

## Acceptance Criteria

### Tests That Must Pass

1. `golden_save_load_round_trip_under_ai` passes with the harness using the canonical validated restore helper.
2. `golden_save_load_preserves_promoted_commitments` and `golden_save_load_preserves_promoted_commitments_replays_deterministically` continue passing.
3. CLI load still restores persisted AI runtime state through the same canonical path.
4. Relevant golden coverage passes: `cargo test -p worldwake-ai golden_save_load`
5. Broader regression checks pass: `cargo test -p worldwake-ai golden` and `cargo clippy --workspace && cargo test --workspace`

### Invariants

1. Save/load parity: uninterrupted and resumed runs produce identical world hashes at every post-save tick (Principle 11)
2. The canonical post-load AI restore path performs validation immediately after deserialization; callers do not manually reimplement that sequence.
3. No backward-compatibility alias path is introduced; existing save bytes are restored through the same low-level trait surface.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — add focused coverage for the new validated restore helper so restore + post-load pruning are proven at the canonical AI-layer boundary.
2. `crates/worldwake-cli/src/handlers/persistence.rs` — strengthen load-path coverage so the production CLI entrypoint proves it restores persisted AI runtime state rather than only world tick/entity count.
3. `crates/worldwake-ai/tests/golden_determinism.rs` — update save/load wording if needed so the assertions describe restored runtime semantics precisely.

### Commands

1. `cargo test -p worldwake-ai saveable_runtime_roundtrip_restores_persisted_driver_state -- --nocapture`
2. `cargo test -p worldwake-ai post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty -- --nocapture`
3. `cargo test -p worldwake-ai golden_save_load -- --nocapture`
4. `cargo test -p worldwake-cli persistence::tests::test_save_load_roundtrip -- --nocapture`
5. `cargo test -p worldwake-ai golden`
6. `cargo clippy --workspace`
7. `cargo test --workspace`

## Outcome

- Completed: 2026-03-27
- What actually changed:
  - Added `AgentTickDriver::from_saved_runtime(...)` in [crates/worldwake-ai/src/agent_tick/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs) so AI runtime deserialization and `post_load_validate()` are one canonical operation.
  - Updated the golden harness round-trip helper in [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) and the CLI load path in [crates/worldwake-cli/src/handlers/persistence.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/persistence.rs) to use that canonical restore surface.
  - Added focused restore-helper coverage in [crates/worldwake-ai/src/agent_tick/tests.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs), strengthened CLI load-path verification in [crates/worldwake-cli/src/handlers/persistence.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/persistence.rs), and corrected stale “fresh AI runtime” wording in [crates/worldwake-ai/tests/golden_determinism.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_determinism.rs).
- Deviations from original plan:
  - Did not remove every internal `AgentTickDriver::new(...)` construction. Reassessment showed the real architectural issue was duplicated restore choreography, not mere fresh construction. The final fix centralized restore + validation instead of changing the save format or forcing a different harness constructor signature everywhere.
- Verification results:
  - Passed `cargo test -p worldwake-ai saveable_runtime_roundtrip_restores_persisted_driver_state -- --nocapture`
  - Passed `cargo test -p worldwake-ai from_saved_runtime_restores_and_validates_driver_state -- --nocapture`
  - Passed `cargo test -p worldwake-ai golden_save_load -- --nocapture`
  - Passed `cargo test -p worldwake-cli test_save_load_roundtrip -- --nocapture`
  - Passed `cargo clippy --workspace`
  - Passed `cargo test --workspace`
