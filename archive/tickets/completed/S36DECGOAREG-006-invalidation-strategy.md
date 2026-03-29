# S36DECGOAREG-006: Introduce InvalidationStrategy and migrate exhaustion dispatch

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: `worldwake-ai` declaration-owned invalidation dispatch only
**Deps**: S36DECGOAREG-002

## Problem

`derive_invalidation_conditions()` in `crates/worldwake-ai/src/exhaustion.rs` still owns a large exhaustive `match GoalKind` even though S36 Phase 2 already centralized static dispatch in `GoalDispatchDeclaration`. This ticket should migrate only the invalidation-strategy selection into declaration data, while preserving the current concrete invalidation/baseline behavior exactly.

## Assumption Reassessment (2026-03-29)

1. Shared abstraction boundary under audit: `GoalDispatchDeclaration` in `crates/worldwake-ai/src/goal_dispatch_decl.rs` currently centralizes static dispatch (`trace_label`, `provenance_family`, `relevant_ops`), while `crates/worldwake-ai/src/exhaustion.rs::derive_invalidation_conditions()` still hard-codes dynamic invalidation routing. This ticket should align those two surfaces without changing invalidation semantics.
2. `derive_invalidation_conditions()` is currently an exhaustive `match GoalKind` and returns `(Vec<ExhaustionInvalidationCondition>, ExhaustionBaseline)`. The live exhaustive surface is 21 authoritative `GoalKind` variants, but S36 dispatch completeness is 24 `GoalDispatchKey` values because `AcquireCommodity` splits three ways and `PunishAccused` splits two ways in the declaration layer.
3. The live invalidation families are not identical to the original ticket text:
   - `ConsumeOwnedCommodity` -> `CommodityChanged(commodity)`.
   - `AcquireCommodity(SelfConsume | RecipeInput)` -> `PositionChanged` + `CommodityChanged(commodity)`.
   - `AcquireCommodity(Restock)` -> `PositionChanged` + `CommodityChanged(commodity)` + `CommodityChanged(Coin)`.
   - `Sleep` -> `NeedChangedBands(Fatigue, live band)` + `FacilitiesChanged`.
   - `Relieve` -> `NeedChangedBands(Bladder, live band)` + `PositionChanged`.
   - `Wash` -> `NeedChangedBands(Dirtiness, live band)` + `FacilitiesChanged`.
   - `EngageHostile` -> `PositionChanged` + `WoundsChanged` + `TargetDead(target)`.
   - `ReduceDanger` -> `PositionChanged` + `WoundsChanged` + `HostilesChanged`.
   - `TreatWounds` -> `PositionChanged` + `WoundsChanged` + `CommodityChanged(Medicine)` + `TargetDead(patient)`.
   - `ProduceCommodity` -> `PositionChanged` + `FacilitiesChanged` + per-input `CommodityChanged(...)` entries when the recipe is present. There is no output-commodity invalidation condition today.
   - `SellCommodity` and `MoveCargo` share `PositionChanged` + `CommodityChanged(commodity)`.
   - `RestockCommodity` -> `PositionChanged` + `CommodityChanged(commodity)` + `CommodityChanged(Coin)`.
   - `LootCorpse` and `BuryCorpse` share `PositionChanged` + `TargetDead(corpse)`.
   - `ShareBelief` -> `PositionChanged` + `TargetDead(listener)`.
   - `ClaimOffice` -> `PositionChanged` + `BlockerExpired`.
   - `SupportCandidateForOffice` -> `PositionChanged` + `BlockerExpired` + `TargetDead(candidate)`.
   - `InvestigateViolation` -> `PositionChanged`.
   - `StealItem` currently uses `PositionChanged` + `TargetDead(target_item)`. This is semantically surprising for an item-target goal, but changing that behavior is out of scope for this structural migration and should be treated as a follow-up bug/architecture ticket if still desired after migration.
   - `Accuse` -> `PositionChanged` + `TargetDead(accused)`.
   - `PunishAccused(Fine | Exile)` currently shares `PositionChanged` + `TargetDead(accused)` + `BlockerExpired)`.
4. Existing focused coverage is broader than the original ticket claimed. `crates/worldwake-ai/src/exhaustion.rs` already has tests covering all live `GoalKind` variants plus several invalidation-specific behavioral cases. The migration should strengthen that surface with strategy-routing assertions instead of replacing it with weaker representative-only checks.
5. `GoalDispatchDeclaration` does not yet carry `invalidation_strategy`, so adding that field will force every declaration entry to opt into a strategy explicitly. That is the intended compile-time completeness mechanism for this ticket.

## Architecture Check

1. Declaration-owned strategy selection is cleaner than the current architecture because S36 already established `GoalDispatchDeclaration` as the AI dispatch read-model. Leaving invalidation routing in a separate monolithic `match GoalKind` preserves a second dispatch matrix and weakens the single-source-of-truth goal of the spec.
2. The robust architecture here is not "one strategy per dispatch key." Several dispatch keys intentionally share invalidation behavior, so the cleaner end-state is a smaller invalidation family enum reused across declarations. That removes duplication while keeping runtime computation grounded in concrete `GoalKind`, live thresholds, recipe contents, and belief state.
3. No backwards-compatibility shims or alias paths. `derive_invalidation_conditions()` stays in place, but it routes through declaration metadata immediately rather than maintaining a parallel authoritative dispatch table.

## Verification Layers

1. Declaration completeness at the audited boundary -> focused declaration test proving every `GoalDispatchKey` declaration has an explicit `invalidation_strategy`.
2. Invalidation routing behavior -> focused `exhaustion.rs` unit tests proving each live invalidation family still derives the same conditions and baselines as before, including payload-sensitive `AcquireCommodity` splits.
3. AI runtime regression -> `cargo test -p worldwake-ai`.
4. Workspace integration / lint -> `cargo test --workspace` and `cargo clippy --workspace`.

## What to Change

### 1. Define `InvalidationStrategy` enum in `goal_dispatch_decl.rs`

One variant per live invalidation family, not per dispatch key. Minimal parameterization is acceptable where it removes meaningless duplication, for example a facility-based need variant versus a position-based need variant, while still leaving concrete payload inspection inside helper functions.

### 2. Add `invalidation_strategy` field to `GoalDispatchDeclaration`

Extend the struct and populate the field in every declaration.

### 3. Refactor `derive_invalidation_conditions()` in `exhaustion.rs`

Replace the monolithic match with:
1. Look up `GoalDispatchKey::from_goal_kind(goal).declaration().invalidation_strategy`.
2. Match on the strategy enum to call family-specific helper functions.
3. Family helpers still take `(goal, agent, view, recipe_registry)` and may inspect concrete payload fields.

### 4. Equivalence tests

Add/adjust focused tests so the migration proves the declaration route is complete and the live invalidation semantics remain unchanged for all currently distinct invalidation families.

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — add `InvalidationStrategy` enum, add field to struct, populate in declarations)
- `crates/worldwake-ai/src/exhaustion.rs` (modify — refactor main dispatch to strategy routing)

## Out of Scope

- Changing `ExhaustionInvalidationCondition` or `ExhaustionBaseline` types
- Changing the `derive_invalidation_conditions()` function signature or return type
- Feasibility strategy migration (ticket 007)
- Wildcard audit (ticket 008)
- Any changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. A declaration test proves every live `GoalDispatchKey` declaration has an explicit `invalidation_strategy`.
2. Focused exhaustion tests prove payload-sensitive `AcquireCommodity` routing still distinguishes restock from self-consume / recipe-input behavior and that shared families (`LootCorpse`/`BuryCorpse`, `SellCommodity`/`MoveCargo`, `PunishFine`/`PunishExile`) still produce identical conditions where intended.
3. Existing suite: `cargo test -p worldwake-ai`
4. Full workspace: `cargo test --workspace`
5. Lint: `cargo clippy --workspace`

### Invariants

1. Zero behavioral change — exhaustion invalidation conditions and baselines are identical.
2. Adding a `GoalDispatchKey` declaration without an `invalidation_strategy` fails compilation (struct field is required).
3. The strategy enum routes to computations; it does not become a second source of truth for recipe inputs, thresholds, positions, commodities, or targets (P3).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — add a declaration completeness test for `invalidation_strategy`.
2. `crates/worldwake-ai/src/exhaustion.rs` — add/adjust focused tests around payload-sensitive and shared-family invalidation routing.

### Commands

1. `cargo test -p worldwake-ai --lib invalidation`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-29
- Actual changes:
  - Added `InvalidationStrategy` to `crates/worldwake-ai/src/goal_dispatch_decl.rs` and required every `GoalDispatchDeclaration` to declare one explicitly.
  - Refactored `crates/worldwake-ai/src/exhaustion.rs::derive_invalidation_conditions()` to route through declaration-owned invalidation strategies and small family helpers while preserving existing invalidation conditions and baselines.
  - Re-exported `InvalidationStrategy` from `crates/worldwake-ai/src/lib.rs`.
  - Added focused declaration and exhaustion tests covering strategy completeness, payload-sensitive acquire splits, and shared-family equivalence.
- Deviations from original plan:
  - The implementation used a smaller shared-family strategy enum rather than one strategy per dispatch key.
  - Verification reused and strengthened the existing focused exhaustion coverage instead of introducing a separate old-vs-new dual-path equivalence harness.
  - No behavior change was made to the semantically surprising `StealItem -> TargetDead(target_item)` invalidation path; that remains follow-up material if the project wants to correct the model itself.
- Verification results:
  - `cargo test -p worldwake-ai --lib invalidation`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
