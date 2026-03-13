# ROUCOMANDJOUPER-009: Journey-Preserving Detour and Abandonment Policy

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — controller policy, interrupt orchestration, and journey lifecycle semantics
**Deps**: archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-004-plan-selection-journey-margin.md, archive/tickets/route-commitment-and-journey-persistence/ROUCOMANDJOUPER-005-journey-field-advancement.md, ROUCOMANDJOUPER-008-explicit-journey-commitment-anchor.md

## Problem

The current controller uses one scalar policy for all reprioritization while a journey is active: if `has_active_journey()` is true, `route_replan_margin` replaces the default switch margin.

That is too coarse. It treats these distinct situations as if they were the same decision:

- abandoning a committed destination for a different destination or goal,
- taking a temporary local detour for urgent self-care or local danger response,
- refreshing the current route toward the same destination.

Those are not architecturally equivalent. A temporary detour should not require "beating" the committed destination by the same margin as permanent abandonment, and resuming the journey after the detour should not depend on ad hoc tests or profile overrides.

The controller needs an explicit policy distinction between:
- `PreserveCommitment`
- `SuspendCommitment`
- `AbandonCommitment`

without introducing compatibility hacks, goal-kind whitelists, or a second travel model.

## Assumption Reassessment (2026-03-13)

1. `select_best_plan()` and `evaluate_interrupt()` already consume an explicit controller-computed switch margin after ticket 004 — confirmed.
2. The current interrupt path evaluates challengers from ranked goals, not from classified concrete challenger plans. That limits the controller's ability to distinguish temporary detours from destination abandonment in a principled way — confirmed.
3. Ticket 006 currently assumes plan replacement should clear journey state broadly. That assumption is too blunt once the architecture supports suspended-but-preserved destination commitment and should be revised after this ticket lands.
4. Ticket 007 currently assumes the debug surface can derive most journey facts from the current plan plus temporal fields. Once suspended commitment exists, debug output will also need the explicit commitment anchor/status from ticket 008.

## Architecture Check

1. The robust solution is to classify a challenger relative to the committed journey before deciding which switch policy applies. This is cleaner than embedding "self-care exception" branches inside low-level interrupt code.
2. Journey-preserving detours should be first-class controller outcomes, not accidental side effects of margin tuning.
3. The controller should reason from concrete plan/commitment relations, not from hardcoded lists of goal kinds. Goal kinds may inform planning, but commitment-preserving vs commitment-abandoning is a controller orchestration concern.
4. No backwards-compatibility aliasing or shims. This ticket should replace the current blanket "active journey => route margin everywhere" rule with explicit relation-based policy.

## What to Change

### 1. Add controller-level journey relation classification

Introduce a controller-facing classification for the selected or proposed challenger:

```rust
pub enum JourneyPlanRelation {
    NoCommitment,
    RefreshesCommitment,
    SuspendsCommitment,
    AbandonsCommitment,
}
```

Semantics:
- `RefreshesCommitment`: same committed goal/destination, new concrete plan.
- `SuspendsCommitment`: local detour plan does not fulfill the committed destination but also does not replace it.
- `AbandonsCommitment`: challenger replaces the committed goal/destination.

The relation should be computed against the explicit commitment anchor from ticket 008, not by re-deriving everything from the current plan.

### 2. Make interrupt decisions plan-backed when commitment exists

When the agent has a journey commitment and a challenger is strong enough to consider interruption:
- build or inspect a concrete challenger plan before finalizing the interrupt outcome,
- classify its relation to the current commitment,
- choose policy based on that relation.

This avoids heuristic goal-only guesses about whether an interrupt is a temporary detour or a permanent abandonment.

### 3. Split switch-margin policy by relation

Use different controller rules for different relation classes:

- `RefreshesCommitment`: always allowed if it improves the same commitment.
- `SuspendsCommitment`: use the default switch/interrupt policy for the detour itself, but keep the commitment anchor intact and mark it `Suspended`.
- `AbandonsCommitment`: require the challenger to satisfy `route_replan_margin`.

This keeps route commitment focused on protecting destination abandonment, not on blocking urgent local corrective action.

### 4. Add explicit suspend/resume lifecycle hooks

When a detour suspends the journey:
- keep `journey_committed_goal` and `journey_committed_destination`,
- set `journey_commitment_state = Suspended`,
- allow the detour plan to replace the current concrete plan.

When the detour resolves and the controller returns to the committed destination:
- mark `journey_commitment_state = Active`,
- replan toward the committed destination using normal planning machinery,
- preserve the original temporal commitment fields unless a true abandonment/clear condition occurs.

### 5. Revise downstream lifecycle tickets to use the new semantics

After this ticket lands:
- ticket 006 should clear commitment only on true abandonment, patience exhaustion, death, incapacity, or explicit invalidation,
- ticket 007 should expose commitment state (`Active` vs `Suspended`) and the relation/policy source used for recent controller decisions.

This ticket should update those assumptions, not preserve their older broader-clear semantics.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — classify relation, suspend/resume commitment, and apply relation-specific policy)
- `crates/worldwake-ai/src/interrupts.rs` (modify or narrow — consume relation-aware policy rather than a blanket journey margin)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — same-destination refresh vs commitment abandonment should not share the same comparison path)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — integrate commitment state transitions from ticket 008)
- `tickets/ROUCOMANDJOUPER-006-journey-clearing-conditions.md` (modify — correct assumptions/scope after the architectural change lands)
- `tickets/ROUCOMANDJOUPER-007-debug-surface.md` (modify — correct assumptions/scope after the architectural change lands)

## Out of Scope

- Changing authoritative travel, topology, or route occupancy systems
- Storing or caching entire routes
- Introducing special-case continuous travel actions
- Save/load persistence of commitment state
- Any compatibility layer that preserves the old blanket journey-margin policy

## Acceptance Criteria

### Tests That Must Pass

1. A temporary self-care detour during a committed journey suspends rather than abandons the commitment.
2. A same-destination route refresh preserves commitment without paying abandonment margin.
3. A challenger that would replace the committed destination still must satisfy `route_replan_margin`.
4. After a suspended detour resolves, the controller can resume planning toward the committed destination without reconstructing commitment from old plan residue.
5. Ticket 006's clearing semantics no longer clear commitment merely because a temporary detour plan replaced the current travel plan.
6. Existing suite: `cargo test -p worldwake-ai`
7. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Route commitment protects destination abandonment, not all temporary local deviations.
2. Temporary detours do not require route storage or a second travel model.
3. The controller distinguishes suspend vs abandon explicitly; it does not rely on heuristic goal whitelists.
4. Commitment state remains transient runtime/controller state only.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` — add controller tests for `RefreshesCommitment`, `SuspendsCommitment`, and `AbandonsCommitment` classification because policy correctness lives at the orchestration layer.
2. `crates/worldwake-ai/src/interrupts.rs` — update interrupt regressions so journey-preserving detours and commitment abandonment are covered separately.
3. `crates/worldwake-ai/src/plan_selection.rs` — add same-destination refresh vs different-destination replacement coverage under an explicit commitment anchor.
4. `crates/worldwake-ai/tests/golden_ai_decisions.rs` — strengthen the multi-leg thirst-interruption scenario so it asserts suspend-then-resume behavior rather than merely "the agent drank somewhere."
5. `tickets/ROUCOMANDJOUPER-006-journey-clearing-conditions.md` and `tickets/ROUCOMANDJOUPER-007-debug-surface.md` — update ticket assumptions and acceptance criteria to match the new architecture.

### Commands

1. `cargo test -p worldwake-ai agent_tick`
2. `cargo test -p worldwake-ai interrupts`
3. `cargo test -p worldwake-ai plan_selection`
4. `cargo test -p worldwake-ai --test golden_ai_decisions`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`
