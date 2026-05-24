# S167COGARCBEH-002: Behavioral-divergence golden with counterfactual symmetry

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: [`archive/tickets/S167COGARCBEH-001.md`](../archive/tickets/S167COGARCBEH-001.md), [`archive/tickets/S167COGARCBEH-001A.md`](../archive/tickets/S167COGARCBEH-001A.md), [`specs/S167-cognitive-archetype-behavioral-proof.md`](../specs/S167-cognitive-archetype-behavioral-proof.md)

## Problem

S152 proved that two same-role agents differing only by archetype resolve
**different profile values** (seven tests in
`crates/worldwake-ai/tests/scenarios/cognitive_archetypes.rs`, 418 lines). It did
not prove the difference propagates to a **divergent decision**: two agents with
identical beliefs choosing different actions under identical local facts. Per
FND-31: "Structural activation is not causal proof." The missing layer is
decision divergence with attribution.

This ticket authors a behavioral-divergence golden at
`crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs` that
loads the dedicated scenario after the S167COGARCBEH-001A route substrate, runs
to the divergence tick, and asserts six sub-assertions including counterfactual
archetype-swap symmetry. Profile-field attribution lives test-side (computed from
the resolved-profile components S152 ships) — no decision-trace surface change.

## Assumption Reassessment (2026-05-24)

1. Existing focused/unit + golden coverage: seven tests in
   `crates/worldwake-ai/tests/scenarios/cognitive_archetypes.rs` prove
   resolved-profile-value divergence. None assert decision/action divergence.
   The new file is a **sibling** of `cognitive_archetypes.rs`, not an extension
   — none of the seven existing tests are modified.
2. The spec
   ([`specs/S167-cognitive-archetype-behavioral-proof.md`](../specs/S167-cognitive-archetype-behavioral-proof.md))
   commits in D1 to six lettered sub-assertions: (a) decision divergence
   (different `GoalKind`/`GoalKey` or same `GoalKey` with different selected plan
   path yielding a different next travel action), (b) trace-side selected-plan
   and route-preference-context divergence, (c) test-side profile-delta
   attribution against the S152 resolved-profile surface, (d) knowledge legality
   (identical decision-side beliefs, equal route-experience memory, no asymmetric
   perception events between spawn and divergence tick), (e) counterfactual
   archetype-swap symmetry, (f) replay/determinism.
3. Shared boundary under audit: route-choice planning. The golden reads the
   existing decision trace surface (`SelectionTrace.selected_opportunity`,
   `SelectedPlanTrace`, and route-preference context emitted from the selected
   plan) as-is. No new fields are added to the trace surface; profile-field
   attribution is computed test-side from `World`-residing resolved-profile
   components.
4. Live `GoalKind` under test: both replays should select
   `AcquireCommodity { commodity: Apple, purpose: SelfConsume, .. }`. The
   divergence is the selected travel path to the resource source: Greedy should
   choose the short previously-mixed direct route, while Cautious should choose
   the slightly longer neutral route after the same concrete route-experience
   memory is priced by each archetype's `RoutePreferenceProfile`.
5. AI regression layer: intended verification layer is golden E2E coverage
   (full action registries required, not local needs-only harness) because
   the divergence depends on the full route-aware planning path. The harness
   must execute through candidate generation → ranking → route-aware search →
   selection → trace emission for both agents in the same tick.
6. Ordering layer: the compared branches diverge at **plan-path search cost**,
   not at candidate emission and not at selected-goal ranking. State this
   explicitly in the golden's preamble per precision rules.
12. Scenario isolation: the scenario must exclude lawful competing
    affordances that would tip the decision independent of archetype —
    e.g., differential hunger, perception of the hostile presence
    differing between agents, route knowledge differing, owned-inventory
    differing. All competing inputs must be symmetric across the two
    agents. The S167COGARCBEH-001A scenario substrate authors symmetric topology
    with direct and alternate routes, but it does not seed generic
    resource-source beliefs or route-experience memory because the live RON
    schema has no such fields. This ticket owns any test-side belief setup and
    identical `AgentDecisionRuntime.route_preference` setup needed before
    asserting no asymmetric inputs slipped through (sub-assertion (d)).

## Architecture Check

1. **Test-side profile-delta attribution over trace-surface extension** —
   FND-31's "authored causal reason" is satisfied by the golden computing
   the resolved profile values for both agents (via the
   `World`-residing per-agent profile components S152 ships) and asserting
   the documented archetype-driven field delta is the decisive factor.
   Extending the decision-trace to carry profile-field names would couple
   trace structure to archetype identity and complicate future
   runtime-adaptation work under FND-22A. The test-side computation reads
   the same authoritative state the route-aware planning pipeline reads, so the
   attribution is equally trustworthy.
2. **Counterfactual symmetry as architectural backstop** — sub-assertion
   (e) (swap archetypes between agents and assert divergence reverses) is
   a metamorphic test in FND-31 vocabulary. It excludes scenario rails,
   agent-specific exception logic, asymmetric scenario seeding, and
   per-agent template wiring errors. Without it, a passing golden could
   silently mean "agent A always picks route X" rather than "Greedy picks
   route X."
3. **Same golden, two simulation runs** — the counterfactual replay loads
   the same scenario file but with the two agents' archetype fields
   swapped. Implementation: either load the scenario twice with a
   per-load override mechanism, or author two scenarios (forward +
   swapped). Prefer per-load override if the scenario loader supports it
   (verify at implementation time); otherwise author the swapped scenario
   alongside the forward one. The seed must be identical across both
   runs.

## Verification Layers

1. Decision divergence (sub-assertion (a)) -> decision trace selected plan
   steps: selected `GoalKey` remains the same while the first travel target
   differs.
2. Route-preference contribution divergence (sub-assertion (b)) -> decision
   trace route-preference context plus selected plan steps for each replay.
3. Profile-delta attribution (sub-assertion (c)) -> authoritative world state
   (test reads `world.get_component_route_preference_profile(agent_id)` for both
   agents and asserts the documented archetype-driven
   `dangerous_traversal_penalty` delta is decisive).
4. Knowledge legality (sub-assertion (d)) -> event-log delta (assert zero
   perception events between spawn tick and divergence tick) and
   authoritative belief-store comparison (both agents' belief stores are
   byte-identical at the divergence tick).
5. Counterfactual symmetry (sub-assertion (e)) -> decision trace from the
   swapped-archetype replay (the first travel target for the swapped Greedy
   agent matches the original Greedy first travel target).
6. Replay determinism (sub-assertion (f)) -> standard golden harness
   determinism check (same seed → byte-identical authoritative state
   across runs).

## What to Change

### 1. Author `crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs`

Author a new golden test file as a sibling of
`cognitive_archetypes.rs`. The file contains one primary test (forward
direction) plus the counterfactual replay test. Both tests share a helper
that:

- Loads `scenarios/cognitive-archetypes-divergence.ron` (with optional
  per-agent archetype override for the swapped replay) and applies any
  test-side belief setup needed to make both agents' decision-side beliefs
  identical under the live schema.
- Seeds identical concrete route-experience memory into each compared replay's
  `AgentDecisionRuntime.route_preference` before planning.
- Runs the simulation through tick `T_divergence` (the first tick at which the
  selected plan path differs; pin during implementation by initial observation
  of the scenario's behavior).
- Captures the per-agent `AgentDecisionTrace` and resolved profile
  components.
- Returns a `DivergenceObservation` struct with the two agents' selected goals,
  selected first travel targets, route-preference context, route-memory entry,
  and resolved `RoutePreferenceProfile` fields.

**Test 1: forward divergence**
- Asserts (a): `greedy.selected_goal == cautious.selected_goal` and the first
  selected travel target differs.
- Asserts (b): the selected plan route-preference context names the direct
  route and exposes the expected preference direction for each archetype.
- Asserts (c): `greedy.route_preference_profile.dangerous_traversal_penalty <
  cautious.route_preference_profile.dangerous_traversal_penalty`, and the
  magnitude is sufficient to tip the direct-vs-neutral perceived travel-cost
  comparison.
- Asserts (d): event log between spawn and divergence tick contains zero
  `Perception*` event payloads for either agent (use whichever payload
  family the perception system emits — verify exact tag names at
  implementation time); both agents' belief stores are byte-identical at
  the divergence tick (use `bincode` or canonical-hash comparison).

**Test 2: counterfactual symmetry**
- Loads the same scenario with the two agents' `archetype` fields
  swapped (Greedy becomes Cautious-named-agent, Cautious becomes
  Greedy-named-agent).
- Asserts (e): the swapped Greedy agent's first travel target matches the
  forward-run Greedy first travel target; same for Cautious. The divergence
  reverses correspondingly.

**Replay determinism (f)** is exercised by the standard golden harness —
the harness re-runs each test with the same seed and asserts byte-identical
authoritative state. No additional assertion needed beyond using the
standard golden harness pattern.

### 2. Pin the divergence tick during implementation

The scenario's exact divergence tick `T_divergence` depends on
the S167COGARCBEH-001A route topology, hunger seeding, test-side route memory,
and travel distance. Implementation steps:

1. Run the scenario locally and observe at which tick the two agents' selected
   first travel targets first differ while the selected goal remains the same.
2. Pin that tick value in the helper's `run_to_divergence(world,
   T_divergence)` call.
3. Add a regression-guard assertion that the divergence tick value
   matches `T_divergence` exactly (so a future profile retune that
   shifts the divergence later/earlier fails loudly with a clear
   message).

### 3. Register the new golden in the test discovery layer

The file is picked up automatically by the workspace test runner if it
follows the existing `crates/worldwake-ai/tests/scenarios/` naming
convention. Verify by running `cargo test -p worldwake-ai --test
golden_ai -- --list | grep cognitive_archetypes_divergence` after
landing the file — the two tests must appear.

## Files to Touch

- `crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modify — likely; verify
  the test-file inclusion convention by inspecting how `cognitive_archetypes.rs`
  is registered)
- Likely: golden harness helper or shared scenario-loading helper if the
  per-agent archetype override mechanism for the counterfactual replay
  requires a shared utility. Discovery: `grep -rn "load_scenario_file\|spawn_scenario" crates/worldwake-ai/tests/` to find the test-side scenario load entry point.

## Out of Scope

- Authoring the scenario file itself — completed in
  [`archive/tickets/S167COGARCBEH-001.md`](../archive/tickets/S167COGARCBEH-001.md).
- Coverage doc regeneration — completed in
  [`archive/tickets/S167COGARCBEH-001.md`](../archive/tickets/S167COGARCBEH-001.md).
- Roadmap formalization — owned by S167COGARCBEH-003.
- CI workflow lane — owned by S167COGARCBEH-004.
- Extending `decision_trace.rs` with new fields naming profile deltas —
  explicit Non-Goal in the spec; attribution lives test-side.
- Modifying the existing seven `cognitive_archetypes.rs` profile-value
  tests.
- Adding new archetype variants, templates, or profile fields.
- Asserting on archetype pairs other than `Greedy vs Cautious` — future
  pairs may land as additional tests in this file or as sibling files
  per the spec's Follow-ups section.

## Acceptance Criteria

### Tests That Must Pass

1. `cognitive_archetypes_divergence::forward` — forward divergence test
   asserts (a)/(b)/(c)/(d) per the structure above.
2. `cognitive_archetypes_divergence::counterfactual_symmetry` — swapped
   replay asserts (e) per the structure above.
3. Replay determinism (f) is exercised automatically by the standard
   golden harness's same-seed byte-identical-state check.
4. Existing suite: `cargo test -p worldwake-ai` — all seven existing
   `cognitive_archetypes.rs` tests still pass unchanged.
5. `cargo test --workspace` passes.

### Invariants

1. Both tests run from the same scenario file
   (`scenarios/cognitive-archetypes-divergence.ron`); the counterfactual
   replay uses an explicit per-agent archetype override, not a separate
   scenario file (unless the loader requires the latter — pin at
   implementation time).
2. The two agents' belief stores are byte-identical at the divergence
   tick (asserted by sub-assertion (d)).
3. The divergence assertion uses the existing decision-trace surface
   (`selected_opportunity`, `SelectedPlanTrace`, and route-preference context)
   without adding new fields.
4. Profile-field attribution is read from authoritative world-residing
   profile components, not from any trace field naming the profile field
   by string.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs`
   (new) — two tests: `forward` proves decision divergence + trace
   contribution divergence + test-side profile attribution + knowledge
   legality; `counterfactual_symmetry` proves the archetype-swap reverses
   the divergence. Together they satisfy D1(a)–(f).

### Commands

1. `cargo test -p worldwake-ai --test golden_ai
   scenarios::cognitive_archetypes_divergence` (targeted; the two new
   tests)
2. `cargo test -p worldwake-ai` (verify no regression in the existing
   `cognitive_archetypes.rs` profile-value tests)
3. `scripts/verify.sh` (full pre-PR gate)
