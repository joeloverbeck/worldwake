# S110: Authoritative Decision History Events

## Summary

Add authoritative decision-history events to the append-only event log so FND-29 ("why did this agent do that?") and FND-29A (causal history is authoritative and queryable) can be answered from world history alone, not from optional `AgentDecisionTrace` sinks. Every committed/rejected goal, adopted/invalidated plan, expectation mismatch, repair application, blocker recording, and replan trigger lands as a typed `EventTag` variant on the main event log. Heavy search-frontier traces remain optional (via the existing `DecisionTraceSink`); this spec lands only the lightweight, always-on causal spine.

## Phase and Status

Phase 8: Belief-First Continual Planning Foundation. Status: Draft.

## Crates

- `worldwake-sim` — new `EventTag` variants; payload structs in the event-log schema
- `worldwake-ai` — emit the new events at goal-commit, goal-reject, plan-adopt, plan-invalidate, expectation-mismatch, repair-apply, blocker-record, replan-trigger call sites
- `worldwake-core` — event-tag ordering invariants, SystemId tag if needed

## Dependencies

- S109 (Typed Discrepancy Taxonomy) — `BlockerRecorded` carries a `Discrepancy`. Soft; S110 can land with a forward-declared placeholder if S109 hasn't landed, but the two should land in the same wave.

## Design Goals

- Every agent decision (commit, reject, suspend, abandon, replan, repair) leaves a record in the authoritative event log. No decision is visible only through debug traces.
- The record is small. Rejected-alternative summaries store counts and reason classes, not full frontier dumps.
- Replay reconstruction from the event log produces the same agenda transitions as the live run (already an invariant of Worldwake; S110 just widens what "state" the event log carries).
- Observer tooling and post-hoc analysis can answer "why was goal X rejected at tick T?" without rerunning the sim with tracing enabled.

## Non-Goals

- Full decision-trace replacement. `AgentDecisionTrace` stays as the opt-in deep-trace sink. S110 is the lightweight spine.
- Query API over the event log — consumers read the log sequentially. A future spec may add indexed queries if observer workload demands.
- Backwards-compatible event-tag decoding — FND-28 applies. Old logs without the new variants are not decodable as S110-era logs.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-29 (Debuggability Is a Product Feature) | The new events answer "why did this agent commit / reject / replan?" directly from world history. No developer spelunking required. |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | Each decision event is append-only, typed, and carries a stable identity. A later exoneration does not erase an earlier rejected-goal record. |
| FND-9 (Scheduling, Simultaneity, Tie-Breaking Are Part of the World Model) | Decision events record the tick at which each transition fired, so simultaneity and ordering are inspectable. |
| FND-22A (Learning, Habits, Preference Shifts Are Concrete State) | `ExpectationMismatch` and `RepairApplied` events are the authoritative origin records for learning updates. Learning hooks read from the log rather than being injected implicitly. |

## Deliverables

### D1: New `EventTag` variants

In `crates/worldwake-sim/src/event_tag.rs` (or wherever `EventTag` lives — grep confirms `EventTag::ActionStarted`, `ActionCommitted`, `ActionAborted`, `Discovery` exist today):

```rust
pub enum EventTag {
    // ... existing variants ...

    /// An emitter produced a candidate goal offer this tick.
    GoalOffered,
    /// Emitter arbitration or ranking suppressed an offer before it
    /// reached the commit step.
    GoalSuppressed,
    /// Ranking selected a goal for commitment.
    GoalCommitted,
    /// A committed goal was suspended (awaiting a condition).
    GoalSuspended,
    /// A committed goal was abandoned.
    GoalAbandoned,
    /// A plan was adopted for the committed goal.
    PlanAdopted,
    /// A live plan was invalidated (belief update, preempt, expectation mismatch).
    PlanInvalidated,
    /// A plan step's expectation did not match what actually occurred.
    ExpectationMismatch,
    /// A local repair was applied to a failing step (alternate target,
    /// alternate route, alternate merchant).
    RepairApplied,
    /// Replan was triggered (after invalidation, exhaustion, or repair-exhausted).
    ReplanTriggered,
    /// A typed discrepancy was recorded against the blocker/discrepancy memories (from S109).
    BlockerRecorded,
}
```

### D2: Event payload schema

Each new tag carries a compact struct on the event's domain-payload side. Representative shapes (final field names at implementation time):

```rust
pub struct GoalOfferedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub emitter: EmitterId,
    pub source_evidence: EvidenceSummary, // compact: kind + count, not full entity list
}

pub struct GoalCommittedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub motive_score: u32,
    pub rejected_alternatives: Vec<RejectedAlternativeSummary>, // bounded, see D3
}

pub struct RejectedAlternativeSummary {
    pub goal_key: GoalKey,
    pub rejection_reason: GoalRejectionReason,
    pub score_gap: i32, // signed: positive = committed exceeded alt; negative = tie-break
}

pub enum GoalRejectionReason {
    LowerMotive,
    FeasibilityProbeFailed,
    SuppressedByBlocker,
    SuppressedByDiscrepancy,
    SuppressedByContentionPreempt,
    ArbitrationLost,
    SwitchMarginInsufficient,
}

pub struct PlanInvalidatedPayload {
    pub agent: EntityId,
    pub plan_id: PlanId,
    pub reason: PlanInvalidationReason,
}

pub enum PlanInvalidationReason {
    BeliefUpdate { claim_key: BeliefClaimKey },
    TargetGone { target: EntityId },
    ExpectationMismatch { step_index: u16 },
    ContentionLost { place: EntityId, action: ActionDefId },
    DiscrepancyRecorded { discrepancy: Discrepancy }, // from S109
    PreemptedByHigherGoal { new_goal: GoalKey },
    AgentIncapacitated,
}

pub struct ExpectationMismatchPayload {
    pub agent: EntityId,
    pub plan_id: PlanId,
    pub step_index: u16,
    pub expected: PlanExpectation,
    pub observed: Option<ObservationDelta>,
}

pub struct RepairAppliedPayload {
    pub agent: EntityId,
    pub plan_id: PlanId,
    pub step_index: u16,
    pub repair_kind: RepairKind,
    pub substitute_target: Option<EntityId>,
}

pub struct BlockerRecordedPayload {
    pub agent: EntityId,
    pub blocker_key: BlockerKey,
    pub discrepancy: Option<Discrepancy>, // Some when S109's memories record; None for legacy BlockerMemory
    pub blocking_fact: Option<BlockingFact>, // for the BlockerMemory path
    pub expires_tick: Tick,
}
```

### D3: Rejected-alternative bounding

`GoalCommittedPayload::rejected_alternatives` is bounded to the first `N` alternatives by motive score (default `N = 5`, profile-driven on `CognitiveProfile::decision_history_alternatives`). The bound keeps event size predictable. A rejected alternative entry is `(goal_key, reason, score_gap)` — three small fields, not a full goal serialization.

### D4: Emission sites

Representative emission call sites (final wiring at implementation time):

- `candidate_generation.rs` → `GoalOffered` (one per emitted offer)
- `ranking.rs::rank_goals` → `GoalSuppressed` (when an offer is filtered before scoring) and the `rejected_alternatives` list captured at commit
- `agent_tick/planning.rs` (post-ranking) → `GoalCommitted` with the captured alternatives
- `agent_tick` goal-switch path → `GoalSuspended` / `GoalAbandoned`
- `agent_tick/planning.rs` (successful plan build) → `PlanAdopted`
- `plan_revalidation.rs` (any invalidation return) → `PlanInvalidated`
- `plan_revalidation.rs` / execution (step expectation check — depends on S114's guards; pre-S114 it fires on explicit `expected_materializations` mismatches) → `ExpectationMismatch`
- `failure_handling.rs` (any successful local repair path) → `RepairApplied`
- `failure_handling.rs` / memory-record call sites → `BlockerRecorded`
- `agent_tick` / `handle_plan_failure` (any replan trigger) → `ReplanTriggered`

### D5: Event-log replay invariance

Add a replay test: given an event log containing the new decision events, replay must produce the same agenda transitions (committed goal sequence, suspended set, discrepancy memory contents) as the live run. Since S110 only adds tags/payloads — it does not change any world-state mutation — this reduces to "decoding must not fail." The invariance test is a safety net, not a semantic change.

### D6: Observer integration

The observer bin (`crates/worldwake-cli/src/bin/observer.rs`) already renders event-log summaries. Extend it to render the new decision events in a dedicated "Decision History" section, formatted as `tick | agent | event | compact payload summary`. This is the primary user-facing benefit of S110 and replaces much of what the observer-behavioral-enrichment waves (S85, S98) previously synthesized from heuristics.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Decision events are agent-local records of the agent's own decisions. They do not propagate to other agents (another agent's learning about this one's commitments goes through `ShareBelief` or witness observation — separate paths). The event log is authoritative but not a social carrier.
2. **Positive-feedback analysis**: Not applicable. Event emission is a record-only side effect; it does not feed back into decisions.
3. **Concrete dampeners**: Not applicable.
4. **Stored state vs. derived read-model**: Event log entries are authoritative stored state (append-only, FND-29A). Observer-side summaries (e.g., "commit rate per agent") are derived views over the log.

## SystemFn Integration

No new SystemFn. Events are emitted inline at existing decision call sites the same way `ActionStarted` / `ActionCommitted` are emitted today.

## Component Registration

None. The new events attach to the event log, not to agent components. `CognitiveProfile` gains `decision_history_alternatives: u8` (D3).

## Cross-System Interactions

- **AI planner ↔ event log**: The planner writes decision events to the same append-only log the action system uses. No privileged cross-system call — the event log is a shared authoritative state surface (FND-26).
- **S109 discrepancy memory ↔ `BlockerRecorded`**: Discrepancy records that land in `DiscrepancyMemory` / `BlockerMemory` also emit a `BlockerRecorded` event with the same payload. The memory is the queryable view; the log is the authoritative append-only record.
- **S114 plan step guards ↔ `ExpectationMismatch`**: When S114 lands, guard-violation detection emits the `ExpectationMismatch` event. Pre-S114, the event fires only on explicit `expected_materializations` mismatches (already tracked in `PlannedStep`).

## Profile-Driven Parameters

| Parameter | Profile | Type | Default | Purpose |
|-----------|---------|------|---------|---------|
| `decision_history_alternatives` | `CognitiveProfile` | `u8` | 5 | Cap on `rejected_alternatives` entries per commit event |

## Validation and Falsification

### Unit tests

1. `GoalCommittedPayload::rejected_alternatives` is truncated to `decision_history_alternatives` entries.
2. Each emitted event decodes round-trip (serde) with no loss.
3. Replay a recorded event log — the sequence of commit/reject/invalidate events matches the live run's agenda transitions exactly.

### Integration tests

4. Existing golden trace (e.g., `golden_healer_acquires_remote_ground_medicine_for_patient`) — confirm the event log contains the expected `GoalCommitted`, `PlanAdopted`, `ActionStarted`, `ActionCommitted` sequence.
5. A revalidation golden — confirm the event log contains `PlanInvalidated` with the correct `PlanInvalidationReason`.
6. A blocker golden — confirm `BlockerRecorded` fires with the typed discrepancy.

### Observer regression

7. Observer "Decision History" section renders the new events for one of the existing scenarios (`survival-baseline.ron`, `survival-contested.ron`). Snapshot test against a small deterministic tick window.

## Outcome

To be filled in at completion.
