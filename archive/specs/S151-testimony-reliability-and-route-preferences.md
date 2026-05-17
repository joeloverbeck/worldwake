# S151: Testimony Source Reliability and Route Preferences

**Status**: COMPLETED

## Summary

Folds in PR-7 (SourceReliabilityMemory for testimony) and PR-8 (RoutePreference; HabitMemory scope-down) from `reports/ai-architecture-improvements.md`.

The current `ReliabilityRecord` in `crates/worldwake-core/src/experience.rs:77-95` is narrow: it tracks `successful_acquisitions`, `failed_attempts`, `last_attempt_tick`, `average_wait_ticks`, `wait_observation_count`, `last_observed_capacity`, and `last_observed_capacity_tick`, keyed by `SourceKey { entity: EntityId, commodity: CommodityKind }` — i.e. only commodity-extraction reliability per S131/S133. It does not track testimony reliability (whether a witness was right about route hazards), accusation credibility (whether a prior accuser's claim panned out), or rumor reliability (whether reported events actually occurred).

The current `crates/worldwake-ai/src/route_threat.rs` estimates dynamic route danger from witnessed/believed threats (`route_threat_estimate_from_memory`, `perceived_direct_travel_cost_from_memory`) but does not track per-agent learned route preferences (which routes the agent has personally found safe vs. dangerous over time, independent of currently-active threats).

S151 lands two new per-agent learned-state stores:

1. **`TestimonyReliability`** keyed by `(source: EntityId, topic: TopicScope)`: tracks per-witness reliability for each topic category. When a witness reports something and the agent later confirms it (true or false), the record updates. Used to dampen ranking on testimony from unreliable sources and to suppress candidate emission from sources well below trust threshold. Enables Scenario G (false rumor → wrongful accusation → correction) by giving agents a concrete record of who has been right or wrong before.

2. **`RoutePreference`** keyed by `RouteSegment` (shared with archived S150, `crates/worldwake-core/src/blocker_scope.rs:67-81`): tracks per-segment safe-traversal count, dangerous-traversal count, and the last witnessed danger event. Used to bias travel-cost estimation toward known-safe routes and away from known-dangerous ones, independent of currently-active threat beliefs.

Both stores live as fields on the existing `AgentDecisionRuntime` struct (`crates/worldwake-ai/src/decision_runtime.rs:153`), alongside `agenda_state` and `exhaustion_cache` — they are runtime-only AI state, not ECS components.

`HabitMemory` (the assessment's broader proposal — preferred goal-schema / method / place per trigger) is **deferred** until a specific habit-relevant pathology surfaces in S144 diagnostics. The scope-down rationale: per-method habit learning has no concrete failure scenario today; the existing `LearnedOpportunityMemory` (S109) and `DiversificationProfile` (S107) cover the present needs. If S144 reports method-thrash patterns, a follow-up spec lands HabitMemory.

## Phase and Status

Phase 12: AI Architecture Evolution - Completed

## Crates

- `worldwake-ai` — owns `TestimonyReliability` and `RoutePreference` runtime structures on `AgentDecisionRuntime`; updates ranking damping, candidate-emission suppression, and travel-cost paths to consume them; owns the new `TestimonyOmissionReason` enum on `decision_trace.rs`.
- `worldwake-core` — exposes `TopicScope`, `TestimonyReliabilityKey`, `TestimonyReliabilityEntry`, `RoutePreferenceEntry`, `TestimonyTrustProfile`, `RoutePreferenceProfile`, `TestimonyTrustSummary`, `RoutePreferenceSummary`, and the `belief_topic_to_topic_scope` mapping function.
- `worldwake-sim` — extends `GoalBeliefView` with accessor methods for the two new universal profiles; provides the `RuntimeBeliefView` impl and the `impl_goal_belief_view!` macro forwarding; bumps `SAVE_FORMAT_VERSION` for the new runtime fields.
- `worldwake-systems` — no change.
- `worldwake-cli` — observer renders reliability and preference snapshots from decision-history payloads; `AgentDef` and `spawn_agent()` set the two new universal profiles; S144 diagnostics aggregate reliability changes.

## Dependencies

- S109 (Typed Discrepancy Taxonomy, archived) — `Discrepancy::BeliefStale` and `Discrepancy::BeliefContradicted` (`crates/worldwake-core/src/discrepancy.rs:11,13`) participate in stale/contradiction observation paths that feed reliability updates.
- S131 (Source Reliability Wait/Capacity, archived) and S133 (Source Composite Tiebreaker, archived) — provide the precedent for per-source learned state and `ReliabilityRecord` shape.
- S139 (AskWitness Goal Layer, archived) — provides the testimony-acquisition path (`GoalKind::AskWitness { witness: EntityId, topic: TellTopic }` at `crates/worldwake-core/src/goal.rs:145-148`) that feeds testimony-reliability updates.
- S150 (Cross-Goal Blocker Scoping, archived at `archive/specs/S150-cross-goal-blocker-scoping.md`) — `RouteSegment` newtype shared; route preferences and route blockers compose (blockers say "currently bad," preferences say "historically [un]safe").
- S130 (Survey Records and Frontier Disconfirmation, archived) — provides confirmation-event substrate for testimony validation.
- S136 (Decision Event Payload Extension, archived) — provides the always-on `DecisionEventPayload` infrastructure and the precedent for embedding summary types (e.g., `BeliefSnapshot` at `crates/worldwake-core/src/decision_event_payload.rs:250-254`).
- S144 (Aggregate Scenario Diagnostics, archived) — provides the `ScenarioDiagnosticsReport.belief` substruct (`crates/worldwake-ai/src/scenario_diagnostics/mod.rs:57-64`) the new fields extend.

## Design Goals

1. **Two narrow stores, not one omnibus.** TestimonyReliability and RoutePreference are conceptually distinct and update through different paths.
2. **Per-agent learned state, not world truth.** Both stores live on the agent's AI runtime (`AgentDecisionRuntime`); they reflect *what this agent has learned*, not authoritative reliability scores.
3. **Concrete update events.** Every update has an `EventId` provenance — what observation produced the trust adjustment.
4. **Decay built in.** Both stores age out stale records per FND-22A.
5. **Inspectable.** Observer surfaces top reliable and unreliable sources per agent; top-preferred and avoided routes — via embedded payloads on existing always-on decision events (no new event tag).
6. **Composable with existing systems.** Testimony reliability extends the existing AskWitness damping site (`apply_ask_witness_learned_damping` at `crates/worldwake-ai/src/ranking.rs:1494-1517`); route preferences add an additive modifier to `perceived_direct_travel_cost_from_memory` (`crates/worldwake-ai/src/route_threat.rs:187-212`).

## Non-Goals

- **No HabitMemory.** Deferred per scope-down.
- **No `SourceReliabilityDiscount` generalization.** `crates/worldwake-ai/src/decision_trace.rs:694-700` already carries commodity-extraction reliability discounting (S131/S133); `TestimonyReliability` is a parallel structure for witness-claim reliability, not a generalization.
- **No global reputation system.** Each agent has their own reliability view.
- **No new event tag.** Updates flow through existing testimony-confirm and route-experience events (`AgentBeliefStore` and `RouteExperience` component deltas; `EventTag::Combat`, `Escalation`, `WildernessRelief` for same-tick threat provenance; belief overwrite sites in `crates/worldwake-core/src/belief.rs:129-150` and `163-193`).
- **No automatic reliability propagation through gossip.** If agent A trusts witness W, A's friend B does not automatically trust W. (S139's existing ShareBelief substrate already lets agents share trust through testimony if a scenario authors it.)
- **No new `Discrepancy` variant.** Testimony-source unreliability is a candidate-emission omission concern (per the Discrepancy-as-Failure-Attribution Surface pattern, option 1), not a typed plan-failure surface.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | Both stores hold concrete counts and event references, not abstract trust scores; the `trust: Permille` and `preference: Permille` fields are derived views per FND-27. The new `TestimonyOmissionReason::SourceUnreliable { source, topic, trust, threshold }` carries concrete state for failure attribution. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | Updates come from local observation events (route traversal, testimony confirmation/refutation via belief-store overwrites that read `PerceptionSource::Report { from: EntityId }` provenance at `crates/worldwake-core/src/belief.rs:2481-2486`); no global truth read. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | Both stores are explicit concrete state with accountable origin (`EventId`), scope (per-agent on `AgentDecisionRuntime`), and decay (`stale_decay_per_tick`, `days_to_decay_observations`). |
| FND-26 (Systems Interact Through State, Not Through Each Other) | New universal profiles are read by the AI crate through the existing `GoalBeliefView` accessor surface (`crates/worldwake-sim/src/belief_view.rs`); reliability and preference are consumed by ranking and candidate generation as state reads. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The new stores live alongside existing `ReliabilityRecord`; the old (commodity-extraction) reliability path is unchanged in identity, not deprecated. `TestimonyOmissionReason` is net-new (no shim around a fictional `SuppressionReason`). |
| FND-29 (Debuggability Is a Product Feature) | Observer surfaces both stores via embedded payload summaries on existing decision events; S144 aggregates reliability-change counts and route-preference distributions. |
| FND-30 (Causal Hooks Declaration) | Section H below enumerates information path, dampeners, stored state, and (per item 10) the agent-local learning origin/scope/decay surface. |

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

Eight categories matching the assessment's "expertise tags" concept. Payload-free (`Copy`, `Hash`, `Ord`) so it is safe to use as a histogram key and a `BTreeMap` key per the Aggregation-key-fidelity rule. Closed enum (extending requires a spec); per FND-22A "agent-local learned summaries are legal even when abstract — they are not world truth."

### D2: `belief_topic_to_topic_scope` mapping function

```rust
// crates/worldwake-core/src/topic_scope.rs (alongside the enum)
pub fn belief_topic_to_topic_scope(topic: &TellTopic) -> TopicScope {
    match topic {
        TellTopic::EntityBelief { subject: _ } => topic_scope_for_entity_aspect(/* aspect lookup */),
        TellTopic::SocialObservation { observation } => topic_scope_for_social_observation(observation),
        TellTopic::InstitutionalClaim { claim } => topic_scope_for_institutional_claim(claim),
    }
}
```

Because `TellTopic` (`crates/worldwake-core/src/belief.rs:1737`) is payload-bearing — its three variants carry `EntityId`, `SocialObservation`, and `InstitutionalClaim` — keying `TestimonyReliability` directly by `TellTopic` would fragment a per-witness histogram by subject identity, observation variant, and claim variant. The mapping function collapses the rich `TellTopic` carrier into the coarser payload-free `TopicScope` used as the reliability key.

The internal helpers `topic_scope_for_entity_aspect`, `topic_scope_for_social_observation`, and `topic_scope_for_institutional_claim` are exhaustive matches over the existing `EntityBeliefAspect` (`crates/worldwake-core/src/entity_belief_claim.rs:17-32`), `SocialObservation`, and `InstitutionalClaim` (`crates/worldwake-core/src/institutional.rs:26`) enums. Example mappings:

- `EntityBeliefAspect::Location` / `Holder` / `Activity` → `EntityWhereabouts`
- `EntityBeliefAspect::Inventory(_)` / `ResourceAvailable(_)` → `ResourceAvailability`
- `EntityBeliefAspect::Alive` / `Wounded` / `Courage` → `EntityWhereabouts`
- `EntityBeliefAspect::WorkstationPresent` / `ContentionState` / `WashBasinState` → `ResourceAvailability`
- `EntityBeliefAspect::Owner` / `Artifact` → `GeneralFact`
- `EntityBeliefAspect::Evidence` → `AccusationCredibility`
- `SocialObservationDetail::WitnessedConflict` → `RouteHazard`; `WitnessedAbsence` / `SuspectedTheft` → `AccusationCredibility`; otherwise → `GeneralFact`
- `InstitutionalClaim::OfficeHolder` / `SupportDeclaration` / `ForceControl` → `OfficeHolder`; `Accusation` / `Verdict` / `ArtifactCredibilityRefutation` → `AccusationCredibility`; `MissingPersonStatus` → `EntityWhereabouts`; otherwise → `GeneralFact`

The `BountyValidity` and `PriceLevel` categories are reserved in `TopicScope` for later testimony topics. The current `InstitutionalClaim` enum does not yet expose bounty-validity or price-level variants, so D2's exhaustive mapping has no live upstream arm that returns those categories in S151TESRELROU-001.

The exhaustive table is asserted by a workspace test that fails compilation if a new `EntityBeliefAspect`, `SocialObservation`, or `InstitutionalClaim` variant lands without an explicit mapping arm.

### D3: `TestimonyReliability` runtime store

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

`TestimonyReliability` is stored as a new field on `AgentDecisionRuntime` (per D11). `trust` is a derived view per FND-27 — it never lives as authoritative state. The provenance ring buffer is bounded (default 8 entries) to prevent unbounded growth. The keying shape mirrors `ReliabilityRecord`'s precedent (`SourceKey { entity: EntityId, commodity: CommodityKind }`) — both are `(source-entity, topic-discriminator)` pairs.

### D4: `RoutePreference` runtime store

```rust
// crates/worldwake-core/src/route_preference.rs (new)
#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct RoutePreference {
    entries: BTreeMap<RouteSegment, RoutePreferenceEntry>,    // RouteSegment from archived S150
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

`RouteSegment` is the canonical-form struct `{ from: EntityId, to: EntityId }` from `crates/worldwake-core/src/blocker_scope.rs:67-81`; its `.new(from, to)` constructor normalizes endpoint order so `RoutePreference` is direction-independent. `RoutePreference` is stored as a new field on `AgentDecisionRuntime` (per D11). `preference` is derived. Higher = more preferred.

### D5: `TestimonyTrustProfile` (universal)

```rust
// crates/worldwake-core/src/testimony_trust_profile.rs (new)
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestimonyTrustProfile {
    pub confirmation_weight: Permille,       // default 250
    pub refutation_penalty: Permille,        // default 400
    pub stale_decay_per_tick: Permille,      // default 1 (slow decay)
    pub contradicted_penalty: Permille,      // default 350
    pub minimum_observations: u8,            // default 2 (below threshold → no derived trust signal)
    pub trust_threshold: Permille,           // default 400 (below → emission suppressed)
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

Universal per FND-22A — registered on `EntityKind::Agent` with default impl. Per-agent topic weights enable "officialist" / "gullible" / "empiricist" variation per the assessment's archetype hints (S152 substrate).

### D6: `RoutePreferenceProfile` (universal)

```rust
// crates/worldwake-core/src/route_preference_profile.rs (new)
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutePreferenceProfile {
    pub safe_traversal_weight: Permille,        // default 200
    pub dangerous_traversal_penalty: Permille,  // default 600
    pub days_to_decay_observations: u32,        // default 30 (in tick days)
    pub minimum_traversals: u8,                 // default 2
}

impl Default for RoutePreferenceProfile { fn default() -> Self { /* see field defaults */ } }
```

Universal per FND-22A — registered on `EntityKind::Agent` with default impl.

### D7: Update paths (explicit hook sites)

S151 introduces **no new event tag**. Updates fire from existing event-log emissions and existing belief-store overwrite sites:

**Testimony reliability updates** fire from the observation-phase hook in `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs`, reading per-agent `AgentBeliefStore` component deltas for the current tick:

- **AskWitness commit + later belief confirmation**: When S139's AskWitness produces a `Report { from: EntityId, chain_len }` claim (per `crates/worldwake-core/src/belief.rs:2481-2486` `PerceptionSource`), and a subsequent direct observation overwrites that claim via `import_entity_snapshot()` (`crates/worldwake-core/src/belief.rs:163-193`), the hook compares the prior claim's value against the new direct observation. Match → `direct_confirmations += 1`; mismatch → `direct_refutations += 1`. The confirming/refuting observation's `EventId` enters `provenance_events`. The mapping from claim's `TellTopic` / aspect to `TopicScope` flows through D2's `belief_topic_to_topic_scope`.
- **Stale claim observation**: When `refute_entity_claims()` (`crates/worldwake-core/src/belief.rs:129-150`) clears a stale claim whose `PerceptionSource` was a `Report { from: witness }`, the hook increments `stale_claims` for `(witness, mapped_topic)`.
- **Contradiction observation**: When two simultaneous Reports about the same `(subject, aspect)` conflict and the agent picks one (per S109's `Discrepancy::BeliefContradicted` emission path), the loser's witness gets `contradicted_claims += 1` for the mapped topic.

**Route preference updates** fire from the same AI tick observation hook, reading per-agent `RouteExperience` component deltas from the current tick's event log:

- **Safe traversal**: When the agent's `RouteExperience` delta increases `safe_trips` for a topology edge, `safe_traversals += 1` and `last_safe_tick = current_tick` for `RouteSegment::new(edge.from(), edge.to())`.
- **Dangerous traversal**: When the agent's `RouteExperience` delta increases `hostile_encounters` for a topology edge, `dangerous_traversals += 1` and `last_dangerous_tick = current_tick`. A same-tick `Combat`, `Escalation`, or `WildernessRelief` event involving the agent supplies `last_traversal_event` when present; otherwise the route-experience mutation event ID is used.

All updates are deterministic and tick-aligned.

### D8: Consumer integration

**Ranking damping** (`crates/worldwake-ai/src/ranking.rs`):

The existing `apply_ask_witness_learned_damping` function at lines 1494-1517 already implements the damping shape S151 needs (look up agent-local memory, apply Permille damping factor, record via `CandidateDampingEntry`). S151 extends this site:

1. Within `apply_ask_witness_learned_damping`, additionally consult `TestimonyReliability` for the AskWitness candidate's `witness` entity and the topic mapped from `AskWitness.topic` via D2.
2. If the derived `trust` is below `TestimonyTrustProfile.trust_threshold` but above any hard-suppression floor (handled separately at emission per below), apply damping proportional to `(trust_threshold - trust)`.
3. Record the damping via a new `CandidateDampingReason::TestimonySourceUnreliable { source: EntityId, topic: TopicScope, trust: Permille, threshold: Permille }` variant added to the existing enum at `crates/worldwake-ai/src/decision_trace.rs:416`.

**Candidate emission suppression** (S146 extractors, primarily `extract_ask_witness_candidates` at `crates/worldwake-ai/src/candidate_generation.rs:2877-3045`):

For testimony-derived candidates from sources well below threshold (specifically: `TestimonyReliabilityEntry.observations >= minimum_observations` AND `trust < trust_threshold * suppression_floor_factor`), the extractor suppresses emission outright and records the reason through a new domain-specific omission enum:

```rust
// crates/worldwake-ai/src/decision_trace.rs (new, parallel to PoliticalCandidateOmissionReason at line 545)
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TestimonyOmissionReason {
    SourceUnreliable {
        source: EntityId,
        topic: TopicScope,
        trust: Permille,
        threshold: Permille,
    },
}
```

This mirrors the three existing domain-omission enums (`PoliticalCandidateOmissionReason`, `BanditCandidateOmissionReason`, `ViolationDetectionOmissionReason`). The omission entry is recorded via the existing diagnostic surface attached to `extract_ask_witness_candidates`'s `CandidateGenerationDiagnostics`.

Sources with `observations < minimum_observations` produce no signal (neither damping nor suppression — they pass through unchanged).

**Travel cost** (`crates/worldwake-ai/src/route_threat.rs:187-212`):

`perceived_direct_travel_cost_from_memory` returns `u32` ticks adjusted by a threat penalty. S151 extends it with a `RoutePreference` lookup AFTER the threat penalty is computed:

1. Look up the segment in `RoutePreference.entries` (the segment is constructed canonically from `(edge_from, edge_to)` via `RouteSegment::new`).
2. Compute `preference: Permille` via `RoutePreferenceEntry::preference(profile)`.
3. Apply preference as an additive cost adjustment: positive preference (more safe traversals than dangerous) reduces cost by `base_ticks * preference / 1000`; negative preference (preference value below the neutral midpoint) increases cost proportionally.
4. The function signature gains a `route_preference: Option<&RoutePreference>` parameter and a `route_preference_profile: Option<&RoutePreferenceProfile>` parameter, both threaded from the caller through the planner's existing belief-view surface. Existing threat estimation continues to dominate near-term hazards; route preferences add a learned bias on top.

### D9: Decision-history surface

Two new optional payload fields are embedded on existing always-on `DecisionEventPayload` variants (`crates/worldwake-core/src/decision_event_payload.rs:14-30`) — following the `BeliefSnapshot` precedent at lines 250-254. No new top-level variants on the enum.

```rust
// crates/worldwake-core/src/decision_event_payload.rs (new types)
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TestimonyTrustSummary {
    pub source: EntityId,
    pub topic: TopicScope,
    pub trust: Permille,
    pub observations: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutePreferenceSummary {
    pub segment: RouteSegment,
    pub preference: Permille,
    pub last_safe_tick: Option<Tick>,
    pub last_dangerous_tick: Option<Tick>,
}
```

Embed targets:

- `GoalCommittedPayload` (`decision_event_payload.rs:159-168`) gains `#[serde(default)] pub testimony_trust_context: Vec<TestimonyTrustSummary>` (for goals whose commit depends on one or more witness sources) and `#[serde(default)] pub route_preference_context: Vec<RoutePreferenceSummary>` (for goals whose plan crosses tracked route segments).
- `GoalSuppressedPayload` gains `#[serde(default)] pub testimony_trust_context: Vec<TestimonyTrustSummary>` for the `TestimonyOmissionReason::SourceUnreliable` suppression path.

`Vec` rather than `Option<_>` because a single goal commit may reference multiple witnesses or multiple segments. Both types are `Copy` (4-5 small fields each) and satisfy the existing `Eq/Hash/Ord/Serialize` derives on the parent payload structs.

Observer Section 3b (Decision History, `crates/worldwake-cli/src/bin/observer.rs:932`) extends its existing `decision_payload_summary()` rendering at line 959 to surface the embedded contexts as continuation rows below the goal-commit / goal-suppressed table entries, following the multi-line continuation pattern already used for `GoalCommitted` motive sources (lines 962-971).

### D10: S144 diagnostics extension

Two **net-new** fields on `ScenarioDiagnosticsReport.belief` (`crates/worldwake-ai/src/scenario_diagnostics/mod.rs:57-64`):

- `source_reliability_changes_by_topic: BTreeMap<TopicScope, u64>` — per-topic breakdown of testimony-reliability updates. Replaces the existing flat `source_reliability_changes: u64` (per FND-28, no shim — the flat field is removed, callers migrate to the map).
- `route_preference_changes: u64` — total route-preference update count across all agents per scenario run. Mirrors the `source_reliability_changes` naming convention.

The archived S144's D1 explicitly fold-rejected the by-topic breakdown until `TopicScope` landed (`archive/specs/S144-aggregate-scenario-diagnostics.md:141-142`); D1 lands the substrate, so D10 lands the field.

The aggregator (`crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`) gains the corresponding update sites in the testimony / route observation paths.

### D11: Component registration

Both new universal components (`TestimonyTrustProfile` from D5, `RoutePreferenceProfile` from D6) are registered per `docs/spec-drafting-rules.md` Section 5:

1. **`component_schema.rs` entries**: Two new entries in the `with_component_schema_entries!` macro (`crates/worldwake-core/src/component_schema.rs`), each with the 13-method registration cluster (`insert_*`, `get_*`, `set_*`, `clear_*`, etc.). Predicate: `|kind| kind == EntityKind::Agent`. Strategy: `txn_simple_set`. Mirrors the `MetabolismProfile` entry at lines 1382-1406.
2. **`AgentDef` fields**: Two new fields in `AgentDef` at `crates/worldwake-cli/src/scenario/types.rs:572-654`, appended to the trailing universal-profile cluster (after `substitute_preferences`):
   ```rust
   #[serde(default)]
   pub testimony_trust_profile: Option<TestimonyTrustProfile>,
   #[serde(default)]
   pub route_preference_profile: Option<RoutePreferenceProfile>,
   ```
   `#[serde(default)]` so existing scenario files continue to deserialize unchanged.
3. **`spawn_agent()` setters**: Two new `set_component_*` calls in `crates/worldwake-cli/src/scenario/mod.rs:617-656` cluster, each using the universal `.unwrap_or_default()` pattern:
   ```rust
   txn.set_component_testimony_trust_profile(agent_id, agent_def.testimony_trust_profile.unwrap_or_default())?;
   txn.set_component_route_preference_profile(agent_id, agent_def.route_preference_profile.unwrap_or_default())?;
   ```

Per the "New Component on EntityKind::Agent" pattern (worldwake-validation-patterns.md), both components are core-resident (D5/D6 define them in `worldwake-core/src/`), classified as universal, and have `Default` impls.

`TestimonyReliability` and `RoutePreference` themselves are **not** ECS components — they are runtime-only state on `AgentDecisionRuntime` (`crates/worldwake-ai/src/decision_runtime.rs:153`). Two new fields are added to that struct:

```rust
pub testimony_reliability: TestimonyReliability,
pub route_preference: RoutePreference,
```

Following the precedent of `agenda_state: AgendaState` and `exhaustion_cache: BTreeMap<OpportunityKey, ExhaustionEntry>` already on the struct.

### D12: `GoalBeliefView` accessor surface

Per the "New Component Read by AI Crate" pattern, the new universal profiles must be exposed via `GoalBeliefView` so the AI crate consumes them through the trait surface (not via direct ECS reads):

1. **Trait extension** (`crates/worldwake-sim/src/belief_view.rs`): Two new accessor methods on `GoalBeliefView`:
   ```rust
   fn testimony_trust_profile(&self, agent: EntityId) -> &TestimonyTrustProfile;
   fn route_preference_profile(&self, agent: EntityId) -> &RoutePreferenceProfile;
   ```
   Following the existing universal-profile accessor convention (e.g., `metabolism_profile`, `cognitive_profile`).
2. **`RuntimeBeliefView` impl**: Backing implementation reads the component via the existing `get_component_*` accessors and `expect()` on known agents (per Section 5 universal-profile contract).
3. **`impl_goal_belief_view!` macro / blanket impl**: Forwarding entry for each new method.

Ranking (D8) and candidate emission (D8) consume the profiles through these accessors. `TestimonyReliability` and `RoutePreference` themselves are read directly from `AgentDecisionRuntime` (they live on the ai-crate runtime structure, not on the sim-crate belief view).

### D13: SAVE_FORMAT_VERSION bump

`crates/worldwake-sim/src/save_load.rs:6` `SAVE_FORMAT_VERSION` increments from `87` to `88` to cover:

- New `TestimonyReliability` and `RoutePreference` fields on `AgentDecisionRuntime`.
- New `testimony_trust_profile` and `route_preference_profile` components on `EntityKind::Agent`.
- New `testimony_trust_context` and `route_preference_context` fields on `GoalCommittedPayload` and `GoalSuppressedPayload` (with explicit omitted-field defaults where the live serializer can represent omitted fields; bincode save-stream compatibility is verified or documented by the D13 ticket).

Save/load round-trip tests cover the new runtime fields per the existing pattern.

### D14: Golden coverage

`golden_testimony_reliability.rs`:

- Witness reports stale route hazard → agent travels, observes no hazard → `direct_refutations` increments → next AskWitness on same source receives damped ranking via `CandidateDampingReason::TestimonySourceUnreliable`.
- Witness reports accurate threat → agent observes confirmed threat → `direct_confirmations` increments → next AskWitness on same source preferred.
- False accusation by repeated unreliable source → subsequent accusation suppressed at trust-threshold via `TestimonyOmissionReason::SourceUnreliable`, recorded in `CandidateGenerationDiagnostics`.
- `belief_topic_to_topic_scope` exhaustive mapping unit tests.

`golden_route_preferences.rs`:

- Agent traverses route A→B safely 5 times → `preference` positive → `perceived_direct_travel_cost_from_memory` returns reduced cost.
- Agent ambushed on A->B (`RouteExperience.hostile_encounters` increases, with same-tick threat provenance when available) → `dangerous_traversals` increments → travel cost increased.
- Route preference decays after `days_to_decay_observations` → falls to neutral.
- `RoutePreference` + `BlockerScope::RouteSegment` (S150) compose: blocker is hard suppression; preference is soft bias.

## FND-01 Section H Analysis

### Information-Path Analysis

**Testimony reliability**: updates fire from belief-store overwrite events (`refute_entity_claims`, `import_entity_snapshot` in `belief.rs`), and from `Discrepancy::BeliefContradicted` emission in the AI tick. All belief overwrites originate from local perception or testimony per FND-15 (`PerceptionSource::Report { from }` carries the witness identity). The `TellTopic → TopicScope` mapping (D2) collapses the rich belief topic carrier into the reliability key without losing the witness identity.

**Route preferences**: updates fire from `RouteExperience` component deltas that record safe trips and hostile encounters per topology edge. Same-tick threat-class event observations (`Combat`, `Escalation`, `WildernessRelief`) provide dangerous-traversal provenance when available. All come through the existing append-only event log.

No global truth queried.

### Positive-Feedback Analysis

**Trust spiral**: agent distrusts witness → asks elsewhere → confirms / refutes → trust adjusts further. Self-limiting because (a) each tick produces at most one trust update per `(source, topic)`, (b) the `minimum_observations` threshold prevents single-observation flipping, (c) `stale_decay_per_tick` slowly relaxes trust toward neutral when sources go silent.

**Route preference loop**: agent prefers safe routes → routes get used more → if they remain safe, preference grows. Dampened by the actual world threat state — if a preferred route becomes dangerous, the next traversal records `dangerous_traversals`, and preference falls.

### Concrete Dampeners

- `stale_decay_per_tick` on testimony reliability (FND-11 — physical decay process, not numeric clamp).
- `days_to_decay_observations` on route preferences.
- `minimum_observations` threshold prevents single-observation cascades.
- `minimum_traversals` threshold prevents single-traversal route-preference cascades.
- Actual world threat state — preference is biased toward, not entitled to, safety.

### Stored State vs. Derived Read-Model List

**Stored state**:
- `TestimonyReliability` (per-agent runtime AI state on `AgentDecisionRuntime`).
- `RoutePreference` (per-agent runtime AI state on `AgentDecisionRuntime`).
- `TestimonyTrustProfile` (universal ECS component on `EntityKind::Agent`).
- `RoutePreferenceProfile` (universal ECS component on `EntityKind::Agent`).
- `TestimonyOmissionReason` enum (failure-attribution data carried in `CandidateGenerationDiagnostics`).
- `CandidateDampingReason::TestimonySourceUnreliable` (failure-attribution data carried in `CandidateDampingEntry`).
- `TestimonyTrustSummary` / `RoutePreferenceSummary` payloads embedded in `GoalCommittedPayload` and `GoalSuppressedPayload`.

**Derived read-model**:
- `trust: Permille` per entry (derived from confirmation/refutation counts and topic weights).
- `preference: Permille` per segment (derived from safe/dangerous traversal counts and decay).

### Agent-Local Learning Provenance (FND-30 item 10)

Every TestimonyReliability and RoutePreference update carries an `EventId` provenance entry, scoped to the per-agent `AgentDecisionRuntime`. Updates are revised by subsequent observation events; entries decay via `stale_decay_per_tick` and `days_to_decay_observations`. Both are summaries (not authoritative truth); the authoritative substrate is the underlying event log entries that produced each provenance reference.

## SystemFn Integration

No new top-level `SystemFn`. Updates fire from a new observation-phase hook inside the existing agent tick (D7), and reads happen inside existing ranking and travel-cost paths (D8). Component registration (D11) integrates via the existing `with_component_schema_entries!` macro and `spawn_agent()` setter pattern.

## Component Registration

Covered by D11. Summary: two new universal components (`TestimonyTrustProfile`, `RoutePreferenceProfile`) on `EntityKind::Agent` with `Default` impls, `AgentDef` `Option<_>` fields with `#[serde(default)]`, and `spawn_agent()` `.unwrap_or_default()` setters. `TestimonyReliability` and `RoutePreference` are runtime-only AI state, not ECS components (exempt per Section 5).

## Cross-System Interactions

- The AI tick's observation-phase hook (D7) reads belief-store overwrite events (`refute_entity_claims`, `import_entity_snapshot` in `crates/worldwake-core/src/belief.rs`) and `RouteExperience` component deltas with same-tick threat-class events for dangerous-route provenance.
- Ranking (`crates/worldwake-ai/src/ranking.rs`) and candidate emission (`crates/worldwake-ai/src/candidate_generation.rs`) consume `TestimonyTrustProfile` and `RoutePreferenceProfile` through `GoalBeliefView` accessors (D12).
- `crates/worldwake-ai/src/route_threat.rs` consumes `RoutePreference` and `RoutePreferenceProfile` through additional function parameters threaded by the planner caller (D8).
- Observer (`crates/worldwake-cli/src/bin/observer.rs`) renders embedded payload contexts via the existing `decision_payload_summary()` path (D9).
- S144 aggregator (`crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`) increments the new fields in response to the same update events D7 hooks.

State-mediated per FND-26. No new cross-system command channels.

## Profile-Driven Parameters

All profile fields are `Permille` or `u32`/`u8` integer counts. No floats. `TestimonyTrustProfile` and `RoutePreferenceProfile` are both universal per FND-22A; per-agent variation enables archetype expression (S152 substrate).

## Verification Result

1. Passed: D14 golden coverage landed as three testimony scenarios and four route-preference scenarios.
2. Passed: deterministic testimony summary derivation and route-preference decay are covered by focused tests.
3. Passed: save/load coverage proves both runtime stores and S151 decision payload context under bumped `SAVE_FORMAT_VERSION = 88`.
4. Passed: component-registration tests cover the new universal profile surface.
5. Passed: `GoalBeliefView` accessor tests cover `RuntimeBeliefView` resolution of the new profiles.
6. Passed: `cargo clippy --workspace --all-targets -- -D warnings`.

## Outcome

Completed on 2026-05-17.

S151 landed the testimony-reliability and route-preference substrate across tickets `archive/tickets/S151TESRELROU-001.md` through `archive/tickets/S151TESRELROU-011.md`:

1. `TopicScope`, `belief_topic_to_topic_scope`, `TestimonyReliability`, `RoutePreference`, `TestimonyTrustProfile`, and `RoutePreferenceProfile` landed in `worldwake-core`.
2. The new universal profiles are registered for agents, scenario-definable through `AgentDef` / `spawn_agent()`, and readable through `GoalBeliefView`.
3. Agent tick observation now updates testimony reliability and route preference state from local belief and route-experience events.
4. AskWitness candidate generation and ranking consume testimony reliability for source suppression and damping, with decision-trace and decision-payload context.
5. Route planning consumes route preferences as a soft learned travel-cost bias that composes with S150 route-segment blockers.
6. Observer and scenario diagnostics surfaces render/aggregate the new testimony and route preference context.
7. Save format version 88 is the completed S151 boundary; version 87 bytes are intentionally rejected under the no-backward-compatibility policy.
8. Golden coverage landed as seven scenario-documented public-contract tests for testimony trust summaries, suppressed-goal payload context, route preference derivation/decay, and blocker/preference composition.

Deviations from the draft:

1. `BountyValidity` and `PriceLevel` remain reserved `TopicScope` categories with no current upstream `InstitutionalClaim` mapping arm.
2. The final route-update seam uses the landed `RouteExperience` delta path rather than a speculative separate route-event hook.
3. The D14 final goldens intentionally prove stable public contract surfaces rather than duplicating crate-private candidate/ranking/cost internals through brittle full authored scenarios. Those internals are covered by focused module tests from the earlier S151 tickets.
4. `HabitMemory` remains deferred until diagnostics expose a concrete method-thrash pathology.

Verification included:

1. Passed: `cargo fmt --all`
2. Passed: focused S151 ticket test lanes across core, sim, ai, systems, cli, observer, diagnostics, save/load, and golden integration surfaces.
3. Passed: `python3 scripts/golden_inventory.py --write --check-docs`
4. Passed: `cargo test --workspace`
5. Passed: `cargo clippy --workspace --all-targets -- -D warnings`
6. Passed: `./scripts/verify.sh`
