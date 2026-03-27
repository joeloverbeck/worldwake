# S31-002: Implement `derive_invalidation_conditions` for All GoalKind Variants

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI exhaustion module
**Deps**: S31-001

## Problem

Each exhausted goal needs to record which world conditions would make it worth re-searching. This ticket implements the mapping from every `GoalKind` variant to its specific `ExhaustionInvalidationCondition` set plus baseline snapshot, following the condition table in the S31 spec.

## Assumption Reassessment (2026-03-27)

Shared abstraction boundary under audit: `GoalKind` -> `ExhaustionInvalidationCondition` plus `ExhaustionBaseline`, derived from `GoalBeliefView` and `RecipeRegistry`, then later consumed by the exhaustion runtime in `crates/worldwake-ai/src/agent_tick/planning.rs`.

1. `GoalKind` currently has 21 live variants, not 23, at `crates/worldwake-core/src/goal.rs`. The match and coverage tests must target the current 21-variant surface.
2. `ExhaustionInvalidationCondition` and `ExhaustionBaseline` already exist in `crates/worldwake-ai/src/exhaustion.rs`, and `ExhaustionEntry` already carries `invalidation_conditions` and `baseline` in `crates/worldwake-ai/src/decision_runtime.rs`. This ticket is no longer introducing those data shapes; it is filling in the derivation logic that populates them.
3. `GoalBeliefView` at `crates/worldwake-sim/src/belief_view.rs` does provide the reads needed for position, needs, commodity counts, unique item counts, wounds, hostiles, and liveness: `effective_place`, `homeostatic_needs`, `commodity_quantity`, `unique_item_count`, `wounds`, `visible_hostiles_for`, and `is_alive`.
4. `GoalBeliefView` does not expose a standalone facility-signature or facility-access snapshot API. `FacilitiesChanged` is still a valid invalidation condition for this ticket, but proving the condition changed remains a follow-on concern for S31-003 via the runtime-side facility/dirty-state path, not via new `GoalBeliefView` methods in this ticket.
5. `RecipeRegistry::get(recipe_id)` returns `Option<&RecipeDefinition>` in `crates/worldwake-sim/src/recipe_registry.rs`, and `RecipeDefinition.inputs` is the live source for deriving `CommodityChanged(...)` invalidation conditions for `GoalKind::ProduceCommodity`.
6. `CommodityPurpose` still has the three expected variants in `crates/worldwake-core/src/goal.rs`: `SelfConsume`, `Restock`, and `RecipeInput(RecipeId)`. Only `Restock` currently adds `CommodityChanged(Coin)`.
7. `HomeostaticNeedId` still has the five expected variants in `crates/worldwake-core/src/needs.rs`: `Hunger`, `Thirst`, `Fatigue`, `Bladder`, and `Dirtiness`.
8. No live goal currently derives a `UniqueItemChanged(...)` invalidation condition from the S31 mapping table. The baseline should still support unique-item snapshots because the runtime type already models them, but this ticket should not invent a new unique-item-driven goal mapping that the spec does not call for.
9. The function should remain pure and deterministic: same `GoalKind`, agent, view, and recipe registry inputs must produce the same condition vector and baseline.

## Architecture Check

1. A pure, exhaustive `GoalKind` match is cleaner than the current coarse dirty-mask architecture because it localizes goal-specific invalidation semantics to one derivation point instead of scattering ad hoc reset rules across runtime code.
2. Keeping the derivation logic in `crates/worldwake-ai/src/exhaustion.rs` is cleaner than pushing it into unrelated ranking, candidate-generation, or runtime-observation modules. The concern here is exhaustion invalidation semantics, so the logic belongs with the exhaustion types.
3. The function should be `pub(crate)` rather than part of the public crate API. `worldwake-ai` already re-exports the data types that other crates need; exporting an internal derivation helper now would widen the API surface without a current external caller.
4. This design is materially better than the current architecture because it replaces global invalidation heuristics with goal-local facts while preserving determinism and belief-only planning. The only notable architectural gap left after this ticket is facility-delta detection, which belongs in the later invalidation-check path rather than in this derivation function.

## Verification Layers

1. exhaustive goal-to-condition coverage -> focused unit tests in `crates/worldwake-ai/src/exhaustion.rs`
2. recipe-input invalidation derivation for `ProduceCommodity` -> focused unit test with a real `RecipeRegistry`
3. restock-vs-self-consume acquisition divergence -> focused unit tests in `crates/worldwake-ai/src/exhaustion.rs`
4. baseline snapshot correctness for the derived condition set -> focused unit test with a mock `GoalBeliefView`
5. end-to-end regression safety for the crate -> `cargo test -p worldwake-ai`

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

Implement the full live mapping table from the S31 spec. Build the baseline by snapshotting relevant state from the view:
- `position` from `view.effective_place(agent)`
- `needs` from `view.homeostatic_needs(agent)`
- `commodity_quantities` for each `CommodityChanged(kind)` condition
- `unique_item_counts` for each `UniqueItemChanged(kind)` condition
- `wound_count` from `view.wounds(agent).len()`
- `hostile_count` from `view.visible_hostiles_for(agent).len()`

If a recipe lookup fails for `ProduceCommodity`, do not panic. Derive the non-recipe conditions that still hold (`PositionChanged`, `FacilitiesChanged`) and leave per-input commodity invalidation absent because the recipe inputs are unknown to the current registry.

### 2. Add `need_value` helper to `exhaustion.rs`

```rust
pub(crate) fn need_value(needs: &HomeostaticNeeds, need: HomeostaticNeedId) -> Permille
```

Maps `HomeostaticNeedId` variant to the corresponding field on `HomeostaticNeeds`.

## Files to Touch

- `crates/worldwake-ai/src/exhaustion.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify only if focused runtime tests need access to the helper through existing crate-internal call paths)

## Out of Scope

- `condition_changed` implementation (S31-003)
- `invalidate_exhausted_goals` implementation (S31-004)
- Wiring into `record_exhausted_goals` (S31-005)
- Removing TTL (S31-006)
- Golden tests (S31-007)
- Adding new methods to `GoalBeliefView` trait — all needed methods already exist
- Public re-export of `derive_invalidation_conditions` from `crates/worldwake-ai/src/lib.rs`

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: every current `GoalKind` variant (all 21) returns a non-empty `invalidation_conditions` vector
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
2. Every current `GoalKind` variant is covered (compiler-enforced exhaustive match)
3. No empty condition vectors returned for any variant
4. `NeedCrossedThreshold` uses `Permille(100)` as the threshold delta (spec mandated)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exhaustion.rs` (inline `#[cfg(test)]`) — table-driven per-variant coverage across the current 21 `GoalKind` variants
2. `crates/worldwake-ai/src/exhaustion.rs` (inline `#[cfg(test)]`) — restock/self-consume divergence and recipe-input derivation tests
3. `crates/worldwake-ai/src/exhaustion.rs` (inline `#[cfg(test)]`) — baseline snapshot correctness and determinism with a mock `GoalBeliefView`

### Commands

1. `cargo test -p worldwake-ai exhaustion`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-27
- What actually changed:
  - Implemented `derive_invalidation_conditions` in `crates/worldwake-ai/src/exhaustion.rs` as a pure exhaustive `GoalKind` match over the current 21 live goal variants.
  - Added `need_value` and baseline construction logic that snapshots only the condition-relevant commodity and unique-item keys plus current position, needs, wound count, and hostile count.
  - Added focused unit coverage for full goal-surface coverage, recipe-input invalidation derivation, acquisition-purpose divergence, threshold usage, target-death invalidation, and deterministic baseline capture.
- Deviations from original plan:
  - Did not modify `crates/worldwake-ai/src/lib.rs`; the derivation helper stays crate-internal because no external caller currently needs it.
  - Did not touch `crates/worldwake-ai/src/agent_tick/planning.rs`; focused unit coverage in `exhaustion.rs` was sufficient for this ticket's scope.
  - Reassessed the live goal surface from 23 claimed variants to 21 actual variants before implementation.
- Verification results:
  - `cargo test -p worldwake-ai exhaustion` passed.
  - `cargo test -p worldwake-ai` passed.
  - `cargo clippy --workspace` passed.
  - `cargo test --workspace` passed.
