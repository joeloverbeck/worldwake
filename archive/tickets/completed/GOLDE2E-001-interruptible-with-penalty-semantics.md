# GOLDE2E-001: Explicit Golden Coverage for InterruptibleWithPenalty Thresholds

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Unlikely
**Deps**: None

## Problem

`InterruptibleWithPenalty` is already implemented and unit-tested in `worldwake-ai/src/interrupts.rs`, and the golden suite already exercises a penalty action end-to-end through multi-leg travel. The actual remaining gap is narrower: no golden test explicitly proves that a penalty action continues while the challenger remains subcritical and only replans once the challenger reaches the `Critical` priority class.

The previous version of this ticket assumed the gap was specifically about craft actions and missing engine semantics. That assumption is stale relative to the current codebase.

## Report Reference

Backlog item **P-NEW-2** in `reports/golden-e2e-coverage-analysis.md` (Tier 1, composite score 6).

## Assumption Reassessment (2026-03-13)

1. `InterruptDecision` and `evaluate_interrupt()` exist in `crates/worldwake-ai/src/interrupts.rs`, with direct unit coverage for penalty interrupts.
2. `InterruptibleWithPenalty` is used by multiple real actions, including travel, craft, transport, wash, and heal. This is not a craft-only contract.
3. `evaluate_interrupt()` does more than the previous ticket claimed:
   - penalty actions replan immediately when the current plan becomes invalid
   - penalty actions only replan for `Critical` reactive challengers (`CriticalSurvival` or `CriticalDanger`)
   - `High` danger does not interrupt penalty actions
4. `golden_goal_switching_during_multi_leg_travel` already exercises a real penalty action (`travel`) through the live AI loop, but it does not explicitly assert the subcritical `Continue` portion of the contract.
5. No harness expansion is currently required. The existing golden harness already supports the needed travel + carried-water setup.

## Scope Correction

Do not add a craft-specific golden path unless the travel-based coverage proves insufficient. The cleaner architectural move is to strengthen the existing travel-based golden scenario so it explicitly proves the threshold contract already implemented by the engine.

Why this is better than the prior plan:

1. It tests an existing penalty action already central to the journey architecture rather than introducing extra craft-specific setup.
2. It reuses an established emergent scenario instead of widening the harness surface area.
3. It aligns the golden suite with the actual architectural seam: interrupt threshold behavior, not crafting.

## Architecture Check

1. The current architecture is sounder than the previous ticket assumed. Penalty semantics live in one place (`evaluate_interrupt()`), and action defs opt into them via `Interruptibility::InterruptibleWithPenalty`.
2. A stronger golden assertion on the travel scenario is more robust and extensible than adding a one-off craft scenario. It validates the shared interrupt contract against the real runtime without coupling the ticket to a specific production domain.
3. No backward-compatibility layer or aliasing is warranted. If the golden test exposes a real semantic gap, fix the engine contract directly.

## What to Change

### 1. Strengthen `golden_goal_switching_during_multi_leg_travel`

Use the existing travel scenario in `crates/worldwake-ai/tests/golden_ai_decisions.rs` and add explicit assertions that:

- the agent starts a penalty-interruptible travel action toward distant food
- thirst rises through subcritical bands while the agent remains on the travel path
- the agent does not consume carried water before thirst reaches `Critical`
- once thirst reaches `Critical`, the running travel plan is interrupted and the agent switches to the local water action
- after the drink detour resolves, the agent resumes or completes the original hunger-driven journey

### 2. Harness helpers

Do not add harness helpers unless the existing test cannot read the needed runtime state cleanly.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `reports/golden-e2e-coverage-analysis.md` (modify)
- Engine files only if the strengthened golden test exposes a real semantic defect

## Out of Scope

- Adding a craft-specific golden scenario unless travel-based coverage proves insufficient
- Changing penalty interrupt thresholds unless the current behavior is architecturally wrong
- Adding new action categories
- Adding backward-compatibility shims

## Acceptance Criteria

### Tests That Must Pass

1. Strengthened `golden_goal_switching_during_multi_leg_travel`
2. Relevant targeted run in `worldwake-ai`
3. `cargo test -p worldwake-ai golden_`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

### Invariants

1. Penalty actions continue through subcritical challenger pressure
2. Penalty actions replan for `Critical` reactive challengers
3. Conservation holds throughout the travel + drink + resumed journey sequence
4. All behavior remains emergent; no manual action queueing

## Post-Implementation

After implementation:

1. Update `reports/golden-e2e-coverage-analysis.md` so Scenario 3c explicitly documents penalty-interrupt threshold coverage.
2. Remove **P-NEW-2** from the backlog.
3. Update summary counts/ordering in the report to reflect that this gap is now closed without adding a new domain-specific scenario.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_goal_switching_during_multi_leg_travel` — strengthened to prove subcritical continue + critical interrupt semantics for a real penalty action

### Commands

1. `cargo test -p worldwake-ai golden_goal_switching_during_multi_leg_travel`
2. `cargo test -p worldwake-ai golden_`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-13
- **What actually changed**:
  - Reassessed the ticket against the current code and corrected its stale assumptions.
  - Strengthened `golden_goal_switching_during_multi_leg_travel` to prove the real `InterruptibleWithPenalty` contract end to end via travel:
    - travel continues through subcritical thirst pressure
    - carried water is not consumed before the critical threshold
    - the plan replans only after thirst reaches critical
    - the agent resumes or completes the original hunger-driven journey after the detour
  - Updated `reports/golden-e2e-coverage-analysis.md` to record the strengthened coverage and remove backlog item `P-NEW-2`.
- **Deviations from original plan**:
  - Did not add a craft-specific golden scenario.
  - Did not expand the golden harness.
  - Reused the existing travel-based scenario because it exercises the shared penalty-interrupt architecture more directly and with less test-only setup.
- **Verification results**:
  - `cargo test -p worldwake-ai golden_goal_switching_during_multi_leg_travel`
  - `cargo test -p worldwake-ai golden_`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
