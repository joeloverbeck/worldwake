# S110: Authoritative Decision History Events

## Summary

Add authoritative decision-history events to the append-only event log so FND-29 ("why did this agent do that?") and FND-29A (causal history is authoritative and queryable) can be answered from world history alone, not from optional `AgentDecisionTrace` sinks. Every committed/rejected goal, adopted/invalidated plan, expectation mismatch, repair application, blocker recording, and replan trigger lands as a typed `EventTag` variant plus a typed decision-event payload on the main event log. Heavy search-frontier traces remain optional (via the existing `DecisionTraceSink`); this spec lands only the lightweight, always-on causal spine.

## Phase and Status

Phase 8: Belief-First Continual Planning Foundation. Status: Draft.

## Crates

- `worldwake-core` — new `EventTag` variants (`event_tag.rs`); new `DecisionEventPayload` sum enum and component payload structs (new `decision_event_payload.rs`); `EventPayload` gains an `Option<DecisionEventPayload>` field; relocate `MaterializationTag` from `worldwake-sim` to `worldwake-core` so core-owned payloads can reference it without violating crate layering; `CognitiveProfile` gains `decision_history_alternatives`.
- `worldwake-sim` — re-export `MaterializationTag` from `worldwake-core` for backwards source compatibility with existing AI-crate and systems-crate consumers; no new behavior.
- `worldwake-ai` — emit the new events at goal-commit, goal-reject, plan-adopt, plan-invalidate, expectation-mismatch, repair-apply, blocker-record, replan-trigger call sites; build `DecisionEventPayload` values from existing trace data at emission time.
- `worldwake-cli` — observer "Decision History" section renders the new events.

## Dependencies

- S109 (Typed Discrepancy Taxonomy) — **landed** at `archive/specs/S109-typed-discrepancy-taxonomy.md`. `BlockerRecordedPayload` reuses the `Discrepancy` / `BlockerKey` / `BlockingFact` types S109 delivered. Both `DiscrepancyMemory` and `BlockerMemory` remain active after S109 — the payload is intentionally shaped to record either memory class.

## Design Goals

- Every agent decision (commit, reject, suspend, abandon, replan, repair) leaves a record in the authoritative event log. No decision is visible only through debug traces.
- The record is small. Rejected-alternative summaries store counts and reason classes, not full frontier dumps. Per-agent truncation via `CognitiveProfile::decision_history_alternatives` means the authoritative log can carry differently-sized commit events for different agents; the event remains authoritative for that agent's decision, not a uniform cross-agent comparable surface.
- Replay reconstruction from the event log produces the same agenda transitions as the live run (already an invariant of Worldwake; S110 just widens what "state" the event log carries).
- Observer tooling and post-hoc analysis can answer "why was goal X rejected at tick T?" without rerunning the sim with tracing enabled.

## Non-Goals

- Full decision-trace replacement. `AgentDecisionTrace` stays as the opt-in deep-trace sink. S110 is the lightweight spine.
- Query API over the event log — consumers read the log sequentially. A future spec may add indexed queries if observer workload demands.
- Backwards-compatible event-tag decoding — FND-28 applies. Old logs without the new variants and new `decision_payload` field are not decodable as S110-era logs.
- Saved game state migration across the EventPayload shape change — old save files are not loadable after this spec lands.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-29 (Debuggability Is a Product Feature) | The new events answer "why did this agent commit / reject / replan?" directly from world history. No developer spelunking required. |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | Each decision event is append-only, typed, and carries a stable identity. A later exoneration does not erase an earlier rejected-goal record. |
| FND-9 (Scheduling, Simultaneity, Tie-Breaking Are Part of the World Model) | Decision events record the tick at which each transition fired, so simultaneity and ordering are inspectable. Deterministic tie-break for the bounded `rejected_alternatives` list is required (see D3). |
| FND-22A (Learning, Habits, Preference Shifts Are Concrete State) | `ExpectationMismatch` and `RepairApplied` events are the authoritative origin records for learning updates. Learning hooks read from the log rather than being injected implicitly. |
| FND-26 (Systems Interact Through State) | The planner writes decision events to the same append-only log the action system uses. No privileged cross-system call — the event log is a shared authoritative state surface. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | Old logs and save states that predate the `EventPayload::decision_payload` field are not loadable. No shim is added. |

## Deliverables

### D1: New `EventTag` variants

In `crates/worldwake-core/src/event_tag.rs`, extend the existing `EventTag` enum with eleven new unit variants. `EventTag` derives `Copy + Clone + Eq + PartialEq + Ord + PartialOrd + Hash + Serialize + Deserialize`, so all new variants remain unit-shaped. The tag is a classifier only; payload data lives in `DecisionEventPayload` (D2).

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
    /// A typed discrepancy or blocker was recorded against the failure memories (from S109).
    BlockerRecorded,
}
```

Update the test-side `ALL_EVENT_TAGS` constant and variant count in `event_tag.rs` to cover the new variants (existing tests verify `Ord` stability and round-trip).

### D2: `DecisionEventPayload` and `EventPayload::decision_payload`

Add `decision_payload: Option<DecisionEventPayload>` to `EventPayload` in `crates/worldwake-core/src/event_record.rs`. The field is `Option<_>` so the vast majority of events (world mutation, trade, combat, travel) carry `None`. Only the eleven new decision-event kinds populate it.

Define the payload family in a new file `crates/worldwake-core/src/decision_event_payload.rs`:

```rust
pub enum DecisionEventPayload {
    GoalOffered(GoalOfferedPayload),
    GoalSuppressed(GoalSuppressedPayload),
    GoalCommitted(GoalCommittedPayload),
    GoalSuspended(GoalSuspendedPayload),
    GoalAbandoned(GoalAbandonedPayload),
    PlanAdopted(PlanAdoptedPayload),
    PlanInvalidated(PlanInvalidatedPayload),
    ExpectationMismatch(ExpectationMismatchPayload),
    RepairApplied(RepairAppliedPayload),
    ReplanTriggered(ReplanTriggeredPayload),
    BlockerRecorded(BlockerRecordedPayload),
}
```

Representative component-payload shapes (final field names at implementation time). Every new type defined here lives in `worldwake-core` and transitively references only core types (`EntityId`, `GoalKey`, `BlockerKey`, `BlockingFact`, `Discrepancy`, `MaterializationTag`, `Tick`, primitives):

```rust
pub struct GoalOfferedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub emitter: EmitterTag,
    pub source_evidence: EvidenceSummary,
}

/// Stable identifier for the candidate-emission function that produced an
/// offer. One variant per emitter registered in
/// `worldwake-ai::candidate_generation`. The AI crate maps its internal
/// emitter inventory onto this core-owned tag at emission time; future
/// emitter additions extend this enum.
pub enum EmitterTag {
    HomeostaticNeeds,
    ObligationExecution,
    Exploration,
    LearnedOpportunity,
    // ... exhaustive per candidate_generation emitters at implementation time.
}

/// Compact summary of the evidence set backing a candidate offer — counts
/// and kinds, never full entity lists. Authoritative stored state; reads
/// at implementation time from the existing `GroundedGoal` evidence fields.
pub struct EvidenceSummary {
    pub evidence_kind_counts: BTreeMap<EvidenceKindTag, u16>,
}

pub enum EvidenceKindTag {
    HomeostaticPressure,
    PerceptionObservation,
    InstitutionalRecord,
    LearnedOpportunity,
    // ... covers every evidence source the emitters currently attach.
}

pub struct GoalSuppressedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub reason: GoalRejectionReason,
}

pub struct GoalCommittedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub motive_score: u32,
    pub rejected_alternatives: Vec<RejectedAlternativeSummary>,
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

pub struct GoalSuspendedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub reason: SuspensionReason, // existing core type, reused
}

pub struct GoalAbandonedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub reason: GoalRejectionReason,
}

pub struct PlanAdoptedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub plan_step_count: u16,
}

pub struct PlanInvalidatedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub reason: PlanInvalidationReason,
}

pub enum PlanInvalidationReason {
    BeliefUpdate { claim_key: BeliefClaimKey }, // core
    TargetGone { target: EntityId },
    ExpectationMismatch { step_index: u16 },
    ContentionLost { place: EntityId, action: ActionDefId },
    DiscrepancyRecorded { discrepancy: Discrepancy }, // S109 core type
    PreemptedByHigherGoal { new_goal: GoalKey },
    AgentIncapacitated,
}

/// Pre-S114 payload shape. Before S114 lands, `ExpectationMismatch` fires
/// only for `expected_materializations` mismatches detected in existing
/// revalidation paths, and the payload carries the materialization-tag
/// set from the failed plan step. When S114 adds `PlanExpectation` with
/// richer guard kinds, this payload is widened in place (FND-28 — old
/// logs are not decodable across that change).
pub struct ExpectationMismatchPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub step_index: u16,
    pub expected_materializations: Vec<MaterializationTag>, // relocated to core; see Crates
}

pub struct RepairAppliedPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub step_index: u16,
    pub repair_kind: RepairKind,
    pub substitute_target: Option<EntityId>,
}

pub enum RepairKind {
    AlternateTarget,
    AlternateRoute,
    AlternateMerchant,
    AlternateRecipe,
    // ... exhaustive per repair paths in failure_handling.rs at implementation time.
}

pub struct ReplanTriggeredPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub reason: ReplanReason,
}

pub enum ReplanReason {
    PlanInvalidated,
    LocalRepairExhausted,
    SearchBudgetExhausted,
    GoalSwitched,
}

pub struct BlockerRecordedPayload {
    pub agent: EntityId,
    pub blocker_key: BlockerKey,
    /// Populated when the failure lands in `DiscrepancyMemory`.
    pub discrepancy: Option<Discrepancy>,
    /// Populated when the failure lands in `BlockerMemory` (contention
    /// losses, structural blockers). Both DiscrepancyMemory and
    /// BlockerMemory remain active per S109; exactly one of the two
    /// fields is `Some` on any given event.
    pub blocking_fact: Option<BlockingFact>,
    pub expires_tick: Tick,
}
```

All types derive `Clone + Debug + Eq + PartialEq + Serialize + Deserialize`; they do NOT need `Copy` because they are not stored on `EventTag` and the hot path does not copy them. They reach the event log inside `EventPayload::decision_payload` as an owned `Option`.

### D3: Rejected-alternative bounding

`GoalCommittedPayload::rejected_alternatives` is bounded to the first `N` alternatives by `(motive_score_desc, goal_key_asc)` (default `N = 5`, profile-driven on `CognitiveProfile::decision_history_alternatives: u8`). The secondary `GoalKey` sort key breaks motive-score ties deterministically, consistent with Worldwake's `ChaCha8Rng` + `BTreeMap` determinism invariant. A rejected alternative entry is `(goal_key, reason, score_gap)` — three small fields, not a full goal serialization.

### D4: Relocate `MaterializationTag` to `worldwake-core`

`MaterializationTag` currently lives in `crates/worldwake-sim/src/action_handler.rs:38` and is a `Copy + Ord + Hash + Serialize` enum with a single variant (`SplitOffLot`). Move the definition to `crates/worldwake-core/src/materialization_tag.rs`; re-export from `crates/worldwake-sim/src/action_handler.rs` (or `lib.rs`) so existing consumers in `worldwake-ai` (`planner_ops.rs::ExpectedMaterialization`) and `worldwake-systems` continue to resolve without API churn. `ExpectedMaterialization` is left in the AI crate — it wraps an AI-internal `HypotheticalEntityId` and is not an event-log concern. The event-payload side uses bare `Vec<MaterializationTag>` (the tag list), not `ExpectedMaterialization`.

### D5: Emission sites

Representative emission call sites (final wiring at implementation time):

- `candidate_generation.rs` (one call per emitted offer) → `EventTag::GoalOffered` + `GoalOfferedPayload`
- `ranking.rs::rank_candidates` / `rank_candidates_with_memories` → `EventTag::GoalSuppressed` + `GoalSuppressedPayload` (when an offer is filtered before scoring). The `rejected_alternatives` list is captured at commit, not here.
- `agent_tick/planning.rs` (post-ranking commit path) → `EventTag::GoalCommitted` + `GoalCommittedPayload` with the captured alternatives
- `agent_tick/planning.rs` (successful plan build) → `EventTag::PlanAdopted` + `PlanAdoptedPayload`
- `goal_switching.rs` / `agent_tick` goal-switch path → `EventTag::GoalSuspended` / `EventTag::GoalAbandoned` + matching payloads
- `plan_revalidation.rs` (every invalidation return path) → `EventTag::PlanInvalidated` + `PlanInvalidatedPayload`
- `plan_revalidation.rs` / action execution (step expectation check — pre-S114 fires on explicit `expected_materializations` mismatches; S114 widens the trigger set) → `EventTag::ExpectationMismatch` + `ExpectationMismatchPayload`
- `failure_handling.rs` (any successful local repair path) → `EventTag::RepairApplied` + `RepairAppliedPayload`
- `failure_handling.rs` / memory-record call sites (both `DiscrepancyMemory` and `BlockerMemory` recording points) → `EventTag::BlockerRecorded` + `BlockerRecordedPayload`
- `agent_tick/active_action.rs::handle_plan_failure` + any replan trigger → `EventTag::ReplanTriggered` + `ReplanTriggeredPayload`

Emission-volume note: every candidate offer emits `GoalOffered`. In high-candidate scenarios (e.g., `survival-contested.ron`), this multiplies per-tick event count meaningfully (tens of offers per agent-tick times many agents). The growth is intentional — the log replaces heuristic observer synthesis from S85/S98 (`archive/specs/S85-observer-behavioral-enrichment.md`, `archive/specs/S98-observer-affordance-change-detection.md`) — but a future spec may add per-agent gating if observer workload demands it.

### D6: Event-log replay invariance

Add a replay test: given an event log containing the new decision events, replay must produce the same agenda transitions (committed goal sequence, suspended set, discrepancy memory contents) as the live run. Since S110 adds tags/payloads only — it does not change any world-state mutation — this reduces to "decoding must not fail and `decision_payload` round-trips." The test wires through `crates/worldwake-sim/src/replay_execution.rs` / `replay_state.rs`, reusing the existing replay harness. The invariance check is a safety net, not a semantic change.

### D7: Observer integration

The observer bin (`crates/worldwake-cli/src/bin/observer.rs`) already renders event-log summaries (see the `event_log` consumption around line 2387). Add a new "Section — Decision History" rendered as a single markdown table:

```
| Tick | Agent | Event | Payload Summary |
```

Rows are emitted in event-log order filtered to the new eleven `EventTag` variants. `Payload Summary` is a deterministic one-line string produced per variant (e.g., `goal=HomeostaticThirst motive=420 alts=3` for `GoalCommitted`). The rendering is deterministic under the existing observer-snapshot-test contract. This replaces much of what the observer-behavioral-enrichment waves (`archive/specs/S85-observer-behavioral-enrichment.md`, `archive/specs/S98-observer-affordance-change-detection.md`) previously synthesized from heuristics.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Decision events are agent-local records of the agent's own decisions. They do not propagate to other agents — another agent learning about this one's commitments goes through `ShareBelief` or witness observation (separate paths). The event log is authoritative but not a social carrier. An observer watching the log sees the agent's private reasoning; an in-world agent does not, except through the usual perception/testimony carriers.
2. **Positive-feedback analysis**: The BlockerRecorded path closes a local loop. A failure emits `BlockerRecorded` → the corresponding `BlockerMemory` or `DiscrepancyMemory` entry gates future candidate emission / ranking → retries are suppressed until TTL → a fresh attempt rebuilds evidence. The loop is bounded by memory TTLs (per-class on `DiscrepancyMemory`, 3-bucket on `BlockerMemory` — both landed in S109). No new amplification is introduced by S110 itself; the loop already existed in the memory architecture and S110 merely records its transitions.
3. **Concrete dampeners**: The S109 TTLs (`stale_belief_backoff_ticks`, `contradicted_belief_backoff_ticks`, `transient_block_ticks`, `structural_block_ticks`, etc. on `CognitiveProfile`) are the concrete dampener. No new dampener is needed for event emission itself: the log is append-only and read-only for consumers.
4. **Stored state vs. derived read-model**: Event log entries (`EventRecord` including the new `EventPayload::decision_payload`) are authoritative stored state (append-only, FND-29A). Observer-side summaries (e.g., "commit rate per agent", "rejection-reason histogram") are derived views over the log and must be recomputable from the log alone.

## SystemFn Integration

No new SystemFn. Events are emitted inline at existing decision call sites the same way `ActionStarted` / `ActionCommitted` are emitted today.

## Component Registration

None. The new events attach to the event log, not to agent components. `CognitiveProfile` gains `decision_history_alternatives: u8` with `#[serde(default = "default_decision_history_alternatives")]` (default: 5). The default function preserves save / scenario round-trip when an older serialized profile is loaded.

## Cross-System Interactions

- **AI planner ↔ event log**: The planner writes decision events to the same append-only log the action system uses. No privileged cross-system call — the event log is a shared authoritative state surface (FND-26).
- **S109 discrepancy memory ↔ `BlockerRecorded`**: Discrepancy records that land in `DiscrepancyMemory` / `BlockerMemory` also emit a `BlockerRecorded` event with the same payload. The memory is the queryable view; the log is the authoritative append-only record. Exactly one of `BlockerRecordedPayload::discrepancy` / `blocking_fact` is `Some`, matching the memory class the failure lands in.
- **S114 plan step guards ↔ `ExpectationMismatch`**: When S114 lands, guard-violation detection widens the trigger set and the `ExpectationMismatchPayload` is widened in place to carry the richer `PlanExpectation` (FND-28 — old logs are not decodable across that change). Pre-S114, the event fires only on explicit `expected_materializations` mismatches already detected by existing revalidation code.

## Profile-Driven Parameters

| Parameter | Profile | Type | Default | Purpose |
|-----------|---------|------|---------|---------|
| `decision_history_alternatives` | `CognitiveProfile` | `u8` | 5 | Cap on `rejected_alternatives` entries per commit event |

## Validation and Falsification

### Unit tests

1. `GoalCommittedPayload::rejected_alternatives` is truncated to `decision_history_alternatives` entries, sorted by `(motive_score desc, goal_key asc)`.
2. Each `DecisionEventPayload` variant decodes round-trip (bincode) with no loss.
3. `EventPayload` with and without `decision_payload = Some(_)` both round-trip through bincode.
4. `MaterializationTag` re-export from `worldwake-sim::action_handler` resolves for existing consumers (compile-only check).

### Integration tests

5. Replay: a recorded event log round-trips — the sequence of commit / reject / invalidate events matches the live run's agenda transitions exactly.
6. Existing golden trace (e.g., `golden_healer_acquires_remote_ground_medicine_for_patient`) — confirm the event log contains the expected `GoalCommitted`, `PlanAdopted`, `ActionStarted`, `ActionCommitted` sequence.
7. A revalidation golden — confirm the event log contains `PlanInvalidated` with the correct `PlanInvalidationReason`.
8. A blocker golden — confirm `BlockerRecorded` fires with the typed discrepancy (`DiscrepancyMemory` path) and with a `BlockingFact` (`BlockerMemory` contention-loss path).

### Observer regression

9. Observer "Decision History" section renders the new events for one of the existing scenarios (`survival-baseline.ron`, `survival-contested.ron`). Snapshot test against a small deterministic tick window.

## Outcome

To be filled in at completion.
