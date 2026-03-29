# S31-010: Align `StealItem` exhaustion invalidation with target item state

**Status**: COMPLETED
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
4. Shared abstraction boundary under audit: the exhaustion invalidation contract for `GoalKind::StealItem { target_item }` must track the same concrete target-item state that candidate generation, planner search, and authoritative `validate_steal()` already rely on. The relevant boundary is `crates/worldwake-ai/src/exhaustion.rs::derive_invalidation_conditions()` plus `GoalBeliefView` target queries such as `effective_place`, `direct_possessor`, `direct_container`, `believed_owner_of`, `load_of_entity`, and `can_control`.
5. The live invalidation route introduced by S36 is now declaration-owned, but the semantic payload for `StealItem` is still `InvalidationStrategy::PositionAndTargetDead` in `crates/worldwake-ai/src/goal_dispatch_decl.rs`. S36 fixed routing ownership, not the correctness of the theft invalidation contract. Reassessment also shows the declaration family itself is now the wrong abstraction boundary for theft: `LootCorpse`, `BuryCorpse`, `ShareBelief`, and `Accuse` are still legitimately "position + target death" goals, but `StealItem` is target-item-state-sensitive and should no longer share that family.
6. `ExhaustionInvalidationCondition::UniqueItemChanged(UniqueItemKind)` already exists in `crates/worldwake-ai/src/exhaustion.rs`, but that is not the right contract for this bug. `StealItem` targets a concrete `ItemLot` entity, not a unique-item kind summary. Replacing the current logic with `UniqueItemChanged(...)` would violate FOUNDATIONS principles around persistent identity and concrete state by collapsing one target entity into a coarse kind count.
7. Existing coverage gap classification after reassessment:
   - focused/unit coverage exists for generic invalidation families in `crates/worldwake-ai/src/exhaustion.rs` and dispatch-family coverage exists in `crates/worldwake-ai/src/goal_dispatch_decl.rs`
   - focused runtime coverage exists for generic exhaustion-cache invalidation in `crates/worldwake-ai/src/agent_tick/tests.rs`
   - there is no theft-specific focused unit coverage proving `StealItem` derives a target-item-state condition, and no theft-specific focused runtime coverage proving a stale exhausted theft entry is cleared after the item becomes lawfully controllable / possessed / otherwise unavailable
   - `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_goal_invalidation_by_another_agent` is unrelated commodity invalidation coverage, not theft coverage
8. This is primarily an `agent_tick` runtime correctness issue backed by a focused lower-layer contract. A pure unit test is not sufficient on its own because the bug matters when an exhausted entry survives a later lawful change in the target item's world state. Focused runtime coverage should prove entry invalidation after the target item changes state. Because the runtime path must expose `steal`, local needs-only harness setup is insufficient; full action registries are required.
9. Adjacent contradiction: the broader `PositionAndTargetDead` strategy now groups `StealItem` with corpse/social/accusation goals for routing convenience, but those goals do not share the same invalidation substrate. Reassessment classifies that grouping as a required consequence of the intended change, not a separate cleanup. This ticket should split theft into its own declaration-owned strategy while keeping the other target-bound goals unchanged.
10. Mismatch + correction: the live archived S31 mapping and the current S36 invalidation family both treat `StealItem` as "position + target death." The corrected scope is to give `StealItem` its own declaration strategy plus target-item-state invalidation contract, while keeping other target-bound goals on their current `PositionAndTargetDead` path unless reassessment proves they share the same substrate.

## Architecture Check

1. The clean architecture is a declaration-owned theft-specific invalidation strategy plus a target-entity state invalidation condition for `StealItem`, not a coarse `UniqueItemChanged` count and not a silent fallback to "rely on generic position change." FOUNDATIONS P3/P4 require the invalidation contract to track the concrete entity whose state actually changed. FOUNDATIONS P7/P12 also support using only the actor's lawful belief-view queries about that item, not any global shortcut.
2. The recommended implementation is to introduce a theft-appropriate invalidation condition and baseline snapshot for the target item's theft-relevant state, and route `GoalKind::StealItem` through its own `InvalidationStrategy` in `crates/worldwake-ai/src/goal_dispatch_decl.rs`. This is cleaner than keeping a semantically mixed `PositionAndTargetDead` family with hidden special-cases. No compatibility aliasing, no dual paths, and no weakening of the existing target-entity goal identity.

## Verification Layers

1. `StealItem` routes through its own declaration-owned invalidation strategy instead of the shared `PositionAndTargetDead` family -> focused `goal_dispatch_decl.rs` or `exhaustion.rs` unit test.
2. `StealItem` derives the corrected target-item invalidation condition instead of `TargetDead(target_item)` alone -> focused `exhaustion.rs` unit test.
3. The new condition fires when the target item moves, becomes possessed, becomes containerized, changes owner/control relation, changes load relative to actor capacity, or otherwise becomes unavailable according to the chosen contract -> focused `condition_changed` unit tests in `exhaustion.rs`.
4. An exhausted theft opportunity is cleared after another actor changes the target item state in a way that makes the old theft plan stale -> focused `agent_tick` runtime test covering exhaustion-cache invalidation, not just a static unit test.
5. Existing target-bound non-theft goals (`LootCorpse`, `BuryCorpse`, `ShareBelief`, `Accuse`) retain their current invalidation behavior -> focused unit assertion where needed, otherwise unchanged existing coverage.
6. Strongest lower-layer proof surface is the target-item invalidation condition itself plus a focused runtime exhaustion-cache test; a golden scenario is optional and should not replace those lower-layer assertions.

## What to Change

### 1. Split theft out of the shared target-death strategy

Add a dedicated `InvalidationStrategy` for `GoalKind::StealItem` in `crates/worldwake-ai/src/goal_dispatch_decl.rs` so declaration ownership stays semantically honest. Do not keep theft under the shared `PositionAndTargetDead` family and hide the real boundary in a helper branch.

### 2. Define a theft-appropriate invalidation contract

Reassess the smallest robust condition type for target-item theft invalidation. Prefer an entity-specific target-item state condition over any kind-count summary. The condition should be derived from the exact theft substrate already used by candidate generation / planner transition / `validate_steal()`.

### 3. Snapshot the relevant target-item baseline state

Extend `ExhaustionBaseline` only as needed to compare the theft-relevant state of the bound target item. Keep the snapshot concrete and local to the actor's lawful `GoalBeliefView`.

### 4. Update `StealItem` invalidation derivation

Change `derive_invalidation_conditions()` so `GoalKind::StealItem { target_item }` derives the new target-item invalidation contract through the new theft-specific declaration strategy instead of relying on `TargetDead(target_item)` alone.

### 5. Add focused runtime proof

Add a focused runtime test showing that an exhausted theft opportunity is invalidated when the bound target item changes in a way that makes the old theft search stale, for example another actor picks it up or moves it away.

## Files to Touch

- `crates/worldwake-ai/src/exhaustion.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` or `crates/worldwake-ai/src/agent_tick/planning.rs` test module (modify)

## Out of Scope

- Reworking theft candidate generation, ranking, or planner operators beyond what is required to align the invalidation contract
- Replacing other target-bound goal invalidation contracts unless reassessment shows they are incorrect for the same reason
- Changing `GoalKind::StealItem` into a kind-based or summary-based goal

## Acceptance Criteria

### Tests That Must Pass

1. A focused dispatch/invalidation test proves `StealItem` no longer routes through the shared `PositionAndTargetDead` strategy.
2. A focused `exhaustion.rs` test proves `StealItem` no longer derives only `TargetDead(target_item)` as its theft-state invalidation contract.
3. A focused runtime test proves an exhausted theft opportunity is cleared when the target item's theft-relevant state changes after exhaustion.
4. Existing suite: `cargo test -p worldwake-ai -- steal`
5. Existing suite: `cargo test -p worldwake-ai -- invalidation`
6. Full workspace: `cargo test --workspace`

### Invariants

1. `StealItem` invalidation tracks the concrete bound item entity, not an abstract unique-item kind or aggregate count.
2. `StealItem` declaration routing names a theft-specific invalidation substrate instead of sharing a semantically incorrect target-death family.
3. The invalidation substrate stays aligned with the same local world facts that make steal candidate emission, planner transition, and authoritative steal validation lawful or unlawful.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_dispatch_decl.rs::test_invalidation_strategies_match_payload_sensitive_and_shared_families` — modified to prove `StealItem` no longer shares the `PositionAndTargetDead` invalidation family with corpse/social/accusation goals.
2. `crates/worldwake-ai/src/exhaustion.rs::steal_item_derives_target_state_condition_instead_of_target_dead` — added to prove theft exhaustion now derives a target-item-state condition and snapshots the bound lot’s theft-relevant state.
3. `crates/worldwake-ai/src/exhaustion.rs::condition_changed_steal_target_detects_control_and_possession_delta` — added to prove theft invalidation fires on concrete target-state changes such as lawful control, possession, containment, and carry-capacity availability changes.
4. `crates/worldwake-ai/src/agent_tick/tests.rs::exhausted_steal_goal_is_cleared_when_target_becomes_lawfully_controllable` — added to prove the runtime exhaustion cache actually clears a stale exhausted `StealItem` entry after the target becomes lawfully controllable.

### Commands

1. `cargo test -p worldwake-ai -- steal`
2. `cargo test -p worldwake-ai -- invalidation`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-29
- What changed:
  - split `StealItem` onto its own declaration-owned `InvalidationStrategy` instead of reusing `PositionAndTargetDead`
  - added a theft-specific exhaustion invalidation condition plus concrete baseline snapshot for bound target-lot state
  - wired invalidation checks to compare live belief-view target state against that snapshot
  - added focused unit and runtime coverage for theft invalidation routing and stale-cache clearing
- Deviations from original plan:
  - the initial ticket allowed keeping theft inside the shared target-death family with a helper branch; reassessment and implementation corrected that architecture and made the declaration split required
  - the runtime proof landed in `crates/worldwake-ai/src/agent_tick/tests.rs` rather than `planning.rs`
- Verification results:
  - `cargo test -p worldwake-ai -- steal` passed
  - `cargo test -p worldwake-ai -- invalidation` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
