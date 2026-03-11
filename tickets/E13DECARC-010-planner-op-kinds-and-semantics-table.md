# E13DECARC-010: PlannerOpKind enum, semantics table, and goal-to-op mapping

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — AI-layer types and logic
**Deps**: E13DECARC-004, E13DECARC-009

## Problem

`ActionDef` has no declarative effect model, so E13 must introduce a planner-local `PlannerOpKind` + semantics table for Phase 2 action families. This table tells the planner which `ActionDefId` maps to which op kind, whether it can appear mid-plan or only as a leaf, whether it is a materialization barrier, how it updates `PlanningState`, and which goal families it is relevant to.

## Assumption Reassessment (2026-03-11)

1. `ActionDefId` is a sequential index in `ActionDefRegistry` — confirmed.
2. `ActionDef` has fields `id`, `name`, `domain`, but no `effects` field — confirmed (no declarative effects).
3. Phase 2 action families: Travel, Consume, Sleep, Relieve, Wash, TradeAcquire, TradeSell, Harvest, Craft, MoveCargo, Heal, Loot, Bury, Attack — from spec.
4. `GoalKind` from E13DECARC-004.

## Architecture Check

1. This is a planner-local semantics table, NOT a modification of `ActionDef`. The engine remains agnostic.
2. Materialization barriers are explicitly tagged: TradeAcquire, Harvest, Craft, Loot produce unknown future entity IDs.
3. Attack is leaf-only in Phase 2 — never appears mid-plan.
4. The semantics table is the bridge between engine actions and planner reasoning.

## What to Change

### 1. Define `PlannerOpKind` in `worldwake-ai/src/planner_ops.rs`

```rust
pub enum PlannerOpKind {
    Travel,
    Consume,
    Sleep,
    Relieve,
    Wash,
    TradeAcquire,
    TradeSell,
    Harvest,
    Craft,
    MoveCargo,
    Heal,
    Loot,
    Bury,
    Attack,
}
```

### 2. Define `PlannerOpSemantics`

```rust
pub struct PlannerOpSemantics {
    pub op_kind: PlannerOpKind,
    pub may_appear_mid_plan: bool,
    pub is_materialization_barrier: bool,
    pub relevant_goal_kinds: &'static [GoalKindDiscriminant],
}
```

Where `GoalKindDiscriminant` is a fieldless enum mirroring `GoalKind` variants for matching.

### 3. Build the semantics table

A static or lazily-built mapping from `ActionDefId` -> `PlannerOpSemantics`. Since `ActionDefId` is registry-dependent, this must be built at runtime from the `ActionDefRegistry` using action domain/name matching.

```rust
pub fn build_semantics_table(
    registry: &ActionDefRegistry,
) -> BTreeMap<ActionDefId, PlannerOpSemantics>
```

Matching rules:
- Travel actions -> `PlannerOpKind::Travel`
- Eat/Drink consume actions -> `PlannerOpKind::Consume`
- Sleep action -> `PlannerOpKind::Sleep`
- Toilet action -> `PlannerOpKind::Relieve`
- Wash action -> `PlannerOpKind::Wash`
- Trade buy actions -> `PlannerOpKind::TradeAcquire`
- Trade sell actions -> `PlannerOpKind::TradeSell`
- Harvest actions -> `PlannerOpKind::Harvest`
- Craft actions -> `PlannerOpKind::Craft`
- Pickup/Drop/Put-in-container -> `PlannerOpKind::MoveCargo`
- Heal actions -> `PlannerOpKind::Heal`
- Loot actions -> `PlannerOpKind::Loot`
- Bury actions -> `PlannerOpKind::Bury` (if they exist)
- Attack actions -> `PlannerOpKind::Attack`

### 4. Implement `GoalKind::relevant_op_kinds()`

```rust
impl GoalKind {
    pub fn relevant_op_kinds(&self) -> &'static [PlannerOpKind] { ... }
}
```

Returns the subset of `PlannerOpKind`s that are relevant successors for this goal.

### 5. Implement goal satisfaction predicates (stubs)

```rust
pub trait GoalSemantics {
    fn is_satisfied(&self, state: &PlanningState) -> bool;
    fn is_progress_barrier(&self, step: &PlannedStep, post_state: &PlanningState) -> bool;
}
```

Full implementations depend on `PlanningState` (E13DECARC-011), so this ticket defines the trait and provides stub/skeleton implementations.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — add PlannerOpKind, semantics, table builder)
- `crates/worldwake-ai/src/goal_model.rs` (modify — add `relevant_op_kinds()`, `GoalKindDiscriminant`)

## Out of Scope

- `PlanningState` update logic per op kind — E13DECARC-011
- Full `GoalSemantics` implementations — E13DECARC-011
- Plan search algorithm — E13DECARC-012
- Action handler modifications — none needed

## Acceptance Criteria

### Tests That Must Pass

1. `PlannerOpKind` has exactly 14 variants
2. `build_semantics_table()` maps all registered Phase 2 action defs to a `PlannerOpKind`
3. `Attack` has `may_appear_mid_plan = false` (leaf-only)
4. `TradeAcquire`, `Harvest`, `Craft`, `Loot` have `is_materialization_barrier = true`
5. `Travel`, `Consume`, `Sleep`, `Relieve`, `Wash`, `TradeSell`, `MoveCargo`, `Heal`, `Bury` have `is_materialization_barrier = false`
6. `GoalKind::ConsumeOwnedCommodity.relevant_op_kinds()` includes `Consume` and `Travel` but not `Attack`
7. `GoalKind::ReduceDanger.relevant_op_kinds()` includes `Travel` and `Attack` and `Heal`
8. `GoalKind::RestockCommodity.relevant_op_kinds()` includes `Travel`, `TradeAcquire`, `Harvest`, `Craft`, `MoveCargo`
9. Table uses `BTreeMap`, not `HashMap`
10. Existing suite: `cargo test --workspace`

### Invariants

1. Semantics table is planner-local — does not modify `ActionDef`
2. `Attack` is leaf-only
3. Materialization barriers are correctly tagged
4. No Phase 3+ op kinds
5. No `HashMap`/`HashSet`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — table construction tests, barrier/leaf assertions, relevant-op-kind coverage

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
