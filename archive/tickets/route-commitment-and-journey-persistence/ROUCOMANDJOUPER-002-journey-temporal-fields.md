# ROUCOMANDJOUPER-002: Journey Temporal Fields on AgentDecisionRuntime

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new fields on AgentDecisionRuntime
**Deps**: None

## Problem

The decision runtime has no memory that "this journey is already in progress toward a concrete destination." Multi-hop journeys restart goal selection from scratch at each intermediate arrival, with no temporal tracking of journey progress or blockage duration.

## Assumption Reassessment (2026-03-13)

1. `AgentDecisionRuntime` is defined in `crates/worldwake-ai/src/decision_runtime.rs` with `#[derive(Clone, Debug, Default, Eq, PartialEq)]` — confirmed.
2. `AgentDecisionRuntime` is NOT registered as a component (enforced by test `agent_decision_runtime_is_not_registered_as_a_component`) — confirmed.
3. `AgentDecisionRuntime` is transient runtime state, not serialized through save/load — confirmed by absence from component schema and by the spec's explicit statement.
4. `Tick` is defined in `worldwake-core::ids` as a newtype wrapper — confirmed.
5. The existing `Default` derive on `AgentDecisionRuntime` sets all `Option` fields to `None` and numeric fields to 0 — confirmed.

## Architecture Check

1. Adding temporal fields to the existing struct fits the current runtime architecture: `AgentDecisionRuntime` already holds transient per-agent controller state (`current_plan`, `current_step_index`, observation snapshots, and materialization bindings). Journey fields belong in that transient runtime layer rather than authoritative world state.
2. These fields are semantically different from the existing observation-cache fields (`last_effective_place`, `last_needs`, etc.). They represent commitment lifecycle state, not world-observation snapshots, so follow-up tickets should keep lifecycle mutations centralized instead of scattering direct field writes.
3. No backwards-compatibility aliasing or shims. The new fields default to `None`/0, so existing construction sites continue to compile while later tickets wire the lifecycle behavior.

## What to Change

### 1. Add three fields to `AgentDecisionRuntime`

```rust
// Inside AgentDecisionRuntime:
pub journey_established_at: Option<Tick>,
pub journey_last_progress_tick: Option<Tick>,
pub consecutive_blocked_leg_ticks: u32,
```

- `journey_established_at`: set when selecting a travel-led plan (a plan whose remaining steps include Travel ops). `Some(tick)` means "this agent committed to a journey at this tick."
- `journey_last_progress_tick`: updated to the current tick when the agent completes a travel leg and arrives at an intermediate place.
- `consecutive_blocked_leg_ticks`: incremented each tick the agent's next travel leg cannot start; reset to 0 on successful leg completion.

All three default to `None`/0 via `Default`, requiring no changes to existing construction sites.

### 2. Add helper methods for journey state queries

On `AgentDecisionRuntime`, add:

```rust
/// Returns true if the agent has an active journey — i.e., the current plan
/// has remaining Travel steps at or beyond `current_step_index` and
/// `journey_established_at` is `Some`.
pub fn has_active_journey(&self) -> bool { ... }

/// Returns the number of remaining Travel steps in the current plan
/// from `current_step_index` onward.
pub fn remaining_travel_steps(&self) -> usize { ... }

/// Clears all journey temporal fields to their default values.
pub fn clear_journey_fields(&mut self) { ... }
```

These helpers centralize journey detection logic so callers don't re-derive it.

### 3. Update the defaults test

Update `agent_decision_runtime_defaults_to_empty_clean_state` to assert that the new fields are `None`/0.

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — add fields, helpers, update test)

## Out of Scope

- `TravelDispositionProfile` component (ticket 001)
- Goal switching margin override (ticket 003)
- When/where to set `journey_established_at` (ticket 004/005)
- When/where to clear journey fields (ticket 006)
- Debug surface exposure (ticket 007)
- Any changes to `worldwake-core`, `worldwake-sim`, or `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. `AgentDecisionRuntime::default()` has `journey_established_at == None`, `journey_last_progress_tick == None`, `consecutive_blocked_leg_ticks == 0`.
2. `has_active_journey()` returns `false` when `journey_established_at` is `None`.
3. `has_active_journey()` returns `false` when `journey_established_at` is `Some` but current plan has no remaining Travel steps.
4. `has_active_journey()` returns `true` when `journey_established_at` is `Some` and current plan has remaining Travel steps at or beyond `current_step_index`.
5. `remaining_travel_steps()` returns 0 when plan is `None`.
6. `remaining_travel_steps()` correctly counts Travel-typed steps from `current_step_index` onward.
7. `clear_journey_fields()` resets all three fields to defaults.
8. Existing suite: `cargo test -p worldwake-ai`
9. Existing suite: `cargo clippy --workspace`

### Invariants

1. `AgentDecisionRuntime` remains NOT registered as a component (existing test enforces this).
2. `AgentDecisionRuntime` remains transient — not serialized through save/load.
3. All existing code compiles without modification because new fields have `Default` values.
4. No new crate dependencies introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_runtime.rs` — update `agent_decision_runtime_defaults_to_empty_clean_state` to cover new fields
2. `crates/worldwake-ai/src/decision_runtime.rs` — new test: `has_active_journey_requires_established_tick_and_travel_steps`
3. `crates/worldwake-ai/src/decision_runtime.rs` — new test: `remaining_travel_steps_counts_from_current_index`
4. `crates/worldwake-ai/src/decision_runtime.rs` — new test: `clear_journey_fields_resets_all_temporal_state`

### Commands

1. `cargo test -p worldwake-ai decision_runtime`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`

## Outcome

- Outcome amended: 2026-03-13

- Completion date: 2026-03-13
- What actually changed:
  - Added `journey_established_at`, `journey_last_progress_tick`, and `consecutive_blocked_leg_ticks` to `AgentDecisionRuntime`.
  - Added `has_active_journey()`, `remaining_travel_steps()`, and `clear_journey_fields()` on `AgentDecisionRuntime`.
  - Strengthened `decision_runtime.rs` unit coverage for defaults, active-journey detection, travel-step counting, and field clearing.
  - Corrected the ticket itself before implementation so journey detection consistently counts travel steps from `current_step_index` onward.
  - Follow-up architectural refinement moved plan-derived route inspection onto `PlannedPlan` (`remaining_travel_steps_from`, `has_remaining_travel_steps_from`, `terminal_travel_destination`) so runtime helpers delegate instead of re-scanning raw step lists directly.
- Deviations from original plan:
  - The original implementation landed in `crates/worldwake-ai/src/decision_runtime.rs`, then a same-day refinement added plan-level read helpers in `crates/worldwake-ai/src/planner_ops.rs` to keep route/destination derivation attached to the plan model.
  - The ticket wording was tightened to distinguish transient commitment lifecycle state from the existing observation-cache fields.
- Verification results:
  - `cargo test -p worldwake-ai decision_runtime` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace` ✅
