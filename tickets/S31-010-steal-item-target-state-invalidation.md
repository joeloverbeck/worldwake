# S31-010: Align `StealItem` exhaustion invalidation with target item state

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` exhaustion invalidation contract for theft goals
**Deps**: `archive/specs/S31-goal-aware-exhaustion-invalidation.md`, `archive/tickets/completed/S36DECGOAREG-006-invalidation-strategy.md`

## Problem

`GoalKind::StealItem { target_item }` is a target-entity goal, but the live exhaustion invalidation contract only records `PositionChanged` and `TargetDead(target_item)`. That is too weak for the actual theft architecture: steal legality and goal satisfiability depend on the concrete state of that exact item lot, including where it is, whether it is already possessed or contained, and whether the actor can now lawfully control it instead of stealing it. As a result, an exhausted theft opportunity can stay cached after the target item has moved, been picked up, been containerized, or otherwise become a different world problem.

## Assumption Reassessment (2026-03-29)

1. Live goal under test: `GoalKind::StealItem { target_item }`. Candidate generation emits it only for locally observed `ItemLot` entities that are not owned by the actor, are not lawfully controllable by the actor, are not directly possessed, are not in a container, and fit remaining carry capacity (`crates/worldwake-ai/src/candidate_generation.rs`, theft emitter around `GoalKind::StealItem { target_item: item }`).
2. The exact authoritative legality surface for the committed `steal` action is `validate_steal()` in `crates/worldwake-systems/src/transport_actions.rs`. Live steal legality depends on the target item being at the actor's place, being an `ItemLot`, not being containerized, being owned by someone else, not being lawfully controllable by the actor, not already possessed, not reserved, and still fitting carry capacity.
3. The exact planner-side semantic surface is also target-entity-based, not kind-based:
   - goal satisfaction is `state.direct_possessor(target_item) == Some(actor)` in `crates/worldwake-ai/src/goal_model.rs`
   - goal-relevant places are `state.effective_place(target_item)` in `crates/worldwake-ai/src/goal_model.rs`
   - planner transition for theft is `PlannerTransitionKind::StealGroundLot`, which rejects targets that are possessed, containerized, at a different place, zero-quantity, or too heavy in `crates/worldwake-ai/src/planner_ops.rs`.
4. Shared abstraction boundary under audit: the exhaustion invalidation contract for `GoalKind::StealItem { target_item }` must track the same concrete target-item state that candidate generation, planner search, and authoritative `validate_steal()` already rely on. The relevant boundary is `crates/worldwake-ai/src/exhaustion.rs::derive_invalidation_conditions()` plus `GoalBeliefView` target queries such as `effective_place`, `direct_possessor`, `direct_container`, and `can_control`.
5. The live invalidation route introduced by S36 is now declaration-owned, but the semantic payload for `StealItem` is still `InvalidationStrategy::PositionAndTargetDead` in `crates/worldwake-ai/src/goal_dispatch_decl.rs`. S36 fixed routing ownership, not the correctness of the theft invalidation contract.
6. `ExhaustionInvalidationCondition::UniqueItemChanged(UniqueItemKind)` already exists in `crates/worldwake-ai/src/exhaustion.rs`, but that is not the right contract for this bug. `StealItem` targets a concrete `ItemLot` entity, not a unique-item kind summary. Replacing the current logic with `UniqueItemChanged(...)` would violate FOUNDATIONS principles around persistent identity and concrete state by collapsing one target entity into a coarse kind count.
7. This is primarily an `agent_tick` runtime correctness issue backed by a focused lower-layer contract. A pure unit test is not sufficient on its own because the bug matters when an exhausted entry survives a later lawful change in the target item's world state. Focused runtime coverage should prove entry invalidation after the target item changes state.
8. Adjacent contradiction: the broader `PositionAndTargetDead` strategy now groups `StealItem` with corpse/social/accusation goals for routing convenience, but those goals do not share the same invalidation substrate. That grouping is acceptable as an interim routing family only because the per-goal helper can still branch on concrete payload. This ticket should correct the `StealItem` behavior without forcing unrelated goals into a broader semantic rewrite.
9. Mismatch + correction: the live archived S31 mapping and the current S36 invalidation family both treat `StealItem` as "position + target death." The corrected scope is to make `StealItem` invalidate on target-item state changes that materially alter theft legality or satisfaction, while keeping other target-bound goals unchanged unless reassessment proves they share the same substrate.

## Architecture Check

1. The clean architecture is a target-entity state invalidation for `StealItem`, not a coarse `UniqueItemChanged` count and not a silent fallback to "rely on generic position change." FOUNDATIONS P3/P4 require the invalidation contract to track the concrete entity whose state actually changed. FOUNDATIONS P7/P12 also support using only the actor's lawful belief-view queries about that item, not any global shortcut.
2. The recommended implementation is to introduce a theft-appropriate invalidation condition and baseline snapshot for the target item's theft-relevant state, then derive that condition only for `GoalKind::StealItem`. No compatibility aliasing, no dual paths, and no weakening of the existing target-entity goal identity.

## Verification Layers

1. `StealItem` derives the corrected target-item invalidation condition instead of `TargetDead(target_item)` alone -> focused `exhaustion.rs` unit test.
2. The new condition fires when the target item moves, becomes possessed, becomes containerized, or becomes lawfully controllable / otherwise unavailable according to the chosen contract -> focused `condition_changed` unit tests in `exhaustion.rs`.
3. An exhausted theft opportunity is cleared after another actor changes the target item state in a way that makes the old theft plan stale -> focused `agent_tick` runtime test covering exhaustion-cache invalidation, not just a static unit test.
4. Existing target-bound non-theft goals (`LootCorpse`, `BuryCorpse`, `ShareBelief`, `Accuse`) retain their current invalidation behavior unless this ticket explicitly broadens scope after reassessment -> focused unit test or unchanged existing coverage.
5. Strongest lower-layer proof surface is the target-item invalidation condition itself plus a focused runtime exhaustion-cache test; a golden scenario is optional and should not replace those lower-layer assertions.
6. This is not a single-layer ticket: the core contract lives in `exhaustion.rs`, but the correctness claim matters because `agent_tick` retains or clears exhausted opportunities based on that contract.

## What to Change

### 1. Define a theft-appropriate invalidation contract

Reassess the smallest robust condition type for target-item theft invalidation. Prefer an entity-specific target-item state condition over any kind-count summary. The condition should be derived from the exact theft substrate already used by candidate generation / planner transition / `validate_steal()`.

### 2. Snapshot the relevant target-item baseline state

Extend `ExhaustionBaseline` only as needed to compare the theft-relevant state of the bound target item. Keep the snapshot concrete and local to the actor's lawful `GoalBeliefView`.

### 3. Update `StealItem` invalidation derivation

Change `derive_invalidation_conditions()` so `GoalKind::StealItem { target_item }` derives the new target-item invalidation contract instead of relying on `TargetDead(target_item)` alone.

### 4. Add focused runtime proof

Add a focused runtime test showing that an exhausted theft opportunity is invalidated when the bound target item changes in a way that makes the old theft search stale, for example another actor picks it up or moves it away.

## Files to Touch

- `crates/worldwake-ai/src/exhaustion.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` or `crates/worldwake-ai/src/agent_tick/planning.rs` test module (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify only if the strategy family name or per-goal helper split needs clarification)

## Out of Scope

- Reworking theft candidate generation, ranking, or planner operators beyond what is required to align the invalidation contract
- Replacing other target-bound goal invalidation contracts unless reassessment shows they are incorrect for the same reason
- Changing `GoalKind::StealItem` into a kind-based or summary-based goal

## Acceptance Criteria

### Tests That Must Pass

1. A focused `exhaustion.rs` test proves `StealItem` no longer derives only `TargetDead(target_item)` as its theft-state invalidation contract.
2. A focused runtime test proves an exhausted theft opportunity is cleared when the target item's theft-relevant state changes after exhaustion.
3. Existing suite: `cargo test -p worldwake-ai -- steal`
4. Existing suite: `cargo test -p worldwake-ai -- invalidation`
5. Full workspace: `cargo test --workspace`

### Invariants

1. `StealItem` invalidation tracks the concrete bound item entity, not an abstract unique-item kind or aggregate count.
2. The invalidation substrate stays aligned with the same local world facts that make steal candidate emission, planner transition, and authoritative steal validation lawful or unlawful.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/exhaustion.rs` — prove the new `StealItem` invalidation condition and its `condition_changed` behavior over concrete target-item state changes.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` or `crates/worldwake-ai/src/agent_tick/planning.rs` test module — prove an exhausted theft opportunity is actually cleared by a later target-item state change in runtime.

### Commands

1. `cargo test -p worldwake-ai -- steal`
2. `cargo test -p worldwake-ai -- invalidation`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace`
