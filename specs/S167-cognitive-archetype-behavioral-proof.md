# S167: Cognitive Archetype Behavioral Proof Lane

**Status**: DRAFT

## Problem Statement

Cognitive archetypes are implemented end-to-end: `worldwake-core` defines the
`CognitiveArchetype` enum (10 variants), `ArchetypeProfileTemplate`, assignment
policy/source, and `PersonalityAssignedPayload`
(`crates/worldwake-core/src/cognitive_archetype.rs`); `spawn_agent()` applies archetype
deltas to perception, cognitive, portfolio, schema-context, risk, epistemic, testimony,
and route-preference profiles and records the assignment payload
(`crates/worldwake-cli/src/scenario/mod.rs:990-1059`); and **S152** added spawn
contracts proven by `crates/worldwake-ai/tests/scenarios/cognitive_archetypes.rs` (418
lines) — including tests that two same-role agents differing only by archetype resolve
**different profile values** (e.g. `cautious.stale_belief_backoff_ticks >
bold.stale_belief_backoff_ticks`, `greedy` higher economic weight than `cautious`).

Two proof gaps remain — both flagged by the triage of
`reports/ai-architecture-improvements-second-iteration.md` (Proposal 4) and both
genuine under FND-31:

1. **No behavioral-divergence proof.** Existing tests prove the *resolved profile
   values* differ. None proves the difference propagates to a **divergent decision**:
   two agents with the same role and the same beliefs, differing only by archetype,
   choosing **different actions/plans** under identical local facts, with the decision
   trace explaining the divergence (motive source, profile delta, selected plan,
   rejected alternative). FND-31 is explicit: "Structural activation is not causal
   proof." Resolved-profile-value divergence is structural activation; decision
   divergence with a trace is causal proof.
2. **No canonical scenario coverage.** No `scenarios/*.ron` activates archetypes
   (verified: zero `archetype` references under `scenarios/`), so the
   `FeatureId::CognitiveArchetypes` detector
   (`crates/worldwake-cli/src/bin/scenario_coverage.rs:414,861`) reports the feature
   absent in **every** scenario row of `docs/generated/scenario-coverage.md:83`. The
   detector and feature row already exist; what is missing is a scenario that exercises
   them.

This spec closes both without adding new mechanics. It is the cleanest
post-consolidation AI proof gap (FND-22/22A diversity, FND-31 falsification).

**Evidence sources.** `reports/ai-architecture-improvements-second-iteration.md`
Proposal 4; `docs/generated/scenario-coverage.md`; verified against
`cognitive_archetype.rs`, `scenario/mod.rs`, `cognitive_archetypes.rs`, and
`scenario_coverage.rs`. **Key interview decision:** scope to the missing
behavioral-divergence golden + canonical RON coverage activation; do not re-implement
archetypes or duplicate the existing profile-value-divergence tests.

## Phase and Status

Adjunct Wave: AI Architecture Improvements — First Iteration. Draft, pending ticket
decomposition.

## Crates

- `worldwake-ai` (tests) — a new behavioral-divergence golden under
  `crates/worldwake-ai/tests/scenarios/` (or an extension of
  `cognitive_archetypes.rs`) that runs two same-role/same-belief agents differing only
  by archetype and asserts divergent decisions plus trace explanation.
- `worldwake-cli` — a canonical `scenarios/*.ron` gains archetype activation (a new
  small archetype scenario, or archetype overrides added to an existing canonical
  scenario such as `final-integration.ron`), so `scenario_coverage` reports
  `CognitiveArchetypes` active.
- Generated docs — regenerate `docs/generated/scenario-coverage.md` (and any
  golden-inventory docs) after the scenario lands.

No engine/crate-source change is expected; this is a proof + coverage spec.

## Dependencies

- **S152** (Cognitive Archetype spawn contracts) — completed/archived. Owns the
  archetype implementation and the profile-value-divergence tests this spec builds on.
- **S110** (Decision History Events) / decision-trace surface — provides the trace
  the divergence golden asserts against.
- The scenario-coverage generator (`scenario_coverage.rs`) and its
  `FeatureId::CognitiveArchetypes` detector — already present.

## Design Goals

1. **Causal proof, not structural.** The golden must assert that the *decision*
   diverges (different selected `GoalKind`/action or different ranked order under
   identical beliefs), and that the trace names the cause (profile delta + motive
   source), satisfying FND-31's "prove the authored causal reason."
2. **Same role, same beliefs, different archetype.** The two agents must be identical
   except for archetype, so the divergence is unambiguously attributable to the
   archetype delta (FND-22).
3. **Lawful divergence only.** The divergence must arise from the archetype's concrete
   profile deltas flowing through ordinary ranking/search — no scenario rail, no
   archetype-specific exception logic (FND-20).
4. **Canonical coverage activation.** After this spec,
   `docs/generated/scenario-coverage.md` shows `CognitiveArchetypes` active in at least
   one canonical scenario, and the regeneration is reproducible.
5. **No new mechanics.** Reuse the shipped archetype templates, assignment, and trace.

## Non-Goals

- **New archetype variants, templates, or profile fields.**
- **Learning/adaptation of archetypes at runtime** (archetypes are spawn-fixed; future
  specs may add experience-driven shifts under FND-22A).
- **Mapping every unmapped coverage field** (portfolio weights, intention disposition,
  expectation store, last-seen memory, social observations). Those are separate
  coverage rows; this spec targets the archetype row. (A follow-up may address the
  others; see Follow-ups.)
- **Duplicating the S152 profile-value-divergence tests.** This spec proves *decision*
  divergence, a distinct layer.

## FOUNDATIONS Alignment

| Principle | How satisfied |
|-----------|---------------|
| FND-20 (Reasoning over scripts) | Divergence flows through ordinary ranking/search from concrete profile deltas; no archetype rail. |
| FND-22 (Diversity through concrete variation) | Two same-role agents choose differently solely because of concrete archetype-driven parameters. |
| FND-22A (Learning/preference shifts are concrete state) | The archetype assignment is concrete, traceable state (`PersonalityAssignedPayload`, resolved profile hash). |
| FND-29 (Debuggability) | The divergence golden asserts the trace explains the chosen vs rejected plan and the profile delta. |
| FND-31 (Validation/falsification first-class) | Converts structural activation into causal proof; adds canonical coverage. |

## Deliverables

### D1. Behavioral-divergence golden

A golden that spawns two agents with identical role, place, inventory, and seeded
beliefs, differing only by `CognitiveArchetype` (a pair chosen so a documented profile
delta drives a decision split — e.g. a risk/economic-weight or backoff/patience delta
that flips goal ranking or action selection under a contrived-but-lawful local
tension). It asserts: (a) the two agents commit different goals/actions (or
materially different ranked order) on the divergence tick; (b) the decision trace for
each names the motive source and the archetype-driven profile value responsible; (c)
neither divergence depends on world truth the agent could not lawfully know. Include a
replay/determinism check.

### D2. Canonical scenario archetype activation

Add archetype activation to a canonical `scenarios/*.ron` — either a small dedicated
archetype scenario or `archetype`/`archetype_assignment_policy` overrides on an
existing canonical scenario — such that `scenario_coverage`'s
`cognitive_archetypes_status(def)` reports active. Choose the scenario so activation is
behaviorally meaningful, not a cosmetic field set.

### D3. Coverage regeneration

Regenerate `docs/generated/scenario-coverage.md` (and golden-inventory docs if the
golden adds a scenario row) so the `Cognitive archetypes` row is no longer absent
across all scenarios. Record the regeneration command in the ticket.

## FND-01 Section H

1. **Information-path analysis.** No new information path. Both agents read only their
   own seeded beliefs and same-tick local observation; the divergence is internal to
   ranking/search over lawful inputs.
2. **Positive-feedback analysis.** None introduced (proof + scenario only).
3. **Concrete dampeners.** Not applicable.
4. **Stored state vs. derived read-model.** No new state. `CognitiveArchetypeComponent`
   and the resolved profiles are pre-existing authoritative spawn state; the decision
   trace is the existing derived surface.
5. **Planner-formalism analysis.** Plain GOAP/ranking; the golden proves divergence
   through the existing formalism with no method or rail.
6. **Causal-equivalence contract.** Not applicable — no compression/offscreen/save-load
   surface introduced. (The activated scenario must still pass existing save/load
   determinism if the harness runs it.)
7. **Systemic-validation analysis.** This spec *is* primarily systemic validation.
   Negative illegal paths the golden must exclude: (a) divergence caused by an agent
   reading world truth it cannot lawfully know; (b) divergence caused by a scenario
   rail or archetype-specific exception rather than profile deltas through ranking; (c)
   a "passing" golden that only proves resolved-profile-value difference (structural)
   rather than decision difference (causal). Checks: the divergence golden (D1) with
   trace assertions, a replay-equivalence check, and the regenerated coverage as a
   structural-activation record complementing the causal golden.

## SystemFn Integration

No new `SystemFn`.

## Component Registration

No new components. `CognitiveArchetypeComponent` is already registered (S152).

## Cross-System Interactions (FND-26)

No new cross-system call. The golden observes the existing
ScenarioDef → archetype assignment → resolved profiles → ranking/decision-trace chain.

## Profile-Driven Parameters

No new parameters. The proof exercises existing archetype-driven profile deltas; the
chosen archetype pair and the contrived local tension must make a documented delta the
decisive factor.

## Authoritative-to-AI Impact Analysis

Not an authoritative-validation change. The golden exercises the full decision cycle
(candidate generation → ranking → search → selection → trace) and asserts divergence
and trace contents, which itself satisfies checklist item 7. Items 1–6 are unaffected
(no validation/affordance/payload change).

## Validation and Falsification (FND-31)

- **Golden**: D1 behavioral-divergence golden with trace assertions + replay check.
- **Coverage**: D2/D3 — `CognitiveArchetypes` active in a canonical scenario,
  reproducibly regenerated.
- **Negative cases**: no illegal-knowledge-driven divergence; no rail-driven
  divergence; structural-only proof rejected.
- **No-regression**: existing `cognitive_archetypes.rs` profile-value tests and the
  survival/integration goldens unaffected.

## Risks

- **Contrived tension feels authored.** The local tension that makes an archetype
  delta decisive must be a lawful world condition (e.g. a marginal economic-vs-safety
  tradeoff), not a scenario rail. Document the causal reason in the golden per FND-31.
- **Flaky divergence under seed changes.** Pin the seed and assert the specific
  profile delta as the cause, so a future profile retune that erases the divergence
  fails loudly rather than silently passing.

## Follow-ups (not actioned by this spec)

- The other coverage rows the report flags as unmapped (portfolio weights, intention
  disposition, expectation store, last-seen memory, social observations) are separate
  proof gaps. If they prove to share a single coverage-detector pattern, a sibling spec
  may address them together; this spec scopes only the archetype row.
