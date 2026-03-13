# GOLDE2E-002: ReduceDanger Defensive Mitigation Under Active Threat

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Likely — `ReduceDanger` goal generation and defensive planner ops may be incomplete
**Deps**: None (combat infrastructure exists from E12, AI from E13)

## Problem

Living combat (offensive `EngageHostile`) is proven in scenario 7c, but the defensive counterpart `ReduceDanger` has never been exercised end-to-end. An agent under active attack should autonomously emit `ReduceDanger` and execute a defensive mitigation (defend action, repositioning, or flight) through the real AI loop. This gap leaves the entire defensive combat path untested.

## Report Reference

Backlog item **P7** in `reports/golden-e2e-coverage-analysis.md` (Tier 1, composite score 5).

## Assumption Reassessment (2026-03-13)

1. `GoalKind::ReduceDanger` exists in `worldwake-core/src/goal.rs`.
2. `ReduceDanger` candidate generation should exist in `worldwake-ai/src/candidate_generation.rs` — verify it triggers from active threat / danger pressure.
3. Defensive actions (defend, flee/reposition) must be registered in the action def registry with planner op semantics.
4. The pressure module (`worldwake-ai/src/pressure.rs`) derives danger permille from wounds and active threats.

## Architecture Check

1. Offensive and defensive combat paths should be symmetric — both driven by concrete relations/state, not special cases.
2. No shims; if `ReduceDanger` candidate generation or defensive planner ops are missing, they must be architecturally added as first-class constructs.

## Engine-First Mandate

If implementing this e2e suite reveals that `ReduceDanger` goal generation, defensive planner operations, or the danger-pressure pipeline are incomplete or architecturally unsound — do NOT patch around it. Instead, design and implement a comprehensive architectural solution that makes the defensive combat contract clean, robust, and extensible. Document any engine changes in the ticket outcome.

## What to Change

### 1. Verify/implement `ReduceDanger` candidate generation

Ensure `candidate_generation.rs` emits `ReduceDanger` when an agent is under active attack and danger pressure exceeds the high-or-above threshold.

### 2. Verify/implement defensive planner ops

Ensure the planner can resolve `ReduceDanger` through a real defensive action (defend, reposition, or flight) registered in the action def registry.

### 3. New golden test in `golden_ai_decisions.rs`

**Setup**: Two agents co-located. Attacker has hostile relation and strong combat profile. Defender has no hostile relation (purely reactive). Attacker starts combat through the real AI loop.

**Assertions**:
- Attacker initiates combat via `EngageHostile`.
- Defender comes under active attack pressure, danger permille rises.
- Defender emits `ReduceDanger` and executes a defensive mitigation without manual action queueing.
- Danger pressure decreases after the defensive action.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify, if `ReduceDanger` generation missing)
- `crates/worldwake-ai/src/planner_ops.rs` (modify, if defensive ops missing)
- Engine files TBD if architectural gaps are discovered

## Out of Scope

- Offensive combat (already proven in 7c)
- Death cascade (already proven in 8)
- Multi-agent group combat coordination

## Acceptance Criteria

### Tests That Must Pass

1. `golden_reduce_danger_defensive_mitigation` — defender autonomously responds to active attack with defensive action
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing for the defender
2. `ReduceDanger` is generated from concrete danger pressure, not from a magic flag
3. Conservation holds throughout the combat sequence
4. Defensive response uses a real registered action, not a stub

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Update GoalKind coverage: `ReduceDanger` → Yes
- Remove P7 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_reduce_danger_defensive_mitigation` — proves defensive combat path

### Commands

1. `cargo test -p worldwake-ai golden_reduce_danger`
2. `cargo test --workspace && cargo clippy --workspace`
