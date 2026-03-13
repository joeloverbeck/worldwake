# GOLDE2E-003: Journey Commitment Suspension and Resumption

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Unlikely — journey commitment state machine already exists in runtime; scope is e2e proof tightening
**Deps**: None (travel and goal-switching infrastructure exists from E07/E13)

## Problem

Multi-hop travel is tested (3b), and goal-switching during travel is tested (3c). The remaining gap is narrower than originally assumed: the runtime journey commitment state machine already exists and is unit-tested, but the golden suite does not explicitly assert the runtime-visible `Active → Suspended → Active` lifecycle while the detour resolves. Scenario 3c behaviorally proves mid-journey goal switching and eventual completion of the original food journey, but it does not directly inspect the runtime commitment snapshot.

## Report Reference

Backlog item **P-NEW-1** in `reports/golden-e2e-coverage-analysis.md` (Tier 1, composite score 5).

## Assumption Reassessment (2026-03-13)

1. `JourneyCommitmentState`, `JourneyPlanRelation`, `journey_last_progress_tick`, and `consecutive_blocked_leg_ticks` already exist in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs).
2. Suspend/reactivate behavior is already wired through plan adoption and plan completion in [`crates/worldwake-ai/src/agent_tick.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs) and guarded by unit tests there.
3. The golden harness already exposes `AgentTickDriver` via `GoldenHarness.driver`, which makes runtime journey snapshots observable from e2e tests without adding new engine hooks.
4. Existing golden scenario 3c already proves the agent leaves the origin, delays the thirst detour until critical, resolves the local need, and then either resumes the original route or is already at the destination when the detour ends.

## Architecture Check

1. The current architecture is directionally correct: journey commitment is a first-class runtime state machine, not an ad-hoc flag.
2. Suspension/resumption is already driven through normal plan selection and interrupt logic using `JourneyPlanRelation`; this is preferable to adding bespoke journey-resume code.
3. The clean extension point is better e2e observability of the existing runtime state, not more engine structure.

## Engine-First Mandate

If tightening the e2e proof reveals a real mismatch between runtime state and behavior, fix the engine directly. Do not add compatibility aliases, parallel code paths, or test-only shims. At present, no architectural rewrite is justified because the underlying runtime model is already clean and explicit.

## What to Change

### 1. Reassess and document actual runtime coverage

Confirm the runtime implementation already covers:
- `JourneyCommitmentState::{Active, Suspended}`
- `JourneyPlanRelation::{RefreshesCommitment, SuspendsCommitment, AbandonsCommitment}`
- progress tracking via `journey_last_progress_tick`
- blocked-leg patience tracking via `consecutive_blocked_leg_ticks`

### 2. Strengthen golden coverage in `golden_ai_decisions.rs`

Use the existing multi-leg travel interruption scenario rather than creating a new parallel scenario unless the current harness cannot observe the runtime state cleanly.

**Assertions**:
- Agent starts a journey toward the distant food source with an active committed destination.
- At the interrupt point, the runtime snapshot shows the original journey commitment preserved but suspended.
- After the local need resolves, the runtime snapshot returns to `Active` for the original committed destination, or the agent has already reached that destination by the time the detour completes.
- The agent eventually reaches the food source and reduces hunger.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify only if a small helper materially improves clarity)
- Engine files only if the runtime behavior and debug snapshot disagree

## Out of Scope

- Journey abandonment (agent gives up on destination entirely)
- Multi-destination journeys
- Journey commitment across save/load

## Acceptance Criteria

### Tests That Must Pass

1. Strengthened journey golden coverage passes, either by expanding `golden_goal_switching_during_multi_leg_travel` or by adding a narrowly-scoped replacement test if expansion is cleaner
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Journey state transitions are driven by real goal ranking and plan selection, not special-case code
3. The runtime snapshot preserves the original committed destination across suspension
4. The agent actually reaches the original destination after resumption, unless the detour resolves at that destination
5. Conservation holds throughout

## Post-Implementation

After implementation, update `reports/golden-e2e-coverage-analysis.md`:
- Amend Scenario 3c to note that it now explicitly inspects runtime journey suspension/reactivation
- Remove or retitle P-NEW-1 in the Part 3 backlog, since the architectural implementation already exists and only the e2e proof gap remained
- Update summary statistics only if test counts or backlog totals change

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_goal_switching_during_multi_leg_travel` — strengthened to assert runtime journey commitment establishment, suspension, progress tracking, and reactivation/arrival semantics

### Commands

1. `cargo test -p worldwake-ai golden_goal_switching_during_multi_leg_travel`
2. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-13
- **What actually changed**: Reassessed the ticket assumptions against the live code, narrowed scope from “implement a missing journey commitment state machine” to “strengthen e2e proof of the existing runtime state machine,” expanded `golden_goal_switching_during_multi_leg_travel` to assert runtime commitment establishment, suspension, progress tracking, and reactivation-or-arrival behavior, and updated `reports/golden-e2e-coverage-analysis.md` to remove the stale backlog gap.
- **Deviations from original plan**: No engine/runtime changes were needed. The architecture already had first-class journey commitment modeling in `decision_runtime.rs` and `agent_tick.rs`, plus unit coverage for suspend/reactivate and blocked-leg tracking. Creating a new parallel golden test or rewriting the runtime would have been redundant and architecturally worse than strengthening the existing 3c scenario.
- **Verification results**: `cargo test -p worldwake-ai golden_goal_switching_during_multi_leg_travel`, `cargo test -p worldwake-ai golden_`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings` all passed.
