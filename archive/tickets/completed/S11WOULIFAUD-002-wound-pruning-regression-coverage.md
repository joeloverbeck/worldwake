# S11WOULIFAUD-002: Wound pruning regression coverage for zero-recovery and no-change paths

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: `specs/S11-wound-lifecycle-audit.md`, `crates/worldwake-systems/src/combat.rs`, `crates/worldwake-ai/tests/golden_combat.rs`, `crates/worldwake-ai/tests/golden_care.rs`

## Problem

The ticket originally assumed the wound-pruning contract was effectively untested and that a production-side debug assertion was needed. Current code and tests show the opposite in part: the authoritative prune predicate in `progress_wounds()` is already explicit and there is existing focused plus golden lifecycle coverage. The real gap is narrower: there is no focused regression coverage for the exact `natural_recovery_rate: pm(0)` persistence path, the mixed-list prune boundary, or the `None` return contract when wound state is static.

## Assumption Reassessment (2026-03-21)

1. `progress_wounds()` in [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs) is still private and authoritative for passive wound progression. It explicitly prunes with `retain(|wound| wound.severity.value() > 0)`, so the prune contract is already encoded directly in production code rather than hidden behind indirect structure.
2. The key recovery gate is still `recovery_conditions_met()` in [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs): no combat engagement and hunger/thirst/fatigue each below their `high` thresholds. With `natural_recovery_rate == pm(0)`, the recovery branch is a no-op because `Permille::saturating_sub(pm(0))` cannot reduce severity.
3. Existing focused/unit coverage already exercises adjacent lifecycle behavior in [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs): `combat_system_progresses_bleeding_wounds_and_applies_clotting`, `non_bleeding_wounds_recover_when_physiology_is_tolerable`, `recovery_is_blocked_during_active_combat_domain_actions`, `recovery_is_blocked_when_physiology_exceeds_tolerable_thresholds`, and `healed_wounds_are_removed_from_wound_list`.
4. Existing golden/E2E coverage also exists. [`crates/worldwake-ai/tests/golden_combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs) contains `golden_wound_bleed_clotting_natural_recovery`, which proves the full bleed -> clot -> recovery -> prune lifecycle, and [`crates/worldwake-ai/tests/golden_care.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs) contains `golden_care_pre_start_wound_disappearance_records_blocker`, which covers the separate AI/runtime failure case where a wound disappears before a care action starts.
5. The spec/ticket claim that the `no_recovery_combat_profile()` workaround was removed is wrong for the current repo. The helper still exists in [`crates/worldwake-ai/tests/golden_harness/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) and is still used by multiple goldens in [`crates/worldwake-ai/tests/golden_emergent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs) and [`crates/worldwake-ai/tests/golden_combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs). That means the correct conclusion is not "the anomaly source is gone", but "the engine path still needs tighter focused regression coverage around zero-recovery wounds."
6. This is not an AI-pipeline change ticket. The intended verification layer is focused systems/unit coverage around `progress_wounds()`, with existing goldens named as higher-level regression backstops rather than the primary proof surface.
7. No ordering contract is involved. The invariant is authoritative world-state progression inside one pure helper, not action lifecycle ordering, event-log ordering, or decision-trace ordering.
8. Mismatch correction: a production `debug_assert!` does not buy meaningful architectural safety here. The current prune predicate already states the contract directly; adding a debug assertion that rephrases that predicate or bakes in speculative bleed assumptions would add noise without strengthening the design. The missing architecture is focused proof, not extra runtime branching.

## Architecture Check

1. The cleanest approach is to leave `progress_wounds()` production logic unchanged and strengthen its focused tests. That preserves a simple authoritative helper with one obvious prune rule instead of layering on redundant assertions that future code would need to interpret around.
2. No backwards-compatibility aliasing or shims are introduced. This ticket only closes a focused verification gap around an existing contract.

## Verification Layers

1. `natural_recovery_rate: pm(0)` does not silently reduce or prune a clotted wound -> focused unit test calling `progress_wounds()` directly
2. Bleed/clot arithmetic remains exact before recovery is allowed -> focused unit test calling `progress_wounds()` directly over repeated ticks
3. Only zero-severity wounds are removed from a mixed list -> focused unit test calling `progress_wounds()` directly
4. Static wound state returns `None` instead of spuriously emitting a changed list -> focused unit test calling `progress_wounds()` directly
5. Additional layer mapping is not needed because this ticket does not change AI planning or action execution; existing goldens remain named regression backstops, not the primary proof surface

## What to Change

### 1. Add focused `progress_wounds()` regression tests

Add focused tests in the [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs) test module that call `progress_wounds()` directly with controlled `WoundList`, `CombatProfile`, needs, and thresholds.

Required cases:

- `zero_recovery_rate_wound_persists`: clotted wound with non-zero severity and `natural_recovery_rate: pm(0)` stays unchanged across repeated progression steps and is never pruned
- `wound_bleed_clot_arithmetic_exact`: bleed/clot progression reaches the exact expected severity and bleed-rate sequence
- `pruning_only_at_severity_zero`: mixed-severity wound list prunes only the wound already at zero severity
- `progress_wounds_returns_none_when_no_change`: static non-bleeding wound with `pm(0)` recovery returns `None`

### 2. Do not change `progress_wounds()` production semantics

Keep the helper’s production logic as-is unless focused TDD exposes a real contradiction. This ticket’s reassessment says the correct architectural move is proof hardening, not a new assertion layer.

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (modify tests only unless TDD proves a production contradiction)

## Out of Scope

- Changing the prune predicate in `progress_wounds()`
- Adding a production `debug_assert!` that duplicates the existing prune rule
- Changing `recovery_conditions_met()` logic
- Any changes to `crates/worldwake-core/src/wounds.rs`
- Any changes to `crates/worldwake-systems/src/needs.rs`
- Any AI/ranking or golden-harness changes

## Acceptance Criteria

### Tests That Must Pass

1. `zero_recovery_rate_wound_persists` proves a clotted wound with `pm(0)` recovery survives repeated progression without severity loss
2. `wound_bleed_clot_arithmetic_exact` proves the exact bleed/clot arithmetic path
3. `pruning_only_at_severity_zero` proves only zero-severity wounds are removed
4. `progress_wounds_returns_none_when_no_change` proves the helper emits no spurious update for static wound state
5. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. A wound is pruned if and only if its post-progression severity is zero
2. A wound with `natural_recovery_rate: pm(0)` cannot have its severity reduced by the passive recovery branch
3. `progress_wounds()` returns `None` when neither any wound field nor wound-list membership changes

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs) `zero_recovery_rate_wound_persists` — closes the exact zero-recovery regression gap that existing focused and golden coverage do not isolate
2. [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs) `wound_bleed_clot_arithmetic_exact` — locks in the exact arithmetic contract independently of higher-level system setup
3. [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs) `pruning_only_at_severity_zero` — proves the prune boundary directly on a mixed wound list
4. [`crates/worldwake-systems/src/combat.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs) `progress_wounds_returns_none_when_no_change` — proves the helper’s no-op return contract so downstream systems do not write spurious component updates

### Commands

1. `cargo test -p worldwake-systems zero_recovery_rate_wound_persists`
2. `cargo test -p worldwake-systems wound_bleed_clot_arithmetic_exact`
3. `cargo test -p worldwake-systems pruning_only_at_severity_zero`
4. `cargo test -p worldwake-systems progress_wounds_returns_none_when_no_change`
5. `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
6. `cargo test -p worldwake-systems`

## Outcome

- Completed: 2026-03-21
- What changed:
  - Reassessed the ticket against the current repo and corrected two stale assumptions: `no_recovery_combat_profile()` still exists in the golden harness and existing wound lifecycle coverage was already broader than the original ticket claimed.
  - Narrowed scope from "production debug assertion plus tests" to "focused regression coverage only" because `progress_wounds()` already expresses the prune contract directly and did not need additional runtime assertion noise.
  - Added four focused `progress_wounds()` tests in `crates/worldwake-systems/src/combat.rs`: `zero_recovery_rate_wound_persists`, `wound_bleed_clot_arithmetic_exact`, `pruning_only_at_severity_zero`, and `progress_wounds_returns_none_when_no_change`.
- Deviations from original plan:
  - No production `debug_assert!` was added. After reassessment and TDD, the helper logic remained unchanged because the architecture was already clean and the missing piece was proof coverage.
  - `Engine Changes` was corrected from `Yes` to `None`.
- Verification results:
  - `cargo test -p worldwake-systems zero_recovery_rate_wound_persists`
  - `cargo test -p worldwake-systems wound_bleed_clot_arithmetic_exact`
  - `cargo test -p worldwake-systems pruning_only_at_severity_zero`
  - `cargo test -p worldwake-systems progress_wounds_returns_none_when_no_change`
  - `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
  - `cargo test -p worldwake-systems`
