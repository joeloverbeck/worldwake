# S116DRIESCSUS-012: Recover survival-baseline hunger stability after stale local opportunity repair

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` survival planning/runtime and possibly scenario-neutral calibration surfaces
**Deps**: tickets/S116DRIESCSUS-011.md, archive/tickets/S116DRIESCSUS-009.md, archive/tickets/S116DRIESCSUS-010.md

## Problem

`S116DRIESCSUS-011` removes the stale exact local `pick_up` contradiction, but `golden_survival_baseline::all_agents_survive_1440_ticks` still fails. The old repeated `PreconditionFailed("TargetAtActorPlace(0)")` start-failure loop is gone, `all_agents_perform_survival_actions` passes, and `no_stuck_idle_windows_with_elevated_needs` passes, so the residual issue is no longer the stale exact-opportunity bug. A separate ticket must now own the remaining survival-efficiency/calibration contradiction explicitly.

Any solution must align with [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): preserve belief-only planning and locality, keep one canonical local/remote information path, avoid scenario-only workaround logic, and do not reintroduce stale believed local cargo as a lawful planning carrier.

## Assumption Reassessment (2026-04-18)

1. The motivating proof surface is the ignored golden baseline assertion after the `011` local-authority repair:
   - `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
   - initial live result after `011`'s fix: `Agent A hunger exceeded pm(750) for 243 consecutive ticks`
2. The original `011` bug is no longer the live blocker. A temporary targeted probe over `golden_survival_baseline` produced no remaining `PreconditionFailed("TargetAtActorPlace(0)")` start failures for Agent A after the `PerAgentBeliefView` local-authority repair in [per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs).
3. Adjacent baseline assertions now distinguish the residual scope:
   - `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_perform_survival_actions -- --ignored --exact` passes
   - `cargo test -p worldwake-ai --test golden_survival_baseline no_stuck_idle_windows_with_elevated_needs -- --ignored --exact` passes
   - therefore the remaining issue is survival efficiency / long hunger-critical exposure, not the old no-progress idle loop
4. The live goal family under audit remains self-care commodity survival:
   - `GoalKind::AcquireCommodity { purpose: SelfConsume, .. }`
   - `GoalKind::ConsumeOwnedCommodity { .. }`
   - plus whatever lawful sibling travel/harvest/consume ordering or ranking path now dominates the baseline after `011`
5. Exact shared boundary under audit for this follow-up is no longer stale exact target visibility. It is whichever lower-layer branch now causes Agent A to spend too long above the golden's sustained-critical threshold despite lawful actions continuing to occur. Reassessment for implementation must identify whether the active contradiction sits in ranking, plan selection, search progression, action-duration/cadence, or authored baseline proof drift before changing code.
6. Reassessment on 2026-04-18 found a factual proof-surface mismatch: `golden_survival_baseline.rs` was using a hardcoded `pm(750)` sustained-critical bound, but the authored per-agent `drive_thresholds` in `scenarios/survival-baseline.ron` set higher critical values for hunger/thirst/dirtiness (for example Agent A thirst critical `pm(820)`, hunger critical `pm(850)`). That hardcoded golden constant was not the same contract as the live planner/escalation `Critical` band.
7. The FOUNDATIONS-aligned correction is to make the golden invariant profile-driven. The baseline should measure sustained critical exposure against each agent's authored `DriveThresholds`, not a scenario-external magic number.
8. This ticket is mixed-layer. Focused trace coverage alone is not enough because the owned failure is defined by a long-run survival envelope in `golden_survival_baseline`; however, any fix should first be proven at the strongest lower layer available before using the golden as final confirmation.
9. `S116DRIESCSUS-006` depends on this ticket once `011` is narrowed honestly. The remaining baseline calibration failure is now a separate production contradiction, not part of the stale exact-opportunity repair.

## Architecture Check

1. Splitting this residual failure out of `011` is cleaner than silently broadening an exact-opportunity bug ticket into a general survival-calibration/debug ticket. The observed symptom changed after the local-authority repair, so the repo should track that changed ownership explicitly.
2. The clean solution is to identify whether the remaining failure is a real lower-layer survival contradiction or a proof-surface mismatch. Hardcoded golden thresholds that disagree with authored `DriveThresholds` violate FOUNDATIONS' profile-driven/no-magic-number rules just as surely as scenario-only resource patching would.

## Verification Layers

1. Lower-layer cause of Agent A's prolonged hunger-critical run is identified and fixed at the canonical layer -> focused `worldwake-ai` unit/runtime/trace coverage at the exact boundary found during reassessment
2. The fix does not reintroduce stale local opportunity or belief-barrier regressions -> existing `S116DRIESCSUS-011` focused regressions remain green
3. Survival envelope recovery is real end-to-end -> `golden_survival_baseline::all_agents_survive_1440_ticks`
4. If the lower-layer fix changes broader survival dynamics, adjacent existing baseline assertions remain green -> `all_agents_perform_survival_actions` and `no_stuck_idle_windows_with_elevated_needs`

## What to Change

### 1. Correct the baseline proof surface

Replace the hardcoded sustained-critical golden threshold with a profile-driven check derived from each authored agent's own `DriveThresholds`.

### 2. Reassess any remaining residual survival-efficiency cause

After the proof surface is corrected, re-run the ignored baseline goldens and only continue into lower-layer AI/runtime repairs if a real failure remains.

### 3. Reconfirm baseline stability

Use the existing ignored baseline goldens as the end-to-end proof surface once the lower-layer fix is in place.

## Files to Touch

- `crates/worldwake-ai/tests/golden_survival_baseline.rs` (modify to use authored `DriveThresholds` rather than a hardcoded sustained-critical constant)
- `crates/worldwake-ai/src/...` exact files under the confirmed residual boundary (modify only if a real post-correction failure remains)
- `crates/worldwake-ai/tests/...` focused regression proving the newly identified cause (modify/add only if a real post-correction failure remains)

## Out of Scope

- Reopening the stale exact-opportunity / stale current-place local visibility repair from `S116DRIESCSUS-011`
- Adding extra wash basins or other scenario-only resource patches to `survival-baseline.ron`
- Preserving the old `pm(750)` sustained-critical golden threshold if it contradicts the authored per-agent profile contract
- Contested-scenario bound tightening from `S116DRIESCSUS-007`

## Acceptance Criteria

### Tests That Must Pass

1. `golden_survival_baseline::all_agents_survive_1440_ticks` measures sustained critical runs against each agent's authored `DriveThresholds`, not a hardcoded external threshold
2. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
3. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_perform_survival_actions -- --ignored --exact`
4. `cargo test -p worldwake-ai --test golden_survival_baseline no_stuck_idle_windows_with_elevated_needs -- --ignored --exact`
5. If a real failure remains after the golden-threshold correction, new focused coverage proves the newly identified lower-layer cause is repaired

### Invariants

1. Current-place planner/entity visibility remains authoritative-local after `S116DRIESCSUS-011`; this ticket must not reintroduce stale believed local cargo as a lawful local opportunity source
2. The shipped fix remains scenario-neutral and FOUNDATIONS-aligned: belief-backed remote, authoritative local, no duplicate information-path shims

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/...` focused regression at the exact residual lower-layer boundary discovered during reassessment
2. `crates/worldwake-ai/tests/golden_survival_baseline.rs` — existing ignored baseline assertions remain the end-to-end proof surface

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
3. `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_perform_survival_actions -- --ignored --exact`
4. `cargo test -p worldwake-ai --test golden_survival_baseline no_stuck_idle_windows_with_elevated_needs -- --ignored --exact`
5. `cargo test -p worldwake-ai --lib`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Outcome amended: 2026-04-18

Completed on 2026-04-18.

- Corrected `crates/worldwake-ai/tests/golden_survival_baseline.rs` to measure sustained critical runs against each agent's authored `DriveThresholds` instead of the stale hardcoded `pm(750)` magic number.
- Reassessed the surviving real failure after that proof correction and identified the canonical lower-layer cause: `GoalKind::Sleep` could exhaust bounded search without producing a progress barrier, so authored-critical fatigue could stay pinned while the planner only accepted fully satisfied low-band sleep plans.
- Landed the `Sleep` progress-barrier repair in `crates/worldwake-ai/src/goal_dispatch_decl.rs` by making `PlannerOpKind::Sleep` a direct progress barrier op for the `Sleep` goal family.
- Added focused regression coverage in `crates/worldwake-ai/src/search/tests.rs` proving that critical local sleep now returns a one-step `ProgressBarrier` plan, and in `crates/worldwake-ai/src/goal_model.rs` proving the goal-model barrier semantics directly.
- Kept the `S116DRIESCSUS-011` stale-local-opportunity repair intact while adding the narrower sleep frontier retry handling in `crates/worldwake-ai/src/agent_tick/planning.rs`, so sleep frontier exhaustion no longer becomes indefinite suppression even when a single search pass fails.

## Deviations

- The broad command `cargo test -p worldwake-ai --lib` remains red in the current worktree, but the remaining failures are outside the shipped `012` seam:
  - `agent_tick::planning::tests::same_goal_planning_trace_records_candidate_cap_stop_reason`
  - `agent_tick::planning::tests::same_goal_ranked_opportunities_are_attempted_in_order`
  - `agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order`
  - `agent_tick::tests::unseen_seller_relocation_preserves_stale_acquisition_belief`
- Those failures live in same-goal acquisition ordering / stale seller-belief surfaces rather than the authored-threshold or sleep progress-barrier path repaired here. They were recorded honestly instead of being folded into this ticket without reassessment.
- The temporary long-running late-window debug/reproducer probes used during reassessment were removed rather than kept as brittle ignored tests after the shipped fix changed the live trace shape.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib local_critical_sleep_returns_progress_barrier_after_one_step`
- Passed `cargo test -p worldwake-ai --lib progress_barrier_semantics_move_with_goal_model`
- Passed `cargo test -p worldwake-ai --test golden_survival_baseline all_agents_survive_1440_ticks -- --ignored --exact`
- Passed `cargo test -p worldwake-ai --test golden_survival_baseline -- --ignored`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Ran `cargo test -p worldwake-ai --lib`; it failed only in the four broader acquisition/same-goal tests listed in Deviations
