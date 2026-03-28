# S33OPPSCOGOAIDE-009: Golden tests for opportunity-scoped source switching

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Usually none; test harness/support updates allowed if needed for observability
**Deps**: S33OPPSCOGOAIDE-004, S33OPPSCOGOAIDE-005, S33OPPSCOGOAIDE-006, S33OPPSCOGOAIDE-010

## Problem

The remaining S33 end-to-end proof is not generic "opportunity behavior"; it is autonomous source switching after one concrete opportunity becomes blocked or exhausted while another remains valid. Adjacent goldens already landed during `S33OPPSCOGOAIDE-002`, but the integrated proof for the final architecture is still missing.

## Assumption Reassessment (2026-03-28)

1. Golden tests live in `crates/worldwake-ai/tests/` and use the AI harness with deterministic seeds.
2. Some nearby coverage already exists from archived `S33OPPSCOGOAIDE-002` work. This ticket should not duplicate those scenarios; it should prove the final multi-source switching invariant once `004`, `005`, `006`, and `010` land.
3. Decision tracing and action tracing remain the right debugging tools for these goldens and should be used for assertions when candidate absence/suppression is the core claim.
4. `PerceptionProfile` still matters anywhere the agent must observe produced or stocked output before replanning.
5. The live desire under test is still `AcquireCommodity` for a concrete commodity with at least two lawful sources.

## Architecture Check

1. Golden tests are the correct verification surface because the final risk is cross-layer: candidate emission, blocking/exhaustion memory, admission ordering, planning scope, and execution must all cooperate.
2. No backward-compatibility shims.

## Verification Layers

1. Blocked-source switching -> golden E2E.
2. Exhausted-source switching -> golden E2E.
3. Replay determinism -> replay companion tests if the harness pattern still requires them.
4. Existing AI golden suite still passes.

## What to Change

### 1. Golden: blocked source switches to alternative

Setup:
- Topology: 3 places (home, orchard, market) with travel edges.
- Agent at home with hunger need driving `AcquireCommodity(Apple)`.
- Apple sources at both orchard and market.
- Block the orchard opportunity using the same authoritative/runtime path the live architecture uses for blocker persistence.
- Step ticks and assert agent plans toward market instead.

Assertions:
- Agent does NOT idle or stall.
- Agent targets the unblocked source rather than stalling or returning to the blocked one.
- Decision trace proves the blocked opportunity was suppressed and the alternative remained live.

### 2. Golden: exhausted source falls through to alternative

Setup:
- Topology: 3 places (home, orchard, market) with travel edges.
- Agent at home with hunger need.
- Orchard has depleted apple source (0 quantity or no resource source).
- Market has available apples (merchant with stock or resource source).
- Step ticks so the higher-ranked or initially chosen source exhausts, then assert fallthrough to the alternative source.

Assertions:
- Exhaustion is recorded for the specific exhausted opportunity, not the whole desire.
- The alternative source remains plannable and is actually chosen.
- The trace/assertions exclude lawful competing explanations such as "the agent never knew about the second source."

### 3. Replay companions

For each golden, add a deterministic replay round-trip test that re-derives the same tick sequence from the initial state + seed + inputs.

## Files to Touch

- `crates/worldwake-ai/tests/` (modify or add the focused golden file that best fits the existing suite structure)

## Out of Scope

- Focused unit tests for individual components
- Production-code behavior changes unless the golden exposes a genuine bug that must be fixed in the same implementation sequence
- New action types or new commodity types — use existing Apple/harvest/trade infrastructure
- Performance optimization

## Acceptance Criteria

### Tests That Must Pass

1. Blocked-source golden proves the blocked opportunity is suppressed while the alternative remains actionable.
2. Exhausted-source golden proves exhaustion is opportunity-scoped and fallthrough occurs.
3. Replay companions pass if the existing golden harness expects them.
4. All existing AI golden tests pass.
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Agents plan from beliefs only, never direct world-state inspection.
2. Blocking one source does not suppress planning for alternative sources (core S33 invariant).
3. Exhaustion is scoped per-opportunity, not per-desire.
4. Planning uses candidate-local evidence scope once `S33OPPSCOGOAIDE-010` lands.
5. Deterministic replay produces identical outcomes from the same seed.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/` — `golden_blocked_source_switches_to_alternative`
2. `crates/worldwake-ai/tests/` — `golden_exhausted_source_switches_to_alternative`
3. Replay companions for both scenarios if the suite keeps paired replay coverage

### Commands

1. `cargo test -p worldwake-ai -- golden`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
4. `cargo test --workspace`
