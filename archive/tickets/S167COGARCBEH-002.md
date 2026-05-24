# S167COGARCBEH-002: Behavioral-divergence golden with counterfactual symmetry

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: [`archive/tickets/S167COGARCBEH-001.md`](S167COGARCBEH-001.md), [`archive/tickets/S167COGARCBEH-001A.md`](S167COGARCBEH-001A.md), [`specs/S167-cognitive-archetype-behavioral-proof.md`](../../specs/S167-cognitive-archetype-behavioral-proof.md)

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
   ([`specs/S167-cognitive-archetype-behavioral-proof.md`](../../specs/S167-cognitive-archetype-behavioral-proof.md))
   commits in D1 to six lettered sub-assertions: (a) decision divergence
   (different `GoalKind`/`GoalKey` or same `GoalKey` with different selected plan
   path yielding a different next travel action), (b) trace-side selected-plan
   and route-preference-context divergence, (c) test-side profile-delta
   attribution against the S152 resolved-profile surface, (d) knowledge legality
   (identical decision-side known-entity beliefs and equal route-experience
   memory before the divergence tick), (e) counterfactual archetype-swap
   symmetry, (f) replay/determinism.
3. Shared boundary under audit: route-choice planning. The golden reads the
   existing decision trace surface (`SelectionTrace.selected_opportunity`,
   `SelectedPlanTrace`, and selected-plan search provenance) as-is. No new
   fields are added to the trace surface; profile-field
   attribution is computed test-side from `World`-residing resolved-profile
   components.
4. Live `GoalKind` under test: both replays should select
   `AcquireCommodity { commodity: Apple, purpose: SelfConsume, .. }`. The
   divergence is the selected travel path to one of two apple sources: Greedy
   should choose the mixed-history Risky Orchard route, while Cautious should
   choose the equally distant neutral Sheltered Cut source after the same
   concrete route-experience memory is priced by each archetype's
   `RoutePreferenceProfile`.
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
    differing between agents, route knowledge differing, hunger pressure
    differing, owned-inventory differing. All competing inputs must be
    symmetric across the two agents. The S167COGARCBEH-001A scenario substrate
    authors symmetric one-hop apple-source topology, but it does not seed
    generic resource-source beliefs, hunger pressure for candidate emission, or
    route-experience memory because the live RON schema has no such fields.
    This ticket owns symmetric test-side hunger setup, known-entity belief
    setup, and identical `AgentDecisionRuntime.route_preference` setup before
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

## Verified Layers

1. Decision divergence (sub-assertion (a)) -> decision trace selected plan
   steps: selected `GoalKey` remains the same while the first travel target
   differs.
2. Route-preference contribution divergence (sub-assertion (b)) -> decision
   trace selected-plan search provenance plus selected plan steps for each
   replay.
3. Profile-delta attribution (sub-assertion (c)) -> authoritative world state
   (test reads `world.get_component_route_preference_profile(agent_id)` for both
   agents and asserts the documented archetype-driven
   `dangerous_traversal_penalty` delta is decisive).
4. Knowledge legality (sub-assertion (d)) -> authoritative belief-store
   comparison of the seeded known-entity beliefs plus identical test-side route
   memory before the divergence tick.
5. Counterfactual symmetry (sub-assertion (e)) -> decision trace from the
   swapped-archetype replay (the first travel target for the swapped Greedy
   agent matches the original Greedy first travel target).
6. Replay determinism (sub-assertion (f)) -> the forward test runs the same
   scenario/setup twice and compares the resulting observation plus
   post-divergence state hash.

## Landed Changes

### 1. Authored `crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs`

Authored a golden test file as a sibling of
`cognitive_archetypes.rs`. The file contains one primary test (`forward`)
plus the counterfactual replay test (`counterfactual_symmetry`). Both tests
share a helper that:

- Loads `scenarios/cognitive-archetypes-divergence.ron` (with optional
  per-agent archetype override for the swapped replay) and applies any
  test-side belief setup needed to make both agents' decision-side beliefs
  identical under the live schema.
- Sets identical hunger pressure high enough to emit the apple-acquisition
  candidate, then seeds identical concrete route-experience memory into each
  compared replay's `AgentDecisionRuntime.route_preference` before planning.
- Runs the simulation through the pinned divergence tick `Tick(0)`, where the
  selected plan path differs.
- Captures the per-agent `AgentDecisionTrace` and resolved profile
  components.
- Returns a `DivergenceObservation` struct with the two agents' selected goals,
  selected first travel targets, selected-search route provenance,
  route-memory entry, and resolved `RoutePreferenceProfile` fields.

**Test 1: forward divergence**
- Asserts (a): `greedy.selected_goal == cautious.selected_goal` and the first
  selected travel target differs.
- Asserts (b): selected-plan search provenance names the direct route and
  exposes the expected preference direction for each archetype.
- Asserts (c): `greedy.route_preference_profile.dangerous_traversal_penalty <
  cautious.route_preference_profile.dangerous_traversal_penalty`, and the
  magnitude is sufficient to tip the direct-vs-neutral perceived travel-cost
  comparison.
- Asserts (d): both agents' seeded known-entity belief maps hash identically
  before the divergence tick, and both agents receive the same test-side route
  memory.

**Test 2: counterfactual symmetry**
- Loads the same scenario with the two agents' `archetype` fields
  swapped (Greedy becomes Cautious-named-agent, Cautious becomes
  Greedy-named-agent).
- Asserts (e): the swapped Greedy agent's first travel target matches the
  forward-run Greedy first travel target; same for Cautious. The divergence
  reverses correspondingly.

**Replay determinism (f)** is exercised by the forward test by re-running the
same seed, scenario, seeded beliefs, route memory, and hunger setup, then
asserting the resulting observation and post-divergence state hash are
identical.

### 2. Pinned the divergence tick

The scenario's exact divergence tick depends on the S167COGARCBEH-001A route
topology, symmetric test-side hunger seeding, test-side route memory, and
travel distance. The landed helper pins `DIVERGENCE_TICK` to `Tick(0)` and
asserts the scheduler advances exactly one planning tick to `Tick(1)`.

The route substrate was tightened to two equally distant one-hop apple sources
because live route-aware search did not expose a stable multi-hop selected
terminal for this proof. The golden now compares the mixed-history Risky
Orchard route against the neutral Sheltered Cut source.

### 3. Registered the golden in the test discovery layer

The file is registered from `crates/worldwake-ai/tests/scenarios/mod.rs`, so
the workspace test runner executes both landed tests through the existing
`golden_ai` target.

### 4. Refreshed generated scenario coverage

Regenerated `docs/generated/scenario-coverage.md` after tightening the scenario
fixture from one facility/resource source to two. The cognitive-archetypes row
was already present from S167COGARCBEH-001A; this ticket refreshes the generated
per-scenario facility/source counts so `scenario-coverage --check` passes.

## Landed Files

- `crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modified)
- `scenarios/cognitive-archetypes-divergence.ron` (modified)
- `docs/generated/scenario-coverage.md` (modified)
- `specs/S167-cognitive-archetype-behavioral-proof.md` (modified to keep D1/D2 wording aligned with the landed one-hop route proof)

## Out of Scope

- Authoring the scenario file itself — completed in
  [`archive/tickets/S167COGARCBEH-001.md`](S167COGARCBEH-001.md).
- Initial coverage doc regeneration — completed in
  [`archive/tickets/S167COGARCBEH-001.md`](S167COGARCBEH-001.md).
- Roadmap formalization — completed in
  [`S167COGARCBEH-003`](S167COGARCBEH-003.md).
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

1. `cognitive_archetypes_divergence::forward` — landed forward divergence test
   asserts (a)/(b)/(c)/(d) per the structure above.
2. `cognitive_archetypes_divergence::counterfactual_symmetry` — landed swapped
   replay asserts (e) per the structure above.
3. Replay determinism (f) is exercised by the forward test's same-seed
   observation and state-hash comparison.
4. Existing suite: `cargo test -p worldwake-ai` — all seven existing
   `cognitive_archetypes.rs` tests still pass unchanged.
5. `cargo test --workspace` passes.

### Invariants

1. Both tests run from the same scenario file
   (`scenarios/cognitive-archetypes-divergence.ron`); the counterfactual
   replay uses an explicit per-agent archetype override, not a separate
   scenario file (unless the loader requires the latter — pin at
   implementation time).
2. The two agents' seeded known-entity belief maps hash identically before the
   divergence tick (asserted by sub-assertion (d)).
3. The divergence assertion uses the existing decision-trace surface
   (`selected_opportunity`, `SelectedPlanTrace`, and selected-plan search
   provenance) without adding additional fields.
4. Profile-field attribution is read from authoritative world-residing
   profile components, not from any trace field naming the profile field
   by string.

## Outcome

Landed the behavioral-divergence golden for S167. Greedy and Cautious now
select the same apple-acquisition goal with identical seeded known-entity
beliefs and identical route memory, then diverge only in the selected first
travel target because their resolved `RoutePreferenceProfile` values price the
mixed-history Risky Orchard route differently. The counterfactual replay swaps
only the two `AgentDef.archetype` assignments and proves the route decision
follows the archetype rather than the authored agent name.

No engine source or decision-trace field was added. The proof reads existing
selected-plan search provenance and computes profile attribution test-side from
authoritative resolved-profile components.

## Verification Result

1. Passed `cargo test -p worldwake-ai --test golden_ai scenarios::cognitive_archetypes_divergence -- --nocapture` (2 passed; proves the two landed golden tests directly).
2. Passed `cargo test -p worldwake-ai` (package suite passed, including the existing seven `cognitive_archetypes.rs` tests).
3. Passed `cargo test --workspace` (workspace suite passed before the full scripted gate).
4. Passed `cargo run -p worldwake-cli --bin scenario-coverage -- --write` (regenerated the coverage doc after fixture tightening).
5. Passed `./scripts/verify.sh` after regeneration (fmt check, workspace tests, repository shell checks, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and scenario coverage check).
