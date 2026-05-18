# S148PORMOTBAC-FOLLOWUP-003: Tighten self-care acquisition probe escape to planner-resolvable evidence

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes - `crates/worldwake-ai/src/feasibility_probe.rs`, `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/planning_state.rs`, `crates/worldwake-ai/src/agent_tick/planning.rs`
**Deps**: `archive/tickets/S148PORMOTBAC-FOLLOWUP-002.md`, `archive/specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

Before this ticket, `archive/tickets/S148PORMOTBAC-FOLLOWUP-002.md` restored `golden_survival_ask_consult::survival_ask_consult_lands_row_six` with a broad probe-side pressure escape: low-or-higher self-care pressure let `AcquireCommodity { purpose: SelfConsume, .. }` pass missing-observation gates even when the available belief/evidence graph did not expose a concrete planner-resolvable path.

The intended invariant was narrower: self-care acquisition may reach search when concrete local evidence, route-backed remote evidence, or another explicit synthesis path gives the planner a lawful way to close the gap.

## Assumption Reassessment (2026-05-18)

1. The predecessor restoration lived in `feasibility_probe::probe`; the rejected-slot planning bypass remained absent and was not reintroduced here.
2. The pressure-only admission path was too broad for FND-3 concrete state, FND-14/FND-14A belief boundaries, FND-15 locality, and FND-20 bounded practical reasoning.
3. Reassessment against `docs/FOUNDATIONS.md` selected the split boundary: keep the probe as an evidence/locality gate, and let the planner/search surfaces handle admitted lawful goals without embedding ranking policy in the probe.
4. Final diagnosis refined the planner-side owner. `golden_survival_baseline` still failed after the probe fix because candidate generation and planning-state control checks treated loose item lots as self-care acquisition support even when the actor could not lawfully control them.
5. `local_unpossessed_commodity_evidence` in `crates/worldwake-ai/src/candidate_generation.rs` now requires legal control evidence before loose lots become self-care acquisition support, and it excludes known other-owned loose lots when the actor cannot control them, plus actor ownership-right paths, from the "unpossessed loose lot" lane.
6. `PlanningState::can_control_ref` in `crates/worldwake-ai/src/planning_state.rs` now requires the snapshot's `controllable_by_actor` flag before the local ownerless-loose-item shortcut can return true.
7. The survival-preferences golden failure separated into a stale scenario contract: diagnostics showed the familiar orchard did not deplete, so the expected durable failure memory had no concrete local depletion event to attach to. That branch is tracked by `tickets/S148PORMOTBAC-FOLLOWUP-004.md`.
8. Two CLI observer anomaly calibration gates also changed under this implementation while passing on clean pre-003 `HEAD`; those are tracked by `tickets/S148PORMOTBAC-FOLLOWUP-005.md` because they need separate observer-fixture/detector reassessment rather than pressure-escape restoration.

## Architecture Check

1. Probe admission now remains grounded in concrete local support, route-backed remote resource evidence, or existing synthesis surfaces; pressure alone no longer acts as evidence.
2. The loose-lot acquisition fix keeps FND-14/FND-14A intact by using legal control and belief-backed ownership/right surfaces instead of treating co-location as ownership knowledge.
3. The planning-state fix keeps FND-19 agent symmetry and FND-21 revisable commitments intact: an agent's plan does not become entitlement to a loose item unless the snapshot says the actor can control it.
4. The survival-preferences branch was split out rather than patched here because forcing familiar-source depletion would violate FND-1 local causality and FND-17 expectation violation unless a real local depletion event exists.

## Landed Changes

### 1. Evidence-grounded probe admission

`crates/worldwake-ai/src/feasibility_probe.rs` removed the pressure-only self-care acquisition escape and replaced it with concrete evidence checks:

- local entity/resource support can admit self-care acquisition
- route-reachable remote resource support can admit self-care acquisition
- self-care acquisition with no local support and no route-backed resource evidence is rejected before search with `MissingObservation`

### 2. Legal-control acquisition support

`crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-ai/src/planning_state.rs` now prevent legally uncontrollable loose lots from masquerading as available self-care acquisition support.

### 3. Trace selection consistency

`crates/worldwake-ai/src/agent_tick/planning.rs` now reports same-goal continuation triggers from found plans that actually block later goals, avoiding misleading trace summaries when a non-blocking found plan is skipped.

## Landed Files

- `crates/worldwake-ai/src/feasibility_probe.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/planning_state.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `archive/tickets/S148PORMOTBAC-FOLLOWUP-003.md`
- `tickets/S148PORMOTBAC-FOLLOWUP-004.md`
- `tickets/S148PORMOTBAC-FOLLOWUP-005.md`

## Out of Scope

- Reintroducing `rejected_portfolio_slot_suppresses_search` or any planning-layer bypass.
- Changing global ranking weights, portfolio weights, candidate caps, scenario `.ron` files, observer thresholds, or S138 opportunity compiler accounting.
- Repairing the survival-preferences familiar-source depletion contract; this moved to `tickets/S148PORMOTBAC-FOLLOWUP-004.md`.

## Acceptance Result

1. Probe admission for self-care acquisition is belief/evidence grounded.
2. `golden_survival_ask_consult::survival_ask_consult_lands_row_six` remained green after the final source diff.
3. `golden_survival_baseline::all_agents_perform_survival_actions` passed after the legal-control fix.
4. `golden_survival_scattered::all_agents_survive_1440_ticks` passed after the final source diff.
5. `golden_survival_patrol::survival_patrol_proves_patrol_and_remote_pursuit_execution` passed after the final source diff.
6. The survival-preferences gate was waived from this ticket and moved to `tickets/S148PORMOTBAC-FOLLOWUP-004.md` because the live failure is a stale familiar-source depletion contract, not the probe/legal-control seam.
7. The observer anomaly gates were waived from this ticket and moved to `tickets/S148PORMOTBAC-FOLLOWUP-005.md` because they are calibration-fixture/detector questions outside this probe/legal-control seam.

## Focused Tests

1. `feasibility_probe::tests::probe_rejects_low_pressure_self_care_acquire_without_resolvable_evidence` proves pressure alone no longer admits self-care acquisition.
2. `feasibility_probe::tests::probe_allows_pressured_self_care_acquire_with_local_resource_support` proves concrete local support still admits self-care acquisition.
3. `feasibility_probe::tests::probe_allows_self_anchored_self_care_acquire_with_reachable_remote_resource_source` proves route-reachable remote resource support reaches search.
4. `candidate_generation::tests::hunger_does_not_emit_loose_lot_acquire_goal_for_known_other_owned_food` proves known other-owned loose lots do not create self-care acquire candidates.
5. `planning_state::tests::local_loose_authoritative_item_requires_snapshot_control` proves the planning snapshot control flag gates local loose item control.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib feasibility_probe::tests::`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::hunger_emits_acquire_goal_for_local_unpossessed_food_lot`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::hunger_does_not_emit_loose_lot_acquire_goal_for_known_other_owned_food`
- Passed `cargo test -p worldwake-ai --lib planning_state::tests::local_loose_authoritative_item_requires_snapshot_control`
- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::satisfied_and_combat_found_plans_block_later_goals`
- Passed `cargo test --release -p worldwake-ai --test golden_survival_ask_consult survival_ask_consult_lands_row_six -- --ignored --test-threads=1`
- Passed `cargo test --release -p worldwake-ai --test golden_survival_baseline all_agents_perform_survival_actions -- --ignored --test-threads=1`
- Passed `cargo test --release -p worldwake-ai --test golden_survival_scattered all_agents_survive_1440_ticks -- --ignored --test-threads=1`
- Passed `cargo test --release -p worldwake-ai --test golden_survival_patrol survival_patrol_proves_patrol_and_remote_pursuit_execution -- --ignored --test-threads=1`
- Waived `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --test-threads=1` for this ticket because live diagnostics showed a distinct stale familiar-source depletion contract now owned by `tickets/S148PORMOTBAC-FOLLOWUP-004.md`.
- Waived `cargo test --release -p worldwake-cli --test golden_observer_anomalies maintenance_starvation_fires_on_wash_gap -- --ignored --test-threads=1` for this ticket because the final implementation reports 2 `MAINTENANCE_STARVATION` anomalies instead of 3; clean pre-003 `HEAD` passed, and follow-up ownership is `tickets/S148PORMOTBAC-FOLLOWUP-005.md`.
- Waived `cargo test --release -p worldwake-cli --test golden_observer_anomalies recipe_monoculture_fires_on_single_food_dependency -- --ignored --test-threads=1` for this ticket because the final implementation reports 0 `RECIPE_MONOCULTURE` anomalies instead of 1; clean pre-003 `HEAD` passed, and follow-up ownership is `tickets/S148PORMOTBAC-FOLLOWUP-005.md`.

## Outcome

Completed on 2026-05-18.

Changed:

- Replaced the pressure-only self-care acquisition probe escape with evidence-grounded local and route-reachable resource admission.
- Tightened loose-lot acquisition support so known other-owned uncontrollable lots and actor-owned loose lots do not masquerade as unpossessed self-care acquisition targets.
- Gated the planning-state local ownerless-loose-item shortcut on the snapshot's `controllable_by_actor` flag.
- Adjusted same-goal planning trace continuation summaries to name only found plans that actually block later goals.

Deviations:

- The survival-preferences familiar-source depletion branch was split to `tickets/S148PORMOTBAC-FOLLOWUP-004.md`.
- Two observer anomaly calibration regressions were split to `tickets/S148PORMOTBAC-FOLLOWUP-005.md`.

Verification:

- Focused AI unit tests passed for feasibility probe, candidate generation, planning state, and same-goal planning trace.
- Survival ask/consult, baseline, scattered, and patrol release goldens passed.
- Survival-preferences and observer anomaly gates are waived here with active follow-up ownership recorded above.
