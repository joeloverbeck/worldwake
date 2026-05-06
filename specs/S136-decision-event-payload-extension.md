# S136: Always-On Decision Event Payload Extension

## Summary

S110 made decision-history events always-on: `EventTag::GoalCommitted`, `PlanAdopted`, `BlockerRecorded`, `ReplanTriggered`, `ExpectationMismatch`, `GoalOffered`, `GoalSuppressed`, `GoalAbandoned`, `GoalSuspended`, `PlanInvalidated`, `RepairApplied`, and `SourceExpectationFailure` are emitted unconditionally in `crates/worldwake-ai/src/agent_tick/`. The optional `DecisionTraceSink` (gated by `enable_tracing()`) handles expensive expansion-level diagnostics — frontier expansion, beam pruning, FF helpful-action analysis. That split is correct.

The remaining gap is in *what the always-on payload contains*. Today's events carry the chosen goal, the plan ID, the typed `Discrepancy` (for failures), and a list of `RejectedAlternativeSummary { goal_key, rejection_reason, score_gap }` for `GoalCommitted`. They do not carry the *minimal causal explanation* on the failure path: which beliefs were decisive, which records or observations the failure rested on, which assumptions the active plan relied on. They also do not surface the *decisive ranking dimension* per rejected alternative on the success path. Reconstructing "why did Agent X commit to Eat instead of Drink at tick 412?" relies on the existing `rejection_reason` field, but answering "which belief made the agent give up at tick 530?" requires either replaying with `enable_tracing()` (expensive — 19+ test sites use it) or inferring backward from the `BeliefStore` snapshot near that tick (lossy). Per FND-29A (causal history must answer "why did this agent do that?"), the always-on layer is incomplete on the failure side.

S136 makes three coordinated changes:

1. Widens the existing `RejectedAlternativeSummary` with an optional `rejection_dimension: Option<RankedGoalComparisonDimensionTag>` so the dimension that ordered the rejected goal against the chosen one is recorded alongside the rejection reason already captured.
2. Extends *failure-path* decision event payloads (`BlockerRecordedPayload`, `ReplanTriggeredPayload`, `ExpectationMismatchPayload`, `SourceExpectationFailurePayload`) with bounded `decisive_beliefs`, `decisive_records`, `decisive_world_observations` lists that name the load-bearing inputs to the failure decision when the live emission seam carries a lawful typed address for that ref family. The classifier on the failure path is mechanical because the relevant failed-claim or observation input is already a function input to those emission sites — no new belief query or placeholder record fabrication. `GoalCommitted` and `PlanAdopted` (success path) do not gain these fields, because no decisive-evidence classifier currently exists outside the opt-in `DecisionTraceSink`. Promoting that classifier to always-on for success events is deliberately out of scope for S136.
3. Adds an `assumptions: Vec<PlanAssumptionRef>` field to `GoalCommittedPayload`, `PlanAdoptedPayload`, `BlockerRecordedPayload`, `ReplanTriggeredPayload`, and `ExpectationMismatchPayload`. To populate `assumptions` at adoption-time emission, S136 reorders `emit_plan_selection_events` so the prepared frame and `populate_assumptions` run before the `GoalCommitted`/`PlanAdopted` emission.

The cost is per-event bytes, not per-tick CPU. The benefit is that observer reports and post-hoc forensics can answer the FND-29A *why-did-this-agent-fail* question without re-running the simulation under tracing.

## Phase and Status

Phase 11: Belief-First Continual Planning Architectural — Draft

## Crates

- `worldwake-core` — extends `decision_event_payload` module:
  - `RejectedAlternativeSummary` gains an optional `rejection_dimension` field.
  - `BlockerRecordedPayload`, `ReplanTriggeredPayload`, `ExpectationMismatchPayload`, `SourceExpectationFailurePayload` gain `decisive_beliefs`, `decisive_records`, `decisive_world_observations` fields.
  - `GoalCommittedPayload`, `PlanAdoptedPayload`, `BlockerRecordedPayload`, `ReplanTriggeredPayload`, `ExpectationMismatchPayload` gain an `assumptions` field.
  - New core-side types: `BeliefRef`, `RecordRef`, `ObservationRef`, `PlanAssumptionRef`, `RankedGoalComparisonDimensionTag` (mirror of the ai-side `RankedGoalComparisonDimension`, following the `BeliefStatusTag` precedent at `decision_event_payload.rs:231`).
- `worldwake-sim` — `SAVE_FORMAT_VERSION` (`save_load.rs:6`) increments by one. Current-format save/load roundtrips preserve the widened payloads. The `RankedGoalComparisonDimension → RankedGoalComparisonDimensionTag` conversion cannot live in `worldwake-sim` because `worldwake-sim` must not depend on `worldwake-ai`; the runtime conversion lives at the AI emission site that populates `rejection_dimension`.
- `worldwake-ai` — three coordinated emission-site changes:
  - `agent_tick/planning.rs::emit_plan_selection_events` — reorder so `update_frame_for_adopted_plan` and `populate_assumptions` run before the `GoalCommitted`/`PlanAdopted` emission, so the populated frame's assumption list is available at emission time.
  - `agent_tick/planning.rs::build_rejected_alternatives` (line 931) — populate the new `rejection_dimension` field from the `RankedGoalComparisonOutcome::decisive_dimension` already computed by ranking (`ranking.rs:2367`, `2587`).
  - Failure-path emission sites (`agent_tick/execution.rs` `BlockerRecorded` / `ReplanTriggered` emissions, `agent_tick/observation.rs` `ExpectationMismatch` emission, `agent_tick/mod.rs` `SourceExpectationFailure` emission) — populate `decisive_beliefs`, `decisive_records`, `decisive_world_observations` from typed failed-claim/observation inputs already in scope at each seam. Record refs remain empty on current paths that do not carry a record entity. `assumptions` populates from the active frame's `assumptions: Vec<FrameAssumption>` (`crates/worldwake-core/src/intention_frame.rs:145`).
- `worldwake-cli` — observer Section 3 (Decision History) extends `decision_payload_summary` (`bin/observer.rs:421`) within the existing single-line table format, adding compact suffixes (e.g., `decisive=B2 R0 O1 assume=2 dim=MotiveScore`) to the affected event summaries. The single-line invariant enforced by the test at `observer.rs:5744` is preserved. Replay decoding handles the schema bump via `#[serde(default)]` zero-fill on the new fields.

## Dependencies

- S110 (Decision History Events) — completed (`archive/specs/S110-decision-history-events.md`). Provides the always-on emission infrastructure and the existing payload types. S136 extends those payloads, not replaces them.
- S109 (Typed Discrepancy Taxonomy) — completed (`archive/specs/S109-typed-discrepancy-taxonomy.md`). `Discrepancy` types continue to carry the failure-reason payload; S136 layers `decisive_*` references atop on the failure path only.
- S113 (Belief Envelope) — completed (`archive/specs/S113-belief-envelope.md`). `BeliefRef` reuses S113's belief addressing through `BeliefClaimKey`. Status is recorded via the existing core-side `BeliefStatusTag` mirror, not the sim-side `BeliefStatus`.
- S122 (Frame Assumption — Commodity Availability) — completed (`archive/specs/S122-frame-assumption-commodity-availability.md`). `PlanAssumptionRef` reuses the live `FrameAssumption` taxonomy. Note that `FrameAssumption` is already self-describing — no separate `AssumptionSource` enum is needed.
- S71 (Event Log Delta Compaction) — completed (`archive/specs/S71-event-log-delta-compaction.md`). Compatible without changes: `event_log.strip_deltas_before(...)` operates on event identity, not payload contents, so additive payload extensions need no compaction-side work. (Earlier draft text incorrectly attributed a payload-aware compaction step to a non-existent `BeliefStoreDiff::CompactSet` variant.)

## Design Goals

1. **Bounded per-event cost.** Each new field is a `Vec<T>` capped at emission time by the existing `cognitive.decision_history_alternatives: u8` profile field (`crates/worldwake-core/src/cognitive_profile.rs:105`, default 5) — the same soft-cap mechanism the existing `rejected_alternatives` field already uses (`planning.rs:991`). No new dependency introduced; capacity remains deterministic because the cap is per-agent profile state.
2. **No new ranking or belief work on the success path; mechanical classification on the failure path.** The dimension (`rejection_dimension`) is derived from the same ranking comparator that ordered the candidates, and the assumption list (`assumptions`) populates from frame preparation. Because `OrderedRanked` does not retain pairwise comparison outcomes, the emission site may call the existing pairwise explanation helper for each emitted rejected alternative; this is bounded by `cognitive.decision_history_alternatives` and is not a new ranking pass or belief query. The failure-path `decisive_*` fields populate from failed-claim or observation inputs already passed to each failure emission site when those inputs carry a lawful typed address (e.g., the `Discrepancy` payload, the violated expectation set, the contradicted-claim set).
3. **Typed references, not snapshots.** `BeliefRef`, `RecordRef`, `ObservationRef`, `PlanAssumptionRef` are stable typed addresses (entity ID + claim key + tick), not embedded value snapshots. Forensics resolves them against the same-tick belief store via existing replay.
4. **Failure-path `decisive_*` is not exhaustive.** The fields name the *load-bearing* facts on the failure decision — beliefs, records, or observations whose absence or contradiction would have flipped the outcome when the corresponding typed address is already present at the seam. Promoting always-on decisive classification to the success path (`GoalCommitted`/`PlanAdopted`) is deliberately deferred to a follow-on spec because no such mechanical classifier exists today outside `DecisionTraceSink`.
5. **`rejection_dimension` is post-tiebreaker, not pre-rank.** The dimension recorded on each `RejectedAlternativeSummary` is the `RankedGoalComparisonOutcome::decisive_dimension` exposed by `explain_ranked_goal_order` over the same comparator as `ranked_goal_ordering` (`ranking.rs:2389-2432`) — i.e., the dimension that orders the rejected goal against the chosen one. Pre-rank-filtered goals (suppressed, infeasible at probe) continue to surface through their existing `GoalSuppressed`/`GoalOffered` events with the existing `rejection_reason: GoalRejectionReason` field, not via dimension.
6. **No new event tag.** S136 extends the payloads of existing tags. No taxonomic growth in `EventTag`.
7. **Determinism preserved.** All references emit in `BTreeMap`-stable order. The Vec-with-cap pattern keeps payload size bounded by per-agent profile state, identical to the existing `rejected_alternatives` discipline.
8. **Replay parity.** S136 preserves current-format replay parity and bumps `SAVE_FORMAT_VERSION`. Pre-S136 v69 saves are rejected by the v70 binary per FND-28/no-backward-compatibility policy; there is no compatibility shim.

## Non-Goals

- **Always-on decisive classifier on the success path.** `GoalCommittedPayload` and `PlanAdoptedPayload` do not gain `decisive_beliefs`/`decisive_records`/`decisive_world_observations` because no current code path classifies them outside `DecisionTraceSink::enable_tracing()`. A follow-on spec may promote that classifier; S136 does not.
- **Full search-frontier traces in the always-on path.** Beam pruning, FF heuristic, expansion-level summaries remain opt-in via `DecisionTraceSink::enable_tracing()`.
- **Cross-tick aggregation of decisive evidence.** Each event carries the single tick's evidence. Patterns across ticks are observer-derived, not stored.
- **Belief value embedding.** `BeliefRef` carries the address; the value at that address must be resolved from the per-tick belief store via replay. No value embedding (avoids payload bloat under contradiction-rich scenarios).
- **New `RecordTopic` or `AssumptionSource` taxonomies.** Earlier drafts named these; they do not exist in the codebase. `RecordRef` carries `record_entity` + `recorded_at_tick` and resolves topic via the record's own state at replay time. `PlanAssumptionRef` carries the `FrameAssumption` itself (which is self-describing) plus `introduced_at_step`.
- **Multi-line observer Section 3 rendering.** The existing single-line table format is preserved (enforced by the `decision_payload_summary_is_single_line_for_goal_committed` test at `observer.rs:5744`). Detailed multi-line rendering, if wanted, is a separate spec.
- **`smallvec` workspace dependency.** S136 uses the existing `Vec<T>` + per-agent soft-cap pattern matching `rejected_alternatives`.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | The new fields name typed references (entity IDs, claim keys, tick stamps), never abstract decision scores. The motive score and existing `rejection_reason` enum remain; S136 adds the *referent* and *dimension* layers. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | `decisive_beliefs` and representable `decisive_world_observations` (failure path) reference the local knowledge the agent acted on; `decisive_records` is populated only when the emission seam carries a lawful record entity. Reconstructing the knowledge path remains traceable without fabricated provenance. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | The "Agent X chose Y because they believed Z" explanation becomes always reconstructable on the failure path; the success path is partially answerable via the existing `rejected_alternatives` plus the new `rejection_dimension`. The full success-path classifier remains a follow-on. |
| FND-21 (Intentions Are Revisable Commitments) | `assumptions` lists the load-bearing `FrameAssumption`s; later `ExpectationMismatch` events name which assumption broke. |
| FND-26 (Systems Through State) | No new system function. All flow is through the existing event-log path. |
| FND-28 (No Backward Compatibility) | Replay-forward via `#[serde(default)]` is boundary-encoding compatibility, allowed by FND-28. No live-authority shim. Pre-S136 saves cannot be loaded by post-S136 binaries by design. |
| FND-29 (Debuggability Is a Product Feature) | "Why did this agent abandon the plan?" is answerable from the always-on event log, not only from a re-run. |
| FND-29A (Causal History Is Authoritative, Append-Only, and Queryable) | Failure-path decision events become self-contained authoritative history rather than indexes that require optional traces to interpret. Success-path explanations remain partial pending the follow-on classifier (Non-Goal). |

## Deliverables

### Per-Tag Field Map

| Event Tag | `rejection_dimension` (on `RejectedAlternativeSummary`) | `decisive_beliefs` / `decisive_records` / `decisive_world_observations` | `assumptions` | Notes |
|-----------|---|---|---|---|
| `GoalCommitted` | yes (via existing `rejected_alternatives` widening) | no | yes | Success path; reorder required (D5). |
| `PlanAdopted` | n/a (no rejected alternatives field) | no | yes | Success path; reorder required (D5). |
| `BlockerRecorded` | n/a | yes | yes | Failure path; classifier from existing `Discrepancy` and contradicted-claim inputs. |
| `ReplanTriggered` | n/a | yes | yes | Failure path; classifier from existing `ReplanReason` and stale-belief inputs. |
| `ExpectationMismatch` | n/a | yes | yes | Failure path; classifier from existing `expected_materializations` and `mismatch_detail` inputs. |
| `SourceExpectationFailure` | n/a | yes | no (per-source, no active plan frame) | Failure path; classifier from existing `cause: ExpectationFailureCauseTag` input. |
| `GoalOffered`, `GoalSuppressed`, `GoalAbandoned`, `GoalSuspended`, `PlanInvalidated`, `RepairApplied` | n/a | no | no | Out of scope for S136. Existing typed reasons (e.g., `GoalRejectionReason`, `SuspensionReason`, `PlanInvalidationReason`) already carry the always-on rationale for these tags. |

### D1 — `RejectedAlternativeSummary` widening with `rejection_dimension`

In `crates/worldwake-core/src/decision_event_payload.rs` (line 164):

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RejectedAlternativeSummary {
    pub goal_key: GoalKey,
    pub rejection_reason: GoalRejectionReason,
    pub score_gap: i32,
    #[serde(default)]
    pub rejection_dimension: Option<RankedGoalComparisonDimensionTag>,  // NEW
}
```

`RankedGoalComparisonDimensionTag` is a new core-side mirror of `worldwake-ai::ranking::RankedGoalComparisonDimension`, defined alongside `BeliefStatusTag` at `decision_event_payload.rs:231` and following the same pattern. The conversion `RankedGoalComparisonDimension → RankedGoalComparisonDimensionTag` lives at the AI emission site that has access to the AI enum; `worldwake-sim` cannot own that conversion without an invalid dependency on `worldwake-ai`.

### D2 — New typed reference types in `worldwake-core::decision_event_payload`

```rust
pub struct BeliefRef {
    pub claim_key: BeliefClaimKey,                            // existing (S113)
    pub claim_held_at_tick: Tick,
    pub status: BeliefStatusTag,                              // existing core mirror
}

pub struct RecordRef {
    pub record_entity: EntityId,
    pub recorded_at_tick: Tick,
    // Topic resolves at replay time from the record entity itself —
    // no separate `RecordTopic` taxonomy is introduced (Non-Goal).
}

pub struct ObservationRef {
    pub observed_entity: EntityId,
    pub aspect: EntityBeliefAspect,                           // existing
    pub observed_tick: Tick,
}

pub struct PlanAssumptionRef {
    pub assumption: FrameAssumption,                          // existing (S122) — self-describing
    pub introduced_at_step: u8,
}
```

All four types derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. `Copy` is derived where the inner field types permit (`BeliefRef`, `ObservationRef`, `PlanAssumptionRef` — all inner types are `Copy`; `RecordRef` is also `Copy`).

### D3 — Failure-path payload extensions

Each of `BlockerRecordedPayload`, `ReplanTriggeredPayload`, `ExpectationMismatchPayload`, `SourceExpectationFailurePayload` gains:

```rust
#[serde(default)]
pub decisive_beliefs: Vec<BeliefRef>,
#[serde(default)]
pub decisive_records: Vec<RecordRef>,
#[serde(default)]
pub decisive_world_observations: Vec<ObservationRef>,
```

Each Vec is capped at emission time by `cognitive.decision_history_alternatives` (the same per-agent profile cap that already bounds `rejected_alternatives`).

### D4 — `assumptions` field on success-path and most failure-path payloads

`GoalCommittedPayload`, `PlanAdoptedPayload`, `BlockerRecordedPayload`, `ReplanTriggeredPayload`, `ExpectationMismatchPayload` gain:

```rust
#[serde(default)]
pub assumptions: Vec<PlanAssumptionRef>,
```

`SourceExpectationFailurePayload` does not carry `assumptions` — source-expectation failures fire per-source with no active-plan frame guaranteed.

### D5 — Reorder `emit_plan_selection_events` so frame is populated before emission

In `crates/worldwake-ai/src/agent_tick/planning.rs` (around line 1685-1710):

Today the call sequence is:
1. `emit_plan_selection_events(...)` emits `GoalCommitted` and `PlanAdopted`.
2. `update_frame_for_adopted_plan(...)` creates the prepared frame.
3. `frame.assumptions = populate_assumptions(...)` populates assumptions.

After D5, the sequence becomes:
1. `update_frame_for_adopted_plan(...)` creates the prepared frame.
2. `frame.assumptions = populate_assumptions(...)` populates assumptions.
3. `emit_plan_selection_events(...)` is called with `frame.assumptions` in scope; passes the assumption list into both the `GoalCommitted` and `PlanAdopted` payloads.

The reorder must preserve:
- The existing `refreshed_view` belief snapshot used by `populate_assumptions` (already obtained before step 1 today).
- The existing post-emission `current_place` and downstream operations.
- All existing decision-trace hooks (decision-trace surface remains unchanged).

D5 has its own golden assertion (see Validation): a paired `GoalCommitted` and `PlanAdopted` event for a contested commit must carry a non-empty `assumptions` field.

### D6 — Emission-site population

At each affected emission site, populate the new fields from already-in-scope state:

- `RejectedAlternativeSummary.rejection_dimension` ← `RankedGoalComparisonOutcome::decisive_dimension` from `ranking::explain_ranked_goal_order` over the emitted rejected alternative and chosen ranked goal, converted to `RankedGoalComparisonDimensionTag`. Wired in `build_rejected_alternatives` at `planning.rs:931-1002`.
- `decisive_beliefs/records/world_observations` ← derived from existing typed inputs to each failure emission site. Each ref family is populated only when that site carries the lawful typed address; otherwise that family remains empty:
  - `BlockerRecorded` (`execution.rs:448, 503`) — from the typed `Discrepancy` payload's contradicted-claim set already passed in.
  - `ReplanTriggered` (`execution.rs:140, 222` and `mod.rs:497`) — from the `ReplanReason` payload's stale-belief / contradicted-claim set.
  - `ExpectationMismatch` (`observation.rs:123`) — from the `expected_materializations` and `mismatch_detail` inputs.
  - `SourceExpectationFailure` (`mod.rs:621`) — from the `cause: ExpectationFailureCauseTag` and the source attribution input.
  - Current S136 failure-path seams do not carry record entities, so `decisive_records` remains empty until a future record-bearing emission seam exists.
- `assumptions` ← active frame's `assumptions: Vec<FrameAssumption>` (`intention_frame.rs:145`). For `GoalCommitted`/`PlanAdopted`, the populated frame is in scope post-D5 reorder. For the failure-path tags, the active frame already exists at emission time.

No new ranking pass, no new belief query.

### D7 — `SAVE_FORMAT_VERSION` bump

In `crates/worldwake-sim/src/save_load.rs` (line 6), increment by one. Pre-S136 v69 saves are rejected by the v70 binary; post-S136 saves cannot be loaded by pre-S136 binaries. Per FND-28, no shim.

### D8 — Observer Section 3 extension within existing format

In `crates/worldwake-cli/src/bin/observer.rs::decision_payload_summary` (line 421), extend the per-tag summaries with compact suffixes for the new fields. The single-line invariant is preserved (see test at line 5744). Examples:

- `GoalCommitted`: `goal=Eat motive=18420 alts=2 dim=MotiveScore assume=2`
- `BlockerRecorded`: `blocker=K reason=BeliefStale decisive=B2 R0 O1 assume=2`
- `ReplanTriggered`: `goal=Eat reason=BeliefUpdate decisive=B1 R0 O0 assume=1`

The detailed multi-line rendering proposal from earlier drafts is not part of S136; it would require breaking the single-line invariant and is deferred to a separate observer-format spec.

## FND-01 Section H — Causal Hooks Declaration

(Section H entries below cover the deliverables S136 introduces; the original payload tags' Section H entries are owned by S110.)

1. **Information-path analysis.** No new information path — every reference S136 stores in the payload was already a function input to the emission site (rejection-comparison outcome at the success site; representable failed-claim/observation inputs at each failure site; populated frame assumptions post-D5 reorder). Ref families with no lawful typed carrier at the seam remain empty. The path is now *recorded* rather than discarded after the tick.
2. **Positive-feedback analysis.** No amplifying loop. Decision events are append-only history.
3. **Concrete dampeners.** Not applicable.
4. **Stored state vs. derived read-model list.**
   - **Stored authoritative state**: extended decision-event payloads in the event log (already authoritative per S110 + FND-29A); the new `rejection_dimension` field on `RejectedAlternativeSummary`; the new `decisive_*` and `assumptions` fields on the named payloads; the new `RankedGoalComparisonDimensionTag` mirror in core.
   - **Derived read-model**: observer Section 3 reconstruction; replay resolution of typed refs (`BeliefRef`, `RecordRef`, `ObservationRef`, `PlanAssumptionRef`) into per-tick values via the per-tick belief store.

## SystemFn Integration

No new `SystemFn`. Emission happens at the same call sites S110 already established, plus one local reorder inside `emit_plan_selection_events` (D5).

## Component Registration

No new components. Decision events live in the event log, not in ECS.

## Cross-System Interactions

- **AI → Core**: emission-site code reads from already-in-scope ranking outcome, failure inputs, and frame assumptions, populates the extended payload, emits through the existing event-log path.
- **AI → Core**: the `RankedGoalComparisonDimension → RankedGoalComparisonDimensionTag` conversion lives at the AI emission site that populates `RejectedAlternativeSummary.rejection_dimension`; the core enum remains the stored event-log shape.
- **Sim → CLI**: observer replays the event log and renders the new payload fields as single-line summary suffixes. No new cross-system call.

## Profile-Driven Parameters

`cognitive.decision_history_alternatives: u8` (`crates/worldwake-core/src/cognitive_profile.rs:105`, default 5) is the per-agent soft cap for both the existing `rejected_alternatives` Vec and the new `decisive_beliefs/records/world_observations` and `assumptions` Vecs. No new profile parameter introduced.

## Validation and Falsification

- **Golden coverage**: new `golden_decision_payload.rs` with four scenarios:
  1. Eat-vs-Drink contested commit → assert `GoalCommittedPayload.rejected_alternatives` contains Drink with the correct `score_gap` AND `rejection_dimension == Some(MotiveScore)`. After the D5 reorder, also assert the same payload's `assumptions` is non-empty (contains at least the inherited `NeedSafeUntilTick` assumption).
  2. Stale-belief replan → assert `ReplanTriggeredPayload.decisive_beliefs` names the contradicted claim with `BeliefStatusTag::Stale`, and `assumptions` names the active frame's assumption set.
  3. Assumption breach → assert `ExpectationMismatchPayload.assumptions` names the breached `FrameAssumption::CommodityAvailableAt` from S122, and `decisive_world_observations` names the post-arrival observation that contradicted it.
  4. Source-expectation failure → assert `SourceExpectationFailurePayload.decisive_world_observations` names the source-attribution input; `decisive_beliefs` and `decisive_records` remain empty for the current seam unless a future implementation-time reassessment finds lawful typed carriers (no `assumptions` field — by D4).
- **Replay parity**: current-format saves replay without behavioral divergence. Pre-S136 v69 saves are rejected after the v70 bump, consistent with the no-backward-compatibility rule.
- **Bounded payload size**: deterministic fixed-seed sweep through the existing `soak_seed_perf` harness asserting per-event payload size never exceeds a per-tag byte ceiling under the canonical scenarios. (Property-based scenario generation is not part of the workspace today; an earlier draft mentioned it inaccurately.)
- **Single-line invariant**: the existing test `decision_payload_summary_is_single_line_for_goal_committed` (`observer.rs:5744`) is extended to cover the failure-path tags whose summaries S136 widens; format must remain single-line.

## Risks

- **Event log size growth.** Four `Vec<T>` fields per failure event plus one `Vec<PlanAssumptionRef>` field per success event grow per-event byte size. Mitigation: the existing per-agent `cognitive.decision_history_alternatives` cap (default 5) bounds each Vec; the soak-harness sweep measures actual footprint pre-merge. The estimated worst-case per-tag growth (with cap=5):
  - `GoalCommittedPayload`: +5 × `sizeof(PlanAssumptionRef)` (assumptions); existing field absorbs `rejection_dimension` widening (+ 0–1 byte per alt).
  - `BlockerRecordedPayload` / `ReplanTriggeredPayload` / `ExpectationMismatchPayload`: +5 × (sizeof(BeliefRef) + sizeof(RecordRef) + sizeof(ObservationRef)) + 5 × sizeof(PlanAssumptionRef).
  - `SourceExpectationFailurePayload`: +5 × (sizeof(BeliefRef) + sizeof(RecordRef) + sizeof(ObservationRef)).
  Under typical scenarios (0–2 decisive items per event), the actual size growth is far below the worst case.
- **D5 reorder behavioral risk.** Moving `update_frame_for_adopted_plan` and `populate_assumptions` ahead of `emit_plan_selection_events` must not change the inputs to `populate_assumptions` (the `refreshed_view` and tick are unchanged) or alter the post-emission state mutations. The golden test in scenario 1 above and the existing planning-flow goldens guard against regression.
- **Decisive-evidence classifier scope.** The failure-path classifier is mechanical because each failure site already has the failed-claim/observation set as a function input. Extending decisive classification to the success path (`GoalCommitted`/`PlanAdopted`) would require new computation and is deliberately deferred (Non-Goal). If a follow-on spec adds the success-path classifier, S136's failure-path machinery is the precedent.
