# HARPREE14-016: Ground-lot acquisition coverage and constrained transport

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes -- AI acquisition candidate/search coverage, golden e2e hardening
**Deps**: HARPREE14-015 (planner-owned hypothetical transitions), HARPREE14-011 (multi-recipe craft-path golden scenario, archived)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-D01, Principle 3, Principle 6

## Problem

The recent multi-recipe golden scenario proved that local ground lots matter, but current coverage is still too narrow:

1. Candidate generation only gained one hunger-driven local-lot regression test.
2. The golden scenario only covers the unconstrained case where the crafted output fits easily.
3. There is no end-to-end proof that local-lot acquisition remains correct when transport constraints or non-food acquisition purposes are involved.

That leaves a real blind spot: acquisition behavior around local concrete inventory carriers can regress again without a strong test net.

## Assumption Reassessment (2026-03-12)

1. `candidate_generation.rs` now includes a local unpossessed food-lot test for `AcquireCommodity { purpose: SelfConsume }` -- confirmed.
2. `golden_multi_recipe_craft_path` in `crates/worldwake-ai/tests/golden_e2e.rs` covers multi-recipe craft -> pickup -> consume in the unconstrained case -- confirmed.
3. No current golden or unit test covers capacity-constrained pickup or a non-self-consume local-lot acquisition path such as restock/recipe-input acquisition -- confirmed.
4. Carry capacity is a foundational concrete-state constraint and should be represented in planner-facing tests whenever local-lot acquisition is under test -- confirmed.

## Architecture Check

1. This ticket is about coverage, not new planner behavior. The goal is to make the new local-lot architecture robust by exercising the real constraint boundaries it depends on.
2. Covering multiple acquisition purposes is cleaner than leaving the behavior implicitly hunger-specific, because `AcquireCommodity` is a shared architectural goal family.
3. No backwards-compatibility shims are needed; this is hardening through stronger tests and scenario design.

## What to Change

### 1. Expand candidate-generation coverage for local lots

Add focused tests showing local unpossessed lots can ground acquisition evidence for more than hunger/self-consume. At minimum cover one of:

- `RestockCommodity`
- `AcquireCommodity { purpose: RecipeInput(_) }`
- another non-food acquisition use already present in the goal model

The point is to prove the local-lot evidence path is shared architecture, not a one-off hunger exception.

### 2. Add constrained-pickup search or e2e coverage

Add tests for the concrete constraint boundary introduced by carry capacity:

- local lot larger than current remaining capacity
- pickup must split or otherwise only partially transfer
- follow-on acquisition/planning behavior stays correct after the constrained transfer

Use the narrowest test level that still proves the behavior, but include at least one planner-facing regression and one higher-level scenario if the edge case meaningfully affects end-to-end behavior.

### 3. Harden the golden multi-recipe scenario family

Add one additional golden or near-golden scenario variant that stresses the local-lot architecture more than the current happy path. Examples:

- crafted output plus competing unrelated local lots
- crafted output when actor inventory is near capacity
- multiple known recipes where only one should remain viable under current concrete inventory

This should remain deterministic and explicitly assert conservation at each relevant phase boundary.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/search.rs` and/or `crates/worldwake-ai/src/planning_state.rs` (modify tests)
- `crates/worldwake-ai/tests/golden_e2e.rs` (modify)

## Out of Scope

- Refactoring the planner transition architecture itself (covered by HARPREE14-015)
- New commodity types or new action families
- Changing carry-capacity rules
- Rewriting existing golden scenarios unless shared helpers genuinely need cleanup

## Acceptance Criteria

### Tests That Must Pass

1. New candidate-generation coverage proves local ground-lot evidence works for at least one non-self-consume acquisition purpose.
2. New constrained-transport coverage proves local acquisition remains correct when capacity forces partial transfer.
3. New golden or near-golden scenario proves the hardened local-lot architecture survives a more demanding end-to-end case than the current happy path.
4. Existing suite: `cargo test -p worldwake-ai --test golden_e2e`
5. Existing suite: `cargo test --workspace`
6. Existing lint: `cargo clippy --workspace`

### Invariants

1. Acquisition behavior remains grounded in concrete carried and ground inventory state.
2. Capacity constraints remain physically modeled, not bypassed by planner/test shortcuts.
3. Conservation invariants hold at every explicit scenario checkpoint.
4. Deterministic replay remains stable for the added scenarios.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` -- local unpossessed lot evidence for a non-self-consume acquisition purpose.
2. `crates/worldwake-ai/src/search.rs` and/or `crates/worldwake-ai/src/planning_state.rs` -- constrained pickup / partial-transfer regression coverage.
3. `crates/worldwake-ai/tests/golden_e2e.rs` -- additional deterministic local-lot scenario with explicit conservation checkpoints.

### Commands

1. `cargo test -p worldwake-ai candidate_generation`
2. `cargo test -p worldwake-ai search`
3. `cargo test -p worldwake-ai --test golden_e2e`
4. `cargo test --workspace`
5. `cargo clippy --workspace`
