# S39LIMSIDBEN-001: Add side-benefit scoring substrate and per-agent weight

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `UtilityProfile` field and new AI-side side-benefit scoring substrate
**Deps**: [specs/S39-limited-side-benefit-plan-scoring.md](/home/joeloverbeck/projects/worldwake/specs/S39-limited-side-benefit-plan-scoring.md)

## Problem

`S39` requires a lightweight post-search side-benefit overlay, but the live codebase has neither a per-agent side-benefit weight on `UtilityProfile` nor a pure scoring/detection module that can evaluate secondary destination benefits without changing planner search. Until that substrate exists, later plan-selection and golden work would have to duplicate arithmetic or mix policy directly into `select_best_plan()`.

## Assumption Reassessment (2026-04-02)

1. The per-agent weighting surface named by the spec is still missing. [`UtilityProfile`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/utility_profile.rs) currently exposes only hunger/thirst/fatigue/bladder/dirtiness/pain/danger/enterprise/social/activity-awareness/courage/care weights; there is no `side_benefit_weight`.
2. The repo already stores `UtilityProfile` authoritatively and uses sample fixtures widely. [`sample_utility_profile()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/test_utils.rs) and the bare-type fixture sites in [`component_tables.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_tables.rs), [`world.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs), [`delta.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/delta.rs), and [`world_txn.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs) all already construct `UtilityProfile`, so this ticket must keep the field addition coherent there.
3. The side-benefit substrate can stay read-only and post-search. [`RankedGoal`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) already carries the needed primary inputs: `grounded.key`, `grounded.anchor`, `priority_class`, and `motive_score`.
4. There is no existing side-benefit module or reusable helper in `worldwake-ai`; [`plan_selection.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs) currently compares only `(priority_class, motive_score, total_estimated_ticks, steps, goal)` and should not absorb new arithmetic until the pure substrate exists.
5. The live `GoalKind` surface this spec relies on is still opportunity-anchored goal selection rather than multi-goal search. This ticket intentionally stops at pure detection/arithmetic and does not yet change any planner operator, affordance, or selection behavior.
6. This is a single-layer substrate ticket. Mixed-layer verification mapping is not yet applicable because no runtime selection, trace, or golden contract changes land here.
7. Mismatch + correction: the spec describes `detect_side_benefits()` as scanning plan travel targets and ranked candidates, but no such helper exists today. This ticket owns creating that pure helper and the per-agent weight field; it does not yet wire the result into `select_best_plan()`.

## Architecture Check

1. Adding a dedicated pure substrate first is cleaner than embedding side-benefit arithmetic directly in [`select_best_plan()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs) because it keeps destination scanning, deduplication, capping, and value math testable without planner-runtime noise.
2. No backwards-compatibility shims are introduced. `UtilityProfile` grows one new canonical field, and the new side-benefit module becomes the only intended place for side-benefit detection logic.

## Verification Layers

1. `UtilityProfile.side_benefit_weight` defaults and roundtrips correctly -> focused `worldwake-core` unit tests on [`utility_profile.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/utility_profile.rs)
2. Existing representative utility-profile fixtures remain coherent after the field addition -> focused `worldwake-core` authoritative storage tests on existing `UtilityProfile` component coverage
3. Side-benefit detection excludes the primary goal, honors place matching, caps counts, and computes bounded values deterministically -> focused pure tests in the new AI-side scoring module
4. Additional layer mapping is not applicable yet because this ticket does not change plan selection, decision traces, or executable behavior

## What to Change

### 1. Extend `UtilityProfile` with side-benefit weighting

Add `side_benefit_weight: Permille` to [`utility_profile.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/utility_profile.rs) with the spec default, and update sample/fixture construction so existing authoritative-storage tests continue to build real profiles.

### 2. Add a pure side-benefit scoring module in `worldwake-ai`

Create a new AI-side module that defines the internal `SideBenefit` and `PlanValue` structures plus pure helpers for:
- extracting distinct visited places from a `PlannedPlan`
- detecting secondary candidates whose target place appears on that path
- excluding the primary goal from self-counting
- capping benefit count and total value using the spec’s bounded arithmetic

### 3. Add focused pure tests for the substrate

Cover the exact detection and arithmetic rules this ticket owns before any selection integration lands: place-match detection, no-primary self-counting, capped secondary count, weighted value math, and bounded total-value math.

## Files to Touch

- `/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/utility_profile.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/test_utils.rs` (modify)
- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/side_benefit.rs` (new)
- `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/lib.rs` (modify)

## Out of Scope

- Integrating side-benefits into [`select_best_plan()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs)
- Extending [`SelectionTrace`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) or planner summaries
- Any golden/E2E scenario proving side-benefit-driven route or market behavior

## Acceptance Criteria

### Tests That Must Pass

1. `UtilityProfile` includes `side_benefit_weight` with the spec default and still roundtrips through existing serde/bincode coverage.
2. Pure side-benefit detection returns only non-primary candidates whose target places appear on the plan path and remains deterministic under stable input ordering.
3. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. This ticket does not change planner search, goal generation, or plan-selection behavior; it only adds a reusable post-search scoring substrate.
2. `side_benefit_weight = Permille(0)` must yield zero side-benefit contribution without requiring special-case alternate code paths elsewhere.

## Test Plan

### New/Modified Tests

1. `/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/utility_profile.rs` — default and roundtrip coverage for the new `side_benefit_weight` field.
2. `/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/side_benefit.rs` — pure detection/arithmetic coverage for side-benefit scanning, capping, and bounded total-value math.

### Commands

1. `cargo test -p worldwake-core utility_profile::tests::utility_profile_roundtrips_through_bincode -- --exact --nocapture`
2. `cargo test -p worldwake-ai --lib`
3. `cargo test -p worldwake-core`
