**Status**: DRAFT

# Route Commitment And Journey Persistence

## Summary
Design route persistence for Worldwake without changing authoritative travel into a single continuous multi-edge action. Agents should be able to commit to a destination and prefer continuing along a chosen route across intermediate places, while still remaining concretely located at each hop and still replanning when new local pressures or new beliefs justify it.

This spec is intentionally forward-looking. It improves the E13 decision architecture and the E10/E08 travel execution stack, but it is not part of the active E14-E22 implementation sequence. Do not schedule implementation ahead of the current phase gates in [IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md).

## Why This Exists
Current architecture is correctly per-leg:
- `travel` is a single-edge action to an adjacent place
- multi-hop journeys are produced by planning + repeated replanning
- intermediate places are real authoritative world states

That architecture is cleaner than continuous multi-edge travel, but it leaves a practical behavioral gap:
- agents can re-evaluate too freely at each hop even when nothing meaningful changed
- long journeys may feel indecisive or noisy because every intermediate arrival restarts selection from scratch
- the runtime has no explicit memory that "this journey is already in progress toward a concrete destination"
- route progress is not represented as first-class AI state

The fix should not be "make travel continuous." That would hide intermediate concrete state behind a long-running abstraction and weaken locality. The correct improvement is to preserve per-leg authoritative travel while adding explicit journey tracking in decision/runtime state.

## Phase
Future AI/runtime hardening, post-E22 scheduling only.

## Crates
- `worldwake-core`
- `worldwake-ai`

No new cross-dependency from `worldwake-systems` to `worldwake-ai` is permitted.

## Dependencies
- E08 time/scheduler/replay
- E10 production/transport/route occupancy
- E13 grounded decision architecture
- E14 perception/beliefs for full value once route decisions are belief-driven rather than omniscient in tests

## Design Goals
1. Preserve per-leg travel as the authoritative world model.
2. Add explicit journey tracking to the AI runtime layer via temporal fields on `AgentDecisionRuntime`.
3. Make route persistence profile-driven per agent rather than hardcoded.
4. Allow interruption when concrete state changes justify it.
5. Keep intermediate places meaningful for perception, danger, opportunity, and death.
6. Avoid compatibility shims or dual travel models.
7. Derive route and destination from the existing plan — no redundant route storage.

## Non-Goals
- No continuous multi-edge travel action.
- No edge-fraction or abstract "progress along route" scalar in authoritative world state.
- No global path cache that agents can query as world truth.
- No teleport-style route skipping that hides intermediate places.
- No separate `JourneyCommitment` struct — journey state is temporal fields on `AgentDecisionRuntime`.
- No `Vec<EntityId>` or `Vec<TravelEdgeId>` route storage — route is derived from the plan's remaining Travel steps and topology on demand.

## Deliverables

### 1. New `TravelDispositionProfile` Component
Add a dedicated per-agent component for route persistence behavior.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct TravelDispositionProfile {
    route_replan_margin: Permille,
    blocked_leg_patience_ticks: NonZeroU32,
}

impl Component for TravelDispositionProfile {}
```

Meaning:
- `route_replan_margin`: how much better a challenger goal must be before the agent abandons an active journey. During an active journey, this value **replaces** `budget.switch_margin_permille` in `compare_goal_switch()` — it is a per-agent override of the global switching margin for multi-hop travel contexts.
- `blocked_leg_patience_ticks`: how long the agent tolerates repeated failure on the same next leg before dropping commitment

Both values are per-agent and seeded at creation to preserve agent diversity.

### 2. Journey Temporal Fields on `AgentDecisionRuntime`
Add three fields to the existing `AgentDecisionRuntime` struct:

```rust
// Inside AgentDecisionRuntime:
journey_established_at: Option<Tick>,
journey_last_progress_tick: Option<Tick>,
consecutive_blocked_leg_ticks: u32,
```

These fields track the temporal dimension of an active journey. The journey itself is defined by the plan:
- **"Is this a journey?"** = `current_plan` has remaining Travel steps.
- **Destination** = derived from the plan's terminal Travel step target.
- **Route** = derived from the plan's remaining Travel steps.

`AgentDecisionRuntime` is transient runtime state — it is **not** serialized through save/load or replay paths. The journey temporal fields are equally transient. On load, the agent re-derives its journey through deterministic replanning from world state, producing the same outcome.

### 3. Plan Selection Override During Active Journeys
Extend plan selection so that when an agent has an active journey (current plan contains remaining Travel steps and `journey_established_at` is `Some`):

- The agent's `TravelDispositionProfile::route_replan_margin` replaces `budget.switch_margin_permille` in `compare_goal_switch()`. This is the per-agent override mechanism — agents with higher `route_replan_margin` are harder to divert mid-journey.
- A challenger goal that would abandon the current destination must beat the committed option by at least this margin.

The comparison uses existing goal-switching machinery (priority class, motive value) with only the margin threshold overridden.

### 4. Journey Field Advancement on Arrival
When a travelling agent completes a leg and arrives at an intermediate place:
- Authoritative state shows the agent grounded at that place (unchanged).
- `journey_last_progress_tick` is updated to the current tick.
- `consecutive_blocked_leg_ticks` is reset to 0.
- The journey remains active if the plan still has remaining Travel steps toward the destination.

This is the key distinction from continuous travel. Arrival remains a real place-level state transition even while the journey intent persists.

### 5. Journey Clearing Conditions
Journey temporal fields are cleared (set to `None` / 0) when any of the following occurs:
- Destination goal is satisfied (plan completes or goal reached).
- Destination becomes unreachable with current beliefs.
- A higher-priority challenger beats the current commitment by the agent's `route_replan_margin`.
- The next leg repeatedly fails for at least `blocked_leg_patience_ticks` consecutive ticks (`consecutive_blocked_leg_ticks >= blocked_leg_patience_ticks`).
- The agent dies, becomes incapacitated, or loses control.
- The plan is replaced for any reason (goal switch, replan).

The clearing reason should be explicit in debug output rather than implicit.

### 6. Cooperation With Existing Interrupt Logic
Interrupt logic remains the gatekeeper for abandoning an in-progress action. This spec does not bypass it.

Behavior split:
- Interrupt rules decide whether the current active action can be abandoned now.
- Journey tracking decides whether the agent prefers resuming the same destination/route after the interruption window closes.

Example:
- Thirst becomes critical during a long food journey.
- Agent may interrupt at an intermediate place to drink.
- After resolving thirst, the agent may resume the same destination if goal ranking still favors the original goal (the journey temporal fields persist across the interruption since the plan is not replaced).

### 7. Integration With Blocked-Intent Memory
Do not create a second independent "cooldown table" for journeys.

Use existing blocked-intent infrastructure to record concrete route failures such as:
- Next leg unavailable.
- Repeated arrival/replan loops with no valid continuation.
- Destination-source unavailable after arrival.

When `consecutive_blocked_leg_ticks` exceeds `blocked_leg_patience_ticks`, the journey clears and the blocked goal is recorded in `BlockedIntentMemory` with the concrete barrier.

### 8. No New Travel Domain Action
Keep the current adjacent-place `travel` action shape.

Do not add:
- `travel_route`
- `travel_to_destination`
- `continue_journey`

Route persistence belongs in selection/runtime semantics, not in action catalog proliferation.

### 9. Belief-Only Compatibility (E14+ Forward Constraint)
> **Note**: This section describes behavior active after E14 (perception/beliefs). Current implementation uses `OmniscientBeliefView` as stand-in. The constraints below become enforceable once E14 delivers real belief state.

Once richer beliefs exist, a committed journey must remain belief-grounded:
- Route selection uses believed topology / believed destination relevance.
- Commitment persists based on what the agent believes is still worth doing.
- New witness information or local observation may invalidate commitment.

No authoritative-world shortcut may keep a commitment alive "because the engine knows the route is still optimal."

### 10. Observable Debug Surface
Expose enough runtime/debug information for tests and CLI inspection:
- Whether the agent has an active journey (derived from plan + temporal fields).
- Current committed destination (derived from plan's terminal Travel step).
- Remaining route length (count of remaining Travel steps in plan).
- `journey_established_at` and `journey_last_progress_tick` values.
- `consecutive_blocked_leg_ticks` value.
- Clearing reason when journey ends.

This is controller/runtime inspection, not authoritative world component exposure.

**Note on GoalKey destination coverage**: Some goal kinds encode a destination directly (e.g., `MoveCargo`, `BuryCorpse`). Others require destination derivation from the plan's terminal Travel step target (e.g., `AcquireCommodity`, `Sleep`, `ProduceCommodity`). The debug surface should handle both cases.

## Component Registration

Register in `component_schema.rs`:
- `TravelDispositionProfile` on `EntityKind::Agent`

No new authoritative component for route progress or route commitment is allowed.

## SystemFn Integration

### `worldwake-core`
- Add `TravelDispositionProfile`.
- Keep authoritative travel/location schema unchanged.

### `worldwake-ai`
- Add journey temporal fields to `AgentDecisionRuntime`.
- Set `journey_established_at` when selecting a travel-led plan.
- Advance/clear journey fields on arrival, blockage, interruption aftermath, and goal satisfaction.
- Override `switch_margin_permille` with `route_replan_margin` during active journeys in `compare_goal_switch()`.
- Integrate journey clearing with blocked-intent memory.

### `worldwake-sim`
- No serialization changes needed — `AgentDecisionRuntime` is transient and not serialized.
- Expose runtime/debug accessors needed for inspection.

### `worldwake-systems`
- No new system-to-system coupling.
- No changes required to the travel action contract beyond any read-only helpers the AI/runtime already uses.

## Cross-System Interactions (Principle 12)
- E10 route occupancy and topology influence commitment only through world state that planning already reads.
- E09 needs can interrupt or outweigh a committed journey through existing need state and interrupt logic.
- E12 wounds/death clear commitment through `DeadAt`, incapacity, and active-action cleanup.
- E14/E15 beliefs can later revise destination or route desirability through belief state updates.

All interactions remain state-mediated:
- active action state
- location / in-transit state
- blocked-intent memory
- homeostatic needs
- wounds / deadness
- topology and occupancy facts
- believed resource / seller / danger facts

No system may call "resume journey" logic in another system module directly.

## FND-01 Section H

### Information-Path Analysis
- Journey state is created from a concrete planned route, not from hidden global route memory.
- Interruption or abandonment must be triggered by concrete state already available to the agent/runtime: local arrival, blocked affordances, changing needs, wounds, or updated beliefs.
- Intermediate places remain fully real information points where observation, reports, and danger can change the agent's next decision.
- The spec does not allow an agent to remain abstractly "between multiple places" across many hops.

### Positive-Feedback Analysis
- stronger commitment -> fewer replans -> more chance to continue long journeys
- successful long journeys -> more future destination completions -> more apparent value in staying committed

Potential failure mode:
- excessive commitment could cause agents to ignore new urgent local opportunities or hazards

### Concrete Dampeners
- `route_replan_margin`: commitment is not absolute; challengers can still win if they exceed the margin
- `blocked_leg_patience_ticks`: repeated failure dissolves commitment after a concrete tick count
- existing interrupt logic: urgent self-care and danger can still preempt
- concrete intermediate arrivals: every hop creates a real reevaluation point
- death / incapacitation / control loss: commitment is immediately cleared

### Stored vs Derived State
Stored authoritative state:
- `TravelDispositionProfile`
- normal location / in-transit components
- blocked-intent memory
- needs, wounds, deadness
- topology / route occupancy data

Transient runtime state (not serialized):
- `journey_established_at`, `journey_last_progress_tick`, `consecutive_blocked_leg_ticks` on `AgentDecisionRuntime`

Derived transient read-model:
- whether the agent has an active journey (plan has remaining Travel steps)
- destination (terminal Travel step target)
- remaining route (plan's Travel steps)
- whether a candidate plan matches the current journey destination
- whether a challenger exceeds the replan margin
- whether the remaining route is still valid under current beliefs

## Determinism
Route extraction inherits the canonical shortest-path tie-break already defined by topology/pathfinding. No second route-ordering policy is introduced. All route derivation from plans is deterministic because plan search and topology pathfinding are already deterministic.

## Invariants
- Authoritative travel remains adjacent-place and per-leg only.
- Intermediate places remain real decision and information points.
- Route persistence uses concrete plan-derived destination/route state, not abstract momentum scores.
- No backward compatibility layer introduces both continuous and per-leg travel semantics.
- Journey fields must be cleared on death and control loss.
- Journey tracking must never allow planners to bypass belief-only restrictions.
- All route ordering and tie-breaks remain deterministic.
- No abstract scores (`route_commitment_weight`, `route_progress_value`) — only concrete temporal and threshold fields (Principle 3).

## Tests
- [ ] Selecting a multi-hop travel-led plan sets `journey_established_at` on `AgentDecisionRuntime`.
- [ ] Arriving at an intermediate place updates `journey_last_progress_tick` and resets `consecutive_blocked_leg_ticks` instead of discarding journey state immediately.
- [ ] A same-destination continuation beats an equivalent-cost diversion because `route_replan_margin` raises the switching threshold.
- [ ] A sufficiently stronger challenger beats the journey when it exceeds `route_replan_margin`.
- [ ] Repeated blocked next-leg failures increment `consecutive_blocked_leg_ticks` and clear journey fields after reaching `blocked_leg_patience_ticks`.
- [ ] Critical self-care interruption can temporarily break action flow without clearing journey fields if the plan is not replaced.
- [ ] Death or incapacitation clears journey fields immediately.
- [ ] After save/load, the agent re-derives the same journey through deterministic replanning (same seed, same world state), not byte-level journey field persistence.
- [ ] Multi-hop golden scenarios become more stable without converting travel into a continuous action.

## Acceptance Criteria
- Agents can persist toward a destination across intermediate arrivals without losing per-leg authoritative travel.
- Route persistence is profile-driven per agent via `TravelDispositionProfile`.
- Interruption and rerouting remain possible through concrete state changes.
- No continuous multi-edge travel action or edge-fraction world state is added.
- Route ordering remains deterministic.
- All new fields use proper newtypes (`Permille`, `NonZeroU32`, `Tick`).
- No abstract scores are stored — only concrete temporal tracking and per-agent thresholds (Principle 3).
- Journey state is transient runtime, not serialized.

## References
- [FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md)
- [IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md)
- [DRAFT-merchant-selling-market-presence.md](/home/joeloverbeck/projects/worldwake/specs/DRAFT-merchant-selling-market-presence.md)
- [golden_ai_decisions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs)
- [travel_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/travel_actions.rs)
- [decision_runtime.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs)
