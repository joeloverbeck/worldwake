# HARCARGOACON-001: Migrate MoveCargo goal identity from lot-based to commodity-based

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-core (goal schema), worldwake-ai (pattern matches)
**Deps**: None (first ticket in chain)

## Problem

`GoalKind::MoveCargo { lot: EntityId, destination: EntityId }` ties cargo goal identity to a specific authoritative lot entity. Partial pickup can split that lot into a new entity, making the goal key stale across replanning. The spec mandates replacing this with `MoveCargo { commodity: CommodityKind, destination: EntityId }`.

## Assumption Reassessment (2026-03-12)

1. `GoalKind::MoveCargo` currently has fields `lot: EntityId, destination: EntityId` — confirmed at `goal.rs:39-42`
2. `GoalKey::from` extracts `entity = Some(lot), place = Some(destination)` — confirmed at `goal.rs:76`
3. Pattern matches exist in 8 files across `worldwake-core` and `worldwake-ai` — confirmed via grep
4. No backward-compatibility aliases should be introduced — confirmed per spec Section "No Backward Compatibility Path"

## Architecture Check

1. Commodity+destination identity survives lot splits because no specific entity is named in the goal key
2. No compatibility shims — all callers updated directly

## What to Change

### 1. `GoalKind::MoveCargo` variant (worldwake-core)

Replace:
```rust
MoveCargo { lot: EntityId, destination: EntityId }
```
With:
```rust
MoveCargo { commodity: CommodityKind, destination: EntityId }
```

### 2. `GoalKey::from` extraction (worldwake-core)

Change the `MoveCargo` arm from:
```rust
GoalKind::MoveCargo { lot, destination } => (None, Some(lot), Some(destination))
```
To:
```rust
GoalKind::MoveCargo { commodity, destination } => (Some(commodity), None, Some(destination))
```

### 3. Update `goal.rs` tests (worldwake-core)

Update `goal_key_extracts_entity_and_place_for_move_cargo` test to construct the new variant and assert `commodity = Some(...)`, `entity = None`, `place = Some(destination)`.

### 4. Fix all AI crate pattern matches (mechanical compile fixes)

Update destructuring in:
- `ranking.rs:258` — change `MoveCargo { lot, destination }` to `MoveCargo { commodity, destination }`. Replace `item_lot_commodity(lot).map_or(...)` with direct `market_signal_for_place(view, agent, commodity, destination)`.
- `goal_model.rs:333` — wildcard match `MoveCargo { .. }`, no field access, no change needed beyond compilation
- `goal_model.rs:130` — wildcard match, no change needed
- `search.rs:239` — wildcard match `MoveCargo { .. }`, no change needed
- `candidate_generation.rs:1562` — wildcard match, no change needed
- `failure_handling.rs` — all uses are `PlannerOpKind::MoveCargo` (not `GoalKind`), no change needed
- `agent_tick.rs:1122` — test constructs `GoalKind::MoveCargo { lot, destination }`; update to use `commodity` field
- `agent_tick.rs:929,1133,1154` — `PlannerOpKind::MoveCargo`, no change needed

### 5. Ranking motive scoring (ranking.rs:258)

This is the one behavioral change in this ticket. The old code looked up commodity from the lot entity via `item_lot_commodity(lot)`. The new code uses the commodity directly from the goal variant:
```rust
GoalKind::MoveCargo { commodity, destination } => {
    let signal = market_signal_for_place(context.view, context.agent, commodity, destination);
    score_product(context.utility.enterprise_weight, signal)
}
```

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — test code only)

## Out of Scope

- Adding new `BeliefView` methods (HARCARGOACON-002)
- Adding `restock_gap_at_destination` (HARCARGOACON-003)
- Changing cargo candidate generation logic (HARCARGOACON-004)
- Changing goal satisfaction or search support (HARCARGOACON-005)
- Any new test scenarios beyond fixing existing tests to compile
- Modifying `PlannerOpKind::MoveCargo` (unchanged)
- Modifying `failure_handling.rs` (uses `PlannerOpKind`, not `GoalKind`)

## Acceptance Criteria

### Tests That Must Pass

1. `goal_key_extracts_entity_and_place_for_move_cargo` — updated to verify `commodity = Some(...)`, `entity = None`, `place = Some(destination)`
2. `ranking.rs` tests — existing ranking tests continue to pass with new destructuring
3. `agent_tick.rs` tests — existing tests compile and pass with updated `MoveCargo` construction
4. `cargo test --workspace` — full workspace compiles and passes
5. `cargo clippy --workspace` — no warnings

### Invariants

1. `GoalKind::MoveCargo` no longer contains any `EntityId` field for a lot — only `commodity: CommodityKind` and `destination: EntityId`
2. `GoalKey` for `MoveCargo` has `commodity = Some(commodity)`, `entity = None`, `place = Some(destination)`
3. No backward-compatibility alias for the old lot-based variant exists anywhere
4. All existing tests that were passing before continue to pass (with updated construction)
5. Determinism: `GoalKind` remains `Ord + Eq + Serialize + Deserialize`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs::goal_key_extracts_entity_and_place_for_move_cargo` — updated to new variant shape
2. `crates/worldwake-ai/src/agent_tick.rs` — test `GoalKind::MoveCargo` constructions updated

### Commands

1. `cargo test -p worldwake-core goal`
2. `cargo test -p worldwake-ai ranking`
3. `cargo test -p worldwake-ai agent_tick`
4. `cargo test --workspace`
5. `cargo clippy --workspace`
