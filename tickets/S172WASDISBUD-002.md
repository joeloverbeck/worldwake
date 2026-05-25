# S172WASDISBUD-002: Close Wash carve-out in survival-scattered

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: specs/S172-wash-discovery-budget-closure.md (D3 scattered portion, D5 distributed)

## Problem

`crates/worldwake-ai/tests/scenarios/survival_scattered.rs` carries a Wash carve-out at `is_budget_checked_survival_goal` (lines 116-120) and a re-citation comment at lines 462-466, both pointing at tracking ID `GOAPTRVLSCAL-001`. Unlike contested, the scattered `.ron` already authors Wash into `survival_health_contract.required_self_care_families` at line 19 — the scenario contract already says Wash matters. The discrepancy is test-side only: the budget-exhaustion check filters Wash out even though the contract requires Wash exercising. S172 D3 (scattered portion) requires the test-side filter to be widened so Wash is uniformly budget-checked under scattered topology, completing the contract enforcement the `.ron` already specifies.

## Assumption Reassessment (2026-05-25)

1. The carve-out exists at `crates/worldwake-ai/tests/scenarios/survival_scattered.rs:116-120` (`is_budget_checked_survival_goal`) consumed by `no_budget_exhaustion_on_survival_goals` at line 472. A re-citation comment lives at lines 462-466. Existing tests in this file: `all_agents_survive_1440_ticks:362`, `all_agents_perform_survival_actions:403`, `isolated_agent_reaches_food_source:438`, `no_budget_exhaustion_on_survival_goals:472` (carve-out consumer), `no_stuck_idle_windows_with_elevated_needs:503`, `seeded_target_location_belief_decays_to_stale_without_refresh:547`. The `.ron` already includes Wash at `scenarios/survival-scattered.ron:19`.
2. S172 Deliverable 3 (`specs/S172-wash-discovery-budget-closure.md`) requires the test-side filter widening for scattered. Deliverable 5 commits to option (3) Reuse with the goal-key-join inspection convention.
3. Shared abstraction boundary: the AI-test-side filter `is_budget_checked_survival_goal` is the only surface that diverges from the contract. The `.ron` contract is already correct.
4. Intended invariant: every agent must lawfully exercise Wash under scattered topology within the run window OR emit a lawful D5 failure-attribution surface — uniformly with the other four self-care goals.
5. Live `GoalKind` under test: `GoalKind::Wash`; current operator surface `WASH_OPS = [PlannerOpKind::Wash, PlannerOpKind::Travel]` at `crates/worldwake-ai/src/goal_schema.rs:101`; `GoalPlanningBudget::SELF_CARE` at `goal_schema.rs:374`.
6. AI regression layer: full action registries required (scattered exercises multi-place travel + needs). Local needs-only harness insufficient.
13. Adjacent contradictions: same conditional risk as ticket 001 — if the planner gap remains, this ticket surfaces it. Treat as required consequence per S172 D5; open follow-up planner-fix ticket if needed; do not weaken this ticket's invariant to mask it.

## Architecture Check

1. Smaller than 001 because the `.ron` is already correct — the ticket is a pure test-side filter widening. No contract change is needed.
2. No backwards-compatibility aliasing: the GOAPTRVLSCAL-001 comment block is removed, not retained.
3. FND-31 alignment: with the filter widened, the test contract matches the scenario contract; no silent escape path.

## Verification Layers

1. Wash budget-check enforcement → focused test `no_budget_exhaustion_on_survival_goals` at line 472 must pass with the widened filter; if Wash exhausts budget, the failure exposes either (a) a still-live planner regression (separate ticket) or (b) a lawful D5 budget-exhausted branch with recovery.
2. Wash commit attribution → action trace + `DecisionEventPayload::WashFacilityUsed` at `crates/worldwake-core/src/decision_event_payload.rs:79`.
3. Same goal-key-join convention as ticket 001 — filter on `goal_key == GoalKind::Wash` + the existing generic cause.
4. Single-layer surfaces are sufficient here because the scattered topology already separates resources across places; the failure-attribution surface differentiation (commit vs. budget-exhausted vs. revalidation-disconfirmed) maps cleanly per D5.

## What to Change

### 1. Remove the Wash carve-out from `is_budget_checked_survival_goal`

`crates/worldwake-ai/tests/scenarios/survival_scattered.rs` (lines 116-120): delete the doc-comment block citing `GOAPTRVLSCAL-001` and rewrite `is_budget_checked_survival_goal` to:

```rust
fn is_budget_checked_survival_goal(goal: &GoalKind) -> bool {
    is_survival_goal(goal)
}
```

Remove the re-citation comment at lines 462-466. If `is_survival_goal` does not include `GoalKind::Wash`, add it.

### 2. Verify `no_budget_exhaustion_on_survival_goals` covers Wash

`crates/worldwake-ai/tests/scenarios/survival_scattered.rs:472`: the test consumer of `is_budget_checked_survival_goal` now sees Wash attempts in its loop. If the test passes unchanged, no further edit is required. If it fails, document the lawful D5 failure branch and either (a) surface the planner regression as a separate ticket per S172 Risk #1, or (b) document the lawful budget-exhausted branch with recovery.

### 3. Add a Wash-commit positive assertion

Add a `#[test]` `wash_is_exercised_in_scattered_topology` or extend `all_agents_perform_survival_actions` (line 403) to filter the decision-event log for at least one `DecisionEventPayload::WashFacilityUsed` per dirty agent within the run window. Asserts payload presence (not specific values).

### 4. Failure-attribution surface assertion (D5)

Same shape as ticket 001 — filter on `goal_key == GoalKind::Wash` AND one of:
- `PlanSearchOutcome::BudgetExhausted` from `decision_trace.rs:1393` via `SelectionTrace.selected_opportunity`, OR
- `RevalidationOutcome::Invalidated { reason: PlanInvalidationReason::ExpectationMismatch, mismatch_detail }` from `plan_revalidation.rs:17` via `ExpectationMismatchPayload.goal_key` at `decision_event_payload.rs:365`.

Silent absence (no Wash candidate emitted) is lawful per D5's no-candidate branch — assertable via `CandidateGenerationDiagnostics`.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/survival_scattered.rs` (modify)

## Out of Scope

- Any change to `scenarios/survival-scattered.ron` — already includes Wash; no modification needed.
- Any change to `survival_contested.rs` — covered by ticket 001.
- Any new failure-attribution payload variant — D5 reuses existing surfaces.
- Belief-only Wash regression — covered by ticket 003.
- Player POV CLI assertion — covered by ticket 004.
- Planner-side fix if a budget-exhaustion regression is discovered — open a separate ticket per Assumption Reassessment #13.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::no_budget_exhaustion_on_survival_goals -- --ignored --exact` — passes with widened filter.
2. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::all_agents_perform_survival_actions -- --ignored --exact` — passes; existing assertions hold.
3. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::wash_is_exercised_in_scattered_topology -- --ignored --exact` (new) OR extension to `all_agents_perform_survival_actions` — at least one `WashFacilityUsed` per dirty agent OR lawful D5 failure-attribution surface fires.
4. Existing scattered module: `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered:: -- --ignored`.

### Invariants

1. `is_budget_checked_survival_goal` carries no goal-specific carve-out.
2. Any Wash failure is attributable through an existing generic emission surface filtered by `goal_key == GoalKind::Wash`.
3. No new failure-attribution payload variant.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_scattered.rs` — modify `is_budget_checked_survival_goal` (remove Wash exclusion), add or extend a test for D5 commit assertion.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered:: -- --ignored` — targeted scattered scenario verification.
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` — lint check.
3. `./scripts/verify.sh` — pre-PR full-suite verification.
