# S142: Contention Event Inspectability

**Status**: Draft

## Summary

S127 (Quantity-Aware Acquisition) and the existing facility-queue substrate (`crates/worldwake-core/src/contention.rs`) carry contention *state*: `ContentionQueue` (per-affordance waiting list), `ContentionGrant` (active grant), `ContentionWaiter` (per-claimant entry), `ResourceExtractionQueues` (per-source slot lists). The state is inspectable post-hoc through ECS reads. What is *not* inspectable is the *resolution moment* — the tick at which two or more agents contended for the same scarce affordance, the rule that resolved it, the evidence that informed the resolution, and the loser set.

When Agent A and Agent B both reach a single-slot orchard at the same tick, today's resolution is silent: the `BTreeMap`-stable iteration over claimants, combined with arrival-time ordering inside the queue, produces a deterministic winner. The winner's grant fires; the loser's request is denied or queued. There is no inspectable record of "at tick 412, three agents contended for orchard slot 1; arrival-time rule selected Agent A; Agents B and C remained queued." Per FND-9 (scheduling and contention are part of the world model — tick order, thread order, container iteration order may not silently decide who saw the dropped coin first), and FND-29 (debuggability), the resolution must produce a queryable artifact, not just a state delta.

S142 adds `ContentionEvent` as a first-class append-only event payload emitted whenever a contention resolution fires. Every grant-issuance, queue-shift, and resource-extraction-slot-grant emits the event with the contested affordance, the claimants in arrival order, the resolution rule that fired, the evidence consulted, and the winner/loser split. Observer Section 6 renders contention events per tick; goldens assert resolution attribution. The existing queue substrate is unchanged; S142 is purely an emission and inspection layer atop it.

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Crates

- `worldwake-core` — extends `EventTag` (`crates/worldwake-core/src/event_tag.rs:7`) with `ContentionResolved`. Adds `ContentionEventPayload` carrying the resolution record. Adds `ContentionResolutionRule` enum (typed names for `ArrivalTime`, `QueuePosition`, `ReservationToken`, `OfficeGrant`, `PhysicalProximity`, `Initiative`, `StrengthContest`, `LegalPriority`, `SeededMicrostate`).
- `worldwake-sim` — `contention.rs` grant-issuance and queue-shift paths emit `ContentionResolved` events. `ResourceExtractionQueues::grant_next_slot` emits at slot-grant time (S127's existing path). `queue_for_facility_use` emits at queue admission and grant-out time. No state shape change.
- `worldwake-systems` — no change beyond the per-action handlers being routed through the (unchanged) contention paths.
- `worldwake-ai` — decision-trace records `ContentionEventRef` for grants the agent's planning depended on. `Discrepancy::ReservationConflict` (existing, S127 ticket -010) carries the resolved-event reference.
- `worldwake-cli` — observer Section 6 (Contention) renders per-tick contention events. Optional `--contention-top-n` flag to surface top-N contentions per run by claimant count.

## Dependencies

- S127 (Quantity-Aware Acquisition) — completed. Provides `ResourceExtractionQueues` and `extraction_slot_*` accessors. S142 emits events from the existing grant path.
- S110 (Decision History Events) — completed. `ContentionResolved` joins the existing event-tag taxonomy.
- S109 (Typed Discrepancy Taxonomy) — completed. `Discrepancy::ReservationConflict` extension carries the new event reference.
- S118 (Stuck-Agent Detector Active-Frame Exclusion) — completed. Observer's stuck-agent detector consumes contention events to attribute waits.

## Design Goals

1. **Every resolution emits.** Any tick at which a `ContentionGrant` issues, an extraction slot grants, or a queue position shifts due to contention pressure produces a `ContentionResolved` event. No silent winner.
2. **Typed resolution rule.** `ContentionResolutionRule` names the rule that fired. `ArrivalTime` covers the dominant case; the other variants exist for future contention-substrate extensions (combat initiative, legal priority for office holders, strength contests for physical contests).
3. **Bounded event payload.** Up to 8 claimants per event recorded inline; longer queues are summarized as `total_claimants: u16` plus the head-8.
4. **Evidence references.** Each event carries `evidence: SmallVec<EvidenceRef, 2>` — for `ArrivalTime`, the arrival-tick records; for `OfficeGrant`, the office-authority reference; for `LegalPriority`, the warrant or contract reference.
5. **No new state.** S142 introduces no ECS component, no derived index, no cache. The events are append-only history.
6. **Determinism preserved.** Event emission order matches the existing grant-issuance order; tie-breaking within the same tick is the existing `BTreeMap`-stable iteration.
7. **Replay-safe.** `ContentionResolved` events feed into the same `BlakeChain` canonical-state hash; replay regenerates them deterministically.
8. **No silent privilege.** Events are emission-only. Reading them does not change game state; AI consultation of events for `Discrepancy::ReservationConflict` attribution is read-only.

## Non-Goals

- **Live contention metrics dashboard.** Aggregate contention-rate metrics live in observer post-hoc analysis and the existing soak telemetry, not in real-time ECS state.
- **Per-agent contention learning state.** Whether the agent learns "that orchard is too contested" is folded into S138's `LearnedOpportunityMemory` and S131's `SourceReliability` (which already records `average_wait_ticks`). S142 does not duplicate.
- **New contention rules.** S142 names the existing `ArrivalTime` rule and reserves variants for future substrate. It does not introduce new resolution mechanics.
- **Save-format break.** Adding an `EventTag` variant + a payload struct under the existing event-log path requires a `SAVE_FORMAT_VERSION` bump (one increment); no shim required since pre-S142 saves simply lack the new event records.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `ContentionResolutionRule` is a typed enum with explicit variants; no abstract "contention score." |
| FND-9 (Scheduling, Simultaneity, and Tie-Breaking Are Part of the World Model) | The resolution rule is declared in the event; tick-order tie-breaking is no longer silent — it appears as `ContentionResolutionRule::ArrivalTime` with arrival-tick evidence. |
| FND-25 (Social Artifacts Are First-Class) | The event itself is a social artifact: it records who contended, who won, by what rule. Other agents can in principle learn from it (existing perception of grant events). |
| FND-29 (Debuggability Is a Product Feature) | "Why did Agent A get the orchard slot and Agent B did not at tick 412?" is answerable from the event log. |
| FND-29A (Causal History Is Authoritative, Append-Only, and Queryable) | Events are append-only; resolution history is queryable by `events_by_tag(EventTag::ContentionResolved)`. |

## Deliverables

### `EventTag::ContentionResolved`

Added to `crates/worldwake-core/src/event_tag.rs:7`. Save-format bump.

### `ContentionEventPayload`

```rust
pub struct ContentionEventPayload {
    pub contested_affordance: AffordanceKey,
    pub place: EntityId,
    pub resolution_rule: ContentionResolutionRule,
    pub claimants: SmallVec<ContentionClaimant, 8>,    // arrival-order
    pub total_claimants: u16,                          // when claimants > 8
    pub winner: Option<EntityId>,                       // None for queue-only shifts
    pub evidence: SmallVec<EvidenceRef, 2>,
    pub at_tick: Tick,
}

pub struct ContentionClaimant {
    pub agent: EntityId,
    pub arrived_tick: Tick,
    pub queue_position: u16,
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
    QueuePosition,
    ReservationToken { token: ReservationTokenId },
    OfficeGrant { office: EntityId },
    PhysicalProximity,
    Initiative { contest: InitiativeContestRef },
    StrengthContest { contest: StrengthContestRef },
    LegalPriority { authority: AuthorityKind },
    SeededMicrostate { seed: u64 },
}
```

`AffordanceKey` is the existing affordance-identifier shape used in `ContentionQueue` keys.

### Emission sites

- `crates/worldwake-sim/src/contention.rs::grant_next_in_queue` → emit on grant.
- `crates/worldwake-sim/src/contention.rs::ResourceExtractionQueues::grant_slot` (S127 path) → emit on slot-grant.
- `crates/worldwake-systems/src/facility_queue_actions.rs` → emit on queue admission and grant-out.

Each emission is a single function call into the existing event-log writer; no new write path.

### Decision-trace integration

`Discrepancy::ReservationConflict` extension:

```rust
pub enum Discrepancy {
    ReservationConflict {
        affordance: AffordanceKey,
        contention_event: Option<EventId>,    // NEW
    },
    // existing variants
}
```

The AI's `agent_tick/execution.rs` populates `contention_event` when the conflict path observes a `ContentionResolved` event for the same affordance at the conflict tick.

### Observer Section 6 (Contention)

```
Tick 412 — Contention: orchard@TownEdge slot-1
  rule: ArrivalTime
  claimants (3):
    Agent A — arrived t=410, position 1, Granted
    Agent B — arrived t=411, position 2, QueuedAhead
    Agent C — arrived t=412, position 3, QueuedBehind
  evidence: arrival-records ×3
```

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** `ContentionResolved` events propagate through the existing event log. Co-located perception of the grant (existing perception substrate) carries the visible part of the event to other agents; uninvolved agents do not perceive the event unless they also contended. The event itself is authoritative history visible to observer-only diagnostics.
2. **Positive-feedback analysis.** No amplification. Each contention resolution emits exactly once.
3. **Concrete dampeners.** Not applicable.
4. **Stored state vs derived read-model list.**
   - **Stored authoritative state**: the appended events themselves (in event log per FND-29A); existing `ContentionQueue`/`ContentionGrant` state unchanged.
   - **Derived read-model**: observer Section 6 rendering, `Discrepancy::ReservationConflict.contention_event` lookup.

## SystemFn Integration

No new `SystemFn`. Emission happens inside the existing contention substrate's grant/queue functions. Tick ordering unchanged.

## Component Registration

No new ECS components. Events live in the event log.

## Cross-System Interactions

- **Sim → Core**: contention emits through the existing event-log writer.
- **Sim → AI**: AI reads `events_by_tag(EventTag::ContentionResolved)` for `Discrepancy::ReservationConflict` attribution.
- **Sim → CLI**: observer reads events.

No direct cross-system calls (FND-26).

## Profile-Driven Parameters

Not applicable — contention resolution is per-affordance, not per-agent. Per-affordance resolution-rule selection lives on the affordance's existing definition (e.g., `ContentionPolicy::FirstArrival` on facility queues), not on a new profile.

## Validation and Falsification

- **Golden coverage**: new `golden_contention_inspectability.rs` with four scenarios:
  1. Three agents converge on a single-slot orchard → expects one `ContentionResolved` event with all three claimants in arrival order, winner = first arrival.
  2. Resource-extraction slot grant in `survival-contested.ron` style → expects per-slot grant events.
  3. Facility queue admission on a wash basin → expects queued-ahead/queued-behind classification per claimant.
  4. `Discrepancy::ReservationConflict` with non-`None` `contention_event` → end-to-end attribution from AI failure trace to the resolution record.
- **Replay parity**: 1440-tick `survival-contested.ron` replay produces identical `ContentionResolved` event sequence pre/post-replay (deterministic emission).
- **Coverage**: `every_grant_emits_contention_event()` conformance test asserts every code path that issues a grant also emits the event.

## Risks

- **Event-log volume.** Contested scenarios may emit many contention events. Mitigation: existing S71/S72 delta compaction handles them; bounded `SmallVec<ContentionClaimant, 8>` caps payload size; soak measures aggregate event-log delta footprint.
- **Conformance test brittleness.** Adding the every-grant-emits assertion requires every grant code path to be instrumented. Mitigation: ticket-001 audits the three known emission sites and the test enforces no fourth path bypasses emission.
- **Discrepancy backreference latency.** The AI populates `contention_event` by looking up an event emitted earlier in the same tick; if the lookup index is per-tick rebuilt, cost grows. Mitigation: ticket-002 measures lookup cost on `survival-contested.ron`; if it exceeds 1% of agent_tick, an O(1) index is added (per-affordance most-recent-resolution pointer).
