# S35OBSACTSIG-007: Opportunity-scoped competition selection and golden regression

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — preserve opportunity identity through same-goal planning and selection
**Deps**: S35OBSACTSIG-001 through S35OBSACTSIG-006

## Problem

S35's observable-activity pipeline is implemented through perception, belief view, ranking, and tracing, but the intended end-to-end behavior still fails for sibling opportunities that share the same `GoalKey`.

The live path is:

`ActionInstance` at a co-located place -> perception records `BelievedActivity` -> `GoalBeliefView::agents_active_at(place, ActionDomain::Production, None)` exposes the believed competitor -> `rank_candidates()` applies an opportunity-scoped production competition discount.

However, plan selection currently collapses sibling opportunities back to `GoalKey`-scoped scores. That means a discounted local branch can still be selected over a higher-ranked remote sibling. The ticket is therefore not just missing golden coverage; it is missing the production fix that preserves opportunity identity from ranking into selection.

## Assumption Reassessment (2026-03-29)

1. The production-side observable-activity implementation already exists in live code. The shared abstraction boundary under audit is: `BelievedEntityState.believed_activity` in [crates/worldwake-core/src/belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) -> `RuntimeBeliefView::agents_active_at` in [crates/worldwake-sim/src/per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs) -> `apply_competition_discount` / `competition_discount_scope` in [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs).
2. The original "central place chooses Orchard B because Orchard A is occupied" narrative is still wrong for the shipped architecture. The observer must be co-located with the observed active action to lawfully acquire the belief (P7 locality). The golden must start the deciding agent at the occupied local place or re-enable them there after the competitor has lawfully started.
3. The live discount surface is narrower than the original narrative. `competition_discount_scope` currently discounts only `GoalKind::ProduceCommodity` and `GoalKind::RestockCommodity`, and only by `OpportunityAnchor::Place(place)` plus `ActionDomain::Production`. This ticket must not claim trade-target behavior.
4. The live query surface is place-anchored, not workstation-target-anchored. `apply_competition_discount` calls `view.agents_active_at(place, domain, None)`, so the golden must prove "occupied local production place redirects to an uncontested remote sibling place," not "choose workstation B over workstation A within the same place."
5. Golden production scenarios already live in [crates/worldwake-ai/tests/golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs). Creating a dedicated new integration test file would be unnecessary fragmentation.
6. The current golden harness already exposes the required surfaces: `GoldenHarness`, `driver.enable_tracing()`, `enable_action_tracing()`, and snapshot/replay support in [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs).
7. The candidate/selection split is where the architecture diverges. In the live failing scenario, the local occupied `RestockCommodity { commodity: Apple }` opportunity is correctly discounted from `900000` to `450000` while the remote sibling remains `900000`, yet the selected opportunity still remains local. The first contradiction is caused by [crates/worldwake-ai/src/plan_selection.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs) keying candidate scores by `GoalKey` instead of full `OpportunityKey`.
8. A second live divergence appears one layer earlier. [crates/worldwake-ai/src/agent_tick/planning.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) stopped planning after the first found plan, which meant later same-goal siblings could never reach selection in the first place. The clean fix is not "plan every candidate forever"; it is to continue through later contiguous siblings that share the same `GoalKey`, then stop once the ranked stream moves on to a different goal.
9. This contradiction is in scope, not adjacent cleanup. The same fact currently has one clean canonical transport path up through ranking, and then unlawful identity collapse/short-circuiting inside planning and selection. The clean architecture is to remove those collapses, not to add heuristics or soften the golden.
10. Architecture judgment: the shipped place-level competition discount is cleaner than the original ticket's source-target narrative because it respects locality and matches the current opportunity identity (`OpportunityAnchor::Place`). The parts that were not clean were the later collapse from opportunity identity back to `GoalKey` during selection and the early stop after the first found sibling during planning. Both should be removed rather than worked around.

## Architecture Check

1. Extending [golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs) is cleaner than creating a one-off file because the behavior under test is production contention and shares setup helpers, recipe registry, and scenario inventory conventions already established there.
2. The correct production fix is to preserve opportunity-scoped scores through selection and preserve same-goal sibling opportunity reachability through planning. Re-ranking inside planning, adding ad-hoc tie-breakers, or weakening the golden would all be workarounds that leave the architecture internally contradictory.
3. A same-place-observation plus remote-sibling-redirection scenario is still the cleanest end-to-end proof surface because it exercises the actual canonical information path and the exact selection boundary that currently loses opportunity identity.
4. A low-awareness control remains useful because it proves the divergence is driven by `activity_awareness_weight`, not by generic contention handling or source depletion. This directly exercises P20 agent diversity.
5. A broader redesign toward workstation-scoped competition remains out of scope. The necessary fix here is narrower and cleaner: remove the `GoalKey` collapse in selection.

## Verification Layers

1. Co-located competitor becomes believed activity at the occupied place -> perception/belief traceability via the ranked local opportunity's `competition_discount`.
2. Same-goal sibling opportunities remain eligible through planning and opportunity-scoped ranking survives into selection -> focused `plan_selection` regression plus end-to-end golden.
3. High-awareness agent redirects to the remote sibling place instead of continuing with the occupied local place -> decision trace selected opportunity / selected plan, then authoritative source depletion at the remote sibling.
4. Low-awareness control remains on the occupied local place -> decision trace still shows the observed competitor, but with zero effective discount and local selected opportunity.
5. Determinism of the end-to-end scenario -> replay companion.

## What to Change

### 1. Preserve same-goal opportunity identity through planning and selection

Update [crates/worldwake-ai/src/plan_selection.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs) so sibling opportunities that share a `GoalKey` do not overwrite each other's ranked `(priority_class, motive_score)` data. The clean fix is to key selection-time scores by full `OpportunityKey` or an equivalent opportunity-scoped carrier.

Update [crates/worldwake-ai/src/agent_tick/planning.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) so a found plan only short-circuits once the ranked stream moves past the current `GoalKey`; later contiguous same-goal siblings must still be searched.

Add focused regression coverage for the selection boundary and end-to-end coverage for the planning+selection combination.

### 2. Add production goldens to the existing file

Add the missing S35 production-competition scenarios to [crates/worldwake-ai/tests/golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs).

### 3. High-awareness remote-redirection scenario

Set up:

- occupant starts a local harvest first
- observer is then re-enabled at the same local place
- observer has `PerceptionProfile.observation_fidelity = 1000`
- observer is configured for live `GoalKind::RestockCommodity { commodity: Apple }`
- observer has `UtilityProfile.activity_awareness_weight = 500`
- local and remote sibling production opportunities are both known and lawful

Assertions:

- decision trace records `competition_discount` on the occupied local opportunity with `domain == ActionDomain::Production`
- the discounted local opportunity's motive is lower than the remote sibling's motive
- selected opportunity is the remote sibling after the selection fix
- travel starts and the remote sibling source is eventually used

### 4. Low-awareness local-control scenario

Use the same shape, but with `activity_awareness_weight = 0`.

Assertions:

- the observed-competition trace remains present, but with `effective_discount = 0`
- selected opportunity remains the occupied local sibling
- local start failure / collision remains visible, proving no avoidance occurred

### 5. Regenerate golden inventory docs

Run `python3 scripts/golden_inventory.py --write --check-docs` after adding the scenario metadata block(s).

## Files to Touch

- [crates/worldwake-ai/src/plan_selection.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs)
- [crates/worldwake-ai/src/agent_tick/planning.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
- [crates/worldwake-ai/tests/golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs)
- [docs/generated/golden-e2e-inventory.md](/home/joeloverbeck/projects/worldwake/docs/generated/golden-e2e-inventory.md)
- [docs/generated/golden-scenario-map.md](/home/joeloverbeck/projects/worldwake/docs/generated/golden-scenario-map.md)

## Out of Scope

- Trade competition goldens
- Workstation-target-scoped competition redesign
- Any refactor of opportunity identity from place-scoped to workstation-scoped
- More-than-sibling-candidate crowd heuristics or performance work

## Acceptance Criteria

### Tests That Must Pass

1. focused `plan_selection` regression proving same-goal sibling opportunities use opportunity-scoped scores
2. `golden_observed_harvest_competition_redirects_to_remote_sibling`
3. `golden_observed_harvest_competition_redirects_to_remote_sibling_replays_deterministically`
4. `golden_zero_activity_awareness_does_not_avoid_observed_harvest_competition`
5. Existing `cargo test -p worldwake-ai` suite

### Invariants

1. The deciding agent changes behavior based on believed local activity, not by querying authoritative remote occupancy.
2. The occupied local place remains lawful; the agent is redirected by ranking discount, not by suppression or authoritative invalidation.
3. Same-goal sibling opportunities remain reachable through planning and opportunity-scoped ranking survives into selection.
4. `activity_awareness_weight` is the branch-divergence control surface.
5. Replay produces the same outcome from the same seed.

## Tests

### New/Modified Tests

1. focused `plan_selection` regression for same-goal sibling opportunities
Rationale: proves the root bug directly at the strongest lower layer by ensuring selection uses opportunity-scoped scores instead of collapsing to `GoalKey`.
2. `golden_observed_harvest_competition_redirects_to_remote_sibling`
Rationale: proves the end-to-end S35 production pipeline after same-goal sibling planning and opportunity-scoped selection are corrected.
3. `golden_observed_harvest_competition_redirects_to_remote_sibling_replays_deterministically`
Rationale: proves the redirection branch is deterministic under replay, not a one-run artifact.
4. `golden_zero_activity_awareness_does_not_avoid_observed_harvest_competition`
Rationale: proves the divergence is driven by `activity_awareness_weight`, not by generic contention handling.

### Commands

1. `cargo test -p worldwake-ai same_goal_sibling_opportunity_selection_uses_opportunity_scoped_scores`
2. `cargo test -p worldwake-ai golden_observed_harvest_competition_redirects_to_remote_sibling`
3. `cargo test -p worldwake-ai golden_zero_activity_awareness_does_not_avoid_observed_harvest_competition`
4. `cargo test -p worldwake-ai --test golden_production`
5. `python3 scripts/golden_inventory.py --write --check-docs`
6. `cargo test -p worldwake-ai`
7. `cargo test --workspace`
8. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-29
- What actually changed: fixed [crates/worldwake-ai/src/plan_selection.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs) to score/select by `OpportunityKey` rather than collapsing sibling branches to `GoalKey`; fixed [crates/worldwake-ai/src/agent_tick/planning.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) to keep planning contiguous later siblings for the same `GoalKey` after the first found plan; added the observable-competition production goldens in [crates/worldwake-ai/tests/golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs); refreshed generated golden inventory docs.
- Deviations from original plan: the ticket started as a selection-only correction, but reassessment against live code showed that selection was not the only architectural collapse. A same-goal planning short-circuit also had to be corrected. The final high-awareness scenario uses a local occupied place plus a remote sibling place and `RestockCommodity { commodity: Apple }`, not the older orchard/source-target narrative.
- Verification results: `cargo test -p worldwake-ai same_goal_sibling_opportunity_selection_uses_opportunity_scoped_scores`; `cargo test -p worldwake-ai golden_observed_harvest_competition_redirects_to_remote_sibling`; `cargo test -p worldwake-ai golden_observed_harvest_competition_redirects_to_remote_sibling_replays_deterministically`; `cargo test -p worldwake-ai golden_zero_activity_awareness_does_not_avoid_observed_harvest_competition`; `cargo test -p worldwake-ai --test golden_production`; `python3 scripts/golden_inventory.py --write --check-docs`; `cargo test -p worldwake-ai`; `cargo test --workspace`; `cargo clippy --workspace`.
