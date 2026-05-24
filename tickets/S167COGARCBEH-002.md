# S167COGARCBEH-002: Behavioral-divergence golden with counterfactual symmetry

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S167COGARCBEH-001, [`specs/S167-cognitive-archetype-behavioral-proof.md`](../specs/S167-cognitive-archetype-behavioral-proof.md)

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
loads the dedicated scenario (S167COGARCBEH-001), runs to the divergence tick,
and asserts six sub-assertions including counterfactual archetype-swap
symmetry. Profile-field attribution lives test-side (computed from the
resolved-profile components S152 ships) — no decision-trace surface change.

## Assumption Reassessment (2026-05-24)

1. Existing focused/unit + golden coverage: seven tests in
   `crates/worldwake-ai/tests/scenarios/cognitive_archetypes.rs` prove
   resolved-profile-value divergence (e.g.,
   `cognitive_archetypes_greedy_resolves_higher_economic_weight_than_cautious`
   at line 255 documents the `portfolio_weights` economic-weight delta this
   ticket's divergence depends on). None assert decision/action divergence.
   The new file is a **sibling** of `cognitive_archetypes.rs`, not an
   extension — none of the seven existing tests are modified.
2. The spec
   ([`specs/S167-cognitive-archetype-behavioral-proof.md`](../specs/S167-cognitive-archetype-behavioral-proof.md))
   commits in D1 to six lettered sub-assertions: (a) decision divergence
   (different `GoalKind`/`GoalKey` or different ranked order yielding
   different next-tick action), (b) trace-side
   `motive_source_contributions` divergence, (c) test-side profile-delta
   attribution against the S152 resolved-profile surface, (d) knowledge
   legality (identical seeded beliefs, no perception events between spawn
   and divergence tick), (e) counterfactual archetype-swap symmetry, (f)
   replay/determinism.
3. Shared boundary under audit: the decision-trace surface
   (`crates/worldwake-ai/src/decision_trace.rs` —
   `RankedGoalSummary.motive_source_contributions: Vec<(MotiveSourceRef,
   u32)>` at line 675, `SelectionTrace.selected_opportunity` at line 1360,
   `SelectedPlanTrace` at line 1397) is read as-is. No new fields are added
   to the trace surface; profile-field attribution is computed test-side
   from `World`-residing resolved-profile components.
4. Live `GoalKind` under test: the divergence tick's selected
   `GoalKind`/`GoalKey` differs between Greedy and Cautious agents because
   the marginal economic-vs-safety tension authored in
   `scenarios/cognitive-archetypes-divergence.ron` (S167COGARCBEH-001) ranks
   the acquisition opportunity above the safety penalty for Greedy and
   below it for Cautious. The exact goal pair depends on the scenario's
   substrate — typically `AcquireCommodity(SelfConsume)` (Greedy choice) vs
   `StayPut` / a self-care goal / `Flee` from hostile presence (Cautious
   choice). Pin the exact goal pair at implementation time once
   S167COGARCBEH-001's scenario is authored.
5. AI regression layer: intended verification layer is golden E2E coverage
   (full action registries required, not local needs-only harness) because
   the divergence depends on the full ranking pipeline reading from the
   resolved profile components. The harness must execute through
   candidate generation → ranking → search → selection → trace emission for
   both agents in the same tick.
6. Ordering layer: the compared branches diverge at **ranking** (motive
   score driven by profile-weight delta), not at candidate emission
   (both agents emit the same candidate set since beliefs are identical),
   not at suppression (no archetype-based suppression exists). State this
   explicitly in the golden's preamble per precision rules.
12. Scenario isolation: the scenario must exclude lawful competing
    affordances that would tip the decision independent of archetype —
    e.g., differential hunger, perception of the hostile presence
    differing between agents, route knowledge differing, owned-inventory
    differing. All competing inputs must be symmetric across the two
    agents. S167COGARCBEH-001's scenario already authors this symmetry;
    the golden asserts no asymmetric inputs slipped through (sub-assertion
    (d)).

## Architecture Check

1. **Test-side profile-delta attribution over trace-surface extension** —
   FND-31's "authored causal reason" is satisfied by the golden computing
   the resolved profile values for both agents (via the
   `World`-residing per-agent profile components S152 ships) and asserting
   the documented archetype-driven field delta is the decisive factor.
   Extending the decision-trace to carry profile-field names would couple
   trace structure to archetype identity and complicate future
   runtime-adaptation work under FND-22A. The test-side computation reads
   the same authoritative state the ranking pipeline reads, so the
   attribution is equally trustworthy.
2. **Counterfactual symmetry as architectural backstop** — sub-assertion
   (e) (swap archetypes between agents and assert divergence reverses) is
   a metamorphic test in FND-31 vocabulary. It excludes scenario rails,
   agent-specific exception logic, asymmetric scenario seeding, and
   per-agent template wiring errors. Without it, a passing golden could
   silently mean "agent A always picks goal X" rather than "Greedy picks
   goal X."
3. **Same golden, two simulation runs** — the counterfactual replay loads
   the same scenario file but with the two agents' archetype fields
   swapped. Implementation: either load the scenario twice with a
   per-load override mechanism, or author two scenarios (forward +
   swapped). Prefer per-load override if the scenario loader supports it
   (verify at implementation time); otherwise author the swapped scenario
   alongside the forward one. The seed must be identical across both
   runs.

## Verification Layers

1. Decision divergence (sub-assertion (a)) -> decision trace
   (`AgentDecisionTrace.outcome.selected_opportunity` or equivalent
   anchor on `SelectedPlanTrace` — pin exact field at implementation
   time).
2. Motive-source contribution divergence (sub-assertion (b)) -> decision
   trace (`RankedGoalSummary.motive_source_contributions` per agent for
   the divergent goal).
3. Profile-delta attribution (sub-assertion (c)) -> authoritative world
   state (test reads `world.get_component_portfolio_weights(agent_id)` —
   or equivalent S152-surfaced accessor; verify exact accessor name at
   implementation time — for both agents and asserts the documented
   archetype-driven field delta is decisive).
4. Knowledge legality (sub-assertion (d)) -> event-log delta (assert zero
   perception events between spawn tick and divergence tick) and
   authoritative belief-store comparison (both agents' belief stores are
   byte-identical at the divergence tick).
5. Counterfactual symmetry (sub-assertion (e)) -> decision trace from the
   swapped-archetype replay (the divergent goal for the swapped Greedy
   agent matches the original Greedy agent's divergent goal).
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
  per-agent archetype override for the swapped replay).
- Runs the simulation through tick `T_divergence` (the first tick at
  which the goal selection differs; pin during implementation by
  initial observation of the scenario's behavior).
- Captures the per-agent `AgentDecisionTrace` and resolved profile
  components.
- Returns a `DivergenceObservation` struct with the two agents' selected
  goals, their `motive_source_contributions` for those goals, and their
  resolved `portfolio_weights` fields.

**Test 1: forward divergence**
- Asserts (a): `greedy.selected_goal != cautious.selected_goal` (or
  matching `GoalKind` with materially different ranked order — pin at
  implementation time once the scenario's tension architecture is
  authored).
- Asserts (b): `greedy.motive_source_contributions` for the divergent
  goal contains the expected economic-motive contribution exceeding
  `cautious.motive_source_contributions` for the same motive source.
- Asserts (c): `greedy.portfolio_weights.economic_weight >
  cautious.portfolio_weights.economic_weight` and the magnitude of the
  delta is sufficient to plausibly tip the motive score. Provide a
  one-line comment in the test naming the documented archetype delta and
  citing
  `cognitive_archetypes_greedy_resolves_higher_economic_weight_than_cautious`
  as the source.
- Asserts (d): event log between spawn and divergence tick contains zero
  `Perception*` event payloads for either agent (use whichever payload
  family the perception system emits — verify exact tag names at
  implementation time); both agents' belief stores are byte-identical at
  the divergence tick (use `bincode` or canonical-hash comparison).

**Test 2: counterfactual symmetry**
- Loads the same scenario with the two agents' `archetype` fields
  swapped (Greedy becomes Cautious-named-agent, Cautious becomes
  Greedy-named-agent).
- Asserts (e): the swapped Greedy agent's selected goal matches the
  forward-run Greedy agent's selected goal; same for Cautious. The
  divergence reverses correspondingly.

**Replay determinism (f)** is exercised by the standard golden harness —
the harness re-runs each test with the same seed and asserts byte-identical
authoritative state. No additional assertion needed beyond using the
standard golden harness pattern.

### 2. Pin the divergence tick during implementation

The scenario's exact divergence tick `T_divergence` depends on
S167COGARCBEH-001's authored tick budget, hunger seeding, and travel
distance. Implementation steps:

1. Run the scenario locally and observe at which tick the two agents'
   selected goals first differ.
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

- Authoring the scenario file itself — owned by S167COGARCBEH-001.
- Coverage doc regeneration — subsumed in S167COGARCBEH-001.
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
   (`motive_source_contributions`, `selected_opportunity`,
   `SelectedPlanTrace`) without adding new fields.
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
