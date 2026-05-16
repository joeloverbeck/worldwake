# S145PLASUBHAR-005: Golden — 5-stage production chain budget scaling

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: No (golden test only; tests the formula change shipped by ticket 001)
**Deps**: S145PLASUBHAR-001

## Problem

S145 D1 (ticket 001) changes the strategic budget formula from `usize::max(1, max_prerequisite_locations * 2)` to `2 * stages.max(1) * max_prerequisite_locations`. The narrative behavior change is "a 5-stage production chain now completes where it timed out under the old `* 2` formula." Per S145's Test Plan, a golden test must concretely prove this — without it, ticket 001's formula change is verified only at the unit level (the per-stage value is correct) and not at the end-to-end planner level (a 5-stage chain actually completes under the new budget). This ticket lands the golden as a P12-type performance-regression guard with explicit metric thresholds.

## Assumption Reassessment (2026-05-16)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Existing golden coverage in `crates/worldwake-ai/tests/golden_two_phase_planning.rs` and `golden_planner_pathology.rs` exercises the two-phase strategic + tactical planner but does not include a scenario with a 5+ stage acquisition chain that would have exhausted the pre-S145 budget. The new golden `golden_strategic_budget_scaling.rs` is net-new coverage. Verification at `cargo test -p worldwake-ai -- --list` will confirm there is no existing test of this name before ticket implementation.
2. The strategic search's exhaustion path at `crates/worldwake-ai/src/search/strategic.rs:128-130` returns `None` rather than a typed `PlanSearchResult::BudgetExhausted` — per S145 reassessment finding M4 and S145 Non-Goals, this is by design. The golden must therefore assert against the downstream observable outcome: a successful `PlanSearchResult::Found` (with a complete plan covering all 5 stages) under the new formula, contrasted against the spec's claim that the pre-S145 formula timed out.
3. Live GoalKind under test: a multi-stage `ProduceCommodity` or equivalent production-chain goal whose strategic stage decomposition yields ≥5 stages. The scenario must verify (via `build_stages` at `strategic.rs:88-96`) that the test scenario's substrate genuinely produces a 5-stage decomposition under default `max_prerequisite_locations = 3`; otherwise the golden tests a different invariant than claimed. The scenario design step should grep `build_stages` to confirm what scenario shape produces deep stage chains, rather than assuming the spec's "PR-2 typical production chain" framing maps directly to existing scenarios.
4. Scenario isolation: the golden scenario must isolate the 5-stage production chain branch from lawful competing affordances (per `docs/precision-rules.md` Rule 8). The intended branch under test is "strategic search completes the 5-stage chain"; lawful competing affordances that would distract the planner (e.g., shorter alternative goals, immediate-satisfaction options for survival needs) must be intentionally excluded from the scenario's setup, with the exclusion documented in the test rationale.

## Architecture Check

1. The golden's primary assertion is on the strategic planner's *capability* (5-stage chains complete) rather than on a specific plan content. This keeps the golden robust to legitimate future planner improvements (e.g., alternative stage orderings, ranking adjustments) that change the specific plan steps but preserve completion.
2. Per the P12-type performance-regression-guard pattern (CLAUDE.md FND-12 and `/spec-to-tickets` constraint), the golden also includes a metric threshold: the strategic search expansions used (`PlanAttemptTrace.strategic_budget.budget_used` from ticket 002, when available) or equivalent deterministic counter must remain at or below the formula-derived budget total. This catches regressions where a future change reverts the formula to the per-stage-unaware shape but the completion assertion still passes by accident.

## Verification Layers

1. Strategic planner completes a 5-stage chain → assert `PlanSearchResult::Found` with `steps.len() >= 5` (or equivalent: terminal kind, plan depth) at the golden's outer-result surface.
2. Strategic budget is consumed proportionally to stage count → assert recorded `budget_used <= budget_total` where `budget_total == ExecutionBudget::strategic_budget_for_stages(stages_count)` from ticket 001. Performance-regression guard.
3. Decision trace surface: if ticket 002's `StrategicBudgetTrace` is available at test-write time (i.e., 002 has already merged), assert `strategic_budget` is `Some` with `exhausted: false` on the test agent's `PlanAttemptTrace`. If ticket 002 has not yet merged, omit this assertion and rely on the outer plan-result surface only — the golden remains the strongest available proof surface for the formula change (per `docs/precision-rules.md` Rule 15, the immediate proof at the strongest available lower layer is sufficient).
4. The golden does not need an action-trace or event-log layer because the contract under test is planning-layer (strategic search) only; no authoritative world state mutates as part of the proof.

## What to Change

### 1. Author a new golden scenario for the 5-stage production chain

Create `crates/worldwake-ai/tests/scenarios/strategic-budget-scaling.ron` (or reuse an existing 5-stage scenario if one exists — grep `scenarios/*.ron` first). The scenario must produce a strategic stage decomposition of length ≥5 under default `ExecutionBudget`. Validate the stage count by reading the strategic search trace during a dry-run, not by assuming the scenario shape; if the chain length is shorter than 5, deepen the scenario (additional intermediate commodity tiers, additional prerequisite-location requirements) until `build_stages` yields ≥5 stages.

### 2. Author `golden_strategic_budget_scaling.rs`

Create `crates/worldwake-ai/tests/golden_strategic_budget_scaling.rs`:

```rust
//! Golden: 5-stage production chain strategic budget scaling.
//!
//! S145 D1 changes the strategic budget formula from `max(1, ml * 2)` to
//! `2 * stages.max(1) * ml`. A 5-stage production chain under default
//! `max_prerequisite_locations = 3` receives 30 expansions under the new
//! formula vs. 6 under the old, and this test proves the chain completes.

#[test]
fn five_stage_production_chain_completes_under_stage_aware_budget() {
    // 1. Load the strategic-budget-scaling scenario.
    // 2. Run the planner for the production-chain goal-bearing agent.
    // 3. Assert the resulting PlanSearchResult::Found with plan steps
    //    covering all 5 stages.
    // 4. Performance-regression guard: assert recorded expansions used
    //    <= strategic_budget_for_stages(5) == 30 (under default execution
    //    budget). If StrategicBudgetTrace from ticket 002 is available,
    //    assert strategic_budget.exhausted == false on the agent's
    //    PlanAttemptTrace.
}
```

The exact assertion shape depends on the golden harness conventions in `tests/golden_harness/` — follow whichever harness pattern existing two-phase planner goldens use (`golden_two_phase_planning.rs` is the closest precedent).

### 3. Register the new test file in the workspace test discovery

The Cargo integration-test convention is that any `*.rs` file in `tests/` is automatically discovered; no `Cargo.toml` change is typically needed. Verify discovery via `cargo test -p worldwake-ai --test golden_strategic_budget_scaling -- --list` after authoring.

## Files to Touch

- `crates/worldwake-ai/tests/golden_strategic_budget_scaling.rs` (new — golden test entry point)
- `crates/worldwake-ai/tests/scenarios/strategic-budget-scaling.ron` (new — scenario file; path is illustrative, confirm against `scenarios/` convention during ticket reassessment)
- Likely: `crates/worldwake-ai/tests/golden_harness/<helper>` (modify — if existing harness needs a small helper for production-chain assertions, follow the convention of existing two-phase planner goldens; confirm path during reassessment)

## Out of Scope

- No change to the strategic search budget formula — that is ticket 001.
- No change to `StrategicBudgetTrace` surface — that is ticket 002. This golden adapts to whichever surface is available at test-write time.
- No deeper-than-5-stage scenarios — S145's narrative cites "PR-2 typical production chain (8 stages)" as an aspirational target, but the spec's Test Plan only requires the 5-stage proof.
- No tactical-side budget validation — strategic phase is the contract under test.

## Acceptance Criteria

### Tests That Must Pass

1. New `five_stage_production_chain_completes_under_stage_aware_budget` passes: the 5-stage chain completes with `PlanSearchResult::Found` and a plan covering all 5 stages.
2. Performance-regression guard: recorded strategic-search expansions used remain ≤ `strategic_budget_for_stages(stages_count)` for the test scenario's stage count.
3. Existing two-phase planner goldens (`golden_two_phase_planning.rs`, `golden_planner_pathology.rs`) continue to pass unchanged.
4. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. The test asserts plan *completion* (not specific plan content) so legitimate future planner improvements that change plan shape but preserve completion do not break the regression guard.
2. The test's metric threshold (`budget_used <= strategic_budget_for_stages(stages_count)`) is a deterministic logical count, not wall-clock time, per CLAUDE.md determinism invariants and `/spec-to-tickets`'s P12-spec performance-guard rule.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_strategic_budget_scaling.rs` (new) — 5-stage production chain golden proving the new formula's chain-completion behavior.

### Commands

1. `cargo test -p worldwake-ai --test golden_strategic_budget_scaling`
2. `cargo test -p worldwake-ai`
3. `scripts/verify.sh`
