# SUPPLYCHAINFIX-001: Fix Multi-Agent Trade Execution Failures

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — worldwake-ai (planner budget/replanning), worldwake-sim (BestEffort failure reporting), worldwake-systems (merchant metabolism)
**Deps**: S08AIDECTRA-005 (diagnostic findings)

## Problem

The S08AIDECTRA-005 golden E2E test (Multi-Role Emergent Supply Chain) uncovered four layered production issues that prevent multi-agent trade from working end-to-end. Each issue was diagnosed using the S08 decision trace system, proving the trace system's value. The issues must be fixed in production code before the full supply chain E2E test can pass.

### Issue 1: SnapshotChanged Replanning Exhausts Plan Search Budget at Hub Nodes

In multi-agent scenarios, other agents' actions trigger `SnapshotChanged` dirty flags every tick, forcing all agents to replan from scratch. When an agent is at VillageSquare (7+ outgoing edges), the default 512-expansion budget exhausts before finding multi-hop plans (e.g., GeneralStore → VillageSquare → SouthGate → EastFieldTrail → OrchardFarm → Harvest).

**Trace evidence**: `BudgetExhausted { expansions_used: 512 }` at tick 1+ for the merchant, despite finding the same plan at tick 0 from GeneralStore.

**Root cause**: The planner discards the current plan on every `SnapshotChanged` and replans from scratch. In single-agent tests, `SnapshotChanged` rarely fires. In multi-agent scenarios, it fires every tick.

**Possible fixes**:
- Plan continuation: preserve and revalidate the existing plan when `SnapshotChanged` fires, only replan if revalidation fails
- Selective dirty: only fire `SnapshotChanged` when the changes are relevant to the agent's current plan
- Budget scaling: increase budget dynamically based on agent's position in the place graph

### Issue 2: BestEffort Action Start Failures Are Silent

When a `BestEffort` action start fails in `tick_step.rs`, the failure is silently skipped. The agent discovers on the next tick that it has no active action and replans. The trace system reveals this as a "plan found but never ACTIVE" pattern, but the actual failure reason is lost.

**Trace evidence**: Consumer selects `AcquireCommodity(Apple)` with `plans_found=1` every 4 ticks, but never enters ACTIVE state. The 4-tick cycle matches `transient_block_ticks`.

**Root cause**: `tick_step.rs` BestEffort path catches action start errors but doesn't record them. S08AIDECTRA-003 added `ActionStartFailure` recording on the `Scheduler`, but the actual precondition failure reason from the trade action handler is not captured.

**Fix**: Record the specific precondition failure reason (e.g., "counterparty busy", "not at same place", "out of stock") in the `ActionStartFailure` struct. The AI layer can then incorporate this into the trace and blocked intent memory.

### Issue 3: Merchant Goal Oscillation After Restock Return

After returning to General Store with apples, the merchant oscillates between `Relieve` and `MoveCargo` goals every tick, never settling into a stable state. Neither goal's action starts successfully, creating a perpetual plan-fail loop.

**Trace evidence**: Merchant alternates `selected=Relieve` and `selected=MoveCargo { Apple, destination: GeneralStore }` at ticks 158-172+. The merchant is already at GeneralStore holding apples, but `MoveCargo` tries to move them to GeneralStore (a no-op that fails).

**Root cause**: The `MoveCargo` candidate generation doesn't check if the agent is already at the destination. The merchant generates a `MoveCargo { Apple, GeneralStore }` goal even though the apples are already at GeneralStore. The `Relieve` goal fires because the merchant has accumulated bladder pressure during the 157-tick round trip.

**Fix**:
- `MoveCargo` candidate generation should skip when the agent is already at the destination with the commodity
- Consider suppressing non-critical needs goals (Relieve) when the agent has pending enterprise obligations

### Issue 4: Consumer Physiological Drift Breaks Co-location

The consumer's default metabolism profile increases bladder/thirst/fatigue, causing the consumer to travel to the Public Latrine or other facilities. This breaks co-location with the merchant at General Store, making trade impossible.

**Note**: This is a scenario design issue, not a production bug. The consumer's needs are realistic. In a full simulation, the consumer would return to the market after relieving itself. But within the 300-tick test window, the timing doesn't align. A production fix for Issues 1-3 would likely resolve this implicitly (faster trade execution before the consumer needs to leave).

## Assumption Reassessment (2026-03-16)

1. `tick_step.rs` BestEffort path exists and silently skips failures. Confirmed at line ~200 of tick_step.rs.
2. `AgentTickDriver` uses `PlanningBudget::default()` with `max_node_expansions: 512`. Confirmed in budget.rs.
3. `SnapshotChanged` dirty flag triggers replanning. Confirmed in agent_tick.rs process_agent logic.
4. `MoveCargo` candidate generation is in candidate_generation.rs. Confirmed at line ~727.
5. `ActionStartFailure` struct exists on `Scheduler` from S08AIDECTRA-003. Confirmed.

## Architecture Check

1. Each fix addresses a specific architectural gap (plan stability, failure observability, candidate validity, goal oscillation) without introducing hacks or workarounds.
2. No backwards-compatibility shims. Each fix is a direct improvement to the existing architecture.

## What to Change

### 1. Plan continuation on SnapshotChanged (worldwake-ai)

When `SnapshotChanged` fires, revalidate the existing plan's next step instead of discarding the entire plan. Only trigger full replanning if revalidation fails.

### 2. BestEffort failure reason recording (worldwake-sim)

Enhance `ActionStartFailure` to include the specific precondition failure reason from the action handler. Expose this through the trace system.

### 3. MoveCargo at-destination guard (worldwake-ai)

In `emit_move_cargo_goals`, skip `MoveCargo { commodity, destination }` when the agent is already at `destination` and controls the commodity there.

### 4. Trade action start validation trace (worldwake-sim + worldwake-ai)

Record the specific trade precondition that failed (counterparty busy, not co-located, out of stock, no payment) in the AI trace.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — plan continuation logic)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — MoveCargo at-destination guard)
- `crates/worldwake-sim/src/tick_step.rs` (modify — BestEffort failure reason)
- `crates/worldwake-sim/src/action_execution.rs` (modify — expose start failure reasons)

## Out of Scope

- Consumer metabolism tuning (scenario design, not production code)
- Planner budget auto-scaling (optimization, can be done later)
- Full supply chain E2E test (depends on these fixes; will be re-attempted after)

## Acceptance Criteria

### Tests That Must Pass

1. The full S02c golden supply chain test (`golden_supply_chain::test_multi_role_supply_chain`) passes with default `PlanningBudget` and realistic agent metabolism.
2. Existing suite: `cargo test -p worldwake-ai` — no regressions.
3. Existing suite: `cargo test --workspace` — no regressions.

### Invariants

1. Plan continuation must preserve determinism — same seed produces same results.
2. `ActionStartFailure` struct additions must be backwards-compatible (no serialization changes to `Scheduler`).
3. `MoveCargo` guard must not suppress valid cargo goals (only suppress when already at destination with commodity).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_supply_chain.rs` — full supply chain test with default budget and metabolism (currently fails, should pass after fixes)

### Commands

1. `cargo test -p worldwake-ai golden_supply_chain`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
