# S145PLASUBHAR-004: Cache compound-order regression tests + module doc

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — module-level doc comment on `planning_state.rs` (D5 subsumed); no runtime behavior change
**Deps**: archive/tickets/S145PLASUBHAR-003.md

## Problem

`PlanningState`'s `entities_at_cache` and `effective_place_cache` at `crates/worldwake-ai/src/planning_state.rs:71-72` already have two single-mutation cross-clone tests at `:4179` (`entities_at_cache_is_invalidated_when_holder_moves_across_branches`) and `:4210` (`effective_place_cache_is_invalidated_when_holder_moves_across_branches`). Per S145 D3 (reassessment finding I4), what is missing is a *compound-order* regression: two sibling search branches that apply the same set of mutations in opposite orders must produce equal cache results across the full six-mutator invalidation surface. Per S145 D3.5, a counter-increment regression is also required once ticket 003's `PlanningStateCacheCounters` infrastructure lands. Per S145 D5 (subsumed), a module-level doc comment documenting the cache invariant surface lands alongside the enforcing tests so the invariant lives next to its proof.

## Assumption Reassessment (2026-05-16)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Existing tests at `crates/worldwake-ai/src/planning_state.rs:4179` and `:4210` cover single-mutation cross-clone invariance (A clones B, B mutates, A's cache unchanged) but do not cover compound-order invariance (A applies X-then-Y, B applies Y-then-X, results equal). The new compound-order test exercises the full six-mutator invalidation surface: `move_lot_ref_to_holder` (line 407), `move_lot_ref_to_ground` (line 434), `move_entity_ref` (line 571), `set_possessor_ref` (line 583), `set_container_ref` (line 596), `mark_removed_ref` (line 618). `set_quantity_ref` at `:605-614` deliberately does NOT invalidate the cache (commodity quantity does not affect entity placement) and serves as the no-op control in the D3.5 counter test.
2. `PlanningStateCacheCounters` and `PlanningState::cache_counters()` accessor were introduced by archived ticket `archive/tickets/S145PLASUBHAR-003.md` — this ticket's D3.5 test depends on them. The deferral is per the Placeholder-replace pattern in spirit: ticket 003 owns the infrastructure; this ticket owns the correctness proof.
3. Shared abstraction boundary: the cache invariant being asserted is "two equivalent mutation sequences applied to clones of the same `PlanningState` produce identical results across `entities_at` and `effective_place_ref` queries." This boundary is `PlanningState`-internal; FND-27 forbids the caches from becoming source of truth, and the new test enforces that the caches' memoization remains a pure function of the substrate, not of the order in which mutations were applied.

## Architecture Check

1. Co-locating the doc comment (D5) with the enforcing test (D3) creates a single review surface for the cache invariant — a future reader who modifies `PlanningState` sees both the documented contract and the test that fails if the contract is violated. This is FND-29 (debuggability) at the architecture-comment layer rather than the trace layer.
2. The compound-order test exercises the cross-product of the six invalidating mutators with their inverse-order pairings, rather than testing each mutator in isolation. This catches order-dependent caching bugs that single-mutator tests cannot — for example, a future cache that memoizes by mutator-call-order rather than by resulting substrate state would pass the existing `:4179` / `:4210` tests but fail the new compound-order test.

## Verified Layers

1. Cache compound-order invariance → focused unit test `cache_results_are_order_independent_across_sibling_branches` in `planning_state.rs` `#[cfg(test)]` module — asserts `entities_at` and `effective_place_ref` results are equal across both mutation orderings.
2. Cache invalidation counter increments correctly across mutators → focused unit test `cache_invalidation_count_increments_on_each_mutation` — asserts `invalidations` counter advances by exactly 1 per invalidating mutator and does NOT advance for `set_quantity_ref`.
3. Documented invariant surface → module-level doc comment readable via `cargo doc -p worldwake-ai --no-deps --open` (no runtime proof surface; documentation correctness is reviewed at merge time).
4. Single-layer ticket (focused unit tests + documentation); no action-trace, event-log, or decision-trace surface is relevant because these tests exercise planner-internal substrate that does not mutate world state. Verification Layer 6 single-layer rationale applies.

## Landed Changes

### 1. Compound-order regression test (D3)

In `crates/worldwake-ai/src/planning_state.rs` `#[cfg(test)]` module, this ticket added:

```rust
#[test]
fn cache_results_are_order_independent_across_sibling_branches() {
    // Build a PlanningState with at least two believed entities and a
    // hypothetical lot. Define a mutation set M = {A, B} where A and B
    // each use a different invalidating mutator from the six-mutator
    // surface (e.g., A: move_lot_ref_to_holder, B: set_possessor_ref).
    //
    // Clone the base state twice; apply A-then-B to branch_ab and
    // B-then-A to branch_ba. For every queried place p and entity e:
    //   assert_eq!(branch_ab.entities_at(p), branch_ba.entities_at(p));
    //   assert_eq!(branch_ab.effective_place_ref(e), branch_ba.effective_place_ref(e));
    //
    // Repeat for additional mutator pairs across the six-mutator surface
    // so the test exercises move/possession/container/removal coverage.
}
```

The landed test applies all six invalidating mutators to distinct authoritative entities in forward and reverse order, primes cache reads before branch mutation, and compares `entities_at` plus `effective_place_ref` results across both branches.

### 2. Cache-counter-invariant test (D3.5)

In the same `#[cfg(test)]` module, this ticket added:

```rust
#[test]
fn cache_invalidation_count_increments_on_each_mutation() {
    // Build a base PlanningState; capture initial counters (all zero).
    //
    // For each of the six invalidating mutators, clone the state, apply
    // one mutation, snapshot cache_counters(), assert .invalidations
    // increased by exactly 1 relative to the pre-mutation snapshot.
    //
    // Additionally apply set_quantity_ref (the no-op control) and
    // assert .invalidations did NOT advance.
}
```

### 3. Module-level doc comment (D5)

At the top of `crates/worldwake-ai/src/planning_state.rs`, before the existing `use` declarations, this ticket added a module doc comment:

```rust
//! Planning-state branch evaluation substrate for the GOAP planner.
//!
//! # Cache invariant
//!
//! `PlanningState` carries two pure-function memoization caches:
//! `entities_at_cache` (place -> Vec<EntityId>) and
//! `effective_place_cache` (entity -> Option<EntityId>). These caches are
//! memoization only; they cache pure functions of `PlanningState`'s mutable
//! substrate (place overrides, possessor overrides, container overrides, and
//! the removed-entity set).
//!
//! Any mutation that could change a cached function's output must call
//! `invalidate_entities_at_cache` before the read path can observe stale
//! data. The six mutators that currently invalidate are:
//! `move_lot_ref_to_holder`, `move_lot_ref_to_ground`, `move_entity_ref`,
//! `set_possessor_ref`, `set_container_ref`, and `mark_removed_ref`.
//! `set_quantity_ref` deliberately does not invalidate because commodity
//! quantity does not affect entity placement.
//!
//! Sibling search branches that mutate state in different orders must
//! produce equal cache outputs. The compound-order regression test
//! `cache_results_are_order_independent_across_sibling_branches` enforces
//! this invariant across the full mutator surface.
//!
//! The cache must never be promoted to source of truth (see FND-27,
//! Derived Summaries Are Caches, Never Truth).
```

## Landed Files

- `crates/worldwake-ai/src/planning_state.rs` (modify — add 2 new tests in `#[cfg(test)]` module + module-level doc comment)

## Out of Scope

- No change to existing tests `entities_at_cache_is_invalidated_when_holder_moves_across_branches` (`:4179`) or `effective_place_cache_is_invalidated_when_holder_moves_across_branches` (`:4210`) — they continue to cover the single-mutation cross-clone invariant, complementing this ticket's compound-order coverage.
- No new mutator added to `PlanningState` — the six existing invalidating mutators and one no-op mutator are exercised as-is.
- No introduction of new public API surface — both tests and the doc comment are scope-limited to the module.

## Acceptance Result

### Tests That Passed

1. `cache_results_are_order_independent_across_sibling_branches` passed against the landed cache implementation.
2. `cache_invalidation_count_increments_on_each_mutation` passed — counter advances by 1 per invalidating mutator and stays unchanged for `set_quantity_ref`.
3. Existing `entities_at_cache_is_invalidated_when_holder_moves_across_branches` and `effective_place_cache_is_invalidated_when_holder_moves_across_branches` passed unchanged through the `planning_state` module run.
4. Existing suite passed through `cargo test --workspace` and `scripts/verify.sh`.

### Invariants

1. `cache_results_are_order_independent_across_sibling_branches` is a permanent regression: any future cache implementation that introduces order-dependence in `entities_at_cache` or `effective_place_cache` outputs must cause this test to fail.
2. The doc comment's mutator list matches the actual six invalidating mutators in `planning_state.rs`. If a future change adds a seventh invalidating mutator (or removes one), the doc comment must be updated in the same change.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/planning_state.rs` (modify, `#[cfg(test)]` module) — `cache_results_are_order_independent_across_sibling_branches` (compound-order D3 coverage).
2. `crates/worldwake-ai/src/planning_state.rs` (modify, `#[cfg(test)]` module) — `cache_invalidation_count_increments_on_each_mutation` (D3.5 counter coverage; depends on ticket 003's `PlanningStateCacheCounters`).

### Passed Commands

1. `cargo test -p worldwake-ai planning_state::tests::cache_results_are_order_independent_across_sibling_branches`
2. `cargo test -p worldwake-ai planning_state::tests::cache_invalidation_count_increments_on_each_mutation`
3. `cargo test -p worldwake-ai planning_state`
4. `scripts/verify.sh`

## Outcome

Completed on 2026-05-16.

- Added the `PlanningState` module-level cache invariant comment describing the memoized `entities_at_cache` and `effective_place_cache`, the six invalidating mutators, the `set_quantity_ref` no-op control, and the requirement that derived cache state never become source of truth.
- Added `cache_results_are_order_independent_across_sibling_branches`, which primes cache reads, applies all six invalidating mutators to distinct authoritative entities in forward and reverse order on sibling branches, and asserts equal `entities_at` plus `effective_place_ref` outputs.
- Added `cache_invalidation_count_increments_on_each_mutation`, which proves each invalidating mutator advances `PlanningStateCacheCounters.invalidations` by exactly one and `set_quantity_ref` does not.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib planning_state::tests::cache_results_are_order_independent_across_sibling_branches -- --exact`
- Passed `cargo test -p worldwake-ai --lib planning_state::tests::cache_invalidation_count_increments_on_each_mutation -- --exact`
- Passed `cargo test -p worldwake-ai --lib planning_state`
- Passed `cargo test --workspace`
- Passed `scripts/verify.sh`
