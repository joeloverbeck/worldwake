# S37COOBASEXH-007: Save/load version bump for ExhaustionEntry schema change

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: No new engine changes required during reassessment; the intended `SAVE_FORMAT_VERSION` bump and cooldown-state persistence were already implemented in live code
**Deps**: [archive/specs/S37-cooldown-based-exhaustion.md](/home/joeloverbeck/projects/worldwake/archive/specs/S37-cooldown-based-exhaustion.md)

## Problem

S37 changed the serialized `ExhaustionEntry` shape by replacing budget-halving state with cooldown state. That required the save/load format gate to advance so pre-change saves fail cleanly at the header boundary instead of drifting into opaque runtime deserialization failures.

## Assumption Reassessment (2026-03-29)

1. The exact boundary under audit is the persisted AI runtime payload crossing `worldwake-ai` -> `worldwake-sim` save/load. `AgentDecisionRuntime` owns the `exhaustion_cache` schema in [crates/worldwake-ai/src/decision_runtime.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs), while save-format version gating lives in [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs).
2. The ticket’s core implementation assumption was stale. Live code already has the delivered S37 shape: `ExhaustionEntry` now stores `next_retry_tick: Option<Tick>` and `consecutive_failures: u8` with `#[serde(default)]` in [crates/worldwake-ai/src/decision_runtime.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs), and the old `consecutive_budget_exhaustions` path is gone.
3. The save format bump is also already present. [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs) currently defines `SAVE_FORMAT_VERSION: u32 = 12`, retains `LEGACY_SAVE_FORMAT_VERSION: u32 = 5`, writes version `12` in `save_to_bytes()`, and rejects mismatched current-format versions in `load_from_bytes()`.
4. Focused save/load proof already exists. [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs) already contains `save_to_bytes_writes_current_format_version`, `load_rejects_wrong_version`, `load_rejects_previous_current_version_after_schema_bump`, and `save_to_bytes_roundtrip_preserves_runtime_payload`.
5. Focused cooldown/runtime proof also already exists. [crates/worldwake-ai/src/decision_runtime.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) contains cooldown factory and progression tests, [crates/worldwake-ai/src/agent_tick/planning.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) contains retry-eligibility and repeated-cooldown tests, and [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) contains `save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation`, which round-trips persisted runtime state containing exhaustion entries.
6. The spec narrative needed correction, not the code. The clean architecture here is explicit version gating, not old-current-format compatibility via field defaults. `#[serde(default)]` on the new cooldown fields is acceptable as same-version resilience inside the AI-owned runtime payload, but it is not the migration mechanism for stale saves because the project forbids backward-compatibility shim paths.
7. No mixed-layer golden narrative is driving this ticket. This is a narrow persistence-contract verification and archival closeout item.

## Architecture Check

1. The delivered architecture is better than the ticket’s original wording because it keeps responsibilities clean: `worldwake-ai` owns runtime schema shape and post-load validation, while `worldwake-sim` owns only envelope version gating and payload transport.
2. Explicit version rejection is cleaner than trying to deserialize stale current-format bytes through field defaults. That avoids aliasing old and new runtime meanings behind the same format number and matches the repository’s no-backward-compatibility rule.
3. No further implementation is warranted. Adding new migration logic now would make the architecture worse, not better.

## Verification Layers

1. Save header writes the live format version -> `save_load::tests::save_to_bytes_writes_current_format_version`
2. Save/load rejects stale current-format headers -> `save_load::tests::load_rejects_wrong_version` and `save_load::tests::load_rejects_previous_current_version_after_schema_bump`
3. Runtime payload survives round-trip through sim save/load envelope -> `save_load::tests::save_to_bytes_roundtrip_preserves_runtime_payload`
4. Persisted AI runtime with exhaustion entries survives round-trip and post-load validation -> `golden_harness::tests::save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation`
5. Cooldown-state semantics themselves remain correct after persistence -> focused `decision_runtime` and `agent_tick::planning` cooldown tests

## What Changed

1. Reassessed the ticket against live code and corrected the scope: no production-code changes were still outstanding.
2. Verified that the intended S37 save/load changes were already implemented and tested.
3. Closed this ticket as documentation/archival cleanup rather than duplicating already-landed code.

## Tests

### New/Modified Tests

1. None. Reassessment showed the required focused coverage already existed at the correct layers.

### Existing Tests Relied On

1. `save_load::tests::save_to_bytes_writes_current_format_version` -> proves the save header writes the bumped version.
2. `save_load::tests::load_rejects_wrong_version` -> proves obviously wrong current-format versions fail cleanly at the gate.
3. `save_load::tests::load_rejects_previous_current_version_after_schema_bump` -> proves the immediately previous current-format version is rejected after the schema bump, which is the real S37 regression surface.
4. `save_load::tests::save_to_bytes_roundtrip_preserves_runtime_payload` -> proves the sim envelope preserves AI runtime bytes.
5. `golden_harness::tests::save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation` -> proves AI runtime persistence, restoration, and validation across save/load with exhaustion-cache content present.
6. `agent_tick::planning::tests::record_exhausted_goals_doubles_cooldown_for_repeated_budget_retry_entries` -> proves repeated cooldown progression stays correct.
7. `agent_tick::planning::tests::has_pending_budget_retry_detects_retryable_budget_entries` -> proves cooldown eligibility drives retry behavior.

### Commands

1. `cargo test -p worldwake-sim save_load`
2. `cargo test -p worldwake-ai save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation -- --exact`
3. `cargo test -p worldwake-ai record_exhausted_goals_doubles_cooldown_for_repeated_budget_retry_entries -- --exact`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-29
- What actually changed:
  - No new production implementation was necessary during this turn because the `ExhaustionEntry` cooldown schema, `SAVE_FORMAT_VERSION` bump to `12`, focused persistence tests, and AI runtime round-trip coverage were already present in live code.
  - The real work was correcting the stale ticket narrative, aligning S37 documentation with the delivered architecture, and updating/archive-moving the ticket/spec/implementation-order records.
- Deviations from original plan:
  - The original ticket expected a fresh version bump and a new round-trip test. Reassessment showed both concerns were already addressed.
  - The original wording leaned too hard on `#[serde(default)]` as migration strategy. The delivered architecture correctly treats explicit version rejection as the canonical old-save behavior.
- Verification results:
  - Focused save/load and cooldown tests passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
