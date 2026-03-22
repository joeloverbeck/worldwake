**Status**: PENDING

# S22: Generalized Intention Frames

## Summary

Replace the travel-specific `JourneyCommitment` component (introduced in S21) with a general `IntentionFrame` model that supports any multi-step commitment: travel, care chains, escort, multi-step production, political errands. An intention frame captures the goal, relied-on assumptions, suspension/resume policy, and patience tracking in a domain-agnostic structure.

## Phase

Phase 3+: AI Architecture Overhaul (Step 13.5, Wave 2)

## Crate

`worldwake-core` (IntentionFrame types), `worldwake-ai` (lifecycle integration)

## Dependencies

- S21 (JourneyCommitment must be an authoritative component first -- S22 generalizes it)

## FOUNDATIONS Alignment

- **P19** (Intentions are revisable commitments): IntentionFrame is the direct implementation of P19's "stable intentions held under assumptions" with explicit monitoring of those assumptions. The frame captures what the agent intends and under what conditions that intention remains valid -- and the agent monitors those conditions and revises when they break.
- **P8** (Actions have preconditions, duration, cost): Intention frames make multi-step commitment visible without silently reserving resources. The frame records what the agent intends, not what the world guarantees. "I planned to use the orchard" does not make the orchard unavailable to others.
- **P3** (Concrete state over abstract scores): Assumptions are concrete belief predicates (target alive, route exists, no critical threat), not abstract "commitment strength" scores. Frame state transitions are driven by assumption evaluation against observable world conditions, not by tunable float thresholds.
- **P20** (Agent diversity through concrete variation): Patience limits are per-agent values stored on the frame, allowing different agents to have different tolerances for stalled commitments based on their profiles.
- **P27** (Debuggability is a product feature): Frame state transitions are recorded in the decision trace sink, making it possible to answer "why did this agent abandon its journey?" or "why did this agent resume its care commitment?"

## Motivation

The current `JourneyCommitment` (S21) is travel-specific: it tracks destination, progress, and patience for travel goals only. When Phase 4 introduces care chains (healer traveling to patient, treating, returning), escort missions (guard accompanying caravan through dangerous route), and political errands (travel to office, declare support, return), each domain would need its own commitment type -- duplicating the commitment pattern.

A general `IntentionFrame` provides:

1. **Thrash prevention**: Any multi-step goal benefits from commitment stability, not just travel
2. **Assumption monitoring**: When assumptions break, the frame suspends -- not the entire goal
3. **Resumability**: Suspended frames can resume when conditions improve, with stalled ticks preserved
4. **Patience tracking**: Domain-agnostic patience exhaustion works identically for travel, queuing, and waiting
5. **Debuggability**: Decision traces can report frame state transitions uniformly across all domains

## Design

### IntentionFrame Component (on Agent entities)

`IntentionFrame` replaces `JourneyCommitment` as a registered Agent-only component in `component_schema.rs`. One agent has at most one active `IntentionFrame` at any time -- the frame for their current committed goal.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentionFrame {
    /// The goal this frame serves. Must match the agent's ActiveGoal.
    pub goal: GoalKey,
    /// Domain tag for domain-specific lifecycle logic.
    pub domain: IntentionDomain,
    /// Concrete assumptions this frame relies on. Evaluated each tick
    /// against the agent's beliefs to detect invalidation.
    pub assumptions: Vec<FrameAssumption>,
    /// Current lifecycle state.
    pub state: FrameState,
    /// When this frame was established.
    pub established_at: Tick,
    /// Last tick where meaningful progress occurred (action step completed
    /// toward the frame's goal). None if no progress has been recorded yet.
    pub last_progress_tick: Option<Tick>,
    /// Consecutive ticks without progress. Incremented when the frame is
    /// Active but no forward step completes. Reset on progress.
    pub stalled_ticks: u32,
    /// Maximum stalled ticks before patience exhaustion. Per-agent, set
    /// from the agent's profile at frame creation time.
    pub patience_limit: u32,
}
```

### IntentionDomain

Domain tags carry the minimal domain-specific data needed for lifecycle operations (e.g., knowing the travel destination for route-exists checks). The enum is non_exhaustive to support future domains without breaking serialization compatibility.

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum IntentionDomain {
    /// Multi-leg travel to a destination place.
    Travel { destination: EntityId },
    /// Multi-step care: travel to patient, treat, potentially return.
    Care { patient: EntityId },
    /// Escort: accompany a target entity along a route.
    Escort { ward: EntityId, destination: EntityId },
    /// Multi-step errand: travel, act at destination, return.
    Errand { destination: EntityId },
    /// Domain not yet specialized. Used for goals that benefit from
    /// commitment stability but have no domain-specific assumptions.
    Generic,
}
```

### FrameAssumption

Each assumption is a concrete predicate that can be evaluated against the agent's current beliefs. Assumptions are not abstract scores -- they are falsifiable conditions (P3).

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum FrameAssumption {
    /// Target entity is alive (not dead, not despawned).
    TargetAlive(EntityId),
    /// A route exists from the agent's current place to the destination.
    RouteExists { from: EntityId, to: EntityId },
    /// Agent is not under critical survival threat (no Critical priority
    /// goal outranking the frame's goal).
    NoCriticalThreat,
    /// A specific commodity is available at a specific place (believed
    /// to exist, not necessarily confirmed).
    CommodityAvailableAt { commodity: CommodityKind, place: EntityId },
}
```

Design notes on assumptions:
- Assumptions are evaluated through the agent's `BeliefView`, not by querying authoritative world state. An agent may hold a stale belief that a route exists when it has in fact been severed. The assumption passes until the agent perceives the change.
- The assumption set is fixed at frame creation. New assumptions are not added dynamically -- if conditions change enough to require different assumptions, the frame should be cleared and a new one established.
- `Ord`/`PartialOrd` derive is required for deterministic serialization ordering in `Vec<FrameAssumption>`.

### FrameState

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FrameState {
    /// Actively pursuing the committed goal.
    Active,
    /// Temporarily suspended. The frame persists but does not contribute
    /// commitment margins to goal switching.
    Suspended {
        reason: SuspensionReason,
        suspended_at: Tick,
    },
    /// Patience exhausted or critical assumption failed. The AI must
    /// clear this frame and allow full replanning.
    Exhausted,
}
```

### SuspensionReason

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SuspensionReason {
    /// A higher-priority goal interrupted the frame's goal
    /// (e.g., flee from danger, critical survival need).
    PriorityInterrupt,
    /// Route to destination became unavailable (believed blocked or severed).
    RouteBlocked,
    /// Target entity became unreachable or believed dead.
    TargetUnreachable,
    /// Critical survival need interrupted (hunger, thirst at dangerous levels).
    SurvivalNeed,
}
```

## Frame Lifecycle

### 1. Creation

When the AI pipeline selects a plan for a goal that qualifies as multi-step (contains travel steps to a non-adjacent destination, or the goal kind maps to a known multi-step domain), and no `IntentionFrame` exists for this agent, create one:

- `goal`: the adopted `GoalKey`
- `domain`: inferred from the `GoalKind` and plan structure:
  - Plans with terminal travel steps: `IntentionDomain::Travel { destination }`
  - `GoalKind::TreatWounds { patient }`: `IntentionDomain::Care { patient }`
  - Other multi-step plans: `IntentionDomain::Generic`
- `assumptions`: populated from the domain:
  - Travel: `RouteExists { from: current_place, to: destination }`
  - Care: `TargetAlive(patient)`, `RouteExists { from: current_place, to: patient_place }`
  - Generic: `NoCriticalThreat`
- `state`: `FrameState::Active`
- `established_at`: current tick
- `last_progress_tick`: `None`
- `stalled_ticks`: 0
- `patience_limit`: from agent's `TravelDispositionProfile::blocked_leg_patience_ticks` (for travel domain) or a domain-appropriate profile field

### 2. Progress

Each tick where the frame's goal makes progress (an action step toward the goal completes successfully):
- Set `last_progress_tick` to the current tick
- Reset `stalled_ticks` to 0

"Progress" means: the completed action's op kind is relevant to the frame's domain (e.g., a Travel step completing for a Travel frame, a treatment action completing for a Care frame).

### 3. Stalling

Each tick where the frame is `Active` but no progress occurs:
- Increment `stalled_ticks` by 1

### 4. Assumption Evaluation

Each tick, after the agent's observation refresh and before the planning pipeline:

- For each `FrameAssumption`, evaluate against the agent's current `BeliefView`
- If any assumption fails:
  - **Critical failure** (e.g., `TargetAlive` fails -- the agent believes the target is dead): transition directly to `FrameState::Exhausted`
  - **Recoverable failure** (e.g., `RouteExists` fails -- the route is believed blocked): transition to `FrameState::Suspended { reason: RouteBlocked, suspended_at: current_tick }`

The distinction between critical and recoverable is determined by the assumption kind:
- `TargetAlive` failure: critical (target death is irreversible in current model)
- `RouteExists` failure: recoverable (routes may re-open)
- `NoCriticalThreat` failure: recoverable (threats may pass)
- `CommodityAvailableAt` failure: recoverable (supply may be replenished)

### 5. Suspension

When a higher-priority goal interrupts the frame's goal (via `evaluate_interrupt` returning `InterruptForReplan`):
- Transition to `Suspended { reason: PriorityInterrupt, suspended_at: current_tick }`
- The frame persists -- it is not cleared
- `stalled_ticks` continues to increment while suspended (patience drains even during suspension)

### 6. Resume

When a suspended frame's goal would rank highest again (the interrupting condition resolves):
- Transition back to `FrameState::Active`
- `stalled_ticks` does NOT reset on resume (accumulated patience drain is permanent for this frame)
- `last_progress_tick` is not updated (resume is not progress)

The resume check is: the frame's `goal` matches the goal selected by the planning pipeline, and all assumptions pass evaluation.

### 7. Exhaustion

When `stalled_ticks >= patience_limit`:
- Transition to `FrameState::Exhausted`
- The AI must clear the frame on the next tick and allow full replanning without commitment margins

### 8. Completion

When the frame's goal is achieved (the plan reaches its terminal state and the goal condition is satisfied):
- Clear the frame (remove the `IntentionFrame` component)

### 9. Goal Switch Abandonment

When the AI pipeline selects a different goal that is not a suspension of the current frame (i.e., the new plan would abandon the frame's commitment):
- Clear the frame
- The new goal may establish its own frame if it qualifies

## Migration from JourneyCommitment

| JourneyCommitment field (S21) | IntentionFrame equivalent |
|---|---|
| `committed_goal: GoalKey` | `goal: GoalKey` |
| `destination: EntityId` | `IntentionDomain::Travel { destination }` |
| `state: JourneyCommitmentState::Active` | `FrameState::Active` |
| `state: JourneyCommitmentState::Suspended` | `FrameState::Suspended { reason, suspended_at }` |
| `established_at: Tick` | `established_at: Tick` |
| `last_progress_tick: Option<Tick>` | `last_progress_tick: Option<Tick>` |
| `consecutive_blocked_leg_ticks: u32` | `stalled_ticks: u32` |
| (implicit patience limit from `TravelDispositionProfile`) | `patience_limit: u32` (explicit, set at creation from profile) |

The `JourneyCommitment` component (registered in S21) is removed and replaced by `IntentionFrame`. The `JourneyCommitmentState` enum is removed. The `TravelDispositionProfile` component remains -- its `blocked_leg_patience_ticks` feeds `patience_limit` at frame creation time.

## Goal Switching Integration

### Commitment Margins

`compare_goal_switch()` and the journey switch policy (`journey_switch_policy.rs`) currently use journey commitment to apply elevated switching margins for committed travel. This generalizes:

- If an `IntentionFrame` with `state == Active` exists for the agent's current goal, apply the commitment switch margin (from `TravelDispositionProfile::route_replan_margin` for Travel frames, or a domain-appropriate margin for other domains)
- If the frame is `Suspended` or `Exhausted`, use the default switch margin (no commitment protection)

The `JourneyPlanRelation` enum generalizes to `FramePlanRelation`:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FramePlanRelation {
    /// No active frame exists.
    NoFrame,
    /// The new plan continues the frame's committed goal.
    ContinuesFrame,
    /// The new plan is a temporary detour (no travel to a different destination).
    SuspendsFrame,
    /// The new plan abandons the frame (different multi-step goal).
    AbandonsFrame,
}
```

### Interrupt Evaluation

`evaluate_interrupt()` receives the agent's current `IntentionFrame` (if any) and uses it to:
1. Determine the effective switch margin (committed vs default)
2. Decide whether an interrupt should suspend or abandon the frame

## Tickets

### S22-001: Define IntentionFrame types in worldwake-core

- Add `IntentionFrame`, `IntentionDomain`, `FrameAssumption`, `FrameState`, `SuspensionReason` to `worldwake-core`
- All types derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`
- `IntentionDomain` and `FrameAssumption` additionally derive `Ord, PartialOrd` for deterministic ordering
- Register `IntentionFrame` as a component on Agent entities in `component_schema.rs` and `component_tables.rs`
- Add `FramePlanRelation` enum to `worldwake-ai` (replacing `JourneyPlanRelation`)
- Verify: `cargo build --workspace`

### S22-002: Replace JourneyCommitment with IntentionFrame for travel

- Migrate all journey commitment creation/update/clear logic to use `IntentionFrame` with `IntentionDomain::Travel`
- Map all existing journey lifecycle operations to frame lifecycle:
  - `has_journey_commitment()` becomes checking for an `IntentionFrame` component
  - `classify_journey_plan_relation()` becomes `classify_frame_plan_relation()` operating on `Option<&IntentionFrame>`
  - `clear_journey_commitment_with_reason()` becomes removing the `IntentionFrame` component
  - Journey progress tracking maps to `last_progress_tick` / `stalled_ticks` on the frame
- Update `agent_tick.rs` to read/write `IntentionFrame` via `World`/`WorldTxn` instead of `JourneyCommitment`
- Remove `JourneyCommitment` component registration from `component_schema.rs` and `component_tables.rs`
- Remove `JourneyCommitmentState` from `worldwake-core`
- Verify: `cargo test -p worldwake-ai` -- all golden tests pass

### S22-003: Implement assumption population and evaluation

- Add `populate_assumptions()` function that derives assumptions from `IntentionDomain` and current belief state
- Add `evaluate_assumptions()` function that checks each `FrameAssumption` against the agent's `BeliefView`:
  - `TargetAlive`: check belief about entity liveness
  - `RouteExists`: check topology pathfinding from belief view
  - `NoCriticalThreat`: check whether any Critical-priority goal candidate exists
  - `CommodityAvailableAt`: check belief about commodity presence at place
- Integrate assumption evaluation into the per-tick AI pipeline in `agent_tick.rs`:
  - After observation refresh, before planning
  - Failed critical assumptions -> `FrameState::Exhausted`
  - Failed recoverable assumptions -> `FrameState::Suspended`
- Write focused unit tests for each assumption kind's evaluation
- Verify: `cargo test -p worldwake-ai` -- golden tests pass, new focused tests pass

### S22-004: Generalize goal switching margins for frames

- Update `journey_switch_policy.rs` (rename to `frame_switch_policy.rs`) to use `FramePlanRelation` instead of `JourneyPlanRelation`
- Update `compare_relation_aware_goal_switch()` to take `FramePlanRelation`
- Update `select_best_plan()` to classify plan relations using `IntentionFrame` presence (any domain) instead of `JourneyCommitment`
- Update `evaluate_interrupt()` to use `IntentionFrame` for margin selection
- Remove `JourneySwitchMarginSource` and `JourneyDebugSnapshot` from `agent_tick.rs`, replace with frame-aware equivalents
- Verify: `cargo test -p worldwake-ai` -- goal switching golden tests pass

### S22-005: Add decision trace integration

- Extend `DecisionTraceSink` to record frame state transitions:
  - `FrameCreated { goal, domain, assumptions, patience_limit }`
  - `FrameProgressed { tick }`
  - `FrameSuspended { reason, tick }`
  - `FrameResumed { tick }`
  - `FrameExhausted { stalled_ticks, patience_limit }`
  - `FrameCleared { reason }`
- Add `FrameTransitionTrace` to the trace data model
- Include frame state in `AgentDecisionTrace` output
- Update `dump_agent()` to display frame lifecycle events
- Verify: trace dump includes frame lifecycle events for travel scenarios

### S22-006: Save/load verification

- Verify `IntentionFrame` survives save/load round-trip (component is registered, so `ComponentTables` serialization handles it)
- Extend or adapt `golden_save_load_round_trip_under_ai` to assert:
  - A mid-journey agent preserves its `IntentionFrame` after reload (goal, domain, state, established_at, stalled_ticks all match)
  - The frame's assumptions list is preserved
  - A suspended frame remains suspended after reload
- Verify: `cargo test -p worldwake-ai --test golden_determinism`

### S22-007: Workspace verification

- `cargo test --workspace` -- all pass
- `cargo clippy --workspace` -- no new warnings
- Deterministic replay produces identical hashes (`golden_deterministic_replay_fidelity`)
- No orphaned `JourneyCommitment`, `JourneyCommitmentState`, or `JourneyPlanRelation` references remain in the codebase

## FND-01 Section H Analysis

### Information-path analysis

`IntentionFrame` is private agent state -- only the owning agent's AI pipeline reads it during `agent_tick` execution. Other agents cannot observe an agent's intention frame directly. They observe the agent's *actions* (travel, facility use, care delivery) and their consequences in the event log. This aligns with P8: "Planner intent is not silent control" and P19: "Selecting a plan does not secretly hold the workstation, the bread, the corpse, the patient, or the road."

Assumption evaluation reads the agent's `BeliefView`, which itself is populated through the perception and witness systems (P7 locality). The frame never queries authoritative world state directly.

### Positive-feedback analysis

No amplifying loops. Intention frames record decisions; they do not create new feedback paths. A frame cannot cause the world to change in ways that reinforce the frame's assumptions. The frame is purely reactive: it observes belief state and transitions accordingly.

### Concrete dampeners

- **Patience exhaustion**: `stalled_ticks >= patience_limit` forces frame clearing and full replanning. This prevents infinite commitment to unreachable goals. The patience limit is a concrete per-agent value set from the agent's profile (P20 diversity), not a global tunable.
- **Assumption failure**: Concrete assumption checks (target alive, route exists) terminate or suspend frames when the agent's beliefs invalidate them. This is driven by actual belief state changes, not by timers or abstract scores.
- **Suspension does not pause patience**: `stalled_ticks` continues to increment during suspension, so an agent that is repeatedly interrupted by survival needs will eventually exhaust patience on its original commitment rather than holding it indefinitely.

### Stored state vs. derived read-model list

**Stored (authoritative, persisted as component)**:
- `IntentionFrame` component: goal, domain, assumptions, state, established_at, last_progress_tick, stalled_ticks, patience_limit

**Derived (ephemeral, computed each tick, NOT stored)**:
- Assumption evaluation results (pass/fail per assumption, derived from current belief view)
- Frame plan relation classification (derived from comparing a candidate plan against the current frame)
- Effective switch margin (derived from frame state: Active -> committed margin, Suspended/Exhausted -> default margin)
- Whether a frame qualifies for creation (derived from plan structure and goal kind)

## Verification

1. `cargo test --workspace` -- all pass
2. `cargo clippy --workspace` -- no new warnings
3. All existing journey commitment golden tests pass with `IntentionFrame` replacing `JourneyCommitment`
4. Save/load preserves `IntentionFrame` state including assumptions and suspension reason
5. Deterministic replay produces identical hashes
6. Decision traces show frame lifecycle events (created, progressed, suspended, resumed, exhausted, cleared)
7. No references to `JourneyCommitment`, `JourneyCommitmentState`, or `JourneyPlanRelation` remain in non-archived code
