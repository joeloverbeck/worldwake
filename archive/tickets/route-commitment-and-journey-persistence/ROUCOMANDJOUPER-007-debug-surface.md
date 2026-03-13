# ROUCOMANDJOUPER-007: Observable Debug Surface for Journey State

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — add transient runtime inspection plus controller-level journey policy inspection
**Deps**: ROUCOMANDJOUPER-004, ROUCOMANDJOUPER-005, ROUCOMANDJOUPER-006, ROUCOMANDJOUPER-008, ROUCOMANDJOUPER-009

## Problem

Tests and CLI inspection need to observe journey state: whether an agent has an active journey, the durable committed destination, remaining route length on the current plan, temporal field values, the effective switch margin currently in force, and the reason a journey was last cleared. Without a structured debug surface, test assertions and human inspection must re-derive journey state from raw fields and separate controller policy.

After ticket 004, the effective journey switch margin is controller-level policy, not runtime-owned state. A runtime-only debug surface would miss a meaningful part of the behavior.

## Assumption Reassessment (2026-03-13)

1. `AgentDecisionRuntime` already has a durable commitment anchor via `journey_committed_goal`, `journey_committed_destination`, and `journey_commitment_state: Active | Suspended` after tickets 008 and 009 — confirmed.
2. `PlannedPlan` already has `goal`, `steps`, `remaining_travel_steps_from()`, and `terminal_travel_destination()` helpers — confirmed.
3. `AgentTickDriver` still owns runtime state as `runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>` — confirmed.
4. `BeliefView::travel_disposition_profile()` already exists and `effective_goal_switch_margin()` is computed at the controller layer from runtime commitment state, `TravelDispositionProfile`, and the budget default — confirmed.
5. Journey commitment clearing currently happens through `clear_journey_commitment()` in multiple paths, but no explicit read-model exposes the last clear reason yet — confirmed gap.

## Architecture Check

1. The durable committed destination should come from the runtime commitment anchor, not be re-derived from the current plan. That anchor exists specifically so commitment can survive planless replanning seams.
2. Plan-derived facts such as remaining travel steps and terminal travel destination for the current plan should remain derived from `PlannedPlan` helpers. They belong in a transient snapshot, not in new authoritative runtime fields.
3. Controller-derived policy facts such as effective switch margin and margin source do not belong on `AgentDecisionRuntime`; they should be assembled by `AgentTickDriver` or another controller-facing inspection surface.
4. Journey clearing reasons should be explicit and inspectable, but they should remain transient debug instrumentation rather than serialized or authoritative state.
4. No backwards-compatibility aliasing or shims.

## What to Change

### 1. Add `JourneyRuntimeSnapshot`

On `AgentDecisionRuntime`:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JourneyRuntimeSnapshot {
    pub committed_destination: Option<EntityId>,
    pub active_plan_destination: Option<EntityId>,
    pub commitment_state: JourneyCommitmentState,
    pub established_at: Option<Tick>,
    pub last_progress_tick: Option<Tick>,
    pub remaining_travel_steps: usize,
    pub consecutive_blocked_ticks: u32,
    pub has_active_journey_travel: bool,
    pub last_clear_reason: Option<JourneyClearReason>,
}
```

With:

```rust
impl AgentDecisionRuntime {
    pub fn journey_runtime_snapshot(&self) -> JourneyRuntimeSnapshot { ... }
}
```

The snapshot should expose:
- the durable commitment anchor from runtime state
- the current plan terminal travel destination, if any
- remaining travel steps via `PlannedPlan::remaining_travel_steps_from(self.current_step_index)`
- the most recent transient clear reason, if any

Do not add a new `journey_destination()` alias. `journey_committed_destination()` already covers the durable commitment case, and `active_plan_destination` in the snapshot covers plan-derived inspection without blurring the distinction.

### 2. Add controller-level policy inspection types

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

### 3. Add `JourneyClearReason` enum for explicit debug semantics

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum JourneyClearReason {
    GoalSatisfied,
    Reprioritized,
    PlanFailed,
    PatienceExhausted,
    Death,
    LostTravelPlan,
}
```

Replace `clear_journey_commitment()` with a reason-aware helper so every abandonment path says why the commitment disappeared:

```rust
pub fn clear_journey_commitment_with_reason(&mut self, reason: JourneyClearReason) {
    self.last_journey_clear_reason = Some(reason);
    ...
}
```

Retain a zero-argument `clear_journey_commitment()` only if tests or call sites still need it internally, and have it delegate to a single explicit default reason only after all callers are audited. Prefer updating every caller to pass the correct reason instead of preserving an ambiguous helper.

### 4. Expose `JourneyDebugSnapshot` on `AgentTickDriver`

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
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add `JourneyRuntimeSnapshot`, `journey_runtime_snapshot()`, `JourneyClearReason`, transient clear-reason storage, and reason-aware clear helper)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — add controller-level `journey_snapshot()` accessor, margin-source reporting, and explicit clear reasons at every clear site)
- `crates/worldwake-ai/src/planner_ops.rs` (read existing plan travel helpers; extend only if another plan-derived route query is genuinely needed)
- `crates/worldwake-ai/src/lib.rs` (modify if snapshot/debug types need re-export)

## Out of Scope

- `TravelDispositionProfile` component (ticket 001)
- Journey field lifecycle implementation details (tickets 005, 006)
- Goal switching margin implementation (tickets 003, 004)
- CLI rendering of journey state (future work beyond this ticket)
- Persistent storage of debug-only journey reasons or policy values
- Changes to `worldwake-core`, `worldwake-sim`, or `worldwake-systems` beyond what ticket 004 already requires

## Acceptance Criteria

### Tests That Must Pass

1. `journey_runtime_snapshot()` returns a `JourneyRuntimeSnapshot` reflecting the durable commitment anchor, plan-derived route state, temporal fields, and last clear reason.
2. `JourneyDebugSnapshot` reports the effective switch margin, whether it came from `BudgetDefault` or `JourneyProfile`, and the runtime commitment state.
3. `JourneyClearReason` has the required variants and derives `Debug`, `Eq`, and `PartialEq`.
4. `clear_journey_commitment_with_reason()` clears anchor/temporal fields and records the last clear reason.
5. `AgentTickDriver::journey_snapshot()` delegates to runtime data and controller policy correctly.
6. Runtime/controller clear paths record explicit reasons for at least goal completion, plan failure, patience exhaustion, death, and controller-driven abandonment when commitment is actually cleared.
9. Existing suite: `cargo test -p worldwake-ai`
10. Existing suite: `cargo clippy --workspace`

### Invariants

1. Runtime inspection stays read-only.
2. Controller policy inspection is assembled at the controller layer, not stored as authoritative or route-owned state.
3. The durable committed destination remains runtime-owned state; plan-derived route facts remain plan-derived.
4. `JourneyRuntimeSnapshot` and `JourneyDebugSnapshot` are transient read-models, not serialized.
5. No new authoritative component for journey state.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` — test: `journey_runtime_snapshot_reflects_anchor_plan_and_temporal_fields`
2. `crates/worldwake-ai/src/decision_runtime.rs` — test: `clear_journey_commitment_with_reason_records_reason_and_clears_fields`
3. `crates/worldwake-ai/src/agent_tick.rs` — test: `journey_snapshot_reports_profile_margin_source_for_active_journey`
4. `crates/worldwake-ai/src/agent_tick.rs` — test: `journey_snapshot_reports_budget_margin_when_no_profile_override_applies`
5. `crates/worldwake-ai/src/agent_tick.rs` — test: `goal_completion_records_goal_satisfied_clear_reason`
6. `crates/worldwake-ai/src/agent_tick.rs` — test: `dead_ai_agent_is_skipped_by_ai_driver`

### Commands

1. `cargo test -p worldwake-ai decision_runtime`
2. `cargo test -p worldwake-ai agent_tick`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-13
- What changed:
  - Added transient `JourneyRuntimeSnapshot` and controller-level `JourneyDebugSnapshot` read-models.
  - Added explicit `JourneyClearReason` instrumentation and threaded concrete reasons through death, plan failure, patience exhaustion, goal completion, and controller-side abandonment paths.
  - Exposed `AgentTickDriver::journey_snapshot()` so tests and future CLI inspection can read runtime and policy state without re-deriving it externally.
- Deviations from original plan:
  - Did not add a new `journey_destination()` alias. The current architecture already has a durable commitment anchor, and adding a plan-derived alias would blur the distinction between committed destination and current-plan destination.
  - Preserved the existing detour model where non-travel plans suspend commitment rather than clear it.
  - Stored the last clear reason as transient runtime debug state only when journey state actually existed.
- Verification results:
  - `cargo test -p worldwake-ai journey`
  - `cargo test -p worldwake-ai goal_completion_records_goal_satisfied_clear_reason`
  - `cargo test -p worldwake-ai dead_ai_agent_is_skipped_by_ai_driver`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
