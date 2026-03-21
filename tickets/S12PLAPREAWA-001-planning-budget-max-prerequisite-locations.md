# S12PLAPREAWA-001: Add `max_prerequisite_locations` to `PlanningBudget`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new field on `PlanningBudget`
**Deps**: None

## Problem

The planner has no budget knob to cap how many prerequisite locations feed into the A* heuristic. Without a cap, an agent who believes a commodity exists at many distant locations would dilute the heuristic, degrading search performance. This ticket adds the `max_prerequisite_locations` field that downstream tickets (S12PLAPREAWA-002) consume.

## Assumption Reassessment (2026-03-21)

1. `PlanningBudget` is defined at `crates/worldwake-ai/src/budget.rs` with fields: `max_candidates_to_plan`, `max_plan_depth`, `snapshot_travel_horizon`, `max_node_expansions`, `beam_width`, `switch_margin_permille`, `transient_block_ticks`, `structural_block_ticks` — confirmed.
2. `PlanningBudget` derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` — confirmed.
3. `PlanningBudget::default()` exists and provides production defaults — confirmed.
4. No existing `max_prerequisite_locations` or prerequisite-related field exists — confirmed via grep.
5. Single-layer ticket adding one struct field — no AI regression, ordering, or heuristic-removal concerns apply.

## Architecture Check

1. Adding a new field to an existing configuration struct is the simplest approach. The alternative — hardcoding the cap — violates the project's profile-driven parameter principle (no magic numbers).
2. No backwards-compatibility shims. The `Default` impl gains one new field with a sensible default.

## Verification Layers

1. `max_prerequisite_locations` default value is `3` → focused unit test on `PlanningBudget::default()`
2. Serialization round-trip preserves the new field → existing `PlanningBudget` serde tests (if any), or manual verification via `cargo test -p worldwake-ai`
3. Single-layer ticket — additional layer mapping not applicable.

## What to Change

### 1. Add field to `PlanningBudget` struct

In `crates/worldwake-ai/src/budget.rs`, add:

```rust
/// Maximum number of prerequisite locations to include in the A* heuristic.
/// Caps `prerequisite_places()` output to the N closest by travel distance.
/// Prevents heuristic dilution when the agent believes a commodity exists
/// at many distant locations. Default: 3.
pub max_prerequisite_locations: u8,
```

### 2. Update `Default` impl

Set `max_prerequisite_locations: 3` in the `Default` implementation.

## Files to Touch

- `crates/worldwake-ai/src/budget.rs` (modify)

## Out of Scope

- `prerequisite_places()` method (S12PLAPREAWA-002)
- `combined_relevant_places()` function (S12PLAPREAWA-003)
- `search_plan()` signature changes (S12PLAPREAWA-003)
- `agent_tick.rs` call site changes (S12PLAPREAWA-004)
- Decision trace changes (S12PLAPREAWA-005)
- Any test files beyond what's needed to keep existing tests compiling

## Acceptance Criteria

### Tests That Must Pass

1. `PlanningBudget::default().max_prerequisite_locations == 3`
2. Existing suite: `cargo test -p worldwake-ai`
3. Existing suite: `cargo clippy --workspace`

### Invariants

1. All existing `PlanningBudget` construction sites must compile (adding the new field to any struct-literal construction)
2. `PlanningBudget` remains `Serialize + Deserialize` — no breaking serde changes
3. No magic numbers — the cap is profile-driven via `PlanningBudget`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/budget.rs` (or test module) — assert `default().max_prerequisite_locations == 3`

### Commands

1. `cargo test -p worldwake-ai budget`
2. `cargo clippy --workspace && cargo test --workspace`
