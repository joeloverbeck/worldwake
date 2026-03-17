# SUPPLYCHAINFIX-001: Fix Multi-Agent Trade Execution Failures

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — worldwake-ai (plan continuation on SnapshotChanged, AI consumption of action start failures)
**Deps**: S08AIDECTRA-005 (diagnostic findings)

## Problem

The S08AIDECTRA-005 golden E2E test (Multi-Role Emergent Supply Chain) uncovered layered production issues that prevent multi-agent trade from working end-to-end with default `PlanningBudget`. Each issue was diagnosed using the S08 decision trace system, proving the trace system's value.

### Issue 1: SnapshotChanged Replanning Exhausts Plan Search Budget at Hub Nodes

In multi-agent scenarios, other agents' actions trigger `SnapshotChanged` dirty flags every tick, forcing all agents to replan from scratch. When an agent is at VillageSquare (7+ outgoing edges), the default 512-expansion budget exhausts before finding multi-hop plans (e.g., GeneralStore → VillageSquare → SouthGate → EastFieldTrail → OrchardFarm → Harvest).

**Trace evidence**: `BudgetExhausted { expansions_used: 512 }` at tick 1+ for the merchant, despite finding the same plan at tick 0 from GeneralStore.

**Root cause**: The planner discards the current plan on every `SnapshotChanged` and replans from scratch (`agent_tick.rs:1044` — `if runtime.dirty { build_candidate_plans(...) }`). In single-agent tests, `SnapshotChanged` rarely fires. In multi-agent scenarios, it fires every tick.

**Fix**: Plan continuation — when `SnapshotChanged` is the ONLY dirty reason AND the agent has a current plan, revalidate the existing plan's next step instead of discarding. Only trigger full replanning if revalidation fails.

### Issue 2: AI Layer Does Not Consume BestEffort Action Start Failures

**Note (corrected 2026-03-16)**: The original ticket stated BestEffort failures were "silent." This is inaccurate. S08AIDECTRA-003 already added `ActionStartFailure` recording in `tick_step.rs:232-243` with `reason: format!("{err:?}")`. The failures ARE recorded on `Scheduler`.

The actual gap: the AI layer (`worldwake-ai`) never calls `drain_action_start_failures()` — grep across the AI crate returns zero matches. So failure reasons are captured but never fed back to the decision trace or blocked intent memory.

**Fix**: In `agent_tick.rs`, drain action start failures from the `Scheduler` at the start of each agent's decision tick and incorporate them into the decision trace.

### Issue 3: MoveCargo at-destination guard — ALREADY RESOLVED

**Note (corrected 2026-03-16)**: The original ticket stated `emit_move_cargo_goals` doesn't check if the agent is at the destination. This is incorrect. `candidate_generation.rs:740-742` already has the guard:
```rust
if current_place == destination {
    return;
}
```
No changes needed. The merchant oscillation described in the original issue may have been caused by the SnapshotChanged replanning issue (Issue 1) rather than missing MoveCargo guards.

### Issue 4: Consumer Physiological Drift Breaks Co-location

**Note**: This is a scenario design issue, not a production bug. A production fix for Issue 1 (faster plan continuation → faster trade execution) should resolve this implicitly by completing trade before the consumer needs to leave.

## Assumption Reassessment (2026-03-16)

1. ~~`tick_step.rs` BestEffort path silently skips failures~~ — CORRECTED: failures ARE recorded via `record_action_start_failure` (S08AIDECTRA-003). The AI layer doesn't consume them.
2. `AgentTickDriver` uses `PlanningBudget::default()` with `max_node_expansions: 512`. Confirmed in budget.rs.
3. `SnapshotChanged` dirty flag triggers full replanning (discards current plan). Confirmed in agent_tick.rs:1044.
4. ~~`MoveCargo` candidate generation missing at-destination guard~~ — CORRECTED: guard exists at candidate_generation.rs:740.
5. `ActionStartFailure` struct exists on `Scheduler` with `drain_action_start_failures()` API. Confirmed in scheduler.rs.
6. AI crate never calls `drain_action_start_failures()`. Confirmed via grep (zero matches in worldwake-ai).

## What to Change

### 1. Plan continuation on SnapshotChanged (worldwake-ai)

In `agent_tick.rs`, when `runtime.dirty` is true but the ONLY dirty reason is `SnapshotChanged` AND the agent has a current plan with a valid next step: revalidate that step instead of running the full `build_candidate_plans` pipeline. If revalidation passes, keep the existing plan and clear dirty. If revalidation fails, fall through to full replanning.

### 2. AI consumption of action start failures (worldwake-ai)

In `agent_tick.rs`, at the start of `process_agent`, drain the agent's action start failures from `Scheduler` and include them in the decision trace. This makes BestEffort failure reasons visible in the trace system.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — plan continuation logic + action start failure consumption)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify — add full combined test with default budget)

## Out of Scope

- Consumer metabolism tuning (scenario design, not production code)
- Planner budget auto-scaling (optimization, can be done later)
- MoveCargo guard changes (already exists)
- `tick_step.rs` or `action_execution.rs` changes (BestEffort recording already works)

## Acceptance Criteria

### Tests That Must Pass

1. Existing segment tests pass with default `PlanningBudget` (no `max_node_expansions: 1024` workaround).
2. A full combined supply chain test (merchant restock → consumer trade in one simulation) passes with default budget.
3. Existing suite: `cargo test -p worldwake-ai` — no regressions.
4. Existing suite: `cargo test --workspace` — no regressions.

### Invariants

1. Plan continuation must preserve determinism — same seed produces same results.
2. No serialization changes to `Scheduler` (the `ActionStartFailure` struct is unchanged).
3. Plan continuation only activates when `SnapshotChanged` is the sole dirty reason and an existing plan is available.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_supply_chain.rs` — modify existing segment tests to use default budget; add full combined supply chain test
2. `crates/worldwake-ai/src/agent_tick.rs` — unit test for plan continuation logic

### Commands

1. `cargo test -p worldwake-ai golden_supply_chain`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

**Completion date**: 2026-03-17

### What changed

1. **Plan continuation on SnapshotChanged** (`agent_tick.rs`): When `SnapshotChanged` is the only dirty reason, the agent has a current plan, AND the current goal is still the top-ranked candidate, the planner revalidates the existing plan's next step instead of running the full plan search. Falls through to full replan if a higher-priority goal has emerged or the step is invalid. Added `is_snapshot_changed_only()` helper. Modified both `plan_and_validate_next_step` and `plan_and_validate_next_step_traced` with a new `dirty_reasons` parameter.

2. **AI consumption of action start failures** (`agent_tick.rs`, `decision_trace.rs`): `process_agent` now reads action start failures for the agent from the `Scheduler` and includes them in `PlanningPipelineTrace.action_start_failures`. Added `ActionStartFailureSummary` struct and `plan_continued: bool` field to the trace model.

3. **Golden supply chain tests** (`golden_supply_chain.rs`): Removed the `max_node_expansions: 1024` workaround — segment tests now pass with default 512 budget. Added a full combined 3-agent supply chain test (`#[ignore]` — see deviations).

### Deviations from original plan

- **Acceptance criterion 2 (full combined test) partially met**: The full 3-agent supply chain test is written but `#[ignore]`d. The combined scenario exposes a deeper architectural gap: the GOAP plan search is spatially blind (Dijkstra/UCS with no heuristic), causing budget exhaustion for return-trip planning from remote locations through hub nodes. This is NOT a plan-continuation issue — it's a search algorithm issue. Tracked in `specs/S09-travel-aware-plan-search.md` (Travel-Aware Plan Search with A* heuristic and goal-directed travel pruning). Ticket 5 of S09 specifically requires enabling these ignored tests.

- **Issue 3 (MoveCargo guard) was already resolved**: The guard at `candidate_generation.rs:740-742` pre-existed the ticket. No code changes needed.

- **Issue 2 reframed**: BestEffort failures were already recorded (S08AIDECTRA-003). The actual gap was AI layer consumption — the AI crate never called `drain_action_start_failures()`. Fixed by reading (not draining) failures in `process_agent` and surfacing them in the decision trace.

- **Plan continuation is priority-aware**: Initial implementation skipped priority re-evaluation entirely, which broke `golden_goal_switching_during_multi_leg_travel` (agent wouldn't switch to critical thirst between travel legs). Revised to check that the current goal is still the top-ranked candidate before continuing the plan.

### Verification results

- `cargo clippy --workspace` — clean
- `cargo test --workspace` — 1,772 passed, 0 failed, 2 ignored
- All golden tests pass including `golden_goal_switching_during_multi_leg_travel`
- Segment supply chain tests pass with default `PlanningBudget` (512 expansions)
- Deterministic replay preserved for all segment tests
