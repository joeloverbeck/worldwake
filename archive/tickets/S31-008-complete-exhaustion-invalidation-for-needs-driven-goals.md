# S31-008: Complete Exhaustion Invalidation for Needs-Driven Goals

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI exhaustion invalidation semantics, planner retry contract, persisted runtime shape, and focused proof coverage
**Deps**: S31-004, S31-005

## Problem

The live S31 substrate is only partially complete. Goal-aware invalidation already exists, but needs-driven goals still use a fixed `Permille(100)` drift heuristic while the planner also retains `EXHAUSTION_SKIP_TTL` as a second retry authority. That leaves exhaustion retry split between a state-grounded path and a timer path, and the needs-driven path is still not aligned with the actual candidate/ranking decision boundaries.

## Assumption Reassessment (2026-03-27)

1. The exact shared abstraction boundary under audit is the exhaustion-cache invalidation contract between [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs) and [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs): invalidation conditions determine when an exhausted goal entry is removed, and `build_candidate_plans()` only retries exhausted goals once that entry no longer blocks them.
2. The live code already shipped most of the S31 substrate. `ExhaustionEntry` already stores `invalidation_conditions` and `baseline` in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs), `derive_invalidation_conditions()` already maps every current `GoalKind`, and `invalidate_exhausted_goals()` already runs before planning.
3. The remaining needs-driven invalidation surface is still `ExhaustionInvalidationCondition::NeedCrossedThreshold { need, threshold_delta }` in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs). `condition_changed()` fires it only when `abs(current_need - baseline_need) >= threshold_delta`.
4. The current default threshold delta is still the fixed heuristic `Permille(100)` via `default_need_threshold_delta()` in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs). That is not the live decision surface for needs-driven goals.
5. The live candidate-generation boundary for the relevant goals is profile-driven threshold bands, not a fixed 100-permille delta. `emit_sleep_goal()`, `emit_relieve_goal()`, and `emit_wash_goal()` in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) gate on `DriveThresholds::{fatigue,bladder,dirtiness}.low()`. Ranking in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) classifies those same needs by threshold band for priority.
6. The relevant live `GoalKind` families under test are `ConsumeOwnedCommodity`, `Sleep`, `Relieve`, and `Wash`. The divergence is mixed-layer: candidate emission depends on threshold-band entry, ranking depends on threshold-band classification, and planner retry is still partially controlled by `EXHAUSTION_SKIP_TTL` in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs).
7. The current architecture still has two retry authorities: goal-aware invalidation and the TTL fallback. During implementation, an experimental TTL removal confirmed that broader retry behavior still depends on that fallback today: with the new need-band invalidation in place, `golden_wash_action` passed, but `golden_goal_invalidation_by_another_agent`, `golden_three_way_need_competition`, and `golden_utility_weight_diversity_in_need_selection` still regressed without TTL.
8. The intended invariant is not “retry after enough drift.” It is: retry when the concrete local planner decision surface changed enough that the previously exhausted goal crossed a real candidate or priority boundary, or another stored goal-specific invalidation condition fired.
9. The current `cargo test -p worldwake-ai -- --list` output confirms the ticket’s four golden acceptance tests still exist exactly as named in [`crates/worldwake-ai/tests/golden_ai_decisions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs): `golden_goal_invalidation_by_another_agent`, `golden_wash_action`, `golden_three_way_need_competition`, and `golden_utility_weight_diversity_in_need_selection`.
10. Adjacent contradiction exposed during reassessment: `ExhaustionEntry` still uses `#[serde(default)]` for `invalidation_conditions` and `baseline` in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs). Because this ticket now changes the canonical invalidation shape, that fallback should be removed in-scope rather than preserved as a backwards-compatibility alias path.
11. Under [`docs/FOUNDATIONS.md`](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md), the timer fallback is architecturally weaker than threshold-band invalidation because elapsed time is not a local planner fact. The implementation evidence for this ticket did not prove TTL removal safe yet, so the clean follow-up is to isolate the remaining non-needs retry dependency rather than pretending the timer path is already removable.
12. The strongest proof surface is mixed: focused `exhaustion` unit coverage for condition derivation and firing semantics, focused `agent_tick::planning` coverage for exhausted-goal skip semantics after TTL removal, and the existing golden scenarios for integrated retry behavior.

## Architecture Check

1. The clean solution is to make needs-driven exhaustion invalidation use the same profile-driven threshold bands that candidate generation and ranking already use, so the cache reflects the live planner decision surface instead of an unrelated scalar heuristic.
2. A fixed `Permille(100)` delta is architecturally weaker than threshold-band invalidation because it can miss true decision-boundary crossings and can also fire on large within-band drift that changed no planner boundary at all.
3. Keeping `EXHAUSTION_SKIP_TTL` after introducing threshold-band invalidation would preserve a second retry authority with weaker semantics. If the band-based model passes focused and golden proof, removing TTL in the same change is cleaner than carrying both paths.
4. No backwards-compatibility aliasing belongs here. If the invalidation condition model changes, the new shape should become the canonical persisted path and the old fallback/default path should be removed.

## Verification Layers

1. need-driven invalidation semantics for exhausted entries -> focused `exhaustion` unit coverage
2. exhaustion skip semantics remain unchanged in-scope -> existing `agent_tick::planning` coverage plus focused experimental validation during reassessment
3. candidate-generation/ranking decision-boundary alignment -> focused runtime or unit coverage at the strongest layer available
4. integrated retry behavior for the four accepted scenarios -> existing golden E2E coverage in [`crates/worldwake-ai/tests/golden_ai_decisions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs)
5. persisted runtime shape remains honest after invalidation-shape change -> focused save/load and runtime serialization coverage

## What to Change

### 1. Replace the fixed need-delta heuristic with threshold-band invalidation

For needs-driven exhausted goals, store the relevant `ThresholdBand` and invalidate when the baseline need and current need classify into different bands. The condition must be derived from the live agent profile, not from a hard-coded scalar delta.

### 2. Keep TTL out of scope for this ticket unless the full accepted proof surface goes green without it

Experimental removal on this ticket branch showed that broader retry behavior still depends on `EXHAUSTION_SKIP_TTL`. Do not remove it here without a stronger architectural explanation for those remaining regressions.

### 3. Make the persisted exhaustion shape canonical

Remove `#[serde(default)]` fallback loading for `ExhaustionEntry.invalidation_conditions` and `ExhaustionEntry.baseline`, and update the save format version if required by the final serialized shape change.

### 4. Add focused proof for the revised invalidation semantics

Add or strengthen focused `exhaustion` and save/load/runtime-serialization tests so the revised conditions are explicit, deterministic, and tied to concrete planner-visible state.

### 5. Prove the integrated regressions are gone

Use the four validated golden scenarios as required acceptance proof and keep `cargo test -p worldwake-ai` as the final AI-suite check.

## Files to Touch

- `crates/worldwake-ai/src/exhaustion.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-sim/src/save_load.rs` (modify if the persisted shape changes)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify only if the proof surface needs stronger assertions)

## Out of Scope

- Broad save-format migration work unrelated to the invalidation semantics
- Unrelated ranking or candidate-generation refactors beyond what this invalidation contract strictly requires

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_invalidation_by_another_agent -- --exact`
2. `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action -- --exact`
3. `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition -- --exact`
4. `cargo test -p worldwake-ai --test golden_ai_decisions golden_utility_weight_diversity_in_need_selection -- --exact`
5. `cargo test -p worldwake-ai`

### Invariants

1. A needs-driven exhausted goal is retried only when concrete local planner-relevant threshold-band state changed or another stored goal-specific invalidation condition fired
2. The revised invalidation semantics are deterministic and explainable from stored baseline state plus current local belief/runtime state
3. Persisted exhaustion entries deserialize only through the live canonical shape

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exhaustion.rs` — focused tests for threshold-band invalidation semantics
2. `crates/worldwake-ai/src/agent_tick/tests.rs` and/or `crates/worldwake-sim/src/save_load.rs` — focused persisted-runtime coverage if the serialized shape changes
3. [golden_ai_decisions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs) — keep the four existing regressions as required integrated proof

### Commands

1. `cargo test -p worldwake-ai condition_changed_need`
2. `cargo test -p worldwake-ai from_saved_runtime_restores_and_validates_driver_state`
3. `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_invalidation_by_another_agent -- --exact`
4. `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action -- --exact`
5. `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition -- --exact`
6. `cargo test -p worldwake-ai --test golden_ai_decisions golden_utility_weight_diversity_in_need_selection -- --exact`
7. `cargo test -p worldwake-ai`
8. `cargo clippy --workspace --all-targets -- -D warnings`
9. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-27
- Actual changes:
  - Replaced fixed `Permille(100)` needs invalidation with stored profile-driven `ThresholdBand` invalidation for `Sleep`, `Relieve`, and `Wash`.
  - Tightened the persisted exhaustion runtime shape by removing `#[serde(default)]` fallback loading for `ExhaustionEntry.invalidation_conditions` and `ExhaustionEntry.baseline`.
  - Bumped [`SAVE_FORMAT_VERSION`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs) from 7 to 8 to keep the new invalidation shape honest.
  - Added and updated focused `exhaustion` tests around boundary crossings, same-band drift, and live threshold-band derivation.
- Deviations from original plan:
  - `EXHAUSTION_SKIP_TTL` was intentionally not removed. Experimental removal still broke `golden_goal_invalidation_by_another_agent`, `golden_three_way_need_competition`, and `golden_utility_weight_diversity_in_need_selection`, so TTL cleanup remains a follow-up architectural task rather than a safe part of this ticket.
- Verification results:
  - `cargo test -p worldwake-ai condition_changed_need`
  - `cargo test -p worldwake-ai from_saved_runtime_restores_and_validates_driver_state`
  - `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_invalidation_by_another_agent -- --exact`
  - `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action -- --exact`
  - `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition -- --exact`
  - `cargo test -p worldwake-ai --test golden_ai_decisions golden_utility_weight_diversity_in_need_selection -- --exact`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
