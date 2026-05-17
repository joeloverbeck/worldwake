# S150: Cross-Goal Blocker Scoping

**Status**: Draft

## Summary

PR-12 (Environmental blocker patterns) from `reports/ai-architecture-improvements.md` proposes broadening `BlockerScope` so blockers can attach to environmental facts that affect multiple goals, not just per-(goal, place, target, action) tuples. The current `BlockerKey` at `crates/worldwake-core/src/blocker_memory.rs:10-16` carries `goal_key: GoalKey, place: Option<EntityId>, target: Option<EntityId>, action_def: Option<ActionDefId>`. The `goal_key` is always set; cross-goal blockers ("this place is dangerous regardless of goal," "this counterparty refuses everyone") cannot be represented without inserting one entry per goal-kind.

The S109 typed `Discrepancy` taxonomy (archived) and `BlockerMemory` / `DiscrepancyMemory` split provide the right substrate for adding new scope shapes. S150 introduces `BlockerScope` as a typed enum that supersedes the flat `BlockerKey` shape for new scopes. Existing `BlockerKey`-style entries continue to live in `BlockerMemory` under `BlockerScope::Exact(BlockerKey)` (a single-truth wrapper, not a shim — `BlockerKey` is contained inside `BlockerScope`, never coexisting as a parallel live key). `DiscrepancyMemory` migrates to `BlockerScope` keys uniformly with `BlockerMemory`, preserving the parallel-substrate symmetry so cross-goal failure attribution applies to both stores. New scopes (`RouteSegment`, `Counterparty`) layer on top.

The scope-down (per triage): ship `RouteSegment` and `Counterparty` only in this spec. `Facility(EntityId)` (already covered by `BlockingFact::ExclusiveFacilityUnavailable` at `blocker_memory.rs:181-186`, where the existing `blocks_goal_generation() != Self::ExclusiveFacilityUnavailable` carve-out gives facility-scoped suppression without a dedicated scope variant), `LegalAuthority { office, jurisdiction }`, and `ResourceAtPlace` are deferred until specific bug patterns surface. The two shipped scopes are the highest-impact for dense world emergence: route blockers (PR-12 lists travel, trade, patrol, escort, bounty pursuit as affected) and counterparty blockers (affects trade, ask-witness, contract negotiation).

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-core` — owns the substrate. Adds `BlockerScope` enum and `RouteSegment` newtype in a new `blocker_scope.rs`; migrates `BlockerMemory` and `DiscrepancyMemory` from `BlockerKey` to `BlockerScope` keys; extends `CognitiveProfile` with per-scope TTL fields; extends `BlockerClearingCondition` with two new variants; extends `BlockerRecordedPayload` with `scope: BlockerScope`.
- `worldwake-ai` — consumer-side updates. Three runtime read sites (`candidate_generation.rs`, `feasibility_probe.rs`, `search/candidates.rs`) consult the new scope-aware lookups; three recording sites (`agent_tick/execution.rs`, `agent_tick/observation.rs`, `failure_handling.rs`) attribute originating `EventId`; `scenario_diagnostics/mod.rs` adds `BlockerScopeVariantId` and the per-scope histogram.
- `worldwake-sim` — `save_load.rs` updates the serialization paths to round-trip `BlockerScope`-keyed entries.
- `worldwake-systems` — `trade_actions.rs` records counterparty blockers via the new `BlockerScope::Counterparty` recording path.
- `worldwake-cli` — observer Section 3b (Decision History) renders typed blocker scopes; S144 diagnostics output (Section 13) aggregates per-scope blocker counts.

## Dependencies

- S109 (Typed Discrepancy Taxonomy, archived at `archive/specs/S109-typed-discrepancy-taxonomy.md`, hard dep) — provides `BlockerMemory` / `DiscrepancyMemory` split substrate and the `Discrepancy::RouteUnknown` / `Discrepancy::NoWillingCounterparty` variants S150 emits during recording.
- S110 (Decision History Events, archived at `archive/specs/S110-decision-history-events.md`, hard dep) — provides the `BlockerRecorded` event tag (`event_tag.rs:108`) and `BlockerRecordedPayload` (`decision_event_payload.rs:477-493`) that this spec extends with `scope: BlockerScope`.
- S139 (Ask-Witness Goal Layer, archived at `archive/specs/S139-epistemic-sensing-subgoals.md`, hard dep) — provides the testimony substrate (Tell, AskWitness, witness-event chains) that S150's `Counterparty` and `RouteSegment` recording paths consume when blockers come from witnessed events rather than direct observation.
- S144 (Aggregate Scenario Diagnostics, archived at `archive/specs/S144-aggregate-scenario-diagnostics.md`, hard dep) — provides `ScenarioDiagnosticsReport` / `BeliefMetrics` and the `CandidateSuppressionCategory::RejectedSuppressedByBlocker` aggregation key. S150 extends `BeliefMetrics` with `blocker_counts_by_scope`.
- S151 (Testimony Reliability and Route Preferences, Phase 12) — `RouteSegment` blockers compose with `RoutePreference` for cross-system route reasoning. Soft dep; S150 ships independently with usable defaults if S151 is not yet implemented.

S146-S149 are explicitly independent of S150 (verified during reassessment): none of them touch `BlockerMemory`, `BlockerKey`, `BlockerClearingCondition`, or the blocker read/record call sites; none add `GoalKind` variants that would enlarge the `BlockerKey.goal_key` surface in a way that conflicts with `BlockerScope::Exact` wrapping; none modify `CognitiveProfile` fields that overlap S150's two new TTL fields. S150 sits in Wave 1 of Phase 12 per `specs/IMPLEMENTATION-ORDER.md` and can ship before any of S146-S149.

## Design Goals

1. **Cross-goal blockers without per-goal duplication.** A `RouteSegment` blocker affects every goal whose plan traverses the segment.
2. **Existing blocker behavior preserved through containment.** `BlockerScope::Exact(BlockerKey)` wraps the current `BlockerKey` shape; all current behavior continues to work after migration. Per FND-28 this is a single-truth wrapper, not a coexistence shim — no live code reads `BlockerKey` outside of `BlockerScope::Exact` after migration.
3. **Typed scope dispatch.** Blocker lookup matches per scope variant; no string-matching, no goal-key wildcarding hacks.
4. **Bounded blocker memory.** Per-scope TTLs prevent blocker accumulation.
5. **Inspectable from observer.** Each scope variant renders distinctly in observer Section 3b (Decision History).

## Non-Goals

- **No new authoritative world state.** Blockers are agent-local learned memory per FND-22A.
- **No `Facility`, `LegalAuthority`, `ResourceAtPlace` scopes.** Facility-scope suppression is already served by `BlockingFact::ExclusiveFacilityUnavailable` plus the carve-out in `Blocker::blocks_goal_generation` at `blocker_memory.rs:181-186`; introducing a dedicated `Facility` scope would duplicate the substrate. `LegalAuthority` and `ResourceAtPlace` await concrete failure-pattern motivation.
- **No cross-agent blocker propagation.** Each agent's blocker memory is per-agent (an agent telling another about a route blocker happens through the ShareBelief/Tell substrate, not through blocker-memory copy).
- **No new event tag.** Only `EventTag::BlockerRecorded` exists today (`event_tag.rs:108`); S150 extends its payload with the typed scope but does not introduce a `BlockerCleared` tag. Clearing is observable via TTL expiry, scope-aware clearing conditions (D6), and supersession via the next `BlockerRecorded` event for the same scope.
- **No blocker-aware ranking damping.** Blockers remain boolean suppression gates per FND-3 (Concrete State Over Abstract Scores); turning blocker freshness into a ranking signal would blur the gate/scoring separation. If a future need surfaces, it would be a separate spec, not a hidden extension of S150.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | Blockers reference concrete entities (route segments, counterparties); no abstract scoring. Blockers remain boolean gates; ranking does not consume blocker freshness. |
| FND-15 (Knowledge Acquired Locally and Travels Physically) | Route blockers come from local observation (witnessed route attack, failed traversal) or via S139 testimony (witnessed danger events); counterparty blockers come from local observation (witnessed refusal) or via S139 testimony. The new `source_event: EventId` field on `Blocker` preserves the carrier-event link for provenance reconstruction. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | `BlockerScope` is per-agent concrete learned state with accountable origin (`source_event: EventId`), decay (per-scope TTLs), and explicit clearing (scope-aware clearing conditions). |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Blocker reads inform candidate / feasibility / search-expansion; no system mutates another. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | `BlockerKey` is contained inside `BlockerScope::Exact`, not preserved alongside it — there is one live key type (`BlockerScope`) post-migration. All 181 `BlockerKey` reference sites across 27 files migrate in one pass; no shim, no dual-truth coexistence. |
| FND-29 (Debuggability Is a Product Feature) | Typed scope renders distinctly in observer Section 3b; per-scope blocker counts surface in S144 diagnostics; `source_event` field enables direct event-log lookup of "why is this blocker here". |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | `source_event: EventId` on `Blocker` and `DiscrepancyEntry` links live learned state back to its originating event in the append-only log. |

## Deliverables

### D1: `BlockerScope` enum and `RouteSegment` newtype

```rust
// crates/worldwake-core/src/blocker_scope.rs (new)
use crate::{BlockerKey, EntityId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum BlockerScope {
    Exact(BlockerKey),
    RouteSegment(RouteSegment),
    Counterparty(EntityId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct RouteSegment {
    pub from: EntityId,
    pub to: EntityId,
}

impl RouteSegment {
    pub fn new(from: EntityId, to: EntityId) -> Self {
        // Canonical ordering for undirected segments
        if from <= to { Self { from, to } } else { Self { from: to, to: from } }
    }
}
```

`RouteSegment` is canonically ordered (from <= to) so `(A, B)` and `(B, A)` are the same segment. Derive set matches `BlockerKey` (`blocker_memory.rs:10`) so `BlockerScope` satisfies the `Copy + Ord + Hash + Serialize` bounds the existing memory stores require.

### D2: Memory substrate migration (`BlockerMemory` and `DiscrepancyMemory`)

Both authoritative learned-state stores migrate from `BlockerKey` to `BlockerScope` keys uniformly. This preserves FND-28 single-truth (one live key type for failure attribution) and the FND-22A symmetry between blocker memory ("don't attempt this") and discrepancy memory ("don't retry this").

**`BlockerMemory`** (`crates/worldwake-core/src/blocker_memory.rs`):

```rust
pub struct BlockerMemory {
    pub intents: BTreeMap<BlockerScope, Blocker>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Blocker {
    pub scope: BlockerScope,                          // replaces blocker_key: BlockerKey
    pub blocking_fact: BlockingFact,
    pub diagnostic_context: Option<BlockerDiagnostic>,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub clearing_condition: BlockerClearingCondition,
    pub baseline_snapshot: Option<ClearingBaseline>,
    pub source_event: EventId,                        // NEW — origin event for FND-22A accountable origin
}
```

The map key changes from `BlockerKey` to `BlockerScope`. The current `BlockerKey`-keyed entries become `BlockerScope::Exact(BlockerKey)` entries. The per-struct `blocker_key` field is replaced with `scope` (single source of truth — no duplication between map key and struct field). The struct name stays `Blocker` (preserving naming convention; eliminates rename churn across 27 files). `diagnostic_context` is preserved unchanged. `source_event` is a new field populated by the recording sites (see D4).

**`DiscrepancyMemory`** (`crates/worldwake-core/src/discrepancy.rs`):

```rust
pub struct DiscrepancyMemory {
    pub entries: BTreeMap<BlockerScope, DiscrepancyEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiscrepancyEntry {
    pub scope: BlockerScope,                          // replaces blocker_key: BlockerKey
    pub discrepancy: Discrepancy,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub clearing_condition: DiscrepancyClearing,
    pub source_event: EventId,                        // NEW — symmetric with Blocker
}
```

The existing methods (`record`, `expire`, `is_suppressed`, `clear_for`, `clear_by_condition`) update their signatures from `&BlockerKey` to `&BlockerScope`. `DiscrepancyClearing` keeps its existing variants unchanged; clearing for the new scopes uses the same patterns (`ReobservationOf { target }`, `BeliefUpdate { claim_key }`, etc.).

### D3: Blocker lookup paths (three runtime read sites)

Three call sites consume blocker memory today:

1. **Candidate generation suppression** (`crates/worldwake-ai/src/candidate_generation.rs:759` `is_blocked(goal_key, place, target, action_def, current_tick)`). When a candidate's route would traverse a `RouteSegment` blocker or its target equals a `Counterparty` blocker, the candidate is suppressed by the new scope-aware lookup. The downstream S144 aggregator counts the suppression under `CandidateSuppressionCategory::RejectedSuppressedByBlocker` (`scenario_diagnostics/mod.rs:94`) — the live emission gate stays at the candidate-generation site; the suppression category is the post-hoc aggregation key, not a runtime suppression reason.
2. **Feasibility probe** (`crates/worldwake-ai/src/feasibility_probe.rs:42`). When a candidate's required route or counterparty has a matching blocker, the probe records the rejection via the existing `FeasibilityVerdict::RejectedBeforeSearch` path, now scope-aware.
3. **Search candidate filtering** (`crates/worldwake-ai/src/search/candidates.rs:1336` `find_blocked_for_search`). This third reader ignores the `blocks_goal_generation()` gate so it suppresses search successors even for facts (like `SourceDepleted`) that allow goal generation. Scope-aware matching is added here for path-spanning lookups.

Each site gets a helper:

```rust
impl BlockerMemory {
    pub fn route_segment_blocked(&self, from: EntityId, to: EntityId, tick: Tick) -> Option<&Blocker>;
    pub fn counterparty_blocked(&self, other: EntityId, tick: Tick) -> Option<&Blocker>;
    pub fn any_blocker_on_path(&self, path: &[EntityId], tick: Tick) -> Option<&Blocker>;
}
```

The existing `is_blocked` / `is_blocked_for_search` / `find_blocked_for_search` signatures continue to serve `BlockerScope::Exact` lookups; the new helpers compose them with `RouteSegment` and `Counterparty` matching.

**Migration blast radius** (per "Existing Variant Payload Widening" pattern): `BlockerKey` has 181 references across 27 files in 5 crates. Migration sites:

- `worldwake-core/src/blocker_memory.rs` (substrate; method signatures `is_blocked`, `is_blocked_for_search`, `find_blocked_for_search`, `record`, `clear_for`, `clear_all_for_goal`)
- `worldwake-core/src/discrepancy.rs` (parallel substrate; method signatures `record`, `is_suppressed`, `clear_for`)
- `worldwake-core/src/decision_event_payload.rs` (`BlockerRecordedPayload.blocker_key` → `BlockerRecordedPayload.scope`)
- `worldwake-core/src/test_utils.rs` (sample helpers updated)
- `worldwake-ai/src/agenda_manager.rs`, `agent_tick/{execution,observation,planning,frame,candidates,tests}.rs`, `candidate_generation.rs`, `decision_trace.rs`, `failure_handling.rs`, `feasibility.rs`, `feasibility_probe.rs`, `search/{candidates,tests}.rs`, `survival_forensics.rs`
- `worldwake-ai/tests/{golden_contention_inspectability,golden_need_projection,golden_plan_repair,golden_portfolio_planning}.rs` and `golden_harness/need_projection_assertions.rs`
- `worldwake-sim/src/save_load.rs` (serialization round-trip)
- `worldwake-systems/src/trade_actions.rs:1920-1930` (NoBuyer blocker recording)
- `worldwake-cli/src/bin/observer.rs` (observer rendering of Section 3b decision history)

Sites that currently construct `BlockerKey { goal_key, place, target, action_def }` continue to produce `BlockerScope::Exact(BlockerKey { … })` (goal-keyed scope is the dominant case). New `RouteSegment` and `Counterparty` constructions happen only at the recording sites enumerated in D4.

### D4: Blocker recording paths and `source_event` capture

Three runtime sites currently record blockers — each must capture an originating `EventId` for the new `source_event` field per FND-22A's "accountable origin" requirement:

1. **`crates/worldwake-ai/src/agent_tick/execution.rs:1341`** — reservation-conflict and soft-facility blockers recorded during action execution. Source event: the `ContentionEvent` or `ActionAbortedEvent` whose payload triggered the recording. Capture from the surrounding tick context (`ctx.last_emitted_event_id` or equivalent).
2. **`crates/worldwake-ai/src/agent_tick/observation.rs:626`** — blockers recorded from same-tick perception observations (e.g., `DangerTooHigh` from a witnessed hazard). Source event: the originating `PerceptionEvent` ID.
3. **`crates/worldwake-ai/src/failure_handling.rs:224`** — blockers recorded when a plan step fails terminal validation. Source event: the `ExpectationMismatchPayload` event or `SourceExpectationFailurePayload` event that triggered failure handling.

`RouteSegment` blocker recording paths:

- Travel-failure outcomes (`TravelTo` action failing because of hazard observation, ambush, blocked terrain) — source event: the failing-action commit event.
- Witnessed dangerous traversal events (S139 testimony chains relayed through `ExpectationMismatch::PartyDeclined` or witness-event payloads) — source event: the inbound testimony event.
- Boundary disruption observations (S62 substrate when shipped — soft dep, not required for S150) — source event: the boundary-event ID.

`Counterparty` blocker recording paths:

- `BlockingFact::NoBuyer` outcomes from `crates/worldwake-systems/src/trade_actions.rs:1920-1930` (the existing `NoBuyer` BlockingFact already captures the counterparty entity; the recording is rewritten to use `BlockerScope::Counterparty(counterparty_id)` instead of `BlockerScope::Exact(BlockerKey { … })`). Source event: the failed-trade commit event.
- `ExpectationMismatch::PartyDeclined` from Tell/AskWitness (S139 substrate) — source event: the `ExpectationMismatchPayload` event.
- Witnessed refusal events — source event: the witness perception event.

The recording paths produce typed `Discrepancy` instances via S109 (`Discrepancy::RouteUnknown` for route observations, `Discrepancy::NoWillingCounterparty` for counterparty observations) and write `Blocker` into the per-scope `BlockerMemory`. The `DiscrepancyMemory` parallel write follows the same `source_event` capture pattern.

### D5: Per-scope TTL fields on `CognitiveProfile`

`CognitiveProfile` (`crates/worldwake-core/src/cognitive_profile.rs`) gains two new fields following the existing per-discrepancy backoff pattern (`route_unknown_backoff_ticks` at line 65, `counterparty_refusal_backoff_ticks` at line 62):

```rust
/// Ticks before a RouteSegment-scoped blocker expires under TtlOnly clearing.
#[serde(default = "default_route_segment_blocker_ticks")]
pub route_segment_blocker_ticks: u32,
/// Ticks before a Counterparty-scoped blocker expires under TtlOnly clearing.
#[serde(default = "default_counterparty_blocker_ticks")]
pub counterparty_blocker_ticks: u32,
```

With matching const-fn defaults sited alongside the existing TTL helpers (`cognitive_profile.rs:195-253`):

```rust
const fn default_route_segment_blocker_ticks() -> u32 {
    // Same order as default_route_unknown_backoff_ticks (200); slight inflation because
    // blockers suppress generation entirely, not just retry. Route conditions change
    // through traversal evidence within a few hundred ticks.
    240
}

const fn default_counterparty_blocker_ticks() -> u32 {
    // Counterparty refusal is more durable than transient unwillingness — once a refusal
    // is observed, the agent should give the counterparty time to revise. Compare to
    // default_counterparty_refusal_backoff_ticks (40, which is per-discrepancy retry);
    // the blocker TTL is the longer "give them time" outer envelope.
    360
}
```

Both new fields receive `Default` impl entries in lockstep (`cognitive_profile.rs:123-163`). The `cognitive_profile_default_matches_split_defaults` and `cognitive_profile_deserialization_defaults_*` tests gain corresponding assertions. Per FND-2, the defaults' rationale is captured in the doc comments above; per FND-22A, both decay are concrete tick counts (no abstract scores).

### D6: New scope-aware `BlockerClearingCondition` variants

`BlockerClearingCondition` (`crates/worldwake-core/src/blocker_memory.rs:131-155`) currently has eight variants — seven fact-specific clearing conditions plus `TtlOnly`:

```rust
// Current shape (blocker_memory.rs:131-155)
pub enum BlockerClearingCondition {
    CommodityAvailabilityChanged { commodity: CommodityKind, place: EntityId },
    InventoryChanged { commodity: CommodityKind },
    UniqueItemAcquired { kind: UniqueItemKind },
    PathDiscovered { destination: EntityId },
    EntityReappeared { entity: EntityId },
    DangerReduced { place: EntityId },
    ContentionChanged { facility: EntityId },
    TtlOnly,
}
```

Two new fact-specific variants are added alongside the existing seven:

```rust
RouteRetraversedSafely(RouteSegment),
CounterpartyAccepted(EntityId),
```

The new variants clear `RouteSegment`-scoped and `Counterparty`-scoped blockers respectively when the agent witnesses a contradicting observation (safe traversal of the segment; successful interaction with the counterparty). The enum derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize` (existing); `RouteSegment` and `EntityId` both satisfy `Copy`, so the additions preserve the derive bounds.

### D7: Decision-history surface — `BlockerRecordedPayload` extension and Section 3b rendering

S109's `BlockerRecorded` event payload (`crates/worldwake-core/src/decision_event_payload.rs:476-493`) gains `scope: BlockerScope` alongside its existing `blocker_key: BlockerKey` field. During the migration window the new field is populated for every emission; the legacy `blocker_key` is then either eliminated (if no replay-decoder consumer needs it) or kept as a derived view of `BlockerScope::Exact(_)` for back-compat with serialized replay state (decide during implementation — neither path is a shim, both single-truth).

Observer Section 3b "Decision History" (`crates/worldwake-cli/src/bin/observer.rs:872`) is the existing render site for `BlockerRecordedPayload`; it gains typed-scope formatting:

```
Blocker: RouteSegment(Thornwall ↔ Ashford) — DangerTooHigh — observed tick 1247, expires 1487
Blocker: Counterparty(Merchant#42) — NoWillingCounterparty — observed tick 1310, expires 1670
Blocker: Exact(Sleep at Inn) — WorkstationBusy — observed tick 1422, expires 1442
```

No new event tag is added — Non-Goal #4 (no `BlockerCleared` event). Clearing is observable through TTL expiry, the new scope-aware `BlockerClearingCondition` variants in D6, and supersession when a new `BlockerRecorded` event arrives for the same scope.

### D8: S144 diagnostics — per-scope blocker histogram

`BeliefMetrics` (`crates/worldwake-ai/src/scenario_diagnostics/mod.rs:57-63`) gains:

```rust
pub blocker_counts_by_scope: BTreeMap<BlockerScopeVariantId, u64>,
```

With the supporting discriminant enum sited alongside the existing `CandidateSuppressionCategory` precedent (`scenario_diagnostics/mod.rs:90-108`):

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum BlockerScopeVariantId {
    Exact,
    RouteSegment,
    Counterparty,
}
```

The aggregator (`scenario_diagnostics/aggregator.rs`) walks `BlockerRecorded` events from the event log, projects each `scope: BlockerScope` to its `BlockerScopeVariantId`, and increments the histogram. Three variants tracked.

### D9: Trait-bound and migration regression tests

- All existing blocker-related goldens regress unchanged (because they exercise `BlockerScope::Exact` semantics that match prior `BlockerKey` behavior).
- New unit tests for `BlockerMemory::route_segment_blocked` / `counterparty_blocked` / `any_blocker_on_path`.
- New unit tests for `DiscrepancyMemory` with `BlockerScope::RouteSegment` and `BlockerScope::Counterparty` keys.
- `BlockerScope` and `RouteSegment` join the existing trait-bound regression suite at `blocker_memory.rs:247-256` (`assert_copy_value_bounds::<BlockerScope>()`, `assert_copy_value_bounds::<RouteSegment>()`).
- Serialization roundtrips for `BlockerScope` variants and `Blocker` / `DiscrepancyEntry` with the new `source_event` field.

### D10: Golden coverage

`golden_cross_goal_blocker_scoping.rs` covers:

- RouteSegment blocker suppresses both `AcquireCommodity` (travel-trade) and `EscortToSafety` (travel-escort) candidates affecting the same segment.
- Counterparty blocker suppresses both `BuyCommodity` and `AskWitness` candidates targeting the same agent.
- TTL expiry restores candidate emission.
- `RouteRetraversedSafely` clearing condition fires on safe traversal observation.
- `CounterpartyAccepted` clearing fires on successful interaction.
- `DiscrepancyMemory` parallel suppression: `Discrepancy::RouteUnknown` keyed by `BlockerScope::RouteSegment` suppresses retries across multiple goals.
- `source_event` field is populated on every recorded blocker and points back to a real event in the log.
- Determinism: same blocker recording sequence → identical lookup behavior; `BlockerScope` ordering is stable across runs.

## FND-01 Section H Analysis

Per the (a)+(b) hybrid rule, Section H is abbreviated to cover only the declarations this spec changes. The original 18-point coverage for the blocker / discrepancy / decision-history / diagnostics surfaces is carried by S109, S110, and S144.

### Information-Path Analysis

`RouteSegment` blockers come from:
- Direct travel-failure observation (existing perception pipeline) — `source_event` = the failing-action commit event.
- Witnessed danger events via S139 testimony — `source_event` = the inbound testimony event.
- Boundary disruption observations (future S62 substrate; not required for S150).

`Counterparty` blockers come from:
- Direct trade-failure observation — `source_event` = the failed-trade commit event.
- Witnessed refusal events via S139 testimony — `source_event` = the inbound testimony event.

All paths reach the agent through existing perception or testimony carriers per FND-15. No global truth queried. The new `source_event: EventId` field preserves the carrier-event link for provenance reconstruction per FND-29A.

### Positive-Feedback Analysis

Potential loop: blocker accumulation slows agent → more failed plans → more blockers. Dampened by D5 (TTL expiry) and D6 (scope-aware clearing conditions on safe witnessing). The parallel `DiscrepancyMemory` migration does not introduce a new loop — it preserves the existing suppress-retry pattern on the same set of failures.

### Concrete Dampeners

- `route_segment_blocker_ticks` and `counterparty_blocker_ticks` TTL expiry.
- `RouteRetraversedSafely` and `CounterpartyAccepted` clearing conditions on lawful witnessing.

Both are concrete world processes per FND-11: time-bounded memory + explicit invalidating observation.

### Stored State vs. Derived Read-Model List

**Stored state**:
- `BlockerMemory` keyed by `BlockerScope` (per-agent, authoritative learned memory).
- `DiscrepancyMemory` keyed by `BlockerScope` (per-agent, authoritative learned memory; parallel to BlockerMemory).
- `route_segment_blocker_ticks` / `counterparty_blocker_ticks` on `CognitiveProfile`.
- `source_event: EventId` field on `Blocker` and `DiscrepancyEntry` (live state link to append-only event log).
- `BlockerScope`-typed `scope` field on `BlockerRecordedPayload` (the event payload itself is an authoritative event-log entry).

**Derived read-model**:
- Per-tick blocker lookups for candidate / feasibility / search-expansion (helpers `route_segment_blocked`, `counterparty_blocked`, `any_blocker_on_path`).
- `BlockerScopeVariantId` histogram in `BeliefMetrics` (computed from `BlockerRecorded` events at diagnostics-build time).

## SystemFn Integration

No new top-level `SystemFn`. Blocker recording and lookup live inside the existing agent-tick decision pipeline.

## Component Registration

No new ECS component. `BlockerMemory` and `DiscrepancyMemory` already live as components on the agent's runtime state (registered in `component_schema.rs`). The migration changes their internal map key types but does not change their component registrations.

## Cross-System Interactions

- Candidate generation reads blocker memory (`candidate_generation.rs:759`, existing pattern, extended scope).
- Feasibility probe reads blocker memory (`feasibility_probe.rs:42`, existing pattern, extended scope).
- Search candidate filtering reads blocker memory (`search/candidates.rs:1336`, existing pattern, extended scope) — this is the third runtime reader; the spec previously misattributed it to `ranking.rs`, which has zero blocker reads.
- Observer Section 3b (Decision History) reads typed scope through S110 decision-event payload (existing path, extended payload).
- S144 diagnostics aggregator (`scenario_diagnostics/aggregator.rs`) reads `BlockerRecorded` event payloads to populate `blocker_counts_by_scope`.

State-mediated per FND-26. No system calls another directly.

## Authoritative-to-AI Impact Analysis

D3 modifies the blocker suppression gates in candidate emission, feasibility probe, and search candidate filtering. Per CLAUDE.md "Authoritative-to-AI Impact Rule", the 7-point checklist applies:

1. `get_affordances` — N/A (blockers don't gate affordance enumeration).
2. `generate_candidates` — **flag** — `candidate_generation.rs:759` `is_blocked` becomes scope-aware. Implementation must enumerate which `emit_*` functions perform the per-scope check (RouteSegment for travel-bearing emitters such as `AcquireCommodity`, `EscortToSafety`, `PatrolRoute`, `BountyHunt`; Counterparty for trade/Tell emitters such as `BuyCommodity`, `AskWitness`, `ContractNegotiate`).
3. `search_plan` — **flag** — `search/candidates.rs:1336` `find_blocked_for_search` needs scope-aware matching to suppress search successors that traverse a blocked segment or target a blocked counterparty.
4. `BestEffort` action start — pass (blockers don't gate action start).
5. `handle_plan_failure` — **flag** — recording paths at `agent_tick/execution.rs:1341`, `agent_tick/observation.rs:626`, and `failure_handling.rs:224` need updates per D4 to capture `source_event`.
6. Payload revalidation — N/A (no new action payloads; existing `with_payload_override_validator` registrations are untouched).
7. Golden tests — pass per D10 (8 scenarios including DiscrepancyMemory parallel suppression and `source_event` provenance).

## Profile-Driven Parameters

- `CognitiveProfile.route_segment_blocker_ticks` (u32, default 240, `#[serde(default = "default_route_segment_blocker_ticks")]`).
- `CognitiveProfile.counterparty_blocker_ticks` (u32, default 360, `#[serde(default = "default_counterparty_blocker_ticks")]`).

Both are concrete tick counts. Profile-driven per FND-22A. Default rationale captured in D5 doc comments.

## Test Plan

- D10 golden coverage (8 scenarios).
- D9 trait-bound and migration regression tests; existing blocker goldens unchanged.
- Determinism unit tests on `BlockerScope` ordering and lookup.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo test --workspace` passing.
