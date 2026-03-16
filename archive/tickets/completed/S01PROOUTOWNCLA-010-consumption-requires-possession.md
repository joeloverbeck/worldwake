# S01PROOUTOWNCLA-010: Consumption actions must require possession, not just control

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — needs action definitions, golden trade tests
**Deps**: S01PROOUTOWNCLA-004 (harvest ownership creates actor-owned ground lots)

## Problem

With production output ownership (S01PROOUTOWNCLA-004), `can_exercise_control(actor, lot)` succeeds for actor-owned unpossessed ground lots. Consumption actions (eat, drink, wash) use `Precondition::ActorCanControlTarget` and `TargetSpec::EntityAtActorPlace` — which means agents can now eat, drink, and wash directly from owned ground lots without picking them up first.

This collapses the distinction between **ownership** and **physical access**, violating Principle 22: "Possession is not ownership. Ownership is not permission. Permission is not capability."

Concrete consequences:
- Agents eat harvested food directly from the ground instead of picking it up first
- Merchants consume trade stock on-site instead of carrying it to market
- The pickup→carry→consume flow that produces visible aftermath and interruption windows is bypassed
- The AI planner rationally skips pickup when direct consumption is available, eliminating the progress barriers the spec (S01) requires

## Assumption Reassessment (2026-03-16)

1. `eat_preconditions()` at `needs_actions.rs:126-141` uses `ActorCanControlTarget(0)` and `EntityAtActorPlace { kind: ItemLot }` — confirmed
2. `drink_preconditions()` at `needs_actions.rs:143-158` uses the same pattern — confirmed
3. `wash_preconditions()` at `needs_actions.rs:160-175` uses the same pattern — confirmed
4. `Precondition::TargetDirectlyPossessedByActor(u8)` already exists in `action_semantics.rs:67` — confirmed
5. `TargetSpec::EntityDirectlyPossessedByActor { kind }` already exists in `action_semantics.rs:34` — confirmed
6. Both are already implemented in `action_validation.rs:107-109` (authoritative) and `affordance_query.rs:226-228, 269-273` (belief) — confirmed
7. The golden trade tests `golden_merchant_restock_return_stock` and `golden_merchant_restock_return_stock_replays_deterministically` fail because the merchant eats harvested apples directly from ground instead of carrying them — confirmed

## Architecture Check

1. This change enforces Principle 22 at the action precondition level: ownership grants legal authority, but physical manipulation (eating, drinking, washing) requires physical access (possession). The infrastructure (`TargetDirectlyPossessedByActor`, `EntityDirectlyPossessedByActor`) already exists — this ticket only wires it in.
2. No backwards-compatibility shims. The old `ActorCanControlTarget` + `EntityAtActorPlace` combination is replaced, not aliased.
3. This preserves the spec's Design Goal 2 (progress barriers after production) and Design Goal 4 (custody separate from ownership): agents must pick up owned goods before consuming them, creating visible aftermath and interruption windows.

## What to Change

### 1. Update eat action definition

In `needs_actions.rs`, change `eat_preconditions()`:
- Replace `Precondition::ActorCanControlTarget(0)` with `Precondition::TargetDirectlyPossessedByActor(0)`

Change the target spec for "eat" in `register_def`:
- Replace `TargetSpec::EntityAtActorPlace { kind: EntityKind::ItemLot }` with `TargetSpec::EntityDirectlyPossessedByActor { kind: EntityKind::ItemLot }`

### 2. Update drink action definition

Same changes as eat in `drink_preconditions()` and the drink target spec.

### 3. Update wash action definition

Same changes as eat/drink in `wash_preconditions()` and the wash target spec.

### 4. Update golden trade tests

The `golden_merchant_restock_return_stock` and `golden_merchant_restock_return_stock_replays_deterministically` tests should pass after the precondition fix, since the merchant will be forced to pick up apples before consuming. If the seed needs adjustment due to delta changes, find a working seed and document it.

### 5. Verify golden production test

The updated `golden_capacity_constrained_ground_lot_pickup` test (from S01PROOUTOWNCLA-004) should continue to pass. With the possession requirement restored, the agent must pick up before eating, which may re-enable the split-pickup path. If so, the test can be strengthened to validate the split-pickup invariant again.

## Files to Touch

- `crates/worldwake-systems/src/needs_actions.rs` (modify — update preconditions and target specs for eat, drink, wash)
- `crates/worldwake-ai/tests/golden_trade.rs` (modify — verify/update seed if needed)
- `crates/worldwake-ai/tests/golden_production.rs` (modify — verify/strengthen if split-pickup returns)

## Out of Scope

- Changing `can_exercise_control()` semantics (correct as-is: ownership grants authority)
- Changing pickup validation (S01PROOUTOWNCLA-007, -008)
- Adding new precondition types (all needed infrastructure exists)
- Modifying the AI planner's goal priorities

## Acceptance Criteria

### Tests That Must Pass

1. Eat action rejects unpossessed owned ground lot (precondition fails)
2. Eat action accepts possessed lot (precondition passes)
3. Drink action rejects unpossessed owned ground lot
4. Drink action accepts possessed lot
5. Wash action rejects unpossessed water lot on ground
6. Wash action accepts possessed water lot
7. Agent must pick up food before eating in the AI loop (golden production test)
8. Merchant picks up apples and carries them for trade (golden trade tests)
9. Existing suite: `cargo test -p worldwake-systems`
10. Full suite: `cargo test --workspace`

### Invariants

1. Ownership grants legal authority but not physical manipulation capability
2. Physical consumption (eat, drink, wash) requires possession
3. Progress barriers after production are preserved: agents must pick up before consuming
4. Principle 22: possession, ownership, permission, and capability remain distinct

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` — add tests for eat/drink/wash rejecting unpossessed owned lots
2. `crates/worldwake-ai/tests/golden_trade.rs` — verify merchant trade loop works with possession requirement
3. `crates/worldwake-ai/tests/golden_production.rs` — verify/strengthen capacity-constrained scenario

### Commands

1. `cargo test -p worldwake-systems eat drink wash`
2. `cargo test -p worldwake-ai --test golden_trade`
3. `cargo test -p worldwake-ai --test golden_production`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

**Completion date**: 2026-03-16

**What changed**:
- `needs_actions.rs`: Replaced `ActorCanControlTarget(0)` with `TargetDirectlyPossessedByActor(0)` and `EntityAtActorPlace` with `EntityDirectlyPossessedByActor` in eat/drink/wash preconditions and target specs. Also removed redundant `TargetAtActorPlace(0)` (possession implies presence). Added 6 new possession-requirement tests.
- `e09_needs_integration.rs`: Updated `scheduler_driven_care_actions` to use directly possessed bread; removed unused `add_controlled_bread_in_satchel` helper.
- `search.rs`: Updated 5 planner search tests — local food now requires possession for 1-step eat; remote food plans correctly require travel → pickup → eat (3 steps); added carry capacity and commodity quantities to test views; adjusted beam width/budget assertions.

**Deviations from plan**: None. No seed adjustments needed for golden tests. The golden production and trade tests passed without modification.

**Verification**: 1713 tests pass, 0 failures, 0 clippy warnings. All 32 golden tests pass (21 production + 11 trade).
