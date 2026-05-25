# S172WASDISBUD-001: Close Wash carve-out in survival-contested

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: specs/S172-wash-discovery-budget-closure.md (D3 contested portion, D5 distributed)

## Problem

`scenarios/survival-contested.ron` (line 33) authors `survival_health_contract.required_self_care_families: [Eat, Drink, Sleep, Relieve]` — explicitly omitting Wash. The sibling test `crates/worldwake-ai/tests/scenarios/survival_contested.rs` carries an explicit Wash carve-out at `is_budget_checked_survival_goal` (lines 99-110) and a comment at line 562-565, both citing tracking ID `GOAPTRVLSCAL-001`. The carve-out was added because Wash exhausted planner budget before any agent discovered a basin under contested topology. S172's D3 (contested portion) requires the carve-out be removed and Wash be elevated to a first-class member of the survival-health contract, with D5's failure-attribution surfaces asserted via the `(goal_key, generic_cause)` inspection convention. If removing the carve-out exposes a still-live planner regression, that surfaces the deferred-budget issue that S172 was authored to close.

## Assumption Reassessment (2026-05-25)

1. The carve-out exists at `crates/worldwake-ai/tests/scenarios/survival_contested.rs:99-110` (`is_budget_checked_survival_goal`) with the matching comment at lines 104-108, and is consumed by `no_budget_exhaustion_on_survival_goals` at line 1201. The carve-out is also re-cited in a comment at line 562-565. The `.ron` exclusion lives at `scenarios/survival-contested.ron:33` inside the `survival_health_contract` block at lines 20-33. Existing tests in this file: `all_agents_survive_1440_ticks:1043`, `all_agents_perform_survival_actions:1083`, `both_water_sources_are_used:1122`, `both_camp_sides_reach_food:1164`, `no_budget_exhaustion_on_survival_goals:1201` (carve-out consumer), `no_stuck_idle_windows_with_elevated_needs:1233`, `per_need_critical_run_limit_override_beats_default_for_dirtiness_only:1246`.
2. S172 Deliverable 3 (`specs/S172-wash-discovery-budget-closure.md`) requires `survival_health_contract.required_self_care_families` to become `[Eat, Drink, Sleep, Relieve, Wash]` and the test carve-out to be removed. Deliverable 5 commits to option (3) Reuse of existing failure-attribution surfaces with the goal-key-join inspection convention — no new variants.
3. Shared abstraction boundary: the `survival_health_contract.required_self_care_families` scenario-authored contract is the cross-system boundary; the carve-out function `is_budget_checked_survival_goal` is the AI-test-side filter. Both must change in this ticket — the contract change is authoritative; the test change consumes it.
4. The intended invariant: every agent must lawfully exercise Wash under contested topology within the run window OR emit the budget-exhausted failure-attribution surface (`PlanSearchOutcome::BudgetExhausted` / `Discrepancy::SearchBudgetExhausted` filtered by `goal_key == GoalKind::Wash`) with a documented recovery branch — never a silent skip.
5. Live `GoalKind` under test: `GoalKind::Wash`. Current operator surface: `WASH_OPS = [PlannerOpKind::Wash, PlannerOpKind::Travel]` at `crates/worldwake-ai/src/goal_schema.rs:101`. Budget classification: `GoalPlanningBudget::SELF_CARE` at `crates/worldwake-ai/src/goal_schema.rs:374`. Both pinned by S172 Deliverable 2.
6. AI regression layer: full action registries are required because `survival_contested` exercises end-to-end agent ticks across multiple needs and contention surfaces. Local needs-only harness is insufficient.
9. First failure boundary: if Wash discovery still exhausts budget before basin discovery, the failure is at the planner search layer (expansion-budget enforcement in `crates/worldwake-ai/src/search/`), surfaced via `PlanSearchOutcome::BudgetExhausted` in `decision_trace.rs:1393`. If the failure is instead at action start, the boundary is `RevalidationOutcome::Invalidated` at `crates/worldwake-ai/src/plan_revalidation.rs:17`. Both branches are lawful per D5.
13. Adjacent contradictions: removing the carve-out may surface a still-live planner regression in Wash discovery / travel-search. Per S172's Risks #1 (the `emit_wash_goal` helper-body audit is pending), if the implementation reveals the planner gap remains, this is a **required consequence** of this ticket — the ticket's success criterion includes EITHER a passing Wash budget-exhaustion check OR a documented lawful failure branch with recovery. If a separate planner fix is warranted, document the discovered gap and open a follow-up ticket; do not weaken this ticket's invariant to mask it.

## Architecture Check

1. Cleaner than alternatives: removing the carve-out + adding the contract row is one atomic change that closes the seam D3 targets. The alternative (extending the carve-out to other goals as they appear deferred-budget-bound) drifts further from FND-31 by allowing additional self-care families to escape lawful budget enforcement.
2. No backwards-compatibility aliasing: the carve-out function `is_budget_checked_survival_goal` is updated in place (Wash filter removed); the GOAPTRVLSCAL-001 comment is removed, not retained as a "historical note" shim.
3. FND-31 alignment: the negative case "Wash exhausts budget without traceable recovery" becomes an active assertion, not a documented exclusion.

## Verification Layers

1. Wash-contract enforcement (every required family must be exercised or its budget-exhausted surface must fire) → focused unit assertion against the post-run `CandidateGenerationDiagnostics` + selection-trace surfaces, filtered by `goal_key == GoalKind::Wash`.
2. Wash commit attribution → action trace + `DecisionEventPayload::WashFacilityUsed` (payload at `crates/worldwake-core/src/decision_event_payload.rs:79`) — verifies that any Wash that does commit carries `basin`, `user`, water/dirtiness deltas, and the partial flag.
3. Lawful budget-exhausted branch (if it fires) → decision trace via `PlanSearchOutcome::BudgetExhausted` at `crates/worldwake-ai/src/decision_trace.rs:1393` + `Discrepancy::SearchBudgetExhausted`. No silent skip.
4. Scenario isolation: the contested scenario specifically exercises a contested-resource topology with shared water/wash; the test contract change must not lawfully starve Wash on otherwise valid setups. If the lawful failure branch fires, the trace must show recovery (other self-care families continue to be exercised).

## What to Change

### 1. Add Wash to the survival-health contract

`scenarios/survival-contested.ron` (lines 20-33, `survival_health_contract` block): change `required_self_care_families: [Eat, Drink, Sleep, Relieve]` → `required_self_care_families: [Eat, Drink, Sleep, Relieve, Wash]`. Remove any in-block comment text that documents Wash as deliberately excluded.

### 2. Remove the Wash carve-out from `is_budget_checked_survival_goal`

`crates/worldwake-ai/tests/scenarios/survival_contested.rs` (lines 99-110): delete the doc-comment block that cites `GOAPTRVLSCAL-001` and rewrite `is_budget_checked_survival_goal` to:

```rust
fn is_budget_checked_survival_goal(goal: &GoalKind) -> bool {
    is_survival_goal(goal)
}
```

Remove the comment block at lines 562-565 that re-cites the carve-out rationale. If `is_survival_goal` does not currently include `GoalKind::Wash`, add it.

### 3. Extend `no_budget_exhaustion_on_survival_goals` to cover Wash

`crates/worldwake-ai/tests/scenarios/survival_contested.rs:1201`: the test currently iterates `is_budget_checked_survival_goal`-filtered attempts and asserts no budget exhaustion. With the filter widened, the test now also covers Wash. If the test passes unchanged, no further edit is required. If it fails, the failure exposes the lawful failure branch S172 D5 contemplates — per Assumption Reassessment #13, surface the discovered gap as a separate ticket and document the lawful failure here.

### 4. Add a Wash-commit positive assertion

Add a new `#[test]` `wash_is_exercised_in_contested_topology` or extend `all_agents_perform_survival_actions` (line 1083) to filter the decision-event log for at least one `DecisionEventPayload::WashFacilityUsed` per dirty agent within the run window. The payload carries `basin`, `user`, `water_consumed`, `agent_dirtiness_delta`, `basin_dirtiness_delta`, `partial` — assert presence (not specific values).

### 5. Failure-attribution surface assertion (D5)

If a Wash attempt fails (budget exhausted or revalidation rejected), the test must filter on `goal_key == GoalKind::Wash` AND the existing generic cause:
- `PlanSearchOutcome::BudgetExhausted` from `decision_trace.rs:1393` with `goal_key` from the enclosing `SelectionTrace`, OR
- `RevalidationOutcome::Invalidated { reason: PlanInvalidationReason::ExpectationMismatch, mismatch_detail }` from `plan_revalidation.rs:17` with `ExpectationMismatchPayload.goal_key` from `decision_event_payload.rs:365`.

The assertion shape: if Wash is not exercised by some agent within the run, at least one of these surfaces must fire for that agent's Wash attempt. Silent absence (no Wash candidate emitted) is itself lawful per D5's no-candidate branch — assertable via `CandidateGenerationDiagnostics` showing zero Wash candidates that tick.

## Files to Touch

- `scenarios/survival-contested.ron` (modify)
- `crates/worldwake-ai/tests/scenarios/survival_contested.rs` (modify)

## Out of Scope

- Any change to `scenarios/survival-scattered.ron` or `survival_scattered.rs` — covered by ticket 002.
- Any new failure-attribution payload variant — D5 commits to option (3) Reuse of existing surfaces; this ticket consumes those surfaces via assertions.
- Any new `GoalKind`, `PlannerOpKind`, or `MetabolismProfile` field — S172 explicitly Non-Goals these.
- Belief-only Wash regression — covered by ticket 003.
- Player POV CLI assertion — covered by ticket 004.
- Self-care occupancy / interruption contracts — S173 deliverable.
- Planner-side fix if a budget-exhaustion regression is discovered — if surfaced, open a separate ticket per Assumption Reassessment #13; this ticket's success criterion is "either Wash is lawfully exercised OR the failure branch is lawfully traced," not "Wash discovery is fixed."

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test survival_contested no_budget_exhaustion_on_survival_goals` — passes with widened filter (no Wash carve-out).
2. `cargo test -p worldwake-ai --test survival_contested all_agents_perform_survival_actions` — passes; existing assertions hold under Wash-inclusive contract.
3. `cargo test -p worldwake-ai --test survival_contested wash_is_exercised_in_contested_topology` (new) OR extension to `all_agents_perform_survival_actions` — at least one `WashFacilityUsed` per dirty agent, OR a lawful D5 failure-attribution surface fires.
4. Existing suite: `cargo test -p worldwake-ai --test survival_contested`.

### Invariants

1. `survival_health_contract.required_self_care_families` always contains Wash for the contested scenario.
2. `is_budget_checked_survival_goal` carries no goal-specific carve-out; all survival goals are budget-checked uniformly.
3. Any Wash failure within the run is attributable through an existing generic emission surface filtered by `goal_key == GoalKind::Wash` — never silent.
4. No new failure-attribution payload variant is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_contested.rs` — modify `is_budget_checked_survival_goal` (remove Wash exclusion), modify `no_budget_exhaustion_on_survival_goals` consumer (no change required if widened filter passes), add `wash_is_exercised_in_contested_topology` OR extend `all_agents_perform_survival_actions` for D5 commit assertion.
2. `scenarios/survival-contested.ron` — modify `survival_health_contract.required_self_care_families`.

### Commands

1. `cargo test -p worldwake-ai --test survival_contested` — targeted suite verification.
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` — lint check on modified test file.
3. `./scripts/verify.sh` — pre-PR full-suite verification.
