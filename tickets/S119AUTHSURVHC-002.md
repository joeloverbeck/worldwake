# S119AUTHSURVHC-002: Classify scattered survival hunger overrun after authored-contract retrofit

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Maybe — scenario contract correction or separate production follow-up, depending on reassessment
**Deps**: tickets/S119AUTHSURVHC-001.md, specs/S120-survival-critical-window-forensics.md

## Problem

After `S119AUTHSURVHC-001` removed the stale file-local survival-envelope constants, `golden_survival_scattered::all_agents_survive_1440_ticks` still failed on a single real authored-contract check:

- `Agent B hunger exceeded authored critical pm(820) for 506 consecutive ticks (max allowed: 400)`

This ticket must classify that failure honestly. It is now either:

1. a wrong authored `survival_health_contract` value in `survival-scattered.ron`, or
2. a real remaining runtime/calibration contradiction in the scattered survival scenario.

## Assumption Reassessment (2026-04-18)

1. The old proof-surface drift is already fixed by `S119AUTHSURVHC-001`; `golden_survival_scattered.rs` now reads authored `DriveThresholds` and scenario-owned `survival_health_contract` values instead of file-local `pm(750)` constants.
2. Focused live evidence:
   - `cargo test -p worldwake-ai --test golden_survival_scattered all_agents_survive_1440_ticks -- --ignored --exact --nocapture`
   - failure: `Agent B hunger exceeded authored critical pm(820) for 506 consecutive ticks (max allowed: 400)`
3. Other scattered envelope checks are green under the same authored contract:
   - `all_agents_perform_survival_actions`
   - `isolated_agent_reaches_food_source`
   - `no_budget_exhaustion_on_survival_goals`
   - `no_stuck_idle_windows_with_elevated_needs`
4. Shared boundary under audit: authored scenario survival truth versus live scattered survival behavior. The same fact no longer has duplicate test-local transport paths; this ticket must now classify whether the authored bound or the live behavior is wrong.
5. Intended invariant before trusting the current scattered failure: the scenario should prove adversarial but healthy scattered survival without silently normalizing long authored-critical hunger runs.
6. Live `GoalKind` surface implicated by the failure remains the food-recovery family (`AcquireCommodity(SelfConsume)` / `ConsumeOwnedCommodity`) in the scattered topology. This ticket must not generalize to wash or water behavior without evidence.
7. The strongest available proof should stay at the goldens plus the nearest lower-layer trace/provenance that explains Agent B's hunger run. If current traces are insufficient to classify the cause, the ticket may depend on the S120 forensics substrate or create a narrower production follow-up instead of guessing.
8. Adjacent contradiction classification:
   - contested dirtiness overrun after S119 retrofit: separate contract-model gap, not owned here
   - any newly exposed lower-layer hunger-planning/runtime bug during reassessment: separate follow-up ticket, not to be silently absorbed into scenario retuning

## Architecture Check

1. Explicit classification is cleaner than changing `survival-scattered.ron` by feel, because it preserves one authored truth path and avoids calibrating against unexplained behavior.
2. This ticket keeps the outcome binary and honest: either the scenario contract changes, or a runtime contradiction gets its own owner.

## Verification Layers

1. Existing scattered survival failure remains reproducible -> `golden_survival_scattered all_agents_survive_1440_ticks -- --ignored --exact`
2. Agent B hunger-run cause is explained at the strongest available lower layer -> decision trace and/or focused runtime/provenance coverage
3. Final classification is proven cleanly:
   - scenario contract correction -> updated scattered golden passes
   - runtime contradiction -> new follow-up ticket created with the lower-layer evidence cited

## What to Change

### 1. Reproduce and explain the hunger overrun

Use the existing golden and the strongest available lower-layer traces/provenance to explain Agent B's long authored-critical hunger run.

### 2. Classify the cause honestly

Either:

- update `survival-scattered.ron` if the authored `max_authored_critical_run_ticks` is wrong, or
- create/update a separate production ticket if the live behavior is wrong

### 3. Update adjacent tickets

If the outcome blocks S119 or S116 further, update their dependency text explicitly.

## Files to Touch

- `tickets/S119AUTHSURVHC-001.md` (modify if the dependency story changes)
- `tickets/S116DRIESCSUS-006.md` (modify if the dependency story changes)
- `scenarios/survival-scattered.ron` (modify only if the authored bound is proven wrong)
- `<new follow-up ticket>` (new, only if a runtime contradiction is found)

## Out of Scope

- Implementing the contested per-need contract model
- Changing survival-baseline
- Blindly retuning `survival-scattered.ron` without classification evidence

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_survival_scattered all_agents_survive_1440_ticks -- --ignored --exact`
2. Any focused lower-layer trace/runtime proof used to justify the final classification
3. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings` if code changes land

### Invariants

1. The scattered scenario's survival-health truth remains scenario-authored, not inferred from a stale golden-local constant.
2. If the live behavior is wrong, it gets a separate explicit owner instead of being normalized into the scenario contract.

## Test Plan

### New/Modified Tests

1. `None initially — classification-first ticket. Add focused lower-layer proof only if the final classification requires a code change or a new regression test.`

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_scattered all_agents_survive_1440_ticks -- --ignored --exact --nocapture`
2. `cargo test -p worldwake-ai --test golden_survival_scattered -- --ignored`
3. `cargo clippy --workspace --all-targets -- -D warnings`
