# S142: Contention Event Inspectability

**Status**: Draft

## Summary

S127 (Quantity-Aware Acquisition) and the existing facility-queue substrate (`crates/worldwake-core/src/contention.rs`) carry contention *state*: `ContentionQueue` (per-affordance waiting list), `ContentionGrant` (active grant), `ContentionWaiter` (per-claimant entry), `ResourceExtractionQueues` (per-slot list). The state is inspectable post-hoc through ECS reads. What is *not* inspectable is the *resolution moment* — the tick at which two or more agents contended for the same scarce affordance, the rule that resolved it, and the loser set.

When Agent A and Agent B both reach a single-slot orchard at the same tick, today's resolution is silent: arrival-time ordering inside the queue (`ContentionWaiter.queued_at`) plus `BTreeMap`-stable iteration over waiters produces a deterministic winner. The winner's grant fires; the loser's request is queued. There is no inspectable record of "at tick 412, three agents contended for orchard slot 1; arrival-time rule selected Agent A; Agents B and C remained queued." Per FND-9 (scheduling and contention are part of the world model — tick order, container iteration order may not silently decide who saw the dropped coin first), and FND-29 (debuggability), the resolution must produce a queryable artifact, not just a state delta.

S142 adds `ContentionEvent` as a first-class append-only event payload emitted whenever a contention resolution fires. The two existing emission substrates today are the **facility-queue path** (`crates/worldwake-systems/src/facility_queue.rs::promote_ready_head`, which already emits `EventTag::QueueGrantPromoted` via the `commit_queue_update` `extra_tag` mechanism) and the **resource-extraction path** (`crates/worldwake-systems/src/production_actions.rs::grant_or_signal_full`, which sets `ContentionQueue.granted` for an extraction slot but does not emit any event today). S142 emits `ContentionResolved` at both paths with the contested affordance, the claimants in arrival order, the resolution rule that fired, and the winner/loser split. Observer adds a new section (Section 12, the next unused number) rendering contention events per tick. Goldens assert resolution attribution. The existing queue substrate is unchanged; S142 is purely an emission and inspection layer atop it.

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Crates

- `worldwake-core` — extends `EventTag` (`crates/worldwake-core/src/event_tag.rs:7`) with `ContentionResolved`. Adds `ContentionEventPayload` carrying the resolution record, an `EventPayload.contention_event_payload` carrier, and a `WorldTxn::set_contention_event_payload` writer. Adds `ContentionResolutionRule` enum with the single live variant `ArrivalTime`. Adds `AffordanceKey { facility: EntityId, action: ActionDefId }` struct identifying a contested affordance. Extends `BlockingFact::ReservationConflict` (`crates/worldwake-core/src/blocker_memory.rs:197`) from a unit variant to a struct variant carrying the contention-event reference.
- `worldwake-sim` — bumps `SAVE_FORMAT_VERSION` and extends save/load round-trip coverage for `ContentionResolved` events carrying `ContentionEventPayload`.
- `worldwake-systems` — `facility_queue.rs::promote_ready_head` extends its existing `commit_queue_update` emission so the successful grant event is tagged with both `QueueGrantPromoted` and `ContentionResolved` and carries the typed contention payload. `production_actions.rs::grant_or_signal_full` adds a new `ContentionResolved` emission at the slot-grant point (currently no event is written there).
- `worldwake-ai` — `agent_tick/execution.rs` populates the new `BlockingFact::ReservationConflict.contention_event` field when the conflict path observes a `ContentionResolved` event for the same affordance at the conflict tick. Existing call sites that construct or destructure `BlockingFact::ReservationConflict` are migrated for the new payload shape (17 sites workspace-wide).
- `worldwake-cli` — observer adds **Section 12 (Contention)** rendering per-tick contention events. Optional `--contention-top-n` flag surfaces top-N contentions per run by claimant count.

## Dependencies

- S127 (Quantity-Aware Acquisition) — completed. Provides `ResourceExtractionQueues` and the `grant_or_signal_full` slot-grant flow. S142 emits events from the existing grant path.
- S110 (Decision History Events) — completed. `ContentionResolved` joins the existing event-tag taxonomy.
- S109 (Typed Discrepancy Taxonomy) — completed. The split between `BlockingFact` (live blockers) and `Discrepancy` (typed discrepancies) is honored: S142 widens `BlockingFact::ReservationConflict` because that is the variant S127's reservation-conflict clearing path uses.
- S118 (Stuck-Agent Detector Active-Frame Exclusion) — completed. Structural prerequisite: S118's active-frame exclusion is the surrounding observer-precision discipline that S142's Section 12 inherits. S142 does not modify the stuck-agent detector itself.

## Design Goals

1. **Every resolution emits.** Any tick at which a `ContentionGrant` issues — facility-queue head promotion or resource-extraction slot grant — produces a `ContentionResolved` event. No silent winner.
2. **Typed resolution rule.** `ContentionResolutionRule` names the rule that fired. The single live variant is `ArrivalTime`; the enum stays open for extension when future contention substrates are added (per FND-28, those variants land with their substrate, not speculatively).
3. **Bounded event payload.** Up to 8 claimants per event recorded inline as `Vec<ContentionClaimant>` truncated at emit time; longer queues are summarized as `total_claimants: u16` plus the head-8.
4. **Per-claimant evidence is intrinsic.** Each `ContentionClaimant` carries its own `arrived_tick`. The `ArrivalTime` rule needs no separate evidence collection — the claimants' arrival ticks are the evidence.
5. **No new state.** S142 introduces no ECS component, no derived index, no cache. The events are append-only history.
6. **Determinism preserved.** Event emission order matches the existing grant-issuance order; tie-breaking within the same tick is the existing `BTreeMap`-stable iteration.
7. **Replay-safe.** `ContentionResolved` events feed into the canonical-state hash via the existing event-log replay path (the workspace's `blake3` hashing substrate); replay regenerates them deterministically.
8. **No silent privilege.** Events are emission-only. Reading them does not change game state; AI consultation of events for `BlockingFact::ReservationConflict` attribution is read-only.
9. **No new external dependency.** Bounded payload uses `Vec<T>` with runtime truncation, not `SmallVec`. `worldwake-core` retains its current external dependencies (`serde`, `bincode`, `blake3`).

## Non-Goals

- **Live contention metrics dashboard.** Aggregate contention-rate metrics live in observer post-hoc analysis and the existing soak telemetry, not in real-time ECS state.
- **Per-agent contention learning state.** Whether the agent learns "that orchard is too contested" is folded into S138's `LearnedOpportunityMemory` and S131's `SourceReliability` (which already records `average_wait_ticks`). S142 does not duplicate.
- **Speculative resolution rules.** S142 names only `ArrivalTime` because that is the resolution mechanism actually present in code today (`promote_head` + `auto_promote`). Future contention rules — office grants, legal priority, initiative contests, strength contests, reservation tokens — are added by the spec that introduces their substrate, not pre-authored here.
- **`ContentionPolicy` as enum.** `ContentionPolicy` (`contention.rs:50`) is a struct (`grant_hold_ticks`, `auto_promote`, `max_waiters`); it is not extended into a discriminated rule selector. The resolution rule for facility-queue and resource-extraction grants is implicit (always `ArrivalTime` today) and is set at emission time, not selected from a per-policy enum.
- **Save-format break.** Adding an `EventTag` variant + an `EventPayload.contention_event_payload` carrier + a later payload widen to `BlockingFact::ReservationConflict` under the existing event-log path requires a `SAVE_FORMAT_VERSION` bump (one increment, 74 → 75). Per FND-28, pre-S142 saves remain unsupported by the current-format loader; no legacy shim is added.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `ContentionResolutionRule` is a typed enum with explicit variants; no abstract "contention score." `AffordanceKey` is a typed struct, not a stringly-typed identifier. |
| FND-9 (Scheduling, Simultaneity, and Tie-Breaking Are Part of the World Model) | The resolution rule is declared in the event; tick-order tie-breaking is no longer silent — it appears as `ContentionResolutionRule::ArrivalTime` with arrival-tick records on each claimant. |
| FND-25 (Social Artifacts Are First-Class) | The event itself is a social artifact: it records who contended, who won, by what rule. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The enum is single-variant by design; it does not pre-author dead variants whose substrate doesn't exist. Future variants land with their substrate. The `BlockingFact::ReservationConflict` payload widening replaces the unit form rather than coexisting with it. |
| FND-29 (Debuggability Is a Product Feature) | "Why did Agent A get the orchard slot and Agent B did not at tick 412?" is answerable from the event log. |
| FND-29A (Causal History Is Authoritative, Append-Only, and Queryable) | Events are append-only; resolution history is queryable by `events_by_tag(EventTag::ContentionResolved)`. |

## Deliverables

### D1. `EventTag::ContentionResolved`

Added to `crates/worldwake-core/src/event_tag.rs:7`. Save-format bump 74 → 75.

### D2. `AffordanceKey`, `ContentionEventPayload`, and supporting types

```rust
pub struct AffordanceKey {
    pub facility: EntityId,
    pub action: ActionDefId,
}

pub struct ContentionEventPayload {
    pub contested_affordance: AffordanceKey,
    pub place: EntityId,
    pub resolution_rule: ContentionResolutionRule,
    pub claimants: Vec<ContentionClaimant>,           // truncated to 8 at emit time, arrival-order
    pub total_claimants: u16,                          // when claimants > 8
    pub winner: Option<EntityId>,                       // None for queue-only shifts (no head promoted)
    pub at_tick: Tick,
}

pub struct ContentionClaimant {
    pub agent: EntityId,
    pub arrived_tick: Tick,
    pub queue_position: u16,                            // derived at emit time from BTreeMap ordinal
    pub outcome: ClaimantOutcome,
}

pub enum ClaimantOutcome {
    Granted,
    QueuedAhead,
    QueuedBehind,
    Denied { reason: DenialReason },
}

pub enum ContentionResolutionRule {
    ArrivalTime,
}
```

All four types live in `worldwake-core`. `AffordanceKey` derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize` (matching `EntityId` + `ActionDefId`). The payload struct derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` (the inner `Vec<ContentionClaimant>` precludes `Copy`). The enum and `ClaimantOutcome` derive `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`.

`queue_position` is derived at emit time from the waiter's ordinal in `ContentionQueue.waiting` (`BTreeMap<u32, ContentionWaiter>`) BEFORE the grant mutation removes the head; emission code reads ordinals and maps the head to position 1, the subsequent claimants to 2, 3, …

`EventPayload` carries the typed payload in `contention_event_payload: Option<ContentionEventPayload>`, exposed through `EventView::contention_event_payload`. Emission code writes it via `WorldTxn::set_contention_event_payload`, which also tags the event with `EventTag::ContentionResolved`.

### D3a. Facility-queue emission

Extend `crates/worldwake-systems/src/facility_queue.rs::promote_ready_head` so its existing `commit_queue_update(world, event_log, facility, queue, tick, QueueUpdateEffects { extra_tag: Some(EventTag::QueueGrantPromoted), ... })` call also writes a `ContentionEventPayload` through `WorldTxn::set_contention_event_payload`. The resulting committed event is indexed by both `QueueGrantPromoted` and `ContentionResolved`. Implementation note: capture the queue's `BTreeMap<u32, ContentionWaiter>` snapshot BEFORE `promote_head` mutates it, so the emission code has the full claimant set with arrival order and ordinals. The `ContentionEventPayload` fields are derived as follows for this path:

- `contested_affordance.facility` = the facility entity holding the queue
- `contested_affordance.action` = the granted waiter's `intended_action`
- `place` = facility's effective place
- `resolution_rule` = `ArrivalTime`
- `claimants` = head-8 of waiters by `queued_at` ordinal, with the head as `Granted` and the rest as `QueuedAhead`/`QueuedBehind` per their position relative to the granted head
- `total_claimants` = full waiting count (including head)
- `winner` = `Some(granted.actor)`

Emit alongside `QueueGrantPromoted`, not in place of it; both tags and the typed payload are part of the canonical event record.

### D3b. Resource-extraction emission

Add `ContentionResolved` emission to `crates/worldwake-systems/src/production_actions.rs::grant_or_signal_full` (line 484). Today this function locates an available slot (line 504), sets `queue.granted = Some(ContentionGrant { … })` at line 528, and writes no event. Extend it so each successful slot grant writes a `ContentionResolved` event via the same `WorldTxn → commit(event_log)` path used by adjacent extraction code. As in D3a, snapshot the chosen slot's `waiting` BTreeMap before mutation. Payload fields are derived as in D3a, with `contested_affordance.facility` set to the workstation entity holding the `ResourceExtractionQueues` and `contested_affordance.action` set to the granted action.

### D4. `BlockingFact::ReservationConflict` payload widening

Ticket S142CONEVEINS-002 widens the pre-S142 unit variant at `crates/worldwake-core/src/blocker_memory.rs:197` (`ReservationConflict,`) to a struct variant:

```rust
pub enum BlockingFact {
    ReservationConflict {
        affordance: AffordanceKey,
        contention_event: Option<EventId>,
    },
    // existing variants unchanged
}
```

`Option<EventId>` is `Copy`, preserving the live `Copy` derive on `BlockingFact`. Mandatory blast-radius work:

- Audit all 17 `BlockingFact::ReservationConflict` use sites workspace-wide (`grep -rn "BlockingFact::ReservationConflict" crates/ --include="*.rs"`); update each construction site to the new struct form and each destructuring/match site to bind the new fields. Tests asserting the bare unit variant need updating.
- Verify the `BlockingFact` enum's derives still hold. If `BlockingFact` derives `Copy`, `AffordanceKey` is `Copy` and `Option<EventId>` is `Copy`, so the derive is preserved.
- Do not add legacy save compatibility for the unit form. Ticket 002 widens the current-format `BlockingFact::ReservationConflict` shape after ticket 001's save-version bump; older save formats remain rejected at the header gate.

### D5. AI population of `contention_event`

`crates/worldwake-ai/src/agent_tick/execution.rs` populates `contention_event` when the reservation-conflict path observes a `ContentionResolved` event for the same affordance at the conflict tick. The lookup uses `events_by_tag(EventTag::ContentionResolved)` filtered by `(affordance, at_tick)`. Reads only; no game-state mutation.

### D6. Observer Section 12 (Contention)

Add `## Section 12 — Contention` to `crates/worldwake-cli/src/bin/observer.rs`, following the existing section header convention (e.g., line 1111: `## Section 11 — Artifact Lifecycle`). Per-tick rendering:

```
Tick 412 — Contention: orchard@TownEdge (Harvest Apples)
  rule: ArrivalTime
  claimants (3):
    Agent A — arrived t=410, position 1, Granted
    Agent B — arrived t=411, position 2, QueuedAhead
    Agent C — arrived t=412, position 3, QueuedBehind
```

Optional CLI flag `--contention-top-n` surfaces top-N contentions per run by claimant count.

## FND-01 Section H — Causal Hooks Declaration

(Section H provides full coverage for the new declarations only, per spec-drafting-rules.md guidance for system-extension specs.)

1. **Information-path analysis.** `ContentionResolved` events propagate through the existing event log. Co-located perception of the grant (existing perception substrate) carries the visible part of the event to other agents; uninvolved agents do not perceive the event unless they also contended. The event itself is authoritative history visible to observer-only diagnostics. Per-claimant `arrived_tick` carries the temporal evidence intrinsically; no separate evidence-reference collection is needed for the `ArrivalTime` rule.
2. **Positive-feedback analysis.** No amplification. Each contention resolution emits exactly once.
3. **Concrete dampeners.** Not applicable.
4. **Stored state vs derived read-model list.**
   - **Stored authoritative state**: the appended events themselves (in event log per FND-29A); existing `ContentionQueue`/`ContentionGrant` state unchanged; `BlockingFact::ReservationConflict { affordance, contention_event }` belongs to the existing `BlockerMemory` substrate (same persistence as before, widened payload).
   - **Derived read-model**: observer Section 12 rendering, AI lookup of `contention_event` from `events_by_tag(EventTag::ContentionResolved)` at the conflict tick. `ContentionClaimant.queue_position` is derived at emit time from `ContentionQueue.waiting` ordinal — not stored on `ContentionWaiter`.

## SystemFn Integration

No new `SystemFn`. Emission happens inside the existing `facility_queue::promote_ready_head` and `production_actions::grant_or_signal_full` functions. Tick ordering unchanged.

## Component Registration

No new ECS components. Events live in the event log.

## Cross-System Interactions

- **Systems → Core**: `facility_queue.rs` and `production_actions.rs` emit `ContentionResolved` events through the existing `WorldTxn → commit(event_log)` writer.
- **Core → AI**: AI reads `events_by_tag(EventTag::ContentionResolved)` for `BlockingFact::ReservationConflict` attribution.
- **Core → CLI**: observer reads events.

No direct cross-system calls (FND-26).

## Profile-Driven Parameters

Not applicable — contention resolution is per-affordance, not per-agent. The `ContentionPolicy` struct on facility queues remains as it is today; S142 does not extend it. The single live `ContentionResolutionRule::ArrivalTime` is set at emission time based on the substrate (facility-queue or resource-extraction), both of which use arrival-time ordering today.

## Validation and Falsification

- **Golden coverage**: new `golden_contention_inspectability.rs` with four scenarios:
  1. Three agents converge on a single-slot orchard via the resource-extraction path → expects one `ContentionResolved` event per slot grant with all three claimants in arrival order, winner = first arrival, `resolution_rule = ArrivalTime`.
  2. Resource-extraction slot grant in `survival-contested.ron` style → expects per-slot grant events with correct `(facility, action)` keys.
  3. Facility queue admission on a wash basin → expects queued-ahead/queued-behind classification per claimant via the `promote_ready_head` path.
  4. `BlockingFact::ReservationConflict` with non-`None` `contention_event` → end-to-end attribution from AI failure path to the resolution record.
- **Replay parity**: 1440-tick `survival-contested.ron` replay produces identical `ContentionResolved` event sequence pre/post-replay (deterministic emission).
- **Coverage**: `every_grant_emits_contention_event()` conformance test asserts both the facility-queue grant path (`promote_ready_head`) and the resource-extraction grant path (`grant_or_signal_full`) emit the event. Any future grant-issuance code path added to either substrate must be instrumented; the test enforces no third path bypasses emission.

## Risks

- **Event-log volume.** Contested scenarios may emit many contention events. Mitigation: existing S71/S72 delta compaction handles them; bounded `Vec<ContentionClaimant>` truncated to 8 at emit time caps payload size; soak measures aggregate event-log delta footprint.
- **`BlockingFact::ReservationConflict` payload-widening blast radius.** The pre-ticket unit form had 17 `worldwake-ai` use sites (mix of construction and destructuring). Mitigation: ticket S142CONEVEINS-002 performs the workspace-wide migration in one pass with grep enumeration; tests covering the bare unit form are updated alongside production code in the same change.
- **Snapshot-before-mutate ordering in emission code.** Both D3a and D3b read `ContentionQueue.waiting` ordinals BEFORE the grant mutation removes the head. Mitigation: emission helper accepts the pre-mutation queue snapshot as a parameter; the unit test for the helper exercises the case where the head is mutated between snapshot and emission to confirm the snapshot is the authoritative source for `queue_position`.
- **Discrepancy backreference latency.** The AI populates `contention_event` by looking up an event emitted earlier in the same tick; if the lookup index is per-tick rebuilt, cost grows. Mitigation: a follow-up ticket measures lookup cost on `survival-contested.ron`; if it exceeds 1% of agent_tick, an O(1) index is added (per-affordance most-recent-resolution pointer).
