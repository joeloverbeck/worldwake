# HARPREE14-016: Ground-lot acquisition coverage and constrained transport

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes -- AI goal/search test coverage, golden e2e hardening
**Deps**: HARPREE14-015 (planner-owned hypothetical transitions), HARPREE14-011 (multi-recipe craft-path golden scenario, archived)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-D01, Principle 3, Principle 6

## Problem

The recent multi-recipe golden scenario proved that local ground lots matter, but current coverage is still too narrow:

1. Candidate generation only gained one hunger-driven local-lot regression test.
2. The golden scenario only covers the unconstrained case where the crafted output fits easily.
3. There is still no deterministic end-to-end proof that local-lot transport remains correct when carry-capacity constraints force a partial pickup before travel.

That leaves a real blind spot: acquisition behavior around local concrete inventory carriers can regress again without a strong test net.

## Assumption Reassessment (2026-03-12)

1. `candidate_generation.rs` includes a local unpossessed food-lot test for `AcquireCommodity { purpose: SelfConsume }` -- confirmed.
2. `golden_multi_recipe_craft_path` in `crates/worldwake-ai/tests/golden_e2e.rs` covers multi-recipe craft -> pickup -> consume in the unconstrained case -- confirmed.
3. The original assumption that planner-facing constrained pickup coverage is missing is false:
   - `cargo_search_handles_partial_pickup_split_before_travel`
   - `authoritative_partial_cargo_pickup_can_reach_goal_satisfaction`
   - `build_successor_uses_transition_metadata_for_partial_pickup`
   - runtime continuity coverage in `agent_tick.rs`
   already exercise split-lot planning and continuation semantics.
4. The original assumption that candidate generation should prove non-self-consume `AcquireCommodity` purposes is architecturally mismatched:
   - current candidate generation emits `AcquireCommodity` only for direct acquisition pressure such as self-consumption
   - enterprise transport/restock paths are modeled through `RestockCommodity` and `MoveCargo`
   - `CommodityPurpose::{Restock, RecipeInput, Treatment}` exist in shared goal/search semantics, but they are not currently top-level candidate-generation outputs
5. The remaining gap is therefore narrower and cleaner:
   - add shared goal/search coverage showing non-self-consume `AcquireCommodity` local-lot pickup still works where that goal kind is used directly
   - add a true end-to-end constrained local-lot transport scenario at runtime
6. Carry capacity remains a foundational concrete-state constraint and should be represented in end-to-end coverage whenever local-lot transport is under test -- confirmed.

## Architecture Check

1. This ticket is about coverage, not new planner behavior. The existing split-lot architecture in `planner_ops.rs` and `search.rs` is already the correct shape: concrete, deterministic, and free of compatibility shims.
2. It would be worse architecture to force candidate generation to emit new top-level `AcquireCommodity` variants just to satisfy a test idea. That would widen behavior, not harden it.
3. The clean long-term architecture is:
   - keep acquisition-purpose sharing in goal/search semantics
   - keep enterprise logistics expressed as `RestockCommodity` plus `MoveCargo`
   - harden those paths with tests at the layer where they actually live
4. No backwards-compatibility shims are needed; this ticket should strengthen tests and scenario design only.

## What to Change

### 1. Add shared non-self-consume acquire coverage at the correct layer

Add focused planner-facing tests showing local unpossessed lots can still satisfy `AcquireCommodity` when the purpose is not `SelfConsume`. Cover one of:

- `AcquireCommodity { purpose: RecipeInput(_) }`
- `AcquireCommodity { purpose: Treatment }`

This coverage belongs in shared goal/search semantics tests, not in candidate generation, because candidate generation does not currently emit those top-level goals.

### 2. Add constrained local-lot runtime coverage

Add a deterministic higher-level scenario for the concrete constraint boundary introduced by carry capacity:

- local lot larger than current remaining capacity
- pickup must split and only partially transfer
- follow-on travel / goal satisfaction stays correct after the constrained transfer

Do not add more `search.rs` production changes unless the new test exposes a real bug. Existing planner-facing partial-pickup tests already cover the search layer.

### 3. Harden the golden scenario family where the real gap exists

Add one additional golden or near-golden scenario variant that stresses the local-lot transport architecture more than the current happy path. Preferred example:

- a merchant-style `MoveCargo` path where a local lot exceeds remaining carry capacity, forcing a split pickup before travel to the destination market

This should remain deterministic and explicitly assert conservation at each relevant phase boundary.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` and/or `crates/worldwake-ai/src/search.rs` (modify tests only)
- `crates/worldwake-ai/tests/golden_e2e.rs` (modify)
- `reports/golden-e2e-coverage-analysis.md` (modify if the golden suite changes)

## Out of Scope

- Refactoring the planner transition architecture itself (covered by HARPREE14-015)
- Adding new candidate-generation behavior solely to broaden `AcquireCommodity` purposes
- New commodity types or new action families
- Changing carry-capacity rules
- Rewriting existing golden scenarios unless shared helpers genuinely need cleanup

## Acceptance Criteria

### Tests That Must Pass

1. New shared goal/search coverage proves a local ground lot can satisfy at least one non-self-consume `AcquireCommodity` purpose.
2. New constrained-transport coverage proves local-lot transport remains correct when capacity forces a partial transfer.
3. New golden or near-golden scenario proves the hardened local-lot transport architecture survives a more demanding end-to-end case than the current happy path.
4. Existing suite: `cargo test -p worldwake-ai --test golden_e2e`
5. Existing suite: `cargo test --workspace`
6. Existing lint: `cargo clippy --workspace`

### Invariants

1. Acquisition and cargo behavior remain grounded in concrete carried and ground inventory state.
2. Capacity constraints remain physically modeled, not bypassed by planner/test shortcuts.
3. Conservation invariants hold at every explicit scenario checkpoint.
4. Deterministic replay remains stable for the added scenarios.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` and/or `crates/worldwake-ai/src/search.rs` -- local unpossessed lot coverage for a non-self-consume `AcquireCommodity` purpose.
2. `crates/worldwake-ai/tests/golden_e2e.rs` -- deterministic constrained local-lot transport scenario with explicit conservation checkpoints.
3. `reports/golden-e2e-coverage-analysis.md` -- updated gap analysis for the added golden coverage.

### Commands

1. `cargo test -p worldwake-ai goal_model`
2. `cargo test -p worldwake-ai search`
3. `cargo test -p worldwake-ai --test golden_e2e`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - Added shared planner/search coverage for non-self-consume local-lot acquisition (`CommodityPurpose::Treatment`) and for partial local food pickup planning.
  - Added a new golden scenario, `golden_capacity_constrained_ground_lot_pickup`, covering harvest -> constrained split pickup -> consume with deterministic replay and conservation checkpoints.
  - Updated `reports/golden-e2e-coverage-analysis.md` to document the new constrained ground-lot scenario.
  - Strengthened candidate generation so locally unpossessed food lots count as immediate self-care relief for suppressing redundant re-production after materialization.
  - Corrected partial `pick_up` planning so the executable step targets the authoritative ground lot while the split-off hypothetical ID is reserved for post-commit materialization binding and later planning.
- Deviations from the corrected plan:
  - The ticket started as coverage-only after reassessment, but the new end-to-end golden scenario exposed two real production bugs:
    - post-materialization self-care ranking still ignored unpossessed local food relief
    - partial pickup steps were recorded against hypothetical targets, making them unexecutable at runtime
  - `goal_model.rs` did not need direct changes; the relevant shared semantics were better exercised in `search.rs`.
- Verification results:
  - `cargo test -p worldwake-ai candidate_generation`
  - `cargo test -p worldwake-ai search`
  - `cargo test -p worldwake-ai --test golden_e2e`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
