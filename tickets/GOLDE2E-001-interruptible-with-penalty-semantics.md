# GOLDE2E-001: InterruptibleWithPenalty Action Semantics

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Possible — if `InterruptibleWithPenalty` semantics are incomplete or missing in the interrupt evaluation path
**Deps**: None (all interrupt evaluation infrastructure exists from E13)

## Problem

No golden e2e test exercises the `InterruptibleWithPenalty` action category. The interrupt evaluation path distinguishes between freely interruptible actions and penalty-interruptible actions, but this distinction has never been proven end-to-end through the real AI loop. A regression in the penalty-interrupt threshold would go undetected.

## Report Reference

Backlog item **P-NEW-2** in `reports/golden-e2e-coverage-analysis.md` (Tier 1, composite score 6).

## Assumption Reassessment (2026-03-13)

1. `InterruptDecision` enum and `evaluate_interrupt()` exist in `worldwake-ai/src/interrupts.rs`.
2. Craft actions are registered as `InterruptibleWithPenalty` in the action def registry.
3. The interrupt evaluation path uses priority class comparisons — only `Critical`-class needs should interrupt penalty actions.
4. The golden harness (`golden_harness/mod.rs`) can set up craft scenarios with workstation + recipe + inputs.

## Architecture Check

1. This test proves the real interrupt evaluation path end-to-end — no manual `InterruptDecision` injection.
2. No backwards-compatibility shims needed; this is a pure coverage addition.

## Engine-First Mandate

If implementing this e2e suite reveals that the engine's interrupt evaluation for penalty actions is incomplete, missing, or architecturally unsound — do NOT patch around it. Instead, design and implement a comprehensive architectural solution that makes the penalty-interrupt contract clean, robust, and extensible. Document any engine changes in the ticket outcome.

## What to Change

### 1. New golden test in `golden_ai_decisions.rs`

**Setup**: Agent at a workstation with recipe inputs and a craft action available. Agent has a medium-level need that rises during crafting (not critical). Agent also has a second need channel driven to critical during the craft.

**Assertions**:
- Agent starts a craft action (penalty-interruptible).
- Medium-priority need rises but does NOT interrupt the craft (`InterruptDecision::Continue`).
- Critical-priority need rises and DOES interrupt the craft (`InterruptDecision::Interrupt`).
- After interruption, the agent addresses the critical need.

### 2. Harness helpers (if needed)

Add any setup helpers to `golden_harness/mod.rs` for configuring craft scenarios with tunable metabolism rates on two need channels.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- Engine files TBD if architectural gaps are discovered

## Out of Scope

- Testing `NonInterruptible` actions (no such actions exist yet)
- Changing interrupt threshold constants (unless the architecture requires it)
- Adding new action categories

## Acceptance Criteria

### Tests That Must Pass

1. `golden_interruptible_with_penalty_semantics` — agent crafts, medium need does NOT interrupt, critical need DOES interrupt
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. Only `Critical`-class priority interrupts penalty actions
2. Conservation holds throughout the craft + interrupt + consume sequence
3. All behavior is emergent — no manual action queueing

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Update the Coverage Matrix in Part 2
- Remove P-NEW-2 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_interruptible_with_penalty_semantics` — proves penalty-interrupt threshold contract

### Commands

1. `cargo test -p worldwake-ai golden_interruptible_with_penalty`
2. `cargo test --workspace && cargo clippy --workspace`
