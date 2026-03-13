# GOLDE2E-003: Journey Commitment Suspension and Resumption

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Likely — journey commitment state machine (`Active → Suspended → Active`) may be unimplemented
**Deps**: None (travel and goal-switching infrastructure exists from E07/E13)

## Problem

Multi-hop travel is tested (3b), and goal-switching during travel is tested (3c), but the journey commitment state machine is not. Scenario 3c proves an agent can switch goals mid-journey, but it does not prove that the agent resumes the original journey after addressing the interrupting need. The `JourneyCommitmentState::Active → Suspended → Active` lifecycle, `journey_last_progress_tick`, and `consecutive_blocked_leg_ticks` are unproven.

## Report Reference

Backlog item **P-NEW-1** in `reports/golden-e2e-coverage-analysis.md` (Tier 1, composite score 5).

## Assumption Reassessment (2026-03-13)

1. Journey commitment types may exist in the AI crate (`decision_runtime.rs` or related) — verify current state.
2. `AgentDecisionRuntime` tracks current goal and plan; journey commitment may be a separate or embedded state.
3. The golden harness can configure multi-hop routes with intermediate stops.
4. Goal switching during travel is proven (3c) but suspension/resumption is not.

## Architecture Check

1. Journey commitment should be a first-class state machine in the decision runtime, not an ad-hoc flag.
2. Suspension/resumption must be driven by the same goal-ranking and interrupt logic — no special-case journey code.

## Engine-First Mandate

If implementing this e2e suite reveals that the journey commitment state machine is incomplete, missing, or architecturally unsound — do NOT patch around it. Instead, design and implement a comprehensive architectural solution that makes journey commitment clean, robust, and extensible. This includes `JourneyCommitmentState` transitions, progress tracking, and blocked-leg detection. Document any engine changes in the ticket outcome.

## What to Change

### 1. Verify/implement journey commitment state machine

Ensure `JourneyCommitmentState` (or equivalent) with `Active`, `Suspended`, and related tracking fields exist and are wired into the decision runtime.

### 2. New golden test in `golden_ai_decisions.rs`

**Setup**: Agent at a distant location from food (multi-hop). Agent has carried water or sleep resource. During the journey, a second need (e.g., fatigue or thirst) escalates to require local action at an intermediate place.

**Assertions**:
- Agent starts a journey toward the distant food source (`JourneyCommitmentState::Active`).
- At an intermediate place, the second need escalates and the agent suspends the journey (`Active → Suspended`).
- Agent addresses the local need (sleep, drink, etc.) at the intermediate place.
- After resolving the local need, the agent resumes the journey toward the original destination (`Suspended → Active`).
- Agent eventually reaches the food source.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify, if journey commitment missing)
- Engine files TBD if architectural gaps are discovered

## Out of Scope

- Journey abandonment (agent gives up on destination entirely)
- Multi-destination journeys
- Journey commitment across save/load

## Acceptance Criteria

### Tests That Must Pass

1. `golden_journey_commitment_suspension_and_resumption` — agent suspends journey for local need, then resumes original destination
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Journey state transitions are driven by real goal ranking, not special-case code
3. The agent actually reaches the original destination after resumption
4. Conservation holds throughout

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Update relevant coverage sections
- Remove P-NEW-1 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_journey_commitment_suspension_and_resumption` — proves journey state machine

### Commands

1. `cargo test -p worldwake-ai golden_journey_commitment`
2. `cargo test --workspace && cargo clippy --workspace`
