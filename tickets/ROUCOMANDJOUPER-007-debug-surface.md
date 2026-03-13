# ROUCOMANDJOUPER-007: Observable Debug Surface for Journey State

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new debug/inspection methods on AgentDecisionRuntime
**Deps**: ROUCOMANDJOUPER-002, ROUCOMANDJOUPER-005

## Problem

Tests and CLI inspection need to observe journey state: whether an agent has an active journey, its destination, remaining route length, temporal field values, and the reason a journey was cleared. Without a structured debug surface, test assertions and human inspection must re-derive journey state from raw fields.

## Assumption Reassessment (2026-03-13)

1. `AgentDecisionRuntime` has `journey_established_at`, `journey_last_progress_tick`, `consecutive_blocked_leg_ticks`, `has_active_journey()`, `remaining_travel_steps()`, and `clear_journey_fields()` after ticket 002 — assumed complete.
2. `PlannedPlan` has `goal: GoalKey` and `steps: Vec<PlannedStep>` — confirmed.
3. `PlannedStep` has `op_kind: PlannerOpKind` and `targets: Vec<PlanningEntityRef>` — confirmed.
4. `GoalKey` encodes some destinations directly (e.g., `MoveCargo` has a target place) while others require derivation from the plan's terminal Travel step — per spec note.
5. `AgentTickDriver` holds `runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>` — confirmed.

## Architecture Check

1. The debug surface consists of read-only query methods on `AgentDecisionRuntime`. These are controller/runtime inspection, not authoritative world component exposure. This follows the existing pattern where runtime state is queried for debugging.
2. No backwards-compatibility aliasing or shims.
3. Journey clearing reasons are best exposed through a log/debug mechanism rather than stored state — storing clearing reasons would add transient state for debugging only.

## What to Change

### 1. Add `journey_destination()` method

On `AgentDecisionRuntime`:

```rust
/// Returns the committed destination EntityId, derived from the plan's
/// terminal Travel step target. Returns `None` if no active journey.
pub fn journey_destination(&self) -> Option<EntityId> {
    if self.journey_established_at.is_none() {
        return None;
    }
    let plan = self.current_plan.as_ref()?;
    // Find the last Travel step in the plan
    plan.steps.iter().rev()
        .find(|step| step.op_kind == PlannerOpKind::Travel)
        .and_then(|step| step.targets.first().copied())
        .and_then(authoritative_target)
}
```

### 2. Add `JourneySnapshot` struct for structured inspection

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JourneySnapshot {
    pub destination: Option<EntityId>,
    pub established_at: Option<Tick>,
    pub last_progress_tick: Option<Tick>,
    pub remaining_travel_steps: usize,
    pub consecutive_blocked_ticks: u32,
}
```

With a method:
```rust
impl AgentDecisionRuntime {
    pub fn journey_snapshot(&self) -> JourneySnapshot { ... }
}
```

### 3. Add `JourneyClearReason` enum for debug logging

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum JourneyClearReason {
    GoalSatisfied,
    GoalSwitched,
    PlanFailed,
    PatienceExhausted,
    Death,
    Incapacitation,
    ControlLoss,
    NonTravelPlan,
}
```

Add a variant of `clear_journey_fields` that accepts a reason for debug logging:

```rust
pub fn clear_journey_fields_with_reason(&mut self, reason: JourneyClearReason) {
    // Log the reason at debug level if needed
    self.journey_established_at = None;
    self.journey_last_progress_tick = None;
    self.consecutive_blocked_leg_ticks = 0;
}
```

Update clearing call sites (ticket 006) to use this method with the appropriate reason. The original `clear_journey_fields()` can delegate to this with a default reason.

### 4. Expose journey state on `AgentTickDriver`

Add a public method:

```rust
impl AgentTickDriver {
    pub fn journey_snapshot(&self, agent: EntityId) -> Option<JourneySnapshot> {
        self.runtime_by_agent.get(&agent).map(|r| r.journey_snapshot())
    }
}
```

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add `journey_destination()`, `JourneySnapshot`, `journey_snapshot()`, `JourneyClearReason`, `clear_journey_fields_with_reason()`)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — add `journey_snapshot()` accessor on driver)

## Out of Scope

- `TravelDispositionProfile` component (ticket 001)
- Journey field lifecycle (tickets 005, 006 — this ticket builds on them)
- Goal switching and plan selection (tickets 003, 004)
- CLI rendering of journey state (future work beyond this spec)
- Storing clearing reasons as persistent state (clearing reasons are debug-only)
- Changes to `worldwake-core`, `worldwake-sim`, or `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. `journey_destination()` returns `None` when `journey_established_at` is `None`.
2. `journey_destination()` returns the target of the last Travel step when journey is active.
3. `journey_destination()` returns `None` when plan has no Travel steps.
4. `journey_snapshot()` returns a `JourneySnapshot` reflecting all temporal fields and derived state.
5. `journey_snapshot()` with no active journey returns all-`None`/zero snapshot.
6. `JourneyClearReason` enum has all required variants and derives `Debug`, `Eq`, `PartialEq`.
7. `clear_journey_fields_with_reason()` clears all fields regardless of reason.
8. `AgentTickDriver::journey_snapshot()` delegates to the runtime correctly.
9. Existing suite: `cargo test -p worldwake-ai`
10. Existing suite: `cargo clippy --workspace`

### Invariants

1. All debug surface methods are read-only — they do not mutate state.
2. `journey_destination()` derives from the plan, not from stored route state (no `Vec<EntityId>` route storage).
3. `JourneySnapshot` is a transient read-model, not serialized.
4. No new authoritative component for journey state.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` — test: `journey_destination_returns_last_travel_target`
2. `crates/worldwake-ai/src/decision_runtime.rs` — test: `journey_destination_returns_none_without_active_journey`
3. `crates/worldwake-ai/src/decision_runtime.rs` — test: `journey_snapshot_reflects_all_temporal_fields`
4. `crates/worldwake-ai/src/decision_runtime.rs` — test: `clear_journey_fields_with_reason_clears_all_fields`

### Commands

1. `cargo test -p worldwake-ai decision_runtime`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
