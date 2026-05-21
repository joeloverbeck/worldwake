# S161 — FOUNDATIONS Constitutional Hardening (Gap Audit 2026-05)

**Status:** COMPLETED
**Type:** Constitutional amendment (`docs/FOUNDATIONS.md` edits + downstream-doc
anchoring; no new simulation state, components, actions, systems, or feedback
loops)
**Priority:** Medium-high. FND-14B is the keystone (it anchors the source-class
doctrine S158 already shipped). Independent of S159/S160; sequence whenever.
**Foundations:** FND-12, FND-13, FND-14, FND-14A, FND-20, FND-27, FND-29A, FND-31
**Source:** `reports/foundations-gap-audit.md` (ChatGPT-Pro). Every load-bearing
claim re-verified against the codebase on 2026-05-21 before acceptance; the most
important correction (the report's headline leak risk is already closed by S158)
is recorded under Evidence below.

## Problem Statement

### Motivation

We are consolidating the AI architecture against `docs/FOUNDATIONS.md` before
upgrading it. The question that prompted the audit was: *are the foundations
complete?* The verdict is **mostly correct with four targeted strengthenings plus
new canonical scenarios** — no rewrite, no reorganization, no new architecture
doctrine.

The constitution is philosophically right but **underspecified at four pressure
points** that near-term architecture is now actively touching:

1. **FND-12** says approximation must "remain equivalent to the explicit model"
   but never defines what *equivalent* requires. That is too weak for the
   compression, boundary, prehistory, and save/load surfaces we will build.
2. **FND-14/14A** correctly separate belief from truth, but the **planner-facing
   source rule is implied, not constitutional.** S158 just shipped that exact rule
   into `docs/planner-contracts.md` and `docs/spec-drafting-rules.md` — yet those
   downstream docs cite FOUNDATIONS as authority for a principle FOUNDATIONS does
   not name. This spec adds the missing anchor (FND-14B).
3. **FND-20** forbids scripts in spirit but does not explicitly bind HTN method
   selection/decomposition/rejection/fallback to the belief-backed source rule.
4. **FND-31** is correct but weaker than the *active* golden-testing doctrine
   (`docs/golden-e2e-testing.md`, `docs/scenario-roadmap.md`) it should match.

### Evidence (verified against code/docs on 2026-05-21)

- **FND-14B is anchoring, not leak-fixing.** The report frames remote-truth leaks
  through seller listings, contention, production, and physical reads as an open
  risk. **S158 (completed, archived 2026-05-21) already closed all of them.**
  `per_agent_belief_view.rs` hard-gates `has_sale_listing`/`seller_for_sale_lot`/
  `listed_sale_lots_at` on co-location (test
  `remote_listed_sale_lot_does_not_read_live_sale_listing`); `has_production_job`,
  `carry_capacity`, `load_of_entity`, and the contention accessors return belief or
  unknown for remote entities, never world truth. `PlanningSnapshot` carries an
  `AdmissionSource` enum (`SelfAuthoritative`, `LocalSameTickPhysical`,
  `GroundedEvidence`, `BeliefLastSeen`, `PossessionContainmentFrontier`,
  `PublicTopology`); `PlanningState`'s `EconomicBeliefView` delegates to the gated
  snapshot with no second authoritative fallback. The source-class rule is written
  into `docs/planner-contracts.md` §2 (lines 98–132) and
  `docs/spec-drafting-rules.md` (Belief-View Accessor Source-Class Rule). FND-14B's
  value is therefore **constitutional anchoring + regression-proofing future
  planner surfaces**, not closing a present hole.
- **FND-31 alignment is real.** `docs/golden-e2e-testing.md` already states a
  scenario golden is valid only when "the scenario passed for the authored causal
  reason, not merely by an accidental or rival lawful branch," and names the exact
  1440-tick failure taxonomy (branch never activated / unrelated fallback / one bug
  masks another / end-state-only assertions). `docs/scenario-roadmap.md` already
  encodes the three-part landing contract (structural activation, authored
  behavior, authored causal reason) and "structural activation and behavioral proof
  are different layers." Strengthening FND-31 absorbs this into the constitution.
  **Caveat:** metamorphic/property-based testing and seed/sensitivity sweeps are
  *not* currently practiced — the FND-31 replacement names them as feature-scoped
  aspiration ("appropriate to the feature… depends on blast radius"), not a blanket
  mandate.
- **HTN surfaces are real.** `MethodSchema` (goal kind, preconditions, subgoals,
  explanation template, motive bias, budget hint), the selector's
  selected+rejected methods with `failed_precondition`, and
  `StrategicFallbackReason::{NoViableMethod, MethodProducedNoStages}` all exist and
  are traceable via `AgentDecisionTrace`. Method preconditions read
  `RuntimeBeliefView`, not authoritative world state. The now-archived
  `archive/specs/S160-htn-authority-honesty.md` hardened HTN honesty, so the
  FND-20 guard reinforces that landed work.
- **Coverage asymmetry.** `docs/generated/golden-coverage-matrix.md`: FND-12 = 6
  scenarios, FND-13 = 2, vs FND-14 = 50, FND-20 = 24, FND-31 = 10. Compression and
  boundary processes are genuinely under-stressed — but **no offscreen sim,
  boundary compression, sleeping-entity, or prehistory system exists yet.** The
  FND-12 strengthening and scenarios K/L are therefore **forward-looking**: they set
  the bar before those systems are built.
- **Scenario I is half-covered.** `golden_belief_wall_trap_remote_sale_listing_
  does_not_leak_live_truth` already proves the belief-view half; the HTN
  method-rejection-on-missing-`SellerKnown` half is novel. Scenario J (HTN
  rejection/fallback) is fully novel coverage.
- **Quote correction.** The report's FND-20 insertion anchor ("Turning such a goal
  into a method-required goal requires an explicit schema contract and tests showing
  that fallback would be semantically invalid") is **not** verbatim. The actual
  sentence at `FOUNDATIONS.md:228` is: "A method-required goal needs an explicit
  schema contract and tests proving that fallback would be semantically invalid."
  Deliverable 3 uses the real anchor.

### Key scoping decisions (brainstorm 2026-05-21)

- **All five proposals accepted; none rejected** — pushback is only on
  framing/urgency (FND-14B already implemented; FND-12 + K/L forward-looking) and
  aspiration-vs-practice (FND-31 metamorphic/property testing).
- **One consolidated spec** (the edits are one tightly-coupled audit).
- **Include constitutional text now, defer artifacts.** Write the full FND-12
  strengthening and scenarios K/L into FOUNDATIONS now (cheap; sets the bar before
  building), but **do not** create `docs/causal-equivalence-contracts.md` or any
  K/L goldens until offscreen/boundary/prehistory systems reach the roadmap. Matches
  the report's own deferral (its §7).
- **No FOUNDATIONS reorganization / renumbering** (would churn generated coverage,
  planner contracts, spec rules, and tests for no philosophical gain).

## Deliverables

The primary deliverable is editing `docs/FOUNDATIONS.md`. Replacement/insertion
text is given verbatim so implementation is mechanical. Anchors are exact as of
the 2026-05-21 file state.

### 1. Replace the body of FND-12 (`### 12. Performance May Compress Computation, Never Causality`)

Replace the **entire** current body — everything from L123 "Optimization is
allowed. Causal cheating is not." through the **Test** line at L131, including the
"The rule is simple: performance may change how the machine computes a result,
never what the world means." sentence (L129), which the replacement drops — with:

> Optimization is allowed. Cheating causality is not.
>
> Offscreen simulation, sleeping entities, batching, region summaries,
> population-level approximations, pre-simulation, cache warmups, save/load,
> replay, migration between simulation regimes, and map-boundary handling may
> change representation, batching, or scheduling only under an explicit
> causal-equivalence contract.
>
> A causal-equivalence contract must name:
> - the explicit higher-fidelity referent it approximates;
> - the causal variables, identities, quantities, obligations, beliefs, records,
>   source/sink paths, timing bounds, and failure modes that must be preserved;
> - the admitted error bounds or nondeterminism, if any;
> - the materialization/decompression boundary where an aggregate becomes concrete
>   entities, records, events, or resources;
> - the tests or audits that compare compressed and explicit behavior.
>
> A compressed representation must never produce a local state, belief, transfer,
> social fact, or record that no lawful explicit simulation could have produced.
> Aggregates and summaries may guide scheduling or serve as declared boundary
> artifacts, but they are not authoritative local truth until they are materialized
> through an explicit world process.
>
> Save/load and replay equivalence are part of the same rule: changing encoding,
> batching, or cached summaries must not change world meaning, causal provenance,
> knowledge provenance, or downstream lawful affordances.
>
> **Test**: An observer entering the situation must never find a state that cannot
> be explained by a legal sequence of world events, and an audit must be able to
> name the approximation contract that preserved that explanation.

### 2. Insert FND-14B after FND-14A (before FND-15)

Insert immediately after the FND-14A **Test** paragraph:

> ### 14B. Planner-Visible Inputs Must Be Belief-Backed or Lawful Boundary Artifacts
>
> Every planner-visible input — including goal emission, goal ranking, affordance
> enumeration, target enumeration, HTN method selection, method preconditions,
> tactical search, heuristic costs, duration estimates, revalidation, fallback, and
> decision traces — must be sourced from one of these surfaces:
> - the actor's self-authoritative state;
> - same-tick local physical observation permitted by Principle 14A;
> - the actor's belief, memory, testimony, record, known-plan, expectation, or
>   institutional-belief state, with provenance/freshness where the system tracks
>   it;
> - public structural substrate that is not character knowledge, such as action
>   definitions or declared topology, provided it does not reveal remote entity
>   state or social fact;
> - an explicit boundary artifact or boundary process declared under Principle 13.
>
> Remote entities, seller listings, stock, ownership, custody, rights, claims,
> offices, routes-as-known, threat estimates, public notices, rumors, institutional
> records, and social artifacts do not become planner inputs merely because they
> exist in authoritative world state. They enter planning only through a lawful
> source above. A planner cache, snapshot, read model, or heuristic must either
> preserve that source classification or be treated as illegal for agent
> decision-making.
>
> Debug views, omniscient test harnesses, generated coverage inventories, and
> authoritative commit checks may inspect world truth, but they must not feed agent
> planning except at an explicit world-process or dispatch-legality boundary.
>
> **Test**: Removing the actor's belief or local observation of a remote listing,
> owner, threat, office holder, or record must remove planner candidates that
> depend on it, even when the authoritative world truth remains unchanged.

A one-line implementation note may accompany the edit (not part of the
constitutional text): the source-class enforcement for the economic, production,
physical, and contention accessors already exists as of S158; FND-14B generalizes
the rule so future planner-visible inputs inherit it by default.

### 3. Insert the HTN anti-script guard into FND-20

**Depends on Deliverable 2:** the inserted text references "Principle 14B," which
Deliverable 2 creates. Apply Deliverable 2 before (or together with) this edit so
the cross-reference is never dangling.

Insert after the existing sentence (the real anchor — see Evidence quote
correction): "A method-required goal needs an explicit schema contract and tests
proving that fallback would be semantically invalid."

> HTN methods are not scripts. A method is lawful only when it is a reusable domain
> pursuit pattern that decomposes into ordinary lawful affordances or subgoals.
> Method selection, precondition evaluation, stage construction, rejection,
> fallback, and failure attribution must obey Principle 14B and must be traceable
> without referring to desired story beats, scenario rails, target-specific
> exceptions, or hidden success recipes.

### 4. Replace the body of FND-31 (`### 31. Validation and Falsification Are First-Class`)

Replace the current body (from "Interesting-looking output is not evidence…"
through the **Test** line) with:

> Interesting-looking output is not evidence by itself. Passing a local golden end
> state is not evidence by itself.
>
> Every subsystem and scenario class must declare:
> - multiple independent patterns it should generate;
> - the path from local traces to aggregate behavior;
> - the artifacts it must never produce;
> - parameters, seeds, timing, topology, population, and resource conditions that
>   should destabilize it;
> - traces that expose failure;
> - negative cases that prove forbidden causal or knowledge paths are absent.
>
> Validation must include both canonical scenario goldens and systemic checks
> appropriate to the feature: invariants, property tests, metamorphic tests,
> cross-scenario composition tests, seed sweeps, sensitivity sweeps, long-run soak
> tests, adversarial scenario sampling, replay/save-load equivalence checks, and
> causal trace audits. The required mix depends on the feature's blast radius, but a
> feature with cross-system consequences cannot be validated only by a single local
> outcome.
>
> A golden passes constitutionally only if it proves the authored causal reason, or
> explicitly accepts a named alternative lawful branch. Structural activation is not
> causal proof. "Survived," "looked plausible," or "ended in the expected state" is
> insufficient when the same result could be produced by an illegal planner read,
> hidden abstraction, omitted contention, stale cache, fallback path, or unrelated
> bug.
>
> The architecture must support inspection of both causal history and knowledge
> history, including evidence that illegal planner-visible inputs were not used.
>
> **Test**: "Looked plausible in one run" is not enough. The system must produce
> inspectable evidence that it behaves for the right reasons, fails in explainable
> ways, and does not pass for prohibited reasons.

The "property tests, metamorphic tests, … sensitivity sweeps" list is **feature-
scoped aspiration**, gated by the explicit "appropriate to the feature… depends on
the feature's blast radius" qualifier. This spec does **not** retroactively require
those test types for existing goldens.

### 5. Add canonical scenarios I–L after Scenario H (before `## VII. Final Rule of Thumb`)

> ### I. Planner Belief Barrier Around Remote Affordances
>
> A seller has grain listed in a remote town, and an agent has a hunger-driven
> reason to acquire grain. Until the agent has a lawful belief, record, testimony,
> public notice, or local observation of that listing, the planner must not emit or
> rank a seller-dependent candidate for that remote listing. After a delayed rumor,
> ledger entry, messenger report, or direct visit creates the belief, the same
> candidate may appear with knowledge provenance. Updating the authoritative
> listing alone must not change the agent's remote plan.
>
> ### J. HTN Method Rejection, Fallback, and Lawful Failure
>
> An agent pursues a goal whose reusable HTN method would be appropriate only if a
> belief-backed precondition holds. When the precondition is unknown, stale,
> contradicted, or false, the method is rejected with a traceable failed
> precondition. The planner may fall back to ordinary affordance search only when
> the goal schema permits it; otherwise it must fail or seek information. No method
> may force the desired story beat through a scenario-specific stage.
>
> ### K. Boundary-Compressed Shock Materializes Into Local Carriers
>
> A neighboring region experiences a compressed drought, raid, migration wave, trade
> disruption, or epidemic pressure. The pressure crosses the boundary only through
> declared boundary processes: delayed travelers, refugees, trade records, missing
> caravans, price/stock changes, rumors, illness carriers, or other materialized
> entities and records. Agents inside the active slice react to those local carriers
> and may hold stale or contradictory beliefs about the outside region. The
> compressed aggregate never becomes authoritative local truth without
> materialization.
>
> ### L. Long Prehistory With Inspectable Compacted Provenance
>
> The world is pre-simulated for months or years before the player arrives. Deaths,
> debts, offices, shortages, grudges, habits, records, relationships, and damaged
> places may be compacted, but the current state must remain explainable by lawful
> prior events. Agents may begin with stale or conflicting beliefs produced during
> that history. An audit can recover enough causal, knowledge, and source/sink
> provenance to explain why the playable state is as it is, even if not every
> low-level tick is retained.

### 6. Downstream doc anchoring (verified genuinely needed)

- **`docs/planner-contracts.md` §2** — add an explicit FND-14B reference where the
  "Planner-visible fields are source-scoped" rule is stated (it currently cites
  FND-14A only). State that the source-class rule *is* the application of FND-14B
  to belief-view accessors.
- **`docs/spec-drafting-rules.md`** — add two new checklist items, both verified
  absent today:
  - a **causal-equivalence contract** checklist item for any spec introducing
    offscreen sim, boundary compression, sleeping entities, region summaries,
    population approximations, prehistory, or new cache/save-load surfaces (cite
    revised FND-12; require the five named contract elements);
  - a **systemic-validation** checklist item for cross-system features (cite revised
    FND-31; require declaring negative illegal-path cases and naming which
    feature-scoped systemic checks apply).
- **`docs/golden-e2e-testing.md`** — terminology alignment with revised FND-31, and
  add an explicit **"illegal planner-input absence"** proof pattern (the
  `belief_wall_trap` negative-candidate assertions are the existing exemplar).

### 7. Deferred (do NOT implement in this spec)

- `docs/causal-equivalence-contracts.md` template — defer until the first
  offscreen/boundary/prehistory system is specced.
- Scenarios **K/L** goldens — defer with their backing systems.
- `docs/scenario-roadmap.md` rows for I–L — add as roadmap targets when their
  goldens are scheduled, not now.
- Scenario **J** golden (HTN rejection/fallback) and the **I** HTN-rejection
  variant (`SellerKnown` precondition absent → method rejected + traced fallback)
  are testable now but are **golden-coverage work, not constitutional work**; track
  via the scenario-roadmap / `/golden-gap-analysis` flow after the FOUNDATIONS edits
  land. They are noted here, not delivered here.

## FND-01 Section H Analysis

Constitutional documentation edit plus downstream-doc anchoring. Introduces no new
simulation state, components, actions, systems, or feedback loops. Per
`docs/spec-drafting-rules.md`, the simulation-system analyses are marked Not
applicable with reasons; the stored-state/derived list still applies.

- **Information-path analysis:** Not applicable. No new information reaches any
  agent; FND-14B *names* the existing lawful information paths (belief, same-tick
  observation, boundary artifact) rather than adding one.
- **Positive-feedback analysis:** Not applicable. No simulation loop introduced.
- **Concrete dampeners:** Not applicable.
- **Stored-state vs. derived read-model list:** **No new types of either kind.**
  This spec edits constitutional/doctrine documents and downstream prose only. It
  declares no authoritative state and promotes no derived value to truth; FND-12 and
  FND-27 language is *strengthened* to forbid exactly that promotion.
- **Planner-formalism analysis:** Unchanged. FND-14B and the FND-20 guard codify the
  *current* belief-bounded, formalism-flexible standard (GOAP default, HTN as
  reusable decomposition with legal flat fallback). No formalism is mandated or
  forbidden beyond what FND-20 already says.

### Proof surface (FND-31)

The constitutional edits are doc changes; their "proof" is downstream-doc
consistency and the absence of contradiction with shipped code:

- **Cross-reference check:** after the edits, `docs/planner-contracts.md` and
  `docs/spec-drafting-rules.md` reference FND-14B and FND-12 without dangling or
  contradictory citations; no generated-doc regeneration is required (no test
  metadata changes).
- **No-regression check:** the existing S158 belief-view goldens
  (`remote_listed_sale_lot_does_not_read_live_sale_listing`, the `belief_wall_trap`
  negative-candidate assertions) remain the live evidence that FND-14B's **Test** is
  already satisfied for the in-scope accessors; this spec must cite them as the
  current FND-14B proof surface rather than duplicate them.
- **Forward obligation:** future specs that add a planner-visible input or a
  compression/boundary surface must satisfy the new `spec-drafting-rules.md`
  checklist items. This is an obligation created by the edit, not a test landed by
  it.
- Scenarios **J** and the **I** HTN-rejection variant are the eventual behavioral
  proofs for scenario classes I/J; they are deferred to golden-coverage work
  (Deliverable 7).

## Outcome

Completed on 2026-05-21.

Implemented the S161 constitutional hardening as a documentation-only change:

- `archive/tickets/S161FNDHARD-001.md` updated `docs/FOUNDATIONS.md` with the
  revised FND-12 causal-equivalence contract, new FND-14B planner-visible-input
  source rule, FND-20 HTN anti-script guard, revised FND-31 validation doctrine,
  and canonical scenarios I-L.
- `archive/tickets/S161FNDHARD-002.md` anchored downstream docs by adding the
  FND-14B source-class reference to `docs/planner-contracts.md`, adding
  causal-equivalence and systemic-validation checklist items to
  `docs/spec-drafting-rules.md`, and adding the illegal planner-input absence proof
  pattern to `docs/golden-e2e-testing.md`.

Deviations from original plan: none. Deferred items remain deferred exactly as
specified: `docs/causal-equivalence-contracts.md`, scenarios K/L goldens,
`docs/scenario-roadmap.md` rows for I-L, scenario J golden coverage, and the
scenario I HTN-rejection variant.

Verification:

- S161FNDHARD-001: `grep -n "### 14B\\.\\|### I\\.\\|### J\\.\\|### K\\.\\|### L\\.\\|HTN methods are not scripts" docs/FOUNDATIONS.md` found the new FND-14B heading, HTN guard, and scenarios I-L; removed stale FND-12/FND-31 phrases returned zero matches; `cargo test --workspace` passed.
- S161FNDHARD-002: `grep -c "FND-14B" docs/planner-contracts.md` returned `1`; `grep -in "causal-equivalence\\|systemic-validation\\|illegal planner-input" docs/spec-drafting-rules.md docs/golden-e2e-testing.md` found the downstream anchors; `cargo test --workspace` passed.
