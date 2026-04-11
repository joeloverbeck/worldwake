# S91PLAPATGOL-003: Role agent missing survival goal generation

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — ticket correction and archival only
**Deps**: None

## Problem

This ticket was drafted from the original observer report claim that Guard Theron never generated survival goals in `scenarios/cli-evaluation.ron`. Reassessment on the current branch showed that the distilled Theron scenario no longer reproduces that claim: Theron now selects and executes food/water survival work in the scenario-adjacent slice. The remaining work for this ticket is to record that mismatch honestly and archive the stale request instead of landing a false regression test.

## Assumption Reassessment (2026-04-11)

1. **Harness infrastructure exists**: `GoldenHarness` at `crates/worldwake-ai/tests/golden_harness/mod.rs:1089`, `step_once()` at line 1196, `seed_actor_beliefs()` at line 158. All confirmed present.
2. **Goal and profile types exist**: `GoalKind::AcquireCommodity` at `crates/worldwake-core/src/goal.rs:22`, `GoalKind::ConsumeOwnedCommodity` at line 19. `PatrolProfile` at `crates/worldwake-core/src/patrol.rs`. `PatrolRoute` at `crates/worldwake-core/src/patrol.rs`. `CombatProfile` at `crates/worldwake-core/src/combat.rs`. `DeadAt` / `DeathCause::NeedDeprivation` at `crates/worldwake-core/src/combat.rs:60`. All confirmed present.
3. **Shared file boundary**: `crates/worldwake-ai/tests/golden_planner_pathology.rs` now exists from S91PLAPATGOL-001. This ticket adds an independent test function in that shared file.
4. **Planner proof boundary**: per `docs/planner-contracts.md`, this ticket's honest planner boundary is the selected-goal trace (`planning.selection.selected_goal()`), not a weaker downstream proxy such as missing committed eat/drink actions. The test should prove that survival goals never win selection under the live Theron boundary.
5. **Scenario isolation correction**: the ticket's original 2-place proxy was too lossy for the user requirement and the motivating observer report. The live owning surface is `crates/worldwake-ai/tests/golden_planner_pathology.rs` using the existing `cli-evaluation` place graph slice (`Dusty Trail`, `Thornwall Village`, `Eldergrove Forest`, `Hearthstone Inn`, `Golden Fields`) plus Guard Theron's exact scenario profile values and the real village substrate relevant to the pathology (`Village Well`, Bread, Grain). This keeps the proof anchored to `scenarios/cli-evaluation.ron` without importing that file directly.
6. **Remote-knowledge confounder**: the observer report says Theron's failed plans already included `AcquireCommodity{Water}` budget exhaustions, so the test must seed the remote Thornwall place/resource beliefs needed for that planner-visible water-search path. Otherwise "no survival goals selected" would be confounded by simple ignorance rather than the reported role-goal dominance.
7. **Focused falsification on current branch**: a Theron-only `cli-evaluation` slice with the live place graph, exact Theron profile values, local Dusty Trail beliefs, and remote Thornwall visibility did **not** reproduce the reported failure. On the current branch Theron selected survival goals and consumed Water/Bread in that distilled setup. The remaining honest proof likely depends on a broader multi-agent `cli-evaluation` reconstruction rather than a single-agent slice.

## Architecture Check

1. The current architecture is cleaner than preserving the stale observer narrative in a new golden. A false golden would encode behavior the current branch no longer exhibits and would become a regression against a prior bugfix rather than a guardrail for a live invariant.
2. The strongest honest proof on the current branch is the failed reproduction attempt itself: under a scenario-adjacent Theron slice, decision traces show food/water goal selection and action commits. That disproves the ticket's original contract more directly than weakening the assertions.
3. No backwards-compatibility shims introduced.

## Verification Layers

1. Original claim under audit -> decision trace in the attempted Theron reproduction (`selection.selected_goal()` and selected actions)
2. Stale observer narrative disproved -> focused golden test attempt on the current branch shows `AcquireCommodity(Water)`, `ConsumeOwnedCommodity(Water)`, and `ConsumeOwnedCommodity(Bread)` selections in the scenario-adjacent slice
3. Additional layer mapping not applicable because no production or golden behavior change landed

## What to Change

### 1. Correct the ticket scope

Update the ticket so it no longer claims a live missing-survival-goal pathology when the current branch disproves that claim in the scenario-adjacent Theron slice.

### 2. Archive the ticket

Mark the ticket completed and archive it with an accurate outcome instead of landing a false golden or keeping a stale pending ticket active.

## Files to Touch

- `tickets/S91PLAPATGOL-003.md` (modify, then move to archive)

## Out of Scope

- Reconstructing the full multi-agent `cli-evaluation.ron` world by hand inside a new golden
- Reopening the underlying pathology without a fresh observer run proving it still exists
- Tests for budget exhaustion or 0-step loop (S91PLAPATGOL-001, -002)
- Any production or golden code changes

## Acceptance Criteria

### Tests That Must Pass

1. The ticket records that the original pathology is not reproducible on the current branch in the scenario-adjacent Theron slice
2. No new golden is added that would falsely encode the stale observer narrative
3. The ticket is archived with an accurate closeout trail

### Invariants

1. No production code modified
2. No golden behavior changed

## Test Plan

### New/Modified Tests

1. `None — no code or golden behavior changes were required beyond correcting and archiving this stale ticket.`

### Commands

1. `cargo test -p worldwake-ai role_agent_generates_survival_goals_under_critical_needs`

## Outcome

- **Completion date**: 2026-04-11
- **What actually changed**: Reassessed the ticket against the current branch, corrected the scenario-isolation assumptions, and attempted a focused Theron reproduction against the live `cli-evaluation` place graph and profile boundary.
- **Deviations from original plan**: No golden was landed. The focused reproduction disproved the original claim: in the scenario-adjacent Theron slice, the current branch selects and executes survival work instead of suppressing it. Keeping the ticket active would require a materially broader multi-agent reconstruction, which this ticket did not honestly own.
- **Verification results**: The focused command `cargo test -p worldwake-ai role_agent_generates_survival_goals_under_critical_needs` was used as a reassessment probe. It failed because the asserted pathology was not reproducible on the current branch, and the decision trace showed survival-goal selections including `AcquireCommodity(Water)`, `ConsumeOwnedCommodity(Water)`, and `ConsumeOwnedCommodity(Bread)`.
