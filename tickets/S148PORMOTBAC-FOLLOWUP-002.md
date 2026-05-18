# S148PORMOTBAC-FOLLOWUP-002: Restore ask_consult self-care AcquireCommodity without re-introducing baseline/scattered/preferences regressions

**Status**: PENDING
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

The stop-gap landed in the current branch (this ticket's parent fix) reverts (2) and keeps (1). Patrol stays green, baseline/scattered/preferences/observer anomalies stay green, and ask_consult is re-broken until this ticket is implemented.

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

## Verification Layers

1. Probe rejects-but-planner-resolves contract -> focused unit test in `feasibility_probe.rs` proves a probe-rejected `AcquireCommodity { commodity: Water, purpose: SelfConsume }` with a remembered place + route + recipe is admitted as Plausible (option 1) OR exposed via a planner-search admission predicate (option 2).
2. `golden_survival_ask_consult::survival_ask_consult_lands_row_six` passes -> golden E2E.
3. `golden_survival_patrol::survival_patrol_proves_patrol_and_remote_pursuit_execution` continues to pass -> golden E2E (the `stale_exact_target_can_reach_search` change from the parent ticket is preserved).
4. `golden_survival_baseline::all_agents_perform_survival_actions` continues to pass -> golden E2E.
5. `golden_survival_scattered::all_agents_survive_1440_ticks` continues to pass -> golden E2E.
6. `golden_survival_preferences::survival_preferences_keeps_proactive_diversification_alive_under_survival` continues to pass -> golden E2E.
7. `golden_observer_anomalies::maintenance_starvation_fires_on_wash_gap` and `::recipe_monoculture_fires_on_single_food_dependency` continue to pass -> observer-binary E2E.
8. Decision-trace surface proves the bypass admits the intended slot and does not silently extend admission to non-self-care goals.

## What to Change

### 1. Pick one of the two architectural shapes from Architecture Check #1

- If probe-side: extend `feasibility_probe::current_place_support_failure` to consult the affordance graph (or a cheap proxy thereof) for the agent at the anchor place, and only return `MissingObservation` when no planner expansion can close the gap. Refactor `known_target_failure` so route-unknown over a believed-route topology can defer to the search.
- If selection-aware bypass: re-introduce `rejected_portfolio_slot_suppresses_search` returning `false` for self-care `AcquireCommodity`, but gate that bypass on the absence of a sibling `ConsumeOwnedCommodity { commodity: <same> }` in `admitted_candidates`. Add a unit test for both directions of the gate.

### 2. Restore ask_consult coverage

The ticket must restore `golden_survival_ask_consult::survival_ask_consult_lands_row_six` without re-regressing the five tests named in Problem.

### 3. Document the decision

Record which shape was chosen and why under Outcome / Deviations sections of the ticket, since the parent ticket's narrower bypass deferred this trade-off.

## Files to Touch

- `crates/worldwake-ai/src/feasibility_probe.rs` (modify, if shape 1)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify, if shape 2)
- `crates/worldwake-ai/src/agent_tick/portfolio.rs` (possibly modify, depending on whether the gate inspects sibling slots)
- `tickets/S148PORMOTBAC-FOLLOWUP-002.md` (move to `archive/tickets/` on completion)

## Out of Scope

- Changing `CognitiveProfile.compile_opportunity_cap` or `PortfolioWeightsProfile::max_plans_*`.
- Re-tuning `ranking::compare_ranked_goals` weights or `compare_relation_aware_goal_switch` margins.
- Changes to the scenario `.ron` files for baseline / scattered / preferences / ask_consult / patrol.
- Changes to observer anomaly detector thresholds in `crates/worldwake-cli/src/bin/observer.rs`.
- Re-litigation of `archive/tickets/S138OPPCOM-012.md`'s `compiled_count` semantic change.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --release -p worldwake-ai --test golden_survival_ask_consult survival_ask_consult_lands_row_six -- --ignored --test-threads=1`
2. `cargo test --release -p worldwake-ai --test golden_survival_baseline all_agents_perform_survival_actions -- --ignored --test-threads=1`
3. `cargo test --release -p worldwake-ai --test golden_survival_scattered all_agents_survive_1440_ticks -- --ignored --test-threads=1`
4. `cargo test --release -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --test-threads=1`
5. `cargo test --release -p worldwake-ai --test golden_survival_patrol survival_patrol_proves_patrol_and_remote_pursuit_execution -- --ignored --test-threads=1`
6. `cargo test --release -p worldwake-cli --test golden_observer_anomalies maintenance_starvation_fires_on_wash_gap -- --ignored --test-threads=1`
7. `cargo test --release -p worldwake-cli --test golden_observer_anomalies recipe_monoculture_fires_on_single_food_dependency -- --ignored --test-threads=1`
8. Existing suite: `./scripts/verify.sh`

### Invariants

1. Self-care acquisition that the planner can resolve through travel → harvest → consume must reach `select_best_plan`. The bypass mechanism (if used) must be selection-aware: a sibling `ConsumeOwnedCommodity { same commodity }` admitted candidate must take precedence so that "already have it, just eat" beats "go get more".
2. `OpportunityCompilerLoad.compiled_count` accounting from `S138OPPCOM-012` must remain post-cap; this ticket must not regress it.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/feasibility_probe.rs` (if shape 1) — focused unit: probe-rejected self-care acquire over a believed-route topology with a co-located harvest affordance is admitted as Plausible.
2. `crates/worldwake-ai/src/agent_tick/planning.rs` (if shape 2) — focused unit: bypass admits self-care acquire when no sibling consume admitted; suppresses when a sibling consume IS admitted.

### Commands

1. `cargo test -p worldwake-ai --lib feasibility_probe::tests::` (or `planning::tests::`) — focused unit gate first.
2. The seven golden commands listed under Acceptance Criteria.
3. `./scripts/verify.sh` — full pre-PR gate.
