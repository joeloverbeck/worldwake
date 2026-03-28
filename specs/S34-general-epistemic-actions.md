# S34: General Epistemic Actions

## Summary

Extend the action framework with social epistemic actions and grounded-goal stale-evidence barriers. Currently investigation is narrowly scoped to S27 violation response. Agents can already refresh many stale facts through lawful arrival perception, but they still need explicit social querying and a clean planner contract that treats arrival-observable stale facts as travel-side progress barriers rather than as forced post-arrival `verify_belief` steps. `ask_witness` is the live explicit epistemic action in that canonical path. `verify_belief` remains a lower-layer action substrate only if future fact classes prove a distinct inspection-only contract.

## Source

Derived from ChatGPT architecture review WW-AI-003 (Expectations, discrepancies, and epistemic actions), filtered to the epistemic action component only. The full expectation-registration system proposed by ChatGPT is not needed -- S27's violation detection already handles expectation-vs-observation mismatches. What's missing is the agent's ability to *deliberately seek information* rather than only *reactively discover mismatches*.

The original proposal included three actions (`inspect_place`, `ask_witness`, `verify_location`). `inspect_place` was dropped during reassessment: passive perception (E14) already updates beliefs for co-located entities each tick. Deliberate information-seeking should be targeted (P5: simulate carriers of consequence, not decorative realism; P18: resource-bounded practical reasoning). Later reassessment showed that the currently modeled `EntityLocation` and `SupplyAvailability` facts are arrival-observable, so the canonical live contract does not force a `verify_belief` step for them after travel.

## Phase

Phase 3+: AI Architecture Overhaul, Step 13.5 Wave 5

## Crates

- `worldwake-core` (new verification subject enum, new disposition profile)
- `worldwake-sim` (action def registration, new action domain, new payload variants)
- `worldwake-systems` (action handlers)
- `worldwake-ai` (grounded-goal stale-evidence derivation, planner ops, search)

## Dependencies

- S27 (expectation-violation goals -- provides violation detection and `InvestigateViolation` this spec extends)
- E14 (perception & belief -- provides `AgentBeliefStore`, `PerceptionSource`, `BelievedEntityState`, `belief_confidence()`)
- E15c (conversation memory -- provides Tell mechanics and `ToldBeliefMemory`/`HeardBeliefMemory` this spec's `ask_witness` reuses)

## FOUNDATIONS Alignment

- **P13** (Knowledge Acquired Locally): Knowledge is acquired through lawful local carriers. In the live contract that means travel plus ordinary arrival perception for arrival-observable facts, and `ask_witness` for social transfer.
- **P15** (Surprise From Violated Expectation): Agents should be able to proactively verify beliefs they're about to act on, especially when confidence is low. This creates the expectation -> verification -> surprise chain.
- **P7** (Locality): Arrival-observable stale facts require travel to the believed place and refresh through ordinary co-location perception. `ask_witness` requires a co-located informant.
- **P5** (Carriers of Consequence): Deliberate verification is targeted -- the agent checks a specific belief it cares about, not "everything at a place." Passive perception handles broad discovery. This avoids decorative realism (broad scans that produce noise without consequence).
- **P8** (Every Action Has Cost): Both epistemic actions have duration, preconditions, and occupancy. Information is not free. Time spent verifying is time not spent producing, trading, or fighting.
- **P18** (Resource-Bounded Reasoning): Agents verify beliefs relevant to their current priorities. A farmer does not verify whether a distant dragon is still at its cave unless planning to go there. Candidate generation ties verification to goal-relevant beliefs.
- **P20** (Agent Diversity): Per-agent `VerificationDispositionProfile` controls verification thresholds and durations. Agents without the profile rely on passive perception only.
- **Scenario D**: Rumor -> Travel -> Empty Source -> Discovery -> Belief Correction -> Replan. In the live contract, the discovery step is the lawful arrival observation itself; the planner barrier is the travel step to the believed place.

## Design Goals

1. **Deliberate information seeking**: Agents can choose to verify beliefs before committing to costly plans.
2. **Cost-bearing**: All epistemic actions have duration, preconditions, and occupancy (P8). Information is not free.
3. **Belief-mediated results**: Epistemic actions update the agent's belief store, not authoritative world state (P12).
4. **Profile-driven behavior**: Per-agent disposition profiles control verification thresholds and action durations (P20).
5. **Planner integration**: The GOAP planner can include travel-side stale-evidence barriers and explicit `ask_witness` barriers under an originating goal when confidence is low.
6. **Targeted verification**: Agents seek specific goal-relevant information rather than broad sweeps of locations (P5, P18).

## Deliverables

### 1. `VerificationSubject` enum (worldwake-core)

```rust
/// Subject of a proactive belief verification.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum VerificationSubject {
    /// Verify whether a specific entity is at the believed place.
    EntityLocation {
        entity: EntityId,
        place: EntityId,
    },
    /// Verify whether a resource source still has available supply.
    SupplyAvailability {
        commodity: CommodityKind,
        source: EntityId,
        place: EntityId,
    },
}
```

### 2. `VerificationDispositionProfile` component (worldwake-core)

```rust
/// Per-agent parameters controlling epistemic action behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationDispositionProfile {
    /// Beliefs below this confidence trigger grounded-goal epistemic barriers.
    /// Permille(400) = 40% confidence threshold.
    pub belief_verification_threshold: Permille,
    /// Duration in ticks for the verify_belief action.
    pub verify_belief_duration_ticks: NonZeroU32,
    /// Duration in ticks for the ask_witness action.
    pub witness_query_duration_ticks: NonZeroU32,
    /// Ticks before an ask-memory entry expires (deduplication window for ask_witness).
    pub ask_memory_retention_ticks: u32,
}
```

Registered on agents via component schema. Agents without this profile do not derive deliberate epistemic barriers (they rely on passive perception only -- P20 diversity).

### 3. Action definitions (worldwake-sim, worldwake-systems)

#### `verify_belief`

- **Preconditions**: Actor alive, not incapacitated, at the place being verified.
- **Duration**: `VerificationDispositionProfile::verify_belief_duration_ticks`.
- **Interruptibility**: FreelyInterruptible.
- **On commit**: Dispatch based on payload variant:
  - `EntityLocation { entity, place }`: Check whether the specific entity the actor expects to find at this place is present. If present, update the actor's `BelievedEntityState` for `entity` with `observed_tick = current_tick` and `source = DirectObservation`. If absent, generate `ViolationKind::EntityMissing { entity, expected_place: place }` record in the actor's `ViolationMemory`.
  - `SupplyAvailability { commodity, source, place }`: Check whether the resource source entity at this place still has `available_quantity > 0` for the specified commodity. If productive, update belief for the source entity with fresh `DirectObservation` (`observed_tick = current_tick`). If depleted, generate `ViolationKind::SupplyDepleted { commodity, source, place }` record.
- **Visibility**: SamePlace.
- **Payload**:

```rust
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct VerifyBeliefPayload {
    pub subject: VerificationSubject,
}
```

- **Partial failure / aftermath** (P9): If the target place changes accessibility mid-action (e.g., place destroyed), the action aborts. The agent retains its prior (stale) belief -- no update, no violation recorded. The spent ticks are consumed (P8: cost is real). If the subject entity or source entity is destroyed mid-action, on commit the agent observes absence and records the appropriate violation.

#### `ask_witness`

- **Preconditions**: Actor alive, not incapacitated. Target agent alive, co-located, not incapacitated.
- **Duration**: `VerificationDispositionProfile::witness_query_duration_ticks`.
- **Interruptibility**: FreelyInterruptible.
- **On commit**: Transfer a subset of the target's `AgentBeliefStore.known_entities` entries to the actor, filtered by topic (the entity or commodity the actor is asking about). Transferred beliefs carry `PerceptionSource::Report { from: target, chain_len: 1 }`. If the target has no relevant beliefs, the commit still succeeds (the absence of information is a result -- the agent learns "this witness does not know"). Respects E15c conversation memory -- suppresses re-asking the same target about the same topic within the retention window.
- **Visibility**: SamePlace (other co-located agents can observe the asking).
- **Payload**:

```rust
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AskWitnessPayload {
    pub target: EntityId,
    /// At least one of these must be Some. Validated at action start.
    pub topic_entity: Option<EntityId>,
    pub topic_commodity: Option<CommodityKind>,
}
```

- **Payload validation**: `start_ask_witness` must reject payloads where both `topic_entity` and `topic_commodity` are `None`. Implemented as an authoritative payload validator (same pattern as `validate_investigate_payload_authoritatively` in S27).

- **Ask-deduplication memory**: Reuse the existing `TellMemoryKey` / conversation memory infrastructure. When an `ask_witness` action commits:
  - The **asker** records a `HeardBeliefMemory` entry with key `TellMemoryKey { counterparty: target, topic }` (where topic is synthesized as `TellTopic::EntityBelief { subject }` for entity queries, or the first matching entity belief topic for commodity queries).
  - The **target** records a `ToldBeliefMemory` entry with the same key structure.
  - This reuses the existing `enforce_conversation_memory` retention/eviction logic from E15c. The `ask_memory_retention_ticks` on the profile controls how long grounded-goal barrier synthesis suppresses re-asking, while the underlying memory entries use the standard Tell memory lifecycle.
  - **Rationale**: Asking is a conversational exchange that produces the same memory artifacts as telling. Adding a separate `AskMemory` system would create a dual representation of the same concept (P26: no backward compatibility / dual representations).

- **Partial failure / aftermath** (P9): If the target agent moves away, dies, or becomes incapacitated mid-action, the action aborts. No beliefs are transferred. Spent ticks are consumed. The abort does not record any memory entry (the conversation did not complete). The agent has spent time and gained nothing, which may shift priorities for the next planning cycle.

### 4. Grounded-goal epistemic barrier substrate (worldwake-ai)

Deliberate verification is not modeled as a separate `GoalKind`. The canonical planner contract is:

- the originating grounded goal remains the selected top-level intention
- stale evidence on that grounded goal derives one or more subject/place requirements inside `worldwake-ai`
- search exposes only lawful epistemic steps for those subjects:
  - `AskWitness` when a co-located witness payload matches the stale subject
  - `Travel` to the believed place when the fact is arrival-observable
- the epistemic step is an explicit progress barrier, so planning stops at the witness exchange or arrival boundary and replans after the world yields new information

This substrate belongs to the grounded-goal/search layer, not `worldwake-core`, because it depends on concrete candidate evidence, anchor, and current belief reads rather than on abstract goal identity alone.

### 5. PlannerOpKind additions (worldwake-ai)

```rust
PlannerOpKind::AskWitness,
```

Planner semantics:
- `AskWitness`: Appears only when the current grounded goal carries a matching stale verification subject and a co-located witness payload is available. Barrier: yes for the same reason.
- `Travel`: remains a normal planner op, but when it reaches the believed place for an arrival-observable stale fact it becomes the progress barrier because new lawful perception can invalidate the stale branch.

The relevant op set is grounded-goal-specific rather than a separate `GoalKind` family. Productive goals keep their normal operator families, but stale-subject handling augments those sets with `AskWitness` and travel-side progress-barrier semantics when a matching epistemic barrier is active.

### 6. Candidate generation (worldwake-ai)

No standalone `emit_verify_belief_goals()` pass remains. Instead, after ordinary goal candidates are emitted, the AI derives stale-evidence barrier requirements from each grounded goal's evidence:

1. **Guard**: Return immediately if agent lacks `VerificationDispositionProfile`.

2. **Scan existing candidates**: For each already-emitted `GroundedGoal` in the candidate list, identify belief dependencies:
   - Extract `evidence_entities` and `evidence_places` from the candidate.
   - For each entity in `evidence_entities`, look up the agent's `BelievedEntityState`. If the belief's `belief_confidence(source, staleness_ticks, policy) < profile.belief_verification_threshold`, this entity becomes a stale-subject barrier candidate for that grounded goal.

3. **Determine `VerificationSubject`**:
   - If the low-confidence belief concerns an entity's location (entity has `last_known_place: Some(place)`), derive `EntityLocation { entity, place }`.
   - If the low-confidence belief concerns a resource source (entity has `resource_source: Some(_)` in the believed state), derive `SupplyAvailability { commodity, source: entity, place }` where `commodity` is the source's commodity and `place` is the source's `last_known_place`.

4. **Deduplication**: Deduplicate by stale subject within the grounded-goal barrier substrate so the same stale prerequisite does not fan out into duplicate barrier ops for one grounded goal.

5. **Conversation memory suppression for `ask_witness`**: When deriving stale-subject barriers, also check whether the agent has recently asked a co-located witness about the same topic (via `HeardBeliefMemory` entries). If a matching entry exists with `told_tick + profile.ask_memory_retention_ticks > current_tick`, suppress `AskWitness` as a planner option for that topic (the affordance system handles this -- the `ask_witness` affordance payload enumerator skips recently-asked targets).

6. **Consumption**: Search root-candidate synthesis consumes those stale subjects and exposes only the matching epistemic planner ops under the originating goal. No rival top-level verification candidate is emitted, and no post-arrival `VerifyBelief` step is synthesized for arrival-observable facts.

### 7. Ranking (worldwake-ai)

No standalone verification ranking family remains. Deliberate verification inherits the priority of the originating grounded goal because it is a prerequisite barrier within that goal's plan, not a competing top-level desire. The verification disposition profile still governs *when* stale evidence becomes barrier-worthy and how long the actions take, but not a separate ranking motive.

### 8. ActionDomain extension (worldwake-sim)

Add `ActionDomain::Epistemic` to categorize `verify_belief` and `ask_witness` actions.

## Component Registration

- `VerificationDispositionProfile`: Register on `EntityKind::Agent` in component schema (same pattern as `ViolationDispositionProfile`).
- `VerificationSubject`: No registration needed (embedded in `GoalKind` and `ActionPayload`).
- Action defs: Register `verify_belief` and `ask_witness` in `ActionDefRegistry` via `register_verify_belief_action()` and `register_ask_witness_action()` in worldwake-systems, called from `register_all_actions()` in `action_registry.rs`.
- Handlers: Register in `ActionHandlerRegistry` with `start_*/tick_*/commit_*/abort_*` functions plus `enumerate_*_payloads` and `validate_*_payload_authoritatively` validators (same pattern as Tell and Investigate actions).
- Payload variants: Add `VerifyBelief(VerifyBeliefPayload)` and `AskWitness(AskWitnessPayload)` to the `ActionPayload` enum with corresponding accessor methods.

## SystemFn Integration

No new system function. Epistemic actions are handled entirely through the action framework (handlers registered in `ActionHandlerRegistry`). Candidate generation and ranking changes are within the existing AI pipeline functions (`generate_candidates`, `rank_candidates`).

## Cross-System Interactions (P24)

Epistemic actions interact with other systems exclusively through state mutation (P24: systems interact through state, not through each other):

- **Perception (E14)**: Epistemic action handlers read the authoritative world to determine what the agent observes, then write to the agent's `AgentBeliefStore`. They do not call perception system functions directly.
- **Violation (S27)**: `verify_belief` writes `RecordedViolation` entries to `ViolationMemory` when expectations are mismatched. The existing violation candidate generation (`emit_recorded_violation_candidates`) then picks these up in subsequent ticks.
- **Conversation (E15c)**: `ask_witness` writes `ToldBeliefMemory`/`HeardBeliefMemory` entries using the same structures the Tell action uses. No direct calls to Tell handler functions.

## FND-01 Section H Analysis

### Information-path analysis

Agent holds stale belief -> ordinary grounded goal is emitted -> grounded-goal stale-evidence derivation detects low confidence -> planner exposes `Travel` to the believed place or `AskWitness` as an explicit barrier step under that same goal -> arrival perception or witness report updates the belief store -> violations are recorded if expectations are mismatched.

For `ask_witness`: agent queries co-located agent -> target's beliefs transferred with `Report { chain_len: 1 }` provenance -> actor's belief store updated. All information paths are local and traceable (P7).

### Positive-feedback analysis

No amplifying loops. Epistemic actions consume time (duration-bearing), which limits how often agents verify. More verification -> less time for production/trade -> natural cost pressure against over-verification.

### Concrete dampeners

Time cost of epistemic actions is the physical dampener. Agents who verify everything accomplish nothing else. `belief_verification_threshold` controls when stale evidence becomes barrier-worthy, and action durations control how expensive verification is. Because verification is a prerequisite barrier inside an originating goal rather than a rival top-level goal, no extra ranking dampener is needed.

### Stored state vs. derived read-model list

- **Stored**: `VerificationDispositionProfile` (component). Action defs (registry). Updated `BelievedEntityState` entries (existing belief store). `RecordedViolation` entries (existing violation memory). `ToldBeliefMemory`/`HeardBeliefMemory` entries (existing conversation memory).
- **Derived**: grounded-goal stale-subject barrier requirements (recomputed from current belief confidence and grounded-goal evidence). Confidence scores (computed from provenance + age via `belief_confidence()`, never stored).

### Contention and scarcity (P28 item 5)

Epistemic actions do not introduce new scarce capacities, exclusive affordances, reservations, queues, or claims. `verify_belief` observes a place -- multiple agents can verify the same place simultaneously. `ask_witness` occupies both the actor and the target for the conversation duration, which is the same occupancy model as the existing `tell` action (E15c). No new contention mechanism is needed.

### Partial failures and aftermath (P28 item 6)

- **`verify_belief`**: If the target place changes accessibility mid-action, the action aborts. The agent retains its prior (stale) belief. Spent ticks are consumed (P8). If the subject entity is destroyed during the action, on commit the agent observes absence and records the appropriate `ViolationKind`.
- **`ask_witness`**: If the target agent moves away, dies, or becomes incapacitated mid-action, the action aborts. No beliefs are transferred. Spent ticks are consumed. The abort does not record any memory entry (the conversation did not complete).
- Both abort paths leave aftermath: the agent has spent time and gained nothing, which may shift priorities for the next planning cycle. This is consequential state, not a silent retry (P9).

### Save/load and replay (P28 item 11)

`VerificationDispositionProfile` is a standard serde-compatible component. `VerificationSubject`, `VerifyBeliefPayload`, and `AskWitnessPayload` all derive `Serialize`/`Deserialize`. All new types compose with the existing bincode pipeline. Ask-deduplication reuses `ToldBeliefMemory`/`HeardBeliefMemory` which are already save/load compatible. No new persistence requirements beyond component registration. Deterministic replay is preserved because all new actions use `DeterministicRng` and `BTreeMap`-based storage.

## Tests

### Focused tests

- [ ] `verify_belief` (EntityLocation): confirms present entity with fresh `DirectObservation` (`observed_tick = current_tick`)
- [ ] `verify_belief` (EntityLocation): generates `EntityMissing` violation for absent entity
- [ ] `verify_belief` (SupplyAvailability): confirms productive source with fresh observation
- [ ] `verify_belief` (SupplyAvailability): generates `SupplyDepleted` violation for exhausted source
- [ ] `verify_belief` aborts cleanly if place accessibility changes mid-action; agent retains stale belief
- [ ] `verify_belief` records violation if subject entity destroyed mid-action (absent on commit)
- [ ] `ask_witness` transfers target's beliefs with `Report { chain_len: 1 }` provenance
- [ ] `ask_witness` commits successfully when target has no relevant beliefs (no-op transfer)
- [ ] `ask_witness` respects conversation memory (no re-ask within retention window)
- [ ] `ask_witness` rejects payload where both `topic_entity` and `topic_commodity` are `None`
- [ ] `ask_witness` aborts if target moves away during action; no beliefs transferred, no memory recorded
- [ ] `ask_witness` records `HeardBeliefMemory` (asker) and `ToldBeliefMemory` (target) on commit
- [ ] Grounded goal with stale evidence derives epistemic barrier subjects only when belief confidence is below threshold
- [ ] Agents lacking `VerificationDispositionProfile` do not derive deliberate epistemic barriers
- [ ] Barrier derivation scans already-emitted grounded goals for belief dependencies
- [ ] Barrier derivation deduplicates repeated stale subjects within one grounded goal
- [ ] Resource-source stale evidence derives `SupplyAvailability` barrier subjects
- [ ] Search constructs a travel-to-place progress barrier under a remote arrival-observable originating goal
- [ ] Search constructs `AskWitness` as an explicit barrier path under an originating goal with a matching co-located witness
- [ ] Originating goal remains selected while epistemic barrier steps are inserted; no standalone verification goal is emitted

### Golden tests

- [ ] Scenario D variant: Agent hears rumor about commodity at distant source -> originating restock goal treats stale source belief as epistemic barrier -> travels -> refreshes through lawful co-located observation -> contradiction recorded -> replans to alternative source
- [ ] Agent asks co-located witness about entity location -> receives report-sourced belief -> uses it to plan travel to entity
- [ ] Agent with stale belief about entity location keeps its originating goal while epistemic barrier handling refreshes the belief and proceeds
- [ ] Deterministic replay companions for each golden

## Acceptance Criteria

1. Agents can deliberately seek information through lawful local carriers, including `ask_witness` and travel-side arrival refresh.
2. Explicit epistemic actions that remain in the live contract have duration, preconditions, and occupancy -- information is not free (P8).
3. `EntityLocation` and `SupplyAvailability` currently use the arrival-observable planner contract rather than a forced post-arrival `verify_belief` step.
4. Deliberate epistemic barriers are derived only when belief confidence is below agent-specific threshold.
5. Barrier requirements are generated by scanning already-emitted grounded goals for low-confidence belief dependencies (goal-relevant, not tick-based scan).
6. Epistemic action results update belief stores with proper provenance, not authoritative world state (P12).
7. Violations detected during epistemic actions integrate with existing S27 violation memory.
8. `ask_witness` reuses Tell conversation memory (`ToldBeliefMemory`/`HeardBeliefMemory`) for deduplication -- no separate memory system (P26).
9. `ask_witness` validates that at least one topic field (`topic_entity` or `topic_commodity`) is populated.
10. Agents without `VerificationDispositionProfile` do not derive deliberate verification barriers (P20 diversity).
11. Canonical Scenario D can emerge through lawful arrival refresh without any duplicate post-arrival verification step.
12. All new types are serde-compatible and survive save/load/replay without changing world meaning (P11).
