# GOLDE2E-011: Put-Down Action (Inventory Management)

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None expected — put-down action exists in transport_actions
**Deps**: None

## Problem

Only pick-up is tested in the golden suite (scenarios 4, 6c). The put-down (dropping items) transport action is untested end-to-end through the real AI loop. While simpler than pick-up, a regression in the put-down handler would go undetected.

## Report Reference

Backlog item **P15** in `reports/golden-e2e-coverage-analysis.md` (Tier 3, composite score 2).

## Assumption Reassessment (2026-03-13)

1. `put_down` action definition and handler exist in `worldwake-systems/src/transport_actions.rs`.
2. The AI must have a reason to put down items — this could arise from `MoveCargo` or from load capacity constraints.
3. Transport domain coverage would go from "partial" to "full" with this test.

## Architecture Check

1. Uses existing put-down infrastructure — no new architecture.
2. The test should prove the put-down path through emergent behavior, not manual queueing.

## Engine-First Mandate

If implementing this e2e suite reveals that the put-down action handler, the AI's ability to generate put-down goals, or the transport domain's put-down path is incomplete or architecturally unsound — do NOT patch around it. Instead, design and implement a comprehensive architectural solution. Document any engine changes in the ticket outcome.

## What to Change

### 1. New golden test in `golden_production.rs`

**Setup**: Agent carrying items that need to be deposited (e.g., merchant putting stock into a container at their store, or an agent dropping items due to load constraints).

**Assertions**:
- Agent executes a put-down action through the real AI loop.
- Items transfer from agent's possession to ground/container.
- Conservation holds.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)

## Out of Scope

- Container-to-container transfers
- Inventory optimization algorithms

## Acceptance Criteria

### Tests That Must Pass

1. `golden_put_down_action` — agent drops items through the real AI loop
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent
2. Conservation holds
3. Transport domain coverage becomes full (pick-up + put-down)

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Update ActionDomain coverage: Transport → full (pick-up + put-down)
- Remove P15 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs::golden_put_down_action` — proves put-down transport path

### Commands

1. `cargo test -p worldwake-ai golden_put_down`
2. `cargo test --workspace && cargo clippy --workspace`
