# S136: Always-On Decision Event Payload Extension

**Status**: Draft

## Summary

S110 made decision-history events always-on: `EventTag::GoalCommitted`, `PlanAdopted`, `BlockerRecorded`, `ReplanTriggered`, `ExpectationMismatch`, `GoalOffered`, `GoalSuppressed`, `GoalAbandoned`, `GoalSuspended`, `PlanInvalidated`, `RepairApplied`, and `SourceExpectationFailure` are emitted unconditionally in `crates/worldwake-ai/src/agent_tick/`. The optional `DecisionTraceSink` (gated by `enable_tracing()`) handles expensive expansion-level diagnostics — frontier expansion, beam pruning, FF helpful-action analysis. That split is correct.

The remaining gap is in *what the always-on payload contains*. Today's events carry the chosen goal, the plan ID, and (for failures) the typed `Discrepancy`. They do not carry the *minimal causal explanation*: which competing goals were rejected and why, which beliefs were decisive, which records or observations the decision rested on, which assumptions the plan relies on. Reconstructing "why did Agent X commit to Eat instead of Drink at tick 412?" requires either replaying with `enable_tracing()` (expensive — 19+ test sites use it) or inferring backward from the `BeliefStore` snapshot near that tick (lossy). Per FND-29A (causal history must answer "why did this agent do that?"), the always-on layer is incomplete.

S136 extends the always-on event payloads with five fields the assessor named: `top_rejected_goals`, `decisive_beliefs`, `decisive_records`, `decisive_world_observations`, and `assumptions`. Each is a small bounded vector of typed references — never the full ranking trace, never the search expansion. The cost is per-event bytes, not per-tick CPU. The benefit is that observer reports and post-hoc forensics can answer FND-29A questions without re-running the simulation under tracing.

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Crates

- `worldwake-core` — extends `decision_event_payload` module: each of the existing `*Payload` structs (`GoalCommittedPayload`, `PlanAdoptedPayload`, `BlockerRecordedPayload`, `ReplanTriggeredPayload`) gains five new typed fields. Adds `GoalRejectionSummary`, `BeliefRef`, `RecordRef`, `ObservationRef`, `PlanAssumptionRef` types. `SAVE_FORMAT_VERSION` increments.
- `worldwake-ai` — emission sites in `agent_tick/planning.rs:1023,1042`, `agent_tick/execution.rs:140,222,448,503`, `agent_tick/observation.rs:123`, `agent_tick/mod.rs:476,497,516,621,682,696,882,1774,1815` populate the new fields from the same per-tick state already computed for ranking, plan adoption, and revalidation. No new computation.
- `worldwake-sim` — event-log delta compaction (`S71`) extends `BeliefStoreDiff::CompactSet` to handle the new payload variants.
- `worldwake-cli` — observer Section 3 (Decision History) renders the new fields. Replay decoding handles the schema bump.

## Dependencies

- S110 (Decision History Events) — completed. Provides the always-on emission infrastructure and the existing payload types. S136 extends those payloads, not replaces them.
- S109 (Typed Discrepancy Taxonomy) — completed. `Discrepancy` types continue to carry the failure-reason payload; S136 layers `decisive_*` references atop.
- S113 (Belief Envelope) — completed. `BeliefRef` reuses S113's belief addressing.
- S122 (Frame Assumption — Commodity Availability) — completed. `PlanAssumptionRef` reuses the live `FrameAssumption` taxonomy.
- S71 (Event Log Delta Compaction) — completed. The compact path absorbs the additive payload growth.

## Design Goals

1. **Bounded per-event cost.** Each new field is a `SmallVec<T, N>` with `N ≤ 4`. The payload growth per event is bounded; replay cost is unaffected.
2. **No new computation.** The five fields populate from state the planner already computes during ranking, plan adoption, and revalidation. No additional ranking pass, no additional belief query.
3. **Typed references, not snapshots.** `BeliefRef`, `RecordRef`, `ObservationRef`, `PlanAssumptionRef` are stable typed addresses (entity ID + claim key + tick), not embedded value snapshots. Forensics resolves them against the same-tick belief store via existing replay.
4. **`GoalRejectionSummary` is post-tiebreaker, not pre-rank.** The summary lists up to four goals that lost the tiebreaker against the chosen goal, with the dimension that ordered them (motive, source-composite, ranking-source). Pre-rank-filtered goals (suppressed, infeasible at probe) appear with their existing `GoalSuppressed`/`GoalOffered` events, not in this summary.
5. **Decisive-evidence is not exhaustive.** The fields name the *load-bearing* facts — beliefs/records/observations/assumptions whose absence would have flipped the decision. Existing decision-trace machinery already classifies these (S110 + S122); S136 surfaces the classification into the always-on payload.
6. **No new event tag.** S136 extends the payloads of existing tags. No taxonomic growth in `EventTag`.
7. **Determinism preserved.** All references emit in `BTreeMap`-stable order. `SmallVec` capacity bounds make payload size deterministic.
8. **Replay parity.** Pre-S136 saves replay forward by zero-filling the new fields (declared via `#[serde(default)]`). Post-S136 saves do not replay backward into pre-S136 binaries — per FND-28, no compatibility shim.

## Non-Goals

- **Full search-frontier traces in the always-on path.** Beam pruning, FF heuristic, expansion-level summaries remain opt-in via `DecisionTraceSink::enable_tracing()`. S136 only lands the *minimal-explanation* layer.
- **Cross-tick aggregation of decisive evidence.** Each event carries the single tick's evidence. Patterns across ticks are observer-derived, not stored.
- **Belief value embedding.** `BeliefRef` carries the address; the value at that address must be resolved from the per-tick belief store via replay. No value embedding (avoids payload bloat under contradiction-rich scenarios).

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | The new fields name typed references (entity IDs, claim keys, expectation IDs), never abstract decision scores. The motive score in the existing payload remains; S136 adds the *referent* layer. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | `decisive_beliefs` and `decisive_records` reference the per-agent provenance the agent acted on. Reconstructing the knowledge path remains traceable. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | The "Agent X chose Y because they believed Z and cared about Q" explanation becomes always reconstructable, satisfying the FND-20 test. |
| FND-21 (Intentions Are Revisable Commitments) | `assumptions` lists the load-bearing `FrameAssumption`s; later `ExpectationMismatch` events name which assumption broke. |
| FND-29 (Debuggability Is a Product Feature) | "Why did this agent do that?" is answerable from the always-on event log, not only from a re-run. |
| FND-29A (Causal History Is Authoritative, Append-Only, and Queryable) | Decision events become self-contained authoritative history rather than indexes that require optional traces to interpret. |

## Deliverables

### `worldwake-core::decision_event_payload` extension

Each existing payload struct gains five fields. Example for `GoalCommittedPayload`:

```rust
pub struct GoalCommittedPayload {
    // existing fields preserved (goal_key, motive_score, plan_id, …)
    pub top_rejected_goals: SmallVec<GoalRejectionSummary, 4>,        // NEW
    pub decisive_beliefs: SmallVec<BeliefRef, 4>,                     // NEW
    pub decisive_records: SmallVec<RecordRef, 4>,                     // NEW
    pub decisive_world_observations: SmallVec<ObservationRef, 4>,     // NEW
    pub assumptions: SmallVec<PlanAssumptionRef, 4>,                  // NEW
}
```

`PlanAdoptedPayload`, `BlockerRecordedPayload`, `ReplanTriggeredPayload`, `ExpectationMismatchPayload` get the same five fields. Other payloads (`GoalOfferedPayload`, `GoalSuppressedPayload`, `GoalAbandonedPayload`, `GoalSuspendedPayload`, `RepairAppliedPayload`, `SourceExpectationFailurePayload`) get only the subset that applies — `GoalSuppressedPayload` carries `decisive_beliefs` and `decisive_records` (the suppression rationale), not `assumptions` (no plan yet).

### New typed reference types

```rust
pub struct GoalRejectionSummary {
    pub rejected_goal_key: GoalKey,
    pub rejection_dimension: RankedGoalComparisonDimension,  // existing enum
    pub margin: i32,                                          // motive-score delta
}

pub struct BeliefRef {
    pub claim_key: BeliefClaimKey,                            // existing
    pub claim_held_at_tick: Tick,
    pub status: BeliefStatus,                                 // existing (S113)
}

pub struct RecordRef {
    pub record_entity: EntityId,
    pub topic: RecordTopic,                                   // existing
    pub recorded_at_tick: Tick,
}

pub struct ObservationRef {
    pub observed_entity: EntityId,
    pub aspect: EntityBeliefAspect,                           // existing
    pub observed_tick: Tick,
}

pub struct PlanAssumptionRef {
    pub assumption: FrameAssumption,                          // existing (S122)
    pub introduced_at_step: u8,
    pub source: AssumptionSource,                             // existing
}
```

### Emission-site population

At each emission site, the `top_rejected_goals` field is populated from the `RankedGoals` already in scope (the rank result the planner used). The `decisive_*` fields are populated from the `DecisionContext` the agent used to rank (already in scope per S110). The `assumptions` field is populated from the active plan's `FrameAssumption` set (S122).

No new query, no new ranking pass.

### Observer Section 3 extension

`crates/worldwake-cli/src/bin/observer.rs` Section 3 (Decision History) renders the new fields under each event:
```
Tick 412 — Agent A — GoalCommitted: Eat
  motive 18420 (above Drink 17890, Wash 12200)
  decisive beliefs: hunger@critical, ApplePresent@market(t=410)
  decisive records: <none>
  decisive observations: AppleAtMarket(t=410)
  assumptions: CommodityAvailableAt(market, apple), NeedSafeUntilTick(550)
```

### `SAVE_FORMAT_VERSION` bump

Increment by one. Pre-S136 saves load via `#[serde(default)]` zero-fills; post-S136 saves cannot be loaded by pre-S136 binaries. Per FND-28, no shim.

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** No new information path — every reference S136 stores in the payload was already in scope at the emission site (motive ranking, decision context, frame assumptions). The path is now *recorded* rather than discarded after the tick.
2. **Positive-feedback analysis.** No amplifying loop. Decision events are append-only history.
3. **Concrete dampeners.** Not applicable.
4. **Stored state vs derived read-model list.**
   - **Stored authoritative state**: extended decision-event payloads in the event log (already authoritative per S110 + FND-29A).
   - **Derived read-model**: observer Section 3 reconstruction.

## SystemFn Integration

No new `SystemFn`. Emission happens at the same call sites S110 already established.

## Component Registration

No new components. Decision events live in the event log, not in ECS.

## Cross-System Interactions

- **AI → Core**: emission-site code reads from already-in-scope ranking and decision-context state, populates the extended payload, emits through the existing event-log path.
- **Sim → CLI**: observer replays the event log and renders the new payload fields. No new cross-system call.

## Profile-Driven Parameters

Not applicable — payload contents are derived per-decision, not per-agent profiles. Bounded `SmallVec<T, 4>` capacity is a global decision-event design constant chosen to bound per-event size.

## Validation and Falsification

- **Golden coverage**: new `golden_decision_payload.rs` with three scenarios:
  1. Eat-vs-Drink contested commit → assert `GoalCommittedPayload.top_rejected_goals` contains Drink with the correct margin.
  2. Stale-belief replan → assert `ReplanTriggeredPayload.decisive_beliefs` names the contradicted claim with `BeliefStatus::Stale`.
  3. Assumption breach → assert `ExpectationMismatchPayload.assumptions` names the breached `FrameAssumption::CommodityAvailableAt` from S122.
- **Replay parity**: every pre-S136 save replays forward (with zero-filled new fields) without behavioral divergence. The S136 fields are observability-only; absence does not change agent decisions.
- **Bounded payload size**: assertion that per-event payload size never exceeds a fixed byte ceiling under property-based scenario generation across the soak harness.

## Risks

- **Event log size growth.** Five `SmallVec<T, 4>` fields per event could double per-event byte size. Mitigation: `SmallVec` inline capacity tuned to scenario-typical sizes (most decisions have 0–2 rejected goals, 1–3 decisive beliefs); S71 delta compaction handles the rest. Soak measures the actual footprint pre-merge.
- **Decisive-evidence classification.** What counts as "decisive" must be defined. Mitigation: ticket-001 fixes the classification rule — a fact is decisive iff the agent's existing decision-context API already names it as a load-bearing input to the ranking or revalidation step that produced the event. The classifier is mechanical, not heuristic.
