# S34: General Epistemic Actions

## Summary

Extend the action framework with general-purpose epistemic actions — inspect_place, ask_witness, verify_location — and a proactive `VerifyBelief` goal kind. Currently investigation is narrowly scoped to S27 violation response. Agents cannot proactively verify stale beliefs, inspect unfamiliar places, or query other agents for information. This blocks canonical Scenario D (rumor → travel → empty source → discovery → belief correction → replan) from emerging through deliberate verification rather than accidental observation.

## Source

Derived from ChatGPT architecture review WW-AI-003 (Expectations, discrepancies, and epistemic actions), filtered to the epistemic action component only. The full expectation-registration system proposed by ChatGPT is not needed — S27's violation detection already handles expectation-vs-observation mismatches. What's missing is the agent's ability to *deliberately seek information* rather than only *reactively discover mismatches*.

## Phase

Phase 3+: AI Architecture Overhaul, Step 13.5 Wave 5

## Crates

- `worldwake-core` (new goal kind, new action domain, new disposition profile)
- `worldwake-sim` (action def registration)
- `worldwake-systems` (action handlers)
- `worldwake-ai` (candidate generation, planner ops, search)

## Dependencies

- S27 ✅ (expectation-violation goals — provides violation detection and `InvestigateViolation` this spec extends)
- E14 ✅ (perception & belief — provides `AgentBeliefStore`, `PerceptionSource`, `BelievedEntityState`)
- E15c ✅ (conversation memory — provides Tell mechanics this spec's `ask_witness` parallels)

## FOUNDATIONS Alignment

- **P13** (Knowledge Acquired Locally): Epistemic actions are the mechanism by which agents deliberately acquire local knowledge. Without them, knowledge acquisition is limited to passive perception and reactive investigation.
- **P15** (Surprise From Violated Expectation): Agents should be able to proactively verify beliefs they're about to act on, especially when confidence is low. This creates the expectation → verification → surprise chain.
- **P7** (Locality): All epistemic actions require co-location. `inspect_place` observes the current location. `ask_witness` requires a co-located informant. `verify_location` requires travel to the target place.
- **Scenario D**: Rumor → Travel → Empty Source → Discovery → Belief Correction → Replan. Without `inspect_place` or `verify_location`, the "discovery" step relies entirely on passive perception rather than deliberate investigation.

## Design Goals

1. **Deliberate information seeking**: Agents can choose to verify beliefs before committing to costly plans.
2. **Cost-bearing**: All epistemic actions have duration, preconditions, and occupancy (P8). Information is not free.
3. **Belief-mediated results**: Epistemic actions update the agent's belief store, not authoritative world state.
4. **Profile-driven behavior**: Per-agent disposition profiles control verification thresholds and action durations (P20).
5. **Planner integration**: The GOAP planner can include epistemic actions as plan prerequisites when confidence is low.

## Deliverables

### 1. `VerificationDispositionProfile` component (worldwake-core)

```rust
/// Per-agent parameters controlling epistemic action behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationDispositionProfile {
    /// Beliefs below this confidence trigger VerifyBelief candidate generation.
    /// Permille(400) = 40% confidence threshold.
    pub belief_verification_threshold: Permille,
    /// Duration in ticks for inspect_place action.
    pub inspection_duration_ticks: NonZeroU32,
    /// Duration in ticks for ask_witness action.
    pub witness_query_duration_ticks: NonZeroU32,
    /// Duration in ticks for verify_location action.
    pub verify_location_duration_ticks: NonZeroU32,
    /// Motive weight for verification goals relative to the plan they support.
    /// Permille(200) = 20% of the supported goal's motive value.
    pub verification_motive_weight: Permille,
}
```

Registered on agents via component schema. Agents without this profile do not generate `VerifyBelief` candidates (they rely on passive perception only — P20 diversity).

### 2. Action definitions (worldwake-sim, worldwake-systems)

#### `inspect_place`
- **Preconditions**: Actor alive, not incapacitated, at a place.
- **Duration**: `VerificationDispositionProfile::inspection_duration_ticks`.
- **Interruptibility**: FreelyInterruptible.
- **On commit**: For each entity at the actor's current place, update the actor's `BelievedEntityState` with `observed_tick = current_tick` and `source = DirectObservation`. For any entity the actor previously believed was at this place but is now absent, generate a `ViolationKind::EntityMissing` record in the actor's `ViolationMemory`.
- **Visibility**: SamePlace.
- **Payload**: None (inspects current location).

#### `ask_witness`
- **Preconditions**: Actor alive, not incapacitated. Target agent alive, co-located, not incapacitated.
- **Duration**: `VerificationDispositionProfile::witness_query_duration_ticks`.
- **Interruptibility**: FreelyInterruptible.
- **On commit**: Transfer a subset of the target's `AgentBeliefStore.known_entities` entries to the actor, filtered by topic (the entity/commodity the actor is asking about). Transferred beliefs carry `PerceptionSource::Report { from: target, chain_len: 1 }`. Respects E15c conversation memory — suppresses re-asking the same target about the same topic within retention window.
- **Visibility**: SamePlace (other co-located agents can observe the asking).
- **Payload**: `AskWitnessPayload { topic_entity: Option<EntityId>, topic_commodity: Option<CommodityKind> }`.

#### `verify_location`
- **Preconditions**: Actor alive, not incapacitated, at the place being verified.
- **Duration**: `VerificationDispositionProfile::verify_location_duration_ticks`.
- **Interruptibility**: FreelyInterruptible.
- **On commit**: Check whether the specific entity the actor expects to find at this place is present. If present, update belief with fresh `DirectObservation`. If absent, generate `ViolationKind::EntityMissing` record.
- **Visibility**: SamePlace.
- **Payload**: `VerifyLocationPayload { expected_entity: EntityId }`.

### 3. `GoalKind::VerifyBelief` (worldwake-core)

```rust
/// Proactively verify a stale or low-confidence belief before acting on it.
GoalKind::VerifyBelief {
    /// The entity the agent wants to verify the status/location of.
    entity: EntityId,
    /// The place the agent believes the entity is at.
    place: EntityId,
}
```

- `GoalKey` derivation: `{ kind: VerifyBelief { entity, place }, commodity: None, entity: Some(entity), place: Some(place) }`
- Satisfaction: Agent has a `BelievedEntityState` for `entity` with `observed_tick >= tick when goal was generated` (fresh observation obtained).

### 4. PlannerOpKind additions (worldwake-ai)

```rust
PlannerOpKind::InspectPlace,
PlannerOpKind::AskWitness,
PlannerOpKind::VerifyLocation,
```

Planner semantics:
- `InspectPlace`: Terminal for `VerifyBelief` goals when actor is at the target place. Barrier: yes (observation results unknown to planner).
- `AskWitness`: Terminal for `VerifyBelief` goals when a co-located agent is available. Barrier: yes (witness knowledge unknown to planner).
- `VerifyLocation`: Terminal for `VerifyBelief` goals at target place for a specific entity. Barrier: yes (entity presence unknown to planner).

### 5. Candidate generation (worldwake-ai)

New `emit_verify_belief_goals()` function:
- For each `GroundedGoal` the agent is about to plan for, check confidence of supporting beliefs.
- If any supporting belief has `belief_confidence(source, staleness, policy) < profile.belief_verification_threshold`, emit a `VerifyBelief` candidate for the stale belief's subject entity + believed place.
- Motive: `supporting_goal_motive * profile.verification_motive_weight`.
- Priority class: Same as the supporting goal (verification inherits urgency).
- Only emit if agent has `VerificationDispositionProfile`.

### 6. ActionDomain extension (worldwake-sim)

Add `ActionDomain::Epistemic` to categorize inspect/ask/verify actions.

## Component Registration

- `VerificationDispositionProfile`: Register on `EntityKind::Agent` in component schema.
- Action defs: Register `inspect_place`, `ask_witness`, `verify_location` in `ActionDefRegistry`.
- Handlers: Register in `ActionHandlerRegistry`.

## FND-01 Section H Analysis

### Information-path analysis
Agent holds stale belief → candidate generation detects low confidence → emits `VerifyBelief` goal → planner includes Travel + InspectPlace/VerifyLocation → agent travels to target place → executes epistemic action → perception system updates belief store with direct observation → violations recorded if expectations mismatched. For `ask_witness`: agent queries co-located agent → target's beliefs transferred with Report provenance → actor's belief store updated. All information paths are local and traceable.

### Positive-feedback analysis
No amplifying loops. Epistemic actions consume time (duration-bearing), which limits how often agents verify. More verification → less time for production/trade → natural cost pressure against over-verification.

### Concrete dampeners
Time cost of epistemic actions is the physical dampener. Agents who verify everything accomplish nothing else. `verification_motive_weight` controls how much agents prioritize verification vs. action (per-agent diversity).

### Stored state vs. derived read-model list
- **Stored**: `VerificationDispositionProfile` (component). Action defs (registry). Updated `BelievedEntityState` entries (existing belief store). `RecordedViolation` entries (existing violation memory).
- **Derived**: `VerifyBelief` candidates (recomputed each tick from current belief confidence). Confidence scores (computed from provenance + age, never stored).

## Tests

### Focused tests
- [ ] `inspect_place` refreshes all entity beliefs at current location with `DirectObservation` source
- [ ] `inspect_place` generates `EntityMissing` violation for expected-but-absent entities
- [ ] `ask_witness` transfers target's beliefs with `Report { chain_len: 1 }` provenance
- [ ] `ask_witness` respects conversation memory (no re-ask within retention window)
- [ ] `verify_location` confirms present entity with fresh observation
- [ ] `verify_location` generates `EntityMissing` violation for absent entity
- [ ] `VerifyBelief` candidate emitted only when belief confidence below threshold
- [ ] `VerifyBelief` candidate not emitted when agent lacks `VerificationDispositionProfile`
- [ ] `VerifyBelief` motive scales with supporting goal's motive
- [ ] Planner can construct Travel → InspectPlace plan for remote `VerifyBelief` goal

### Golden tests
- [ ] Scenario D variant: Agent hears rumor about commodity at distant place → travels → executes inspect_place → finds place empty → violation recorded → replans to alternative source
- [ ] Agent asks co-located witness about commodity location → receives report-sourced belief → uses it to plan acquisition
- [ ] Deterministic replay companions for each golden

## Acceptance Criteria

1. Agents can deliberately seek information through inspect_place, ask_witness, and verify_location actions.
2. All epistemic actions have duration, preconditions, and occupancy — information is not free.
3. `VerifyBelief` candidates are generated only when belief confidence is below agent-specific threshold.
4. Epistemic action results update belief stores with proper provenance, not authoritative world state.
5. Violations detected during epistemic actions integrate with existing S27 violation memory.
6. Agents without `VerificationDispositionProfile` do not generate verification goals (P20 diversity).
7. Canonical Scenario D can emerge through deliberate verification, not only passive perception.
