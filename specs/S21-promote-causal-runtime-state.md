**Status**: PENDING

# S21: Promote Causal Runtime State to Authoritative Components

## Summary

Move causally relevant state from `AgentDecisionRuntime` (non-persisted runtime held in `AgentTickDriver`'s `BTreeMap<EntityId, AgentDecisionRuntime>`) into authoritative ECS components registered in `worldwake-core`. This fixes a FOUNDATIONS violation where save/load loses journey commitments, facility queue intents, and active goal identity because `GoldenHarness::from_simulation_state` constructs a fresh `AgentTickDriver::new(PlanningBudget::default())` with empty runtime maps.

## Phase

Phase 3+: AI Architecture Overhaul (post-E13, Wave 1)

## Crates

- `worldwake-core` (new component types and registration)
- `worldwake-ai` (runtime migration, read/write through World/WorldTxn)

## Dependencies

- S20 (structural cleanup) -- `agent_tick.rs` is cleaner to modify after S20 splits its 1800+ line monolith

## FOUNDATIONS Alignment

- **P11** (save/load must not change world meaning): Currently violated. `from_simulation_state` at `golden_harness/mod.rs:1170` creates `AgentTickDriver::new(PlanningBudget::default())`, discarding all per-agent runtime state. A mid-journey agent loses its travel commitment and may thrash or re-derive a different plan after reload.
- **P16** (memories and commitments are world state): Journey commitments and facility queue positions are causally relevant state that affects future decisions across multiple ticks. They must be representable, inspectable, and persistent.
- **P19** (intentions are revisable commitments): The intention to continue a journey or wait for a facility must be inspectable world state, not hidden runtime magic. Other systems should in principle be able to observe that an agent is committed to a journey.

## Motivation

`AgentDecisionRuntime` (defined in `crates/worldwake-ai/src/decision_runtime.rs`) holds per-agent state that is:

1. **Persistent across ticks** -- journey commitment affects behavior over dozens of ticks
2. **Causally relevant** -- commitment prevents goal thrashing; facility intents affect contention resolution
3. **Lost on save/load** -- `from_simulation_state` creates a fresh driver, zeroing all runtime state

The existing `golden_save_load_round_trip_under_ai` test passes because agents re-derive equivalent behavior from world state within the 30 post-resume ticks. But this is fragile and violates P11: a mid-journey agent should preserve its commitment, not silently re-derive it. A scenario with more agents, longer journeys, or tighter margins could diverge.

The precedent exists: `BlockedIntentMemory` is already a registered Agent-only component in `component_schema.rs` (line 333) that survives save/load. The promoted state follows the same pattern.

## State Classification

### Promote to Authoritative Components

| Runtime Field | Target Component | Rationale |
|---|---|---|
| `current_goal: Option<GoalKey>` | `ActiveGoal` | Agent's current intention -- causally relevant, affects goal switching margins and interrupt thresholds |
| `journey_committed_goal: Option<GoalKey>` | `JourneyCommitment` | Travel commitment anchor -- prevents thrashing across multi-leg journeys |
| `journey_committed_destination: Option<EntityId>` | `JourneyCommitment` | Where the agent is committed to traveling |
| `journey_commitment_state: JourneyCommitmentState` | `JourneyCommitment` | Active vs Suspended |
| `journey_established_at: Option<Tick>` | `JourneyCommitment` | When commitment was made -- used for patience calculations |
| `journey_last_progress_tick: Option<Tick>` | `JourneyCommitment` | Last tick the agent made forward progress -- patience tracking |
| `consecutive_blocked_leg_ticks: u32` | `JourneyCommitment` | Blocked leg counter -- patience exhaustion triggers replanning |
| `queued_facility_intents: BTreeMap<EntityId, QueuedFacilityIntent>` | `FacilityQueueIntents` | Facility use intentions -- affect contention and queue ordering |

### Keep as Ephemeral Runtime (rederivable or diagnostic-only)

| Runtime Field | Rationale |
|---|---|
| `current_plan: Option<PlannedPlan>` | Rederivable from `ActiveGoal` + world state via `search_plan`. Plans are search output, not authoritative state (P3). |
| `current_step_index: usize` | Derived from plan + scheduler active action. After reload the plan is re-searched and step index resets to 0. |
| `step_in_flight: bool` | Derived from scheduler active action state. |
| `dirty: bool` | Observation cache flag -- rederivable by comparing current snapshot to last snapshot. A fresh runtime is always dirty (triggers immediate replan). |
| `last_needs: Option<HomeostaticNeeds>` | Observation cache -- rederivable from world state. |
| `last_wounds: Vec<Wound>` | Observation cache -- rederivable from world state. |
| `last_commodity_signature` | Observation cache -- rederivable from world state. |
| `last_unique_item_signature` | Observation cache -- rederivable from world state. |
| `last_facility_access_signature` | Observation cache -- rederivable from world state. |
| `last_priority_class` | Observation cache -- rederivable from ranking. |
| `last_effective_place` | Observation cache -- rederivable from world state. |
| `materialization_bindings: MaterializationBindings` | Resolved fresh each tick from action outcomes. Bindings are transient within a single plan execution. |
| `last_journey_clear_reason` | Diagnostic only -- used for trace output, not decision logic. |

## New Components

All three are Agent-only components, following the same registration pattern as `BlockedIntentMemory`.

### `ActiveGoal`

```rust
/// The agent's currently adopted goal intention.
/// Persisted through save/load so goal switching margins and
/// interrupt thresholds are preserved across representation boundaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActiveGoal {
    /// The goal the agent is currently pursuing.
    pub goal_key: GoalKey,
    /// The tick at which this goal was adopted. Used for commitment
    /// stability calculations (e.g., minimum hold time before switching).
    pub adopted_at: Tick,
}
```

### `JourneyCommitment`

```rust
/// A multi-tick travel commitment that prevents goal thrashing during
/// multi-leg journeys. Persisted so that save/load mid-journey does not
/// cause the agent to re-evaluate and potentially abandon its route.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JourneyCommitment {
    /// The goal that motivated this journey.
    pub committed_goal: GoalKey,
    /// The terminal destination place entity.
    pub destination: EntityId,
    /// Whether the commitment is actively being executed or temporarily
    /// suspended (e.g., agent is handling an urgent local need).
    pub state: JourneyCommitmentState,
    /// The tick at which the journey commitment was established.
    pub established_at: Tick,
    /// The last tick at which the agent made forward travel progress.
    /// `None` if no progress has been recorded yet.
    pub last_progress_tick: Option<Tick>,
    /// How many consecutive ticks the current travel leg has been blocked.
    /// Used for patience exhaustion -- if this exceeds a threshold the
    /// commitment is abandoned.
    pub consecutive_blocked_leg_ticks: u32,
}
```

Note: `JourneyCommitmentState` already exists in `decision_runtime.rs` and must be moved to `worldwake-core` (since components live in core). The enum is trivial (`Active | Suspended`) and already derives the necessary traits.

### `FacilityQueueIntents`

```rust
/// Per-agent record of which facilities the agent intends to use and
/// what action it plans to perform there. Affects contention resolution
/// when multiple agents queue for the same exclusive facility.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct FacilityQueueIntents {
    /// Map from facility EntityId to the agent's queued intent.
    pub intents: BTreeMap<EntityId, QueuedFacilityIntent>,
}
```

Note: `QueuedFacilityIntent` currently lives in `decision_runtime.rs` and must also be moved to `worldwake-core`.

## Type Relocation

The following types must move from `worldwake-ai::decision_runtime` to `worldwake-core` so that the new components can reference them without creating a circular dependency:

| Type | Current Location | New Location |
|---|---|---|
| `JourneyCommitmentState` | `worldwake-ai::decision_runtime` | `worldwake-core` (new module or existing `goal.rs`) |
| `QueuedFacilityIntent` | `worldwake-ai::decision_runtime` | `worldwake-core` (new module or existing `goal.rs`) |

`JourneyPlanRelation`, `JourneyClearReason`, `JourneyRuntimeSnapshot`, and `MaterializationBindings` remain in `worldwake-ai` since they are not needed by core components.

## Migration Strategy

1. **Define types in core**: Move `JourneyCommitmentState` and `QueuedFacilityIntent` to `worldwake-core`. Define `ActiveGoal`, `JourneyCommitment`, `FacilityQueueIntents` as component types.
2. **Register components**: Add entries in `component_schema.rs` and `component_tables.rs` using the existing macro pattern. Restrict all three to `EntityKind::Agent`.
3. **Migrate reads**: In the AI crate, replace `runtime.journey_committed_goal` etc. with `world.get_component_journey_commitment(agent)` (or the WorldTxn equivalent).
4. **Migrate writes**: Replace `runtime.journey_committed_goal = Some(...)` etc. with `txn.set_component_journey_commitment(agent, JourneyCommitment { ... })`. Journey clearing becomes `txn.clear_component_journey_commitment(agent)`.
5. **Remove promoted fields**: Delete the promoted fields from `AgentDecisionRuntime`. Update `Default` impl and all tests in `decision_runtime.rs`.
6. **Update helper methods**: Methods like `has_journey_commitment()`, `clear_journey_commitment()`, `classify_journey_plan_relation()` move to free functions or trait extension methods that take `Option<&JourneyCommitment>` instead of `&self`.
7. **Verify save/load**: The new components are automatically serialized as part of `World` (via `ComponentTables`), so `save_to_bytes`/`load_from_bytes` preserves them without additional work.

## Tickets

### S21-001: Define and register ActiveGoal, JourneyCommitment, FacilityQueueIntents components

- Move `JourneyCommitmentState` and `QueuedFacilityIntent` from `worldwake-ai::decision_runtime` to `worldwake-core`
- Define `ActiveGoal`, `JourneyCommitment`, `FacilityQueueIntents` structs in `worldwake-core`
- Add `impl Component for ...` (or equivalent macro registration)
- Register all three in `component_schema.rs` and `component_tables.rs`, restricted to `EntityKind::Agent`
- Re-export moved types from `worldwake-ai` to avoid breaking downstream imports during migration
- Verify: `cargo build --workspace`

### S21-002: Migrate journey commitment fields from runtime to JourneyCommitment component

- Replace all reads of `runtime.journey_committed_goal`, `runtime.journey_committed_destination`, `runtime.journey_commitment_state`, `runtime.journey_established_at`, `runtime.journey_last_progress_tick`, `runtime.consecutive_blocked_leg_ticks` with `world.get_component_journey_commitment(agent)` (or WorldTxn reads)
- Replace all writes with `txn.set_component_journey_commitment(agent, ...)` or `txn.clear_component_journey_commitment(agent)`
- Refactor `has_journey_commitment()`, `clear_journey_commitment()`, `clear_journey_commitment_with_reason()`, `has_active_journey_travel()`, `remaining_travel_steps()`, `journey_runtime_snapshot()`, `journey_committed_destination()`, `classify_journey_plan_relation()` to work with `Option<&JourneyCommitment>` instead of `&AgentDecisionRuntime` fields
- Remove the six journey fields from `AgentDecisionRuntime`
- Update unit tests in `decision_runtime.rs`
- Verify: `cargo test -p worldwake-ai` -- all golden tests pass

### S21-003: Migrate active goal from runtime to ActiveGoal component

- Replace reads of `runtime.current_goal` with `world.get_component_active_goal(agent)`
- Replace writes with `txn.set_component_active_goal(agent, ActiveGoal { ... })` or `txn.clear_component_active_goal(agent)`
- Goal switching logic in `goal_switching.rs` and `plan_selection.rs` reads the component
- `agent_tick.rs` writes the component when adopting or abandoning a goal
- Remove `current_goal` field from `AgentDecisionRuntime`
- Update unit tests
- Verify: `cargo test -p worldwake-ai` -- all golden tests pass

### S21-004: Migrate facility queue intents from runtime to FacilityQueueIntents component

- Replace reads of `runtime.queued_facility_intents` with `world.get_component_facility_queue_intents(agent)`
- Replace writes with `txn.set_component_facility_queue_intents(agent, ...)`
- Facility queue patience and expiry logic reads/writes the component
- Remove `queued_facility_intents` field from `AgentDecisionRuntime`
- Update unit tests
- Verify: `cargo test -p worldwake-ai` -- all golden tests pass

### S21-005: Update save/load round-trip test to verify commitment preservation

- Extend `golden_save_load_round_trip_under_ai` (or add a companion test) to assert:
  - An agent with an `ActiveGoal` before save still has the same `ActiveGoal` after load
  - An agent mid-journey before save still has the same `JourneyCommitment` after load (destination, state, established_at all match)
  - An agent with `FacilityQueueIntents` before save still has the same intents after load
- The scenario must create a situation where at least one agent has an active journey at the save boundary (tick 20). This may require adjusting the scenario topology or tick count.
- Verify: `cargo test -p worldwake-ai --test golden_determinism`

### S21-006: Workspace verification and cleanup

- `cargo test --workspace` -- all pass
- `cargo clippy --workspace` -- no new warnings
- Verify deterministic replay still produces identical hashes (the `golden_deterministic_replay_fidelity` test)
- Remove any temporary re-exports added in S21-001 if they are no longer needed
- Verify the `agent_decision_runtime_is_not_registered_as_a_component` test still passes (AgentDecisionRuntime itself remains unregistered)

## FND-01 Section H Analysis

### Information-path analysis

`ActiveGoal`, `JourneyCommitment`, and `FacilityQueueIntents` are private agent state -- they are read only by the owning agent's AI pipeline during `agent_tick` execution. No other agent observes them directly. Information about an agent's intentions reaches other agents only through observable actions (starting travel, occupying a facility) and their consequences in the event log. This is consistent with P7 (locality) and P12 (world state is not belief state).

### Positive-feedback analysis

No amplifying loops introduced. These components record existing decisions that were already being made in the runtime. Promoting them to authoritative state does not create new causal paths -- it preserves existing ones across save/load boundaries.

### Concrete dampeners

N/A -- no new feedback loops are introduced by this change.

### Stored state vs. derived read-model list

**Stored (authoritative, persisted)**:
- `ActiveGoal` -- the agent's current goal intention
- `JourneyCommitment` -- multi-tick travel commitment with patience tracking
- `FacilityQueueIntents` -- facility use intentions

**Derived (ephemeral, rederivable, NOT stored)**:
- `current_plan` -- rederivable via `search_plan` from `ActiveGoal` + world state
- `current_step_index` -- derived from plan + scheduler active action
- `step_in_flight` -- derived from scheduler active action state
- `dirty` -- observation cache flag, always true on fresh construction
- `last_needs`, `last_wounds`, `last_commodity_signature`, `last_unique_item_signature`, `last_facility_access_signature`, `last_effective_place`, `last_priority_class` -- observation caches rederivable from world state
- `materialization_bindings` -- transient per-tick resolution
- `last_journey_clear_reason` -- diagnostic only

## Verification

1. `cargo test --workspace` -- all existing tests pass
2. `cargo clippy --workspace` -- no new warnings
3. Save/load round-trip preserves journey commitments, active goals, and facility intents (new assertions in S21-005)
4. Deterministic replay produces identical hashes (`golden_deterministic_replay_fidelity`)
5. No golden test behavioral changes -- agents make the same decisions (world and event log hashes unchanged for same seeds)
6. `AgentDecisionRuntime` remains unregistered as a component (existing test `agent_decision_runtime_is_not_registered_as_a_component` still passes)
