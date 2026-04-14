# S103BELCLADED-002: Skip summary re-derivation when no claims pruned

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — belief pruning (worldwake-core)
**Deps**: S101 (completed), archive/tickets/S103BELCLADED-004.md (completed)

## Problem

`prune_decayed_beliefs` calls `refresh_entity_summary_from_claims` for every entity that has claims, regardless of whether any claims were actually removed during the retain pass. Once semantic entity belief updates are canonicalized through `entity_claims`, this means entities whose claim sets did not change still incur O(claims) iteration in `derive_entity_summary`. With many known entities and few actual prune events per tick, most of this work is wasted.

## Assumption Reassessment (2026-04-14)

1. `prune_decayed_beliefs` at `crates/worldwake-core/src/belief.rs:196` still collects all `entity_claims` keys before pruning and unconditionally calls `refresh_entity_summary_from_claims` for every affected entity after pruning — verified.
2. `refresh_entity_summary_from_claims` at `crates/worldwake-core/src/belief.rs:131` still calls `derive_entity_summary`, which iterates all claims for the entity — verified.
3. The original direct implementation of this ticket was tested on 2026-04-14 and failed `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots guard_theron_water_at_thornwall_finds_harvest_plan`. Reassessment showed the failure came from duplicate semantic transport paths into `known_entities`, not from the optimization itself.
4. The exact shared abstraction boundary under audit for this ticket is: `entity_claims` as authoritative semantic belief storage, `known_entities` as derived summary cache. That boundary is not fully restored on the current branch yet.
5. `archive/tickets/S103BELCLADED-004.md` now owns the completed cleanup: activity, departure projection, evidence, and imported snapshot semantics are claim-backed before this ticket is implementable without changing world meaning.
6. Intended invariant before implementation: once `S103BELCLADED-004` lands, unchanged `entity_claims` must imply unchanged semantic `known_entities` summaries, so this ticket becomes a pure FND-12 computation compression.
7. Existing focused/core coverage relevant after the dependency lands remains `test_prune_decayed_beliefs_removes_below_threshold`, `test_prune_decayed_beliefs_removes_orphan_claims`, and any new claim-backed summary-stability tests added by `S103BELCLADED-004`.

## Architecture Check

1. Keeping this ticket narrow preserves clean review boundaries. `S103BELCLADED-004` restores the belief transport contract; this ticket then applies a pure changed-entity optimization on top of that contract.
2. No backward-compatibility shims. After the dependency lands, this ticket is just a local retain-pass optimization in `prune_decayed_beliefs`.

## Verification Layers

1. Claim vectors that lose entries still trigger summary re-derivation -> focused `worldwake-core` unit coverage on changed entities
2. Claim vectors that do not change keep identical semantic summaries because the dependency made `known_entities` purely claim-backed -> focused `worldwake-core` unit coverage added by `S103BELCLADED-004`
3. Planner-visible behavior remains unchanged after the optimization -> `guard_theron_water_at_thornwall_finds_harvest_plan` plus full `worldwake-ai` suite
4. The optimization remains a computation-only change after the boundary cleanup -> code inspection plus the mixed-layer verification above
5. Additional trace mapping is not applicable here once the dependency lands; the contract is local to claim pruning and planner-visible stability

## What to Change

### 1. Track changed entities during the retain pass

In `prune_decayed_beliefs`, record which entities actually lost claims during the confidence-threshold retain pass.

### 2. Refresh only changed claim-backed summaries

After pruning empty claim vectors, call `refresh_entity_summary_from_claims` only for entities whose claim vectors changed.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)

## Out of Scope

- Canonicalizing semantic belief transport paths (`S103BELCLADED-004`)
- Changing what gets pruned (confidence thresholds, decay rates)
- Claim deduplication (`S103BELCLADED-001`)
- Social observation deduplication (`S103BELCLADED-003`)

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: changed claim vectors still re-derive summaries correctly while unchanged claim vectors preserve identical claim-backed summaries
2. Existing: `test_prune_decayed_beliefs_removes_below_threshold`
3. Existing: `test_prune_decayed_beliefs_removes_orphan_claims`
4. Existing suite: `cargo test -p worldwake-core`
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `prune_decayed_beliefs` changes no world meaning once `known_entities` is claim-backed
2. Entities with all claims pruned are still removed from `entity_claims`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — focused prune test covering changed vs unchanged claim vectors after `S103BELCLADED-004` lands
2. `None — dependency ticket supplies the boundary-restoration tests; this ticket adds only the changed-entity pruning proof on top of them.`

### Commands

1. `cargo test -p worldwake-core --lib belief::tests::test_prune_decayed_beliefs_`
2. `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots guard_theron_water_at_thornwall_finds_harvest_plan`
3. `cargo test -p worldwake-core`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`
