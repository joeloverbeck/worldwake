# S170: Learned-State Provenance Hardening

**Status**: DRAFT

## Problem Statement

Three learned-state surfaces record updates without sufficient causal
provenance, leaving FND-22A and FND-29A under-served and giving the
third-iteration AI architecture report
(`reports/ai-architecture-improvements-third-iteration.md` Proposal 3 and
Item E) a concrete and verified complaint:

1. **`LearnedOpportunityMemory::OpportunityEntry`**
   (`crates/worldwake-core/src/learned_opportunity_memory.rs:5-11`) records
   `opportunity`, `observed_tick`, `expires_tick`, and `observed_at`. It
   stores **no event reference**, so an audit cannot answer "which event
   produced this learning update?" — only "the agent learned of opportunity
   X around tick Y at place Z." That fails the FND-29A queryable-causal-
   history test for this component class.
2. **`RoutePreferenceEntry`**
   (`crates/worldwake-core/src/route_preference.rs:14-21`) carries
   `last_traversal_event: Option<EventId>` and writes it in `record_dangerous`
   (`route_preference.rs:91-95`) but **not in `record_safe`**
   (`route_preference.rs:85-88`). Safe-traversal updates lose event
   provenance asymmetrically. FND-22A's accountable-origin requirement is
   partial.
3. **`apply_pending_discrepancies`**
   (`crates/worldwake-ai/src/agent_tick/observation.rs:416-434`) hardcodes
   `source_event: None` when recording every `DiscrepancyEntry`. A
   discrepancy can legitimately lack a single source event (a read-phase
   inference is not a discrete world event), but conflating "the source
   event was not recorded" with "there is no source event" loses
   information. FND-29 wants the distinction explicit.

`TestimonyReliability`
(`crates/worldwake-core/src/testimony_reliability.rs:20-62`) is *not* in this
spec's scope — it already keeps a ring buffer of provenance events with
capacity 8 and a `push_provenance` enforcer. It is the model the other three
surfaces should approximate.

**Critical scoping decision.** The third-iteration report proposes a unified
`LearnedStateUpdate { subject_key, source_scope, update_kind, observed_tick,
source_event_or_reason, decay_or_expiry, overwrite_policy,
decision_effect_trace }` contract across all three stores. This spec **does
not** introduce that abstraction. The report itself flags the risk of
"abstract learning sludge" (Proposal 3 Risks); a unified contract trait would
force `RouteSafety`, `OpportunityKey`, and `Discrepancy` into a single
narrative they do not share. The narrow fix is to close the three confirmed
provenance gaps with the minimal additive change to each concrete type. No
new trait, no new struct, no behavior change.

**Evidence sources.** `reports/ai-architecture-improvements-third-iteration.md`
Proposal 3 (rank 3, "Adopt in modified form") and Item E (pending
discrepancy provenance); `docs/triage/2026-05-25-ai-architecture-
improvements-third-iteration-triage.md`; current-code citations above.

## Phase and Status

Adjunct Wave: AI Architecture Improvements — Second Iteration. Independent of
S169 (Generalized Lawful Verification Substrate); may land in either order.
Builds on completed S109 (typed discrepancy taxonomy) and S151 (testimony
reliability + route preferences).

## Crates

- `worldwake-core`
  - `src/learned_opportunity_memory.rs` — extend `OpportunityEntry` with
    `source_event: EventId`. Update `record(...)` signature; update all call
    sites.
  - `src/route_preference.rs` — `record_safe` accepts and stores an
    `EventId`. Existing `last_traversal_event` field already exists; the
    asymmetry collapses.
  - `src/discrepancy.rs` (or wherever `DiscrepancyEntry` lives) — replace
    `source_event: Option<EventId>` with an enum:
    `pub enum DiscrepancySource { Event(EventId), ReadPhaseInference }`.
    Migrate the field name to `source` and the type accordingly. Old
    `source_event: None` becomes `source: ReadPhaseInference` with
    explicit semantics; old `source_event: Some(id)` becomes
    `source: Event(id)`.
- `worldwake-ai`
  - `src/agent_tick/observation.rs` (line 416-434) — replace the hardcoded
    `source_event: None` with the explicit `DiscrepancySource::
    ReadPhaseInference` enum value, since read-phase discrepancies genuinely
    have no single producing event.
  - Any other call site that records a `DiscrepancyEntry` — verify each
    passes a real event id or `ReadPhaseInference` deliberately.
  - Any call site that updates `LearnedOpportunityMemory` — pass the
    triggering event id.
  - Any call site that calls `RoutePreference::record_safe` — pass the
    triggering event id.
- No `worldwake-systems` changes.

## Dependencies

- **Completed**: S109 (typed discrepancy taxonomy), S151 (testimony
  reliability + route preferences).
- **No new dependencies** on S60–S66.
- **Does not depend on S169**; both specs may land in parallel.

## Design Goals

1. **Concrete event provenance where an event exists.** Every learning
   update that *has* a producing event records it. No silent `None`s.
2. **Explicit "no-event" sentinel where appropriate.** When a learning
   update genuinely lacks a single producing event (read-phase inference),
   say so explicitly. Conflating "no event recorded" with "no event possible"
   is the bug being fixed.
3. **Minimal additive change.** Three field additions / one enum migration.
   No new traits, no new generic abstractions, no behavior change.
4. **Save/load equivalence.** Existing saves continue to load; new saves
   round-trip the new fields exactly.

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

## FOUNDATIONS Alignment

| Principle | Application |
|---|---|
| FND-3 (concrete state) | New fields are concrete `EventId` references, not derived summaries. |
| FND-22A (learning is concrete state) | Every learned update has accountable origin: either an event id or an explicit `ReadPhaseInference` sentinel. |
| FND-26 (state-mediated systems) | No new cross-system call paths. Existing event-id provenance threads via state, not direct calls. |
| FND-28 (no fossils) | `source_event: None` is replaced; no shim left behind. |
| FND-29 (debuggability) | "Why did this agent prefer this route?" answerable via `last_traversal_event` for both safe and dangerous traversals. |
| FND-29A (causal history) | Append-only history retains the event reference; audits can chain back through it. |

## Deliverables

### D1. `LearnedOpportunityMemory::OpportunityEntry.source_event`

Add field `source_event: EventId` to `OpportunityEntry`. Update the public
recording method to require an `EventId` argument. Update each call site to
pass the triggering event (the perception or observation event that produced
the learning). Update all tests.

If a call site cannot identify a single triggering event, the call site is
the bug; reassess the call site rather than introducing an `Option` here.
Learned-opportunity updates always arise from a concrete observation event.

### D2. `RoutePreference::record_safe` event provenance

`record_safe` (`route_preference.rs:85-88`) accepts and stores an `EventId`
into the existing `last_traversal_event` field. The asymmetry with
`record_dangerous` collapses. Update call sites to pass the
travel-completion event id.

### D3. `DiscrepancySource` enum

Replace `source_event: Option<EventId>` on `DiscrepancyEntry` with:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DiscrepancySource {
    Event(EventId),
    ReadPhaseInference,
}
```

Field rename: `source_event` → `source`. Update all call sites:
- `agent_tick/observation.rs:416-434` (`apply_pending_discrepancies`) — set
  `source: DiscrepancySource::ReadPhaseInference` (explicit, since read-phase
  inference genuinely lacks a single triggering event).
- Any other call site (audit during implementation) — pass a real
  `Event(event_id)` where one exists.

### D4. Call-site audit

Grep `record_safe`, `LearnedOpportunityMemory::record` (or equivalent
public update method), and every `DiscrepancyEntry` construction. Each
site must supply a real event id or, for `DiscrepancySource`, deliberately
choose `ReadPhaseInference` with a one-line comment justifying the absence.

### D5. Save/load migration

The new `OpportunityEntry::source_event` and
`RoutePreferenceEntry`'s new field-population path require a save/load
strategy for pre-S170 saves. Two options:

1. (**Preferred**) Hard cut. Pre-S170 saves are not supported; project's
   FND-28 "no backward compatibility in live authority paths" mandate
   applies. Existing save format is replaced.
2. (Fallback) `serde(default)` on the new field, deserializing pre-S170
   saves with a placeholder `EventId(0)` or similar sentinel. This violates
   the spirit of "no fossils" and is rejected unless implementation
   discovers active dependency on pre-S170 save artifacts (none expected).

Per FND-28, Option 1 is the spec's selected approach. Replay determinism
preserved: any new save round-trips its new fields exactly.

### D6. Tests

- Unit: `LearnedOpportunityMemory::record` accepts and stores the event id;
  `RoutePreference::record_safe` accepts and stores the event id;
  `DiscrepancyEntry` round-trips both `Event(...)` and `ReadPhaseInference`.
- Save/load: round-trip of each component with the new fields populated.
- Golden: at least one existing golden touching each surface
  (learned-opportunity, route-safe, discrepancy) is updated to assert the
  presence of the event id (or the explicit `ReadPhaseInference` sentinel
  for discrepancies). No new dedicated goldens are required — the
  assertion is additive to existing coverage.

## FND-01 Section H

### Information-path analysis

Each new field carries a reference to an already-existing append-only event.
The path from the source event to the learning update is:
- Authoritative action handler emits a perception/travel-completion/
  observation event.
- The event id is the handler's return value or the system tick's local
  state at the moment of belief update.
- The agent's tick logic (`agent_tick/*`) reads the event id at the same
  call site where it currently invokes `record_safe` / `record(...)` /
  constructs `DiscrepancyEntry`.

No new information path is created. The fields make an existing local
piece of information durable in component state.

### Positive-feedback analysis

None. The new fields are passive metadata; they do not feed back into agent
decision logic in this spec.

### Concrete dampeners

Not applicable — no positive feedback loop introduced.

### Stored state vs derived read-model list

**Authoritative stored state** introduced:
- `LearnedOpportunityMemory::OpportunityEntry.source_event: EventId`
- `RoutePreferenceEntry`'s safe-traversal event id (writes the existing
  `last_traversal_event` field on the safe branch — the field is already
  authoritative).
- `DiscrepancyEntry.source: DiscrepancySource` (renamed; type changed from
  `Option<EventId>` to a typed enum).

**Derived**: none introduced.

### Planner-formalism analysis

Not applicable — no planner surface changes. Learned state remains an input
to existing ranking and candidate-generation discounts.

### Causal-equivalence contract

Save/load behavior changes per D5 (Option 1, hard cut). Replay
equivalence: any save produced by the post-S170 code round-trips
deterministically. Pre-S170 saves are not supported — consistent with FND-
28's standing prohibition on backward-compat live authority paths.

### Systemic-validation analysis (FND-31)

Negative cases:
- A `LearnedOpportunityMemory` write at a call site that does not pass an
  event id (compile error — required positional argument).
- A `RoutePreference::record_safe` write that does not store an event id
  (signature enforces).
- A `DiscrepancyEntry` constructed with the legacy `source_event: None`
  pattern (type system enforces the explicit enum variant).
- A save/load round-trip that drops or rewrites any new field.

Required tests per D6.

## SystemFn Integration

No new `SystemFn` registrations. The new fields are populated at existing
call sites inside the agent tick (`worldwake-ai`) and any other systems that
touch these stores. No system signature changes.

## Component Registration

The three modified components (`LearnedOpportunityMemory`, `RoutePreference`,
`DiscrepancyMemory`) are already registered. No new `EntityKind::Agent`
profile fields. Agent Profile Scenario Contract does not apply.

## Cross-System Interactions (FND-26)

No new cross-system calls. `worldwake-systems` is unaware of this spec.
`worldwake-ai` updates its existing call sites to supply event ids that
already flow through local state at those sites.

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

Negative cases per Section H. Required tests per D6.

Additional invariants the spec must preserve:
- No ranking-discount value (testimony reliability discount, learned-
  opportunity discount, route-preference discount) changes as a function of
  the new event-id field.
- The set of learned-opportunity entries an agent holds at tick T is
  unchanged (same insertion/eviction/expiry semantics).
- The set of route-preference entries an agent holds at tick T is
  unchanged.
- The set of discrepancy entries an agent holds at tick T is unchanged.

These invariants pin down that S170 is a pure-provenance refactor: nothing
the agent *does* changes, only what an auditor can *ask* about the agent's
state.

## Risks

1. **Call-site audit miss.** Missing a `record(...)` call site that should
   pass an event id leaves the field unpopulated. Mitigation: signature
   change forces compile error at every site; no `Option` escape hatch.
2. **`DiscrepancySource` enum churn.** Renaming `source_event` → `source`
   and changing the type touches many sites. Mitigation: type changes
   propagate through the compiler; no silent drift possible.
3. **Pre-S170 save support.** Per D5/FND-28 standing decision, hard cut.
   No fallback shim. Risk: in-flight saves are abandoned. Mitigation:
   project does not currently rely on long-lived save artifacts;
   user-confirm at implementation time if any such artifact exists.
4. **Test-side noise.** Many existing tests construct
   `LearnedOpportunityMemory` / `RoutePreference` / `DiscrepancyEntry`
   inline. Each constructor call must be updated. This is mechanical
   compiler-driven work, low risk.

## Outcome

(Filled in upon completion.)
