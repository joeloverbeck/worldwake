# HARPREE14-011: Multi-recipe golden e2e scenario

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: AI candidate/search semantics
**Deps**: HARPREE14-007 (benefits from named recipe lookup, soft dep)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-D01

## Problem

The golden e2e only tests a single harvest recipe (`Harvest Apples` via orchard). Multi-recipe registration and craft-path behavior are untested at the integration level. This is a coverage gap for the AI's ability to operate correctly when multiple recipes coexist and a hunger response must go through a crafted food path instead of a direct harvest.

## Assumption Reassessment (2026-03-11)

1. Golden e2e exists at `crates/worldwake-ai/tests/golden_e2e.rs` -- confirmed
2. Current golden harness registers only `Harvest Apples` and seeds every agent with `KnownRecipes::with([RecipeId(0)])` -- confirmed
3. `RecipeRegistry` already supports multiple registration and string-keyed lookup from HARDEN-B01 / HARPREE14-007 -- confirmed
4. `KnownRecipes` component exists for agents -- confirmed
5. The original proposed chain `Harvest Grain -> Bake Bread -> consume bread` is NOT currently a valid assumption:
   - `CommodityKind::Grain` is itself consumable food, so a hungry agent can satisfy hunger directly from grain
   - candidate generation only exposes a craft recipe when its required inputs are already locally controlled
   - planner/search treat harvest and craft as materialization barriers and do not plan across them as a single recursive production chain
6. Because of (5), this ticket must not promise a true harvest-to-craft chain without first changing planner architecture in a separate ticket.
7. While implementing the corrected craft-path scenario, another real gap surfaced: crafted outputs materialize as unpossessed ground lots, but `AcquireCommodity` evidence generation does not currently surface local unpossessed lots as reacquirable evidence. Without fixing that, a craft-path golden scenario cannot close the loop from craft -> pick up -> consume.
8. To keep the golden scenario deterministic under current commodity semantics, the crafted-food recipe used by the test should consume a non-edible carried input. This isolates the craft path instead of racing against direct self-consumption of an edible intermediate such as grain.

## Architecture Check

1. A new golden scenario is still the right vehicle, but it must target behavior the current architecture actually supports.
2. The clean coverage target is: multiple recipes registered, the actor knows multiple recipes, a craft recipe is the only locally viable hunger path, and deterministic replay/conservation still hold.
3. Changing the planner to support recursive harvest->materialize->pickup->craft chains would be architecturally meaningful, but that is larger than this hardening ticket and should not be smuggled in as "just a test."
4. The minimal production changes that fit this ticket are:
   - candidate generation must treat local unpossessed item lots as acquisition evidence
   - planner search must treat `pick_up` as a concrete acquisition transition for acquisition-style goals so a local ground lot can become owned inventory in hypothetical planning

## What to Change

### 1. Add new test scenario to golden_e2e.rs

Create a new `#[test]` function covering a supported multi-recipe craft path:
- Multiple recipes registered: `Harvest Apples`, `Harvest Grain`, `Bake Bread`
- An agent with `KnownRecipes` containing all registered recipes
- World setup where the actor already has the crafted-food recipe's non-edible input and has access to a mill workstation
- No direct edible lot in inventory at start
- Optional extra registered harvest workstations may exist, but the scenario must not rely on unsupported recursive harvest->craft planning

### 2. Assert multi-recipe craft behavior

Verify the agent:
- Crafts bread at the mill using the carried non-edible input
- Re-acquires the crafted bread from the ground via the normal transport path
- Consumes bread to satisfy hunger
- Does so with multiple recipes present in the registry / known-recipes set, proving unrelated recipes do not break the craft path

### 2b. Close the craft materialization loop

Add or strengthen candidate-generation coverage so a local unpossessed lot of a requested commodity emits `AcquireCommodity` evidence. This is required for crafted outputs to be reachable after they materialize on the ground.

### 3. Assert conservation invariants

Assert the existing lot + authoritative conservation invariants at phase-appropriate checkpoints throughout the scenario.
- Example: before crafting, carried input totals match the seeded input quantity and bread totals are zero
- After craft materialization, carried input totals drop to zero and bread totals match the crafted output
- After consumption, bread totals return to zero

### 4. Assert deterministic replay

Record state hashes for the new scenario and verify replay produces identical hashes for the same seed.

## Files to Touch

- `crates/worldwake-ai/tests/golden_e2e.rs` (modify -- add new test scenario)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify -- treat local unpossessed lots as acquisition evidence)
- `crates/worldwake-ai/src/search.rs` (modify -- let local pickup resolve as a concrete acquisition transition in search)

## Out of Scope

- Modifying the existing golden e2e scenarios beyond shared helper extraction
- Changing commodity balance so `Grain` stops being edible
- Adding recursive production planning across materialization barriers
- Changing planner/search semantics to support recursive multi-step production across materialization barriers
- Adding new recipe types to production code

## Acceptance Criteria

### Tests That Must Pass

1. New golden scenario proves a hungry agent with multiple known recipes can craft bread from carried non-edible inputs, pick the bread up after materialization, and then consume it
2. Conservation invariants hold at every phase checkpoint used by the scenario
3. Deterministic replay for the new scenario produces identical hashes
4. All existing golden e2e scenarios still pass
5. `cargo clippy --workspace` passes with no new warnings
6. A focused candidate-generation test covers local unpossessed commodity lots as acquisition evidence

### Invariants

1. Production changes stay minimal and localized to candidate-generation evidence plus search-state transitions for local pickup
2. Existing golden e2e scenarios remain behaviorally unchanged
3. Conservation invariants (live lots + authoritative totals) hold at the scenario's explicit pre-craft, post-craft, and post-consume checkpoints
4. Deterministic replay is verified for the new scenario
5. The ticket must explicitly document that recursive harvest->craft hunger chains remain a separate architectural gap

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_e2e.rs` -- new multi-recipe craft-path golden scenario
2. `crates/worldwake-ai/tests/golden_e2e.rs` -- any shared helper updates required to seed multi-recipe known-recipes sets or deterministic replay helpers
3. `crates/worldwake-ai/src/candidate_generation.rs` -- unit test for local unpossessed lot acquisition evidence
4. `crates/worldwake-ai/src/search.rs` -- unit test for `AcquireCommodity` planning against a local ground lot

### Commands

1. `cargo test -p worldwake-ai --test golden_e2e <new-test-name>` (targeted)
2. `cargo test -p worldwake-ai --test golden_e2e` (full e2e suite)
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - Added `golden_multi_recipe_craft_path` to `crates/worldwake-ai/tests/golden_e2e.rs`
  - Added helper coverage for multi-recipe known-recipe seeding and deterministic replay in the golden harness
  - Fixed `AcquireCommodity` candidate generation so local unpossessed lots count as acquisition evidence
  - Fixed search-state transitions so `pick_up` can satisfy an acquire goal against a local ground lot
  - Added focused unit tests for both of those production fixes
- Deviations from original plan:
  - Did not implement the originally proposed `Harvest Grain -> Bake Bread -> consume bread` chain because current architecture does not support recursive production across materialization barriers, and `Grain` is already edible
  - The final golden scenario uses a crafted-food recipe with a non-edible carried input so the test isolates the craft path deterministically
  - Production code changes were required even though the original ticket claimed test-only scope
- Verification results:
  - `cargo test -p worldwake-ai --test golden_e2e`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
