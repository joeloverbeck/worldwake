# E13DECARC-017: Belief-only audit, Phase 2 gate tests, and invariant enforcement

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — minimal E13 runtime fix if verification exposes spec drift
**Deps**: E13DECARC-016

## Problem

After E13 landed, we need to verify the actual implementation against the spec and Phase 2 gate. The goal is not to duplicate existing unit coverage with broad new integration files, but to close any real verification gaps and correct implementation drift where the current runtime violates the spec.

## Assumption Reassessment (2026-03-11)

1. All prior E13 tickets are implemented enough for verification work to begin — confirmed by green workspace tests as of 2026-03-11.
2. Phase 2 gate criteria from `specs/IMPLEMENTATION-ORDER.md` are broader than this ticket originally stated: autonomous bodily upkeep, merchant restock through physical procurement, 24+ hour survival without deadlock, no infinite harvest from empty sources, concrete in-transit route occupancy, plus T12/T14/T15.
3. The E13 spec checklist contains 23 items, not 22.
4. Most E13 spec items are already covered by existing unit tests in `worldwake-ai` and `worldwake-sim`; this ticket should add targeted missing verification rather than re-implement the whole checklist in three new integration files.
5. `worldwake-systems` and `worldwake-sim` already provide the Phase 2 action families and affordance/query infrastructure needed for E13 verification.
6. `build_prototype_world()` remains a valid base fixture for focused runtime tests.
7. The current implementation legitimately uses `&mut World` at the AI orchestration boundary (`agent_tick`) for scheduler integration and persistence. The actual architectural invariant is that decision logic and planner reads flow through `&dyn BeliefView`, not that `worldwake-ai` contains zero `World` references anywhere.
8. Verification exposed one concrete implementation drift from the E13 spec: progress-barrier plans currently clear the top-level goal when the barrier step completes, but the spec requires preserving the goal across the barrier and replanning from the new belief state next tick.

## Architecture Check

1. This is primarily a verification ticket, but a minimal production fix is in scope when tests expose a direct contradiction of the accepted E13 architecture.
2. Existing targeted unit tests remain the primary acceptance mechanism; new tests should fill gaps, not duplicate them.
3. The belief-only audit must target decision/planning read paths, not scheduler/world-mutation plumbing.
4. The progress-barrier invariant is architecturally important because barrier steps are the extensibility mechanism for future materializing actions; dropping the parent goal after the barrier weakens the design and forces accidental re-discovery instead of explicit continuation.

## What to Change

### 1. Belief-only architecture audit

Add a targeted source audit that verifies planner/decision modules depend on `BeliefView` rather than direct `World` reads. Exclude the orchestration layer that must own `&mut World` to integrate with the scheduler and persist authoritative memory.

### 2. Progress-barrier regression test and fix

Add a runtime regression test for the E13 barrier invariant:

- when a `ProgressBarrier` plan finishes its final step successfully,
- the runtime must preserve the top-level goal,
- clear the finished plan,
- mark the agent dirty,
- and force replanning from the next belief state.

If the regression test fails, fix the runtime in the minimal place that restores this behavior without adding compatibility shims.

### 3. Checklist and gate coverage reconciliation

Document which E13 spec items and Phase 2 gate conditions are already covered by existing tests, and add only the missing focused tests needed to close the remaining gap(s).

## Files to Touch

- `tickets/E13DECARC-017-belief-only-audit-and-phase2-gate-tests.md`
- `crates/worldwake-ai/src/agent_tick.rs`
- `crates/worldwake-ai/tests/` (new integration test dir, only if the focused audit is best expressed there)

## Out of Scope

- Broad duplicate integration suites for already-covered E13 unit-test behavior
- E14 per-agent beliefs
- Phase 3+ gate tests
- Performance benchmarking

## Acceptance Criteria

### Tests That Must Pass

1. Existing E13-focused tests and the workspace suite remain green after reconciliation.
2. The belief-only audit passes for planning/decision read modules.
3. Progress-barrier completion preserves the parent goal and triggers follow-up replanning from refreshed beliefs.
4. No backward-compatibility aliases or duplicate decision paths are introduced.
5. `cargo test --workspace` passes.
6. `cargo clippy --workspace -- -D warnings` passes.

### Invariants

1. No direct `World` reads in planner/decision modules that should depend on `BeliefView`
2. `AgentDecisionRuntime` not in component tables
3. `PlanningState`/`PlanningSnapshot` not in component tables
4. All deterministic: BTreeMap/BTreeSet only, no floats, no HashMap/HashSet

## Test Plan

### New/Modified Tests

1. Focused belief-boundary audit test
2. Progress-barrier runtime regression test
3. Any additional focused reconciliation test only if coverage is still missing after code inspection

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace -- -D warnings`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - corrected the ticket assumptions and scope to match the codebase and E13 spec
  - added a focused belief-boundary audit in `crates/worldwake-ai/src/agent_tick.rs`
  - added a regression test for progress-barrier goal persistence in `crates/worldwake-ai/src/agent_tick.rs`
  - fixed the E13 runtime so `ProgressBarrier` completion preserves the parent goal, clears the finished plan, and marks the agent dirty for follow-up replanning
- Deviations from original plan:
  - did not add three new broad integration files because the codebase already had substantial E13 unit coverage and duplicating it would weaken test quality
  - expanded scope from \"tests only\" to include a minimal production fix because verification exposed a direct contradiction of the accepted E13 architecture
- Verification results:
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace -- -D warnings` passed
