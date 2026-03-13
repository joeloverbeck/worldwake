# ROUCOMANDJOUPER-009: Journey-Preserving Detour and Abandonment Policy

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — relation-aware controller policy and transient journey suspension semantics
**Deps**: archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-004-plan-selection-journey-margin.md, archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-005-journey-field-advancement.md, ROUCOMANDJOUPER-008-explicit-journey-commitment-anchor.md

## Problem

Ticket 008 already fixed the original planless-commitment gap by adding a durable transient journey anchor. The remaining defect is narrower and more concrete:

- `select_best_plan()` still compares replacements with one scalar margin and no relation to the current commitment.
- `evaluate_interrupt()` still reasons from ranked goals, not from concrete challenger plans.
- `update_journey_fields_for_adopted_plan()` currently clears commitment for every non-travel plan, so a local detour still looks like permanent abandonment.

That means the current controller still conflates three different situations:

- refreshing the current route toward the same committed destination,
- suspending the journey for a temporary local detour,
- abandoning the committed destination for a different destination or goal.

Those are not architecturally equivalent. A temporary detour should not destroy the durable commitment anchor, and abandonment protection should apply only when the challenger would actually replace that commitment.

## Assumption Reassessment (2026-03-13)

1. Ticket 008 is already implemented. `AgentDecisionRuntime` now has `journey_committed_goal`, `journey_committed_destination`, `has_journey_commitment()`, `has_active_journey_travel()`, and `clear_journey_commitment()` — confirmed.
2. `effective_goal_switch_margin()` already keys off any durable journey commitment, not only `has_active_journey_travel()` — confirmed in `agent_tick.rs`.
3. `select_best_plan()` and `evaluate_interrupt()` already consume explicit controller-computed margin input after ticket 004 — confirmed.
4. The current interrupt path still evaluates challengers from ranked goals rather than classified concrete challenger plans. That prevents relation-aware detour vs abandonment policy in the active-action branch — confirmed.
5. `update_journey_fields_for_adopted_plan()` currently calls `clear_journey_commitment()` for every adopted non-travel plan. That means local detours still erase the journey anchor — confirmed.
6. `advance_completed_step()` currently clears commitment on `GoalSatisfied` and `CombatCommitment` terminal plans unconditionally. Once suspended detours exist, detour completion must preserve commitment instead of clearing it — confirmed.
7. Ticket 006 currently assumes broad plan-replacement clearing. That assumption becomes incorrect once detour suspension exists and should be revised after this ticket lands.
8. Ticket 007 currently assumes the debug surface only needs the commitment anchor plus temporal fields. Once suspension exists, debug output also needs explicit commitment status and recent relation/policy context.

## Architecture Check

1. The robust solution is to classify a concrete plan relative to the committed journey before deciding which switch policy applies. This is cleaner than goal-kind whitelists or "if non-travel then maybe preserve" branches scattered across lifecycle code.
2. Journey-preserving detours should be first-class controller/runtime outcomes, not accidental side effects of margin tuning or broad clearing exceptions.
3. The durable commitment anchor from ticket 008 is the right foundation. This ticket should extend it with explicit transient commitment status (`Active` vs `Suspended`) rather than replacing it with a new commitment object.
4. Plan selection and interrupt policy should use the same relation vocabulary. Idle and active branches can differ in orchestration, but they should not have divergent definitions of refresh vs suspend vs abandon.
5. No backwards-compatibility aliasing or shims.

## What to Change

### 1. Add explicit commitment status plus relation classification

Add a small transient status enum on `AgentDecisionRuntime`:

```rust
pub enum JourneyCommitmentState {
    Active,
    Suspended,
}
```

Add a controller/runtime relation enum for comparing a concrete plan to the current commitment:

```rust
pub enum JourneyPlanRelation {
    NoCommitment,
    RefreshesCommitment,
    SuspendsCommitment,
    AbandonsCommitment,
}
```

Semantics:
- `RefreshesCommitment`: same committed goal and destination, new concrete plan.
- `SuspendsCommitment`: a local plan temporarily replaces the current plan without replacing the durable commitment anchor.
- `AbandonsCommitment`: a plan would replace the committed goal or destination.

The relation should be computed against the explicit commitment anchor from ticket 008, not by re-deriving commitment from the current plan.

### 2. Make plan selection relation-aware

Update `select_best_plan()` so it no longer assumes one scalar margin is sufficient for every replacement. It should:

- compare each actionable challenger plan against the current commitment relation,
- always allow `RefreshesCommitment`,
- evaluate `SuspendsCommitment` against the default controller switch margin,
- evaluate `AbandonsCommitment` against `route_replan_margin`,
- fall back to the current plan when no challenger satisfies its relation-specific policy.

This keeps the abandonment guard focused on true destination replacement instead of blocking local corrective detours.

### 3. Make interrupt decisions plan-backed when commitment exists

When the agent has a journey commitment and the active-action branch considers interruption:

- inspect at least one concrete actionable challenger plan before finalizing detour-vs-abandon policy,
- classify that plan against the current commitment,
- apply the same relation vocabulary as idle plan selection,
- keep existing reactive-goal interrupt restrictions unless the concrete plan relation requires a different decision.

This avoids goal-only guesses about whether an interrupt is a temporary detour or a permanent abandonment.

### 4. Split switch-margin policy by relation

Use different controller rules for different relation classes:

- `RefreshesCommitment`: always allowed if it improves the same commitment.
- `SuspendsCommitment`: use the default switch/interrupt policy for the detour itself, but keep the commitment anchor intact and mark it `Suspended`.
- `AbandonsCommitment`: require the challenger to satisfy `route_replan_margin`.

`NoCommitment` continues to use the default controller switch policy.

### 5. Add explicit suspend/resume lifecycle hooks

When a detour suspends the journey:
- keep `journey_committed_goal` and `journey_committed_destination`,
- set `journey_commitment_state = Suspended`,
- allow the detour plan to replace the current concrete plan.

When the detour resolves and the controller returns to the committed destination:
- mark `journey_commitment_state = Active`,
- replan toward the committed destination using normal planning machinery,
- preserve the original temporal commitment fields unless a true abandonment/clear condition occurs.

Concretely, detour-plan completion should no longer fall through the same unconditional commitment-clearing path used by committed-journey completion.

### 6. Revise downstream lifecycle tickets to use the new semantics

After this ticket lands:
- ticket 006 should clear commitment only on true abandonment, patience exhaustion, death, incapacity, or explicit invalidation,
- ticket 007 should expose commitment state (`Active` vs `Suspended`) and the relation/policy source used for recent controller decisions.

This ticket should update those assumptions, not preserve their older broader-clear semantics.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — classify relation, suspend/resume commitment, and apply relation-specific policy)
- `crates/worldwake-ai/src/interrupts.rs` (modify or narrow — active-action interruption must inspect concrete challenger plans when commitment exists)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — replacement must use relation-aware policy instead of one blanket margin)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add `JourneyCommitmentState` and helpers)
- `archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-006-blocked-leg-patience-exhaustion.md` (modify — correct assumptions/scope after the architectural change lands)
- `tickets/ROUCOMANDJOUPER-007-debug-surface.md` (modify — correct assumptions/scope after the architectural change lands)

## Out of Scope

- Changing authoritative travel, topology, or route occupancy systems
- Storing or caching entire routes
- Introducing special-case continuous travel actions
- Save/load persistence of commitment state
- Any compatibility layer that preserves the old blanket journey-margin policy

## Acceptance Criteria

### Tests That Must Pass

1. Adopting a temporary local detour plan during a committed journey preserves `journey_committed_goal` and `journey_committed_destination` and marks the commitment `Suspended`.
2. Completing that detour does not clear the commitment; the controller can resume planning toward the committed destination without reconstructing commitment from old plan residue.
3. A same-destination route refresh preserves commitment and does not pay abandonment margin.
4. A challenger that would replace the committed destination still must satisfy `route_replan_margin`.
5. Active-action interruption during a committed journey no longer decides detour vs abandonment from ranked goals alone.
6. Ticket 006's clearing semantics no longer clear commitment merely because a temporary detour plan replaced the current travel plan.
7. Existing suite: `cargo test -p worldwake-ai`
8. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Route commitment protects destination abandonment, not all temporary local deviations.
2. Temporary detours do not require route storage or a second travel model.
3. The controller distinguishes refresh vs suspend vs abandon from concrete plan/commitment relation, not from heuristic goal whitelists.
4. Commitment state remains transient runtime/controller state only.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` — add tests for commitment-state defaults and helpers because suspension becomes part of the runtime contract.
2. `crates/worldwake-ai/src/agent_tick.rs` — add controller tests for `RefreshesCommitment`, `SuspendsCommitment`, and `AbandonsCommitment` classification because policy correctness lives at the orchestration layer.
3. `crates/worldwake-ai/src/plan_selection.rs` — add relation-aware replacement coverage so same-destination refresh, temporary detour, and different-destination abandonment do not share one blanket threshold.
4. `crates/worldwake-ai/src/interrupts.rs` — update interrupt regressions so commitment-preserving detours and commitment abandonment are covered separately when concrete challenger plans are available.
5. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — strengthen the multi-leg thirst-interruption scenario so it asserts suspend-then-resume behavior rather than merely "the agent drank somewhere."
6. `archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-006-blocked-leg-patience-exhaustion.md` and `tickets/ROUCOMANDJOUPER-007-debug-surface.md` — update ticket assumptions and acceptance criteria to match the new architecture.

### Commands

1. `cargo test -p worldwake-ai decision_runtime`
2. `cargo test -p worldwake-ai agent_tick`
3. `cargo test -p worldwake-ai plan_selection`
4. `cargo test -p worldwake-ai interrupts`
5. `cargo test -p worldwake-ai --test golden_ai_decisions`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Outcome amended: 2026-03-13

- Completion date: 2026-03-13
- What actually changed:
  - Added transient `JourneyCommitmentState` plus concrete `JourneyPlanRelation` classification on `AgentDecisionRuntime`.
  - Made plan selection relation-aware so same-commitment refresh, local detour suspension, and true abandonment no longer share one blanket threshold.
  - Made the active interrupt path inspect concrete challenger plans when a durable journey commitment exists, and removed the old goal-only fallback in that case.
  - Preserved commitment across adopted non-travel detours and reactivated it on detour completion instead of clearing it unconditionally.
  - Centralized relation-aware switch-margin selection in a shared internal helper so idle plan replacement and active interrupts cannot silently drift apart again.
  - Strengthened unit and golden coverage around suspend/resume behavior and updated downstream tickets 006/007 assumptions to match the new architecture.
- Deviations from original plan:
  - The ticket was narrowed before implementation because ticket 008 had already delivered the durable commitment anchor; this work extended that anchor instead of redesigning it.
  - The strengthened golden regression now asserts post-detour journey completion after departure rather than a brittle specific intermediate stop, while the exact suspend/refresh/abandon distinctions are covered by the new unit tests.
  - No new travel action model, route storage, or compatibility layer was introduced.
- Verification results:
  - `cargo test -p worldwake-ai journey_switch_policy -- --nocapture` ✅
  - `cargo test -p worldwake-ai decision_runtime` ✅
  - `cargo test -p worldwake-ai plan_selection` ✅
  - `cargo test -p worldwake-ai interrupts` ✅
  - `cargo test -p worldwake-ai agent_tick` ✅
  - `cargo test -p worldwake-ai golden_goal_switching_during_multi_leg_travel -- --nocapture` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
