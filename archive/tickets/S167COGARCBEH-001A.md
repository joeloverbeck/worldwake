# S167COGARCBEH-001A: Lawful route-preference substrate for archetype divergence

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: [`archive/tickets/S167COGARCBEH-001.md`](S167COGARCBEH-001.md), [`archive/specs/S167-cognitive-archetype-behavioral-proof.md`](../specs/S167-cognitive-archetype-behavioral-proof.md)

## Problem

`S167COGARCBEH-002` was drafted around a Greedy-vs-Cautious
economic-vs-safety selected-goal divergence. Live reassessment disproved that
premise: the archived `scenarios/cognitive-archetypes-divergence.ron` scenario,
even with identical test-side beliefs and activated hunger pressure, yields the
same selected goals for isolated Greedy and Cautious replays through tick 31, and
the selected summaries carry zero `Greed` contribution. The live planner also
keeps search order tied to ranked candidate preference; `PortfolioWeightsProfile`
is a trace/probe/cap input and does not by itself steer selected-goal ordering.

This prerequisite retargeted the lawful behavioral substrate before the golden
was implemented. It initially exposed a same-goal, different-plan-path decision
boundary through ordinary route topology. The later
[`S167COGARCBEH-002`](S167COGARCBEH-002.md) golden tightened that substrate to
two equally distant one-hop apple sources and proved that identical concrete
route-experience memory plus the archetype-resolved
`RoutePreferenceProfile.dangerous_traversal_penalty` makes Greedy prefer the
mixed-history Risky Orchard route while Cautious prefers the neutral Sheltered
Cut source.

## Assumption Reassessment (2026-05-24)

1. The original economic-vs-safety premise is false on the live branch.
   Exploratory focused golden work showed isolated Greedy and Cautious replays
   selected `AcquireCommodity(Apple)` at ticks 0 and 3 and
   `ConsumeOwnedCommodity(Apple)` at ticks 6, 7, and 9; no selected summary had a
   nonzero `MotiveSource::Greed` contribution.
2. The live planner contract in
   `crates/worldwake-ai/src/agent_tick/planning.rs` builds portfolio slots and
   records `plausible_slots_by_score`, but its `search_order` follows
   `ranking::compare_ranked_goals` over admitted candidates. A proof that relies
   on economic portfolio weight alone would regress to structural activation, not
   FND-31 causal proof.
3. The exact shared boundary under audit is route-choice planning:
   `AgentDecisionRuntime.route_preference` plus the authoritative
   `RoutePreferenceProfile` component feed
   `build_candidate_plans_with_route_preference`, then
   `PlanningSnapshot::direct_perceived_travel_cost` and HTN/GOAP travel search.
4. Intended invariant: with the same goal, same beliefs, same topology, and same
   concrete route-experience memory, Greedy and Cautious may choose different
   first travel steps because their concrete archetype-resolved
   `RoutePreferenceProfile.dangerous_traversal_penalty` differs. The later golden
   must prove the path divergence through decision trace plan steps, not just
   resolved profile values.
5. Live `GoalKind` under test remains `AcquireCommodity { commodity: Apple,
   purpose: SelfConsume, .. }`. The exact operator surface is the travel
   prerequisite path to the resource source, not candidate emission or greed
   motive ranking.
6. AI regression layer is golden E2E with full action registries. Local
   needs-only or portfolio-only tests are insufficient because the behavior is in
   candidate generation -> ranking -> route-aware search -> selected plan.
7. Ordering layer is plan-path search cost, not selected-goal ranking. The
   compared branches are symmetric except for the archetype-resolved
   route-preference profile. Divergence must not depend on actor tick order,
   resource contention, or asymmetric perception.
8. No heuristic is removed or weakened. The substrate uses existing route
   preference state and existing perceived-travel-cost arithmetic.
9. Current scenario isolation is insufficient because it has only one route from
   Market Green to Risky Orchard. This ticket adds an alternate neutral route so
   route preference can lawfully affect plan path.
10. Adjacent contradiction classified as current-ticket prerequisite: the active
    spec and `S167COGARCBEH-002` still describe economic-vs-safety selected-goal
    divergence. This ticket must truth-sync that wording to route-preference
    plan-path divergence before implementation continues.

## Architecture Check

1. Route preference is a cleaner substrate than changing portfolio search order
   because it already represents concrete agent-local learned state and already
   participates in route-aware planning. The divergence explanation is "same
   remembered route history, different concrete penalty profile," which matches
   FND-20, FND-22, FND-22A, and FND-31.
2. No backwards-compatibility aliasing or archetype-specific rail is introduced.
   Greedy/Cautious remain ordinary archetype templates resolved into ordinary
   profile components.

## Verified Layers

1. Scenario route substrate exists -> scenario parse/spawn and topology inspection
   in the later golden.
2. Profile delta exists -> authoritative world state
   (`RoutePreferenceProfile.dangerous_traversal_penalty` for Greedy vs Cautious).
3. Equal concrete route memory is available -> later golden seeds identical
   `AgentDecisionRuntime.route_preference` entries before planning and asserts the
   same route-experience counts for both replays.
4. Plan-path divergence -> later golden decision trace selected plan steps.
5. No candidate/ranking divergence dependency -> later golden asserts selected
   `GoalKey` remains the same while first travel step differs.

## Landed Changes

### 1. Retarget S167 prose to route-preference plan-path divergence

Update the active spec and `S167COGARCBEH-002` so D1 and the ticket no longer
claim a false economic-vs-safety selected-goal premise. The behavioral proof is a
same-goal, different-plan-path divergence caused by
`RoutePreferenceProfile.dangerous_traversal_penalty`.

### 2. Add the alternate route substrate to the S167 scenario

At this ticket's completion, modified
`scenarios/cognitive-archetypes-divergence.ron` so Market Green had both:

- a short direct route to Risky Orchard, and
- a slightly longer neutral route through an intermediate place.

The route costs allow the later golden to seed equal route history where
Greedy's lower dangerous-traversal penalty makes the direct route cheaper, while
Cautious's higher penalty makes the neutral route cheaper.

Outcome amended: 2026-05-24. The later
[`S167COGARCBEH-002`](S167COGARCBEH-002.md) golden refined this substrate to two
equally distant one-hop apple sources because the live route-aware search did
not expose a stable multi-hop selected terminal for the proof.

## Landed Files

- `archive/specs/S167-cognitive-archetype-behavioral-proof.md` (modified before archival)
- `archive/tickets/S167COGARCBEH-002.md` (later modified and archived by the
  completed golden ticket)
- `scenarios/cognitive-archetypes-divergence.ron` (modify)
- `.codex/run-state/implement-spec-tickets.json` (modify)

## Out of Scope

- Implementing the final behavioral-divergence golden; remains owned by
  `S167COGARCBEH-002`.
- Changing planner search order, portfolio semantics, archetype templates, or
  decision-trace schema.
- Adding scenario-authored route-preference memory. The later golden may seed
  identical concrete `AgentDecisionRuntime.route_preference` state as test-side
  setup, just as it seeds belief state.

## Acceptance Result

### Tests Passed

1. Passed: `cargo test -p worldwake-ai --test golden_ai -- --list` shows the active golden
   suite still discovers.
2. Passed: `cargo test -p worldwake-cli scenario::tests::test_spawn_applies_archetype_deltas_and_emits_assignment_events` still proves archetype profile resolution.
3. Passed: Existing suite `cargo test -p worldwake-ai --test golden_ai scenarios::cognitive_archetypes`
4. Passed: `cargo run -p worldwake-cli --bin scenario-coverage -- --check` passes after
   regenerating coverage docs.

### Invariants

1. The S167 active spec and active tickets no longer claim the disproven
   economic-vs-safety selected-goal premise.
2. The substrate preserves lawful same-belief divergence: no agent-specific
   scenario rail, no omniscient planner input, and no portfolio-ordering behavior
   change.

## Verification Result

### Test Surface

1. Passed: `None — substrate and ticket/spec truthing only. The behavioral golden is owned
   by S167COGARCBEH-002.`

### Commands Run

1. Passed: `cargo test -p worldwake-cli scenario::tests::test_spawn_applies_archetype_deltas_and_emits_assignment_events`
2. Passed: `cargo test -p worldwake-ai --test golden_ai scenarios::cognitive_archetypes`
3. Passed: `cargo test -p worldwake-ai --test golden_ai -- --list`
4. Passed: `cargo run -p worldwake-cli --bin scenario-coverage -- --write`
5. Passed: `cargo run -p worldwake-cli --bin scenario-coverage -- --check`

## Outcome

Completed: 2026-05-24

Outcome amended: 2026-05-24. The later
[`S167COGARCBEH-002`](S167COGARCBEH-002.md) golden refined the scenario
substrate from a direct-vs-multi-hop route comparison to two equally distant
one-hop apple sources. This keeps the prerequisite's lawful same-goal
route-preference proof boundary intact while matching the implemented golden.

Implemented:

- Updated the active S167 spec from the disproven economic-vs-safety
  selected-goal premise to a same-goal, different-plan-path route-preference
  proof.
- Updated `S167COGARCBEH-002` so the behavioral golden depends on this
  prerequisite and asserts `RoutePreferenceProfile.dangerous_traversal_penalty`
  attribution, selected plan path divergence, and counterfactual symmetry.
- At this ticket's completion, modified
  `scenarios/cognitive-archetypes-divergence.ron` to provide a direct route and
  a slightly longer alternate route from Market Green to Risky Orchard, and
  removed the old public threat-warning artifacts so route-choice cost was not
  confounded by destination threat estimates. The later S167COGARCBEH-002 golden
  refined this to two equally distant one-hop apple sources and archived the
  landed proof.
- Regenerated `docs/generated/scenario-coverage.md` after the scenario topology
  change.
- Refreshed `.codex/run-state/implement-spec-tickets.json` so the S167 harness
  clears the blocker and processes this prerequisite before returning to
  `S167COGARCBEH-002`.

Deviations:

- Did not add scenario-authored route-preference memory. The live RON schema has
  no such field, and adding one would be a broader scenario-schema feature. The
  downstream golden may seed identical concrete `AgentDecisionRuntime.route_preference`
  state as test-side setup.

Verification:

- `cargo test -p worldwake-cli scenario::tests::test_spawn_applies_archetype_deltas_and_emits_assignment_events`
- `cargo test -p worldwake-ai --test golden_ai scenarios::cognitive_archetypes`
- `cargo test -p worldwake-ai --test golden_ai -- --list`
- `cargo run -p worldwake-cli --bin scenario-coverage -- --write`
- `cargo run -p worldwake-cli --bin scenario-coverage -- --check`
