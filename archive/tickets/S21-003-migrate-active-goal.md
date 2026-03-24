# S21-003: Migrate active goal from runtime to ActiveGoal component

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI crate runtime reads/writes of current_goal migrated to World/WorldTxn
**Deps**: S21-001

## Problem

`AgentDecisionRuntime.current_goal: Option<GoalKey>` is causally relevant (affects goal switching margins and interrupt thresholds) but lost on save/load. After S21-001 defines the `ActiveGoal` component, this ticket migrates all reads and writes to use the authoritative component.

## Assumption Reassessment (2026-03-23)

1. `current_goal` field is at `decision_runtime.rs` line 68.
2. Read/write sites confirmed via grep `current_goal` in AI crate (10 files):
   - `agent_tick/mod.rs` — goal adoption/clearing orchestration
   - `agent_tick/planning.rs` — plan search uses current goal for switching margin
   - `agent_tick/active_action.rs` — goal identity check during active action
   - `agent_tick/journey.rs` — goal comparison for journey relevance
   - `agent_tick/observation.rs` — goal comparison for dirty detection
   - `agent_tick/tests.rs` — test fixtures set current_goal
   - `plan_selection.rs` — goal switching compares current vs candidate
   - `failure_handling.rs` — goal clearing on plan failure
   - `interrupts.rs` — interrupt threshold depends on current goal
   - `decision_runtime.rs` — field definition and Default impl
3. The spec adds `adopted_at: Tick` to `ActiveGoal` — this is a NEW field not currently on `AgentDecisionRuntime`. It must be set when the goal is adopted (the tick at which `set_component_active_goal` is called).
4. This ticket is independent of S21-002 (journey) and S21-004 (facility). They can be done in parallel after S21-001.

## Architecture Check

1. `ActiveGoal` adds `adopted_at: Tick` which currently has no equivalent in `AgentDecisionRuntime`. This is new capability enabled by promotion (supports future commitment stability calculations). For now, it is set at adoption time and read-only.
2. No backward-compatibility shims — the `current_goal` field is removed, not deprecated.

## Verification Layers

1. All `current_goal` reads now go through `world.get_component_active_goal(agent)` → grep for `runtime.current_goal` must return zero hits
2. All `current_goal` writes go through `txn.set_component_active_goal(agent, ...)` or `txn.clear_component_active_goal(agent)` → grep for `runtime.current_goal =` must return zero hits
3. No behavioral change → all golden tests pass with unchanged hashes
4. Single-layer: unit tests + golden E2E already cover goal switching, adoption, and clearing

## What to Change

### 1. Migrate reads in agent_tick/ modules

In `mod.rs`, `planning.rs`, `active_action.rs`, `journey.rs`, `observation.rs`:
- Replace `runtime.current_goal` reads with `world.get_component_active_goal(agent)` (or WorldTxn read)
- Extract `goal_key` from the `ActiveGoal` component where needed

### 2. Migrate writes in agent_tick/ modules

- Goal adoption: `txn.set_component_active_goal(agent, ActiveGoal { goal_key, adopted_at: current_tick })`
- Goal clearing: `txn.clear_component_active_goal(agent)`

### 3. Migrate reads/writes in plan_selection.rs

- `compare_goal_switch()` and related logic reads the component instead of `runtime.current_goal`
- Update test fixtures to set `ActiveGoal` component

### 4. Migrate reads/writes in interrupts.rs and test fixtures in failure_handling.rs

- `failure_handling.rs`: production code does NOT read/write `current_goal` — only test fixtures and assertions reference it. Update test fixtures to set `ActiveGoal` component and assertions to read from it.
- `interrupts.rs`: interrupt threshold comparisons read from the local `Option<ActiveGoal>` (passed as parameter following S21-002 persist-at-end pattern)
- Update test fixtures in both files

### 5. Add persist_active_goal in agent_tick/execution.rs

Following the S21-002 pattern (`persist_journey_commitment`): read component at start of `process_agent` into local `original_active_goal` / `current_active_goal`, mutate locally throughout the tick, persist at end via `persist_active_goal(world, event_log, agent, tick, original, current)`.

### 6. Remove current_goal from AgentDecisionRuntime

Delete the field from struct definition (line 68). Update `Default` impl.

### 7. Update unit tests

Tests that set `runtime.current_goal = Some(goal_key)` must instead set the `ActiveGoal` component on a test World or pass `Option<ActiveGoal>` to functions that now accept it as a parameter.

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — remove `current_goal` field)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — goal adoption/clearing via txn)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — read component for plan search)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify — read component)
- `crates/worldwake-ai/src/agent_tick/journey.rs` (modify — goal comparison via component)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — dirty detection via component)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — update fixtures)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — read component + update tests)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — add `persist_active_goal` function)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — update test fixtures only; production code does not reference `current_goal`)
- `crates/worldwake-ai/src/interrupts.rs` (modify — read from parameter + update tests)

## Out of Scope

- Journey commitment migration (S21-002)
- Facility queue intents migration (S21-004)
- New save/load golden tests (S21-005)
- Any changes to `worldwake-core` component definitions (S21-001)
- Any changes to `worldwake-sim` or `worldwake-systems`
- The `adopted_at` field is set but not actively used in decision logic yet — future specs (S22 intention frames) will consume it

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — all unit tests and golden tests pass
2. `cargo test -p worldwake-ai --test golden_determinism` — replay hashes unchanged
3. `cargo test -p worldwake-ai --test golden_ai_decisions` — all AI decision goldens pass
4. `cargo clippy --workspace` — no new warnings

### Invariants

1. Zero occurrences of `runtime.current_goal` in the codebase (grep verification)
2. `AgentDecisionRuntime` no longer has a `current_goal` field
3. All goal writes go through `WorldTxn`, producing `ComponentDelta` entries in the event log
4. `adopted_at` is set to the tick at which the goal was adopted
5. No golden test behavioral changes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` tests — updated to not reference `current_goal`
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — updated fixtures to set `ActiveGoal` component
3. `crates/worldwake-ai/src/plan_selection.rs` tests — set component instead of runtime field
4. `crates/worldwake-ai/src/failure_handling.rs` tests — set component instead of runtime field
5. `crates/worldwake-ai/src/interrupts.rs` tests — set component instead of runtime field

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-24
- **What changed**: Removed `current_goal: Option<GoalKey>` from `AgentDecisionRuntime`. All reads/writes now go through the `ActiveGoal` component on the World ECS, following the S21-002 persist-at-end pattern: read from world at start of `process_agent`, track as mutable local, persist via `persist_active_goal` at end. `adopted_at: Tick` is set at every goal adoption site.
- **Files modified**: `decision_runtime.rs`, `agent_tick/mod.rs`, `agent_tick/planning.rs`, `agent_tick/active_action.rs`, `agent_tick/execution.rs`, `agent_tick/observation.rs`, `agent_tick/journey.rs`, `interrupts.rs`, `plan_selection.rs`, `agent_tick/tests.rs`, `failure_handling.rs` (tests only).
- **Deviations from ticket**: (1) `failure_handling.rs` production code does not read/write `current_goal` — only test fixtures were updated, not production code. Ticket section 4 was corrected before implementation. (2) `agent_tick/execution.rs` was added to the files list for the new `persist_active_goal` function. (3) Line number for field was 68, not 78 as originally stated.
- **Verification**: `cargo test --workspace` all pass, `cargo clippy --workspace` clean, `grep runtime.current_goal` returns zero hits, all golden tests pass with unchanged hashes.
