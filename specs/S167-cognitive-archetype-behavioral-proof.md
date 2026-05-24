# S167: Cognitive Archetype Behavioral Proof Lane

**Status**: DRAFT

## Problem Statement

Cognitive archetypes are implemented end-to-end: `worldwake-core` defines the
`CognitiveArchetype` enum (10 variants: `Cautious, Bold, Stubborn, Methodical,
Opportunistic, Sociable, Skeptical, Dutiful, Greedy, Fearful` at
`crates/worldwake-core/src/cognitive_archetype.rs:6-17`), `ArchetypeProfileTemplate`
(`cognitive_archetype.rs:35-55`), `ArchetypeAssignmentPolicy`
(`cognitive_archetype.rs:58-63`), `ArchetypeAssignmentSource`
(`cognitive_archetype.rs:66-69`), and `PersonalityAssignedPayload`
(`cognitive_archetype.rs:72-78`); `spawn_agent()` applies archetype deltas to
perception, cognitive, portfolio, schema-context, risk, epistemic, testimony, and
route-preference profiles and records the assignment payload
(`crates/worldwake-cli/src/scenario/mod.rs:990-1059`); and **S152** added spawn
contracts proven by `crates/worldwake-ai/tests/scenarios/cognitive_archetypes.rs`
(418 lines, 7 tests) — including tests that two same-role agents differing only by
archetype resolve **different profile values** (e.g.
`cautious.stale_belief_backoff_ticks > bold.stale_belief_backoff_ticks` at line 190,
`greedy` higher portfolio economic weight than `cautious` at line 255).

Two proof gaps remain — both flagged by the triage of
`reports/ai-architecture-improvements-second-iteration.md` (Proposal 4) and both
genuine under FND-31:

1. **No behavioral-divergence proof.** Existing tests prove the *resolved profile
   values* differ. None proves the difference propagates to a **divergent decision**:
   two agents with the same role and the same beliefs, differing only by archetype,
   choosing **different actions/plans** under identical local facts, with the decision
   trace explaining the divergence (motive source) and the test independently proving
   the responsible profile-field delta. FND-31 is explicit: "Structural activation is
   not causal proof." Resolved-profile-value divergence is structural activation;
   decision divergence with attribution is causal proof.
2. **No canonical scenario coverage.** No `scenarios/*.ron` activates archetypes
   (verified: zero `archetype` references under `scenarios/`), so the
   `FeatureId::CognitiveArchetypes` detector
   (`crates/worldwake-cli/src/bin/scenario_coverage.rs:414` for the `FeatureDef`
   metadata registration, `:865` for `cognitive_archetypes_status(def)`) reports the
   feature absent in **every** scenario row of `docs/generated/scenario-coverage.md`
   (line 83 in current generation). The detector and feature row already exist; what
   is missing is a scenario that exercises them.

This spec closes both without adding new mechanics or extending the decision-trace
surface. The trace-attribution clause is satisfied by test-side computation against
the existing S152 resolved-profile surface, not by a trace-field extension. It is
the cleanest post-consolidation AI proof gap (FND-22/22A diversity, FND-31
falsification).

**Evidence sources.** `reports/ai-architecture-improvements-second-iteration.md`
Proposal 4; `docs/generated/scenario-coverage.md`; verified against
`cognitive_archetype.rs`, `scenario/mod.rs:990-1059`, `cognitive_archetypes.rs`,
`scenario_coverage.rs:414/865`, and `decision_trace.rs` (where `SelectionTrace`,
`SelectedPlanTrace`, and selected-plan search provenance are the trace-side
anchors for the divergence assertion). **Key interview decision:** scope to the
missing behavioral-divergence golden + dedicated canonical RON scenario + formal
roadmap/CI lane; do not re-implement archetypes, do not
duplicate the existing profile-value-divergence tests, and do not extend the
decision-trace surface.

## Phase and Status

Adjunct Wave: AI Architecture Improvements — First Iteration. Draft, pending ticket
decomposition.

## Crates

- `worldwake-ai` (tests) — a new behavioral-divergence golden at
  `crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs` (sibling
  of the existing `cognitive_archetypes.rs`) that runs two same-role/same-belief
  agents differing only by archetype, asserts divergent selected plan paths,
  asserts the expected selected-plan search provenance in the trace, and independently
  asserts (via S152's resolved-profile surface) that the documented
  archetype-driven profile-field delta is the decisive factor.
- `worldwake-cli` (scenarios) — a new dedicated canonical scenario
  `scenarios/cognitive-archetypes-divergence.ron` activates archetypes via explicit
  per-agent `AgentDef.archetype` assignments (not `archetype_assignment_policy`,
  which is randomized and incompatible with paired-agent determinism).
- Generated docs — regenerate `docs/generated/scenario-coverage.md` so the
  `Cognitive archetypes` row is no longer absent across all scenarios.
- Roadmap docs — formalize the row in `docs/scenario-roadmap.md` per the
  per-feature catalog convention.
- CI — a new `.github/workflows/golden-cognitive-archetypes.yml` lane modeled on
  `.github/workflows/golden-drive-escalation.yml` runs the golden in isolation on
  push/PR.

No engine/crate-source change. No decision-trace surface change. This is a
proof + coverage + lane-formalization spec.

## Dependencies

- **S152** (Cognitive Archetype spawn contracts) — completed/archived at
  [`archive/specs/S152-cognitive-archetypes-seeded-diversity.md`](../archive/specs/S152-cognitive-archetypes-seeded-diversity.md)
  (completed 2026-05-20). Owns the archetype implementation, the
  `PersonalityAssignedPayload` event, and the seven profile-value-divergence tests
  this spec builds on. The resolved-profile read surface S152 ships (the post-spawn
  per-agent profile components) is the surface D1's test-side attribution reads.
- **S110** (Decision History Events) — completed/archived at
  [`archive/specs/S110-decision-history-events.md`](../archive/specs/S110-decision-history-events.md)
  (completed 2026-04-20). Owns the decision-event payload taxonomy and selected
  plan trace surfaces that the divergence golden asserts against.
- The decision-trace surface in `crates/worldwake-ai/src/decision_trace.rs`
  (`SelectionTrace`, `SelectedPlanTrace`, and selected-plan search provenance) — used
  as-is. **The spec deliberately does not extend the trace surface**;
  profile-field attribution is computed test-side from the resolved-profile
  components S152 already ships.
- The scenario-coverage generator
  (`crates/worldwake-cli/src/bin/scenario_coverage.rs`) and its
  `FeatureId::CognitiveArchetypes` detector — already present.

## Design Goals

1. **Causal proof, not structural.** The golden asserts that the *decision*
   diverges (different selected `GoalKind`/action or the same `GoalKey` with a
   different selected plan path under identical beliefs), that the trace exposes
   the selected-plan search provenance in the expected direction, and that the
   test-computed resolved-profile-field delta is the decisive factor — satisfying
   FND-31's "prove the authored causal reason" without extending the trace
   surface.
2. **Same role, same beliefs, different archetype.** The two agents must be
   identical except for archetype, so the divergence is unambiguously attributable
   to the archetype delta (FND-22).
3. **Lawful divergence only.** The divergence must arise from the archetype's
   concrete profile deltas flowing through ordinary ranking/search — no scenario
   rail, no archetype-specific exception logic (FND-20).
4. **Counterfactual symmetry.** The golden also replays the scenario with the two
   archetypes swapped between the two agents and asserts the divergence reverses
   correspondingly. This excludes accidentally-asymmetric scenario seeding or
   per-agent template wiring (FND-31 metamorphic check).
5. **Canonical coverage activation.** After this spec,
   `docs/generated/scenario-coverage.md` shows `CognitiveArchetypes` active in at
   least the dedicated scenario, and the regeneration is reproducible.
6. **Formalized lane.** The scenario lands as a roadmap row in
   `docs/scenario-roadmap.md` with the same entry contract as other landed rows,
   and runs in a dedicated CI workflow lane so future profile retunes that erase
   the divergence fail loudly in isolation rather than silently in a batched lane.
7. **No new mechanics.** Reuse the shipped archetype templates, assignment,
   resolved-profile read surface, and trace.

## Non-Goals

- **New archetype variants, templates, or profile fields.**
- **Decision-trace surface extension** — no new trace fields naming profile
  deltas. Profile-field attribution lives in the golden's test logic, computed
  from the resolved-profile components S152 already ships.
- **Learning/adaptation of archetypes at runtime.** Archetypes remain spawn-fixed
  in this spec. Future specs may add experience-driven shifts under FND-22A; this
  spec preserves that path by not coupling decision-trace fields to archetype
  identity.
- **Mapping every unmapped coverage field** (portfolio weights, intention
  disposition, expectation store, last-seen memory, social observations). Those
  are separate coverage rows; this spec targets the archetype row. (A follow-up
  may address the others; see Follow-ups.)
- **Duplicating the S152 profile-value-divergence tests.** This spec proves
  *decision* divergence, a distinct layer.
- **Adding `AgentDef.archetype` overrides to existing canonical scenarios such as
  `final-integration.ron`.** A dedicated scenario was chosen instead so existing
  scenarios are not destabilized and the archetype tension is not diluted by
  unrelated competing tensions.

## FOUNDATIONS Alignment

| Principle | How satisfied |
|-----------|---------------|
| FND-3 (Concrete state over abstract scores) | Archetype assignment, `PersonalityAssignedPayload`, and the resolved per-agent profile components are concrete authoritative state, not summary scores. The decisive profile-field delta is read from those components in the test, not from any abstract archetype score. |
| FND-20 (Reasoning over scripts) | Divergence flows through ordinary route-aware search from concrete profile deltas; no archetype rail. |
| FND-22 (Diversity through concrete variation) | Two same-role agents choose differently solely because of concrete archetype-driven parameters. |
| FND-22A (Learning/preference shifts are concrete state) | Out of scope for this spec — archetypes remain spawn-fixed. Listed as a deliberate Non-Goal so future runtime-adaptation work can land cleanly on top without retrofitting this spec's proof shape. |
| FND-29 (Debuggability) | The divergence golden asserts the trace explains the selected plan path via existing selected-plan search provenance; the responsible profile-field is independently asserted against the resolved-profile components. |
| FND-31 (Validation/falsification first-class) | Converts structural activation into causal proof; adds counterfactual symmetry as a metamorphic check; adds canonical coverage; adds a dedicated CI lane so the proof runs in isolation. |

## Deliverables

### D1. Behavioral-divergence golden

A golden at `crates/worldwake-ai/tests/scenarios/cognitive_archetypes_divergence.rs`
that loads `scenarios/cognitive-archetypes-divergence.ron` (D2), runs the simulation
to the divergence tick, and asserts:

(a) **Decision divergence** — the two agents either commit different
`GoalKind`/`GoalKey` on the divergence tick, or commit the same `GoalKey` with a
materially different selected plan path such that the next travel action differs.

(b) **Trace-side causal contribution divergence** — for the divergent branch, the
decision trace exposes the selected plan path and selected-plan search
provenance used to price travel. For `Greedy vs Cautious`, the Greedy replay's
lower dangerous-traversal penalty makes the mixed-history Risky Orchard route
cheaper, while the Cautious replay's higher penalty makes the equally distant
neutral Sheltered Cut route cheaper. The trace assertion uses the existing
decision-trace surface as-is; no new trace fields are introduced.

(c) **Test-side profile-delta attribution** — the test independently reads the
two agents' resolved profile components (via the S152 surface), identifies the
documented archetype-driven profile-field delta for the chosen pair
(`RoutePreferenceProfile.dangerous_traversal_penalty` for `Greedy vs Cautious`),
and asserts that delta is large enough to tip the perceived travel-cost comparison
in the observed direction. This is the FND-31 "authored causal reason" anchor.

(d) **Knowledge legality** — neither divergence depends on world truth the agent
could not lawfully know. The golden constructs identical decision-side
known-entity beliefs for both agents and identical test-side route-experience
memory before the divergence tick.

(e) **Counterfactual symmetry** — the golden also runs the scenario with the two
archetypes swapped between the two agents and asserts the divergence reverses
correspondingly (whichever agent was `Greedy` now exhibits the `Greedy` decision,
and likewise for `Cautious`). This metamorphic check excludes accidentally
agent-asymmetric scenario seeding or per-agent template wiring.

(f) **Replay/determinism** — same seed + scenario + test-side setup reproduces
the same divergence observation and post-divergence state hash.

**Committed archetype pair and tension class:** `Greedy vs Cautious` at a
route-choice tradeoff while pursuing the same local acquisition goal. Live
reassessment on 2026-05-24 disproved the original economic-vs-safety
selected-goal premise: the existing scenario selected the same goals for isolated
Greedy and Cautious replays, and selected summaries carried zero `Greed`
contribution. The retained FND-31 proof target is therefore same-goal,
different-plan-path behavior. The pair is chosen because spawn-time archetype
deltas resolve different `RoutePreferenceProfile.dangerous_traversal_penalty`
values. Given identical concrete route-experience memory, Greedy should prefer a
mixed-history apple-source route while Cautious should prefer the equally distant
neutral apple-source route. The scenario authors that route-choice substrate via
ordinary topology and resource sources, not via a scenario rail or
archetype-specific exception.

### D2. Dedicated canonical scenario

A new `scenarios/cognitive-archetypes-divergence.ron` authored with:

- Two agents of the **same role** at the **same place**, with **identical**
  metabolism, perception, cognitive, schema, epistemic, testimony,
  route-preference, and risk profile fields **except** for the archetype-driven
  deltas applied at spawn.
- Explicit per-agent `AgentDef.archetype` assignment (`Greedy` on one,
  `Cautious` on the other). The scenario does **not** use
  `archetype_assignment_policy` (which is randomized via `DefaultUniformFive` /
  `Uniform` / `Weighted` per `cognitive_archetype.rs:58-63` and incompatible with
  a paired-agent deterministic divergence assertion).
- A local route-choice tension that makes the documented
  `RoutePreferenceProfile.dangerous_traversal_penalty` delta decisive while both
  agents pursue the same `AcquireCommodity(Apple, SelfConsume)` goal. The
  scenario provides two equally distant one-hop apple sources; the behavioral
  golden owns seeding identical mixed route-experience memory on the Risky
  Orchard route and proving the archetype-resolved profile delta changes the
  selected first travel step.
- Identical authored inputs between the two agents. The live `AgentDef` schema
  has no generic resource-source belief injection field, so the downstream
  behavioral golden owns any additional test-side belief setup or assertion
  needed to prove identical decision-side known-entity beliefs and identical
  route-experience memory before the divergence tick.
- A short tick budget sufficient to reach the divergence tick and assert it
  (this scenario is a focused proof, not a 1440-tick survival-coexistence
  scenario; survival-health contract is out of scope for this row — see Roadmap
  Status below).
- All scenario primitives (commodity names, place tags, workstation tags, recipe
  names if any) must be existing variants. The ticket implementer is responsible
  for cross-referencing each value against the actual enum definitions and
  existing scenarios (`scenarios/*.ron`) at authoring time, per the project's
  scenario-design convention.

### D3. Coverage regeneration

Regenerate `docs/generated/scenario-coverage.md` so the `Cognitive archetypes`
row is no longer absent across all scenarios. Regeneration command, recorded in
the ticket:

```bash
cargo run -p worldwake-cli --bin scenario-coverage -- --write
```

This is the only generated doc affected; no separate golden-inventory file
updates from this run.

### D4. Scenario roadmap formalization

Add the new scenario to `docs/scenario-roadmap.md` per the existing entry
contract template (`docs/scenario-roadmap.md` §4.1):

- **Catalog row** in §2 Gameplay Feature Catalog: a new `Cognitive archetypes`
  row with activation signal `Per-agent AgentDef.archetype set OR
  archetype_assignment_policy authored`, backing system
  `crates/worldwake-core/src/cognitive_archetype.rs` plus the spawn-time
  delta-application path in `crates/worldwake-cli/src/scenario/mod.rs:990-1059`,
  and current roadmap status pointing at the new landed row.
- **Ordering table row** in §4.2 Ordered Roadmap: a new row at the end of the
  table (after row 17 `final-integration`) marked `Landed` when D1 + D2 + D3
  land. The "Why it sits here" column notes that this is an auxiliary
  behavior-proof row rather than a survival-coexistence row, so it is not
  subject to the 1440-tick survival-health contract.
- **Landed entry** in §5: a new sub-section (next available number) using the
  template from §4.1 with Status `Landed`, Source scenario
  `scenarios/cognitive-archetypes-divergence.ron`, Backing goldens
  `cognitive_archetypes_divergence.rs`, Depends on the S152 archetype
  implementation. The entry explicitly classifies this row as **auxiliary
  behavior coverage** (analogous to the §5.17 auxiliary-and-non-roadmap entries
  for `simulation_gaps.rs`) — it owns the FND-31 archetype-decision causal
  proof, not a survival-coexistence landing.
- **Status Summary update** in §3.

The roadmap row formalizes the proof contract so future archetype work (e.g.,
runtime adaptation under FND-22A) inherits the same landing rules.

### D5. Dedicated CI workflow lane

Add `.github/workflows/golden-cognitive-archetypes.yml` modeled on
`.github/workflows/golden-drive-escalation.yml`:

- Triggers: `push` to `main`/`master`, `pull_request`.
- Concurrency group: `golden-cognitive-archetypes-${{ github.ref }}`.
- Single matrix entry: `scenario: cognitive_archetypes_divergence`, `filter:
  "scenarios::cognitive_archetypes_divergence::"`.
- Test command:
  `cargo test --release -p worldwake-ai --test golden_ai -- --ignored
  --test-threads=1 ${{ matrix.filter }}`.
- Toolchain pinned to the same version as sibling golden workflows
  (`1.93.0` at time of writing — match the value used in
  `golden-drive-escalation.yml` at the time the ticket lands, do not hardcode in
  the spec).

The dedicated lane keeps the archetype proof visible in CI on every PR, so a
profile retune that erases the divergence fails loudly in isolation rather than
silently in a batched lane.

## FND-01 Section H

1. **Information-path analysis.** No new information path. Both agents read only
   their own test-asserted decision-side beliefs and same-tick local observation
   (FND-14A); the divergence is internal to ranking/search over lawful inputs.
   The golden seeds identical known-entity beliefs and route-experience memory
   before the divergence tick, so no known decision-side input differs between
   agents except the spawn-time archetype delta and its resolved profiles.
2. **Positive-feedback analysis.** None introduced (proof + scenario + roadmap +
   CI only).
3. **Concrete dampeners.** Not applicable — no positive-feedback loops.
4. **Stored state vs. derived read-model.** No new state.
   `CognitiveArchetypeComponent` and the resolved per-agent profile components
   are pre-existing authoritative spawn state. The selected-plan trace and
   selected-plan search provenance are existing derived surfaces; D1's profile-delta
   attribution is a derived read in test logic (computed from the
   resolved-profile components) and is not stored.
5. **Planner-formalism analysis.** Plain route-aware GOAP/search; the golden proves
   divergence through the existing formalism with no method, rail, or
   archetype-specific operator. No HTN method registered.
6. **Causal-equivalence contract.** Not applicable — no compression / offscreen /
   save-load surface introduced. The activated scenario must still pass existing
   save/load determinism if the harness runs it.
7. **Systemic-validation analysis.** This spec *is* primarily systemic
   validation. Negative illegal paths the golden must exclude, with the concrete
   exclusion mechanism for each:
   - **(a) Divergence caused by an agent reading world truth it cannot lawfully
     know.** Excluded by D1(d): seeded identical known-entity beliefs and
     identical test-side route memory before the divergence tick.
   - **(b) Divergence caused by a scenario rail or archetype-specific exception
     logic rather than profile deltas through ranking.** Excluded by D1(e)
     counterfactual symmetry: swapping the archetypes between the two agents
     reverses the divergence. A scenario rail or archetype-specific exception
     would not produce symmetric reversal.
   - **(c) A "passing" golden that only proves resolved-profile-value difference
     (structural) rather than decision difference (causal).** Excluded by D1(a):
     the golden asserts divergent `GoalKind`/`GoalKey` or divergent selected
     plan path leading to a different next travel action, not just divergent
     profile hashes. Pure profile-hash divergence is already proven by S152's
     tests and would not satisfy this golden's assertions.

   Additional systemic checks: D1(f) replay/determinism check; D3 regenerated
   coverage as a structural-activation record complementing the causal golden;
   D5 dedicated CI lane keeps the proof visible on every PR.

## SystemFn Integration

No new `SystemFn`. No `SystemFn` modifications.

## Component Registration

No new components. `CognitiveArchetypeComponent` is already registered in
`crates/worldwake-core/src/component_schema.rs:1108-1131` (S152).

## Cross-System Interactions (FND-26)

No new cross-system call. The golden observes the existing chain:

ScenarioDef → archetype assignment in `spawn_agent()`
(`crates/worldwake-cli/src/scenario/mod.rs:990-1059`) → resolved profile
components → ranking/decision in `worldwake-ai` → decision-trace emission via
existing surfaces.

## Profile-Driven Parameters

No new parameters. The proof exercises existing archetype-driven profile deltas
shipped by S152. The chosen archetype pair (`Greedy vs Cautious`) and the
contrived local tension (same-goal route choice under identical route-experience
memory) must make a documented delta the decisive factor; the relevant delta is
the `RoutePreferenceProfile.dangerous_traversal_penalty` difference resolved by
the same archetype spawn path S152 already verifies.

## Authoritative-to-AI Impact Analysis

Not an authoritative-validation change. The golden exercises the full decision
cycle (candidate generation → ranking → search → selection → trace) and asserts
divergence, trace contents, and counterfactual symmetry, which itself satisfies
checklist item 7. Items 1–6 are unaffected (no validation/affordance/payload
change).

## Validation and Falsification (FND-31)

- **Golden**: D1 behavioral-divergence golden with (a) decision divergence
  assertion, (b) trace-side selected-plan and route-preference-context
  assertion, (c) test-side profile-delta attribution against the S152
  resolved-profile surface, (d)
  knowledge-legality assertions, (e) counterfactual symmetry replay, and (f)
  replay/determinism check.
- **Coverage**: D2 dedicated scenario authors per-agent `AgentDef.archetype`;
  D3 regenerates the coverage doc so `CognitiveArchetypes` is no longer absent.
- **Negative cases**: enumerated in Section H §7 with concrete exclusion
  mechanism per case.
- **No-regression**: existing `cognitive_archetypes.rs` profile-value tests and
  the survival/integration goldens unaffected.
- **CI lane**: D5 dedicated workflow ensures the proof runs in isolation on
  every PR.

## Risks

- **Contrived tension feels authored.** The local tension that makes the
  archetype delta decisive must be a lawful world condition (a route-choice
  tradeoff built from existing topology, two concrete apple sources, and
  concrete route-experience memory),
  not a scenario rail. Document the causal reason in the golden's preamble per
  FND-31. Counterfactual symmetry (D1(e)) is the architectural backstop.
- **Flaky divergence under seed changes.** Pin the seed and assert the specific
  `RoutePreferenceProfile.dangerous_traversal_penalty` delta as the cause (via
  D1(c) test-side attribution), so a future profile retune that erases the
  divergence fails loudly in D1(a) and is explained by D1(c) rather than silently
  passing.
- **Tension architecture brittleness.** The route-choice primitives (alternate
  topology + route-preference memory + travel search) must remain stable in the
  codebase. If a future spec restructures those primitives, the scenario may
  need a parallel revision. Mitigated by the dedicated CI lane: the failure is
  visible in isolation.

## Follow-ups (not actioned by this spec)

- The other coverage rows the report flags as unmapped (portfolio weights,
  intention disposition, expectation store, last-seen memory, social
  observations) are separate proof gaps. If they prove to share a single
  coverage-detector pattern, a sibling spec may address them together; this spec
  scopes only the archetype row.
- Runtime archetype adaptation under FND-22A (experience-driven shifts) is
  reserved for a future spec; this spec's proof shape is intentionally
  spawn-fixed and does not couple any new trace fields to archetype identity, so
  a future runtime-adaptation spec can land on top without retrofit.
- If future archetype pairs prove decision-decisive (e.g., `Bold vs Methodical`
  at a multi-stage planning tradeoff, `Sociable vs Skeptical` at an
  ask-vs-act tradeoff), each can land as a sibling matrix entry in the
  `golden-cognitive-archetypes` lane without an additional CI workflow file.
