# S148PORMOTBAC-FOLLOWUP-001: Resolve remaining S148-004 behavioral regressions in patrol and ask_consult goldens

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — targeted fixes in `crates/worldwake-ai/src/feasibility_probe.rs` and `crates/worldwake-ai/src/agent_tick/planning.rs`.
**Deps**: `archive/specs/S148-portfolio-and-motive-backed-intentions.md`, `archive/tickets/S148PORMOTBAC-004.md`

## Problem

Before this ticket, the cap-restoration + canonical-slot-routing fix shipped on top of S148 (commits 66e67d39, 9317f8fb) had recovered most of the golden survival suite, but two CI-only goldens still failed at branch HEAD:

1. `golden_survival_patrol::survival_patrol_proves_patrol_and_remote_pursuit_execution` — Guard Mira's `EngageHostile { target: Fugitive Vale }` candidate was emitted on the right tick with the correct `Market Road -> Old Mill` route cost (the upstream assertions at `golden_survival_patrol.rs:303-308` passed), but `planning.selection.selected_goal_is(EngageHostile)` was never true, so `remote_pursuit_selected` stayed `false` and the assertion at `:311` failed.
2. `golden_survival_ask_consult::survival_ask_consult_lands_row_six` — Witness Mira committed `{ask_witness, eat, harvest:Harvest Apples, pick_up, relieve_wilderness, sleep, tell, toilet, travel, wash}` but never `drink`. Her thirst stayed below critical (the cap+slot fix moved her from "dying of thirst for 443 ticks" to "alive but never drinks"), so the failure was the required-self-care-family assertion at `golden_harness/mod.rs:242` — she never picked up `Harvest Water` or `AcquireCommodity(Water, SelfConsume)` despite having the recipe and water existing at Commons Hall, 2 travel ticks away.

Both regressions bisect cleanly to commit `babb5054` (`Implemented S148PORMOTBAC-004`), the slot-assembly rewrite. They survive both the cap-restoration commit (the agents had cap 6 and 4 respectively, restored intact) and the canonical-slot-routing commit (both `EngageHostile` and `AcquireCommodity(SelfConsume)` are already routed to canonical slots by that fix).

Three further experiments ruled out cheap fixes:

- Disabling `apply_mode`'s `Emergency`-mode zeroing of `EconomicOpportunity`/`SocialMotive` did not change ask_consult's outcome (Witness Mira is in Normal mode most ticks).
- Increasing Witness Mira's `max_plans_*` from 4 to 8 did not surface the `AcquireCommodity(Water)` candidate in her search order.
- Adding `GoalKind::EngageHostile` to `canonical_slot_for_kind` as `ObligationDuty` did not change patrol's outcome.

At draft time, the residual failures were known not to be cap-, slot-routing-, or operating-mode-driven. The working hypothesis placed them downstream of `assemble_portfolio` in candidate ranking, search-order construction, or plan selection — specifically the way S148-004 wired `derive_operating_mode` and cached `runtime.operating_mode` into the candidate-plan pipeline at `crates/worldwake-ai/src/agent_tick/planning.rs:488` and `:1969`/`:2420`.

## Assumption Reassessment (2026-05-18)

1. Both failing tests pass on `main` (commit `a59b314a`) and on S148-003 (`25ec2e74`); both fail on S148-004 (`babb5054`) and every later S148 commit. Verified by per-commit `cargo test --release -p worldwake-ai --test golden_survival_ask_consult -- --ignored --test-threads=1` runs.
2. Both failures persist after the cap restoration in commit `66e67d39` (per-agent `portfolio_weights_profile` with `max_plans_*` set to the pre-S148 cap) and after the canonical-slot routing in commit `9317f8fb` (`primary_motive_slot` consults `canonical_slot_for_kind` first).
3. For ask_consult, the agent's behaviour shifted between the two failure modes: before the fixes, Witness Mira died of thirst (`thirst exceeded authored critical pm(800) for 443 consecutive ticks`); after the fixes, she stays alive on apples but never harvests or drinks water. That suggests the canonical-slot fix recovered some need-driven activity but did not restore the `AcquireCommodity(Water, SelfConsume)` candidate to her top-N search order.
4. For patrol, the candidate IS in admitted_candidates (assertions at `golden_survival_patrol.rs:303-308` confirm emission, route cost, and timing) but never wins selection. So the failure is downstream of `build_candidate_plans_with_sources` and inside plan selection or planning iteration.
5. The S148-004 changes considered during reassessment, beyond the slot-bucketing already addressed in `9317f8fb`:
   - `derive_operating_mode` is now called and cached on `AgentDecisionRuntime.operating_mode` at three sites in `planning.rs`. Anything downstream that reads `runtime.operating_mode` may behave differently than before S148-004.
   - `feasibility_probe.rs::political_exact_target_can_reach_search` and `route_place_target` reshape stale-belief handling for `ClaimOffice`/`SupportCandidateForOffice`. These changes are narrowly scoped but cooccur with the ask_consult failure (the scenario does involve political goals and consultation chains).
   - `select_best_candidate_for_slot` always prefers the committed opportunity over the highest-ranked slot match if the committed candidate also matches the slot's predicate. The OLD `select_commitment_candidate` only honored committed for the Commitment slot. This is a broader committed-preference behaviour that could pin agents to a stale committed plan over a better one.

## Architecture Check

1. FND-14 / 14A alignment: investigation must remain belief-grounded. Any plan-selection or ranking change must not introduce world-state reads on behalf of an agent.
2. FND-21 alignment: intentions remain revisable. If `select_best_candidate_for_slot`'s committed preference is too sticky, the fix should loosen it for slots where higher-ranked candidates clearly dominate, not pin commitments.
3. FND-26 alignment: cross-system effects propagate through state. Any fix in `planning.rs` must not invoke other crates directly; effects ride the existing portfolio/search/selection pipeline.
4. FND-28 alignment: no backward-compatibility wrappers. The fix should adjust the live code path, not branch on operating mode to preserve old behaviour.

## Outcome

The remaining S148-004 regressions were both pre-search admission problems, not a portfolio cap, operating-mode zeroing, or committed-plan stickiness problem.

1. `feasibility_probe.rs` now lets stale exact-target `EngageHostile` and `RaidTarget` opportunities reach planner search, matching the existing treatment for political exact-target goals. This keeps remote pursuit belief-grounded: the candidate still depends on remembered target evidence and route knowledge, but a stale remembered target location is no longer treated as a pre-search contradiction.
2. `agent_tick/planning.rs` now keeps rejected `AcquireCommodity { purpose: SelfConsume, .. }` portfolio slots eligible for planner search instead of adding them to the rejected-opportunity suppression set. The portfolio probe can still record `MissingObservation`, but self-care acquisition is search-validated because the planner may satisfy it through travel, harvest, pick-up, or consume paths that the probe does not fully expand.
3. Focused regressions cover both boundaries:
   - `probe_allows_remote_pursuit_stale_target_belief_to_reach_search` proves stale remote pursuit reaches `FeasibilityVerdict::Plausible`.
   - `rejected_self_consume_acquire_slot_still_reaches_search` and `rejected_non_acquire_slot_suppresses_search_budget` prove the self-care exception is narrow and does not disable suppression for other rejected slots.

## Deviations

The live fix did not change `found_plan_blocks_later_goals`, `derive_operating_mode`, slot caps, canonical slot routing, ranking weights, golden metadata, or generated docs. Temporary action-trace diagnostics were used during reassessment and removed before closeout.

## Verification Result

- Passed: `cargo test -p worldwake-ai feasibility_probe::tests`
- Passed: `cargo test -p worldwake-ai rejected_self_consume_acquire_slot_still_reaches_search`
- Passed: `cargo test -p worldwake-ai rejected_non_acquire_slot_suppresses_search_budget`
- Passed: `cargo test --release -p worldwake-ai --test golden_survival_patrol survival_patrol_proves_patrol_and_remote_pursuit_execution -- --ignored --test-threads=1`
- Passed: `cargo test --release -p worldwake-ai --test golden_survival_ask_consult survival_ask_consult_lands_row_six -- --ignored --test-threads=1`
- Passed: `cargo clippy --workspace --all-targets -- -D warnings`
- Blocked, covered by `tickets/S138OPPCOM-012.md`: `cargo test -p worldwake-ai` fails in `golden_opportunity_compiler::survival_baseline_replay_is_deterministic_and_compiler_load_is_bounded` because `compile_opportunities` records `OpportunityCompilerLoad.compiled_count` before truncating to `compile_opportunity_cap`, producing `max=23` against the test's `<= 16` assertion. This is a compiler-load accounting/cap assertion and was not changed by this ticket's feasibility/search-admission fix.

## Out of Scope

- The cap restoration and canonical-slot routing already shipped on the branch. This ticket is the residual investigation, not a redo of those.
- No revert of the S148 work in aggregate — only the targeted code path responsible for the two remaining regressions.

## References

- Branch HEAD: `implemented-S148-portfolio-and-motive-backed-intentions` at commit `9317f8fb`.
- Fix commits already on branch: `66e67d39` (cap restoration), `9317f8fb` (canonical-slot routing).
- Failing tests: `crates/worldwake-ai/tests/golden_survival_patrol.rs:391`, `crates/worldwake-ai/tests/golden_survival_ask_consult.rs:391`.
- Investigated surfaces: `crates/worldwake-ai/src/agent_tick/planning.rs:488,1969,2420` (operating mode wiring), `crates/worldwake-ai/src/agent_tick/portfolio.rs::select_best_candidate_for_slot:149` (committed preference per slot).
