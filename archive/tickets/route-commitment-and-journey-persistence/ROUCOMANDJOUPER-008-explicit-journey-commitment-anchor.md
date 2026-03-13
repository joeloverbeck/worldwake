# ROUCOMANDJOUPER-008: Explicit Journey Commitment Anchor on AgentDecisionRuntime

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — decision-runtime commitment state and controller/runtime helpers
**Deps**: archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-002-journey-temporal-fields.md, archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-004-plan-selection-journey-margin.md, archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-005-journey-field-advancement.md

## Problem

Current journey state is only partially durable. The runtime keeps temporal journey fields, but destination commitment is still inferred from the current concrete plan.

That is too weak in code paths that intentionally drop the concrete plan while preserving the underlying journey intent:

- `ProgressBarrier` completion clears `current_plan` but keeps `current_goal` and journey temporal fields.
- recoverable blocked travel drops `current_plan` so the controller can replan the next leg.
- same-goal replanning can refresh the concrete route while preserving the higher-level commitment.

Across those seams, the current architecture loses the committed destination and falls back to non-journey behavior such as the default switch margin. The problem is real today even before ticket 009's suspend/resume detour policy exists.

The runtime therefore needs a first-class transient commitment anchor that can outlive a single plan instance without storing routes or creating a second travel model.

## Assumption Reassessment (2026-03-13)

1. `AgentDecisionRuntime` currently stores only temporal journey fields; it does not store an explicit committed goal or committed destination — confirmed.
2. `PlannedPlan` already exposes `terminal_travel_destination()` and remaining-travel helpers, so the destination can be derived once at adoption time and retained as transient runtime state — confirmed.
3. The current code already has planless replanning seams that preserve some journey state: `ProgressBarrier` completion and recoverable blocked travel both drop `current_plan` without necessarily abandoning the higher-level journey intent — confirmed.
4. `effective_goal_switch_margin()` currently keys off `has_active_journey()`, which requires remaining travel steps on `current_plan`. That means a still-committed agent can silently lose journey margin protection during planless replanning — confirmed.
5. The draft spec and index currently say destination/route are always derived from the plan. That is no longer accurate for destination commitment once the runtime supports planless replanning seams. Route can remain plan-derived; committed destination should not. This ticket corrects that runtime contract.
6. Explicit suspend/resume state for temporary detours is not needed to solve the current gap and should remain in ticket 009, where controller relation semantics are introduced. Adding `Suspended` state here would be speculative.

## Architecture Check

1. The clean solution is to separate transient destination commitment from transient concrete plan state. A committed destination is not the same thing as the currently materialized plan.
2. The minimal concrete anchor needed now is: committed goal identity plus committed destination. That is enough to preserve controller policy across planless replanning seams.
3. Route shape should remain derived from the current plan. This ticket should not store route vectors, edge lists, or progress scalars.
4. Detour suspension semantics belong in ticket 009. This ticket should provide the anchor that 009 builds on, not pre-bake unused controller states.
5. No backwards-compatibility aliasing or shims. Update the runtime contract and downstream callers directly.

## What to Change

### 1. Add explicit committed goal/destination fields to `AgentDecisionRuntime`

Add transient runtime fields that represent the durable journey anchor independent of the current concrete plan:

```rust
pub struct AgentDecisionRuntime {
    // existing fields...
    pub journey_committed_goal: Option<GoalKey>,
    pub journey_committed_destination: Option<EntityId>,
}
```

Meaning:
- `journey_committed_goal`: the current committed journey goal, when one exists.
- `journey_committed_destination`: the concrete committed destination anchor.

No suspend/active enum yet. That belongs to ticket 009 once detour semantics exist.

### 2. Replace the runtime contract with commitment-aware helpers

Refactor runtime helpers so callers can ask distinct questions:

```rust
pub fn has_journey_commitment(&self) -> bool;
pub fn has_active_journey_travel(&self) -> bool;
pub fn journey_committed_destination(&self) -> Option<EntityId>;
pub fn clear_journey_commitment(&mut self);
```

Semantics:
- `has_journey_commitment()`: does a durable committed goal/destination exist right now?
- `has_active_journey_travel()`: is there currently a concrete plan with remaining travel steps for that commitment?
- `clear_journey_commitment()`: clears committed goal, committed destination, and the existing temporal journey fields together.

This replaces the old plan-only notion of "active journey" as the sole runtime contract.

### 3. Establish or refresh the commitment anchor when adopting a travel-led plan

When a travel-led plan is adopted:
- derive the destination via `PlannedPlan::terminal_travel_destination()`,
- set `journey_committed_goal`,
- set `journey_committed_destination`,
- establish journey temporal fields if this is a new commitment.

Same-goal and same-destination replanning should preserve the existing temporal fields instead of re-establishing them.

Same-goal but different-destination replanning should replace the commitment anchor and restart temporal tracking.

### 4. Preserve the commitment anchor across planless replan seams

Any path that intentionally drops the concrete plan without abandoning the journey should keep:
- `journey_committed_goal`,
- `journey_committed_destination`,
- journey temporal fields.

This applies to the currently implemented seams:
- recoverable blocked travel-step replanning,
- `ProgressBarrier` completion that requires replanning the next leg.

### 5. Use the commitment anchor for controller-level journey policy

Controller/runtime policy that is supposed to protect the current journey commitment should key off the commitment anchor, not just current-plan travel steps.

In particular, `effective_goal_switch_margin()` should continue using `TravelDispositionProfile::route_replan_margin` while a durable journey commitment exists, even if the current concrete plan has been dropped for immediate replanning.

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add committed goal/destination fields and helpers)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — establish/preserve/clear the commitment anchor and use it in controller policy)
- `crates/worldwake-ai/src/planner_ops.rs` (read existing destination helpers; extend only if another concrete plan-derived query is truly needed)
- `tickets/ROUCOMANDJOUPER-000-index.md` (modify — correct the family-level invariant that destination is always plan-derived)

## Out of Scope

- Controller policy for deciding whether a challenger detour suspends or abandons commitment (ticket 009)
- Any `JourneyCommitmentState` or `Suspended` state enum
- Route vector storage or cached path storage
- Any new travel action shape
- Save/load serialization of journey commitment state
- Debug surface redesign beyond the runtime contract changes needed here

## Acceptance Criteria

### Tests That Must Pass

1. Adopting a travel-led plan sets an explicit committed destination and committed goal on `AgentDecisionRuntime`.
2. Same-goal and same-destination replanning preserves the existing commitment anchor and temporal fields.
3. Same-goal but different-destination replanning replaces the commitment anchor and restarts temporal tracking.
4. Recoverable blocked-travel replanning preserves the commitment anchor while dropping the concrete plan.
5. `ProgressBarrier` completion preserves the commitment anchor while dropping the concrete plan.
6. Clearing commitment removes committed goal, committed destination, and temporal journey fields together.
7. Controller margin policy continues to use `route_replan_margin` while a durable journey commitment exists during planless replanning.
8. Existing suite: `cargo test -p worldwake-ai`
9. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Commitment remains transient runtime state only; it is never authoritative world state.
2. No route list, edge list, or abstract progress scalar is stored.
3. Committed destination can outlive a single concrete plan instance.
4. Route shape remains plan-derived when a concrete plan exists.
5. No backwards-compatibility shim preserves the old "journey exists only if current_plan has travel steps" contract.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` — add tests for commitment helpers and full commitment clearing because the runtime contract is changing.
2. `crates/worldwake-ai/src/agent_tick.rs` — add tests for travel-led plan adoption establishing a committed destination, same-destination replanning preservation, different-destination replacement, blocked-step preservation, and `ProgressBarrier` preservation.
3. `crates/worldwake-ai/src/agent_tick.rs` — add coverage showing controller margin selection remains journey-aware while the commitment exists but `current_plan` is absent.

### Commands

1. `cargo test -p worldwake-ai decision_runtime`
2. `cargo test -p worldwake-ai agent_tick`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Implemented a minimal durable journey anchor rather than the broader suspended-detour model originally proposed.

- Added transient `journey_committed_goal` and `journey_committed_destination` fields to `AgentDecisionRuntime`.
- Replaced the old plan-only helper contract with commitment-aware runtime helpers and clearing semantics.
- Preserved commitment across the currently real planless replanning seams: recoverable blocked travel and `ProgressBarrier` completion.
- Updated controller margin selection to honor the durable journey commitment even when `current_plan` is temporarily absent.
- Deferred explicit suspend/resume detour state to ticket 009, where the controller relation semantics belong.
