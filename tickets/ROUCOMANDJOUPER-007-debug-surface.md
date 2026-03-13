# ROUCOMANDJOUPER-007: Observable Debug Surface for Journey State

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — add runtime inspection plus controller-level journey policy inspection
**Deps**: ROUCOMANDJOUPER-002, ROUCOMANDJOUPER-004, ROUCOMANDJOUPER-005

## Problem

Tests and CLI inspection need to observe journey state: whether an agent has an active journey, its destination, remaining route length, temporal field values, the effective switch margin currently in force, and the reason a journey was cleared. Without a structured debug surface, test assertions and human inspection must re-derive journey state from raw fields and separate controller policy.

After ticket 004, the effective journey switch margin is controller-level policy, not runtime-owned state. A runtime-only debug surface would miss a meaningful part of the behavior.

## Assumption Reassessment (2026-03-13)

1. `AgentDecisionRuntime` has `journey_established_at`, `journey_last_progress_tick`, `consecutive_blocked_leg_ticks`, `has_active_journey()`, `remaining_travel_steps()`, and `clear_journey_fields()` after ticket 002 — assumed complete.
2. `PlannedPlan` has `goal`, `steps`, and plan-level travel helpers such as `terminal_travel_destination()` — confirmed.
3. `AgentTickDriver` holds `runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>` — confirmed.
4. After the revised ticket 004, effective switch margin is computed at the controller layer from runtime state, `TravelDispositionProfile`, and the default budget margin — required.
5. `BeliefView` can expose `travel_disposition_profile()` after ticket 004 — required.

## Architecture Check

1. Runtime-derived journey facts such as destination and remaining travel steps belong on `AgentDecisionRuntime` as thin read-only delegates to `PlannedPlan`.
2. Controller-derived policy facts such as effective switch margin and margin source do not belong on `AgentDecisionRuntime`; they should be assembled by `AgentTickDriver` or another controller-facing inspection surface.
3. Journey clearing reasons should be explicit and inspectable, but they should not become authoritative state.
4. No backwards-compatibility aliasing or shims.

## What to Change

### 1. Add `journey_destination()` method

On `AgentDecisionRuntime`:

```rust
pub fn journey_destination(&self) -> Option<EntityId> {
    if self.journey_established_at.is_none() {
        return None;
    }
    self.current_plan
        .as_ref()
        .and_then(PlannedPlan::terminal_travel_destination)
}
```

### 2. Add `JourneyRuntimeSnapshot`

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JourneyRuntimeSnapshot {
    pub destination: Option<EntityId>,
    pub established_at: Option<Tick>,
    pub last_progress_tick: Option<Tick>,
    pub remaining_travel_steps: usize,
    pub consecutive_blocked_ticks: u32,
}
```

With:

```rust
impl AgentDecisionRuntime {
    pub fn journey_runtime_snapshot(&self) -> JourneyRuntimeSnapshot { ... }
}
```

### 3. Add controller-level policy inspection types

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum JourneySwitchMarginSource {
    BudgetDefault,
    JourneyProfile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JourneyDebugSnapshot {
    pub runtime: JourneyRuntimeSnapshot,
    pub effective_switch_margin: Permille,
    pub switch_margin_source: JourneySwitchMarginSource,
}
```

`JourneyDebugSnapshot` should be assembled at the controller layer, not stored on the runtime.

### 4. Add `JourneyClearReason` enum for explicit debug semantics

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum JourneyClearReason {
    GoalSatisfied,
    Reprioritized,
    PlanFailed,
    PatienceExhausted,
    Death,
    NonTravelPlan,
}
```

Add a variant of `clear_journey_fields` that accepts a reason for instrumentation/debug inspection:

```rust
pub fn clear_journey_fields_with_reason(&mut self, reason: JourneyClearReason) {
    self.journey_established_at = None;
    self.journey_last_progress_tick = None;
    self.consecutive_blocked_leg_ticks = 0;
}
```

The implementation may emit this reason to a debug surface or store the most recent clear reason in a transient controller-inspection field if tests need post-fact access. Do not turn it into authoritative state.

### 5. Expose `JourneyDebugSnapshot` on `AgentTickDriver`

Add a public method that assembles both runtime and policy state:

```rust
impl AgentTickDriver {
    pub fn journey_snapshot(
        &self,
        world: &worldwake_core::World,
        agent: EntityId,
    ) -> Option<JourneyDebugSnapshot> { ... }
}
```

This method should:
- read the runtime entry for the agent
- derive `JourneyRuntimeSnapshot`
- compute `effective_switch_margin`
- report whether that margin came from the journey profile or the budget default

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add `journey_destination()`, `JourneyRuntimeSnapshot`, `journey_runtime_snapshot()`, `JourneyClearReason`, and reason-aware clear helper)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — add controller-level `journey_snapshot()` accessor and assemble policy debug data)
- `crates/worldwake-ai/src/planner_ops.rs` (read existing `terminal_travel_destination()` helper; extend only if another plan-derived route query is genuinely needed)

## Out of Scope

- `TravelDispositionProfile` component (ticket 001)
- Journey field lifecycle implementation details (tickets 005, 006)
- Goal switching margin implementation (tickets 003, 004)
- CLI rendering of journey state (future work beyond this ticket)
- Persistent storage of debug-only journey reasons or policy values
- Changes to `worldwake-core`, `worldwake-sim`, or `worldwake-systems` beyond what ticket 004 already requires

## Acceptance Criteria

### Tests That Must Pass

1. `journey_destination()` returns `None` when `journey_established_at` is `None`.
2. `journey_destination()` returns the target of the last Travel step when journey is active.
3. `journey_destination()` returns `None` when plan has no Travel steps.
4. `journey_runtime_snapshot()` returns a `JourneyRuntimeSnapshot` reflecting all temporal fields and derived route state.
5. `JourneyDebugSnapshot` reports the effective switch margin and whether it came from `BudgetDefault` or `JourneyProfile`.
6. `JourneyClearReason` has the required variants and derives `Debug`, `Eq`, and `PartialEq`.
7. `clear_journey_fields_with_reason()` clears all fields regardless of reason.
8. `AgentTickDriver::journey_snapshot()` delegates to runtime data and controller policy correctly.
9. Existing suite: `cargo test -p worldwake-ai`
10. Existing suite: `cargo clippy --workspace`

### Invariants

1. Runtime inspection stays read-only.
2. Controller policy inspection is assembled at the controller layer, not stored as authoritative or route-owned state.
3. `journey_destination()` derives from the plan via plan-level helpers, not from stored route state.
4. `JourneyRuntimeSnapshot` and `JourneyDebugSnapshot` are transient read-models, not serialized.
5. No new authoritative component for journey state.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` — test: `journey_destination_returns_last_travel_target`
2. `crates/worldwake-ai/src/decision_runtime.rs` — test: `journey_destination_returns_none_without_active_journey`
3. `crates/worldwake-ai/src/decision_runtime.rs` — test: `journey_runtime_snapshot_reflects_all_temporal_fields`
4. `crates/worldwake-ai/src/decision_runtime.rs` — test: `clear_journey_fields_with_reason_clears_all_fields`
5. `crates/worldwake-ai/src/agent_tick.rs` — test: `journey_snapshot_reports_profile_margin_source_for_active_journey`
6. `crates/worldwake-ai/src/agent_tick.rs` — test: `journey_snapshot_reports_budget_margin_when_no_active_journey`

### Commands

1. `cargo test -p worldwake-ai decision_runtime`
2. `cargo test -p worldwake-ai agent_tick`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`
