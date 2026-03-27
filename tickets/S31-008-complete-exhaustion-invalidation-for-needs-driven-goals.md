# S31-008: Complete Exhaustion Invalidation for Needs-Driven Goals

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI exhaustion invalidation semantics, planner retry contract, and focused proof coverage
**Deps**: S31-004, S31-005

## Problem

The current S31 invalidation substrate is not complete enough to replace the planner’s TTL retry fallback. Existing goldens show that some exhausted needs-driven goals can become worth retrying without any current invalidation condition firing. Until that contradiction is fixed, removing `EXHAUSTION_SKIP_TTL` would reintroduce indefinite caching.

## Assumption Reassessment (2026-03-27)

1. The exact shared abstraction boundary under audit is the exhaustion-cache invalidation contract between `crates/worldwake-ai/src/exhaustion.rs` and `crates/worldwake-ai/src/agent_tick/planning.rs`: invalidation conditions determine when an exhausted goal entry is removed, and `build_candidate_plans()` only retries exhausted goals once that entry no longer blocks them.
2. The current needs-driven invalidation surface is `ExhaustionInvalidationCondition::NeedCrossedThreshold { need, threshold_delta }` in `crates/worldwake-ai/src/exhaustion.rs`. `condition_changed()` currently fires that condition only when `abs(current_need - baseline_need) >= threshold_delta`.
3. The current default threshold delta is `Permille(100)` via `default_need_threshold_delta()` in `crates/worldwake-ai/src/exhaustion.rs`.
4. The live candidate-generation decision surface for the relevant goals is not “100 permille change.” `emit_sleep_goal()`, `emit_relieve_goal()`, and `emit_wash_goal()` in `crates/worldwake-ai/src/candidate_generation.rs` are driven by `DriveThresholds::* .low()`, and other needs-driven goal competition is governed by the full ranking substrate rather than a fixed scalar delta.
5. The live failing golden scenarios prove the contradiction sits in production semantics, not only in coverage. After experimentally removing TTL, `cargo test -p worldwake-ai` failed: `golden_goal_invalidation_by_another_agent`, `golden_wash_action`, `golden_three_way_need_competition`, and `golden_utility_weight_diversity_in_need_selection`.
6. The relevant live `GoalKind` families under test are `ConsumeOwnedCommodity`, `Sleep`, and `Wash`, and the divergence is a mixed-layer combination of candidate-generation thresholds, ranking/priority behavior, and planner exhaustion retry. This is not a pure search-level or pure candidate-generation-only ticket.
7. The intended invariant is not “retry after enough drift.” It is: retry when the concrete local planner decision surface changed enough that the previously exhausted search could now lawfully produce a materially different result.
8. Under `docs/FOUNDATIONS.md`, any replacement for TTL must be grounded in concrete local state rather than abstract elapsed time or a magic periodic retry. That rules out solving this by merely changing the TTL value or adding another timer-shaped fallback.
9. Adjacent contradiction exposed during reassessment: `ExhaustionEntry` still uses `#[serde(default)]` for `invalidation_conditions` and `baseline`. That save-format cleanup is real but separate; it should not be mixed into this ticket unless the final invalidation design requires a runtime-shape update.
10. The strongest currently available proof surface is mixed: focused `exhaustion` unit coverage for condition derivation and firing semantics, plus the existing golden scenarios for the integrated retry behavior. If those traces remain insufficient to explain the new retry semantics, a follow-up traceability ticket may be needed, but this ticket should first restore correctness at the strongest production-facing layer.

## Architecture Check

1. The clean solution is to make exhaustion invalidation model concrete planner-relevant state changes directly, so the cache reflects the same local decision boundaries the planner actually uses. That aligns with Principles 2, 3, 19, 25, 26, and 27 in `docs/FOUNDATIONS.md`.
2. A fixed `Permille(100)` delta is too generic to stand in for all needs-driven retry conditions. It is a heuristic standing in for missing planner-visible substrate. This ticket exists to replace that heuristic with a cleaner contract rather than tuning it blindly.
3. The desired end-state is: once this ticket lands, `S31-006` can remove the TTL fallback without reopening regressions, and the exhaustion cache has one concrete, explainable invalidation path.
4. No backwards-compatibility aliasing belongs here. If a richer invalidation condition model is needed, it should become the canonical live path.

## Verification Layers

1. need-driven invalidation semantics for exhausted entries -> focused `exhaustion` unit coverage
2. candidate-generation/ranking decision-boundary alignment -> focused runtime or unit coverage at the strongest layer available
3. integrated retry behavior for previously failing scenarios -> existing golden E2E coverage in `golden_ai_decisions.rs`
4. future TTL-removal readiness -> `cargo test -p worldwake-ai` with `S31-006` still blocked until this ticket passes

## What to Change

### 1. Reassess the needs-driven invalidation model against live decision boundaries

Audit each needs-driven exhausted goal family and identify what concrete local facts actually make a previously exhausted search materially different under current code. Do not assume a fixed-delta rule is sufficient just because it is easy to store.

### 2. Replace or refine the current need invalidation condition model

Implement an invalidation contract that tracks the real planner-visible state changes for needs-driven retries. The likely direction is threshold-band or decision-surface-aware invalidation rather than a hard-coded `Permille(100)` delta, but the final shape should be chosen only after the audit in step 1 proves it matches live candidate/ranking semantics.

### 3. Add focused proof for the revised invalidation semantics

Add or strengthen focused `exhaustion` tests so the revised conditions are explicit, deterministic, and tied to concrete planner-visible state.

### 4. Prove the integrated regressions are gone before unblocking S31-006

Use the existing golden scenarios as required acceptance proof. `S31-006` should remain blocked until they pass with the revised invalidation substrate still in place.

## Files to Touch

- `crates/worldwake-ai/src/exhaustion.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (only if needed to support the richer invalidation contract without removing TTL yet)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify only if the proof surface needs stronger assertions)

## Out of Scope

- Removing `EXHAUSTION_SKIP_TTL` directly
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

1. A needs-driven exhausted goal is retried only when concrete local planner-relevant state changed enough to make the search space materially different
2. No new time-based fallback or second retry authority is introduced
3. The revised invalidation semantics are deterministic and explainable from stored baseline state plus current local belief/runtime state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exhaustion.rs` — focused tests for the revised needs-driven invalidation semantics
2. [golden_ai_decisions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs) — keep the four existing regressions as required integrated proof

### Commands

1. `cargo test -p worldwake-ai exhaustion`
2. `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action -- --exact`
3. `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition -- --exact`
4. `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_invalidation_by_another_agent -- --exact`
5. `cargo test -p worldwake-ai --test golden_ai_decisions golden_utility_weight_diversity_in_need_selection -- --exact`
6. `cargo test -p worldwake-ai`
