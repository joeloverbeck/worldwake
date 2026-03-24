**Status**: PENDING

# S22: Generalized Intention Frames

## Summary

Replace the travel-specific `JourneyCommitment` component (introduced in S21) with a general `IntentionFrame` model that supports any multi-step commitment: travel, care chains, escort, multi-step production, political errands. An intention frame captures the goal, relied-on assumptions, suspension/resume policy, and patience tracking in a domain-agnostic structure.

Additionally, replace the travel-specific `TravelDispositionProfile` with a general `IntentionDispositionProfile` that provides per-domain patience limits and commitment switch margins, enabling agent diversity (P20) across all commitment domains.

When a frame is exhausted (patience or assumption failure), it creates a `BlockedIntent` via S23's compound-keyed system to prevent immediate re-adoption of the same goal.

## Phase

Phase 3+: AI Architecture Overhaul (Step 13.5, Wave 2)

## Crate

`worldwake-core` (IntentionFrame types, IntentionDispositionProfile, new BlockingFact variants), `worldwake-sim` (BeliefView updates), `worldwake-ai` (lifecycle integration, progress detection, trace)

## Dependencies

- S21 (JourneyCommitment must be an authoritative component first -- S22 generalizes it)
- S23 (BlockedIntentMemory compound-keyed system -- S22 creates blocked intents on frame exhaustion)
- S26 (Planner conformance golden tests -- must remain passing through migration)

## FOUNDATIONS Alignment

- **P19** (Intentions are revisable commitments): IntentionFrame is the direct implementation of P19's "stable intentions held under assumptions" with explicit monitoring of those assumptions. The frame captures what the agent intends and under what conditions that intention remains valid -- and the agent monitors those conditions and revises when they break.
- **P8** (Actions have preconditions, duration, cost): Intention frames make multi-step commitment visible without silently reserving resources. The frame records what the agent intends, not what the world guarantees. "I planned to use the orchard" does not make the orchard unavailable to others.
- **P3** (Concrete state over abstract scores): Assumptions are concrete belief predicates (target alive, route exists, no critical threat), not abstract "commitment strength" scores. Frame state transitions are driven by assumption evaluation against observable world conditions, not by tunable float thresholds.
- **P20** (Agent diversity through concrete variation): Patience limits are per-agent per-domain values in `IntentionDispositionProfile`, allowing different agents to have different tolerances for stalled commitments across different domains.
- **P24** (Systems interact through state, not through each other): Frame exhaustion communicates with the planning pipeline through `BlockedIntentMemory` (state-mediated), not by directly calling into candidate generation.
- **P26** (No backward compatibility layers): `JourneyCommitment`, `JourneyCommitmentState`, `JourneyPlanRelation`, and `TravelDispositionProfile` are all removed entirely -- no shims, aliases, or deprecated wrappers.
- **P27** (Debuggability is a product feature): Frame state transitions are recorded in the decision trace sink, making it possible to answer "why did this agent abandon its journey?" or "why did this agent resume its care commitment?"

## Motivation

The current `JourneyCommitment` (S21) is travel-specific: it tracks destination, progress, and patience for travel goals only. When Phase 4 introduces care chains (healer traveling to patient, treating, returning), escort missions (guard accompanying caravan through dangerous route), and political errands (travel to office, declare support, return), each domain would need its own commitment type -- duplicating the commitment pattern.

The current `TravelDispositionProfile` is similarly travel-specific: it stores patience and switch margin for travel only. Different commitment domains need different patience tolerances.

A general `IntentionFrame` + `IntentionDispositionProfile` provides:

1. **Thrash prevention**: Any multi-step goal benefits from commitment stability, not just travel
2. **Assumption monitoring**: When assumptions break, the frame suspends -- not the entire goal
3. **Resumability**: Suspended frames can resume when conditions improve, with stalled ticks preserved
4. **Patience tracking**: Domain-agnostic patience exhaustion works identically for travel, queuing, and waiting
5. **Exhaustion memory**: Patience exhaustion creates a `BlockedIntent` (via S23), preventing the agent from immediately re-committing to the same unreachable goal
6. **Progress detection**: Concrete action-def matching determines what counts as progress per domain
7. **Debuggability**: Decision traces can report frame state transitions uniformly across all domains

## Relationship to FacilityQueueIntents

`IntentionFrame` and `FacilityQueueIntents` (S21) serve different architectural roles and coexist as separate components:

- **IntentionFrame**: Goal-level commitment stability for multi-step plans. Tracks patience, assumptions, and suspension/resume lifecycle. One per agent.
- **FacilityQueueIntents**: Resource contention positions. Tracks which facilities an agent intends to use. Multiple entries per agent (one per facility).

Clearing an `IntentionFrame` does NOT affect `FacilityQueueIntents` -- they are orthogonal concerns. An agent may be committed to a journey (frame) while also holding a facility queue position at the destination (queue intent).

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
    /// from the agent's IntentionDispositionProfile at frame creation time.
    /// Stored on the frame so it is self-contained and survives save/load
    /// without requiring the profile to be re-read.
    pub patience_limit: u32,
}
```

### IntentionDomain

Domain tags carry the minimal domain-specific data needed for lifecycle operations (e.g., knowing the travel destination for route-exists checks).

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

`IntentionDomain` has a `fn domain_tag(&self) -> IntentionDomainTag` method returning the data-free discriminant.

### IntentionDomainTag

A data-free discriminant enum suitable for use as a `BTreeMap` key in profiles. Lives in `worldwake-core` alongside `IntentionDomain`.

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum IntentionDomainTag {
    Travel,
    Care,
    Escort,
    Errand,
    Generic,
}
```

### IntentionDispositionProfile Component (on Agent entities)

Replaces `TravelDispositionProfile` entirely (P26 -- no backward compatibility layers). Provides per-agent, per-domain patience limits and commitment switch margins.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentionDispositionProfile {
    /// Per-domain patience limits. If the agent's current frame's domain tag
    /// is not present, falls back to `default_patience_ticks`.
    pub domain_patience: BTreeMap<IntentionDomainTag, NonZeroU32>,
    /// Fallback patience limit when no domain-specific entry exists.
    pub default_patience_ticks: NonZeroU32,
    /// Switch margin applied when the agent has an active frame and a
    /// challenger plan would abandon it. Generalizes the former
    /// TravelDispositionProfile::route_replan_margin.
    pub commitment_switch_margin: Permille,
}
```

Helper method:

```rust
impl IntentionDispositionProfile {
    /// Returns the patience limit for a given domain tag, falling back to
    /// the default if no domain-specific entry exists.
    pub fn patience_for(&self, tag: IntentionDomainTag) -> u32 {
        self.domain_patience
            .get(&tag)
            .map_or(self.default_patience_ticks.get(), |v| v.get())
    }
}
```

Migration from `TravelDispositionProfile`:
- `TravelDispositionProfile::blocked_leg_patience_ticks` → `IntentionDispositionProfile::domain_patience[IntentionDomainTag::Travel]`
- `TravelDispositionProfile::route_replan_margin` → `IntentionDispositionProfile::commitment_switch_margin`
- `TravelDispositionProfile` is removed from `component_schema.rs`, `component_tables.rs`, and all `BeliefView` trait methods.

### Progress Detection via PlannerOpKind

Each `IntentionDomain` declares which `PlannerOpKind` values count as forward progress. This mapping lives in `worldwake-ai` (since `PlannerOpKind` is defined there) as a function, not on the `IntentionDomain` enum itself:

```rust
/// Returns the set of PlannerOpKind values that count as forward progress
/// for a given intention domain. When a completed action's op kind appears
/// in this set, the frame's stalled_ticks resets and last_progress_tick
/// is updated.
pub fn progress_op_kinds(domain: &IntentionDomain) -> &[PlannerOpKind] {
    match domain {
        IntentionDomain::Travel { .. } => &[PlannerOpKind::Travel],
        IntentionDomain::Care { .. } => &[PlannerOpKind::Heal, PlannerOpKind::Travel],
        IntentionDomain::Escort { .. } => &[PlannerOpKind::Travel],
        IntentionDomain::Errand { .. } => &[
            PlannerOpKind::Travel,
            PlannerOpKind::DeclareSupport,
            PlannerOpKind::PressForceClaim,
            PlannerOpKind::YieldForceClaim,
        ],
        IntentionDomain::Generic => GENERIC_PROGRESS_OPS, // all op kinds
    }
}
```

When a plan step completes in `agent_tick`, the step's `PlannerOpKind` is looked up via the semantics table. If it appears in `progress_op_kinds(frame.domain)`, the frame records progress (reset `stalled_ticks` to 0, set `last_progress_tick`).

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
- Assumptions are evaluated through the agent's `BeliefView` (for `TargetAlive`, `RouteExists`, `CommodityAvailableAt`) or through ranked candidates (for `NoCriticalThreat`), never by querying authoritative world state. An agent may hold a stale belief that a route exists when it has in fact been severed. The assumption passes until the agent perceives the change.
- **`NoCriticalThreat` is not a BeliefView query.** It is evaluated by checking whether any `GoalPriorityClass::Critical` candidate exists in the current tick's ranked candidates. This evaluation happens during the planning pipeline, after candidate generation and ranking.
- **`RouteExists` requires a new BeliefView method.** `fn route_exists(&self, from: EntityId, to: EntityId) -> bool` must be added to the `BeliefView` trait. `OmniscientBeliefView` delegates to `Topology::find_route()`. Future `PerceptionBeliefView` would check the agent's believed topology.
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

### FrameClearReason

Generalizes the existing `JourneyClearReason` enum. Stored on `AgentDecisionRuntime` to record why the last frame was cleared, aiding debuggability (P27).

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FrameClearReason {
    /// The frame's goal was achieved.
    GoalSatisfied,
    /// A higher-priority goal permanently replaced the frame's goal.
    Reprioritized,
    /// The plan for the frame's goal failed (action rejected, precondition broken).
    PlanFailed,
    /// Patience exhausted (stalled_ticks >= patience_limit).
    PatienceExhausted,
    /// A critical assumption failed (e.g., target believed dead).
    AssumptionFailed,
    /// Agent died.
    Death,
    /// The plan was lost (no remaining steps, no replanning yielded a plan).
    LostPlan,
}
```

### New BlockingFact Variants

Two new variants added to `BlockingFact` in `worldwake-core/src/blocked_intent.rs`:

```rust
pub enum BlockingFact {
    // ... existing 14 variants ...
    /// Frame patience exhausted for this goal at this place/target.
    PatienceExhausted,
    /// A critical frame assumption failed (target dead, route severed).
    AssumptionFailed,
}
```

Both variants cause `blocks_goal_generation()` to return `true`, preventing the agent from immediately re-adopting the same goal after frame exhaustion.

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
- `patience_limit`: from agent's `IntentionDispositionProfile::patience_for(domain.domain_tag())`. If no `IntentionDispositionProfile` component exists, use `PlanningBudget::default_patience_ticks` as fallback.

### 2. Progress

Each tick where the frame's goal makes progress:
- Set `last_progress_tick` to the current tick
- Reset `stalled_ticks` to 0

**Progress detection mechanism**: When a plan step completes, look up the step's `PlannerOpKind` via the `PlannerOpSemantics` table. Call `progress_op_kinds(frame.domain)` and check if the completed op kind is in the returned set. If yes, the step counts as progress.

Examples:
- Travel frame: only `PlannerOpKind::Travel` completions count as progress. Eating food mid-journey does not reset stalled ticks.
- Care frame: both `PlannerOpKind::Travel` (traveling to patient) and `PlannerOpKind::Heal` (treating wounds) count as progress.
- Generic frame: any completed action step counts as progress.

### 3. Stalling

Each tick where the frame is `Active` but no progress occurs:
- Increment `stalled_ticks` by 1

The patience limit is stored on the frame (set from profile at creation time).

### 4. Assumption Evaluation

Each tick, after the agent's observation refresh and before the planning pipeline:

- For each `FrameAssumption`, evaluate against the agent's current beliefs:
  - `TargetAlive(entity)`: call `view.is_alive(entity)` on the agent's `BeliefView`
  - `RouteExists { from, to }`: call `view.route_exists(from, to)` on the agent's `BeliefView` (new method -- see Design section)
  - `NoCriticalThreat`: evaluated during the planning pipeline after candidate ranking -- check if any candidate has `GoalPriorityClass::Critical` and a different goal than the frame's goal
  - `CommodityAvailableAt { commodity, place }`: call existing commodity presence queries on the agent's `BeliefView`

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
- **Create a `BlockedIntent` in `BlockedIntentMemory`** to prevent immediate re-adoption:
  - `BlockerKey`:
    - `goal_key`: the frame's `goal`
    - `place`: agent's current effective place (from belief view)
    - `target`: domain-specific target entity (`destination` for Travel, `patient` for Care, `ward` for Escort, `destination` for Errand, `None` for Generic)
    - `action_def`: `None` (frame exhaustion is not action-specific)
  - `BlockingFact::PatienceExhausted`
  - `diagnostic_context`: `None`
  - `observed_tick`: current tick
  - `expires_tick`: `current_tick + budget.structural_block_ticks`
- Set `AgentDecisionRuntime::last_frame_clear_reason` to `FrameClearReason::PatienceExhausted`
- The AI must clear the frame on the next tick and allow full replanning without commitment margins

When a critical assumption fails (e.g., `TargetAlive` returns false):
- Transition to `FrameState::Exhausted`
- **Create a `BlockedIntent`** with:
  - `BlockerKey`: same `goal_key`, `place` = current place, `target` = the assumption's entity (if applicable)
  - `BlockingFact::AssumptionFailed`
  - TTL: `budget.structural_block_ticks`
- Set `AgentDecisionRuntime::last_frame_clear_reason` to `FrameClearReason::AssumptionFailed`

### 8. Completion

When the frame's goal is achieved (the plan reaches its terminal state and the goal condition is satisfied):
- Clear the frame (remove the `IntentionFrame` component)
- Set `last_frame_clear_reason` to `FrameClearReason::GoalSatisfied`
- No `BlockedIntent` is created (successful completion is not a blocker)

### 9. Goal Switch Abandonment

When the AI pipeline selects a different goal that is not a suspension of the current frame (i.e., the new plan would abandon the frame's commitment):
- Clear the frame
- Set `last_frame_clear_reason` to `FrameClearReason::Reprioritized`
- No `BlockedIntent` is created (voluntary goal switching is not a blocker)
- The new goal may establish its own frame if it qualifies

### Cross-Component Consistency on Frame Clearing

When an `IntentionFrame` is cleared:
- `FacilityQueueIntents` is NOT affected (separate concern)
- `ActiveGoal` is cleared only if the frame's goal matches the current `ActiveGoal` (the agent may have already switched goals before the frame was cleared)
- `AgentDecisionRuntime::last_frame_clear_reason` is always updated with the appropriate `FrameClearReason`
- `BlockedIntentMemory` is updated only for exhaustion/assumption-failure clears (not for completion or voluntary abandonment)

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
| (implicit patience limit from `TravelDispositionProfile`) | `patience_limit: u32` (set from `IntentionDispositionProfile` at creation) |

| TravelDispositionProfile field | IntentionDispositionProfile equivalent |
|---|---|
| `blocked_leg_patience_ticks: NonZeroU32` | `domain_patience[IntentionDomainTag::Travel]: NonZeroU32` |
| `route_replan_margin: Permille` | `commitment_switch_margin: Permille` |

| Journey AI type | Frame equivalent |
|---|---|
| `JourneyPlanRelation` | `FramePlanRelation` |
| `JourneyClearReason` | `FrameClearReason` |
| `JourneySwitchMarginSource` | removed (margin always from `IntentionDispositionProfile`) |
| `JourneyDebugSnapshot` | `FrameDebugSnapshot` |
| `JourneyRuntimeSnapshot` | `FrameRuntimeSnapshot` |

The following types are removed entirely (P26):
- `JourneyCommitment` component
- `JourneyCommitmentState` enum
- `TravelDispositionProfile` component
- `JourneyPlanRelation` enum
- `JourneyClearReason` enum
- `JourneySwitchMarginSource` enum
- `JourneyDebugSnapshot` struct
- `JourneyRuntimeSnapshot` struct

## Goal Switching Integration

### Commitment Margins

`compare_goal_switch()` and the frame switch policy (`frame_switch_policy.rs`, renamed from `journey_switch_policy.rs`) use the intention frame to apply elevated switching margins for committed goals. This generalizes:

- If an `IntentionFrame` with `state == Active` exists for the agent's current goal, apply the commitment switch margin from `IntentionDispositionProfile::commitment_switch_margin`
- If the frame is `Suspended` or `Exhausted`, use the default switch margin (no commitment protection)

The `FramePlanRelation` enum (replaces `JourneyPlanRelation`):

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FramePlanRelation {
    /// No active frame exists.
    NoFrame,
    /// The new plan continues the frame's committed goal.
    ContinuesFrame,
    /// The new plan is a temporary detour (different goal, no multi-step
    /// commitment of its own).
    SuspendsFrame,
    /// The new plan abandons the frame (different multi-step goal).
    AbandonsFrame,
}
```

Classification logic (`classify_frame_plan_relation`):
- `NoFrame`: no `IntentionFrame` exists
- `ContinuesFrame`: new plan's `goal == frame.goal` (domain-agnostic goal match)
- `SuspendsFrame`: new plan has a different goal but does not qualify as a multi-step commitment
- `AbandonsFrame`: new plan has a different goal and would establish its own frame

### Interrupt Evaluation

`evaluate_interrupt()` receives the agent's current `IntentionFrame` (if any) and uses it to:
1. Determine the effective switch margin (committed via `IntentionDispositionProfile::commitment_switch_margin` vs default via `PlanningBudget::switch_margin`)
2. Decide whether an interrupt should suspend or abandon the frame

## AgentDecisionRuntime Changes

The following fields and types on `AgentDecisionRuntime` are updated:

- `last_journey_clear_reason: Option<JourneyClearReason>` → `last_frame_clear_reason: Option<FrameClearReason>`

The following free functions in `decision_runtime.rs` are renamed/generalized:

| Old function | New function |
|---|---|
| `has_journey_commitment(jc)` | `has_intention_frame(frame)` |
| `journey_committed_destination(jc)` | removed (destination is domain-specific, accessed via `frame.domain`) |
| `has_active_journey_travel(jc, plan, step_index)` | `has_active_frame_travel(frame, plan, step_index)` |
| `journey_runtime_snapshot(jc, runtime)` | `frame_runtime_snapshot(frame, runtime)` |
| `classify_journey_plan_relation(jc, plan)` | `classify_frame_plan_relation(frame, plan)` |

## BeliefView Changes

### New method: `route_exists`

Add to the `RuntimeBeliefView` trait (in `worldwake-sim/src/belief_view.rs`):

```rust
/// Returns true if a route exists from `from` to `to` through the
/// believed topology. Used for IntentionFrame assumption evaluation.
fn route_exists(&self, from: EntityId, to: EntityId) -> bool;
```

Implementations:
- `OmniscientBeliefView`: delegates to `Topology::find_route(from, to)` and returns `route.is_some()`
- `PerAgentBeliefView` (future E14): checks the agent's believed topology

### Replaced method: `intention_disposition_profile`

Replace `travel_disposition_profile()` with:

```rust
fn intention_disposition_profile(&self, agent: EntityId) -> Option<IntentionDispositionProfile>;
```

All existing callers of `travel_disposition_profile()` must be updated. Implementations:
- `OmniscientBeliefView`: reads `IntentionDispositionProfile` from world
- `PerAgentBeliefView`: reads from agent's believed components

## Migration Scope (Files Affected)

### worldwake-core

| File | Changes |
|---|---|
| `intention.rs` | Remove `JourneyCommitment`, `JourneyCommitmentState`. Add `IntentionFrame`, `IntentionDomain`, `IntentionDomainTag`, `FrameAssumption`, `FrameState`, `SuspensionReason`, `FrameClearReason`. |
| `travel_disposition.rs` | Remove entirely. Replace with new `intention_disposition.rs` containing `IntentionDispositionProfile`. |
| `blocked_intent.rs` | Add `PatienceExhausted` and `AssumptionFailed` variants to `BlockingFact`. |
| `component_schema.rs` | Deregister `JourneyCommitment` and `TravelDispositionProfile`. Register `IntentionFrame` and `IntentionDispositionProfile`. |
| `component_tables.rs` | Same registration changes as component_schema. |
| `mod.rs` / `lib.rs` | Update module declarations and re-exports. |

### worldwake-sim

| File | Changes |
|---|---|
| `belief_view.rs` | Replace `travel_disposition_profile()` with `intention_disposition_profile()`. Add `route_exists()`. |
| `omniscient_belief_view.rs` | Implement `intention_disposition_profile()` and `route_exists()`. |
| `per_agent_belief_view.rs` | Implement `intention_disposition_profile()` and `route_exists()`. |
| `affordance_query.rs` | Update mock BeliefView impls in tests. |
| `trade_valuation.rs` | Update mock BeliefView impls in tests. |

### worldwake-ai

| File | Changes |
|---|---|
| `decision_runtime.rs` | Replace `JourneyPlanRelation`, `JourneyClearReason`, `JourneyRuntimeSnapshot`, all journey helper functions with frame equivalents. |
| `agent_tick/mod.rs` | Replace all journey reads/writes with frame reads/writes. Source margin from `IntentionDispositionProfile`. |
| `agent_tick/journey.rs` | Rename to `agent_tick/frame.rs`. Generalize `update_journey_for_adopted_plan()` → `update_frame_for_adopted_plan()`. Generalize `handle_recoverable_travel_step_blockage()` to frame-aware blockage handling. |
| `agent_tick/active_action.rs` | Replace `has_journey_commitment` with `has_intention_frame`. Update planned_candidates build. |
| `agent_tick/execution.rs` | Replace `persist_journey_commitment` with `persist_intention_frame`. |
| `agent_tick/planning.rs` | Update plan selection integration to pass frame instead of journey. |
| `agent_tick/tests.rs` | Update test setup to use IntentionFrame/IntentionDispositionProfile. |
| `journey_switch_policy.rs` | Rename to `frame_switch_policy.rs`. Update all types. |
| `plan_selection.rs` | Replace `JourneyCommitment`/`JourneyPlanRelation` with frame equivalents. |
| `interrupts.rs` | Replace `JourneyCommitment` parameter with `IntentionFrame`. |
| `failure_handling.rs` | Replace `JourneyCommitment` with `IntentionFrame`. Add blocked intent creation on frame exhaustion/assumption failure. |
| `decision_trace.rs` | Add `FrameTransitionTrace` and frame lifecycle events. |
| `lib.rs` | Update module declarations and re-exports. |

## Tickets

### S22-001: Define IntentionFrame and IntentionDispositionProfile types in worldwake-core

- Add to `worldwake-core/src/intention.rs`: `IntentionFrame`, `IntentionDomain`, `IntentionDomainTag`, `FrameAssumption`, `FrameState`, `SuspensionReason`, `FrameClearReason`
- All types derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`
- `IntentionDomain`, `IntentionDomainTag`, and `FrameAssumption` additionally derive `Ord, PartialOrd` for deterministic ordering
- `IntentionDomainTag` additionally derives `Hash` for map key use
- Add `IntentionDomain::domain_tag()` method
- Add `worldwake-core/src/intention_disposition.rs`: `IntentionDispositionProfile` with `patience_for()` helper
- Register `IntentionFrame` as a component on Agent entities in `component_schema.rs` and `component_tables.rs`
- Register `IntentionDispositionProfile` as a component on Agent entities
- Add `PatienceExhausted` and `AssumptionFailed` variants to `BlockingFact` in `blocked_intent.rs`
- Add `FramePlanRelation` enum to `worldwake-ai` (replacing `JourneyPlanRelation`)
- Verify: `cargo build --workspace`

### S22-002: Replace JourneyCommitment and TravelDispositionProfile with frame equivalents

- Migrate all journey commitment creation/update/clear logic to use `IntentionFrame` with `IntentionDomain::Travel`
- Map all existing journey lifecycle operations to frame lifecycle:
  - `has_journey_commitment()` → `has_intention_frame()`
  - `classify_journey_plan_relation()` → `classify_frame_plan_relation()` operating on `Option<&IntentionFrame>`
  - Journey progress tracking maps to `last_progress_tick` / `stalled_ticks` on the frame
  - `persist_journey_commitment()` → `persist_intention_frame()`
- Rename `agent_tick/journey.rs` → `agent_tick/frame.rs`; generalize all functions
- Rename `journey_switch_policy.rs` → `frame_switch_policy.rs`; update all types
- Update `agent_tick/mod.rs` to read/write `IntentionFrame` via `World`/`WorldTxn` instead of `JourneyCommitment`
- Replace `JourneySwitchMarginSource` and `JourneyDebugSnapshot` with frame-aware equivalents
- Replace `last_journey_clear_reason` with `last_frame_clear_reason: Option<FrameClearReason>`
- Source commitment switch margin from `IntentionDispositionProfile::commitment_switch_margin` instead of `TravelDispositionProfile::route_replan_margin`
- Replace `travel_disposition_profile()` with `intention_disposition_profile()` on all BeliefView traits and implementations
- Add `route_exists()` to BeliefView trait with `OmniscientBeliefView` and `PerAgentBeliefView` implementations
- Remove `JourneyCommitment` and `TravelDispositionProfile` component registrations from `component_schema.rs` and `component_tables.rs`
- Remove `JourneyCommitmentState` from `worldwake-core`
- Remove `travel_disposition.rs` module
- Update all test BeliefView mocks in `affordance_query.rs`, `trade_valuation.rs`
- Full file list: see Migration Scope section (17+ files across 3 crates)
- Verify: `cargo test -p worldwake-ai` -- all golden tests pass

### S22-003: Implement assumption population and evaluation

- Add `populate_assumptions()` function that derives assumptions from `IntentionDomain` and current belief state
- Add `evaluate_assumptions()` function that checks each `FrameAssumption`:
  - `TargetAlive`: call `view.is_alive(entity)` on agent's BeliefView
  - `RouteExists`: call `view.route_exists(from, to)` on agent's BeliefView
  - `NoCriticalThreat`: check ranked candidates for `GoalPriorityClass::Critical` (NOT a BeliefView query -- evaluated during planning pipeline after candidate ranking)
  - `CommodityAvailableAt`: call existing commodity presence queries on BeliefView
- Integrate assumption evaluation into the per-tick AI pipeline in `agent_tick/mod.rs`:
  - After observation refresh, before planning
  - Failed critical assumptions → `FrameState::Exhausted` + `BlockedIntent` with `BlockingFact::AssumptionFailed`
  - Failed recoverable assumptions → `FrameState::Suspended`
- Write focused unit tests for each assumption kind's evaluation
- Verify: `cargo test -p worldwake-ai` -- golden tests pass, new focused tests pass

### S22-004: Implement progress detection via PlannerOpKind

- Add `progress_op_kinds()` function in `worldwake-ai` mapping `IntentionDomain` → `&[PlannerOpKind]`
- Integrate into `agent_tick` step completion: when a plan step completes, look up its `PlannerOpKind` via the semantics table and check against `progress_op_kinds(frame.domain)`
- If match: reset `stalled_ticks = 0`, set `last_progress_tick = Some(current_tick)`
- Write focused unit tests for progress detection per domain
- Verify: `cargo test -p worldwake-ai` -- golden tests pass

### S22-005: Implement frame exhaustion → BlockedIntent integration

- On patience exhaustion (`stalled_ticks >= patience_limit`): create `BlockedIntent` in `BlockedIntentMemory` with `BlockingFact::PatienceExhausted`, `BlockerKey` scoped by goal/place/domain-target, TTL = `budget.structural_block_ticks`
- On critical assumption failure: create `BlockedIntent` with `BlockingFact::AssumptionFailed`
- Update `handle_plan_failure()` to pass `BlockedIntentMemory` for frame-related blocked intents
- Write focused tests: verify that after frame exhaustion, the same goal is blocked for `structural_block_ticks` ticks
- Verify: `cargo test -p worldwake-ai` -- golden tests pass, no immediate re-adoption after exhaustion

### S22-006: Add decision trace integration

- Extend `DecisionTraceSink` to record frame state transitions:
  - `FrameCreated { goal, domain_tag, patience_limit, assumptions_count }`
  - `FrameProgressed { tick }`
  - `FrameSuspended { reason, tick }`
  - `FrameResumed { tick }`
  - `FrameExhausted { stalled_ticks, patience_limit, blocked_intent_recorded: bool }`
  - `FrameCleared { reason: FrameClearReason }`
- Add `FrameTransitionTrace` to the trace data model
- Add `frame_transition: Option<FrameTransitionTrace>` to `DecisionOutcome::ActiveAction` and `DecisionOutcome::Planning`
- Update `dump_agent()` to display frame lifecycle events
- Verify: trace dump includes frame lifecycle events for travel scenarios

### S22-007: Save/load verification

- Verify `IntentionFrame` survives save/load round-trip (component is registered, so `ComponentTables` serialization handles it)
- Verify `IntentionDispositionProfile` survives save/load round-trip
- Extend or adapt `golden_save_load_round_trip_under_ai` to assert:
  - A mid-journey agent preserves its `IntentionFrame` after reload (goal, domain, state, established_at, stalled_ticks, patience_limit all match)
  - The frame's assumptions list is preserved
  - A suspended frame remains suspended after reload
  - `IntentionDispositionProfile` per-domain patience is preserved
- Verify: `cargo test -p worldwake-ai --test golden_determinism`

### S22-008: Workspace verification

- `cargo test --workspace` -- all pass
- `cargo clippy --workspace` -- no new warnings
- Deterministic replay produces identical hashes (`golden_deterministic_replay_fidelity`)
- No orphaned `JourneyCommitment`, `JourneyCommitmentState`, `JourneyPlanRelation`, or `TravelDispositionProfile` references remain in non-archived code

## FND-01 Section H Analysis

### Information-path analysis

`IntentionFrame` is private agent state -- only the owning agent's AI pipeline reads it during `agent_tick` execution. Other agents cannot observe an agent's intention frame directly. They observe the agent's *actions* (travel, facility use, care delivery) and their consequences in the event log. This aligns with P8: "Planner intent is not silent control" and P19: "Selecting a plan does not secretly hold the workstation, the bread, the corpse, the patient, or the road."

Assumption evaluation reads the agent's `BeliefView`, which itself is populated through the perception and witness systems (P7 locality). The frame never queries authoritative world state directly.

Frame exhaustion communicates with the planning pipeline through `BlockedIntentMemory` (P24: state-mediated interaction). The blocked intent record is read by `candidate_generation` to suppress re-adoption of the exhausted goal.

### Positive-feedback analysis

No amplifying loops. Intention frames record decisions; they do not create new feedback paths. A frame cannot cause the world to change in ways that reinforce the frame's assumptions. The frame is purely reactive: it observes belief state and transitions accordingly.

The exhaustion → blocked intent path is a dampener, not an amplifier: it prevents commitment cycling (adopt → stall → exhaust → re-adopt → stall → ...).

### Concrete dampeners

- **Patience exhaustion**: `stalled_ticks >= patience_limit` forces frame clearing and full replanning. This prevents infinite commitment to unreachable goals. The patience limit is a concrete per-agent per-domain value set from the agent's `IntentionDispositionProfile` (P20 diversity), not a global tunable.
- **Blocked intent on exhaustion**: Patience exhaustion creates a `BlockedIntent` with TTL, preventing immediate re-adoption of the same goal. This breaks the adopt-stall-exhaust-readopt cycle. The TTL is `structural_block_ticks` from `PlanningBudget`.
- **Assumption failure**: Concrete assumption checks (target alive, route exists) terminate or suspend frames when the agent's beliefs invalidate them. This is driven by actual belief state changes, not by timers or abstract scores.
- **Suspension does not pause patience**: `stalled_ticks` continues to increment during suspension, so an agent that is repeatedly interrupted by survival needs will eventually exhaust patience on its original commitment rather than holding it indefinitely.

### Stored state vs. derived read-model list

**Stored (authoritative, persisted as component)**:
- `IntentionFrame` component: goal, domain, assumptions, state, established_at, last_progress_tick, stalled_ticks, patience_limit
- `IntentionDispositionProfile` component: domain_patience, default_patience_ticks, commitment_switch_margin

**Derived (ephemeral, computed each tick, NOT stored)**:
- Assumption evaluation results (pass/fail per assumption, derived from current belief view)
- Frame plan relation classification (derived from comparing a candidate plan against the current frame)
- Effective switch margin (derived from frame state: Active → committed margin from IntentionDispositionProfile, Suspended/Exhausted → default margin from PlanningBudget)
- Whether a frame qualifies for creation (derived from plan structure and goal kind)
- Progress-op-kind set for current frame (derived from `IntentionDomain` variant via `progress_op_kinds()`)
- Domain-specific target entity for BlockerKey (derived from `IntentionDomain` variant)

## Verification

1. `cargo test --workspace` -- all pass
2. `cargo clippy --workspace` -- no new warnings
3. All existing journey commitment golden tests pass with `IntentionFrame` replacing `JourneyCommitment`
4. Save/load preserves `IntentionFrame` and `IntentionDispositionProfile` state including assumptions, suspension reason, and per-domain patience
5. Deterministic replay produces identical hashes
6. Decision traces show frame lifecycle events (created, progressed, suspended, resumed, exhausted, cleared)
7. Frame exhaustion creates a `BlockedIntent` that prevents immediate re-adoption
8. No references to `JourneyCommitment`, `JourneyCommitmentState`, `JourneyPlanRelation`, or `TravelDispositionProfile` remain in non-archived code
