# S56PEREXP-002: Add `attention_cost` field to `ActionDef`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — ActionDef struct, all action registration sites, test helpers
**Deps**: S56PEREXP-001

## Problem

S56 requires perception modulation based on the active action's attention demand. Currently `ActionDef` has no way to express how much an action occupies perceptual bandwidth. Per FOUNDATIONS P8 (actions declare their own occupancy costs), this belongs on the action definition, not as a hardcoded per-domain constant.

## Assumption Reassessment (2026-04-06)

1. `ActionDef` at `crates/worldwake-sim/src/action_def.rs:10` has 14 fields. Adding `attention_cost: Permille` makes 15. Derives: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.
2. Workspace sweep via `rg -n 'ActionDef \\{' crates` shows `ActionDef` struct literals in `worldwake-systems`, `worldwake-sim`, and `worldwake-ai` test/planning helpers. The ticket's listed files cover the current live surface except `planner_ops.rs`, which uses `ActionDefRegistry` but does not construct `ActionDef` literals directly.
3. ~4 `sample_action_def` test helpers in `crates/worldwake-sim/src/` (action_def.rs, action_def_registry.rs, affordance.rs, affordance_query.rs) must include the new field.
4. `ActionDef` struct literals also exist in `crates/worldwake-ai/src/` (goal_model.rs, planning_state.rs, decision_trace.rs, plan_revalidation.rs) — test/planning helpers.
5. `SAVE_FORMAT_VERSION` is 28. `crates/worldwake-sim/src/save_load.rs` serializes `SimulationState` plus optional runtime bytes; it does not serialize an `ActionDefRegistry`. No save-format version bump is needed for this ticket.
6. Single-crate boundary change (`worldwake-sim`) with downstream consumers in `worldwake-systems` and `worldwake-ai`.

## Architecture Check

1. Per-action `attention_cost` is architecturally cleaner than a per-domain match (P2, P8). Each action declares its own occupancy at registration. No central function to update when adding new actions.
2. No backwards-compatibility shims — all struct literal sites updated atomically in one ticket.

## Verification Layers

1. All action registrations include `attention_cost` -> compilation proof (struct literal completeness)
2. Guideline values set correctly per domain -> code review of registration sites
3. Test helpers updated -> existing tests pass
4. Single-layer ticket (data model change) — no decision/action trace needed.

## What to Change

### 1. Add field to `ActionDef`

In `crates/worldwake-sim/src/action_def.rs`, add alongside the other action-shape fields near `body_cost_per_tick` / `interruptibility`:

```rust
pub attention_cost: Permille,
```

### 2. Update all action registration sites in `worldwake-systems`

Set `attention_cost` for each registration function. Guideline values:

| Domain | Attention Cost | Files |
|--------|---------------|-------|
| Combat | `Permille::new_unchecked(400)` | `combat.rs` (attack, defend, loot, bury, heal, queue_for_corpse, queue_for_care) |
| Production | `Permille::new_unchecked(200)` | `production_actions.rs` (harvest, craft) |
| Travel | `Permille::new_unchecked(100)` | `travel_actions.rs` |
| Transport | `Permille::new_unchecked(100)` | `transport_actions.rs` |
| Trade | `Permille::ZERO` | `trade_actions.rs`, `stock_actions.rs` |
| Needs | `Permille::ZERO` | `needs_actions.rs`, `needs.rs` |
| Social | `Permille::ZERO` | `tell_actions.rs`, `consult_record_actions.rs` |
| Epistemic | `Permille::ZERO` | `epistemic_actions.rs` |
| Office | `Permille::ZERO` | `office_actions.rs` |
| Justice | `Permille::ZERO` | `justice_actions.rs` |
| Patrol | `Permille::new_unchecked(100)` | `patrol_actions.rs` |
| Investigate | `Permille::ZERO` | `investigate_actions.rs` |
| Artifact | `Permille::ZERO` | `artifact_actions.rs` |
| Bandit camp | `Permille::new_unchecked(200)` | `bandit_camp_actions.rs` |
| Facility queue | `Permille::ZERO` | `facility_queue_actions.rs` |
| Perception observe | `Permille::ZERO` | `perception.rs` |

### 3. Update test helpers in `worldwake-sim`

Add `attention_cost: Permille::ZERO` to all `sample_action_def` functions in:
- `crates/worldwake-sim/src/action_def.rs`
- `crates/worldwake-sim/src/action_def_registry.rs`
- `crates/worldwake-sim/src/affordance.rs`
- `crates/worldwake-sim/src/affordance_query.rs`

### 4. Update test/planning helpers in `worldwake-ai`

Add `attention_cost: Permille::ZERO` to `ActionDef` struct literals in:
- `crates/worldwake-ai/src/goal_model.rs`
- `crates/worldwake-ai/src/planning_state.rs`
- `crates/worldwake-ai/src/decision_trace.rs`
- `crates/worldwake-ai/src/plan_revalidation.rs`

### 5. Update remaining `ActionDef` struct literals in `worldwake-sim`

Add `attention_cost: Permille::ZERO` to struct literals in:
- `crates/worldwake-sim/src/action_handler_registry.rs`
- `crates/worldwake-sim/src/action_handler.rs`
- `crates/worldwake-sim/src/tick_action.rs`
- `crates/worldwake-sim/src/tick_step.rs`
- `crates/worldwake-sim/src/start_gate.rs`
- `crates/worldwake-sim/src/interrupt_abort.rs`
- `crates/worldwake-sim/src/action_trace.rs`
- `crates/worldwake-sim/src/per_agent_belief_view.rs`

## Files to Touch

- `crates/worldwake-sim/src/action_def.rs` (modify)
- `crates/worldwake-sim/src/action_def_registry.rs` (modify)
- `crates/worldwake-sim/src/affordance.rs` (modify)
- `crates/worldwake-sim/src/affordance_query.rs` (modify)
- `crates/worldwake-sim/src/action_handler_registry.rs` (modify)
- `crates/worldwake-sim/src/action_handler.rs` (modify)
- `crates/worldwake-sim/src/tick_action.rs` (modify)
- `crates/worldwake-sim/src/tick_step.rs` (modify)
- `crates/worldwake-sim/src/start_gate.rs` (modify)
- `crates/worldwake-sim/src/interrupt_abort.rs` (modify)
- `crates/worldwake-sim/src/action_trace.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-systems/src/combat.rs` (modify)
- `crates/worldwake-systems/src/production_actions.rs` (modify)
- `crates/worldwake-systems/src/travel_actions.rs` (modify)
- `crates/worldwake-systems/src/transport_actions.rs` (modify)
- `crates/worldwake-systems/src/trade_actions.rs` (modify)
- `crates/worldwake-systems/src/stock_actions.rs` (modify)
- `crates/worldwake-systems/src/needs_actions.rs` (modify)
- `crates/worldwake-systems/src/needs.rs` (modify)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-systems/src/consult_record_actions.rs` (modify)
- `crates/worldwake-systems/src/epistemic_actions.rs` (modify)
- `crates/worldwake-systems/src/office_actions.rs` (modify)
- `crates/worldwake-systems/src/justice_actions.rs` (modify)
- `crates/worldwake-systems/src/patrol_actions.rs` (modify)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify)
- `crates/worldwake-systems/src/artifact_actions.rs` (modify)
- `crates/worldwake-systems/src/bandit_camp_actions.rs` (modify)
- `crates/worldwake-systems/src/facility_queue_actions.rs` (modify)
- `crates/worldwake-systems/src/perception.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify)

## Out of Scope

- Perception system integration (S56PEREXP-004)
- Adding new action types
- Changing `SAVE_FORMAT_VERSION` (action defs are code-defined, not save-persisted)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo build --workspace` compiles (all struct literals complete)
2. `cargo test --workspace` passes (all existing tests unaffected)
3. Combat actions have `attention_cost` ~400, production ~200, travel ~100, others 0

### Invariants

1. Every `ActionDef` struct literal in the workspace includes `attention_cost`
2. No existing test behavior changes — this is purely additive data

## Test Plan

### New/Modified Tests

1. None — documentation-only field addition; verification is compilation and existing runtime coverage.

### Commands

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-06.

- Added `attention_cost: Permille` to `ActionDef` in `crates/worldwake-sim/src/action_def.rs` and updated the core `ActionDef` tests to require the field.
- Set per-action `attention_cost` values across `worldwake-systems` registrations using the ticket's launch guidance: combat 400‰, production 200‰, travel/transport/patrol 100‰, and non-occupying domains 0‰.
- Updated all remaining `ActionDef` literals in `worldwake-sim` and `worldwake-ai` helper/test surfaces to include `attention_cost: Permille::ZERO` so the new field stays additive until later S56 integration tickets consume it.

## Verification Result

- Passed `cargo build --workspace`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
