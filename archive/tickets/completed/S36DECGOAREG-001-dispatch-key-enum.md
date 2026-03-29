# S36DECGOAREG-001: Introduce GoalDispatchKey enum and GoalKind→Key lookup

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S33 ✅, S31 ✅, S25 ✅

## Problem

AI goal dispatch is scattered across ~8 files with per-goal-kind match statements. The first step toward consolidation is a payload-aware declaration key that captures dispatch-distinguishing goal shapes — finer than `GoalKindTag` where static dispatch actually differs on payload.

## Assumption Reassessment (2026-03-29)

1. `GoalKind` is defined in `crates/worldwake-core/src/goal.rs` with 21 variants. `GoalKindTag` is defined in `crates/worldwake-ai/src/goal_model.rs`, not in `worldwake-core`. The original ticket text that located `GoalKindTag` in `worldwake-ai` was correct; the S36 spec narrative is stale on this specific symbol location.
2. Live payload-sensitive static dispatch splits confirmed:
   - `GoalKind::ranked_goal_provenance_family()` in `crates/worldwake-ai/src/goal_model.rs`: `AcquireCommodity` splits by `CommodityPurpose` between `SelfConsume|RecipeInput(_)` and `Restock`, and `PunishAccused` remains coarse there.
   - `drive_goal_ranking_provenance()`, `priority_class()`, and `motive_score()` in `crates/worldwake-ai/src/ranking.rs`: `AcquireCommodity::SelfConsume` and `AcquireCommodity::RecipeInput(_)` already route to different ranking computations. `SelfConsume` uses self-need pressure; `RecipeInput(_)` uses recipe-output demand. This is a real static dispatch distinction and must become a distinct `GoalDispatchKey`.
   - `GoalKind::relevant_op_kinds()` in `crates/worldwake-ai/src/goal_model.rs`: `PunishAccused` splits by `PunishmentKind` — `Fine` → `FINE_OPS`, `Exile` → `EXILE_OPS`.
   - `derive_invalidation_conditions()` in `crates/worldwake-ai/src/exhaustion.rs`: `AcquireCommodity::Restock` adds `CommodityChanged(Coin)` not present for `SelfConsume` and currently also not present for `RecipeInput(_)`.
3. `AcquireCommodity::SelfConsume` and `AcquireCommodity::RecipeInput(_)` do not share identical static dispatch. They share `relevant_op_kinds()`, exhaustion behavior, and current feasibility routing, but they already diverge in ranking provenance, priority, and motive arithmetic in `crates/worldwake-ai/src/ranking.rs`. The original collapse-to-one-key scope was incorrect and is corrected here.
4. `StealItem` shares `MOVE_CARGO_OPS` with `MoveCargo` in `GoalKind::relevant_op_kinds()` but remains a separate dispatch shape because it already has distinct motive, priority, feasibility, candidate-generation, and trace semantics.
5. `CommodityPurpose` variants confirmed in `crates/worldwake-core/src/goal.rs`: `SelfConsume`, `RecipeInput(RecipeId)`, `Restock`.
6. `PunishmentKind` variants are defined in `crates/worldwake-core/src/crime.rs`, not in `goal.rs`. Live variants are `Fine { commodity: CommodityKind, amount: Quantity }` and `Exile { from_faction: EntityId }`.
7. S36 sibling tickets already exist in `tickets/`: `S36DECGOAREG-002` through `S36DECGOAREG-008`. This ticket remains the phase-1 prerequisite and must not silently pre-implement later-ticket migration work.
8. Existing focused coverage already proves some of the intended behavior:
   - `crates/worldwake-ai/src/goal_model.rs`: `ranked_goal_provenance_family_is_payload_aware`, `steal_goal_uses_move_cargo_ops_while_punishment_uses_live_verdict_actions`
   - `crates/worldwake-ai/src/exhaustion.rs`: `derive_invalidation_conditions_covers_every_live_goalkind_variant`, `acquire_restock_includes_coin_but_self_consume_does_not`
   This ticket still needs dedicated dispatch-key tests because the new type becomes the future declaration substrate.

## Architecture Check

1. The dispatch key stays AI-internal in `worldwake-ai`, derived from authoritative `GoalKind`. That is the clean boundary under P25: `GoalKind` stays truth, `GoalDispatchKey` becomes a derived dispatch read-model for registration.
2. The key must model actual dispatch-distinguishing shapes, not just today’s lowest-common-denominator intersections. Splitting `AcquireCommodity` into `AcquireSelfConsume`, `AcquireRecipeInput`, and `AcquireRestock` is cleaner than collapsing them now and re-splitting later because ranking already proves these are architecturally distinct branches.
3. No backwards-compatibility shim path. `GoalKindTag` remains for existing coarse-family consumers during the migration window, but this ticket does not alias it to the new key or treat it as a second registration substrate.
4. This phase remains intentionally structural. It is more beneficial than the current architecture because it establishes the correct declaration boundary without yet moving behavior. The cleaner long-term architecture is a single declaration table keyed by this type; that later consolidation belongs to sibling tickets, not to ad hoc early rewrites here.

## Verification Layers

1. Exhaustive key mapping -> compile-time match exhaustiveness in `GoalDispatchKey::from_goal_kind()`.
2. `AcquireCommodity` payload routing -> focused unit tests on the new key, proving `SelfConsume`, `RecipeInput(_)`, and `Restock` map to different dispatch keys.
3. `PunishAccused` payload routing -> focused unit tests on the new key, proving `Fine` and `Exile` map to different dispatch keys.
4. Whole-enum coverage -> focused unit test constructing one representative `GoalKind` per live variant and asserting mapping succeeds for all 21 variants.
5. Regression safety -> `cargo test -p worldwake-ai` plus `cargo clippy --workspace`.

## What to Change

### 1. New file: `crates/worldwake-ai/src/goal_dispatch_key.rs`

Define `GoalDispatchKey` enum with one variant per dispatch-distinguishing goal shape. Expected variants based on live code analysis:

- `ConsumeOwnedCommodity`
- `AcquireSelfConsume`
- `AcquireRecipeInput`
- `AcquireRestock` (distinct provenance, exhaustion behavior)
- `Sleep`, `Relieve`, `Wash`
- `EngageHostile`, `ReduceDanger`
- `TreatWounds`
- `ProduceCommodity`
- `SellCommodity`, `RestockCommodity`
- `MoveCargo`
- `LootCorpse`, `BuryCorpse`
- `ShareBelief`
- `ClaimOffice`, `SupportCandidateForOffice`
- `InvestigateViolation`
- `StealItem`
- `Accuse`
- `PunishFine`, `PunishExile` (distinct relevant_ops)

If implementation discovers additional live payload-sensitive static splits, add variants in-scope.

### 2. Exhaustive `GoalKind → GoalDispatchKey` lookup

Implement `GoalDispatchKey::from_goal_kind(goal: &GoalKind) -> GoalDispatchKey` (or a `From` impl) with an exhaustive match on all 21 `GoalKind` variants. No wildcard arm.

### 3. Register in `lib.rs`

Add `mod goal_dispatch_key;` and `pub use goal_dispatch_key::GoalDispatchKey;` to `crates/worldwake-ai/src/lib.rs`.

### 4. Unit tests

Add focused tests in the same file or a `#[cfg(test)]` module.

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_key.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — add module declaration)

## Out of Scope

- `GoalDispatchDeclaration` struct (ticket 002)
- Migrating any existing dispatch site to use the key (tickets 003–008)
- Modifying `GoalKindTag` or `GoalKind` definitions
- Candidate generation (`candidate_generation.rs`)
- `IntentionDomain` progress-op ownership (`agent_tick/frame.rs`)
- Any changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. `test_goal_dispatch_key_payload_sensitive_acquire_splits`: `AcquireCommodity::SelfConsume`, `AcquireCommodity::RecipeInput(_)`, and `AcquireCommodity::Restock` map to three distinct keys.
2. `test_goal_dispatch_key_payload_sensitive_punish_splits`: `PunishAccused::Fine` maps to `PunishFine`, `PunishAccused::Exile` maps to `PunishExile`.
3. `test_goal_dispatch_key_exhaustive_coverage`: One representative instance of every live `GoalKind` variant maps successfully.
4. `test_goal_dispatch_key_recipe_inputs_collapse_by_dispatch_shape`: different `RecipeInput(recipe_id)` payload values map to the same `AcquireRecipeInput` key because recipe id does not currently change static dispatch shape.
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Adding a `GoalKind` variant in `worldwake-core` without updating the dispatch-key lookup in `worldwake-ai` must fail compilation (exhaustive match, no wildcard).
2. The dispatch key is a derived AI-internal type — it must not appear in `worldwake-core`.
3. Zero behavioral change to any existing AI behavior.

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_dispatch_key.rs` — `test_goal_dispatch_key_payload_sensitive_acquire_splits`
Rationale: proves the key preserves the real static routing boundary across `SelfConsume`, `RecipeInput(_)`, and `Restock` instead of prematurely collapsing them.
2. `crates/worldwake-ai/src/goal_dispatch_key.rs` — `test_goal_dispatch_key_payload_sensitive_punish_splits`
Rationale: locks in the existing `PunishAccused` `Fine` vs `Exile` operator split as a declaration-key distinction.
3. `crates/worldwake-ai/src/goal_dispatch_key.rs` — `test_goal_dispatch_key_recipe_inputs_collapse_by_dispatch_shape`
Rationale: proves the key distinguishes dispatch shape, not irrelevant payload identity; different recipe ids should not create different declaration keys.
4. `crates/worldwake-ai/src/goal_dispatch_key.rs` — `test_goal_dispatch_key_exhaustive_coverage`
Rationale: gives a focused regression surface for the full live `GoalKind` enum and complements compile-time exhaustiveness with explicit representative coverage.

### Commands

1. `cargo test -p worldwake-ai test_goal_dispatch_key_payload_sensitive_acquire_splits`
2. `cargo test -p worldwake-ai test_goal_dispatch_key_payload_sensitive_punish_splits`
3. `cargo test -p worldwake-ai test_goal_dispatch_key_recipe_inputs_collapse_by_dispatch_shape`
4. `cargo test -p worldwake-ai test_goal_dispatch_key_exhaustive_coverage`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed: added `crates/worldwake-ai/src/goal_dispatch_key.rs` with an exhaustive `GoalDispatchKey` enum plus `GoalKind -> GoalDispatchKey` conversion, re-exported it from `crates/worldwake-ai/src/lib.rs`, and added focused unit tests for acquire-purpose splits, punish splits, recipe-input collapse-by-shape, and full live-enum coverage.
- Deviations from original plan: the ticket was corrected before implementation to treat `AcquireCommodity::RecipeInput(_)` as a distinct dispatch key rather than collapsing it with `SelfConsume`, because live ranking dispatch in `crates/worldwake-ai/src/ranking.rs` already proves those are different static routing branches.
- Verification results: `cargo test -p worldwake-ai test_goal_dispatch_key_payload_sensitive_acquire_splits`, `cargo test -p worldwake-ai test_goal_dispatch_key_payload_sensitive_punish_splits`, `cargo test -p worldwake-ai test_goal_dispatch_key_recipe_inputs_collapse_by_dispatch_shape`, `cargo test -p worldwake-ai test_goal_dispatch_key_exhaustive_coverage`, `cargo test -p worldwake-ai`, and `cargo clippy --workspace` all passed on 2026-03-29.
