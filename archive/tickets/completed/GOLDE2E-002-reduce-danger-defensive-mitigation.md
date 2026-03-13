# GOLDE2E-002: ReduceDanger Defensive Mitigation Under Active Threat

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Unclear — current code already contains `ReduceDanger` generation, danger pressure, and defensive planner ops; implementation should only change engine code if the new e2e proof exposes a real gap
**Deps**: None (combat infrastructure exists from E12, AI from E13)

## Problem

Living combat (offensive `EngageHostile`) is proven in scenario 7c, but that scenario does not prove the defender-side `ReduceDanger` path end-to-end. The codebase already models defensive danger pressure and registers defensive planner ops, but the golden suite does not currently prove that an agent under active attack autonomously enters a `ReduceDanger` path through the real AI loop. This leaves a coverage gap around defensive combat behavior and its concrete runtime consequences.

## Report Reference

Backlog item **P7** in `reports/golden-e2e-coverage-analysis.md` (Tier 1, composite score 5).

## Assumption Reassessment (2026-03-13)

1. `GoalKind::ReduceDanger` already exists in `worldwake-core/src/goal.rs`.
2. `ReduceDanger` candidate generation already exists in `worldwake-ai/src/candidate_generation.rs` and emits at `danger.high()` or above when supported by concrete evidence.
3. Defensive planner operations already exist: `planner_ops.rs` maps combat `defend`, travel remains available to `ReduceDanger`, and `goal_model.rs` treats `ReduceDanger` as a first-class goal.
4. The pressure module (`worldwake-ai/src/pressure.rs`) already derives danger from concrete visible hostiles, current attackers, wounds, and incapacitation.
5. The current gap is therefore not missing goal plumbing by default; it is missing golden proof that the defender-side path actually manifests in the live runtime.

## Architecture Check

1. Offensive and defensive combat paths should remain symmetric in the sense that both emerge from concrete local state, planner ops, and registered actions rather than scripted branches.
2. No shims; if the golden reveals a real engine gap, fix the underlying architecture rather than special-casing the test.
3. Do not assume a successful defensive action must immediately lower computed danger pressure. The current danger model is driven by concrete attackers and wounds, so `defend` may mitigate incoming harm without instantly satisfying the goal.

## Engine-First Mandate

If implementing this e2e suite reveals that defender-side combat behavior is incomplete or architecturally unsound — do NOT patch around it. Instead, design and implement the smallest clean architectural correction that keeps defensive mitigation a first-class, extensible part of the combat/AI contract. Document any engine changes in the ticket outcome.

## What to Change

### 1. Re-verify the existing `ReduceDanger` contract

Confirm that the existing `ReduceDanger` path is still the intended architectural contract: emitted from concrete danger pressure and resolved through real planner ops.

### 2. Add missing e2e proof in the combat golden suite

Add a new golden scenario in the combat suite that proves a defender under active attack autonomously enters a `ReduceDanger` mitigation path without manual queueing.

### 3. Only implement engine changes if the golden exposes a real gap

If the new golden fails because the live runtime does not reach a legitimate mitigation path, fix the underlying architecture instead of weakening the test.

### 4. New golden test in `golden_combat.rs`

**Setup**: Two agents co-located. Attacker has hostile relation and strong combat profile. Defender has no hostile relation (purely reactive). Attacker starts combat through the real AI loop.

**Assertions**:
- Attacker initiates combat via `EngageHostile`.
- Defender comes under active attack pressure, danger permille rises.
- Defender autonomously enters a real mitigation path associated with `ReduceDanger` without manual action queueing.
- The observed mitigation is concrete world behavior, such as `defend` becoming active, a defending stance being applied, or successful relocation away from the local threat.
- The test should assert concrete mitigation behavior rather than assuming immediate danger-pressure satisfaction.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify only if the golden exposes a real gap)
- `crates/worldwake-ai/src/planner_ops.rs` (modify only if the golden exposes a real gap)
- Engine files TBD if architectural gaps are discovered

## Out of Scope

- Offensive combat (already proven in 7c)
- Death cascade (already proven in 8)
- Multi-agent group combat coordination

## Acceptance Criteria

### Tests That Must Pass

1. `golden_reduce_danger_defensive_mitigation` — defender autonomously responds to active attack with a real mitigation path
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing for the defender
2. `ReduceDanger` is generated from concrete danger pressure, not from a magic flag
3. Conservation holds throughout the combat sequence
4. Defensive response uses a real registered action or movement path, not a stub

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Update GoalKind coverage: `ReduceDanger` → Yes
- Remove P7 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs::golden_reduce_danger_defensive_mitigation` — proves defender-side mitigation under active threat

### Commands

1. `cargo test -p worldwake-ai golden_reduce_danger`
2. `cargo test --workspace && cargo clippy --workspace`

## Outcome

Originally planned:
- Add a golden proof for defender-side `ReduceDanger`
- Only change engine code if the proof exposed a real gap

Actually changed:
- Added `golden_reduce_danger_defensive_mitigation` in `crates/worldwake-ai/tests/golden_combat.rs`
- Exposed the real engine gap: the AI read phase was not using a runtime-aware belief view, so `current_attackers_of()` was invisible during live planning
- Split offensive hostility from defensive threat response more cleanly:
  - `EngageHostile` generation now uses explicit outgoing hostility targets
  - `ReduceDanger` no longer resolves through `Attack`
  - danger pressure now promotes wounded agents facing visible hostiles
- Added unit coverage for the exposed invariant and planner suppression behavior
- Updated `reports/golden-e2e-coverage-analysis.md` to mark `ReduceDanger` as covered and remove P7 from the backlog
