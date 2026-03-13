# ROUCOMANDJOUPER-008: Explicit Journey Commitment Anchor on AgentDecisionRuntime

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — decision-runtime commitment state and controller/runtime helpers
**Deps**: archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-002-journey-temporal-fields.md, archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-004-plan-selection-journey-margin.md, archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-005-journey-field-advancement.md

## Problem

Current journey commitment is inferred from the current plan: if `current_plan` has remaining Travel steps and `journey_established_at` is `Some`, the agent is considered to be on an active journey.

That works only while the concrete active plan is itself travel-led. It breaks once the architecture needs to support a temporary local detour that should preserve destination commitment:

- a thirsty agent stopping to drink mid-journey is not abandoning the destination,
- a stale travel plan may need to be dropped for replanning without erasing commitment,
- debug and policy code cannot distinguish "journey suspended for a detour" from "journey abandoned" if the destination exists only inside `current_plan`.

Deriving destination solely from the current plan is therefore too weak for robust interruption semantics. The runtime needs a first-class transient commitment anchor that outlives any single concrete plan instance while still avoiding route storage or a second travel model.

## Assumption Reassessment (2026-03-13)

1. `AgentDecisionRuntime` currently stores temporal journey fields but no explicit committed destination or committed goal; `has_active_journey()` is derived from `journey_established_at` plus remaining Travel steps on `current_plan` — confirmed.
2. `PlannedPlan` already exposes `terminal_travel_destination()` and remaining-travel helpers, which are sufficient to establish a commitment anchor when a travel-led plan is adopted — confirmed.
3. Ticket 005 now preserves temporal journey state across recoverable blocked travel steps while dropping stale plans for replanning. That correction exposes the architectural gap more clearly: commitment can survive longer than any single concrete plan, but the runtime has no dedicated place to represent that — confirmed.
4. The draft spec and existing tickets assume destination/route are always derived from the plan. That assumption no longer cleanly supports temporary detours or suspended commitment and should be corrected rather than worked around with controller heuristics.

## Architecture Check

1. The clean solution is to separate transient destination commitment from transient concrete plan state. A journey commitment is not the same thing as the currently executing plan, and the runtime should model that distinction explicitly.
2. This should remain minimal and concrete: store committed goal identity, committed destination, and commitment status. Do not store route vectors, edge lists, or abstract momentum scores.
3. A transient runtime anchor is cleaner than special-casing "if the current challenger looks like self-care, maybe keep the old journey fields around." Explicit state scales; whitelists do not.
4. No backwards-compatibility aliasing or shims. Replace the current "plan-derived only" commitment model with the new runtime contract and update downstream callers.

## What to Change

### 1. Add explicit commitment fields to `AgentDecisionRuntime`

Add transient runtime fields that represent the committed journey independent of the current concrete plan:

```rust
pub enum JourneyCommitmentState {
    Inactive,
    Active,
    Suspended,
}

pub struct AgentDecisionRuntime {
    // existing fields...
    pub journey_commitment_state: JourneyCommitmentState,
    pub journey_committed_goal: Option<GoalKey>,
    pub journey_committed_destination: Option<EntityId>,
}
```

Meaning:
- `Inactive`: no destination commitment exists.
- `Active`: the agent is currently executing or directly replanning toward the committed destination.
- `Suspended`: the agent is temporarily pursuing a non-destination detour while intending to resume the committed destination afterward.

`journey_committed_destination` is the concrete destination anchor. It replaces the fragile assumption that the destination must always live inside `current_plan`.

### 2. Replace plan-only journey helpers with commitment-aware helpers

Refactor runtime helpers so callers can ask distinct questions:

```rust
pub fn has_journey_commitment(&self) -> bool;
pub fn has_active_journey_travel(&self) -> bool;
pub fn journey_committed_destination(&self) -> Option<EntityId>;
pub fn clear_journey_commitment(&mut self);
```

`has_active_journey_travel()` should answer "am I actively executing a travel-led commitment right now?"

`has_journey_commitment()` should answer "do I still have a destination commitment, even if the current plan is suspended or absent?"

### 3. Establish the commitment anchor when adopting a travel-led plan

When a travel-led plan is adopted:
- derive the destination via `PlannedPlan::terminal_travel_destination()`,
- set `journey_committed_goal`,
- set `journey_committed_destination`,
- mark `journey_commitment_state = Active`.

Same-goal/same-destination replanning should preserve the existing commitment anchor and temporal fields instead of re-establishing them.

### 4. Preserve the commitment anchor when dropping stale plans

Any path that drops a stale travel-led plan for local replanning without actual abandonment should keep:
- `journey_commitment_state`,
- `journey_committed_goal`,
- `journey_committed_destination`,
- journey temporal fields.

This is the foundation that later tickets can use for detours, resume behavior, patience exhaustion, and debug surfaces.

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add explicit commitment fields and helpers)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — establish/preserve commitment anchor during plan adoption and recoverable replanning)
- `crates/worldwake-ai/src/planner_ops.rs` (read existing destination helpers; extend only if another concrete plan-derived query is truly needed)

## Out of Scope

- Controller policy for deciding whether a challenger detour suspends or abandons the commitment (follow-up ticket)
- Route vector storage or cached path storage
- Any new travel action shape
- Save/load serialization of journey commitment state
- Debug surface redesign beyond the runtime contract changes needed here

## Acceptance Criteria

### Tests That Must Pass

1. Adopting a travel-led plan sets an explicit committed destination and committed goal on `AgentDecisionRuntime`.
2. Same-goal/same-destination replanning preserves the existing commitment anchor instead of resetting it.
3. Dropping a stale travel plan for recoverable replanning preserves the commitment anchor.
4. Clearing commitment removes commitment state, committed goal, committed destination, and temporal journey fields together.
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Commitment remains transient runtime state only; it is never authoritative world state.
2. No route list, edge list, or abstract progress scalar is stored.
3. Destination commitment can outlive a single concrete plan instance.
4. No backwards-compatibility shim preserves the old "commitment exists only if current_plan has travel steps" contract.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` — add tests for commitment helpers and full commitment clearing because the runtime contract is changing.
2. `crates/worldwake-ai/src/agent_tick.rs` — add tests for travel-led plan adoption establishing a committed destination and for recoverable replanning preserving it.
3. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — strengthen at least one multi-leg detour scenario to assert that commitment survives absence or replacement of the immediate concrete plan.

### Commands

1. `cargo test -p worldwake-ai decision_runtime`
2. `cargo test -p worldwake-ai agent_tick`
3. `cargo test -p worldwake-ai --test golden_ai_decisions`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`
