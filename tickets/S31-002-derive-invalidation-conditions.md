# S31-002: Implement `derive_invalidation_conditions` for All GoalKind Variants

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI exhaustion module
**Deps**: S31-001

## Problem

Each exhausted goal needs to record which world conditions would make it worth re-searching. This ticket implements the mapping from every `GoalKind` variant to its specific `ExhaustionInvalidationCondition` set plus baseline snapshot, following the condition table in the S31 spec.

## Assumption Reassessment (2026-03-27)

1. `GoalKind` has 23 variants at `crates/worldwake-core/src/goal.rs:16-84`. All must be covered by the match.
2. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:29` provides `effective_place`, `commodity_quantity`, `unique_item_count`, `wounds`, `homeostatic_needs`, `visible_hostiles_for`, `is_alive`.
3. `RecipeRegistry::get(recipe_id)` returns `Option<&RecipeDefinition>` which has `inputs: Vec<(CommodityKind, Quantity)>` for deriving per-input `CommodityChanged` conditions on `ProduceCommodity`.
4. `CommodityPurpose` at `crates/worldwake-core/src/goal.rs` has variants `SelfConsume`, `Restock`, `RecipeInput(RecipeId)`. The `AcquireCommodity` condition set varies by purpose (Restock adds `CommodityChanged(Coin)`).
5. `HomeostaticNeedId` at `crates/worldwake-core/src/needs.rs:18-25` has 5 variants.
6. The function signature uses `&dyn GoalBeliefView` — the belief interface already used by candidate generation and ranking.
7. Pure function: same inputs -> same outputs. No side effects.

## Architecture Check

1. Pure function with exhaustive match over `GoalKind`. Compiler enforces coverage of all variants. No missing arms possible.
2. No backward-compatibility concerns — this is new code.

## Verification Layers

1. Every `GoalKind` variant returns non-empty conditions -> unit test per variant
2. `ProduceCommodity` includes `CommodityChanged` for each recipe input -> unit test with mock recipe registry
3. `AcquireCommodity { purpose: Restock }` includes `CommodityChanged(Coin)` -> unit test
4. Baseline snapshot captures correct current state from view -> unit test with mock view
5. Pure function determinism -> same inputs produce identical output across calls

## What to Change

### 1. Add `derive_invalidation_conditions` to `crates/worldwake-ai/src/exhaustion.rs`

```rust
pub fn derive_invalidation_conditions(
    goal: &GoalKind,
    agent: EntityId,
    view: &dyn GoalBeliefView,
    recipe_registry: &RecipeRegistry,
) -> (Vec<ExhaustionInvalidationCondition>, ExhaustionBaseline)
```

Implement the full mapping table from the S31 spec (Section 4). Build the baseline by snapshotting relevant state from the view:
- `position` from `view.effective_place(agent)`
- `needs` from `view.homeostatic_needs(agent)`
- `commodity_quantities` for each `CommodityChanged(kind)` condition
- `unique_item_counts` for each `UniqueItemChanged(kind)` condition
- `wound_count` from `view.wounds(agent).len()`
- `hostile_count` from `view.visible_hostiles_for(agent).len()`

### 2. Add `need_value` helper to `exhaustion.rs`

```rust
pub(crate) fn need_value(needs: &HomeostaticNeeds, need: HomeostaticNeedId) -> Permille
```

Maps `HomeostaticNeedId` variant to the corresponding field on `HomeostaticNeeds`.

### 3. Export from `lib.rs`

Re-export `derive_invalidation_conditions` from the crate root.

## Files to Touch

- `crates/worldwake-ai/src/exhaustion.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export)

## Out of Scope

- `condition_changed` implementation (S31-003)
- `invalidate_exhausted_goals` implementation (S31-004)
- Wiring into `record_exhausted_goals` (S31-005)
- Removing TTL (S31-006)
- Golden tests (S31-007)
- Adding new methods to `GoalBeliefView` trait — all needed methods already exist

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: every `GoalKind` variant (all 23) returns non-empty `invalidation_conditions`
2. Unit test: `ConsumeOwnedCommodity { commodity: Bread }` returns exactly `[CommodityChanged(Bread)]`
3. Unit test: `AcquireCommodity { commodity: Apple, purpose: Restock }` includes `CommodityChanged(Coin)`
4. Unit test: `AcquireCommodity { commodity: Apple, purpose: SelfConsume }` does NOT include `CommodityChanged(Coin)`
5. Unit test: `ProduceCommodity { recipe_id }` includes `CommodityChanged(input)` for each recipe input
6. Unit test: `Sleep` includes `NeedCrossedThreshold { Fatigue, Permille(100) }`
7. Unit test: `Wash` includes `NeedCrossedThreshold { Dirtiness, Permille(100) }`
8. Unit test: `EngageHostile { target }` includes `TargetDead(target)`
9. Unit test: baseline snapshot captures position, needs, commodity quantities from mock view
10. Existing suite: `cargo test --workspace`

### Invariants

1. `derive_invalidation_conditions` is a pure function — same inputs produce identical outputs
2. Every `GoalKind` variant is covered (compiler-enforced exhaustive match)
3. No empty condition vectors returned for any variant
4. `NeedCrossedThreshold` uses `Permille(100)` as the threshold delta (spec mandated)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exhaustion.rs` (inline `#[cfg(test)]`) — per-variant condition coverage (23 test cases or a table-driven test)
2. `crates/worldwake-ai/src/exhaustion.rs` (inline `#[cfg(test)]`) — baseline snapshot correctness with mock `GoalBeliefView`

### Commands

1. `cargo test -p worldwake-ai exhaustion`
2. `cargo clippy --workspace && cargo test --workspace`
