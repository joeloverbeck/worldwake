# S172WASDISBUD-001: Close Wash carve-out in survival-contested

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` plan selection active-goal/current-plan retention guard; contested scenario contract/test assertions
**Deps**: archive/specs/S172-wash-discovery-budget-closure.md (D3 contested portion, D5 distributed)

## Problem

Before this ticket, `scenarios/survival-contested.ron` authored `survival_health_contract.required_self_care_families: [Eat, Drink, Sleep, Relieve]` — explicitly omitting Wash. The sibling test `crates/worldwake-ai/tests/scenarios/survival_contested.rs` carried an explicit Wash carve-out at `is_budget_checked_survival_goal` and a scenario comment, both citing tracking ID `GOAPTRVLSCAL-001`. The carve-out was added because Wash exhausted planner budget before any agent discovered a basin under contested topology. S172's D3 (contested portion) required the carve-out be removed and Wash be elevated to a first-class member of the survival-health contract, with D5's failure-attribution surfaces asserted via the `(goal_key, generic_cause)` inspection convention.

## Assumption Reassessment (2026-05-25)

1. The carve-out exists at `crates/worldwake-ai/tests/scenarios/survival_contested.rs:99-110` (`is_budget_checked_survival_goal`) with the matching comment at lines 104-108, and is consumed by `no_budget_exhaustion_on_survival_goals` at line 1201. The carve-out is also re-cited in a comment at line 562-565. The `.ron` exclusion lives at `scenarios/survival-contested.ron:33` inside the `survival_health_contract` block at lines 20-33. Existing tests in this file: `all_agents_survive_1440_ticks:1043`, `all_agents_perform_survival_actions:1083`, `both_water_sources_are_used:1122`, `both_camp_sides_reach_food:1164`, `no_budget_exhaustion_on_survival_goals:1201` (carve-out consumer), `no_stuck_idle_windows_with_elevated_needs:1233`, `per_need_critical_run_limit_override_beats_default_for_dirtiness_only:1246`.
2. S172 Deliverable 3 (`archive/specs/S172-wash-discovery-budget-closure.md`) requires `survival_health_contract.required_self_care_families` to become `[Eat, Drink, Sleep, Relieve, Wash]` and the test carve-out to be removed. Deliverable 5 commits to option (3) Reuse of existing failure-attribution surfaces with the goal-key-join inspection convention — no new variants.
3. Shared abstraction boundary: the `survival_health_contract.required_self_care_families` scenario-authored contract is the cross-system boundary; the carve-out function `is_budget_checked_survival_goal` is the AI-test-side filter. Both must change in this ticket — the contract change is authoritative; the test change consumes it.
4. The intended invariant: every agent must lawfully exercise Wash under contested topology within the run window OR emit the budget-exhausted failure-attribution surface (`PlanSearchOutcome::BudgetExhausted` / `Discrepancy::SearchBudgetExhausted` filtered by `goal_key == GoalKind::Wash`) with a documented recovery branch — never a silent skip.
5. Live `GoalKind` under test: `GoalKind::Wash`. Current operator surface: `WASH_OPS = [PlannerOpKind::Wash, PlannerOpKind::Travel]` at `crates/worldwake-ai/src/goal_schema.rs:101`. Budget classification: `GoalPlanningBudget::SELF_CARE` at `crates/worldwake-ai/src/goal_schema.rs:374`. Both pinned by S172 Deliverable 2.
6. AI regression layer: full action registries are required because `survival_contested` exercises end-to-end agent ticks across multiple needs and contention surfaces. Local needs-only harness is insufficient.
9. First failure boundary: if Wash discovery still exhausts budget before basin discovery, the failure is at the planner search layer (expansion-budget enforcement in `crates/worldwake-ai/src/search/`), surfaced via `PlanSearchOutcome::BudgetExhausted` in `decision_trace.rs:1393`. If the failure is instead at action start, the boundary is `RevalidationOutcome::Invalidated` at `crates/worldwake-ai/src/plan_revalidation.rs:17`. Both branches are lawful per D5.
13. Adjacent contradictions: removing the carve-out could have surfaced a still-live planner regression in Wash discovery / travel-search. Later S172 implementation closed the `emit_wash_goal` helper-body audit in `archive/specs/S172-wash-discovery-budget-closure.md` and did not require a separate planner follow-up.
14. Implementation correction: the widened ignored `survival_contested` run exposed a live planner consistency bug before any Wash-specific failure assertion fired. `select_best_plan` could retain `runtime.current_plan` when `agenda_state.committed` named a different active goal, producing a debug assertion in `determine_selected_plan_source` (`Apple` active goal vs `Water` retained plan). This was required current-ticket fallout because the contested proof could not run with the stale retained-plan path. The landed guard in `crates/worldwake-ai/src/plan_selection.rs` returns to the best searched plan when `active_goal != Some(current_plan.goal)`.

## Architecture Check

1. Cleaner than alternatives: removing the carve-out + adding the contract row is one atomic change that closes the seam D3 targets. The alternative (extending the carve-out to other goals as they appear deferred-budget-bound) drifts further from FND-31 by allowing additional self-care families to escape lawful budget enforcement.
2. No backwards-compatibility aliasing: the carve-out function `is_budget_checked_survival_goal` is updated in place (Wash filter removed); the GOAPTRVLSCAL-001 comment is removed, not retained as a "historical note" shim.
3. FND-31 alignment: the negative case "Wash exhausts budget without traceable recovery" becomes an active assertion, not a documented exclusion.

## Verified Layers

1. Wash-contract enforcement (every required family must be exercised or its budget-exhausted surface must fire) → focused unit assertion against the post-run `CandidateGenerationDiagnostics` + selection-trace surfaces, filtered by `goal_key == GoalKind::Wash`.
2. Wash commit attribution → action trace + `DecisionEventPayload::WashFacilityUsed` (payload at `crates/worldwake-core/src/decision_event_payload.rs:79`) — verifies that any Wash that does commit carries `basin`, `user`, water/dirtiness deltas, and the partial flag.
3. Lawful budget-exhausted branch (if it fires) → decision trace via `PlanSearchOutcome::BudgetExhausted` at `crates/worldwake-ai/src/decision_trace.rs:1393` + `Discrepancy::SearchBudgetExhausted`. No silent skip.
4. Scenario isolation: the contested scenario specifically exercises a contested-resource topology with shared water/wash; the test contract change must not lawfully starve Wash on otherwise valid setups. If the lawful failure branch fires, the trace must show recovery (other self-care families continue to be exercised).

## Landed Changes

### 1. Added Wash to the survival-health contract

`scenarios/survival-contested.ron` now authors `required_self_care_families: [Eat, Drink, Sleep, Relieve, Wash]`.

### 2. Removed the Wash carve-out from `is_budget_checked_survival_goal`

`crates/worldwake-ai/tests/scenarios/survival_contested.rs` now delegates `is_budget_checked_survival_goal` directly to `is_survival_goal`, so `GoalKind::Wash` is budget-checked with Eat, Drink, Sleep, Relieve, and ExploreLocation.

### 3. Added explicit Wash commit payload proof

`wash_facility_payloads_record_every_agent` asserts every contested-scenario agent emits at least one persisted `DecisionEventPayload::WashFacilityUsed` payload. This proves the D5 completed-Wash branch through the existing generic payload surface.

### 4. Fixed stale current-plan retention exposed by the widened proof

`crates/worldwake-ai/src/plan_selection.rs` now refuses to retain `runtime.current_plan` when the committed active goal no longer matches that current plan's goal. The focused regression `current_plan_is_not_retained_when_active_goal_differs` covers the Apple-active/Water-retained mismatch shape exposed by the contested survival run.

### 5. Refreshed generated golden docs

`python3 scripts/golden_inventory.py --write --check-docs` updated the survival-contested inventory entry and source-line references in generated golden docs.

## Landed Files

- `scenarios/survival-contested.ron` (modify)
- `crates/worldwake-ai/tests/scenarios/survival_contested.rs` (modify)
- `crates/worldwake-ai/src/plan_selection.rs` (modify)
- `docs/generated/golden-*.md` and `docs/generated/golden-scenario-details/*.md` (regenerated)

## Out of Scope

- Any change to `scenarios/survival-scattered.ron` or `survival_scattered.rs` — covered by ticket 002.
- Any new failure-attribution payload variant — D5 commits to option (3) Reuse of existing surfaces; this ticket consumes those surfaces via assertions.
- Any new `GoalKind`, `PlannerOpKind`, or `MetabolismProfile` field — S172 explicitly Non-Goals these.
- Belief-only Wash regression — covered by ticket 003.
- Player POV CLI assertion — covered by ticket 004.
- Self-care occupancy / interruption contracts — S173 deliverable.
- Planner-side fix if a budget-exhaustion regression is discovered — if surfaced, open a separate ticket per Assumption Reassessment #13; this ticket's success criterion is "either Wash is lawfully exercised OR the failure branch is lawfully traced," not "Wash discovery is fixed."

## Acceptance Result

### Verified Tests

1. Passed `cargo test -p worldwake-ai plan_selection::tests::current_plan_is_not_retained_when_active_goal_differs -- --exact`.
2. Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_contested:: -- --ignored`; this is the live replacement for the stale drafted `--test survival_contested` command. It ran all 7 ignored contested scenario tests, including `all_agents_perform_survival_actions`, `no_budget_exhaustion_on_survival_goals`, and `wash_facility_payloads_record_every_agent`.
3. Passed `cargo test -p worldwake-ai`.
4. Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.

### Invariants

1. `survival_health_contract.required_self_care_families` always contains Wash for the contested scenario.
2. `is_budget_checked_survival_goal` carries no goal-specific carve-out; all survival goals are budget-checked uniformly.
3. Any Wash failure within the run is attributable through an existing generic emission surface filtered by `goal_key == GoalKind::Wash` — never silent.
4. No new failure-attribution payload variant is introduced.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/plan_selection.rs` — added `current_plan_is_not_retained_when_active_goal_differs`.
2. `crates/worldwake-ai/tests/scenarios/survival_contested.rs` — removed the Wash budget carve-out, added `wash_facility_payloads_record_every_agent`, and regenerated scenario metadata.
3. `scenarios/survival-contested.ron` — added Wash to `survival_health_contract.required_self_care_families`.

### Commands Run

1. Passed `cargo test -p worldwake-ai plan_selection::tests::current_plan_is_not_retained_when_active_goal_differs -- --exact`.
2. Passed `cargo test -p worldwake-ai --test golden_ai survival_contested -- --list`.
3. Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_contested:: -- --ignored`.
4. Passed `python3 scripts/golden_inventory.py --write --check-docs`.
5. Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
6. Passed `cargo test -p worldwake-ai`.
7. Waived `./scripts/verify.sh` for this per-ticket closeout because the `implement-spec-tickets` final branch phase owns the full pre-push gate after all S172 tickets land.

## Outcome

Completed on 2026-05-25.

Outcome amended: 2026-05-25.

- `survival-contested.ron` now requires Wash alongside Eat, Drink, Sleep, and Relieve.
- The contested golden no longer excludes Wash from budget-exhaustion checks.
- The contested golden now asserts persisted `WashFacilityUsed` payloads for every authored agent.
- The planner no longer retains a current plan whose goal diverges from the committed active goal, closing the Apple-active/Water-retained mismatch exposed by the widened contested run.
- Generated golden inventory/docs were refreshed from the new scenario metadata.
- Later S172 closeout closed the helper-body audit risk this ticket cited during reassessment; no planner follow-up was required.

## Deviations

- The drafted ticket listed `Engine Changes: None`, but the widened proof exposed a production planner retention bug that blocked the scenario. The implemented seam includes a focused `select_best_plan` guard plus regression coverage.
- The drafted commands used nonexistent target `--test survival_contested`; live verification used the `golden_ai` integration target with the `scenarios::survival_contested::` module filter.

## Verification Result

- Passed `cargo test -p worldwake-ai plan_selection::tests::current_plan_is_not_retained_when_active_goal_differs -- --exact`.
- Passed `cargo test -p worldwake-ai --test golden_ai survival_contested -- --list`.
- Passed `cargo test -p worldwake-ai --test golden_ai scenarios::survival_contested:: -- --ignored`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
- Passed `cargo test -p worldwake-ai`.
- Waived `./scripts/verify.sh` for this per-ticket closeout; the spec-family harness final branch phase owns the full pre-push gate before push.
