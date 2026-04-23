# S124CANOPPEXP-004: Surface canonical incident through DecisionEventPayload

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — decision-history event payload surface
**Deps**: `specs/S124-canonical-opportunity-expectation-failure.md`, `tickets/S124CANOPPEXP-002.md`

## Problem

S110 delivered [`DecisionEventPayload`](../crates/worldwake-core/src/decision_event_payload.rs) at `worldwake-core/src/decision_event_payload.rs:10` with 11 variants (`GoalOffered`, `GoalSuppressed`, `GoalCommitted`, `GoalSuspended`, `GoalAbandoned`, `PlanAdopted`, `PlanInvalidated`, `ExpectationMismatch`, `RepairApplied`, `ReplanTriggered`, `BlockerRecorded`). Ticket `S124CANOPPEXP-002` introduces `OpportunityExpectationFailureIncident` as a runtime-only reasoning artifact in `worldwake-ai`, but the canonical contradiction currently does not travel through the decision-history event surface. That means future debugging and trace inspection cannot answer "which committed opportunity failed, which concrete source was being trusted, which phase detected the contradiction, what the cause was, and what attribution outcome resulted" from the authoritative event log — it can only answer through ad hoc ai-layer inspection.

This ticket extends `DecisionEventPayload` to surface the incident, with the ai-layer emitter living adjacent to `apply_source_reliability_failure_observations` so every attribution decision produces one event.

## Assumption Reassessment (2026-04-23)

1. `DecisionEventPayload` is defined at [`crates/worldwake-core/src/decision_event_payload.rs:10`](../crates/worldwake-core/src/decision_event_payload.rs). Its 11 variants are all concrete payloads (no `Raw` or catch-all). `ExpectationMismatchPayload` at lines 214-221 currently has fields `agent`, `goal_key`, `step_index`, `expected_materializations`, `expectation_kind: Option<ExpectationKindTag>`, `mismatch_detail: Option<MismatchDetail>`. `ExpectationKindTag` is a separate enum in core used for general step-level expectation mismatch, not for source-backed expectation failure.
2. Existing emit sites confirmed via grep — 17+ `DecisionEventPayload::` construction sites across `agent_tick/mod.rs`, `agent_tick/execution.rs`, `agent_tick/observation.rs`, and `agent_tick/tests.rs`. All emit into the decision-event log; none currently surface source-reliability attribution outcomes.
3. Shared abstraction boundary under audit: `DecisionEventPayload` in `worldwake-core` is the canonical decision-history surface. The incident type `OpportunityExpectationFailureIncident` in `worldwake-ai` (ticket 002) is a runtime-only reasoning artifact. This ticket bridges the two: the ai-layer emitter reads the incident + attribution outcome and constructs a core-resident payload. The incident itself stays in ai; only the flattened trace payload lives in core.
4. Two implementation options per spec D6:
   - **Option A**: Extend `ExpectationMismatchPayload` at `decision_event_payload.rs:214-221` with an optional `source_expectation_failure: Option<SourceExpectationFailurePayload>` field (nested struct).
   - **Option B**: Add a new `DecisionEventPayload::SourceExpectationFailure(SourceExpectationFailurePayload)` variant at line 10-22.
   Option B gives cleaner event separation; option A keeps all expectation-layer events under one variant. Either is spec-compliant. Pick at implementation time based on existing `ExpectationMismatch` usage — if downstream consumers would treat source-backed failure differently from step-level mismatch, option B is cleaner; if they consume both uniformly, option A compresses the surface.
5. Information-path refactor (precision rule 16): the same fact ("a source-backed expectation failed") currently travels only through ai-layer runtime state (pending-failure set + `SourceReliability` mutation). After this ticket, it also travels through the authoritative decision-event log as a first-class event. Both paths coexist intentionally — the runtime incident drives attribution and reconsideration (short-lived, tick-scoped), while the event payload drives inspection and trace (durable, append-only). Neither supersedes the other; no alias removal is required.
6. Durable surface contract (precision rule 7): `DecisionEventPayload` already derives `Serialize`/`Deserialize`; the new payload struct must derive the same so save/load and event-log replay round-trip cleanly. All nested types (`OpportunityKey`, `SourceKey`, `Tick` from core; `OpportunityExpectationKind`, `ExpectationFailurePhase`, `ExpectationFailureCause` from ai) must be `Serialize`/`Deserialize`-compatible. The ai-layer enums currently are NOT serde-derived per ticket 002's "runtime-only, not persisted" framing. This ticket requires either (i) adding serde derives to the ai-layer enums (widening ticket 002's scope retroactively — flag if chosen), OR (ii) defining parallel core enum tags (`OpportunityExpectationKindTag`, `ExpectationFailurePhaseTag`, `ExpectationFailureCauseTag`) in core that the emitter maps the ai-layer values into. Option (ii) is preferred because it preserves the ai-layer "runtime-only" classification and keeps core enum stability independent of ai-layer refactors.
7. Mismatch + correction: ticket 002's Change 1 explicitly says "`Serialize`/`Deserialize` are NOT required — these types are runtime-only." This ticket must respect that boundary; adding serde derives to ai-layer enums would violate it. The correct integration is to define `*Tag` mirrors in core and perform the ai→core mapping at the emit site. Flag as implementation requirement.

## Architecture Check

1. Extending the existing `DecisionEventPayload` surface (option A or B) is cleaner than introducing a parallel decision-event type for source-backed failures. The event log already carries 11 payload variants; adding one more keeps all decision history in one inspectable stream and under one serde-round-tripped surface.
2. Core tag mirrors vs. serde-widening ai-layer enums: the tag-mirror approach preserves the "ai-layer types are runtime-only" invariant from ticket 002 and matches existing patterns in core (e.g., `ExpectationKindTag`, `EmitterTag`, `EvidenceKindTag` are all core-resident tag enums used by `DecisionEventPayload` despite the ai-layer owning the richer runtime types). No shim or conversion layer is introduced beyond the direct ai→core mapping at emit time.
3. Emit site colocation: placing the emitter adjacent to `apply_source_reliability_failure_observations` (`crates/worldwake-ai/src/agent_tick/mod.rs:1904`) means every attribution decision produces exactly one decision event. Scattering the emitter across the three detection sites would violate the spec's "one attribution path" contract.

## Verification Layers

1. A source-expectation failure produces exactly one `DecisionEventPayload` entry per attributed incident, with accurate `OpportunityKey`, `SourceKey`, `expectation_kind`, `phase`, `cause`, and attribution outcome -> focused unit coverage on the emit path, asserting against the decision-event log buffer.
2. Event-log replay round-trips the new payload (serde correctness) -> focused unit coverage in `decision_event_payload.rs` tests module (or a sibling test file) exercising serialize-then-deserialize equivalence for representative payload instances.
3. Existing 11 `DecisionEventPayload` variants continue to encode/decode unchanged -> existing event-log and decision-history tests in the worldwake-ai crate.
4. Causal history is queryable after the ticket lands -> spec FND-29A invariant; inspected via the existing decision-event consumers (observer binary, decision trace tooling).
5. Single-layer ticket beyond those surfaces — action-trace and authoritative world-state layers are N/A because this ticket surfaces an ai-layer reasoning artifact through the decision-event log, not a world-state mutation.

## What to Change

### 1. Define the core payload type and (if option B) variant

In `crates/worldwake-core/src/decision_event_payload.rs`, add:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceExpectationFailurePayload {
    pub agent: EntityId,
    pub opportunity: OpportunityKey,
    pub source: SourceKey,
    pub expectation_kind: OpportunityExpectationKindTag,
    pub phase: ExpectationFailurePhaseTag,
    pub cause: ExpectationFailureCauseTag,
    pub detected_at_tick: Tick,
    pub attribution_outcome: SourceAttributionOutcomeTag,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum OpportunityExpectationKindTag {
    AcquireCommodityFromConcreteSource,
    RestockCommodityFromConcreteSource,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ExpectationFailurePhaseTag {
    Observation,
    CandidateGeneration,
    Search,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ExpectationFailureCauseTag {
    SourceAbsentLocally,
    SourceDepletedLocally,
    SameGoalSearchInfeasibleWhileSiblingSucceeded,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SourceAttributionOutcomeTag {
    SourceReliabilityDecremented,
    SourceInvalidatedFrameReconsidered,
    CoalescedDuplicate,
}
```

If option **B**, add the variant to the enum:
```rust
pub enum DecisionEventPayload {
    // ...existing 11 variants...
    SourceExpectationFailure(SourceExpectationFailurePayload),
}
```

If option **A**, extend `ExpectationMismatchPayload` with `source_expectation_failure: Option<SourceExpectationFailurePayload>`.

The attribution outcome tag gives debuggers the ability to distinguish coalesced duplicates (no additional state change) from the primary attribution event. `CoalescedDuplicate` events are emitted when rule 5 of the writer elides a duplicate.

### 2. Map ai-layer runtime types to core tags at emit site

In `crates/worldwake-ai/src/agent_tick/mod.rs` (adjacent to `apply_source_reliability_failure_observations` at line 1904), add a mapping helper that converts `OpportunityExpectationFailureIncident` (ai-layer) into `SourceExpectationFailurePayload` (core):

```rust
fn to_decision_payload(
    agent: EntityId,
    incident: &OpportunityExpectationFailureIncident,
    attribution_outcome: SourceAttributionOutcomeTag,
) -> SourceExpectationFailurePayload
```

Handle the 1:1 enum mapping `OpportunityExpectationKind → OpportunityExpectationKindTag`, `ExpectationFailurePhase → ExpectationFailurePhaseTag`, `ExpectationFailureCause → ExpectationFailureCauseTag`.

### 3. Emit decision events from the writer

Inside `apply_source_reliability_failure_observations(...)`, after each attribution decision, emit one `DecisionEventPayload::SourceExpectationFailure(payload)` (option B) or one `DecisionEventPayload::ExpectationMismatch(payload_with_source_field)` (option A) through the existing decision-event log facility. Grep `agent_tick/mod.rs` for the current pattern used to emit `PlanInvalidated` or `GoalAbandoned` and follow the same emit shape.

Each incident produces one decision event; coalesced duplicates produce a `CoalescedDuplicate` event so the event log remains a complete record of attribution decisions even when `SourceReliability` is only mutated once per distinct source.

### 4. Round-trip test the new payload

Add a focused unit test that constructs a `SourceExpectationFailurePayload` with representative values, serializes it via `bincode` (or whatever the event log uses), deserializes, and asserts equality. This confirms the new payload is save-format-safe.

## Files to Touch

- `crates/worldwake-core/src/decision_event_payload.rs` (modify — add payload struct + tag enums + variant/field)
- `crates/worldwake-core/src/lib.rs` (modify — re-export new types if needed by ai)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — mapping helper + emit in writer)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — assertion on emitted event)
- `crates/worldwake-core/src/decision_event_payload.rs` or sibling test file (modify — round-trip test)

## Out of Scope

- The ai-layer incident type (`OpportunityExpectationFailureIncident`) and its enums — delivered by ticket `S124CANOPPEXP-002`.
- Adding `Serialize`/`Deserialize` to the ai-layer runtime types — explicitly avoided per ticket 002 scoping; core tag mirrors are the chosen integration.
- Observer binary / decision-trace tooling consumers that read the new event — downstream of this ticket, not part of its scope (follow-up if needed).
- Reconsideration routing (`SourceInvalidated` outcome) — delivered by ticket `S124CANOPPEXP-003`.
- Surfacing systems-layer authoritative-action reliability writes (from `experience_recording.rs`) through `DecisionEventPayload` — they remain on the existing authoritative event path; bridging them into decision-history is out of scope.

## Acceptance Criteria

### Tests That Must Pass

1. A new focused unit test in `agent_tick/tests.rs` (or a sibling) proves that `apply_source_reliability_failure_observations` emits one `DecisionEventPayload::SourceExpectationFailure` (option B) or one `DecisionEventPayload::ExpectationMismatch` with populated `source_expectation_failure` (option A) per attributed incident, and one with `attribution_outcome = CoalescedDuplicate` per elided duplicate.
2. A new focused unit test in `decision_event_payload.rs` tests module (or a sibling) round-trips `SourceExpectationFailurePayload` through `bincode` serialize → deserialize and asserts equality.
3. Existing regression (all 11 existing `DecisionEventPayload` variants continue to round-trip unchanged): `cargo test -p worldwake-core --lib decision_event_payload`
4. Existing regression: `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
5. Existing suite: `cargo test --workspace`

### Invariants

1. Every distinct `(agent, opportunity, source, phase, tick)` attribution decision produces exactly one `DecisionEventPayload` entry; coalesced duplicates produce additional entries tagged `CoalescedDuplicate` so the event log is a complete record of decisions, not merely a sampled one.
2. No ai-layer runtime type gains a `Serialize`/`Deserialize` derive as a result of this ticket. All serde-bearing types for the new surface live in `worldwake-core` as `*Tag` mirrors.
3. The 11 pre-existing `DecisionEventPayload` variants round-trip unchanged; no field, variant, or derive on those variants is modified.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/decision_event_payload.rs` (or sibling test file) — round-trip test for `SourceExpectationFailurePayload` and its tag enums.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — focused test asserting emission count + payload fields after a multi-incident `apply_source_reliability_failure_observations` call, including a coalesced-duplicate assertion.

### Commands

1. `cargo test -p worldwake-core --lib decision_event_payload`
2. `cargo test -p worldwake-ai --lib agent_tick::tests -- --exact`
3. `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
4. `cargo test --workspace`
5. `scripts/verify.sh`
