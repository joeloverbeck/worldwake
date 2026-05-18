# S148PORMOTBAC-FOLLOWUP-007: Architectural fix for self-care acquisition feasibility filtering without remote-acquisition regression

**Status**: REJECTED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — reassessment and implementation attempt only; temporary runtime edits were removed after golden regressions.
**Deps**: `archive/tickets/S148PORMOTBAC-FOLLOWUP-002.md`, `archive/tickets/S148PORMOTBAC-FOLLOWUP-003.md`

## Problem

`archive/tickets/S148PORMOTBAC-FOLLOWUP-003.md` (commit `8a24e288`) introduced three intertwined changes to fix `golden_survival_baseline`, `golden_survival_scattered`, and the `golden_survival_preferences` familiar-source-failure contract:

1. **Probe tightening** in `feasibility_probe::probe` — removed the pressure-only self-care `AcquireCommodity` escape introduced by `archive/tickets/S148PORMOTBAC-FOLLOWUP-002.md`; replaced with `reachable_remote_resource_self_care_acquire_can_reach_search` and stricter local-evidence checks.
2. **`local_unpossessed_commodity_evidence` EARLY `can_control` filter** in `crates/worldwake-ai/src/candidate_generation.rs` — added `if !view.can_control(agent, entity) { continue; }` before the existing belief-driven `has_known_uncontrollable_other_owner` check.
3. **Static `controllable_by_actor` check** in `crates/worldwake-ai/src/planning_state.rs::can_control_ref` shortcut — added `&& snapshot.control.controllable_by_actor` to the loose-ItemLot/UniqueItem/Container co-located shortcut.

Together those passed the named survival goldens but silently broke:

- **11 worldwake-ai unit tests** in `candidate_generation::tests` and `search::tests` covering REMOTE acquisition and REMOTE production planning. The static `controllable_by_actor` flag is computed at the initial planning snapshot (when the actor is not yet co-located with the remote entity) and never refreshed as the planner simulates travel, so any plan involving "travel to remote unowned ItemLot, pick up" returns `can_control_ref = false` for the entire search.
- **`golden_survival_trade::survival_trade_proves_substitute_market_branch`** — Merchant Sera develops `StuckIdleWindow`s with elevated needs (e.g. ticks 1043–1102 at max_need 475, ticks 1341–1422 at max_need 550). The `can_control` filter removes legitimate acquisition support from her candidate generation, starving the trade negotiation loop.
- **`local_unpossessed_commodity_evidence` REMOTE acquisition**: in production, when the actor is at place A and a loose unowned `ItemLot` is at place B, `PerAgentBeliefView::can_control` returns `false` (first branch fails on `effective_place(actor) != effective_place(entity)`; second branch on `can_exercise_control` denies for non-rights actors). The EARLY filter then drops the lot from evidence, so no acquire goal is generated for B. The agent never plans a trip to pick it up. This is exposed by `acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence`, but is a real production regression, not a test-only artifact.

This ticket reverts FOLLOWUP-003 to restore the unit tests, trade, and remote acquisition, then re-opens the baseline/scattered/preferences contracts that FOLLOWUP-003 was originally fixing — but with full diagnostic context so a proper architectural fix can replace the over-broad filter.

Implementation attempt on 2026-05-18 rejected this combined ticket as an implementable seam. The proposed broad owner joins at least two competing contracts: pressure-only self-care acquire admission is still required by the current baseline/trade survival loops, while durable same-goal search failure persistence is still required by baseline but violates the preferences familiar-source-failure assertion. A successor ticket, `../tickets/S148PORMOTBAC-FOLLOWUP-008.md`, now owns the narrower split design.

## Assumption Reassessment (2026-05-18)

1. With FOLLOWUP-003 fully reverted on the current branch HEAD (`0e6d06f6` + revert of `8a24e288`), the following local results hold:
   - `cargo test -p worldwake-ai --lib` — 1644 passed, 0 failed.
   - `cargo test --release -p worldwake-ai --test golden_survival_trade -- --ignored --test-threads=1` — both tests pass.
   - `cargo test --release -p worldwake-ai --test golden_survival_ask_consult -- --ignored --test-threads=1` — passes.
   - `cargo test --release -p worldwake-ai --test golden_survival_patrol -- --ignored --test-threads=1` — both tests pass.
   - `cargo test --release -p worldwake-ai --test golden_survival_baseline -- --ignored --test-threads=1` — `all_agents_survive_1440_ticks` and `no_stuck_idle_windows_with_elevated_needs` fail (Agent A starvation + stuck idle ticks 1086–1326 at need 817).
   - `cargo test --release -p worldwake-ai --test golden_survival_scattered -- --ignored --test-threads=1` — `no_stuck_idle_windows_with_elevated_needs` fails (Agent A stuck idle ticks 1207–1337 at need 840).
   - `cargo test --release -p worldwake-ai --test golden_survival_preferences -- --ignored --test-threads=1` — `survival_preferences_keeps_proactive_diversification_alive_under_survival` fails (`familiar_failed_attempts == 1`, expected 0).
2. The probe tightening (FOLLOWUP-003 change 1) and the can_control filters (changes 2 + 3) are SEPARABLE on paper but cooperate in practice: without the filters, more `AcquireCommodity { purpose: SelfConsume, .. }` candidates reach `take(candidate_cap)` in `agent_tick::planning::build_candidate_plans_with_sources`, displacing `ConsumeOwnedCommodity` and infeasible-but-prioritized acquire goals crowd out essential consume goals. This is the same per-tick cap displacement mechanism described in `commit 619404ab` for the FOLLOWUP-001 bypass; FOLLOWUP-003's can_control filter functioned as a feasibility pre-filter that compensated for that displacement.
3. Live `GoalKind` under test is `AcquireCommodity { commodity, purpose: SelfConsume, quantity }` competing with `ConsumeOwnedCommodity { commodity }` for the same per-tick slot.
4. The intended invariant is two-sided: agents must (a) be able to plan remote-loose-lot acquisition over the travel horizon when they have belief evidence about an unowned lot at a remote place; and (b) not waste planning budget on co-located lots they cannot lawfully control (e.g. another agent's possession at the same place). FOLLOWUP-003 collapsed both onto a single world-state `can_control` query that violates (a) for remote places and works by accident for co-located ones.
5. The `planning_state::can_control_ref` shortcut combines a dynamic check (`effective_place_ref(actor) == effective_place_ref(entity)`) with a static check (`snapshot.control.controllable_by_actor`). Only the dynamic check tracks the simulated planner state; the static check freezes the initial-state control judgment. Any fix must use only state that the planner re-evaluates as the actor moves, OR re-evaluate `controllable_by_actor` after each planning step.
6. The `local_unpossessed_commodity_evidence` LATER check (`has_known_uncontrollable_other_owner`) is the belief-driven filter that catches the intended case (agent believes another agent owns the lot AND cannot exercise control). It is FND-14 compliant. The EARLY check (`!view.can_control(agent, entity)`) bypasses belief and queries world authoritative state directly; it violates FND-14 and additionally over-filters remote loose lots even when the agent has no ownership belief.
7. The preferences `familiar_failed_attempts == 0` contract was added in `commit 8fc9a9c5` (FOLLOWUP-004) under the assumption that FOLLOWUP-003's filter would keep the familiar source out of failure memory. With the revert, one false-failure is recorded; this needs a separate fix at the source reliability layer or the test contract needs to be re-evaluated.
8. The trade scenario `Merchant Sera` StuckIdleWindow signal was visible in CI before FOLLOWUP-003 landed (run `26017281853`) — it was a pre-existing latent failure masked by other behavior, and FOLLOWUP-003 made it worse by removing acquisition support. Reverting FOLLOWUP-003 makes trade pass again, but the underlying brittleness of the trade contract under tight per-tick caps should be revisited.
9. Mismatch + correction: `archive/tickets/S148PORMOTBAC-FOLLOWUP-003.md` Acceptance Result lists baseline + scattered + ask_consult + patrol as gates; it did NOT run the 11 worldwake-ai unit tests in `candidate_generation::tests` / `search::tests` that this revert restores, so the regression slipped through.
10. 2026-05-18 implementation attempt result: removing the pressure-only self-care probe admission from `crates/worldwake-ai/src/feasibility_probe.rs` passed focused probe/lib checks but regressed `golden_survival_baseline` (`Agent C` did not commit `Eat`; `Agent B` exceeded critical hunger for 1179 ticks) and did not fix `golden_survival_trade` (`Buyer Nila` still exceeded critical hunger for a long window). This disproves probe-only tightening as the owner for the combined ticket.
11. 2026-05-18 implementation attempt result: filtering `ExpectationFailurePhase::Search` + `SameGoalSearchInfeasibleWhileSiblingSucceeded` out of durable `SourceReliability.failed_attempts` fixed the preferences-style focused assertion, but regressed `golden_survival_baseline` (`Agent A` thirst exceeded critical for 550 ticks; stuck idle ticks 1086-1326). Restoring transient source-expectation event emission was not enough; the durable search-failure memory participates in baseline survival behavior.
12. Mismatch + correction: the remaining work is not one implementation choice among the three shapes below. The next owner must first split durable ranking memory from public/familiar source-failure accounting, and separately preserve self-care pressure behavior without reopening false positives in trade/preferences.

## Architecture Check

The clean fix must respect:

1. **FND-14 (belief-only planning)**: candidate filtering must use belief surfaces (`believed_owner_of`, `believed_rights`) rather than world-state `can_control` lookahead.
2. **FND-14A (same-tick direct observation)**: when the actor is co-located with the entity, world-state observation is admissible. The EARLY filter could lawfully fire ONLY when `view.effective_place(agent) == Some(place)`. This is Option A from this investigation; it did not pass trade/scattered/ask_consult, so co-location alone is not enough either.
3. **FND-21 (revisable commitments)**: planner shortcuts must track dynamic state, not freeze initial-state control judgments.
4. **FND-20 (bounded practical reasoning)**: the per-tick candidate cap is a real bound; the fix needs to either widen the cap, change cap admission to prefer concrete consume/acquire pairs over speculative acquire-only goals, or filter at a different layer that does not violate FND-14.

Three architectural shapes the next ticket should evaluate (none chosen here; user selects after diagnosing baseline/scattered/preferences):

1. **Belief-driven evidence tightening**: replace world-state `can_control` with a belief-only feasibility predicate (e.g. "agent has no contradicting ownership belief AND has belief evidence the lot is unowned OR self-owned"). Implement entirely in `candidate_generation`. Removes the FND-14 violation. May still need cap-admission tuning to keep ConsumeOwned from being displaced.
2. **Cap-admission rebalancing**: in `agent_tick::planning::build_candidate_plans_with_sources`, prefer `ConsumeOwnedCommodity` over competing `AcquireCommodity` for the same commodity before truncation. Surgically fixes the displacement problem the FOLLOWUP-003 filter was working around without removing legitimate acquire goals.
3. **Probe-side affordance graph**: tighten `feasibility_probe::current_place_support_failure` to also consult the planner's affordance graph (the "deeper" probe shape from `archive/tickets/S148PORMOTBAC-FOLLOWUP-002.md` Architecture Check 1.1). Probe rejects infeasible acquires without touching candidate generation. Most invasive but cleanest architectural boundary.

## Verification Layers

1. `acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence` (focused) — remote loose lot at unowned place B emits an acquire goal for the agent at place A.
2. `search::tests::s146_search_trace_records_per_goal_budget_under_elevated_cognitive_ceiling` (focused, production view) — `search_plan` finds a remote production plan with an unowned firewood lot at orchard_farm.
3. `golden_survival_baseline::all_agents_survive_1440_ticks` (golden) — no agent exceeds critical hunger for > 300 ticks.
4. `golden_survival_baseline::no_stuck_idle_windows_with_elevated_needs` (golden) — no idle windows > 220 ticks with needs > 300.
5. `golden_survival_scattered::no_stuck_idle_windows_with_elevated_needs` (golden) — no idle windows > 50 ticks with needs > 300.
6. `golden_survival_preferences::survival_preferences_keeps_proactive_diversification_alive_under_survival` (golden) — `familiar_failed_attempts == 0`.
7. `golden_survival_trade::survival_trade_proves_substitute_market_branch` (golden) — no `StuckIdleWindow` >= 60 ticks with need > 300.
8. `golden_survival_ask_consult::survival_ask_consult_lands_row_six` (golden) — Witness Mira commits Drink.
9. `golden_survival_patrol::*` (golden) — both patrol gates remain green.

## What to Change

### 1. Diagnose baseline Agent A starvation under FOLLOWUP-002-state code

Run `cargo test --release -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --test-threads=1` with `WORLDWAKE_AI_DECISION_TRACE` and `WORLDWAKE_ACTION_TRACE` (see `docs/debugging-traces.md`). Identify the per-tick reason Agent A stops planning consume/acquire actions at the failure window. Most likely a per-tick cap displacement — confirm with decision trace.

### 2. Diagnose scattered Agent A stuck-idle at ticks 1207–1337

Same trace methodology. Cross-check whether the displacement is the same mechanism as baseline (same root cause = single fix) or a different mechanism (separate fix required).

### 3. Diagnose preferences `familiar_failed_attempts == 1`

The source reliability tracking is recording a false familiar-source failure. Identify which tick + which evidence path emits the failure record. Determine whether the right fix is at the source reliability layer (`crates/worldwake-ai/src/source_reliability/` or similar) or in the test contract.

### 4. Choose and implement one of the three architectural shapes

Per the Architecture Check section. The implementation must pass ALL verification layers above (1–9) simultaneously.

### 5. Regenerate scenario diagnostics fixture

`WORLDWAKE_UPDATE_SCENARIO_DIAGNOSTICS_FIXTURE=1 cargo test --release -p worldwake-ai --test golden_scenario_diagnostics_fixture -- --ignored --test-threads=1`.

## Files to Touch

- `crates/worldwake-ai/src/feasibility_probe.rs` (modify, depending on chosen shape)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify, depending on chosen shape)
- `crates/worldwake-ai/src/planning_state.rs` (modify, depending on chosen shape)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify, if shape 2 is chosen)
- `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` (regenerate)
- `tickets/` / `archive/tickets/` per archival workflow

## Out of Scope

- Re-introducing the FOLLOWUP-001 planning.rs bypass (was reverted in `commit 619404ab` for cap-displacement reasons; do not re-open).
- Changing `CognitiveProfile.compile_opportunity_cap` or `PortfolioWeightsProfile::max_plans_*` without measuring per-scenario impact.
- Changing the trade scenario `.ron` to mask the underlying contract brittleness — fix at the planner/feasibility layer.

## Acceptance Criteria

### Tests That Must Pass

1. All 9 verification layers above.
2. `./scripts/verify.sh` — full pre-PR gate.
3. The 11 worldwake-ai unit tests this revert restored must continue to pass.

### Invariants

1. Remote loose unowned `ItemLot` evidence is admitted for acquire-goal generation when the actor has belief evidence of the lot, regardless of whether the actor is currently controlling-co-located.
2. The planner-state `can_control_ref` shortcut tracks dynamic planning-state location, not the initial snapshot's `controllable_by_actor` flag.
3. Candidate filtering uses belief surfaces (FND-14); world-state `can_control` lookahead is admissible only when the agent is same-tick co-located with the entity (FND-14A).

## Test Plan

### New/Modified Tests

1. The 11 restored unit tests in `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-ai/src/search/tests.rs` must remain unchanged; the chosen fix must accommodate their assertions.
2. New focused test capturing the per-tick cap displacement mechanism (whatever the diagnostic in change 1 reveals) — TDD: failing test first, fix second.
3. New focused test capturing the preferences familiar-source-failure mechanism (whatever the diagnostic in change 3 reveals) — TDD.

### Commands

1. `cargo test -p worldwake-ai --lib` (focused unit, fast)
2. The 6 named golden commands in Verification Layers 3–9 (CI-only, run via golden-survival workflow locally with `--release --ignored --test-threads=1`)
3. `./scripts/verify.sh`

## Outcome

- **Completion date**: 2026-05-18
- **What actually changed**: Reassessed and attempted the live implementation. Removed all temporary engine edits after release goldens showed regressions.
- **Deviations from original plan**: The ticket is rejected as too broad. Probe-only tightening and source-reliability-only narrowing each break a required survival golden contract.
- **Verification results**:
  - Passed `cargo test -p worldwake-ai --lib feasibility_probe::tests::` with the temporary probe tightening before it was removed.
  - Passed `cargo test -p worldwake-ai --lib agent_tick::tests::apply_source_reliability_failure_observations_coalesces_duplicates_without_persisting_search_only_failures -- --exact` with the temporary source-reliability narrowing before it was removed.
  - Passed `cargo test -p worldwake-ai --lib` with the temporary probe/source edits before release golden regression checks.
  - Failed `cargo test --release -p worldwake-ai --test golden_survival_baseline -- --ignored --test-threads=1` after probe tightening: `Agent C` did not commit `Eat`; `Agent B` hunger exceeded critical for 1179 ticks.
  - Failed `cargo test --release -p worldwake-ai --test golden_survival_trade -- --ignored --test-threads=1` after probe tightening: `Buyer Nila` hunger exceeded critical for the substitute market branch.
  - Failed `cargo test --release -p worldwake-ai --test golden_survival_baseline -- --ignored --test-threads=1` after source-reliability-only narrowing: `Agent A` thirst exceeded critical for 550 ticks; stuck idle ticks 1086-1326.
