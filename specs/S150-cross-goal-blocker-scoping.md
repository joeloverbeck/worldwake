# S150: Cross-Goal Blocker Scoping

**Status**: Draft

## Summary

PR-12 (Environmental blocker patterns) from `reports/ai-architecture-improvements.md` proposes broadening `BlockerScope` so blockers can attach to environmental facts that affect multiple goals, not just per-(goal, place, target, action) tuples. The current `BlockerKey` at `crates/worldwake-ai/src/blocker_memory.rs:11-16` carries `goal_key: GoalKey, place: Option<EntityId>, target: Option<EntityId>, action_def: Option<ActionDefId>`. The `goal_key` is always set; cross-goal blockers ("this place is dangerous regardless of goal," "this counterparty refuses everyone") cannot be represented without inserting one entry per goal-kind.

The S109 typed `Discrepancy` taxonomy (archived) and `BlockerMemory` / `DiscrepancyMemory` split provide the right substrate for adding new scope shapes. S150 introduces `BlockerScope` as a typed enum that supersedes the flat `BlockerKey` shape for new scopes. Existing `BlockerKey`-style entries continue to live in `BlockerMemory` under `BlockerScope::Exact(BlockerKey)` (a wrapper variant that preserves all current behavior); new scopes (`RouteSegment`, `Counterparty`, `ResourceAtPlace`) layer on top.

The scope-down (per triage): ship `RouteSegment` and `Counterparty` only in this spec. `Facility(EntityId)` (already largely covered by `BlockerMemory::ExclusiveFacilityUnavailable`), `LegalAuthority { office, jurisdiction }`, and `ResourceAtPlace` are deferred until specific bug patterns surface. The two shipped scopes are the highest-impact for dense world emergence: route blockers (PR-12 lists travel, trade, patrol, escort, bounty pursuit as affected) and counterparty blockers (affects trade, ask-witness, contract negotiation).

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-ai` — extends `BlockerMemory` / `BlockerScope` types; updates blocker-lookup paths in candidate generation, feasibility checks, and ranking damping.
- `worldwake-core` — exposes `BlockerScope` enum and `RouteSegment` newtype.
- `worldwake-sim` — no change.
- `worldwake-systems` — no change.
- `worldwake-cli` — observer renders typed blocker scopes; S144 diagnostics aggregate per-scope blocker counts.

## Dependencies

- S109 (Typed Discrepancy Taxonomy, archived, hard dep) — provides `BlockerMemory` substrate.
- S110 (Decision History Events, archived) — `BlockerRecorded` event carries the new scope.
- S151 (Testimony Reliability and Route Preferences, Phase 12) — `RouteSegment` blockers compose with `RoutePreference` for cross-system route reasoning. Soft dep; S150 ships independently with usable defaults if S151 isn't yet implemented.

## Design Goals

1. **Cross-goal blockers without per-goal duplication.** A `RouteSegment` blocker affects every goal whose plan traverses the segment.
2. **Existing blocker behavior preserved.** `BlockerScope::Exact(BlockerKey)` wraps the current `BlockerKey` shape; all current call sites work unchanged after migration.
3. **Typed scope dispatch.** Blocker lookup matches per scope variant; no string-matching, no goal-key wildcarding hacks.
4. **Bounded blocker memory.** Per-scope TTLs prevent blocker accumulation.
5. **Inspectable from observer.** Each scope variant renders distinctly in observer Section 7.

## Non-Goals

- **No new authoritative world state.** Blockers are agent-local learned memory per FND-22A.
- **No `Facility`, `LegalAuthority`, `ResourceAtPlace` scopes.** Deferred per triage scope-down.
- **No cross-agent blocker propagation.** Each agent's blocker memory is per-agent (an agent telling another about a route blocker happens through the ShareBelief/Tell substrate, not through blocker-memory copy).
- **No new event tag.** Existing `BlockerRecorded` / `BlockerCleared` events carry the typed scope.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | Blockers reference concrete entities (route segments, counterparties); no abstract scoring. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | Route blockers come from local observation (a witnessed route attack, a failed traversal); counterparty blockers come from local observation (a witnessed refusal). |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | `BlockerScope` is per-agent concrete learned state with accountable origin, decay, and clearing. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Blocker reads inform candidate / feasibility / ranking; no system mutates another. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | `BlockerKey` is wrapped (`BlockerScope::Exact`) only as a containment, not a shim; all internal call sites move to `BlockerScope`. |
| FND-29 (Debuggability Is a Product Feature) | Typed scope render distinctly in observer; per-scope blocker counts surface in S144 diagnostics. |

## Deliverables

### D1: `BlockerScope` enum

```rust
// crates/worldwake-core/src/blocker_scope.rs (new)
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
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

`RouteSegment` is canonically ordered (from <= to) so `(A, B)` and `(B, A)` are the same segment.

### D2: `BlockerMemory` migration

```rust
// crates/worldwake-ai/src/blocker_memory.rs (extended)
pub struct BlockerMemory {
    entries: BTreeMap<BlockerScope, BlockerEntry>,
}

pub struct BlockerEntry {
    pub fact: BlockingFact,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub clearing_condition: BlockerClearingCondition,
    pub baseline_snapshot: Option<ClearingBaseline>,
    pub source_event: EventId,
}
```

The map key changes from `BlockerKey` to `BlockerScope`. The current `BlockerKey`-keyed entries become `BlockerScope::Exact(BlockerKey)` entries. Lookup paths use scope-matching.

### D3: Blocker lookup paths

Three call sites consume blocker memory today:

1. **Candidate generation suppression** (`crates/worldwake-ai/src/candidate_generation.rs` `emit_*` functions). When a candidate's route would traverse a `RouteSegment` blocker or its target equals a `Counterparty` blocker, the candidate is suppressed with `SuppressionReason::BlockerMatched` (per S144's D4).
2. **Feasibility probe** (`crates/worldwake-ai/src/feasibility_probe.rs`). When a candidate's required route or counterparty has a matching blocker, the probe records `Infeasible(BlockerScope)`.
3. **Ranking damping** (`crates/worldwake-ai/src/ranking.rs`). Blocker-recent goals receive damping proportional to blocker freshness.

Each call site gets a helper:

```rust
impl BlockerMemory {
    pub fn route_segment_blocked(&self, from: EntityId, to: EntityId, tick: Tick) -> Option<&BlockerEntry>;
    pub fn counterparty_blocked(&self, other: EntityId, tick: Tick) -> Option<&BlockerEntry>;
    pub fn any_blocker_on_path(&self, path: &[EntityId], tick: Tick) -> Option<&BlockerEntry>;
}
```

### D4: Blocker recording paths

Two paths record `RouteSegment` and `Counterparty` blockers:

**RouteSegment blockers**:
- Travel-failure outcomes (`TravelTo` action failing because of hazard observation, ambush, blocked terrain).
- Witnessed dangerous traversal events (S139-substrate testimony about route hazards).
- Boundary disruption observations (S62 substrate when shipped — soft dep, not required for S150).

**Counterparty blockers**:
- `MatchOutcome::NoWillingCounterparty` outcomes from trade actions (existing in S109).
- `ExpectationMismatch::PartyDeclined` from Tell/AskWitness (S139 substrate).
- Witnessed refusal events.

The recording paths produce typed `Discrepancy` instances via S109 (`Discrepancy::RouteUnknown` for route observations, `Discrepancy::NoWillingCounterparty` for counterparty observations) and write `BlockerEntry` into the per-scope `BlockerMemory`.

### D5: TTL profile extension

`CognitiveProfile` gains:

```rust
pub route_segment_blocker_ticks: u32,    // default 240
pub counterparty_blocker_ticks: u32,     // default 360
```

Per-scope TTL because route conditions change faster than personality grudges. Per FND-22A (concrete decay policy).

### D6: Blocker clearing conditions

`BlockerClearingCondition` (existing) gains scope-aware variants:

```rust
pub enum BlockerClearingCondition {
    TtlExpiry,                              // existing
    NewObservation(BeliefPredicate),        // existing — extended
    RouteRetraversedSafely(RouteSegment),   // new
    CounterpartyAccepted(EntityId),         // new
}
```

The new variants clear the corresponding scope on lawful witnessing.

### D7: Decision-history surface

S109's `BlockerRecorded` event payload (`crates/worldwake-core/src/decision_event_payload.rs`) gains `scope: BlockerScope`. Observer Section 3b renders typed scope per decision:
```
Blocker: RouteSegment(Thornwall ↔ Ashford) — DangerTooHigh — observed tick 1247, expires 1487
Blocker: Counterparty(Merchant#42) — NoWillingCounterparty — observed tick 1310, expires 1670
```

### D8: S144 diagnostics extension

`ScenarioDiagnosticsReport.belief.blocker_counts_by_scope`:
```rust
pub blocker_counts_by_scope: BTreeMap<BlockerScopeVariantId, u64>,
```

Three variants tracked: `Exact`, `RouteSegment`, `Counterparty`.

### D9: Migration tests

- All existing blocker-related goldens regress unchanged (because they exercise `BlockerScope::Exact`).
- New unit tests for `BlockerMemory::route_segment_blocked` / `counterparty_blocked` / `any_blocker_on_path`.

### D10: Golden coverage

`golden_cross_goal_blocker_scoping.rs` covers:
- RouteSegment blocker suppresses both `AcquireCommodity` (travel-trade) and `EscortToSafety` (travel-escort) candidates affecting the same segment.
- Counterparty blocker suppresses both `BuyCommodity` and `AskWitness` candidates targeting the same agent.
- TTL expiry restores candidate emission.
- `RouteRetraversedSafely` clearing condition fires on safe traversal observation.
- `CounterpartyAccepted` clearing fires on successful interaction.
- Determinism: same blocker recording sequence → identical lookup behavior.

## FND-01 Section H Analysis

### Information-Path Analysis

`RouteSegment` blockers come from:
- Direct travel-failure observation (existing perception).
- Witnessed danger events via S139 testimony.
- Boundary disruption observations (future S62 substrate; not required for S150).

`Counterparty` blockers come from:
- Direct trade-failure observation.
- Witnessed refusal events via S139 testimony.

All paths reach the agent through existing perception or testimony carriers per FND-15. No global truth queried.

### Positive-Feedback Analysis

Potential loop: blocker accumulation slows agent → more failed plans → more blockers. Dampened by D5 (TTL expiry) and D6 (clearing conditions on safe witnessing).

### Concrete Dampeners

- `route_segment_blocker_ticks` and `counterparty_blocker_ticks` TTL expiry.
- `RouteRetraversedSafely` and `CounterpartyAccepted` clearing conditions.

Both are concrete world processes per FND-11: time-bounded memory + explicit invalidating observation.

### Stored State vs. Derived Read-Model List

**Stored state**:
- `BlockerMemory` keyed by `BlockerScope` (per-agent, authoritative learned memory).
- `route_segment_blocker_ticks` / `counterparty_blocker_ticks` on `CognitiveProfile`.

**Derived read-model**:
- Per-tick blocker lookups for candidate / feasibility / ranking.

## SystemFn Integration

No new top-level `SystemFn`. Blocker recording and lookup live inside the existing agent-tick decision pipeline.

## Component Registration

No new ECS component. `BlockerMemory` already lives on the agent's runtime AI state.

## Cross-System Interactions

- Candidate generation reads blocker memory (existing pattern, extended scope).
- Feasibility probe reads blocker memory (existing pattern).
- Ranking damping reads blocker memory (existing pattern).
- Observer reads typed scope through S110 decision-event payload (existing path, extended payload).

State-mediated per FND-26.

## Profile-Driven Parameters

- `CognitiveProfile.route_segment_blocker_ticks` (u32, default 240).
- `CognitiveProfile.counterparty_blocker_ticks` (u32, default 360).

Both are concrete tick counts. Profile-driven per FND-22A.

## Test Plan

- D10 golden coverage (6 scenarios).
- D9 migration regression: existing blocker goldens unchanged.
- Determinism unit tests on `BlockerScope` ordering and lookup.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
