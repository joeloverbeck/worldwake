# S165: Epistemic Verification Repair

**Status**: DRAFT

## Problem Statement

`RepairKind::InsertVerification` is wired into the live repair attempt order
(`crates/worldwake-ai/src/plan_repair.rs:63-71`, position 3 of 5) but its handler
unconditionally returns `Err(RepairFailure::NoEpistemicSubstrate)`
(`plan_repair.rs:131`). It consults none of the `PlanRepairContext` fields. Two
focused tests (`insert_verification_returns_no_epistemic_substrate_without_s139`)
and one scenario test (`stale_belief_breach_attempts_insert_verification_without_s139`)
assert the always-fail behavior, with comments stating "the S139 epistemic-subgoal
substrate is intentionally absent" and "until ticket 007 wires replacement."

The substrate those comments waited on now exists. **S137**
(`archive/specs/S137-plan-causal-links-and-repair.md`) shipped the repair search,
`CausalLink` provenance, `PlanRepairContext`, and the revalidation-seam routing
(`S137PLACAULIN-007`). **S139** (`archive/specs/S139-epistemic-sensing-subgoals.md`)
shipped `GoalKind::AskWitness { witness, topic }` — a first-class lawful verification
goal with a satisfaction predicate over the belief envelope, candidate emission,
ranking, and payload override. Both are complete and archived. S137 explicitly
declared the bridge a *soft dependency in the inverse direction*: "`RepairKind::
InsertVerification` can splice AskWitness-class verification goals as repair steps …
without an epistemic substrate, `InsertVerification` returns `RepairFailure::
NoEpistemicSubstrate` and the search falls through to the next `RepairKind`."

**No spec or ticket ever owned that bridge.** S137PLACAULIN-007 wired
`RebindTarget`/`ReplaceProvider` candidates at the revalidation seam; the test
comment naming "ticket 007" is stale — that ticket is complete and never claimed the
verification axis. The result is a permanently-dead repair axis: every stale or
contradicted belief breach that should provoke a lawful "go find out" detour instead
collapses straight to `DowngradeToTypedBarrier` or full replan.

This spec closes the bridge for the **co-located witness** case. It is the
highest-leverage post-consolidation AI improvement that is not already implemented,
accepted in the triage of
`reports/ai-architecture-improvements-second-iteration.md` (Proposal 1). It serves
FOUNDATIONS Scenario D (Rumor → Travel → Empty Source → Discovery → Belief Correction
→ Replan) and FND-16 (ignorance/uncertainty/contradiction first-class) by making the
agent's response to a broken belief-backed causal link a lawful evidence-seeking
action rather than a generic reset.

**Architectural constraint (from reassessment).** `attempt_repair_then_replan`
(`plan_repair.rs:74`) is a pure composition engine: it takes only
`(context, cognitive, repair_memory)` and `plan_from_parts` (`plan_repair.rs:261`)
assembles a repaired plan **solely from pre-built `PlannedStep`s** —
`preserved_prefix`, `reusable_suffix`, and the supplied `replacement_candidates`. It
has no planner/search access and cannot turn a goal into steps. Repair candidates are
therefore constructed *upstream* at the revalidation seam
(`agent_tick/execution.rs:412`, `repair_candidates_from_reusable_suffix`), where the
agent's belief view and place context are available. This spec follows that existing
contract: the verification step is built at the seam and passed in as a
`RepairPlanCandidate`; `plan_repair` only selects it.

**Evidence sources.** `reports/ai-architecture-improvements-second-iteration.md`
Proposal 1; verified against `plan_repair.rs`, `agent_tick/execution.rs`,
`candidate_generation.rs`, `decision_event_payload.rs`, the S137/S139 archived specs,
and `S137PLACAULIN-007`. **Key reassessment decisions:** (1) build the verification
step at the revalidation seam, not inside `plan_repair`; (2) scope to the co-located
`AskWitness` path only (a single step that fits `RepairPlanCandidate.step`);
(3) record the verification anchor in the **authoritative** `RepairApplied` event via
the existing `substitute_target` field per FND-29A, not only in the transient
diagnostic trace.

## Phase and Status

Adjunct Wave: AI Architecture Improvements — First Iteration. Draft, pending
ticket decomposition.

## Crates

- `worldwake-ai` —
  - `agent_tick/execution.rs` — the revalidation seam. Extend the repair-candidate
    construction (alongside `repair_candidates_from_reusable_suffix`) to build a
    single-step `AskWitness` verification candidate when the breach is epistemic and a
    lawful co-located witness exists; add the `InsertVerification` arm to
    `substitute_target_from_repaired_plan` so the authoritative event records the
    witness anchor.
  - `plan_repair.rs` — the `InsertVerification` arm selects the supplied verification
    candidate via the existing `attempt_candidate_repair`; returns `NoEpistemicSubstrate`
    when none was supplied (fall-through to `DowngradeToTypedBarrier`).
  - `candidate_generation.rs` — extract a reusable single-`(witness, subject)` →
    `PlannedStep` constructor from the per-witness inner logic of
    `extract_ask_witness_candidates` (the existing emitter is a bulk per-agent pass and
    is not directly reusable for one targeted step).
  - `decision_trace.rs` — extend `RepairAttemptTrace` so an attempted verification
    records the chosen witness anchor and, on rejection, the missing-affordance cause
    (observer/diagnostic detail; the provenance of record is the authoritative event).
  - `plan_revalidation.rs` — the spliced step's payload revalidation (see D6).
- `worldwake-sim` — no new accessors expected. The seam reads the existing co-located
  witness / `entity_beliefs_sourced_from_witness` surfaces S139 added; if a needed read
  is missing it must be added under the Belief-View Accessor Source-Class Rule
  (belief-backed), not by a live world read.
- `worldwake-core` — no new authoritative state and (for the witness anchor) **no new
  field**: `RepairAppliedPayload.substitute_target` (`decision_event_payload.rs:440`)
  already exists. `RepairKind`/`RepairFailure`/`CausalLink`/`CausalProvider` already
  exist. (See D5 for the deferred subject-anchor sub-decision.)
- `worldwake-cli` — observer rendering of the verification repair routes through the
  existing decision-trace path; verify the witness anchor appears in the repair-trace
  summary. No `AgentDef` change.

## Dependencies

- **S137** (Plan Causal Links and Repair) — completed/archived. Provides
  `PlanRepairContext`, `CausalLink`, `RepairKind`, `attempt_repair_then_replan`,
  `attempt_candidate_repair`, `plan_from_parts`, the revalidation-seam candidate
  construction (`repair_candidates_from_reusable_suffix`,
  `substitute_target_from_repaired_plan`), and `RepairMemory` backoff.
- **S139** (Ask-Witness Goal Layer) — completed/archived. Provides
  `GoalKind::AskWitness`, the `ask_witness` action and its
  `validate_ask_witness_payload_override` payload-override validator, the
  `extract_ask_witness_candidates` emitter, `entity_beliefs_sourced_from_witness`, and
  `AskWitnessMemory` cooldown.
- **S149** (Partial Plan Segments and Typed Terminals) — completed/archived.
  Provides `PlanTerminalKind::InformationBarrier { topic }` and the
  `DowngradeToTypedBarrier` fall-through (`typed_barrier_for_repair_context` already
  maps `MissingObservation` → `InformationBarrier`) this spec defers to when no
  verification affordance is lawful.

## Design Goals

1. **Verification is an ordinary lawful affordance, never truth correction.**
   The repair splices a single `ask_witness` action step (toward a co-located witness
   whose belief envelope is a lawful source for the breach subject) before the
   preserved suffix. The belief is not edited by the repair. Any belief change happens
   later, through the `ask_witness` effect sink carrying `PerceptionSource::Report`
   provenance (FND-14, FND-15, Scenario D step 6).
2. **Anchor only on belief-backed breaches.** Verification is built only when
   `context.broken_link.provider` is `CausalProvider::Belief { claim_key }`,
   `CausalProvider::Observation { .. }`, or `CausalProvider::Record { .. }` — i.e. the
   link rested on the agent's belief/observation/record state — and the
   `discrepancy_entry` indicates a stale/contradicted/missing belief
   (`DiscrepancyClearing::BeliefUpdate`/`ReobservationOf`). A breach on a `PriorStep`,
   `CarriedItem`, or `Expectation` provider is not an epistemic breach; no candidate is
   built and `InsertVerification` returns `NoEpistemicSubstrate`.
3. **Single co-located step, built at the seam.** Because `plan_repair` cannot search,
   the verification step is constructed at the revalidation seam where the belief view
   is available, and only when a witness is **co-located** (so the verification is a
   single `ask_witness` step with no travel — fitting `RepairPlanCandidate.step` and
   the one-step-per-repair rule). If no lawful co-located witness exists, no candidate
   is built; `InsertVerification` returns `NoEpistemicSubstrate` and the search falls
   through to `DowngradeToTypedBarrier`, which produces the typed `InformationBarrier`.
4. **Reuse, do not reinvent, step construction.** The `ask_witness` `PlannedStep` is
   built through a constructor extracted from the existing
   `extract_ask_witness_candidates` per-witness logic and uses the same
   `validate_ask_witness_payload_override` validator, so the spliced step is identical
   to one the planner would have produced organically. No second construction path for
   `AskWitness` (FND-28).
5. **Thrash resistance through existing substrate.** Repeated verification for the
   same breach is bounded by the existing `RepairMemory` per-`BreachSignature` backoff,
   `AskWitnessMemory` cooldown (`ask_memory_retention_ticks`), and
   `LearnedOpportunityMemory` damping. No new bounding mechanism is introduced.
6. **Authoritative provenance (FND-29A).** The chosen witness anchor is recorded in the
   **authoritative** `RepairApplied` event through the existing
   `RepairAppliedPayload.substitute_target` field — not only in the transient
   diagnostic trace — so "why did the agent ask this witness to repair this goal?" is
   reconstructable from append-only history.
7. **Determinism.** Witness selection at the seam iterates `BTreeMap`-stable belief
   state; ties broken by the existing candidate-ranking order.

## Non-Goals

- **Place-search / `ExploreLocation` verification.** Deferred to a future sibling spec.
  Exploring a place requires travel (no `explore_location` action exists; `ExploreLocation`
  is consumed by the `travel` action), making it inherently multi-step — incompatible
  with the single-step `RepairPlanCandidate.step` shape and the one-step-per-repair
  rule. The multi-step verification-with-travel case belongs with partial-plan
  suspension machinery (S168 domain), not the synchronous repair seam.
- **`ConsultRecord` / `InspectContainer` verification.** Neither has a `GoalKind`
  surface (S139 deferred both). They require their own goal-layer + access-right
  substrate and belong to a future sibling spec.
- **Forcing belief correction.** The verification may yield nothing (witness ignorant).
  That is a lawful outcome — the agent stays wrong longer (FND-16).
- **`SubstituteMethodBranch`.** Out of scope; deferred with HTN methods per S137.
- **Multi-step verification chains in one repair.** One co-located `ask_witness` step
  is spliced per repair attempt; deeper chains emerge through subsequent ordinary
  planning.

## FOUNDATIONS Alignment

| Principle | How satisfied |
|-----------|---------------|
| FND-14 (World ≠ Belief) | The repair never reads or writes world truth for the breach subject; it splices a lawful action whose effect sink updates belief through the normal carrier. |
| FND-15 (Knowledge acquired locally, travels physically) | The spliced `ask_witness` step acquires evidence through co-location and `PerceptionSource::Report` provenance — the same carrier ordinary sensing uses. Co-location means no travel is hidden inside the repair. |
| FND-16 (Ignorance/uncertainty/contradiction first-class) | A broken belief-backed link triggers a lawful "find out" detour rather than a magic correction or a blind reset; verification may fail, leaving the agent lawfully uncertain. |
| FND-17 (Surprise from violated expectation) | The repair fires only off an existing `DiscrepancyEntry`/broken `CausalLink` — a concrete expectation violation, never global absence detection. |
| FND-20 (Resource-bounded reasoning) | Verification is a bounded affordance under repair budget, witness cooldown, and learned-opportunity damping; no unbounded fan-out. |
| FND-21 (Revisable commitments) | The agent suspends the broken pursuit to verify, then resumes or abandons — commitment held under assumptions, revised on new evidence. |
| FND-29 (Debuggability) | The repair trace records the chosen witness anchor, or the explicit `NoEpistemicSubstrate` cause. |
| FND-29A (Authoritative, queryable history) | The witness anchor is recorded in the append-only `RepairApplied` event (existing `substitute_target` field), not only in the transient diagnostic trace. |

## Deliverables

### D1. Epistemic-breach classification predicate

In `agent_tick/execution.rs` (the seam), add a pure predicate over the breach context
that returns the verification subject when the breach is epistemic: it inspects
`broken_link.provider` for `Belief { claim_key }` / `Observation { observed_entity,
aspect }` / `Record { record_entity, topic }`, and `discrepancy_entry.clearing_condition`
for `BeliefUpdate { claim_key }` / `ReobservationOf { target }`. It yields either a
subject `EntityId` (from `claim_key.subject` / `observed_entity` / `record_entity`) or
`None`. Non-epistemic providers yield `None`. The predicate is consumed at the seam
(D3), not inside the `InsertVerification` arm.

### D2. Single-`(witness, subject)` AskWitness step constructor

In `candidate_generation.rs`, extract the per-witness inner logic of
`extract_ask_witness_candidates` (`candidate_generation.rs:3063`) into reusable
gate/payload helpers plus a repair-facing constructor. Because a `PlannedStep` requires
the concrete `ActionDefId`, the constructor takes the resolved `ask_witness` action id
alongside the agent, co-located witness, subject, and belief view. It returns the
`ask_witness` `PlannedStep` (with the synthesized `AskWitnessPayload` and
`TellTopic::EntityBelief { subject }`) or `None` when the S139 anchoring rule does not
deem that witness a lawful source for the subject or the `AskWitnessMemory` cooldown is
active. `extract_ask_witness_candidates` is refactored to call the same gate/payload
helpers so there is one construction path for the anchoring decision and payload shape
(FND-28). The bulk emitter's behavior is unchanged.

### D3. Seam-side verification-candidate construction

In `agent_tick/execution.rs`, where repair candidates are assembled, build a
verification `RepairPlanCandidate { kind: RepairKind::InsertVerification, step,
… }` when D1 yields a subject AND D2 yields a step for a co-located lawful witness.
Append it to the `replacement_candidates` passed into `attempt_repair_then_replan`.
When D1 yields `None` or no lawful co-located witness exists, build no candidate.

### D4. `InsertVerification` arm

In `plan_repair.rs`, replace
`RepairKind::InsertVerification => Err(RepairFailure::NoEpistemicSubstrate)` with a call
to the existing `attempt_candidate_repair(context, RepairKind::InsertVerification)`,
which selects the supplied verification candidate and composes the repaired plan via
`plan_from_parts` (suffix-dedup provenance preserved). When no `InsertVerification`
candidate was supplied, return `RepairFailure::NoEpistemicSubstrate` so the search
falls through to `DowngradeToTypedBarrier` then `Abandon`.

### D5. Authoritative witness-anchor recording (FND-29A)

In `agent_tick/execution.rs`, add the `RepairKind::InsertVerification` arm to
`substitute_target_from_repaired_plan` (`execution.rs:655`) so the emitted
`RepairApplied(RepairAppliedPayload { repair_kind: InsertVerification, substitute_target:
Some(witness), … })` records the witness in the **authoritative** append-only event log
through the existing `substitute_target` field. No new `RepairAppliedPayload` field and
no `SAVE_FORMAT_VERSION` bump are required for the witness anchor.

*Deferred sub-decision (ticket-time):* whether the belief-*subject* (distinct from the
witness) must also be authoritatively recorded for the applied-but-unexecuted case
(repair selected but the step never runs). In the executed case the subject is already
in the authoritative `ask_witness` action event. If a distinct authoritative subject
record is judged necessary, it requires a new `RepairAppliedPayload` field and a
`SAVE_FORMAT_VERSION` bump (pre-authorized); otherwise `goal_key` + `substitute_target`
suffice. Resolve when decomposing this deliverable.

### D6. Payload revalidation

The spliced verification step uses S139's planner-synthesized `AskWitnessPayload`;
confirm the handler's registered `validate_ask_witness_payload_override`
(`epistemic_actions.rs:27`) accepts it at the revalidation seam
(`plan_revalidation.rs`), per the Authoritative-to-AI Impact Rule item 6.

### D7. Repair trace (observer detail)

`RepairAttemptTrace` (`decision_trace.rs`) now includes the optional
`verification_anchor` diagnostic field in addition to `breach`, `chosen_kind`,
`rejected`, `budget_consumed`, and `budget_total`. Attempted verification records the
chosen witness anchor; rejected verification continues to record its missing-affordance
cause through the existing `rejected` entries rather than a bare placeholder. This is
the AI-crate diagnostic trace (consumed by `scenario_diagnostics`); it is **not** the
provenance of record (that is D5's authoritative event) and adds no
`SAVE_FORMAT_VERSION` impact. Observer rendering reuses the existing repair-attempt
summary path.

### D8. Test migration

Replace the three "without_s139" assertions. The unit/scenario tests must now assert:
(a) an epistemic breach with a lawful co-located witness yields a `Repaired` outcome
whose plan contains an `ask_witness` step toward that witness AND whose `RepairApplied`
event records `substitute_target = Some(witness)`; (b) an epistemic breach with **no**
lawful co-located witness yields `NoEpistemicSubstrate` and falls through to
`DowngradeToTypedBarrier` (typed `InformationBarrier`); (c) a non-epistemic breach
(e.g. `PriorStep` provider) still yields `NoEpistemicSubstrate`.

## FND-01 Section H

1. **Information-path analysis.** Verification acquires knowledge through the shipped
   `ask_witness` effect sink (`PerceptionSource::Report { from: witness, chain_len: 1 }`).
   The breach itself arrived through the existing `DiscrepancyEntry`/`CausalLink`
   machinery (revalidation seam). No information reaches the agent without a traceable
   carrier; the repair adds an *action that creates a carrier*, not a read.
2. **Positive-feedback analysis.** One potential loop: breach → verify → fresh belief
   → new plan → new breach → verify. Identical in shape to the S139 verification loop.
3. **Concrete dampeners.** `AskWitnessMemory.ask_memory_retention_ticks` cooldown per
   `(witness, topic)`; `RepairMemory` per-`BreachSignature` backoff (`RecentlyFailed`);
   `LearnedOpportunityMemory` damping of fruitless verification; the repair budget
   fraction (`CognitiveProfile.repair_budget_fraction`). All are pre-existing
   physical/world-process dampeners (witness availability, attention cooldown, bounded
   reasoning budget), not numeric clamps.
4. **Stored state vs. derived read-model.** No new authoritative state type. The
   verification subject and constructed step are transient per-repair derived values.
   The witness anchor is written into the **existing** authoritative
   `RepairAppliedPayload.substitute_target` (D5) — an existing field, not a new
   authoritative type. `RepairMemory`/`AskWitnessMemory`/`LearnedOpportunityMemory`
   (all pre-existing authoritative state) are read, and `RepairMemory` is written
   exactly as S137 already does. `RepairAttemptTrace` (D7) is a derived diagnostic
   view, not authoritative.
5. **Planner-formalism analysis.** Plain GOAP/affordance composition. The verification
   step is an ordinary `ask_witness` affordance built at the seam and selected by the
   existing candidate-repair path. No HTN method, no method-required leaf, no scenario
   rail. `plan_repair` performs no search (it cannot); the step is pre-built.
6. **Causal-equivalence contract.** Not applicable for the witness-anchor path — no
   offscreen simulation, compression, or new serialized field. `SAVE_FORMAT_VERSION`
   is unchanged because D5 reuses the existing `substitute_target` field and D7 extends
   a non-saved diagnostic trace. (Only the deferred D5 subject-record sub-decision could
   introduce a new serialized field and a save-format bump, resolved at ticket time.)
7. **Systemic-validation analysis.** Cross-system (AI seam + AI repair + sim action +
   belief import). Negative illegal paths the feature must not produce: (a) belief
   becoming true with no carrier; (b) verification toward a witness the agent has no
   lawful co-located knowledge of; (c) a verification step appearing without
   preconditions, duration, or cost; (d) the repaired plan reading world truth for the
   breach subject; (e) a multi-step (travel-bearing) verification spliced into a single
   repair candidate. Checks: focused repair/seam tests (D8), one golden proving
   Scenario D end-to-end (stale belief → mismatch → spliced co-located `ask_witness` →
   lawful evidence → resume or lawful abandonment with discrepancy retained, and the
   authoritative `RepairApplied` event records the witness), and a replay/save-load
   equivalence check on that golden. The existing 1440-tick survival goldens must not
   regress (verification fires only on belief-backed breaches).

## SystemFn Integration

No new `SystemFn`. Candidate construction and repair run inside the existing
`agent_tick` revalidation seam (`attempt_repair_then_replan`). The spliced action
executes through the existing `ask_witness` handler and scheduler.

## Component Registration

No new components. No registration change.

## Cross-System Interactions (FND-26)

- **AI → AI**: the revalidation seam (`agent_tick/execution.rs`) reads the belief view
  to build the verification step (D2/D3) and passes it to `plan_repair` as a candidate;
  same-crate, state-mediated through the belief view.
- **AI → Sim**: the repaired plan's `ask_witness` step dispatches the existing action;
  no new direct call.
- **Sim → Core**: the action's effect sink imports belief via the shipped path; the
  `RepairApplied` event is appended to the core event log.
No new direct cross-system call.

## Profile-Driven Parameters

No new parameters. Reuses `EpistemicDispositionProfile.stale_evidence_barrier_threshold`
and `ask_memory_retention_ticks` (S139), and `CognitiveProfile.repair_budget_fraction`
/ `repair_memory_ticks` (S137). Two agents with different thresholds verify or barrier
differently on the same breach.

## Authoritative-to-AI Impact Analysis

1. `get_affordances` — N/A; verification reuses the shipped `ask_witness` affordance.
2. `generate_candidates` — affected; the single-`(witness, subject)` step constructor
   (D2) is extracted from `extract_ask_witness_candidates` and reused at the seam (D3).
   The bulk emitter's behavior is unchanged.
3. `search_plan` — N/A; the co-located `ask_witness` verification is a single pre-built
   step, so no multi-step search is invoked inside the repair.
4. `BestEffort` action start — N/A; reuses the shipped `ask_witness` handler.
5. `handle_plan_failure` — affected; on `NoEpistemicSubstrate` the search must fall
   through to `DowngradeToTypedBarrier` then `Abandon`, then the existing full-replan
   path; verify no infinite repair loop.
6. **Payload revalidation** — affected (D6); the synthesized `AskWitnessPayload` must
   pass `validate_ask_witness_payload_override` at the seam.
7. Golden tests — required (D8 + the Scenario D golden).

## Validation and Falsification (FND-31)

- **Golden**: `golden_epistemic_verification_repair.rs` — agent acts on a stale
  belief-backed plan, the link breaks at the revalidation seam, a co-located
  `ask_witness` verification candidate is built and selected, the action executes,
  evidence imports with `Report` provenance, the authoritative `RepairApplied` event
  records `substitute_target = Some(witness)`, and the agent resumes or abandons with
  the discrepancy retained until the carrier updates the belief. Plus a no-witness
  branch proving fall-through to a typed `InformationBarrier`.
- **Focused**: D8 unit/scenario tests, including the authoritative-anchor assertion.
- **Negative cases**: no belief-without-carrier correction; no verification toward a
  non-co-located or unknown witness; verification step always carries duration/cost;
  no travel-bearing step spliced into a single repair candidate.
- **Replay/save-load equivalence**: the golden replays identically;
  `SAVE_FORMAT_VERSION` is unchanged.
- **No-regression**: 1440-tick survival goldens unaffected.

## Risks

- **Verification thrash.** Mitigated by reused `RepairMemory`/`AskWitnessMemory`/
  `LearnedOpportunityMemory` bounding; the golden's no-witness branch locks the
  fall-through.
- **Anchor over-eagerness.** The S139 anchoring rule (lawful co-located witness for the
  subject) must gate construction at the seam; a too-loose anchor would let the agent
  "verify" with a witness it has no lawful reason to consult. The source-class
  declaration (belief-backed) and the negative test guard this.
- **Single-construction-path discipline.** D2 must be the only `ask_witness`
  step-construction path; if `extract_ask_witness_candidates` keeps a parallel inline
  construction, the two can drift (FND-28). The refactor must route both through the
  extracted constructor.
- **Stale test fossils.** The three "without_s139" tests must be migrated, not
  deleted-and-skipped, so the closed gap stays proven.
