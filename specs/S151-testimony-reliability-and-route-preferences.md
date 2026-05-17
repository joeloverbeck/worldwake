# S151: Testimony Source Reliability and Route Preferences

**Status**: Draft

## Summary

Folds in PR-7 (SourceReliabilityMemory for testimony) and PR-8 (RoutePreference; HabitMemory scope-down) from `reports/ai-architecture-improvements.md`.

The current `ReliabilityRecord` in `crates/worldwake-core/src/experience.rs` is narrow: it tracks `successful_acquisitions`, `failed_attempts`, `average_wait_ticks`, `wait_observation_count`, `last_available_quantity` keyed by `(commodity, purpose)` — i.e. only commodity-extraction reliability per S131/S133. It does not track testimony reliability (whether a witness was right about route hazards), accusation credibility (whether a prior accuser's claim panned out), or rumor reliability (whether reported events actually occurred).

The current `route_threat.rs` estimates dynamic route danger from witnessed/believed threats but does not track per-agent learned route preferences (which routes the agent has personally found safe vs. dangerous over time, independent of currently-active threats).

S151 lands two new per-agent learned-state stores:

1. **`TestimonyReliability`** keyed by `(EntityId, TopicScope)`: tracks per-witness reliability for each topic category. When a witness reports something and the agent later confirms it (true or false), the record updates. Used to dampen ranking on testimony from unreliable sources and to boost candidate emission from reliable ones. Enables Scenario G (false rumor → wrongful accusation → correction) by giving agents a concrete record of who has been right or wrong before.

2. **`RoutePreference`** keyed by `RouteSegment` (shared with S150): tracks per-segment safe-traversal count, dangerous-traversal count, and the last witnessed danger event. Used to bias travel candidates toward known-safe routes and away from known-dangerous ones, independent of currently-active threat beliefs.

`HabitMemory` (the assessment's broader proposal — preferred goal-schema / method / place per trigger) is **deferred** until a specific habit-relevant pathology surfaces in S144 diagnostics. The scope-down rationale: per-method habit learning has no concrete failure scenario today; the existing `LearnedOpportunityMemory` (S109) and `DiversificationProfile` (S107) cover the present needs. If S144 reports method-thrash patterns, a follow-up spec lands HabitMemory.

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-ai` — owns `TestimonyReliability` and `RoutePreference` runtime structures; updates ranking and candidate-generation paths to consume them.
- `worldwake-core` — exposes `TopicScope`, `TestimonyReliabilityKey`, `RoutePreferenceEntry` types.
- `worldwake-sim` — no change.
- `worldwake-systems` — no change.
- `worldwake-cli` — observer renders reliability and preference snapshots; S144 diagnostics aggregate reliability changes.

## Dependencies

- S109 (Typed Discrepancy Taxonomy, archived) — `Discrepancy::BeliefStale` and `Discrepancy::BeliefContradicted` carry the substrate that surfaces "this source was wrong before."
- S131 (Source Reliability Wait/Capacity, archived) and S133 (Source Composite Tiebreaker, archived) — provide the precedent for per-source learned state and `ReliabilityRecord` shape.
- S139 (AskWitness Goal Layer, archived) — provides the testimony-acquisition path that feeds testimony-reliability updates.
- S150 (Cross-Goal Blocker Scoping, archived at `archive/specs/S150-cross-goal-blocker-scoping.md`) — `RouteSegment` newtype shared; route preferences and route blockers compose (blockers say "currently bad," preferences say "historically [un]safe").
- S130 (Survey Records and Frontier Disconfirmation, archived) — provides confirmation-event substrate for testimony validation.

## Design Goals

1. **Two narrow stores, not one omnibus.** TestimonyReliability and RoutePreference are conceptually distinct and update through different paths.
2. **Per-agent learned state, not world truth.** Both stores live on the agent's AI runtime; they reflect *what this agent has learned*, not authoritative reliability scores.
3. **Concrete update events.** Every update has an `EventId` provenance — what observation produced the trust adjustment.
4. **Decay built in.** Both stores age out stale records per FND-22A.
5. **Inspectable.** Observer surfaces top reliable and unreliable sources per agent; top-preferred and avoided routes.
6. **Composable with existing systems.** Testimony reliability dampens AskWitness ranking; route preferences modify travel cost in route_threat.rs's existing infrastructure.

## Non-Goals

- **No HabitMemory.** Deferred per scope-down.
- **No SellerReliabilityMemory.** S131/S133 already cover this for commodity extraction sources; testimony reliability does not duplicate.
- **No global reputation system.** Each agent has their own reliability view.
- **No new event tag.** Updates flow through existing testimony-confirm and route-traversal events.
- **No automatic reliability propagation through gossip.** If agent A trusts witness W, A's friend B does not automatically trust W. (S139's existing ShareBelief substrate already lets agents share trust through testimony if a scenario authors it.)

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | Both stores hold concrete counts and event references, not abstract trust scores; the `trust: Permille` field is a derived view per FND-27. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | Updates come from local observation events (route traversal, testimony confirmation/refutation); no global truth read. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | Both stores are explicit concrete state with accountable origin (`EventId`), scope (per-agent), and decay (`DecayPolicy`). |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Reliability and preference are consumed by ranking and candidate generation as state reads. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The new stores live alongside existing `ReliabilityRecord`; the old (commodity-extraction) reliability path is unchanged in identity, not deprecated. |
| FND-29 (Debuggability Is a Product Feature) | Observer surfaces both stores; S144 aggregates reliability-change counts and route-preference distributions. |

## Deliverables

### D1: `TopicScope` enum

```rust
// crates/worldwake-core/src/topic_scope.rs (new)
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum TopicScope {
    RouteHazard,
    ResourceAvailability,
    OfficeHolder,
    AccusationCredibility,
    BountyValidity,
    PriceLevel,
    EntityWhereabouts,
    GeneralFact,
}
```

Seven categories matching the assessment's "expertise tags" concept. Closed enum (extending requires a spec); per FND-22A "agent-local learned summaries are legal even when abstract — they are not world truth."

### D2: `TestimonyReliability` store

```rust
// crates/worldwake-core/src/testimony_reliability.rs (new)
#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct TestimonyReliability {
    entries: BTreeMap<TestimonyReliabilityKey, TestimonyReliabilityEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TestimonyReliabilityKey {
    pub source: EntityId,
    pub topic: TopicScope,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestimonyReliabilityEntry {
    pub direct_confirmations: u32,
    pub direct_refutations: u32,
    pub stale_claims: u32,
    pub contradicted_claims: u32,
    pub last_updated_tick: Tick,
    pub provenance_events: Vec<EventId>,    // bounded ring buffer
}

impl TestimonyReliabilityEntry {
    pub fn trust(&self, profile: &TestimonyTrustProfile) -> Permille { /* derived */ }
}
```

`trust` is a derived view per FND-27 — it never lives as authoritative state. The provenance ring buffer is bounded (default 8 entries) to prevent unbounded growth.

### D3: `RoutePreference` store

```rust
// crates/worldwake-core/src/route_preference.rs (new)
#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct RoutePreference {
    entries: BTreeMap<RouteSegment, RoutePreferenceEntry>,    // RouteSegment from S150
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutePreferenceEntry {
    pub safe_traversals: u32,
    pub dangerous_traversals: u32,
    pub last_safe_tick: Option<Tick>,
    pub last_dangerous_tick: Option<Tick>,
    pub last_traversal_event: Option<EventId>,
}

impl RoutePreferenceEntry {
    pub fn preference(&self, profile: &RoutePreferenceProfile) -> Permille { /* derived */ }
}
```

`preference` is derived. Higher = more preferred.

### D4: `TestimonyTrustProfile` (universal)

```rust
// crates/worldwake-core/src/testimony_trust_profile.rs (new)
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestimonyTrustProfile {
    pub confirmation_weight: Permille,       // default 250
    pub refutation_penalty: Permille,        // default 400
    pub stale_decay_per_tick: Permille,      // default 1 (slow decay)
    pub contradicted_penalty: Permille,      // default 350
    pub minimum_observations: u8,            // default 2 (below threshold → no derived trust signal)
    pub topic_weight_route_hazard: Permille,
    pub topic_weight_resource_availability: Permille,
    pub topic_weight_office_holder: Permille,
    pub topic_weight_accusation_credibility: Permille,
    pub topic_weight_bounty_validity: Permille,
    pub topic_weight_price_level: Permille,
    pub topic_weight_entity_whereabouts: Permille,
    pub topic_weight_general_fact: Permille,
}

impl Default for TestimonyTrustProfile { fn default() -> Self { /* see field defaults */ } }
```

Per-agent topic weights enable "officialist" / "gullible" / "empiricist" variation per the assessment's archetype hints (S152 substrate).

### D5: `RoutePreferenceProfile` (universal)

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutePreferenceProfile {
    pub safe_traversal_weight: Permille,        // default 200
    pub dangerous_traversal_penalty: Permille,  // default 600
    pub days_to_decay_observations: u32,        // default 30 (in tick days)
    pub minimum_traversals: u8,                 // default 2
}
```

### D6: Update paths

**Testimony reliability updates** fire from:

- **AskWitness commit + later belief confirmation**: when S139's AskWitness produces a belief, and a subsequent direct observation or trusted-source observation confirms or refutes the original testimony, `direct_confirmations` or `direct_refutations` increments. The confirming observation's `EventId` enters `provenance_events`.
- **Stale claim observation**: when a belief from witness W expires through age and is replaced by a fresh observation (direct or otherwise), `stale_claims` increments. The agent learns that W's claims age fast.
- **Contradiction observation**: when two simultaneous claims about the same topic conflict and the agent picks one, the loser's source gets `contradicted_claims` incremented.

**Route preference updates** fire from:

- **Safe traversal**: when an agent completes a `TravelTo` action across `RouteSegment(A, B)` without taking damage or witnessing threats, `safe_traversals` increments and `last_safe_tick = current_tick`.
- **Dangerous traversal**: when a `TravelTo` is interrupted by an ambush, hazard observation, or wound event, `dangerous_traversals` increments and `last_dangerous_tick = current_tick`. The `EventId` of the dangerous event enters `last_traversal_event`.

All updates are deterministic and tick-aligned.

### D7: Consumer integration

**Ranking damping** (`crates/worldwake-ai/src/ranking.rs`):

- `AskWitness` candidates ranked against witnesses with `trust < threshold` receive damping proportional to `(threshold - trust)`.
- `AcquireCommodity` candidates whose route traverses a `RouteSegment` with negative `preference` receive damping proportional to `-preference`.

**Candidate emission** (S146 extractors):

- Testimony-derived candidates from sources with very low trust (below `minimum_observations` threshold = no signal; below trust threshold = suppressed) are emitted with `SuppressionReason::SourceUnreliable` and dropped pre-rank.

**Travel cost** (`route_threat.rs`):

- `route_traversal_cost` returns an additive modifier from `RoutePreferenceEntry.preference()`. Existing threat estimation continues to dominate near-term hazards; route preferences add a learned bias on top.

### D8: Decision-history surface

Two new payload variants on always-on decision events (S136 substrate):

```rust
pub struct TestimonyTrustSummary {
    pub source: EntityId,
    pub topic: TopicScope,
    pub trust: Permille,
    pub observations: u32,
}

pub struct RoutePreferenceSummary {
    pub segment: RouteSegment,
    pub preference: Permille,
    pub last_safe_tick: Option<Tick>,
    pub last_dangerous_tick: Option<Tick>,
}
```

Observer Section 3b surfaces these when a goal commit involves testimony or route traversal.

### D9: S144 diagnostics extension

`ScenarioDiagnosticsReport.belief`:
- `source_reliability_changes_by_topic: BTreeMap<TopicScope, u64>` (existing field from S144's D1; this spec populates the data).
- `route_preference_updates: u64` (new).

### D10: Golden coverage

`golden_testimony_reliability.rs`:
- Witness reports stale route hazard → agent travels, observes no hazard → `direct_refutations` increments → next AskWitness on same source receives damped ranking.
- Witness reports accurate threat → agent observes confirmed threat → `direct_confirmations` increments → next AskWitness on same source preferred.
- False accusation by repeated unreliable source → subsequent accusation suppressed at minimum-observations threshold.

`golden_route_preferences.rs`:
- Agent traverses route A→B safely 5 times → `preference` positive → travel cost reduced.
- Agent ambushed on A→B → `dangerous_traversals` increments → travel cost increased.
- Route preference decays after `days_to_decay_observations` → falls to neutral.
- RoutePreference + RouteSegment blocker (S150) compose: blocker is hard suppression; preference is soft bias.

## FND-01 Section H Analysis

### Information-Path Analysis

**Testimony reliability**: updates from AskWitness commit events, observation events that confirm/refute prior testimony, contradiction-detection events. All come through existing perception or testimony carriers per FND-15.

**Route preferences**: updates from `TravelTo` action commit events (safe traversals) and threat-event observation (dangerous traversals). All come through existing event log.

No global truth queried.

### Positive-Feedback Analysis

Potential loop: agent distrusts witness → asks elsewhere → confirms / refutes → trust adjusts further. The loop is self-limiting because (a) each tick produces at most one trust update per source/topic, (b) the `minimum_observations` threshold prevents noisy single-observation flipping, (c) `stale_decay_per_tick` slowly relaxes trust toward neutral when sources go silent.

Potential loop on routes: agent prefers safe routes → routes get used more → if they remain safe, preference grows. The dampener is the actual world threat state — if a preferred route becomes dangerous, the next traversal records `dangerous_traversals`, and trust falls.

### Concrete Dampeners

- `stale_decay_per_tick` on testimony reliability.
- `days_to_decay_observations` on route preferences.
- `minimum_observations` threshold prevents single-observation cascades.
- Actual world threat state — preference is biased toward, not entitled to, safety.

### Stored State vs. Derived Read-Model List

**Stored state**:
- `TestimonyReliability` (per-agent runtime AI state).
- `RoutePreference` (per-agent runtime AI state).
- `TestimonyTrustProfile` (universal agent component).
- `RoutePreferenceProfile` (universal agent component).

**Derived read-model**:
- `trust: Permille` per entry (derived).
- `preference: Permille` per segment (derived).

## SystemFn Integration

No new top-level `SystemFn`. Updates fire from existing tick-level event consumers in the AI runtime.

## Component Registration

- **New universal components**: `TestimonyTrustProfile` and `RoutePreferenceProfile` on `EntityKind::Agent`. Default impls. Per `docs/spec-drafting-rules.md` Section 5.
- **No role-specific components.**

`TestimonyReliability` and `RoutePreference` are runtime-only AI state (not ECS components; live on `AgentAiState`).

## Cross-System Interactions

- Reads existing testimony-confirmation events (AskWitness commit, observation confirmation, contradiction detection — all per S139 / S130).
- Reads existing travel-completion and threat-observation events.
- Writes derived trust/preference into ranking and candidate-generation paths.

State-mediated per FND-26.

## Profile-Driven Parameters

All profile fields are `Permille` or `u32`/`u8` integer counts. No floats.

## Test Plan

- D10 golden coverage (3 + 4 = 7 scenarios).
- Determinism: same observation sequence → identical trust/preference values.
- Decay tests: aging without new observations → entries decay toward neutral.
- Save/load coverage for both runtime stores.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
