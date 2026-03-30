# S40: Belief-Backed Remote Pursuit

## Summary

Enable agents to pursue concrete targets across the place graph without breaking the existing local-combat contract. Today `EngageHostile` and `RaidTarget` are exact local combat goals: they can only commit through a lawful same-place `Attack` affordance. This spec keeps that boundary intact while allowing the planner to produce `Travel + Attack` plans when the agent holds a lawful belief about the target's current place. Pursuit is belief-backed, interruptible, and fallible: the agent may arrive and find the target gone, may abandon the chase when confidence decays, and may replan away under new danger or self-care pressure.

The pursuit infrastructure is domain-neutral. Bandit combat pursuit and guard/justice pursuit share the same `PursuitProfile` and belief helper. The goal-kind distinction (combat vs. justice) comes from candidate generation, not the profile.

## Source

Derived from the post-E18 planner reassessment that removed synthesized `Attack` root targets from combat exact goals. That fix corrected the illegal alias path but left the intended emergent chase behavior absent. `specs/E18-bandit-dynamics.md` clearly wants bandits to do more than same-place opportunistic attacks, and `specs/E19-guard-patrol.md` already relies on planner-driven travel toward believed suspect locations in the justice domain. This spec defines the shared pursuit substrate that both domains consume.

## Phase

Phase 4: Adaptation & Integration

## Crates

- `worldwake-core` (new pursuit profile)
- `worldwake-ai` (candidate generation, goal-model helpers, prerequisite-aware search integration, invalidation, traces)

## Dependencies

- E14 ✅ (belief-backed entity location and confidence are required)
- E15 ✅ (testimony/witness pathways must remain lawful inputs to remote pursuit)
- S12 ✅ (prerequisite-aware search is the canonical way to add travel before a local terminal action)
- S36 ✅ (goal dispatch declarations already centralize relevant-op surfaces)
- E18 ✅ (bandit remote pursuit is the motivating first consumer)
- E19 ✅ (guard patrol / justice pursuit is the second consumer of the shared infrastructure)

## FOUNDATIONS Alignment

- **Principle 1, Maximal Emergence Through Local Causality**: pursuit emerges from the same planning system as other behavior. No chase scripts, no authored "aggro radius" events.
- **Principle 2, No Ungrounded Triggers or Probabilities**: no `chase_chance` or hidden pursuit rolls. Agents pursue because they hold a concrete target-location belief, can travel there, and their profile says the chase is worth attempting.
- **Principle 3, Concrete State Over Abstract Scores**: pursuit is driven by concrete target identity, concrete believed location, concrete route cost, and concrete blocked-intent history. No stored "threat heat" or "aggro score." Confidence is derived from provenance and staleness, never stored.
- **Principle 7, Locality of Motion, Interaction, and Communication**: remote pursuit uses lawful information paths only. The agent must have learned a target's place via perception, witness testimony, records, or other explicit carriers already in world state.
- **Principle 14, World State Is Not Belief State**: pursuit plans are built from beliefs and may be wrong. Reaching the believed place does not guarantee target presence.
- **Principle 20, Resource-Bounded Practical Reasoning**: pursuit reuses bounded GOAP search. It is expressed as prerequisite travel to a believed place, not as a second planner or special chase loop.
- **Principle 21, Intentions Are Revisable Commitments**: pursuit is interruptible and belief-sensitive. If target-location belief changes, confidence collapses, or danger spikes, the plan is revised rather than secretly tracking omniscient truth.
- **Principle 27, Derived Summaries Are Caches, Never Truth**: `PursuitTargetBelief` contains provenance fields (`source`, `observed_tick`) from which confidence is derived on demand via `belief_confidence()`. No stored confidence value.

## Design Goals

1. **Keep attack local**: `Attack` remains a lawful same-place terminal affordance. This spec must not reintroduce synthesized combat legality.
2. **Reuse existing goal identity**: remote pursuit is an extension of `EngageHostile` and `RaidTarget`, not a parallel alias goal that duplicates hostility semantics.
3. **Use prerequisite travel, not chase scripts**: `Travel` is the only remote step. Search reaches the target's believed place first, then attempts local combat if the affordance exists.
4. **Require concrete target identity and place belief**: no pursuit of generic "danger in area." A chase starts only when the agent can name the target entity and a believed place for that entity.
5. **Make pursuit profile-driven**: pursuit willingness uses explicit per-agent parameters rather than hardcoded route-distance or confidence cutoffs.
6. **Fail honestly**: arrival at the believed place without the target should end the pursuit path and trigger normal replanning. The system must not "peek" ahead to where the target really went.
7. **Domain-neutral infrastructure**: the pursuit profile and belief helper serve both combat and justice pursuit. Goal-kind distinction comes from candidate generation.

## Deliverables

### 1. `PursuitProfile` component (`worldwake-core`)

```rust
/// Per-agent rules controlling whether a remote pursuit is attempted.
/// Used by both combat pursuit (bandits) and justice pursuit (guards).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PursuitProfile {
    /// Minimum confidence required in a target's believed place before the
    /// agent will travel there. Confidence is derived from
    /// `belief_confidence(source, staleness, policy)`, never stored.
    pub min_location_confidence: Permille,
    /// Maximum total travel ticks the agent is willing to spend on one
    /// pursuit plan.
    pub max_pursuit_travel_ticks: NonZeroU32,
}
```

This is intentionally small. The profile answers only two questions:
- is the target-place belief reliable enough?
- is the chase short enough to be worth attempting?

No morale, no abstract chase intensity, no hidden cooldown score.

### 2. Centralized pursuit-belief helper (`worldwake-ai`)

Add a declaration-owned helper surface, conceptually:

```rust
pub struct PursuitTargetBelief {
    pub target: EntityId,
    pub believed_place: EntityId,
    pub source: PerceptionSource,
    pub observed_tick: Tick,
}

pub fn pursuit_target_belief(
    view: &dyn GoalBeliefView,
    actor: EntityId,
    target: EntityId,
) -> Option<PursuitTargetBelief>;
```

Contract:
- reads the agent's belief-backed entity state only (via `known_entity_beliefs`)
- returns `None` if place is unknown, target is believed dead, or the target is already co-located
- preserves the information-path provenance (`source`, `observed_tick`) needed for decision traces and confidence derivation
- does NOT store confidence; callers derive it via `belief_confidence(&belief.source, current_tick - belief.observed_tick, &view.belief_confidence_policy(actor))`

`GoalBeliefView` already exposes `belief_confidence_policy()` (`belief_view.rs:140`), so the helper can derive confidence through the view without additional parameters.

This helper becomes the shared boundary used by candidate generation, goal-model place derivation, and invalidation. Do not duplicate target-place inference across those modules.

### 3. Candidate generation for remote pursuit opportunities (`worldwake-ai`)

Extend combat-family candidate generation to emit existing exact combat goals when the target is remote but lawfully known:

- `RaidTarget { target }` may be emitted when:
  - the bandit still has a lawful raid reason against that concrete target
  - `pursuit_target_belief()` returns a place belief
  - derived confidence (via `belief_confidence()`) is at or above `PursuitProfile.min_location_confidence`
  - route cost to that place is at or below `PursuitProfile.max_pursuit_travel_ticks`
  - no active blocked-intent record forbids retrying that target/place combination

- `EngageHostile { target }` may be emitted remotely only when the agent has a concrete hostile target with a lawfully learned place belief satisfying the same pursuit-profile constraints.

The local and remote variants remain the same `GoalKind`. The difference is whether search needs prerequisite travel before the local attack affordance appears.

### 4. Goal-model and search integration (`worldwake-ai`)

Remote pursuit must use the existing prerequisite-aware planner contract, not new op kinds:

- `GoalKind::relevant_op_kinds()` for `RaidTarget` and `EngageHostile` remains `Attack` only.
- `GroundedGoal::synthesized_root_candidate_targets()` for `PlannerOpKind::Attack` remains `NoSynthesisPath`.
- `GoalKind::goal_relevant_places()` already returns `state.effective_place(*target)` for `EngageHostile` and `RaidTarget` (`goal_model.rs:1005-1008`). Post-E14, `effective_place` reads from belief state. When the agent believes the target is at a remote place, the heuristic already guides search toward that place. No change is needed to `goal_relevant_places()` itself.
- `search_plan()` uses standard `Travel` expansions to reach that believed place, then standard same-place `Attack` affordances for the terminal step.

This yields the desired lawful shape:
- co-located target → one-step `Attack`
- remote believed target → `Travel + ... + Attack`
- target moved / belief stale → no terminal attack; plan fails or collapses into replanning without omniscient chase continuation

### 5. Invalidation and blocker semantics (`worldwake-ai`)

Remote pursuit plans must invalidate when their underlying location assumption changes:

- if the target's believed place changes, the current pursuit plan is dirty and replanned
- if target-location confidence (derived from staleness) drops below `min_location_confidence`, pursuit is dropped
- if the target is believed dead or no longer believed hostile, pursuit is dropped
- `BlockedIntentMemory` remains target/place scoped; a blocker at the old place must not suppress pursuit after the target is believed elsewhere

**Arrival failure**: when the pursuer arrives at the believed location and the target is absent, the failure records `BlockingFact::TargetGone` (already in `failure_handling.rs`). This feeds into `BlockedIntentMemory` and suppresses immediate re-pursuit at the same believed place. The agent replans normally.

### 6. Ranking and interrupt policy (`worldwake-ai`)

This spec intentionally keeps current goal-family ownership:

- `RaidTarget` retains the current raid-family ranking contract already live in the codebase
- `EngageHostile` retains the current hostile/danger-family contract
- pursuit does **not** create a new priority class or a new interrupt role

The only new behavior is that those same goals can now be planned remotely when belief-backed prerequisite travel exists.

If the project later decides `RaidTarget` should stop sharing its motive family with reactive danger combat, that is a separate ranking-spec follow-up, not part of this pursuit substrate.

### 7. Decision-trace extension (`worldwake-ai`)

Decision traces should make remote pursuit legible:

- why a remote pursuit goal was emitted or omitted
- what believed place anchored the chase
- what derived confidence value was computed and whether it met `min_location_confidence`
- whether omission was due to unknown place, low confidence, over-range route, or blocked target/place memory
- when a running pursuit was invalidated because the target-place belief changed or confidence decayed below threshold

This should reuse existing decision-trace patterns rather than add ad hoc logging.

## Guard/Justice Pursuit Reuse

The pursuit infrastructure defined above is shared by guard/justice pursuit (E19). Guards pursuing suspects use the same `PursuitProfile` and `pursuit_target_belief()` helper but emit justice-family goals rather than combat-family goals. The distinction is purely in candidate generation:

- **Combat pursuit** (E18): bandit candidate generation emits `RaidTarget`/`EngageHostile` when a hostile target is believed remote.
- **Justice pursuit** (E19): guard candidate generation emits justice goals (e.g., `ApprehendSuspect`, `InvestigateViolation`) when a suspect is believed remote.

Both domains use the same profile shape, the same belief helper, the same confidence derivation, and the same invalidation contract. No parallel pursuit infrastructure should be built for the justice domain.

## SystemFn Integration

No new `SystemFn` is required. This is planner/candidate-generation work on top of existing perception, belief update, and AI tick loops.

## Component Registration

- register `PursuitProfile` on `EntityKind::Agent`
- expose it through the belief/runtime-view boundary used by `worldwake-ai`

## Cross-System Interaction (Principle 26)

Remote pursuit composes through state, never direct system-to-system calls:

- perception / Tell / records update belief state about target location
- candidate generation reads that belief state and emits the existing combat or justice goal
- planner search uses topology and belief-backed place knowledge to produce travel prerequisites
- action execution uses normal `Travel` and `Attack` (or justice-domain equivalents)
- combat outcomes update wounds, death, hostility, blocked intents, and downstream beliefs

No bandit system calls combat directly. No combat system calls AI directly. No pursuit manager tracks agents out-of-band.

## FND-01 Section H Analysis

### H.1 Information-path analysis

| Information | Source | Path to Agent | Latency |
|-------------|--------|--------------|---------|
| "target X is at place Y" | direct observation | immediate perception into belief store | 0 ticks |
| "target X was seen at place Y" | witness testimony | Tell or other lawful social carrier | travel + co-location latency |
| "target X is no longer at Y" | arrival/re-observation or new testimony | perception refresh / Tell | variable |
| "route to Y is too long / blocked" | topology + planner snapshot | local planner read | 0 ticks at planning time |

Pursuit never consults authoritative world truth for remote target position. It only reads the agent's lawful belief state plus public topology.

### H.2 Positive-feedback analysis

The main loop is a **contact reinforcement loop**:
- attacker observes target
- target flees
- attacker pursues
- renewed co-location may produce more combat and more pursuit

This is desirable emergence, but left unchecked it can become implausible map-wide tail-chasing.

### H.3 Concrete dampeners

1. **Belief confidence floor**: low-confidence target-place beliefs cannot trigger pursuit.
2. **Travel budget cap**: `max_pursuit_travel_ticks` bounds chase radius through concrete route cost.
3. **Arrival failure**: if the target is gone when the pursuer arrives, no attack occurs and the chase ends without omniscient continuation. Records `BlockingFact::TargetGone`.
4. **Normal survival/danger interruption**: wounded, thirsty, or threatened pursuers still abandon chases under existing ranking/interrupt rules.
5. **Blocked target/place memory**: repeated failed or overly risky pursuit attempts suppress immediate retry at the same believed place.
6. **Natural confidence decay during travel**: `belief_confidence()` applies `staleness_penalty_per_tick` each tick. During multi-tick travel, the target-location belief grows staler. If derived confidence drops below `min_location_confidence` before arrival, the pursuit becomes invalid and the plan is abandoned. This makes long pursuits self-limiting without any artificial leash distance — the longer the chase, the less the agent trusts its information.

These are all concrete world or profile constraints, not abstract "AI leash distance" knobs hidden in planner code.

### H.4 Stored state vs. derived read-model list

- **Stored**:
  - `PursuitProfile`
  - existing belief-store entity-location beliefs (`BelievedEntityState.last_known_place`, `.source`, `.observed_tick`)
  - existing hostility relations / justice records / blocked intents
- **Derived**:
  - `PursuitTargetBelief` (extracted from `BelievedEntityState`)
  - target-location confidence (derived via `belief_confidence()` from source + staleness + policy)
  - "remote pursuit available" candidate emission
  - prerequisite places for remote goals
  - omission reasons in traces

### H.5 Quantities conserved

N/A. Pursuit does not create or destroy quantities. Travel and Attack have their own resource costs specified in their respective action definitions. No new conservation concern.

### H.6 Scarce capacities

Agent body is occupied during pursuit travel (implicit from Travel action occupancy contract). One pursuit plan at a time per agent (standard planner single-plan constraint). No new exclusive affordance or reservation is introduced.

### H.7 Partial failures and aftermath

Three failure modes:

1. **Arrival failure**: pursuer reaches believed place, target is absent. Records `BlockingFact::TargetGone` in `BlockedIntentMemory`. Agent replans via normal failure-handling pipeline. No omniscient continuation.
2. **Mid-travel belief decay**: during multi-tick travel, staleness increases and derived confidence may drop below `min_location_confidence`. Pursuit becomes invalid. Plan abandoned; agent replans from current position.
3. **Combat outcome — target flees**: pursuer engages, target flees to adjacent place. Pursuer observes departure direction, gaining a fresh belief about the target's new location. This can trigger a new pursuit cycle, bounded by remaining travel budget and the new belief's confidence. This is expected emergence (see golden tests).

### H.8 Temporal resolution

Uses existing tick system. No new temporal model. Travel duration comes from topology edge costs. Belief staleness increments each tick via `current_tick - observed_tick`. The `staleness_penalty_per_tick` in `BeliefConfidencePolicy` governs decay rate.

### H.9 Boundary conditions

Pursuit toward map-edge places or dead-end topology is bounded by `max_pursuit_travel_ticks` and topology connectivity. If the target is believed at a reachable place, the agent travels there normally. If the target is believed at an unreachable place (no path), candidate generation filters it out (route cost check fails). Dead-end arrival follows the standard arrival-failure path if the target is absent.

### H.10 Save/load/replay

`PursuitProfile` is a Component on `EntityKind::Agent`. It survives save/load via standard serde. Pursuit plans are transient planner state, not persisted; they are reconstructed from beliefs on reload like all other plans. `PursuitTargetBelief` is a derived read-model, not stored. No new persistence requirement.

## Tests

### Focused tests

- [ ] Remote `RaidTarget` candidate emitted when target place is known, derived confidence is high enough, and route cost is within pursuit profile
- [ ] Remote `RaidTarget` not emitted when target place is unknown
- [ ] Remote pursuit not emitted when derived confidence is below `min_location_confidence`
- [ ] Remote pursuit not emitted when route cost exceeds `max_pursuit_travel_ticks`
- [ ] `RaidTarget` search returns `Travel + Attack` when remote target place is believed and the target remains there
- [ ] `EngageHostile` search returns `Travel + Attack` under the same lawful remote conditions
- [ ] `Attack` root synthesis remains absent for remote hostile goals
- [ ] Pursuit plan invalidates when believed target place changes
- [ ] Pursuit plan invalidates when derived confidence decays below `min_location_confidence` during multi-tick travel
- [ ] Arrival at believed place without target records `BlockingFact::TargetGone` and triggers replanning
- [ ] Blocked target/place memory suppresses repeat pursuit only for that same believed place

### Golden tests

- [ ] Bandit witnesses a traveler leave for an adjacent place, pursues, and attacks only if real co-location is re-established there
- [ ] Bandit pursues stale target information to a last-known place, fails to reacquire, and falls back to normal replanning rather than omnisciently continuing
- [ ] Combat-flee-re-pursue: bandit engages target, target flees, bandit observes departure direction and initiates fresh pursuit to adjacent place (bounded by travel budget and confidence floor)
- [ ] Deterministic replay companions for all golden tests

## Acceptance Criteria

1. `RaidTarget` and `EngageHostile` can plan remote pursuit through belief-backed prerequisite travel.
2. `Attack` remains a lawful same-place terminal affordance; no synthesized combat alias path is reintroduced.
3. Pursuit uses concrete target identity plus believed place, never generic area hostility.
4. Pursuit is profile-driven through `PursuitProfile`, not hardcoded confidence or distance cutoffs.
5. Arrival at a stale target place produces honest failure/replanning, not omniscient continuation.
6. Existing ranking and interrupt hierarchies remain intact; this spec extends planning reach, not goal-family semantics.
7. All existing AI and golden suites continue to pass.
8. Guard/justice pursuit reuses `PursuitProfile` and `pursuit_target_belief()` without building parallel infrastructure. Goal-kind distinction comes from candidate generation only.
9. Target-location confidence is always derived via `belief_confidence()`, never stored in `PursuitTargetBelief` or elsewhere.
