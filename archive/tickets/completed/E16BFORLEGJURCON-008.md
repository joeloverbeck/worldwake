# E16BFORLEGJURCON-008: Wire force-law `ClaimOffice` through AI planning and candidate generation

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI planner ops, goal model, planning snapshot/state, candidate generation
**Deps**: E16BFORLEGJURCON-007, E16BFORLEGJURCON-006

## Problem

AI agents currently cannot pursue force-law office claims end to end even though the force-claim actions already exist in `worldwake-systems`. The AI layer still treats `GoalKind::ClaimOffice` as support-law-only in three places:

1. `planner_ops.rs` intentionally leaves `press_force_claim` / `yield_force_claim` unclassified.
2. `candidate_generation.rs` explicitly omits political candidates for `SuccessionLaw::Force`.
3. `goal_model.rs` and planning state treat `ClaimOffice` satisfaction as "has support majority", which only matches support succession.

The clean architectural target is to keep one goal, `GoalKind::ClaimOffice`, for the shared political outcome, and branch at the AI layer by office-local succession law and belief state. Force-law offices should use `PressForceClaim` toward belief-level force control, while support-law offices keep using support declarations. No compatibility aliasing and no extra goal kind.

## Assumption Reassessment (2026-03-22)

1. `PlannerOpKind` currently contains 20 variants. `PressForceClaim` and `YieldForceClaim` are absent, and `planner_ops.rs` tests explicitly expect the registered action defs `press_force_claim` and `yield_force_claim` to remain unclassified today.
2. The force-claim actions already exist in `worldwake-systems/src/office_actions.rs` with affordance enumeration, payload validators, and commit handlers. This ticket is AI integration, not action creation.
3. `GoalKind::ClaimOffice` and `GoalKindTag::ClaimOffice` already exist. Introducing a new goal kind for force-law claims would be the wrong abstraction because the desired outcome remains "secure the office"; only the lawful path differs by office-local succession law.
4. `candidate_generation.rs` does not "already cover political scenarios generally" for force law. It has an explicit `ForceSuccessionLaw` omission path and tests asserting that force-law offices must not emit `ClaimOffice` or `SupportCandidateForOffice`.
5. `goal_model.rs` currently hard-codes `ClaimOffice` satisfaction as `state.has_support_majority(office, actor)`. That is a support-law assumption and must be generalized so support offices still use support-majority logic while force offices use belief-level force-controller logic.
6. `planning_snapshot.rs` and `planning_state.rs` currently cache and override office-holder/support beliefs, but not force-controller beliefs. If the planner needs to reason about force-law terminal states without omniscient reads, these AI planning structures must carry `believed_force_controller`.
7. `YieldForceClaim` exists as a real action, but there is no evidence in the current AI goal model that it is required on the critical path for `ClaimOffice`. Its planner classification should be added only if it materially improves force-law planning/replanning rather than as symmetry for its own sake.
8. `SupportCandidateForOffice` remains support-law-only. Force-law offices should emit `ClaimOffice` when appropriate, but should not emit support-candidate goals or attempt to route through `DeclareSupport`.
9. This ticket depends on `E16BFORLEGJURCON-006` having already supplied `believed_force_controller()` through the belief-view layer, which is now present in `worldwake-sim`.
10. Existing AI tests and decision-trace assertions currently encode the old exclusion behavior. Those tests must be updated as part of the ticket because they describe stale architecture, not a desired invariant.

## Architecture Check

1. The desirable long-term architecture is one political goal (`ClaimOffice`) with succession-law-specific planner behavior driven by office-local data and belief queries. That keeps the goal vocabulary stable while allowing support and force institutions to evolve independently behind the same high-level intent.
2. `YieldForceClaim` should not be threaded through the planner unless there is a concrete replanning or failure-handling benefit. Clean architecture favors adding the minimum operation surface that the planner can justify.
3. No backward-compatibility shims. No goal aliases. Force-law support exclusions that encode the old architecture should be removed rather than preserved.

## Verification Layers

1. `press_force_claim` action def is classified into `PlannerOpKind::PressForceClaim` and mapped to `GoalKindTag::ClaimOffice`
2. `GoalKind::ClaimOffice` satisfaction branches by succession law:
   - support law -> support-majority logic remains
   - force law -> belief-level `believed_force_controller()` logic
3. `ClaimOffice` relevant planner ops include `PressForceClaim` for force offices without routing force offices through `DeclareSupport`
4. Force-law `ClaimOffice` candidates emit when the agent is eligible and the office is believed vacant, uncontrolled, or controlled by an enemy
5. Force-law political diagnostics no longer record `ForceSuccessionLaw` omission for `ClaimOffice`
6. `SupportCandidateForOffice` remains absent for force-law offices

## What to Change

### 1. Add `PlannerOpKind::PressForceClaim`

Semantics:
- **Goal relevance**: `GoalKindTag::ClaimOffice`
- **Transition kind**: goal-model fallback
- **Planner usage**: available to `GoalKind::ClaimOffice` when the office uses `SuccessionLaw::Force`
- **Hypothetical effect**: update planning-state force-controller belief for the office to `(Some(actor), false)` when the actor is at jurisdiction

This remains belief-level planner modeling, not an authoritative shortcut.

### 2. Rework `ClaimOffice` goal logic to branch by succession law

- Keep `ClaimOffice` as the single high-level goal.
- Support-law offices continue to use the current support-majority path.
- Force-law offices should:
  - treat `PressForceClaim` as the terminal political step
  - use `believed_force_controller()` for satisfaction
  - use office jurisdiction as the place prerequisite when travel is needed
  - avoid `DeclareSupport`, `Bribe`, and `Threaten` unless future design work gives those force-law meaning

### 3. Extend planning snapshot/state to carry force-controller beliefs

- Snapshot force-controller beliefs for included offices.
- Add planning-state read/override helpers for force-controller beliefs.
- Use those helpers in `GoalKind::ClaimOffice` satisfaction and in the hypothetical transition for `PressForceClaim`.

### 4. Ensure candidate generation covers force offices cleanly

Extend `candidate_generation.rs` so that for `SuccessionLaw::Force` offices:
- eligible agents can emit `ClaimOffice { office }`
- support-candidate goals are still suppressed
- emission is driven by belief-level force-control state, not support-vacancy assumptions

At minimum this should cover:
- believed vacant/uncontrolled offices
- offices believed controlled by an enemy

It should not require support-law office-register consultation just to consider a force claim.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — add 2 variants + semantics)
- `crates/worldwake-ai/src/goal_model.rs` (modify — branch `ClaimOffice` behavior by succession law)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — capture force-controller beliefs)
- `crates/worldwake-ai/src/planning_state.rs` (modify — carry hypothetical force-controller beliefs)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — emit force-law `ClaimOffice`, keep support-candidate exclusion)
- `crates/worldwake-ai/src/agent_tick.rs` (modify tests/trace expectations if the old omission invariant is asserted there)

## Out of Scope

- Force-claim action handlers and affordance enumeration — already present from prior E16b work
- Institutional belief queries — E16BFORLEGJURCON-006
- Force control system — E16BFORLEGJURCON-005
- Golden E2E tests — E16BFORLEGJURCON-009
- Action handlers — E16BFORLEGJURCON-004

## Acceptance Criteria

### Tests That Must Pass

1. `PlannerOpKind::PressForceClaim` is classified from the registered `press_force_claim` action and marked relevant to `GoalKindTag::ClaimOffice`
2. `GoalKind::ClaimOffice` remains satisfied by support majority for support-law offices
3. `GoalKind::ClaimOffice` is satisfied for force-law offices only when `believed_force_controller()` reads `(Some(actor), false)`
4. Force-law `ClaimOffice` candidates are generated for eligible agents when the office is believed uncontrolled/vacant
5. Force-law `ClaimOffice` candidates are generated for eligible agents when the office is believed controlled by an enemy
6. Force-law offices do not emit `SupportCandidateForOffice`
7. Ineligible agents do not emit force-law `ClaimOffice`
8. Existing AI suite passes, including updated trace/candidate expectations

### Invariants

1. `ClaimOffice` stays the single goal for office acquisition across succession laws
2. Force-law planning uses belief-level force-controller reads, not authoritative controller reads
3. Support-law planning behavior remains intact
4. Force-law candidate generation and planning use office-local succession law and jurisdiction data, not hardcoded global branches

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` test module — classify `press_force_claim` and verify relevant goal semantics
2. `crates/worldwake-ai/src/goal_model.rs` test module — `ClaimOffice` satisfaction for both support and force succession
3. `crates/worldwake-ai/src/candidate_generation.rs` test module — force-law `ClaimOffice` emission and continued force-law support-candidate suppression
4. `crates/worldwake-ai/src/agent_tick.rs` test module — update decision-trace expectations if it currently encodes the old force-law omission

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-22
- What actually changed:
  - Added AI planner classification and semantics for `PressForceClaim`
  - Extended planning snapshot/state to carry belief-level force-controller data
  - Reworked `GoalKind::ClaimOffice` to branch by `SuccessionLaw`, keeping support-law majority logic and adding force-law controller logic
  - Updated candidate generation so force-law offices emit `ClaimOffice` while continuing to suppress `SupportCandidateForOffice`
  - Updated stale unit/trace tests and added focused force-law coverage
- Deviations from original plan:
  - `YieldForceClaim` was not wired into active `ClaimOffice` planning because it had no concrete planner benefit on the current critical path
  - The candidate-generation rule was refined so visibly vacant force offices can emit `ClaimOffice` without requiring a separate `ForceControllerOf = None` belief first
  - The actual implementation surface was broader than originally stated because the prior architecture encoded support-law assumptions in planning snapshot/state and `ClaimOffice` satisfaction
- Verification results:
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy --workspace` passed
  - `cargo test --workspace` passed
