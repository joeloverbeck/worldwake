# S30-004: Implement SaveableRuntime for AgentTickDriver

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AgentTickDriver gains save/restore capability
**Deps**: S30-002 (serde derives on AgentDecisionRuntime), S30-003 (SaveableRuntime trait exists)

## Problem

`AgentTickDriver` holds per-agent runtime state (`runtime_by_agent`, `budget`) that must survive save/load boundaries to maintain AI decision continuity (Principle 11). The originally planned work has already landed in the codebase; this ticket now records the verified architecture, corrects stale assumptions, and captures the remaining focused verification added during reassessment.

## Assumption Reassessment (2026-03-27)

Shared abstraction boundary under audit: the opaque AI runtime payload exchanged between [`crates/worldwake-sim/src/save_load.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs) and [`crates/worldwake-ai/src/agent_tick/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs), with `worldwake-sim` owning transport and `worldwake-ai` owning interpretation and post-load validation.

1. `AgentTickDriver` already has the expected private fields and already implements `SaveableRuntime`; `AgentTickDriverState` already exists as a private serde-backed intermediary in [`crates/worldwake-ai/src/agent_tick/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs).
2. `AgentDecisionRuntime`, `MaterializationBindings`, and `ExhaustionEntry` already derive serde in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs). The ticket’s future-tense assumptions were stale.
3. `SaveableRuntime` already uses `SaveError`, not `SaveLoadError`, in [`crates/worldwake-sim/src/saveable_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/saveable_runtime.rs). The ticket’s symbol name was wrong.
4. `worldwake-sim` save/load format v6, optional runtime payload transport, and legacy v5 loading are already implemented in [`crates/worldwake-sim/src/save_load.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs).
5. The golden harness already saves the driver runtime, restores it after `load_from_bytes`, and calls `post_load_validate`; the proposed `from_simulation_state(..., ai_runtime_bytes)` constructor change is unnecessary and would make construction less clean than the current explicit restore path in [`crates/worldwake-ai/tests/golden_harness/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs).
6. The focused positive-path tests the ticket planned already exist:
   - `agent_tick::tests::saveable_runtime_roundtrip_restores_persisted_driver_state`
   - `agent_tick::tests::post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
   - `golden_save_load_round_trip_under_ai`
   - `golden_save_load_preserves_promoted_commitments`
7. One gap remained during reassessment: `worldwake-ai` did not directly assert that corrupt runtime bytes surface a `SaveError::RuntimeDeserialization` from `AgentTickDriver::restore_runtime_state()`. That focused negative-path coverage belongs in this ticket.

## Architecture Check

1. The implemented architecture is cleaner than threading AI-specific bytes through `SimulationState` constructors: `worldwake-sim` carries opaque runtime bytes, while `worldwake-ai` owns `AgentTickDriverState`, restore semantics, and `post_load_validate()`. That preserves the crate boundary and keeps `SimulationState` free of AI knowledge.
2. The private `AgentTickDriverState` intermediary remains the right shape. It serializes only persisted runtime truth (`runtime_by_agent`, `budget`) and deliberately excludes rebuildable/session-local fields (`semantics_cache`, `trace_sink`), matching Principles 11, 24, and 25.
3. No backward-compatibility aliasing was introduced. Legacy compatibility lives only at the save-format boundary in `worldwake-sim` version handling, not as duplicate AI runtime paths.
4. The proposed constructor-signature expansion in the original ticket would have been a regression in architecture. Restoring runtime explicitly after `from_simulation_state()` is more robust, keeps object construction deterministic, and avoids coupling the harness constructor to one optional persistence transport.

## Verification Layers

1. Driver payload round-trip fidelity -> focused `agent_tick` unit test asserting persisted `budget`, `runtime_by_agent`, bindings, and exhaustion cache values
2. Derived/session-local exclusion -> focused `agent_tick` unit test asserting `semantics_cache`, `trace_sink`, `dirty`, `last_priority_class`, and `last_frame_clear_reason` are reset on restore
3. Corrupt runtime payload handling -> focused `agent_tick` unit test asserting `restore_runtime_state()` returns `SaveError::RuntimeDeserialization`
4. Save/load AI continuity across crate boundary -> golden determinism scenario `golden_save_load_round_trip_under_ai`
5. Mid-plan promoted commitment continuity -> golden determinism scenarios `golden_save_load_preserves_promoted_commitments` and `_replays_deterministically`

## What Changed

1. Verified the driver/runtime save-load implementation already exists in production code and matches the intended architecture.
2. Added focused negative-path coverage for corrupt runtime payloads in the AI crate.
3. Corrected the ticket so its scope matches the live code and current verification surfaces rather than proposing already-landed or architecturally inferior changes.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — add corrupt-runtime-payload negative test)
- `tickets/S30-004-implement-saveable-runtime-for-driver.md` (modify — corrected reassessment and completion record)

## Out of Scope

- Reworking the existing save/runtime architecture; reassessment found the current boundary design already clean and robust
- Any additional constructor or harness API churn to thread runtime bytes through `from_simulation_state()`
- Behavioral changes to AI planning or action execution

## Acceptance Criteria

### Tests That Must Pass

1. `agent_tick::tests::saveable_runtime_roundtrip_restores_persisted_driver_state`
2. `agent_tick::tests::post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
3. `agent_tick::tests::restore_runtime_state_rejects_invalid_bytes`
4. `golden_save_load_round_trip_under_ai`
5. `golden_save_load_preserves_promoted_commitments`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace`
8. `cargo test --workspace`

### Invariants

1. `AgentTickDriverState` remains private to `worldwake-ai`
2. Serialization remains deterministic (`BTreeMap`, no floats)
3. `semantics_cache` and `trace_sink` remain excluded from persisted runtime state
4. Post-load validation remains the canonical place to prune stale references and reinitialize derived state
5. No new ECS components or alternate runtime persistence paths

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — `saveable_runtime_roundtrip_restores_persisted_driver_state`
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — `post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
3. `crates/worldwake-ai/src/agent_tick/tests.rs` — `restore_runtime_state_rejects_invalid_bytes`
4. `crates/worldwake-ai/tests/golden_determinism.rs` — `golden_save_load_round_trip_under_ai`
5. `crates/worldwake-ai/tests/golden_determinism.rs` — `golden_save_load_preserves_promoted_commitments`

### Commands

1. `cargo test -p worldwake-ai saveable_runtime_roundtrip_restores_persisted_driver_state`
2. `cargo test -p worldwake-ai post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
3. `cargo test -p worldwake-ai restore_runtime_state_rejects_invalid_bytes`
4. `cargo test -p worldwake-ai golden_save_load_round_trip_under_ai -- --nocapture`
5. `cargo test -p worldwake-ai golden_save_load_preserves_promoted_commitments -- --nocapture`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace`
8. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-27
- What actually changed: reassessment confirmed the production implementation was already present; added focused negative-path coverage for corrupt runtime payload deserialization; corrected the ticket to match the live architecture and verification surface.
- Deviations from original plan: no production code or harness constructor changes were needed. The original proposal to thread runtime bytes through `from_simulation_state()` was rejected as a less clean architecture than the existing explicit restore flow.
- Verification results: focused driver save/restore and post-load validation tests passed; golden save/load parity tests passed; final workspace-wide verification recorded after ticket close-out.
