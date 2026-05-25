# S172WASDISBUD-002: Close Wash carve-out in survival-scattered

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: archive/specs/S172-wash-discovery-budget-closure.md (D3 scattered portion, D5 distributed)

## Problem

Before this ticket, `crates/worldwake-ai/tests/scenarios/survival_scattered.rs` carried a Wash carve-out at `is_budget_checked_survival_goal` and a re-citation comment, both pointing at tracking ID `GOAPTRVLSCAL-001`. Unlike contested, the scattered `.ron` already authored Wash into `survival_health_contract.required_self_care_families` — the scenario contract already said Wash mattered. The discrepancy was test-side only: the budget-exhaustion check filtered Wash out even though the contract required Wash exercising. S172 D3 (scattered portion) required the test-side filter to be widened so Wash is uniformly budget-checked under scattered topology, completing the contract enforcement the `.ron` already specifies.

## Assumption Reassessment (2026-05-25)

1. Before implementation, the carve-out existed at `crates/worldwake-ai/tests/scenarios/survival_scattered.rs` (`is_budget_checked_survival_goal`) and was consumed by `no_budget_exhaustion_on_survival_goals`. A re-citation comment lived in that test's scenario metadata. Existing tests in this file included `all_agents_survive_1440_ticks`, `all_agents_perform_survival_actions`, `isolated_agent_reaches_food_source`, `no_budget_exhaustion_on_survival_goals`, `no_stuck_idle_windows_with_elevated_needs`, and `seeded_target_location_belief_decays_to_stale_without_refresh`. The `.ron` already included Wash in `scenarios/survival-scattered.ron`.
2. S172 Deliverable 3 (`archive/specs/S172-wash-discovery-budget-closure.md`) requires the test-side filter widening for scattered. Deliverable 5 commits to option (3) Reuse with the goal-key-join inspection convention.
3. Shared abstraction boundary: the AI-test-side filter `is_budget_checked_survival_goal` is the only surface that diverges from the contract. The `.ron` contract is already correct.
4. Intended invariant: every agent must lawfully exercise Wash under scattered topology within the run window OR emit a lawful D5 failure-attribution surface — uniformly with the other four self-care goals.
5. Live `GoalKind` under test: `GoalKind::Wash`; current operator surface `WASH_OPS = [PlannerOpKind::Wash, PlannerOpKind::Travel]` at `crates/worldwake-ai/src/goal_schema.rs:101`; `GoalPlanningBudget::SELF_CARE` at `goal_schema.rs:374`.
6. AI regression layer: full action registries required (scattered exercises multi-place travel + needs). Local needs-only harness insufficient.
13. Adjacent contradictions: same conditional risk as ticket 001 — if the planner gap remains, this ticket surfaces it. Treat as required consequence per S172 D5; open follow-up planner-fix ticket if needed; do not weaken this ticket's invariant to mask it.

## Architecture Check

1. Smaller than 001 because the `.ron` is already correct — the ticket is a pure test-side filter widening. No contract change is needed.
2. No backwards-compatibility aliasing: the GOAPTRVLSCAL-001 comment block is removed, not retained.
3. FND-31 alignment: with the filter widened, the test contract matches the scenario contract; no silent escape path.

## Verified Layers

1. Wash budget-check enforcement → focused test `no_budget_exhaustion_on_survival_goals` at line 472 must pass with the widened filter; if Wash exhausts budget, the failure exposes either (a) a still-live planner regression (separate ticket) or (b) a lawful D5 budget-exhausted branch with recovery.
2. Wash commit attribution → action trace + `DecisionEventPayload::WashFacilityUsed` at `crates/worldwake-core/src/decision_event_payload.rs:79`.
3. Same goal-key-join convention as ticket 001 — filter on `goal_key == GoalKind::Wash` + the existing generic cause.
4. Single-layer surfaces are sufficient here because the scattered topology already separates resources across places; the failure-attribution surface differentiation (commit vs. budget-exhausted vs. revalidation-disconfirmed) maps cleanly per D5.

## Landed Changes

### 1. Removed the Wash carve-out from `is_budget_checked_survival_goal`

`crates/worldwake-ai/tests/scenarios/survival_scattered.rs` now routes budget checking through the full survival-goal predicate:

```rust
fn is_budget_checked_survival_goal(goal: &GoalKind) -> bool {
    is_survival_goal(goal)
}
```

The stale `GOAPTRVLSCAL-001` Wash exclusion comment was removed from the helper and from the budget-exhaustion scenario metadata. `is_survival_goal` already included `GoalKind::Wash`.

### 2. Verified `no_budget_exhaustion_on_survival_goals` covers Wash

The test consumer of `is_budget_checked_survival_goal` now sees Wash attempts in its loop. The focused exact ignored test and the full ignored scattered module both passed with Wash included, so no planner-fix follow-up was needed from this ticket.

### 3. Added a Wash-commit positive assertion

Added `wash_facility_payloads_record_every_agent`, which filters the append-only decision-event log for `DecisionEventPayload::WashFacilityUsed` and asserts every scattered-scenario agent emitted at least one payload during the run window. The assertion checks payload presence by user, not specific water/dirtiness values.

### 4. Failure-attribution surface assertion (D5)

The landed proof used the D5 commit branch for scattered: `WashFacilityUsed` payloads keyed by user and basin. The widened budget-exhaustion check also proves the `PlanSearchOutcome::BudgetExhausted` branch does not fire for Wash in this topology.

## Landed Files

- `crates/worldwake-ai/tests/scenarios/survival_scattered.rs`

## Out of Scope

- Any change to `scenarios/survival-scattered.ron` — already includes Wash; no modification needed.
- Any change to `survival_contested.rs` — covered by ticket 001.
- Any new failure-attribution payload variant — D5 reuses existing surfaces.
- Belief-only Wash regression — covered by ticket 003.
- Player POV CLI assertion — covered by ticket 004.
- Planner-side fix — not needed here because the widened scattered budget proof passed.

## Acceptance Result

### Tests Passed

1. Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::no_budget_exhaustion_on_survival_goals -- --ignored --exact` with the widened filter.
2. Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::all_agents_perform_survival_actions -- --ignored --exact`.
3. Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::wash_facility_payloads_record_every_agent -- --ignored --exact`.
4. Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered:: -- --ignored`.

### Invariants

1. `is_budget_checked_survival_goal` carries no goal-specific carve-out.
2. Any Wash failure is attributable through an existing generic emission surface filtered by `goal_key == GoalKind::Wash`.
3. No new failure-attribution payload variant.

## Test Plan Result

### Modified Tests

1. `crates/worldwake-ai/tests/scenarios/survival_scattered.rs` — modified `is_budget_checked_survival_goal` to include Wash and added `wash_facility_payloads_record_every_agent` for the D5 commit assertion.

### Commands Run

1. Passed `cargo fmt --all`.
2. Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::no_budget_exhaustion_on_survival_goals -- --ignored --exact`.
3. Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::wash_facility_payloads_record_every_agent -- --ignored --exact`.
4. Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::all_agents_perform_survival_actions -- --ignored --exact`.
5. Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered:: -- --ignored`.
6. Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
7. Waived `./scripts/verify.sh` for per-ticket closeout because `implement-spec-tickets` owns the final full pre-push gate after the full S172 ticket family lands.

## Outcome

Completed on 2026-05-25.

- Removed the scattered Wash budget-exhaustion carve-out so Wash is checked through the same survival-goal predicate as Eat, Drink, Sleep, Relieve, and ExploreLocation.
- Added the scattered `WashFacilityUsed` payload assertion, mirroring the contested D5 commit-branch proof without adding a Wash-specific failure-attribution variant.
- No scenario `.ron`, production planner, or failure-attribution payload changes were needed.

## Verification Result

- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::no_budget_exhaustion_on_survival_goals -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::wash_facility_payloads_record_every_agent -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered::all_agents_perform_survival_actions -- --ignored --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_scattered:: -- --ignored`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
- Waived `./scripts/verify.sh` in this ticket because the harness final branch phase owns the full pre-push verification gate for the S172 family.
