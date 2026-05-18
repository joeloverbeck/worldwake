# S148PORMOTBAC-FOLLOWUP-002: Restore ask_consult self-care AcquireCommodity without re-introducing baseline/scattered/preferences regressions

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — investigation surface is `crates/worldwake-ai/src/agent_tick/planning.rs` and `crates/worldwake-ai/src/feasibility_probe.rs`; resolution may also require deeper changes in `crates/worldwake-ai/src/opportunity_compiler/` or `crates/worldwake-ai/src/agent_tick/portfolio.rs`.
**Deps**: `archive/tickets/S148PORMOTBAC-FOLLOWUP-001.md`, `archive/specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

`archive/tickets/S148PORMOTBAC-FOLLOWUP-001.md` (commit `4bdca1b7`) landed two changes to recover the survival ask_consult and patrol goldens after S148-004:

1. `crates/worldwake-ai/src/feasibility_probe.rs`: `stale_exact_target_can_reach_search` now includes `GoalKind::EngageHostile` and `GoalKind::RaidTarget`. This recovered `golden_survival_patrol::survival_patrol_proves_patrol_and_remote_pursuit_execution`.
2. `crates/worldwake-ai/src/agent_tick/planning.rs`: introduced `rejected_portfolio_slot_suppresses_search` returning `false` for `GoalKind::AcquireCommodity { purpose: SelfConsume, .. }`, so probe-rejected self-care acquisition slots stayed in `search_order` at their composite-ranking position. This recovered `golden_survival_ask_consult::survival_ask_consult_lands_row_six`.

Change (2) silently regressed five other CI-only goldens that had been green at branch `4961b4c3`:

- `crates/worldwake-ai/tests/golden_survival_baseline.rs::all_agents_perform_survival_actions` — Agent C drinks but never commits `eat` (no `harvest:Harvest Apples` either) across 1440 ticks.
- `crates/worldwake-ai/tests/golden_survival_scattered.rs::all_agents_survive_1440_ticks` — Agent A hunger exceeds the authored critical run limit (`pm(850) for 1070 consecutive ticks`, ceiling 550).
- `crates/worldwake-ai/tests/golden_survival_preferences.rs::survival_preferences_keeps_proactive_diversification_alive_under_survival` — Scout Ilen hunger exceeds the authored critical run limit (`pm(820) for 914 consecutive ticks`, ceiling 320).
- `crates/worldwake-cli/tests/golden_observer_anomalies.rs::maintenance_starvation_fires_on_wash_gap` — anomaly count drifts from 3 to 2.
- `crates/worldwake-cli/tests/golden_observer_anomalies.rs::recipe_monoculture_fires_on_single_food_dependency` — anomaly count drifts from 1 to 0.

CI run `26013605898` made the regression invisible at closeout because the observer suite produced the expected anomaly counts on that runner; CI run `26014846392` (same branch HEAD + cosmetic doc commit) reproduces the local-deterministic failure. Local reproduction at branch HEAD is 100%.

Before this ticket, the parent stop-gap had reverted (2) and kept (1). Patrol, baseline, scattered, preferences, and observer anomalies were green, while ask_consult was re-broken.

## Assumption Reassessment (2026-05-18)

1. Branch `4961b4c3` (the commit immediately before `4bdca1b7`) passes baseline / scattered / preferences / observer maintenance_starvation / observer recipe_monoculture locally, and fails only ask_consult and patrol. Verified by per-commit `cargo test --release -p worldwake-{ai,cli} --test golden_survival_{baseline,scattered,preferences,ask_consult,patrol} -- --ignored --test-threads=1` and `cargo test --release -p worldwake-cli --test golden_observer_anomalies maintenance_starvation_fires_on_wash_gap -- --ignored --test-threads=1` (also for `recipe_monoculture_fires_on_single_food_dependency`).
2. The S148-004 narrative correctly identified the rejection of Witness Mira's `AcquireCommodity { commodity: Water, purpose: SelfConsume, .. }` as the failure mode. The narrowing experiments tried during the parent ticket (operating-mode zeroing toggle, `max_plans_*` increase to 8, `canonical_slot_for_kind` extension) failed because they did not change `rejected_portfolio_slot_suppresses_search` (which was added as the live fix).
3. Cap-extension and bonus-loop variants tried during this ticket's draft (extend `candidate_cap` by the bypassed count, by 3× the count, run a separate bonus loop after the primary cap, exclude bonus opportunities from `continue_same_goal_after_found`) all failed to restore baseline. This confirms the regression is not purely a cap-displacement problem; the agent-decision loop interacts with the bypassed slot beyond the cap walk (commit/continuation, blocked-memory/exhaustion-cache updates, or downstream selection scoring).
4. The shared abstraction boundary under audit is the contract between `feasibility_probe::probe` (provisional rejection of a self-care acquisition based on local evidence/route) and `agent_tick::planning::build_candidate_plans_with_sources` (search-order admission). The mismatch is asymmetric: the probe correctly rejects when no current-tick evidence supports the slot, but the planner's expansion (travel → harvest → consume) can lawfully close the gap.
5. Live `GoalKind` under test is `AcquireCommodity { commodity, purpose: SelfConsume, quantity }`; live operator surface is `PlannerOp::{Travel, Harvest, PickUp, Consume}`; live prerequisite surface is `evidence_places` + believed-route reachability + recipe knowledge.
6. AI-layer intended layer: `agent_tick::planning::build_candidate_plans_with_sources` (search-order construction) + plan-selection downstream. Mixed-layer concern: bypass mechanism interacts with `continue_same_goal_after_found` and the `results` / `select_best_plan` pipeline, which means a narrow fix here must avoid changing the priority/motive ordering that drives selection elsewhere.
7. Ordering layer involved: `ranking::compare_ranked_goals` (composite preference) and `compare_relation_aware_goal_switch` (selection-time switching). The compared branches are not symmetric: a bypassed self-care acquire competes with admitted `ConsumeOwnedCommodity` (which can also be a self-care path) and with sibling `AcquireCommodity` slots for a different commodity. Divergence depends on the conjunction of `priority_class`, `motive_score`, the bypass-induced presence in `search_order`, and the goal-blocking continuation rule.
8. Heuristic being bypassed: the probe's pre-search rejection on `MissingObservation` / `RouteUnknown` / etc. for self-care `AcquireCommodity`. The missing substrate is "the planner search can close this gap when the probe cannot", and no current code captures that contract symmetrically (the probe is one-sided). Re-introducing the bypass without the substrate is what re-opens the regression.
9. Stale-request / contested-affordance / start-failure framing: not a start-failure ticket; this is a planning-admission ticket. First failure boundary is `build_candidate_plans_with_sources` deciding which `OpportunityKey`s enter `search_order` and `take(candidate_cap)`.
10. Political office-claim framing: not applicable.
11. ControlSource / queued input framing: not applicable.
12. Golden scenario isolation: `survival-ask-consult.ron` is authored so Witness Mira has a remembered `Commons Hall` water source 2 travel ticks away, the `Harvest Water` recipe, and persistent thirst pressure. The scenario does not exclude apple harvesting as a competing path; the regression is in the planner not finding the water path, not in the scenario hiding alternatives.
13. Adjacent contradictions: `OpportunityCompilerLoad.compiled_count` semantic change in `archive/tickets/S138OPPCOM-012.md` (commit `1b6fc5e7`) drifted `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json`. That drift was a separate, intentional regression of `compiled_count` from pre-cap to post-cap accounting; the fixture has been regenerated in the current branch and is not part of this ticket.
14. Mismatch + correction: parent ticket's "Verification Result" did not list `golden_survival_baseline`, `golden_survival_scattered`, `golden_survival_preferences`, or `golden_observer_anomalies` among the gates it ran, so the regression slipped through. This ticket explicitly names every regressed test under its acceptance criteria.
15. Authoritative arithmetic: `golden_survival_baseline` requires Agent C to commit at least one `eat` in 1440 ticks; the regressed run accumulates ~10 idle eat-able ticks between drink/wash cycles where the planner picks water/sleep/wash instead, never falling through to `ConsumeOwnedCommodity(Apple)` or `AcquireCommodity(Apple, SelfConsume)`. The 1440-tick window has plenty of accumulation budget; the failure is qualitative (eat never selected), not quantitative.

## Architecture Check

1. The clean fix should make the probe's contract honest about what the planner can close. Two viable shapes:
   1. **Probe-side fix**: tighten `feasibility_probe::current_place_support_failure` and `known_target_failure` so they do not reject self-care `AcquireCommodity` when the planner's affordance graph can clearly travel → harvest → consume. This keeps `search_order` semantics intact and removes the need for any bypass in `planning.rs`.
   2. **Selection-aware bypass**: keep the bypass in `planning.rs` but only admit it when the agent currently has no `ConsumeOwnedCommodity { commodity }` plan candidate for the same commodity (i.e. when acquisition is the *only* path to satisfy the need). This narrows the bypass to Mira's case without re-displacing Agent C's `ConsumeOwnedCommodity(Apple)`.
2. Either shape must satisfy `FND-14` (belief-only planning), `FND-21` (intentions remain revisable), and avoid `FND-28` backward-compatibility shims. Selection-aware bypass is closer to a heuristic; probe-side tightening is closer to a clean abstraction boundary.
3. The combined effect must NOT re-introduce the `S138OPPCOM-012` accounting regression: `compile_opportunities` already records `compiled_count` post-truncation. The fix must not move it back.
4. Cross-system effects must propagate through state (`FND-26`): no direct cross-crate planner-to-systems calls; the affordance-graph reasoning if added in (1) is inside `worldwake-ai`.

## Outcome

Completed on 2026-05-18.

- Chose the probe-side shape. `feasibility_probe::probe` now treats low-or-higher self-care pressure as enough to let `AcquireCommodity { purpose: SelfConsume, .. }` reach search instead of being stopped by current-place or final missing-observation checks.
- Added a remote self-care acquisition probe escape for believed route topologies, including entity-anchored remote sources, so the probe does not reject a planner-resolvable travel path before search.
- Preserved `agent_tick::planning` search-order semantics. The rejected-slot bypass was not reintroduced, so the parent regression mechanism stays removed.
- Preserved the S138 post-cap `OpportunityCompilerLoad.compiled_count` contract; this ticket did not edit opportunity compiler accounting.

## Deviations

- The selected implementation did not consult the full affordance graph inside `current_place_support_failure`; it uses the live belief-view pressure and route evidence as a cheap probe contract for planner-resolvable self-care acquisition. The named focused tests and seven golden gates cover the intended surface.
- Post-ticket review moved the completed ticket to `archive/tickets/S148PORMOTBAC-FOLLOWUP-002.md`.
- Post-ticket review created the now-archived `archive/tickets/S148PORMOTBAC-FOLLOWUP-003.md` to tighten the cheap pressure proxy into a more explicitly planner-resolvable probe predicate without reopening this completed restoration.

## Verified Layers

1. Probe-side self-care acquisition admission is covered by `feasibility_probe::tests::probe_allows_low_pressure_self_care_acquire_to_reach_search`.
2. Remote self-care acquisition over believed route topology is covered by `probe_allows_remote_self_care_acquire_with_believed_route_to_reach_search` and `probe_allows_remote_entity_anchored_self_care_acquire_to_reach_search`.
3. `golden_survival_ask_consult::survival_ask_consult_lands_row_six` passes with Witness Mira committing Drink.
4. The parent regression goldens stay green: baseline, scattered, preferences, patrol, and the two observer anomaly cases all passed.
5. The full `./scripts/verify.sh` wrapper passed after the final source edit.

## Landed Files

- `crates/worldwake-ai/src/feasibility_probe.rs`
- `archive/tickets/S148PORMOTBAC-FOLLOWUP-002.md`

## Out of Scope

- Changing `CognitiveProfile.compile_opportunity_cap` or `PortfolioWeightsProfile::max_plans_*`.
- Re-tuning `ranking::compare_ranked_goals` weights or `compare_relation_aware_goal_switch` margins.
- Changes to the scenario `.ron` files for baseline / scattered / preferences / ask_consult / patrol.
- Changes to observer anomaly detector thresholds in `crates/worldwake-cli/src/bin/observer.rs`.
- Re-litigation of `archive/tickets/S138OPPCOM-012.md`'s `compiled_count` semantic change.

## Acceptance Result

1. Passed `cargo test --release -p worldwake-ai --test golden_survival_ask_consult survival_ask_consult_lands_row_six -- --ignored --test-threads=1`.
2. Passed `cargo test --release -p worldwake-ai --test golden_survival_baseline all_agents_perform_survival_actions -- --ignored --test-threads=1`.
3. Passed `cargo test --release -p worldwake-ai --test golden_survival_scattered all_agents_survive_1440_ticks -- --ignored --test-threads=1`.
4. Passed `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --test-threads=1`.
5. Passed `cargo test --release -p worldwake-ai --test golden_survival_patrol survival_patrol_proves_patrol_and_remote_pursuit_execution -- --ignored --test-threads=1`.
6. Passed `cargo test --release -p worldwake-cli --test golden_observer_anomalies maintenance_starvation_fires_on_wash_gap -- --ignored --test-threads=1`.
7. Passed `cargo test --release -p worldwake-cli --test golden_observer_anomalies recipe_monoculture_fires_on_single_food_dependency -- --ignored --test-threads=1`.
8. Passed `./scripts/verify.sh`.

## Test Plan Result

1. Passed `cargo test -p worldwake-ai --lib feasibility_probe::tests::`.
2. Passed the seven golden commands listed in Acceptance Result.
3. Passed `./scripts/verify.sh`.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib feasibility_probe::tests::probe_allows_low_pressure_self_care_acquire_to_reach_search -- --exact`.
- Passed `cargo test -p worldwake-ai --lib feasibility_probe::tests::`.
- Passed `cargo test --release -p worldwake-ai --test golden_survival_ask_consult survival_ask_consult_lands_row_six -- --ignored --test-threads=1`.
- Passed `cargo test --release -p worldwake-ai --test golden_survival_baseline all_agents_perform_survival_actions -- --ignored --test-threads=1`.
- Passed `cargo test --release -p worldwake-ai --test golden_survival_scattered all_agents_survive_1440_ticks -- --ignored --test-threads=1`.
- Passed `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --test-threads=1`.
- Passed `cargo test --release -p worldwake-ai --test golden_survival_patrol survival_patrol_proves_patrol_and_remote_pursuit_execution -- --ignored --test-threads=1`.
- Passed `cargo test --release -p worldwake-cli --test golden_observer_anomalies maintenance_starvation_fires_on_wash_gap -- --ignored --test-threads=1`.
- Passed `cargo test --release -p worldwake-cli --test golden_observer_anomalies recipe_monoculture_fires_on_single_food_dependency -- --ignored --test-threads=1`.
- Passed `./scripts/verify.sh`.
