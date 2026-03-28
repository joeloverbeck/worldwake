# S33OPPSCOGOAIDE-009: Golden tests for opportunity-scoped goal switching

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only
**Deps**: S33OPPSCOGOAIDE-002, S33OPPSCOGOAIDE-003, S33OPPSCOGOAIDE-004, S33OPPSCOGOAIDE-005, S33OPPSCOGOAIDE-006

## Problem

The spec requires golden E2E tests demonstrating that agents autonomously switch between alternative sources when one is blocked or exhausted. These tests validate the full pipeline: candidate generation → two-pass filtering → ranking → dedup → plan search → execution → source switching.

## Assumption Reassessment (2026-03-28)

1. Golden tests live in `crates/worldwake-ai/tests/` (golden test files). Existing golden tests use the test harness with `h.step_once()` and assertion helpers.
2. The spec requires two golden scenarios:
   - Agent with two known apple sources: blocks one, autonomously switches to alternative.
   - Agent exhausts search at orchard (source depleted), travels to market instead.
3. Decision tracing (`h.driver.enable_tracing()`) and action tracing (`h.enable_action_tracing()`) are available for debugging.
4. `PerceptionProfile` is required on agents that need to observe post-production output (per CLAUDE.md). Golden tests for production/acquisition scenarios must set up perception.
5. Deterministic replay companions are required per spec — each golden must have a replay round-trip test.
6. This is a golden-driven ticket. The live `GoalKind` under test is `AcquireCommodity { commodity: Apple, purpose: Consume }`. The operator surface includes: travel actions, harvest/trade actions, candidate generation for acquire goals.
7. Setup must include: topology with two distinct places (orchard + market), commodity sources at each, agent with needs that drive acquisition, and blocker/depletion mechanisms.

## Architecture Check

1. Golden tests are the correct verification surface for end-to-end pipeline behavior. Focused unit tests (in prior tickets) verify individual components; goldens verify the integrated pipeline produces correct autonomous behavior.
2. No backward-compatibility shims.

## Verification Layers

1. Source switching on block → golden E2E: agent's active action changes from orchard-directed to market-directed after block.
2. Source switching on exhaustion → golden E2E: agent replans to alternative source after search exhaustion.
3. Replay determinism → replay round-trip test: identical outcome from same seed + inputs.
4. All existing golden tests continue to pass → existing single-source scenarios are behavioral equivalents.

## What to Change

### 1. Golden: Agent switches source when one is blocked

Setup:
- Topology: 3 places (home, orchard, market) with travel edges.
- Agent at home with hunger need driving `AcquireCommodity(Apple)`.
- Apple sources at both orchard and market.
- Block the orchard opportunity (via `BlockedIntentMemory::record()` or by making orchard unreachable/depleted).
- Step ticks and assert agent plans toward market instead.

Assertions:
- Agent does NOT idle or stall.
- Agent's plan targets the unblocked source.
- Decision trace shows the blocked opportunity was filtered and the alternative was selected.

### 2. Golden: Agent exhausts orchard, travels to market

Setup:
- Topology: 3 places (home, orchard, market) with travel edges.
- Agent at home with hunger need.
- Orchard has depleted apple source (0 quantity or no resource source).
- Market has available apples (merchant with stock or resource source).
- Step ticks: agent first attempts orchard (search exhausts), then replans to market.

Assertions:
- Exhaustion is recorded for `OpportunityKey { ..., Place(orchard) }`.
- Market opportunity remains plannable.
- Agent eventually travels to market.

### 3. Replay companions

For each golden, add a deterministic replay round-trip test that re-derives the same tick sequence from the initial state + seed + inputs.

## Files to Touch

- `crates/worldwake-ai/tests/golden_opportunity_switching.rs` (new — golden test file)

## Out of Scope

- Focused unit tests for individual components (covered by S33OPPSCOGOAIDE-002 through S33OPPSCOGOAIDE-007)
- Changes to production code
- New action types or new commodity types — use existing Apple/harvest/trade infrastructure
- Performance optimization

## Acceptance Criteria

### Tests That Must Pass

1. `golden_blocked_source_switches_to_alternative` — agent with blocked orchard autonomously targets market.
2. `golden_exhausted_source_replans_to_alternative` — agent exhausts orchard search, replans to market.
3. Replay companions pass for both goldens.
4. All existing golden tests pass (behavioral equivalence for single-source scenarios).
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Agents plan from beliefs only, never world state (P12).
2. Blocking one source does not suppress planning for alternative sources (core S33 invariant).
3. Exhaustion is scoped per-opportunity, not per-desire.
4. Deterministic replay produces identical outcomes from same seed.
5. `IntentionFrame` continuity is maintained when tactic switches within the same desire.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_opportunity_switching.rs` — `golden_blocked_source_switches_to_alternative` — blocked orchard, agent targets market.
2. `crates/worldwake-ai/tests/golden_opportunity_switching.rs` — `golden_exhausted_source_replans_to_alternative` — depleted orchard, agent replans to market.
3. `crates/worldwake-ai/tests/golden_opportunity_switching.rs` — replay companions for both above.

### Commands

1. `cargo test -p worldwake-ai -- golden_opportunity`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace && cargo test --workspace`
