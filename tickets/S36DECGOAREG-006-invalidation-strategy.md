# S36DECGOAREG-006: Introduce InvalidationStrategy and migrate exhaustion dispatch

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S36DECGOAREG-002

## Problem

`derive_invalidation_conditions()` in `exhaustion.rs` contains a large exhaustive match on all 21 `GoalKind` variants to produce invalidation conditions and baselines. This dispatch should be mediated by a declaration-owned `InvalidationStrategy` enum that selects which family-specific computation runs, while the computation itself continues to consume concrete `GoalKind`, belief views, recipe inputs, and threshold data as runtime arguments (P3).

## Assumption Reassessment (2026-03-29)

1. `derive_invalidation_conditions()` is at `exhaustion.rs:39-154`. Exhaustive match on `GoalKind`, no wildcard. Returns `(Vec<ExhaustionInvalidationCondition>, ExhaustionBaseline)`.
2. The match body groups goals by invalidation behavior — distinct families identified from live code:
   - **CommodityBased**: `ConsumeOwnedCommodity` — single commodity changed condition.
   - **AcquireNeedDriven**: `AcquireCommodity` with `SelfConsume|RecipeInput` — position + commodity changed.
   - **AcquireRestock**: `AcquireCommodity` with `Restock` — position + commodity + Coin changed.
   - **NeedBased**: `Sleep`, `Relieve`, `Wash` — position + need band changed (each with different `HomeostaticNeedId`).
   - **CombatTarget**: `EngageHostile` — hostiles changed + target dead.
   - **DangerReduction**: `ReduceDanger` — wounds changed + hostiles changed.
   - **PatientTarget**: `TreatWounds` — wounds changed + position changed + target dead.
   - **RecipeProduction**: `ProduceCommodity` — position + facilities + recipe input commodities + commodity changed.
   - **EnterpriseTrade**: `SellCommodity`, `MoveCargo` — position + commodity changed.
   - **EnterpriseRestock**: `RestockCommodity` — position + commodity + Coin changed.
   - **CorpseTarget**: `LootCorpse`, `BuryCorpse` — position + target dead.
   - **SocialTarget**: `ShareBelief` — position + target dead.
   - **PoliticalOffice**: `ClaimOffice` — position changed.
   - **PoliticalSupport**: `SupportCandidateForOffice` — position + target dead.
   - **Investigation**: `InvestigateViolation` — position changed.
   - **TheftTarget**: `StealItem` — position + unique item changed.
   - **Accusation**: `Accuse` — position + target dead.
   - **Punishment**: `PunishAccused` — position + target dead.
3. Some families share logic (e.g., `LootCorpse`/`BuryCorpse` are identical). Strategy variants can collapse shared families.
4. The function signature takes `(goal, agent, view, recipe_registry)` — the strategy selector routes to family-specific helpers that still take these runtime arguments.

## Architecture Check

1. Strategy selectors are compile-time routing decisions (P3: the selector is static declaration data; the computation consumes concrete state). This is cleaner than a monolithic match because adding a new goal requires choosing a strategy in the declaration (compile-time enforced) rather than adding an arm to a 150-line match.
2. No backwards-compatibility shims. The `derive_invalidation_conditions()` function is refactored in-place. Its signature and return type are unchanged.

## Verification Layers

1. Strategy routing equivalence → focused unit test: for every `GoalKind` variant, the strategy-routed result exactly matches the pre-migration result given the same inputs.
2. Behavioral equivalence → full AI test suite: all golden tests pass unchanged.
3. Single-layer ticket: exhaustion dispatch only.

## What to Change

### 1. Define `InvalidationStrategy` enum in `goal_dispatch_decl.rs`

One variant per identified invalidation family (see list above). Some variants may be parameterized minimally (e.g., `NeedBased { need_id: HomeostaticNeedId }`) or the family helper can derive this from the concrete `GoalKind`.

### 2. Add `invalidation_strategy` field to `GoalDispatchDeclaration`

Extend the struct and populate the field in every declaration.

### 3. Refactor `derive_invalidation_conditions()` in `exhaustion.rs`

Replace the monolithic match with:
1. Look up `GoalDispatchKey::from_goal_kind(goal).declaration().invalidation_strategy`.
2. Match on the strategy enum to call family-specific helper functions.
3. Family helpers still take `(goal, agent, view, recipe_registry)` and may inspect concrete payload fields.

### 4. Equivalence tests

Add tests comparing strategy-routed results against known expected outputs for representative goal shapes.

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

1. `test_invalidation_strategy_equivalence`: For every `GoalKind` variant (including all `CommodityPurpose` and `PunishmentKind` splits), the strategy-routed `derive_invalidation_conditions()` produces identical `(conditions, baseline)` as the pre-migration implementation, given the same mock belief view.
2. `test_invalidation_strategy_completeness`: Every `InvalidationStrategy` variant is used by at least one declaration.
3. Existing suite: `cargo test -p worldwake-ai`
4. Full workspace: `cargo test --workspace`

### Invariants

1. Zero behavioral change — exhaustion invalidation conditions and baselines are identical.
2. Adding a `GoalDispatchKey` without an `invalidation_strategy` fails compilation (struct field is required).
3. The strategy enum routes to computations; it does not encode conditions or data directly (P3).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exhaustion.rs` (test module) — strategy routing equivalence tests across all goal shapes with mock belief views.

### Commands

1. `cargo test -p worldwake-ai -- invalidation`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
