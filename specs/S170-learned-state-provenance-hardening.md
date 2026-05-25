# S170: Learned-State Provenance Hardening

**Status**: DRAFT

## Problem Statement

Four typed-memory surfaces record updates without sufficient causal
provenance, leaving FND-22A and FND-29A under-served and giving the
third-iteration AI architecture report
(`reports/ai-architecture-improvements-third-iteration.md` Proposal 3 and
Item E) a concrete and verified complaint:

1. **`LearnedOpportunityMemory::OpportunityEntry`**
   (`crates/worldwake-core/src/learned_opportunity_memory.rs:5-11`) records
   `opportunity`, `observed_tick`, `expires_tick`, and `observed_at`. It
   stores **no causal source**, so an audit cannot answer "what produced
   this learning update?" — only "the agent learned of opportunity X
   around tick Y at place Z." That fails the FND-29A queryable-causal-
   history test for this component class. The only runtime call site
   (`record_learned_opportunities_from_read_phase` in
   `crates/worldwake-ai/src/agent_tick/mod.rs:2558-2589`) is a read-phase
   inference over the agent's belief state and does not have a single
   triggering event id in scope — the surface needs an explicit sentinel
   for "this learning is a read-phase inference," not a fabricated event
   reference.
2. **`RoutePreferenceEntry`**
   (`crates/worldwake-core/src/route_preference.rs:14-21`) carries
   `last_traversal_event: Option<EventId>` and writes it in `record_dangerous`
   (`route_preference.rs:91-96`) but **not in `record_safe`**
   (`route_preference.rs:85-89`). Safe-traversal updates lose event
   provenance asymmetrically. FND-22A's accountable-origin requirement is
   partial. The runtime safe-traversal call site
   (`crates/worldwake-ai/src/agent_tick/learned_state_observation.rs:215-219`)
   already has an authoritative event id in scope: the `provenance_event`
   on `RoutePreferenceObservation`, which is the event whose
   `RouteExperience` component-set delta triggered the observation
   (`learned_state_observation.rs:42-60`). No new event flow is needed —
   the safe-side call site must pass the same id the dangerous-side fallback
   already uses.
3. **`apply_pending_discrepancies`**
   (`crates/worldwake-ai/src/agent_tick/observation.rs:416-434`) hardcodes
   `source_event: None` when recording every `DiscrepancyEntry`. A
   discrepancy can legitimately lack a single source event (a read-phase
   inference is not a discrete world event — `PendingDiscrepancyRecord`
   (`crates/worldwake-ai/src/candidate_generation.rs:253-258`) carries
   scope/discrepancy/observed_tick/clearing_condition but no event id),
   but conflating "the source event was not recorded" with "there is no
   source event" loses information. FND-29 wants the distinction explicit.
4. **`Blocker`**
   (`crates/worldwake-core/src/blocker_memory.rs:212-221`) carries the same
   `source_event: Option<EventId>` field as `DiscrepancyEntry`. Over 30
   runtime construction sites across `candidate_generation.rs`,
   `failure_handling.rs`, `agenda_manager.rs`, `plan_repair.rs`,
   `feasibility_probe.rs`, and `observation.rs:653` currently write
   `source_event: None`. The same FND-22A/FND-29A "conflates no-event-
   recorded with no-event-possible" criticism applies symmetrically. The
   third-iteration report named the three "learned" surfaces explicitly and
   silently omitted Blocker; reassessment surfaces this as a symmetric gap.
   Most Blocker constructions are planning-time inferences (the agent
   inferred a blocker from belief state during candidate generation), not
   responses to discrete world events, so Blocker needs the same explicit
   sentinel discipline as DiscrepancyEntry.

`TestimonyReliability`
(`crates/worldwake-core/src/testimony_reliability.rs:20-62`) is *not* in this
spec's scope — it already keeps a ring buffer of provenance events with
capacity 8 and a `push_provenance` enforcer. It is the model the other
four surfaces should approximate.

**Critical scoping decision.** The third-iteration report proposes a unified
`LearnedStateUpdate { subject_key, source_scope, update_kind, observed_tick,
source_event_or_reason, decay_or_expiry, overwrite_policy,
decision_effect_trace }` contract across the named stores. This spec **does
not** introduce that abstraction. The report itself flags the risk of
"abstract learning sludge" (Proposal 3 Risks); a unified contract trait would
force `RouteSafety`, `OpportunityKey`, `Discrepancy`, and `BlockingFact` into
a single narrative they do not share. The narrow fix is to close the four
confirmed provenance gaps with the minimal additive change to each concrete
type. Each surface gets its own domain-specific source enum with appropriate
sentinel variant — no shared `ProvenanceSource` abstraction, per FND-3
(concrete state over abstract scores) and the report's own anti-sludge
warning.

**Evidence sources.** `reports/ai-architecture-improvements-third-iteration.md`
Proposal 3 (rank 3, "Adopt in modified form") and Item E (pending
discrepancy provenance); `docs/triage/2026-05-25-ai-architecture-
improvements-third-iteration-triage.md`; current-code citations above.

## Phase and Status

Adjunct Wave: AI Architecture Improvements — Second Iteration. Independent of
S169 (Generalized Lawful Verification Substrate, now archived); may land in
any order. Builds on completed S109 (typed discrepancy taxonomy) and S151
(testimony reliability + route preferences).

## Crates

- `worldwake-core`
  - `src/learned_opportunity_memory.rs` — replace `OpportunityEntry` with
    `source: LearnedOpportunitySource` field. Define
    `LearnedOpportunitySource = Event(EventId) | ReadPhaseInference` in the
    same module. Update all `OpportunityEntry { … }` construction sites
    (compiler-driven).
  - `src/route_preference.rs` — `record_safe` accepts and stores an
    `EventId`. The existing `last_traversal_event` field is already
    authoritative; the asymmetry between `record_safe` and `record_dangerous`
    collapses.
  - `src/discrepancy.rs` — define
    `DiscrepancySource = Event(EventId) | ReadPhaseInference` enum.
    Replace `DiscrepancyEntry::source_event: Option<EventId>` with
    `DiscrepancyEntry::source: DiscrepancySource`. The rename is scoped to
    `DiscrepancyEntry` only — `Blocker::source_event` at
    `crates/worldwake-core/src/blocker_memory.rs:220` is a separate field
    on a separate type and is migrated by D4 (not by a workspace-wide
    grep-and-replace on the shared identifier).
  - `src/blocker_memory.rs` — define
    `BlockerSource = Event(EventId) | Inferred` enum. Replace
    `Blocker::source_event: Option<EventId>` with `Blocker::source:
    BlockerSource`. `Blocker.source = BlockerSource::Inferred` covers
    planning-time inferences (the agent inferred a blocker during
    candidate generation from belief state) where no triggering event id
    exists. Sites that have a real event id (e.g., a refused trade event)
    write `BlockerSource::Event(id)`.
- `worldwake-ai`
  - `src/agent_tick/observation.rs` (line 416-434) — replace the hardcoded
    `source_event: None` in `apply_pending_discrepancies` with the explicit
    `DiscrepancySource::ReadPhaseInference` enum value, since read-phase
    discrepancies genuinely have no single producing event.
  - `src/agent_tick/observation.rs` (line 653) — `Blocker` construction in
    `apply_pending_facility_intents` writes `BlockerSource::Inferred` (the
    blocker is inferred from observation, no discrete event).
  - `src/agent_tick/learned_state_observation.rs` (line 218) —
    `record_route_preference_updates` passes `observation.provenance_event`
    as the EventId to `record_safe`. This is the same id the dangerous-side
    fallback (line 226) already uses; the asymmetry collapses without a new
    event-flow surface.
  - `src/agent_tick/mod.rs` (line 2582) —
    `record_learned_opportunities_from_read_phase` constructs
    `OpportunityEntry { source: LearnedOpportunitySource::ReadPhaseInference,
    … }`. Read-phase opportunity learning is synthesis from belief state, not
    response to a discrete event.
  - Runtime DiscrepancyEntry construction sites (per D5 enumeration):
    `agent_tick/observation.rs:422`, `agent_tick/frame.rs:733/871/894`,
    `agent_tick/planning.rs:1666`, `agent_tick/execution.rs:612`,
    `failure_handling.rs:267`. Each writes
    `source: DiscrepancySource::Event(id)` where a triggering event id is in
    scope, or `source: DiscrepancySource::ReadPhaseInference` with a
    one-line rationale comment where it is not.
  - Runtime Blocker construction sites (per D5 enumeration) across
    `candidate_generation.rs` (~18 sites in extractor and candidate
    emission), `failure_handling.rs` (~15 sites), `agenda_manager.rs`,
    `plan_repair.rs`, `feasibility_probe.rs`. Each writes
    `source: BlockerSource::Event(id)` where a triggering event id is in
    scope, or `source: BlockerSource::Inferred` with a one-line rationale.
- No `worldwake-systems` changes.

## Dependencies

- **Completed**: S109 (typed discrepancy taxonomy), S151 (testimony
  reliability + route preferences).
- **No new dependencies** on S60–S66.
- **Does not depend on S169** (completed and archived 2026-05-25); both
  specs were independent.

## Design Goals

1. **Concrete causal source where an event exists.** Every learning
   update that *has* a producing event records it. No silent `None`s and
   no fabricated event references.
2. **Explicit "no-event" sentinel where appropriate.** When a learning
   update genuinely lacks a single producing event (read-phase inference
   for opportunities and discrepancies; planning-time inference for
   blockers), say so explicitly. Conflating "no event recorded" with "no
   event possible" is the bug being fixed.
3. **Domain-specific sentinel naming over a unified abstraction.** Three
   distinct source enums (`LearnedOpportunitySource`, `DiscrepancySource`,
   `BlockerSource`) instead of one shared `ProvenanceSource`. Each enum's
   sentinel variant carries the right domain semantics:
   `ReadPhaseInference` for read-phase synthesis, `Inferred` for
   planning-time blocker inferences. Per FND-3 and the third-iteration
   report's own anti-sludge warning.
4. **Minimal additive change.** Three new enums, one field-type change on
   each of the four touched components, one new field-population on the
   safe-traversal call site. No new traits, no new generic abstractions,
   no behavior change.
5. **Save/load equivalence.** Existing post-S170 saves round-trip the new
   fields exactly. Pre-S170 saves are not supported (FND-28).

## Non-Goals

1. **Unified `LearnedStateUpdate` trait/struct.** Explicitly dropped per the
   report's own warning against abstract learning sludge.
2. **Decision-effect trace coupling.** The report proposes that every
   learned update carry a "decision_effect_trace" pointer. This requires
   wiring through `DecisionTrace` per update site and is genuine scope
   expansion. Out of scope for S170; track as a future audit.
3. **Decay/expiry policy changes.** Existing policies remain.
4. **`TestimonyReliability` changes.** Already provenance-rich.
5. **New observer telemetry surfaces.** The new fields are accessible
   through the existing serialization surfaces; observer enrichment is a
   separate concern.
6. **`PartialPlan::source_event`** (`crates/worldwake-ai/src/partial_plan.rs:227`).
   Already a required non-Option `EventId`; no provenance gap to close.
   Mentioned here only to make the rename's scope unambiguous: D3 and D4
   touch `DiscrepancyEntry::source_event` and `Blocker::source_event`
   respectively; the same identifier on `PartialPlan` is unrelated.

## FOUNDATIONS Alignment

| Principle | Application |
|---|---|
| FND-3 (concrete state) | Three domain-specific source enums (one per surface) instead of one abstract shared type. Each sentinel variant names the actual mechanism (`ReadPhaseInference`, `Inferred`), not an opaque score. |
| FND-22A (learning is concrete state) | Every learned update has accountable origin: either an event id or an explicit domain-appropriate sentinel. No silent `None`. |
| FND-26 (state-mediated systems) | No new cross-system call paths. Existing event-id provenance threads via state, not direct calls. |
| FND-28 (no fossils) | Old `source_event: Option<EventId>` on `DiscrepancyEntry` and `Blocker` is replaced wholesale; no shim, no `serde(default)`, no `serde(alias)`. Pre-S170 saves are not supported per the standing prohibition on backward-compat in live authority paths. |
| FND-29 (debuggability) | "Why did this agent prefer this route?" answerable via `last_traversal_event` for both safe and dangerous traversals. "Why did this agent suppress this attempt?" answerable via `DiscrepancySource::Event(...)` or the explicit `ReadPhaseInference` sentinel. "Why was this candidate blocked?" answerable via `BlockerSource::Event(...)` or the explicit `Inferred` sentinel. |
| FND-29A (causal history) | Append-only history retains the event reference where one exists, and the explicit sentinel where one does not; audits can chain back through it without ambiguity. |

## Deliverables

### D1. `LearnedOpportunitySource` enum and `OpportunityEntry::source`

Define in `crates/worldwake-core/src/learned_opportunity_memory.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LearnedOpportunitySource {
    /// The learning update is attributable to a specific world event
    /// (perception, travel completion, transaction, etc.).
    Event(EventId),
    /// The learning update emerged from the agent's read-phase
    /// candidate-generation pass over its current belief state; no
    /// single discrete event produced it.
    ReadPhaseInference,
}
```

Replace `OpportunityEntry::observed_at: EntityId` and the absence of an
event field with a new field:

```rust
pub struct OpportunityEntry {
    pub opportunity: OpportunityKey,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub observed_at: EntityId,
    pub source: LearnedOpportunitySource,
}
```

(`observed_at` stays — it answers "where" the learning happened; `source`
answers "what" produced it.)

The only runtime call site,
`record_learned_opportunities_from_read_phase`
(`crates/worldwake-ai/src/agent_tick/mod.rs:2582`), writes
`source: LearnedOpportunitySource::ReadPhaseInference`. This is the
FND-3/FND-29A-honest attribution: read-phase candidate generation
synthesizes opportunities from the agent's belief state, not from a
discrete world event. The belief state itself carries perception
provenance separately, so the causal chain remains traceable through
the belief store without fabricating a per-opportunity event id.

If future call sites are added that *do* have a triggering event id in
scope (e.g., a future event-driven opportunity-learning path), they
write `LearnedOpportunitySource::Event(id)`.

### D2. `RoutePreference::record_safe` event provenance

`record_safe` (`route_preference.rs:85-89`) accepts and stores an `EventId`
into the existing `last_traversal_event` field. The asymmetry with
`record_dangerous` collapses.

The single runtime call site
(`crates/worldwake-ai/src/agent_tick/learned_state_observation.rs:215-219`)
passes `observation.provenance_event` — the event id of the record whose
`RouteExperience` component-set delta triggered the route-preference
observation (`learned_state_observation.rs:42-60`). This is the authentic
causal trigger; `provenance_event` is the same id the dangerous-side
fallback (line 226) already uses. No new event-search helper is needed
(there is no "safe traversal" event tag to search for parallel to
`Combat`/`Escalation`/`WildernessRelief`); the per-tick state-delta
event IS the safe-traversal causal record.

### D3. `DiscrepancySource` enum and `DiscrepancyEntry::source`

Define in `crates/worldwake-core/src/discrepancy.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiscrepancySource {
    /// The discrepancy was triggered by a specific world event (a
    /// perception that contradicted prior belief, a travel completion
    /// that invalidated a route assumption, etc.).
    Event(EventId),
    /// The discrepancy emerged from a read-phase inference over the
    /// agent's belief state during candidate generation (e.g., a
    /// `PendingDiscrepancyRecord` produced by an extractor). No
    /// single triggering event exists.
    ReadPhaseInference,
}
```

Replace `DiscrepancyEntry::source_event: Option<EventId>` with
`DiscrepancyEntry::source: DiscrepancySource`. The rename is scoped to
`DiscrepancyEntry` only.

The runtime call site
(`crates/worldwake-ai/src/agent_tick/observation.rs:416-434`,
`apply_pending_discrepancies`) writes
`source: DiscrepancySource::ReadPhaseInference` (explicit, since
`PendingDiscrepancyRecord` (candidate_generation.rs:253-258) carries no
event id and the discrepancy is synthesized from extractor logic over
belief state).

Other runtime DiscrepancyEntry construction sites (enumerated in D5)
each write `source: DiscrepancySource::Event(id)` where a triggering
event id is in scope, or `source: DiscrepancySource::ReadPhaseInference`
with a one-line rationale comment where it is not.

### D4. `BlockerSource` enum and `Blocker::source`

Define in `crates/worldwake-core/src/blocker_memory.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BlockerSource {
    /// The blocker was recorded in response to a specific world event
    /// (a refused trade, a denied access attempt, a contested
    /// reservation that resolved against this agent, etc.).
    Event(EventId),
    /// The blocker is a planning-time inference from the agent's
    /// belief state: candidate generation evaluated a candidate and
    /// determined the blocking fact applies without a discrete
    /// triggering event (e.g., `NoKnownSeller` inferred from absence
    /// of belief-store entries).
    Inferred,
}
```

Replace `Blocker::source_event: Option<EventId>` with `Blocker::source:
BlockerSource`. Sentinel variant name (`Inferred`) deliberately differs
from `DiscrepancySource::ReadPhaseInference` because Blocker inferences
happen during planning generally, not specifically during read-phase
synthesis — the variant name encodes the right domain semantic per
FND-3.

Runtime Blocker construction sites span `candidate_generation.rs`
(~18 sites in extractor and candidate emission paths),
`failure_handling.rs` (~15 sites), `agenda_manager.rs`,
`plan_repair.rs`, `feasibility_probe.rs`,
`agent_tick/observation.rs:653`. Most are currently `source_event: None`
and become `source: BlockerSource::Inferred` (the planning-time
inference case). Sites with a concrete triggering event (e.g., a
contention/escalation event that produced the blocker) become
`source: BlockerSource::Event(id)`.

### D5. Runtime call-site audit

The implementer must visit every runtime construction site for the four
touched types and choose the appropriate source variant. Test-only
construction sites are mechanical compiler-driven updates (the type
change forces a fix); runtime sites require attribution thinking. The
runtime enumeration as of 2026-05-25:

**Runtime `OpportunityEntry { … }` construction:**

- `crates/worldwake-ai/src/agent_tick/mod.rs:2582`
  (`record_learned_opportunities_from_read_phase`) →
  `LearnedOpportunitySource::ReadPhaseInference`

**Runtime `RoutePreference::record_safe(…)` invocation:**

- `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs:218`
  → pass `observation.provenance_event`

**Runtime `DiscrepancyEntry { … }` construction:**

- `crates/worldwake-ai/src/agent_tick/observation.rs:422`
  (`apply_pending_discrepancies`) → `DiscrepancySource::ReadPhaseInference`
- `crates/worldwake-ai/src/agent_tick/frame.rs:733, 871, 894` →
  audit each: most are response-to-event sites and become
  `DiscrepancySource::Event(...)`
- `crates/worldwake-ai/src/agent_tick/planning.rs:1666` → audit
- `crates/worldwake-ai/src/agent_tick/execution.rs:612`
  (`discrepancy_entry_for_repair`) → audit; if no event in scope,
  `ReadPhaseInference` with one-line rationale
- `crates/worldwake-ai/src/failure_handling.rs:267` → audit

**Runtime `Blocker { … }` construction:**

- Approximately 30+ sites across `crates/worldwake-ai/src/candidate_generation.rs`
  (~18 sites in extractor/candidate-emission paths),
  `crates/worldwake-ai/src/failure_handling.rs` (~15 sites at lines
  260, 277, 2957–4000s),
  `crates/worldwake-ai/src/agenda_manager.rs:2750`,
  `crates/worldwake-ai/src/plan_repair.rs:455`,
  `crates/worldwake-ai/src/feasibility_probe.rs:772, 822`,
  `crates/worldwake-ai/src/agent_tick/observation.rs:653`. Most are
  planning-time inferences (currently `source_event: None`) and become
  `BlockerSource::Inferred`. Sites that already have a real `EventId`
  in scope (e.g., a `ReservationConflict` blocker derived from a
  contention event) become `BlockerSource::Event(id)`.

Each site must supply a real event id or deliberately choose the
sentinel variant (`ReadPhaseInference` for discrepancies/opportunities,
`Inferred` for blockers) with a one-line comment justifying the
absence. The exact runtime count may drift; the implementer uses the
compiler errors from the type change as the audit driver, and the list
above as the seed inventory.

### D6. Save/load migration

Per FND-28's prohibition on backward-compat in live authority paths,
the new fields are required. Pre-S170 saves are not supported. No
`serde(default)`, no `serde(alias)`, no shim. Existing save format is
replaced. Post-S170 saves round-trip all four new fields exactly;
replay determinism is preserved for any save produced by the post-S170
code.

### D7. Tests

- **Unit**: `LearnedOpportunityMemory::record` accepts and stores both
  `LearnedOpportunitySource::Event(...)` and
  `LearnedOpportunitySource::ReadPhaseInference`;
  `RoutePreference::record_safe` accepts and stores the event id;
  `DiscrepancyEntry` round-trips both `DiscrepancySource::Event(...)`
  and `DiscrepancySource::ReadPhaseInference`; `Blocker` round-trips both
  `BlockerSource::Event(...)` and `BlockerSource::Inferred`.
- **Save/load**: round-trip of each component with the new fields
  populated by both variants.
- **Golden**: at least one existing golden touching each surface
  (learned-opportunity, route-safe, discrepancy, blocker) is updated to
  assert the presence of the event id or the explicit sentinel variant.
  No new dedicated goldens are required — the assertion is additive to
  existing coverage.

## FND-01 Section H

### Information-path analysis

Each new field carries a reference to an already-existing append-only
event, or an explicit domain-appropriate sentinel that says "no single
event produced this." The path from the source event to the learning
update is:

- Authoritative action handler emits a perception/travel-completion/
  observation event.
- The event id is the handler's return value or the system tick's local
  state at the moment of belief update.
- The agent's tick logic (`agent_tick/*`) reads the event id at the same
  call site where it currently invokes `record_safe` / `record(...)` /
  constructs `DiscrepancyEntry` / constructs `Blocker`.
- When no single triggering event exists (read-phase opportunity
  learning, read-phase discrepancy inference, planning-time blocker
  inference), the sentinel variant is written instead. The agent's
  belief state — which the inference reads — carries its own perception
  provenance, so the causal chain remains traceable without a fabricated
  per-update event id.

No new information path is created. The fields make an existing local
piece of information durable in component state, or explicitly record
its absence with the right semantic label.

### Positive-feedback analysis

None. The new fields are passive metadata; they do not feed back into
agent decision logic in this spec.

### Concrete dampeners

Not applicable — no positive feedback loop introduced.

### Stored state vs derived read-model list

**Authoritative stored state** introduced:

- `LearnedOpportunityMemory::OpportunityEntry.source: LearnedOpportunitySource`
  (new field; the type is a new enum defined in the same module).
- `RoutePreferenceEntry`'s safe-traversal event id (writes the existing
  `last_traversal_event` field on the safe branch — the field is already
  authoritative).
- `DiscrepancyEntry.source: DiscrepancySource` (the field is renamed
  from `source_event` and its type changed from `Option<EventId>` to a
  typed enum; the old `source_event` field is removed wholesale per
  FND-28, not aliased).
- `Blocker.source: BlockerSource` (the field is renamed from
  `source_event` and its type changed from `Option<EventId>` to a typed
  enum; the old `source_event` field is removed wholesale per FND-28,
  not aliased).

**New types introduced**: `LearnedOpportunitySource`, `DiscrepancySource`,
`BlockerSource`. Each is a `Copy` two-variant enum with `Event(EventId)`
and a domain-appropriate sentinel variant
(`ReadPhaseInference`/`ReadPhaseInference`/`Inferred`). The three enums
deliberately do *not* share a common abstract supertype, per FND-3 and
the third-iteration report's anti-sludge warning.

**Derived**: none introduced.

### Planner-formalism analysis

Not applicable — no planner surface changes. Learned state remains an
input to existing ranking and candidate-generation discounts.

### Causal-equivalence contract

Save/load behavior changes per D6 (hard cut). Replay
equivalence: any save produced by the post-S170 code round-trips
deterministically. Pre-S170 saves are not supported — consistent with
FND-28's standing prohibition on backward-compat live authority paths.

### Systemic-validation analysis (FND-31)

Negative cases:

- A `LearnedOpportunityMemory` write at a call site that does not pass a
  `LearnedOpportunitySource` value (compile error — required positional
  field).
- A `RoutePreference::record_safe` write that does not store an event id
  (signature enforces).
- A `DiscrepancyEntry` constructed with the legacy `source_event: None`
  pattern (type system enforces the explicit enum variant).
- A `Blocker` constructed with the legacy `source_event: None` pattern
  (type system enforces the explicit enum variant).
- A save/load round-trip that drops or rewrites any new field.

Required tests per D7.

## SystemFn Integration

No new `SystemFn` registrations. The new fields are populated at existing
call sites inside the agent tick (`worldwake-ai`) and any other systems
that touch these stores. No system signature changes.

## Component Registration

The four modified components (`LearnedOpportunityMemory`, `RoutePreference`,
`DiscrepancyMemory`, `BlockerMemory`) are already registered. No new
`EntityKind::Agent` profile fields. Agent Profile Scenario Contract does
not apply.

## Cross-System Interactions (FND-26)

No new cross-system calls. `worldwake-systems` is unaware of this spec.
`worldwake-ai` updates its existing call sites to supply event ids that
already flow through local state at those sites, or to write the
domain-appropriate sentinel where no event id exists.

## Profile-Driven Parameters

No new parameters. No new numeric values introduced anywhere. Permille rule
is vacuously satisfied.

## Authoritative-to-AI Impact Analysis

This spec does not change authoritative validation, action preconditions,
`validate_*` functions, or `can_exercise_control`. The
Authoritative-to-AI Impact Rule's seven-step trace does not apply: the
agent decision cycle reads existing learned-state fields and is unchanged
by the addition of provenance metadata. Goldens covering ranking, candidate
generation, and search continue to pass without modification.

## Validation and Falsification (FND-31)

Negative cases per Section H. Required tests per D7.

Additional invariants the spec must preserve:

- No ranking-discount value (testimony reliability discount, learned-
  opportunity discount, route-preference discount) changes as a function of
  the new source field.
- The set of learned-opportunity entries an agent holds at tick T is
  unchanged (same insertion/eviction/expiry semantics).
- The set of route-preference entries an agent holds at tick T is
  unchanged.
- The set of discrepancy entries an agent holds at tick T is unchanged.
- The set of blocker entries an agent holds at tick T is unchanged.

These invariants pin down that S170 is a pure-provenance refactor: nothing
the agent *does* changes, only what an auditor can *ask* about the agent's
state.

## Risks

1. **Call-site audit miss.** Missing a runtime `record(...)` or
   construction call site that should pass an event id (and instead defaults
   to the sentinel) silently weakens provenance. Mitigation: signature
   change forces compile error at every site; there is no `Option` escape
   hatch. The audit's correctness condition is that every site visited
   has either a concrete event id or a one-line rationale for the
   sentinel; the compiler enforces the structural completeness.
2. **Same-named field on two types.** `DiscrepancyEntry::source_event` and
   `Blocker::source_event` share the identifier `source_event` but live on
   different types in different modules. A workspace-wide grep-and-replace
   on the bare identifier would conflate them. Mitigation: the migration
   is per-type, driven by the field-type change on each struct
   (DiscrepancyEntry in D3, Blocker in D4); the compiler validates each
   site's reference to the correct field independently. A third
   same-named field exists on `PartialPlan` at `partial_plan.rs:227`
   (`pub source_event: EventId`, non-Option, already required) — it is
   unaffected per Non-Goal #6.
3. **Sentinel variant naming divergence.** `DiscrepancySource` and
   `LearnedOpportunitySource` both name their sentinel `ReadPhaseInference`;
   `BlockerSource` names its sentinel `Inferred`. The divergence is
   deliberate (Blocker inferences happen during planning broadly, not
   specifically read-phase). Risk: reviewers may try to harmonize the
   names into a shared name or shared enum, reintroducing the "abstract
   learning sludge" the spec rejects. Mitigation: the FND-3 rationale is
   stated in Design Goals #3.
4. **Pre-S170 save support.** Per D6/FND-28 standing decision, hard cut.
   No fallback shim. Risk: in-flight saves are abandoned. Mitigation:
   project does not currently rely on long-lived save artifacts;
   user-confirm at implementation time if any such artifact exists.
5. **Test-side noise.** Many existing tests construct
   `LearnedOpportunityMemory` / `RoutePreference` / `DiscrepancyEntry` /
   `Blocker` inline. Each constructor call must be updated. This is
   mechanical compiler-driven work, low risk.

## Outcome

(Filled in upon completion.)
