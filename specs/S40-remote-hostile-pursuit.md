# S40: Belief-Backed Remote Hostile Pursuit

## Summary

Enable agents to pursue concrete hostile targets across the place graph without breaking the existing local-combat contract. Today `EngageHostile` and `RaidTarget` are exact local combat goals: they can only commit through a lawful same-place `Attack` affordance. This spec keeps that boundary intact while allowing the planner to produce `Travel + Attack` plans when the agent holds a lawful belief about the hostile target's current place. Pursuit is belief-backed, interruptible, and fallible: the agent may arrive and find the target gone, may abandon the chase when confidence decays, and may replan away under new danger or self-care pressure.

## Source

Derived from the post-E18 planner reassessment that removed synthesized `Attack` root targets from combat exact goals. That fix corrected the illegal alias path but left the intended emergent chase behavior absent. `specs/E18-bandit-dynamics.md` clearly wants bandits to do more than same-place opportunistic attacks, and `specs/E19-guard-patrol.md` already relies on planner-driven travel toward believed suspect locations in the justice domain. This spec defines the combat-side version of that contract.

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
- E18 (bandit remote pursuit is the motivating first consumer)

## FOUNDATIONS Alignment

- **Principle 1, Maximal Emergence Through Local Causality**: pursuit emerges from the same planning system as other behavior. No chase scripts, no authored “aggro radius” events.
- **Principle 2, No Ungrounded Triggers or Probabilities**: no `chase_chance` or hidden pursuit rolls. Agents pursue because they hold a concrete target-location belief, can travel there, and their profile says the chase is worth attempting.
- **Principle 3, Concrete State Over Abstract Scores**: pursuit is driven by concrete target identity, concrete believed location, concrete route cost, and concrete blocked-intent history. No stored “threat heat” or “aggro score.”
- **Principle 7, Locality of Motion, Interaction, and Communication**: remote pursuit uses lawful information paths only. The agent must have learned a target's place via perception, witness testimony, records, or other explicit carriers already in world state.
- **Principle 12, World State Is Not Belief State**: pursuit plans are built from beliefs and may be wrong. Reaching the believed place does not guarantee target presence.
- **Principle 20, Resource-Bounded Practical Reasoning**: pursuit reuses bounded GOAP search. It is expressed as prerequisite travel to a believed place, not as a second planner or special chase loop.
- **Principle 21, Intentions Are Revisable Commitments**: pursuit is interruptible and belief-sensitive. If target-location belief changes, confidence collapses, or danger spikes, the plan is revised rather than secretly tracking omniscient truth.

## Design Goals

1. **Keep attack local**: `Attack` remains a lawful same-place terminal affordance. This spec must not reintroduce synthesized combat legality.
2. **Reuse existing goal identity**: remote pursuit is an extension of `EngageHostile` and `RaidTarget`, not a parallel alias goal that duplicates hostility semantics.
3. **Use prerequisite travel, not chase scripts**: `Travel` is the only remote step. Search reaches the hostile's believed place first, then attempts local combat if the affordance exists.
4. **Require concrete target identity and place belief**: no pursuit of generic “danger in area.” A chase starts only when the agent can name the target entity and a believed place for that entity.
5. **Make pursuit profile-driven**: pursuit willingness uses explicit per-agent parameters rather than hardcoded route-distance or confidence cutoffs.
6. **Fail honestly**: arrival at the believed place without the target should end the pursuit path and trigger normal replanning. The system must not “peek” ahead to where the target really went.

## Deliverables

### 1. `HostilePursuitProfile` component (`worldwake-core`)

```rust
/// Per-agent rules controlling whether a remote hostile chase is attempted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostilePursuitProfile {
    /// Minimum confidence required in a target's believed place before the
    /// agent will travel there for combat.
    pub min_location_confidence: Permille,
    /// Maximum total travel ticks the agent is willing to spend on one hostile
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
    pub confidence: Permille,
    pub source: PerceptionSource,
}

pub fn hostile_pursuit_target_belief(
    view: &dyn GoalBeliefView,
    actor: EntityId,
    target: EntityId,
) -> Option<PursuitTargetBelief>;
```

Contract:
- reads the agent's belief-backed entity state only
- returns `None` if place is unknown, target is believed dead, or the target is already co-located
- preserves the information-path provenance needed for decision traces

This helper becomes the shared boundary used by candidate generation, goal-model place derivation, and invalidation. Do not duplicate target-place inference across those modules.

### 3. Candidate generation for remote hostile opportunities (`worldwake-ai`)

Extend combat-family candidate generation to emit existing exact combat goals when the target is remote but lawfully known:

- `RaidTarget { target }` may be emitted when:
  - the bandit still has a lawful raid reason against that concrete target
  - `hostile_pursuit_target_belief()` returns a place with confidence at or above `HostilePursuitProfile.min_location_confidence`
  - route cost to that place is at or below `HostilePursuitProfile.max_pursuit_travel_ticks`
  - no active blocked-intent record forbids retrying that target/place combination

- `EngageHostile { target }` may be emitted remotely only when the agent has a concrete hostile target with a lawfully learned place belief satisfying the same pursuit-profile constraints.

The local and remote variants remain the same `GoalKind`. The difference is whether search needs prerequisite travel before the local attack affordance appears.

### 4. Goal-model and search integration (`worldwake-ai`)

Remote pursuit must use the existing prerequisite-aware planner contract, not new op kinds:

- `GoalKind::relevant_op_kinds()` for `RaidTarget` and `EngageHostile` remains `Attack` only.
- `GroundedGoal::synthesized_root_candidate_targets()` for `PlannerOpKind::Attack` remains `NoSynthesisPath`.
- `GoalKind::goal_relevant_places()` and/or `GoalKind::prerequisite_places()` gain the target's believed place for remote hostile goals.
- `search_plan()` uses standard `Travel` expansions to reach that believed place, then standard same-place `Attack` affordances for the terminal step.

This yields the desired lawful shape:
- co-located target -> one-step `Attack`
- remote believed target -> `Travel + ... + Attack`
- target moved / belief stale -> no terminal attack; plan fails or collapses into replanning without omniscient chase continuation

### 5. Invalidation and blocker semantics (`worldwake-ai`)

Remote pursuit plans must invalidate when their underlying location assumption changes:

- if the target's believed place changes, the current pursuit plan is dirty and replanned
- if target-location confidence drops below `min_location_confidence`, pursuit is dropped
- if the target is believed dead or no longer believed hostile, pursuit is dropped
- `BlockedIntentMemory` remains target/place scoped; a blocker at the old place must not suppress pursuit after the target is believed elsewhere

This is the chase equivalent of the existing “assumptions changed, plan no longer valid” contract.

### 6. Ranking and interrupt policy (`worldwake-ai`)

This spec intentionally keeps current goal-family ownership:

- `RaidTarget` retains the current raid-family ranking contract already live in the codebase
- `EngageHostile` retains the current hostile/danger-family contract
- pursuit does **not** create a new priority class or a new interrupt role

The only new behavior is that those same goals can now be planned remotely when belief-backed prerequisite travel exists.

If the project later decides `RaidTarget` should stop sharing its motive family with reactive danger combat, that is a separate ranking-spec follow-up, not part of this pursuit substrate.

### 7. Decision-trace extension (`worldwake-ai`)

Decision traces should make remote pursuit legible:

- why a remote hostile goal was emitted or omitted
- what believed place anchored the chase
- whether omission was due to unknown place, low confidence, over-range route, or blocked target/place memory
- when a running pursuit was invalidated because the target-place belief changed

This should reuse existing decision-trace patterns rather than add ad hoc logging.

## SystemFn Integration

No new `SystemFn` is required. This is planner/candidate-generation work on top of existing perception, belief update, and AI tick loops.

## Component Registration

- register `HostilePursuitProfile` on `EntityKind::Agent`
- expose it through the belief/runtime-view boundary used by `worldwake-ai`

## Cross-System Interaction (Principle 12)

Remote hostile pursuit composes through state, never direct system-to-system calls:

- perception / Tell / records update belief state about target location
- candidate generation reads that belief state and emits the existing hostile goal
- planner search uses topology and belief-backed place knowledge to produce travel prerequisites
- action execution uses normal `Travel` and `Attack`
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
3. **Arrival failure**: if the target is gone when the pursuer arrives, no attack occurs and the chase ends without omniscient continuation.
4. **Normal survival/danger interruption**: wounded, thirsty, or threatened pursuers still abandon chases under existing ranking/interrupt rules.
5. **Blocked target/place memory**: repeated failed or overly risky pursuit attempts suppress immediate retry at the same believed place.

These are all concrete world or profile constraints, not abstract “AI leash distance” knobs hidden in planner code.

### H.4 Stored state vs. derived read-model list

- **Stored**:
  - `HostilePursuitProfile`
  - existing belief-store entity-location beliefs
  - existing hostility relations / justice records / blocked intents
- **Derived**:
  - `PursuitTargetBelief`
  - “remote pursuit available” candidate emission
  - prerequisite places for remote hostile goals
  - omission reasons in traces

## Tests

### Focused tests

- [ ] Remote `RaidTarget` candidate emitted when target place is known, confidence is high enough, and route cost is within pursuit profile
- [ ] Remote `RaidTarget` not emitted when target place is unknown
- [ ] Remote hostile pursuit not emitted when confidence is below `min_location_confidence`
- [ ] Remote hostile pursuit not emitted when route cost exceeds `max_pursuit_travel_ticks`
- [ ] `RaidTarget` search returns `Travel + Attack` when remote target place is believed and the target remains there
- [ ] `EngageHostile` search returns `Travel + Attack` under the same lawful remote conditions
- [ ] `Attack` root synthesis remains absent for remote hostile goals
- [ ] Pursuit plan invalidates when believed target place changes
- [ ] Arrival at stale last-known place without target does not fabricate attack and triggers replanning
- [ ] Blocked target/place memory suppresses repeat pursuit only for that same believed place

### Golden tests

- [ ] Bandit witnesses a traveler leave for an adjacent place, pursues, and attacks only if real co-location is re-established there
- [ ] Bandit pursues stale target information to a last-known place, fails to reacquire, and falls back to normal replanning rather than omnisciently continuing
- [ ] Deterministic replay companions for both

## Acceptance Criteria

1. `RaidTarget` and `EngageHostile` can plan remote pursuit through belief-backed prerequisite travel.
2. `Attack` remains a lawful same-place terminal affordance; no synthesized combat alias path is reintroduced.
3. Pursuit uses concrete target identity plus believed place, never generic area hostility.
4. Pursuit is profile-driven through `HostilePursuitProfile`, not hardcoded confidence or distance cutoffs.
5. Arrival at a stale target place produces honest failure/replanning, not omniscient continuation.
6. Existing ranking and interrupt hierarchies remain intact; this spec extends planning reach, not goal-family semantics.
7. All existing AI and golden suites continue to pass.
