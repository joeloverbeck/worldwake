# S84SHBELOP-003: Reassess frontier-exhaustion diagnostics in PlanAttemptTrace

**Status**: COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: No — reassessment-only closeout
**Deps**: None

## Problem

The original proposal assumed frontier-exhausted planning attempts only recorded the coarse outcome and expansion count, leaving no planner-boundary explanation for why a relevant root operator never surfaced. Live reassessment found that the planner already records and renders typed root-omission diagnostics through the existing `SearchExpansionSummary` surface.

## Assumption Reassessment (2026-04-10)

1. **`PlanAttemptTrace` already carries planner-stage diagnostics through `expansion_summaries`**: `PlanAttemptTrace` stores `expansion_summaries`, and the root expansion already records `root_candidates` plus `root_omissions` in `SearchExpansionSummary`.
2. **The live planner-traceability contract is explicit**: `docs/planner-contracts.md` names root omission tracing as the authoritative boundary for omitted relevant operators and missing prerequisites. The owned symbols are `search/candidates.rs`, `search/mod.rs`, `decision_trace.rs`, and `agent_tick/planning.rs`.
3. **Typed omission reasons already exist**: `RootOperatorOmissionReason` already distinguishes `NoMatchingActionDef`, `NoAffordanceOrSynthesisPath`, `SynthesisUnsupportedGoalOp`, `SynthesisTargetDerivationFailed`, and `ConditionalBarrierUnavailable`.
4. **Rendered trace output already exposes these omissions**: `decision_trace.rs` formats `root omission: <op> -> <reason>` lines for each root omission attached to a traced expansion.
5. **Existing proof is already present**: `decision_trace` tests assert the rendered omission output, and planner/golden tests inspect `root_omissions` directly on root expansion summaries.

## Architecture Check

1. Adding a second frontier-exhaustion diagnostic carrier on `PlanAttemptTrace` would overlap the existing `root_omissions` contract instead of filling a missing planner-boundary gap.
2. No backward-compatibility shims. Because reassessment found the intended traceability already live, the correct outcome is to close this ticket without code changes.

## Verification Layers

1. Frontier-exhausted trace already records root omission provenance on the root expansion -> existing `decision_trace` tests
2. Rendered decision-trace output already includes omitted-operator diagnostics -> existing `decision_trace` summary test
3. Planner and golden consumers can already inspect `root_omissions` directly -> existing planner/golden tests
4. Single-layer ticket after reassessment: no new diagnostic fields are required on `PlanAttemptTrace`

## What to Change

### 1. Reassess the live planner-boundary diagnostic carrier

Verify whether frontier-exhausted attempts already expose omitted relevant operators and missing synthesis/affordance paths through `SearchExpansionSummary.root_omissions`. If that surface is already live and rendered, do not add parallel per-attempt counters.

### 2. Reassess the renderer and consumer proof surface

Verify both that `decision_trace.rs` renders root omissions and that downstream planner/golden tests can inspect `root_omissions` directly. If both are already true, document the live contract and close the ticket.

### 3. Close the ticket according to the live contract

If reassessment shows the planner already explains omitted root operators at the correct boundary, update the ticket to the confirmed contract instead of adding duplicate counters or reason enums.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (read-only reassessment reference)
- `crates/worldwake-ai/src/search/mod.rs` (read-only reassessment reference)
- `crates/worldwake-ai/src/search/candidates.rs` (read-only reassessment reference)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (read-only reassessment reference)

## Out of Scope

- Changing candidate generation diagnostics (`CandidateGenerationDiagnostics`)
- Adding duplicate per-attempt counters or reason enums when root-omission diagnostics already exist
- Using diagnostics to gate search (that proposal was reassessed in S84SHBELOP-002)
- Fixing the root cause of frontier exhaustion (handled by S84SHBELOP-001)

## Acceptance Criteria

### Tests That Must Pass

1. Existing `decision_trace` test proving planning summaries render root omissions still passes
2. Existing planner/golden tests that inspect root expansion omission traces still pass
3. Focused suite: `cargo test -p worldwake-ai decision_trace`

### Invariants

1. `PlanAttemptTrace` continues to surface planner diagnostics through `expansion_summaries`
2. Omitted relevant operators remain explained through `RootOperatorOmissionTrace`, not a second parallel counter/reason carrier
3. Existing trace consumers continue to read the live `root_omissions` contract without changes

## Test Plan

### New/Modified Tests

1. Existing `crates/worldwake-ai/src/decision_trace.rs` tests for rendered root omission summaries
2. Existing planner/golden tests that inspect root expansion summaries directly

### Commands

1. `cargo test -p worldwake-ai decision_trace`

## Outcome

Completed on 2026-04-10.

- Reassessed the proposed frontier-exhaustion diagnostic extension against the live planner-traceability contract and found no remaining production delta to implement.
- Confirmed `PlanAttemptTrace` already carries planner-stage diagnostics through `expansion_summaries`, whose root expansion includes both `root_candidates` and typed `root_omissions`.
- Confirmed `RootOperatorOmissionReason` already records the planner-boundary reasons this ticket was trying to add as a second carrier.
- Confirmed `decision_trace.rs` already renders omitted-root diagnostics and existing planner/golden tests already consume the same `root_omissions` surface directly.
- Closed the ticket without code changes because the proposed counters and `ZeroTargetReason` enum would duplicate the live omission-trace contract rather than fix a missing diagnostic path.

## Verification Result

- Passed `cargo test -p worldwake-ai decision_trace`
