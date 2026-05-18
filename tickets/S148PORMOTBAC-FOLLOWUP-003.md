# S148PORMOTBAC-FOLLOWUP-003: Tighten self-care acquisition probe escape to planner-resolvable evidence

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `crates/worldwake-ai/src/feasibility_probe.rs`
**Deps**: `archive/tickets/S148PORMOTBAC-FOLLOWUP-002.md`, `archive/specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

`archive/tickets/S148PORMOTBAC-FOLLOWUP-002.md` restored `golden_survival_ask_consult::survival_ask_consult_lands_row_six` without reintroducing the rejected-slot planning bypass that regressed baseline/scattered/preferences/observer goldens.

The landed fix intentionally used a cheap probe-side pressure proxy: low-or-higher self-care pressure lets `AcquireCommodity { purpose: SelfConsume, .. }` pass the current-place and final missing-observation probe gates. That is truthful for the restored golden surface, but it is broader than the ideal contract described during reassessment: "the planner search can close this gap when the probe cannot."

The remaining cleanup is to make the self-care acquisition probe escape depend on concrete planner-resolvable evidence instead of pressure alone, while preserving the restored ask_consult and regression-golden behavior.

## Assumption Reassessment (2026-05-18)

1. The completed predecessor deliberately did not reintroduce `agent_tick::planning`'s rejected-slot bypass. The restored behavior lives in `feasibility_probe::probe`.
2. Current code has two self-care probe escapes:
   - `remote_self_care_acquire_can_reach_search(...)` requires a self-care acquire goal, relevant action defs, a non-local target place, evidence at that target place, and a believed route.
   - `self_care_acquire_pressure_allows_search(...)` requires self-care acquire pressure at or above the relevant low drive threshold, but it does not by itself prove a local support entity, local affordance, remote target place, relevant action definition, or recipe-backed expansion path.
3. The exact shared boundary remains `feasibility_probe::probe` deciding whether to suppress a ranked opportunity before `agent_tick::planning::build_candidate_plans_with_sources` can search it.
4. Live `GoalKind` under test is `AcquireCommodity { commodity, purpose: SelfConsume, quantity }`.
5. The intended invariant is not "all pressured self-care acquisition reaches search." The intended invariant is "self-care acquisition reaches search when the available belief/evidence graph gives the planner a concrete path that may close the gap."
6. The predecessor's broad verification proved no regression in the named goldens, but it did not add a negative focused test for a pressured self-care acquire with no local support, no route-backed remote evidence, and no synthesis path.
7. This is not a ranking, selection, or candidate-cap ticket. It should keep `agent_tick::planning` search-order semantics unchanged.
8. This is not a scenario-authoring ticket; no `.ron` scenario changes are expected.
9. The S138 post-cap `OpportunityCompilerLoad.compiled_count` contract is outside this ticket and must remain untouched.

## Architecture Check

1. A tightened probe predicate is cleaner than a planning bypass because it keeps the budget-saving decision at the feasibility boundary that owns pre-search rejection.
2. The predicate should continue using belief-view surfaces only, preserving FND-14 belief-only planning and FND-15 locality of knowledge.
3. The implementation should avoid a new compatibility shim or parallel admission path. Prefer refining the existing helper(s) and tests in `feasibility_probe.rs`.

## Verification Layers

1. Local self-care pressure with concrete local support -> focused `feasibility_probe.rs` unit test returns `Plausible`.
2. Local self-care pressure with no local support and no remote/resolution evidence -> focused `feasibility_probe.rs` unit test returns `RejectedBeforeSearch { reason: MissingObservation }`.
3. Remote self-care acquisition with believed route and evidence place -> existing or updated focused unit tests remain `Plausible`.
4. ask_consult restoration -> `golden_survival_ask_consult::survival_ask_consult_lands_row_six`.
5. Parent regression guardrails -> baseline/scattered/preferences/patrol plus the two observer anomaly goldens.

## What to Change

### 1. Refine the pressure escape

Update `self_care_acquire_pressure_allows_search(...)` or its call sites so pressure alone is not sufficient. The escape should require one of:

- concrete local support/affordance evidence that the planner can use at the current place
- remote place/entity evidence with a believed route, as in `remote_self_care_acquire_can_reach_search(...)`
- another explicit planner-synthesis path already represented in `feasibility_probe.rs`

### 2. Preserve the predecessor restoration

Keep `golden_survival_ask_consult::survival_ask_consult_lands_row_six` green and do not modify `agent_tick::planning` rejected-slot filtering.

## Files to Touch

- `crates/worldwake-ai/src/feasibility_probe.rs` (modify)

## Out of Scope

- Reintroducing `rejected_portfolio_slot_suppresses_search` or any planning-layer bypass.
- Changing ranking weights, portfolio weights, candidate caps, scenario `.ron` files, observer thresholds, or S138 opportunity compiler accounting.

## Acceptance Criteria

### Tests That Must Pass

1. Focused feasibility-probe tests for both admitted and rejected self-care acquisition paths.
2. `cargo test --release -p worldwake-ai --test golden_survival_ask_consult survival_ask_consult_lands_row_six -- --ignored --test-threads=1`
3. `cargo test --release -p worldwake-ai --test golden_survival_baseline all_agents_perform_survival_actions -- --ignored --test-threads=1`
4. `cargo test --release -p worldwake-ai --test golden_survival_scattered all_agents_survive_1440_ticks -- --ignored --test-threads=1`
5. `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --test-threads=1`
6. `cargo test --release -p worldwake-ai --test golden_survival_patrol survival_patrol_proves_patrol_and_remote_pursuit_execution -- --ignored --test-threads=1`
7. `cargo test --release -p worldwake-cli --test golden_observer_anomalies maintenance_starvation_fires_on_wash_gap -- --ignored --test-threads=1`
8. `cargo test --release -p worldwake-cli --test golden_observer_anomalies recipe_monoculture_fires_on_single_food_dependency -- --ignored --test-threads=1`

### Invariants

1. Probe admission for self-care acquisition stays belief/evidence grounded.
2. The completed predecessor's ask_consult restoration remains green.
3. The rejected-slot planning bypass remains absent.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/feasibility_probe.rs` — add a negative pressured self-care acquisition test with no local support and no remote resolvability.
2. `crates/worldwake-ai/src/feasibility_probe.rs` — keep or update positive local/remote self-care acquisition admission tests.

### Commands

1. `cargo test -p worldwake-ai --lib feasibility_probe::tests::`
2. The seven golden commands listed under Acceptance Criteria.
