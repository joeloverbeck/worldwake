# S169: Generalized Lawful Verification Substrate

**Status**: DRAFT

## Problem Statement

`RepairKind::InsertVerification` now succeeds (`plan_repair.rs:131-132`) but only
when a `RepairPlanCandidate` with an `AskWitness` step is supplied. The candidate
construction is exclusively `ask_witness_verification_step()`
(`crates/worldwake-ai/src/candidate_generation.rs:3280`) invoked from
`append_insert_verification_candidate()`
(`crates/worldwake-ai/src/agent_tick/execution.rs:452-491`); every other breach type
collapses to `RepairFailure::NoEpistemicSubstrate` and falls through to
`DowngradeToTypedBarrier` or `Abandon`. The information-barrier companion path
(`agenda_manager.rs:309-327`) is symmetrically hardcoded: it constructs a
`GoalKind::AskWitness { witness, topic }` companion regardless of the breach's
information-carrier class.

This is a half-finished substrate. `ConsultRecord` and `SearchPlace` are real
lawful actions today
(`crates/worldwake-systems/src/consult_record_actions.rs:45-62` and
`crates/worldwake-systems/src/search_actions.rs:40-62`) with same-place
visibility, payload validators, duration, and authoritative validation. The
prior triage
(`docs/triage/2026-05-22-ai-architecture-improvements-second-iteration-triage.md`)
explicitly identified the gap as a known follow-up: *"`ConsultRecord` /
`InspectContainer` as verification `GoalKind`s — needed to widen S165 beyond
`AskWitness`/`ExploreLocation`; deferred with S139's own Non-Goals."* The
third-iteration report (`reports/ai-architecture-improvements-third-iteration.md`
Proposal 1) re-raises it with the same operative claim verified against current
`main`.

The architectural cost of the gap is concrete. Stale or contradicted *social* and
*institutional* facts — office holdership, jurisdiction, bounty status, route
safety as recorded, ownership/rights claims sourced from records — can never be
lawfully repaired through the verification axis today. They reach `RepairKind::
InsertVerification` and emit no candidate because the candidate constructor only
knows how to ask a co-located witness. The witness path is correct for *entity
belief* topics (`TellTopic::EntityBelief { subject }`,
`agenda_manager.rs:321-323`); it is wrong as a sole strategy when the
information-carrier class is a record or a place to be searched. The agent
collapses to `Abandon` and replans, which is *not* the FOUNDATIONS-prescribed
behaviour for FND-16/17/18/21: stale belief should provoke a lawful evidence
seek through a record or a place, not a hidden reset.

**Critical scoping decision.** The third-iteration report lists four providers
(AskWitness, ConsultRecord, SearchPlace, direct same-tick local observation) and
a "future inspection provider." Same-tick local observation is dropped from this
spec: it is not a planned action that repair can splice — it is the FND-14A
belief-view layer's automatic behaviour at the perception step, and modelling it
as a verification "provider" is a category error. The future-inspection
placeholder is also dropped per FND-28 (no fossils). Three providers only.

**Scope boundary.** This spec extends the **repair seam** only. Information-
barrier *agenda companion* extension to non-AskWitness goals requires new
`GoalKind` variants (`GoalKind::ConsultRecord`, etc.) with full candidate
generation, ranking, payload override, and HTN compatibility — that scope is
deferred to a follow-up spec. S169 makes the repair seam polymorphic; goal-
level companions remain `AskWitness`-only and become a clear next step.

**Evidence sources.** `reports/ai-architecture-improvements-third-iteration.md`
Proposal 1 (rank 1, "Adopt");
`docs/triage/2026-05-25-ai-architecture-improvements-third-iteration-triage.md`;
the prior follow-up reaffirmation in the 2026-05-22 second-iteration triage; and
current-code citations in this spec.

## Phase and Status

Adjunct Wave: AI Architecture Improvements — Second Iteration. Independent of
S170 (Learned-State Provenance Hardening); may land in either order. Builds on
the completed S137 (repair context), S139 (`AskWitness` goal substrate), and
S165 (the repair-axis bridge for the AskWitness case).

## Crates

- `worldwake-ai`
  - `src/plan_repair.rs` — the `InsertVerification` arm continues to consume
    `RepairPlanCandidate`s from `PlanRepairContext.replacement_candidates`
    (`plan_repair.rs:18`); no change inside `attempt_repair_then_replan`.
  - `src/verification_provider/` (new module) — `VerificationNeed`,
    `VerificationCandidate`, the `VerificationCandidateProvider` trait, the
    fixed three-element provider registry, and provider-selection trace.
  - `src/agent_tick/execution.rs` — the revalidation seam
    (currently builds the AskWitness verification candidate inline near
    `agent_tick/execution.rs:412`). Refactor to delegate to the provider
    registry and accept all three op kinds as candidate steps.
  - `src/decision_trace.rs` — extend the existing `RepairAttemptTrace`
    struct (`decision_trace.rs:199-206`, which already carries
    `verification_anchor: Option<EntityId>` from S165) with two new
    per-attempt fields: `verification_provider: Option<VerificationProviderKind>`
    and `verification_rejections: Vec<(VerificationProviderKind,
    VerificationRejection)>`.
- `worldwake-core`
  - `src/decision_event_payload.rs` — extend `RepairAppliedPayload`
    (`decision_event_payload.rs:435`) with a single new field
    `provider_kind: VerificationProviderKind`. The verification target
    is carried by the existing `substitute_target: Option<EntityId>` field
    (which already records the AskWitness witness anchor per S165) — its
    semantic interpretation (witness / record / place) is disambiguated by
    `(repair_kind = InsertVerification, provider_kind)`. No parallel
    `target` field is introduced — see Addition 1 / FND-28. Preserves
    FND-29A causal history.
  - `src/decision_event_payload.rs` — define
    `pub enum VerificationProviderKind { AskWitness, ConsultRecord,
    SearchPlace }` here (alongside `RepairAppliedPayload` so the payload's
    field type is locally resolvable from core). The enum is payload-free
    with derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd,
    Hash, Serialize, Deserialize` to match sibling event-payload types.
    No core-side mirror needed (Q1=(a)): the enum is semantically
    identical between core and ai, so a `*Tag` mirror would create
    parallel authority per FND-28.
- `worldwake-systems`
  - No authoritative validator changes expected. `ConsultRecord` and
    `SearchPlace` action defs already enforce same-place legality + payload
    validity at dispatch time. Any missing validator for repair-side
    synthesized payloads is added under D7.

## Dependencies

- **Completed**: S137 (repair search + `CausalLink` provenance), S139
  (`AskWitness` goal), S149 (partial-plan / barrier classification), S165
  (AskWitness repair-axis bridge).
- **No new dependencies** on S60–S66 (gameplay specs excluded per directive).
- **Does not depend on S170**; both specs may land in parallel.

## Design Goals

1. **Polymorphic repair-seam verification.** The verification axis must produce
   a lawful repair candidate for at least three breach classes: stale entity
   belief (existing AskWitness path), stale institutional/record-backed claim
   (new ConsultRecord path), and overdue/unknown expectation about a place
   (new SearchPlace path).
2. **One provider registry, three concrete providers.** No generic
   "extensibility hook" with empty slots; a fixed enum-dispatched registry of
   exactly three providers, each with a typed `try_build(...)` returning
   `Option<VerificationCandidate>` plus a `RejectionReason` for absences.
3. **Trace-able selection and rejection.** The decision trace records both the
   selected provider (if any) and every rejected provider with its rejection
   reason (no lawful local witness, no lawful local record, no lawful local
   place, breach type does not match this provider, etc.). FND-29.
4. **Authoritative provider provenance.** The `RepairAppliedPayload` extends
   with a single new `provider_kind` field; the existing `substitute_target`
   field continues to carry the verification target (witness/record/place
   `EntityId`). The chosen verification is reconstructible from history
   (FND-29A), not only from a transient decision trace.
5. **Same-place legality preserved.** Every provider consumes only the actor's
   `PerAgentBeliefView` and same-place observations. No provider may read
   remote world state to determine candidate availability. FND-14B.
6. **S165 parity preserved.** The existing `AskWitness` verification scenario
   and its goldens must continue to pass with byte-identical authoritative
   event sequences (modulo the new `provider_kind` field).

## Non-Goals

1. **Goal-level agenda companion polymorphism.** Extending
   `information_barrier_companion_entry` (`agenda_manager.rs:309-327`) to
   construct `GoalKind::ConsultRecord` or other non-AskWitness goals is **out
   of scope**. That work needs new `GoalKind` variants, candidate generation
   extractors, ranking discounts, and HTN compatibility. Tracked as a follow-up
   if S169's provider abstraction surfaces enough pressure.
2. **New `GoalKind` variants.** No new goal kinds. Verification at the repair
   seam is performed by splicing a *step* (an action invocation packaged in a
   `RepairPlanCandidate`), not by committing to a new goal.
3. **Direct same-tick local observation provider.** Dropped per the scoping
   decision above. Same-tick co-located observation is already handled by the
   belief-view layer under FND-14A; it is not a planned action.
4. **Future / placeholder providers.** No empty registry slots, no
   "inspection" provider stub. FND-28 (no fossils).
5. **Candidate ranking changes.** The provider chooses the lawful candidate;
   ranking remains driven by existing motive/source/feasibility machinery.
6. **Cross-agent / cross-place verification.** A verification candidate is
   lawful only if the *actor* is at the same place as the witness, record, or
   target place. No remote-target verification.

## FOUNDATIONS Alignment

| Principle | Application |
|---|---|
| FND-1 (local causality) | Verification candidates are constructed from actor-local belief + same-place perception only. |
| FND-7 (locality) | Each provider's `try_build` accepts only the actor's `PerAgentBeliefView` and same-place context; no global state queries. |
| FND-14 (belief ≠ world) | Providers read only the agent's belief and same-place physical state; no provider may read remote authoritative truth. |
| FND-14A (same-tick local observation) | Co-location with a record or a witness is read as same-tick physical perception (the artifact's existence and `kind` are physical). Social fields on the artifact (recorded claim contents, source attribution beyond co-location) remain belief-gated. |
| FND-14B (planner-visible inputs) | The candidate step is sourced from actor self-state, actor belief, and same-tick local physical observation only. The candidate's `step` carries no remote-fact payload. |
| FND-15 (knowledge travels physically) | `ConsultRecord` and `SearchPlace` are real lawful carriers; the verification splice schedules them, not a hidden world-truth read. |
| FND-16 (ignorance/contradiction first-class) | Stale/contradicted/disputed belief is the *trigger* for verification; the substrate is what lets the agent act on the uncertainty lawfully. |
| FND-17 (surprise from violated expectation) | Overdue-expectation breach is one of the new breach classes the SearchPlace provider serves. |
| FND-18 (records are world state) | The ConsultRecord provider treats records as first-class evidence carriers. |
| FND-20 (bounded reasoning) | Provider iteration is bounded by the existing `repair_budget`; no new unbounded loops. |
| FND-21 (revisable commitments) | A verification step is itself a revisable commitment; if interrupted, the agent's intention resumes or replaces lawfully via existing partial-plan machinery. |
| FND-28 (no fossils) | No placeholder providers; the registry is exactly the three implementations. |
| FND-29 (debuggability) | Provider selection and per-provider rejection reasons are written to `DecisionTrace`. |
| FND-29A (causal history) | `RepairAppliedPayload` extends with `provider_kind`; the existing `substitute_target` field carries the target entity so the choice survives in append-only history. |
| FND-31 (validation/falsification) | Three goldens (consult-record, search-place, negative omniscience) plus parity with the S165 AskWitness golden. |

## Deliverables

### D1. `VerificationNeed` and `VerificationCandidate` types

`VerificationProviderKind` lives in `worldwake-core` (defined in
`decision_event_payload.rs` per the Crates section) because
`RepairAppliedPayload.provider_kind` consumes it. All other types in this
deliverable live in a new module `crates/worldwake-ai/src/verification_provider/mod.rs`:

```rust
// in worldwake-core/src/decision_event_payload.rs:
pub enum VerificationProviderKind { AskWitness, ConsultRecord, SearchPlace }

// in worldwake-ai/src/verification_provider/mod.rs:
pub enum VerificationNeed {
    StaleEntityBelief { subject: EntityId, aspect: EntityBeliefAspect },
    StaleInstitutionalClaim { record_topic: RecordTopic },
    OverdueExpectationAtPlace { expectation: ExpectationId, place: EntityId },
}

pub struct VerificationCandidate {
    pub provider_kind: VerificationProviderKind,
    pub target: VerificationTarget,
    pub repair_candidate: RepairPlanCandidate,
    pub source_belief: Option<BeliefRef>,
}

pub enum VerificationTarget {
    Witness(EntityId),
    Record(EntityId),
    Place(EntityId),
}
```

`VerificationTarget` is a runtime-only enum: when the candidate is
serialized into `RepairAppliedPayload`, only the inner `EntityId` is
persisted (into the existing `substitute_target` field); the
witness/record/place discrimination is reconstructible from
`provider_kind` alone.

Classification of a breach into a `VerificationNeed` happens at the seam by
reading `CausalLink.provider` and `CausalLink.fact` from the
`PlanRepairContext.broken_link`, together with the actor's
`PerAgentBeliefView`. `BreachSignature` is unchanged — it is a `struct`
(`crates/worldwake-core/src/repair_memory.rs:8`) used as a `RepairMemory`
key, not an enum to extend. If no `VerificationNeed` class applies, the
seam emits no `VerificationCandidate` and the `InsertVerification` arm
fails with `NoEpistemicSubstrate` exactly as today.

### D2. `VerificationProvider` enum-dispatched registry

A fixed enum-dispatched registry of three providers. With exactly three
known providers and FND-28 forbidding extensibility-for-its-own-sake, an
enum + match dispatch is preferred over a `Box<dyn Trait>` registry — no
vtable, no heap allocation per provider, exhaustive-match enforcement at
compile time.

```rust
pub enum VerificationRejection {
    BreachClassMismatch,
    NoLawfulLocalTarget,
    PayloadValidationFailed,
    RecentlyFailedAtTarget,
}

pub fn try_build_verification_candidate(
    provider: VerificationProviderKind,
    need: &VerificationNeed,
    ctx: &VerificationContext<'_>,
) -> Result<VerificationCandidate, VerificationRejection> {
    match provider {
        VerificationProviderKind::AskWitness   => ask_witness_provider::try_build(need, ctx),
        VerificationProviderKind::ConsultRecord => consult_record_provider::try_build(need, ctx),
        VerificationProviderKind::SearchPlace   => search_place_provider::try_build(need, ctx),
    }
}
```

Each provider's `try_build` is a free function in its own submodule
(`verification_provider/ask_witness_provider.rs`,
`verification_provider/consult_record_provider.rs`,
`verification_provider/search_place_provider.rs`).

`VerificationContext` carries the actor `EntityId`, the actor's
`PerAgentBeliefView`, the actor's effective place, the breach context, and the
seam's existing repair scaffolding. **No `&World` access**.

Registry iteration at the call site is a single fixed loop over
`[AskWitness, ConsultRecord, SearchPlace]` in that declared deterministic
order. The first `Ok(...)` wins; the others' rejection reasons are still
emitted to the trace.

### D3. AskWitness provider parity

Reimplement the current `append_insert_verification_candidate` logic
(`crates/worldwake-ai/src/agent_tick/execution.rs:452-491`) as
`ask_witness_provider::try_build`. The S165 golden suite must continue
to pass — this is the parity gate.

### D4. ConsultRecord provider

`consult_record_provider::try_build` accepts a `VerificationNeed::Stale
InstitutionalClaim { record_topic }`. It searches the actor's
`entities_at(effective_place)` for a `kind = EntityKind::Record` whose recorded
topic is — *per the actor's belief about that record* — relevant to the
`record_topic`. The candidate's `step` is a `ConsultRecord` action invocation
on the local record, using the existing `ConsultRecord` action def
(`crates/worldwake-systems/src/consult_record_actions.rs:25`) and reusing
its existing payload-override validator
`validate_consult_record_payload_override`
(`consult_record_actions.rs:147`) — no new validator is added; see D7.

### D5. SearchPlace provider

`search_place_provider::try_build` accepts `VerificationNeed::OverdueExpectation
AtPlace { expectation, place }`. The target place must equal the actor's
effective place. The candidate's `step` is a `SearchPlace` action invocation
on the actor's current place, using the existing `SearchPlace` action def
(`crates/worldwake-systems/src/search_actions.rs:25`) and reusing its
existing payload-override validator `validate_search_place_payload_override`
(`search_actions.rs:132`) — no new validator is added; see D7.

### D6. Seam integration

Refactor the inline verification-candidate construction at
`agent_tick/execution.rs:412` (alongside `repair_candidates_from_reusable_
suffix`) to: (a) classify the breach into `Option<VerificationNeed>`, (b) when
present, iterate the provider registry and collect the first `Ok` candidate
plus all rejections, (c) thread the chosen `RepairPlanCandidate` into
`PlanRepairContext.replacement_candidates` exactly as today.

### D7. Payload override validator reuse

All three action defs already register `with_payload_override_validator`:
- `validate_ask_witness_payload_override`
  (`crates/worldwake-systems/src/epistemic_actions.rs:155`, registered at
  line 27)
- `validate_consult_record_payload_override`
  (`crates/worldwake-systems/src/consult_record_actions.rs:147`, registered
  at line 28)
- `validate_search_place_payload_override`
  (`crates/worldwake-systems/src/search_actions.rs:132`, registered at
  line 29)

D4 and D5's repair-synthesized payloads must satisfy these existing
validators (the same validators that gate affordance-derived payloads).
Verify in unit tests that each provider's synthesized payload passes the
corresponding validator. No net-new validator code is added by this spec;
this deliverable's work is the verification that synthesis matches existing
validation, not new validator registration.

### D8. `RepairAppliedPayload` event extension

The authoritative event for a successful verification repair already records
the witness anchor via the existing `substitute_target: Option<EntityId>`
field on `RepairAppliedPayload`
(`crates/worldwake-core/src/decision_event_payload.rs:435`, used by S165
for the AskWitness witness anchor). Extend `RepairAppliedPayload` with a
single new field `provider_kind: VerificationProviderKind` so all three
provider kinds are reconstructible from append-only history. The target
entity (witness / record / place) continues to be carried by the existing
`substitute_target` field; its semantic interpretation is disambiguated by
`(repair_kind = InsertVerification, provider_kind)`. No parallel `target`
field is introduced (FND-28: single authoritative field). FND-29A.

### D9. Decision trace per-attempt

Extend the existing `RepairAttemptTrace` struct
(`crates/worldwake-ai/src/decision_trace.rs:199-206`, which already carries
`verification_anchor: Option<EntityId>` from S165) with two new per-attempt
fields:

```rust
pub struct RepairAttemptTrace {
    // ... existing fields ...
    pub verification_anchor: Option<EntityId>,           // S165, unchanged
    pub verification_provider: Option<VerificationProviderKind>,
    pub verification_rejections: Vec<(VerificationProviderKind, VerificationRejection)>,
}
```

`verification_provider` is `Some(kind)` when a provider built a candidate
that was selected for the repair, mirroring the existing
`verification_anchor` field (which carries the target `EntityId`).
`verification_rejections` records every provider examined that did not
produce a candidate, with its rejection reason. Per-attempt placement
matches the granularity of the existing `verification_anchor` and
`chosen_kind` fields; no separate top-level `verification_provider_selection`
field is added to `AgentDecisionTrace`. FND-29.

### D10. Goldens

Three new scenarios under `crates/worldwake-ai/tests/scenarios/`:

1. **`verification_consult_record_repair.rs`** — agent holds a stale
   `RecordTopic::OfficeRule`-class belief; a co-located record exists; the
   repair seam splices a `ConsultRecord` verification step; the belief is
   updated via the authoritative action's effect schema (not by direct
   write); assert provider selection trace, RepairApplied provider_kind,
   absence of `NoEpistemicSubstrate`.
2. **`verification_search_place_repair.rs`** — agent has an overdue
   expectation at the actor's current place; the repair seam splices a
   `SearchPlace` verification step; assertions parallel D10.1.
3. **`verification_no_remote_truth.rs`** — negative omniscience test. The
   stale belief is about a record at a *remote* place; no provider builds a
   candidate; `NoEpistemicSubstrate` is the lawful outcome. Assert that no
   `RepairApplied` event with the remote record's contents is produced.

Plus parity assertion: the S165 AskWitness golden runs through the new
registry path and produces byte-identical authoritative events modulo the new
`provider_kind = AskWitness` field.

## FND-01 Section H

### Information-path analysis

Information enters the substrate via:
- The actor's `PerAgentBeliefView` (the breach itself — a stale or contradicted
  belief is the *trigger* state).
- Same-tick local physical observation under FND-14A: which records, witnesses,
  or place-targets are co-located with the actor.
- The actor's belief about the candidate carrier's relevance (the actor must
  *believe* the record's topic relates to the breach, not query authoritative
  record contents).

Information *exits* the substrate via the spliced action's effect schema
executing through the normal authoritative path: `ConsultRecord` produces a
perception event that updates the actor's record-derived belief; `SearchPlace`
produces a perception event that updates the actor's discovery-derived belief;
`AskWitness` continues per S139/S165. No provider writes belief directly.

### Positive-feedback analysis

A naïve concern: verification cascades — agent verifies belief A, the new
evidence contradicts belief B, agent verifies B, et cetera. This is a loop of
the form *verification → new evidence → new contradiction → new verification*.

### Concrete dampeners

- **Per-action duration** (FND-8). Each verification action is a real
  duration-bearing action (`DurationExpr::ConsultRecord`,
  `DurationExpr::ActorInvestigationDisposition`,
  `DurationExpr::AskWitness`) that occupies the actor's body/attention. The
  agent cannot chain verifications faster than ticks pass.
- **`repair_budget`** per tick (`plan_repair.rs:55-60`). Each
  `InsertVerification` attempt consumes one expansion; chained verifications
  exhaust the budget and force fallthrough to `DowngradeToTypedBarrier` or
  `Abandon`.
- **`RepairMemory::recently_failed`** (`plan_repair.rs:109-118`). A repair
  kind that recently failed against the same `BreachSignature` is skipped;
  this prevents repeated provider thrash on a single belief.
- **Agenda capacity**. Verification splices steps into an *existing* plan
  tail; it does not introduce new agenda entries. The total active-goal
  population is unchanged.

No numerical clamps are introduced. The dampeners are all real world processes
(action time, agent attention, recently-failed memory).

### Stored state vs derived read-model list

**Authoritative stored state** introduced by this spec:
- `RepairAppliedPayload.provider_kind: VerificationProviderKind` event field
  (FND-29A append-only history). The existing `substitute_target` field
  is unchanged; its semantic role is disambiguated by `provider_kind`.
- The `VerificationProviderKind` enum itself is a code-level type defined
  in `worldwake-core/src/decision_event_payload.rs`; only the persisted
  enum *value* on each `RepairApplied` event is authoritative state.

**Derived / transient** introduced by this spec:
- `VerificationNeed`, `VerificationCandidate`, `VerificationTarget`,
  `VerificationRejection`, `VerificationContext` — all per-repair-attempt
  computed values; never persisted to component state or event log. The
  `VerificationTarget` enum's witness/record/place discrimination is
  reconstructible at audit time from the persisted `provider_kind`.
- `RepairAttemptTrace.verification_provider` and
  `RepairAttemptTrace.verification_rejections` — observer-visible trace
  details, derived per repair attempt, not authoritative.

No belief, learned-state, or component field is added by this spec.

### Planner-formalism analysis

Plain GOAP-level repair. A verification candidate is a single `PlannedStep`
spliced into an existing plan tail by the established repair pipeline (S137,
S165). No HTN methods are added. No `RequiredActionLeaf` is introduced.

### Causal-equivalence contract

No new caches, snapshots, region summaries, or save/load surfaces are
introduced. Existing serializable components are unchanged. The
`RepairAppliedPayload` gains one new field (`provider_kind`); old saved
events deserialize with `provider_kind` defaulted to `AskWitness` (the
S165 status quo) under standard `serde` `default` attribute. Replay
equivalence: a save from pre-S169 reloaded post-S169 produces identical
world meaning; a save from post-S169 reloaded post-S169 round-trips the
new field exactly. No causal-variable elision.

### Systemic-validation analysis (FND-31)

Required checks:
- **Provider-parity invariant**: `AskWitnessProvider` produces, for the
  pre-S169 breach types, an identical `RepairPlanCandidate` to the prior
  inline construction. Unit test in
  `crates/worldwake-ai/src/verification_provider/tests.rs`.
- **Locality invariant**: each provider's `try_build` free-function
  signature accepts only `&VerificationNeed` and `&VerificationContext<'_>`
  (no `&World`); compile-time enforced by the function signature plus a
  unit test that constructs a witness/record/place outside the actor's
  place and asserts `VerificationRejection::NoLawfulLocalTarget`.
- **Negative omniscience golden** (D10.3): under FND-14B, no provider may
  emit a candidate whose payload reflects remote authoritative truth.
- **Provider-selection trace completeness**: golden assertion that every
  successful repair attempt records at least one of `selected` or `rejected`
  entries per provider examined.
- **Authoritative-event reconstructibility**: a `RepairApplied` event must
  carry sufficient information (provider_kind + target) to reproduce the
  agent's verification choice from history alone.
- **Save/load round-trip**: a successful verification repair, saved and
  reloaded, replays to the same plan tail.

Negative illegal paths the substrate must not produce:
- A verification step whose target is at a different place than the actor.
- A verification candidate emitted from a `VerificationContext` whose
  belief-view denies access to the carrier.
- A `RepairApplied` event recording a provider that the actor's belief-view
  did not lawfully see.
- Belief updates that bypass the action's effect schema (silent direct
  writes).

## SystemFn Integration

No new `SystemFn` registrations. The verification provider registry is
invoked at the existing revalidation seam (`agent_tick/execution.rs:412`)
during the agent's normal tick. The downstream authoritative actions
(`ConsultRecord`, `SearchPlace`, `AskWitness`) already have handlers registered
in `worldwake-systems`; no handler changes required beyond the D7 payload
override validators.

## Component Registration

No new ECS components. No new `EntityKind::Agent` profile fields. The Agent
Profile Scenario Contract from `docs/spec-drafting-rules.md` does not apply.

## Cross-System Interactions (FND-26)

The substrate is contained entirely inside `worldwake-ai`. Its interactions
with other crates are mediated through:

- **state reads**: `PerAgentBeliefView` (already a stable interface);
  `entities_at(place)` for same-place co-location.
- **action dispatch**: spliced `PlannedStep`s execute through the existing
  authoritative `ActionDef` pipeline in `worldwake-systems`. No direct
  cross-system calls.
- **event log**: `RepairApplied` event extension is written through the
  existing event-append surface.

`worldwake-systems` is not aware of the provider registry and gains no
dependency on `worldwake-ai`. FND-26 preserved.

## Profile-Driven Parameters

No new numeric parameters. The substrate inherits its budgeting from the
existing `CognitiveProfile.repair_budget_fraction` (`Permille`-typed,
`plan_repair.rs:55-60`). No `f32`/`f64` literals or hardcoded constants are
introduced. The deterministic provider iteration order is a fixed code-level
sequence, not a tunable.

## Authoritative-to-AI Impact Analysis

Per CLAUDE.md's "Authoritative-to-AI Impact Rule":

1. **`get_affordances` produces correct candidates** — unchanged. The verification
   step targets a co-located witness/record/place that is already a valid
   affordance for the underlying action.
2. **`generate_candidates` emits the right goal kinds** — unchanged. No new
   goal kinds added.
3. **`search_plan` finds valid plans** — repair runs before search; the
   spliced verification step is appended to a *pre-built* prefix. Search is
   not re-invoked for verification.
4. **`BestEffort` action start** — the spliced step uses an existing action
   op kind; the existing start handler applies.
5. **`handle_plan_failure` replans correctly** — if the verification step
   itself fails at dispatch, the existing failure path runs (plan repair
   re-attempts with `RepairMemory::recently_failed` blocking the same
   provider).
6. **Payload revalidation** — N/A on net-new validator registration. The
   three action defs already register payload-override validators (D7).
   D4 and D5's synthesized payloads must satisfy the existing validators;
   D7 verifies this in unit tests. Without compatibility,
   `plan_revalidation.rs::requested_affordance_matches` would silently
   reject the step — D7 is the affirmative check.
7. **All goldens pass** — D10's three new goldens plus the S165 parity
   assertion plus the full `cargo test -p worldwake-ai` suite.

## Validation and Falsification (FND-31)

Negative cases the substrate must not produce:
- A `RepairApplied` event recording a non-co-located target.
- A verification step whose synthesized payload encodes a remote-record fact
  the actor's belief-view did not contain.
- A `cargo test -p worldwake-ai` regression on the S165 golden.
- A provider selection without a corresponding `provider_kind` field in the
  emitted authoritative event.

Required tests:
- Unit: provider parity (D3); locality enforcement (each provider); rejection
  reason coverage.
- Integration: `InsertVerification` end-to-end with each provider; payload
  override validator registration.
- Golden: three scenarios per D10.
- Save/load: a successful ConsultRecord verification, saved mid-repair-tick
  and reloaded, completes identically.
- Replay: an event log containing pre-S169 `RepairApplied` events deserializes
  with `provider_kind = AskWitness` defaults; replay is identical.

## Risks

1. **Provider fan-out.** Three providers per repair attempt could increase
   per-tick repair cost. Mitigation: providers short-circuit on
   `BreachClassMismatch` before any expensive work; the
   `repair_budget` cap is unchanged.
2. **Payload synthesis drift.** The repair-side synthesized payload could
   diverge from the affordance-derived payload over time. Mitigation: D7's
   override validators are registered on the same handlers, so any drift
   surfaces as revalidation failure rather than silent acceptance.
3. **Inadvertent goal-companion regression.** This spec deliberately does not
   change `information_barrier_companion_entry`. A future spec broadening
   goal companions must coordinate the breach-classification logic between
   the two seams; the current spec keeps them independent by reusing only
   the provider type, not the registry's seam-side caller.
4. **Trace shape coupling.** Adding `verification_provider` and
   `verification_rejections` fields to `RepairAttemptTrace` could
   over-specify trace shape for golden assertions. Mitigation: assertions
   key on `VerificationProviderKind` and `VerificationRejection` enum
   tags, not on full trace structural equality.

## Outcome

(Filled in upon completion.)
