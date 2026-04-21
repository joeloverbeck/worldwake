# S123PREORDAUT-001: Introduce OrderedRanked newtype and authoritative entrypoints

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — additive-only API in `worldwake-ai::ranking` (no behaviour change; nothing consumes the new API until S123PREORDAUT-002 lands)
**Deps**: specs/S123-preference-ordering-authority.md

## Problem

`RankedGoal` has exactly one authoritative total order (`ranking::compare_ranked_goals`), but nothing structurally prevents another module inside `worldwake-ai` from sorting a `Vec<RankedGoal>` with its own comparator and handing the result to a consumer that accepts `&[RankedGoal]`. The S112 post-mortem documented this class of regression: `agent_tick/portfolio.rs` shipped a parallel comparator whose tie-breaker silently disagreed with ranking's chain, and the divergence surfaced only after a 1440-tick behavioural golden run. Closing the structural gap requires a module-private newtype (`OrderedRanked<'a>`) whose only constructors also perform the authoritative sort — but before the 25+ parameter sites in `worldwake-ai` can migrate, that newtype and its two entrypoints (`sort_in_place`, `RankingOutcome::ordered`) must exist.

This ticket lands the infrastructure additively. No call sites migrate and no visibility is demoted. After this ticket, the new API is reachable but unused; S123PREORDAUT-002 flips the consumers and S123PREORDAUT-003 locks the invariant.

## Assumption Reassessment (2026-04-21)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `ranking::compare_ranked_goals` exists at `crates/worldwake-ai/src/ranking.rs:1923` with visibility `pub(crate)`; `ranking::ranked_goal_ordering` (the internal `Ordering`-producer that `compare_ranked_goals` wraps) exists at line 1794. `RankingOutcome` is defined at line 36 with `pub ranked: Vec<RankedGoal>` at line 38 and `pub(crate) suppressed` at line 40. `RankingOutcome::into_ranked` already exists at line 48. Verified by direct read.
2. The spec's reference to a compile-fail doctest precedent in the same crate is correct: `crates/worldwake-ai/src/planning_snapshot.rs:399` and `:407` each contain a `///` block tagged `compile_fail` that gates external access to authoritative travel data. S123PREORDAUT-003 will reuse that doctest mechanism for `OrderedRanked` and `compare_ranked_goals`; this ticket only lays the infrastructure.
3. Shared abstraction boundary under audit: the `&[RankedGoal]` parameter shape that every downstream module currently accepts. This ticket does **not** change that boundary — it only adds `OrderedRanked<'a>` as a new type alongside. The boundary flip is the scope of S123PREORDAUT-002.
4. Mismatch + correction: The spec (line 145) asserts "no consumer outside `worldwake-ai` currently reads this field." Grep verified: zero matches for `RankingOutcome` or `outcome.ranked` in `crates/worldwake-sim`, `crates/worldwake-cli`, and `crates/worldwake-ai/tests/` outside of comment strings. No correction needed.

## Architecture Check

1. Additive landings are the safest prologue to an invariant migration: the new API can be reviewed for shape independently of the 25-site boundary flip, and rollback cost is zero (remove three new items from one file). A single-PR land of D1+D2+D3+D4 would bury the newtype shape in a mechanical-refactor diff, making the shape review weaker.
2. No backwards-compatibility shims: the new API is net-new. No alias, no re-export, no deprecated wrapper. `compare_ranked_goals` retains its current `pub(crate)` visibility until S123PREORDAUT-003; the point is that no downstream module migrates yet.
3. `OrderedRanked::new` is declared with no visibility qualifier (strictly module-private). The test-only `from_sorted_for_test` is `pub(crate)` so in-crate tests can build fixtures; S123PREORDAUT-003 adds the compile-fail doctest proving this is not reachable from outside the crate.

## Verification Layers

1. Shape of the new public API (`OrderedRanked`, `sort_in_place`, `RankingOutcome::ordered`) -> in-file unit tests in `ranking.rs` covering `len`, `is_empty`, `first`, `iter`, `as_slice`, `find`, and `sort_in_place` equivalence to the existing ranker sort.
2. No behavioural change introduced by this ticket -> `cargo test -p worldwake-ai` passes identically to `main`; the new API has no callers yet, so golden suites are unaffected.
3. Single-layer ticket: This is a pure type-system addition in `worldwake-ai::ranking`. No decision trace, action trace, event-log, or authoritative-state change applies; the only proof surface is in-file unit coverage. S123PREORDAUT-002 is where call-site migration needs broader layer mapping, and S123PREORDAUT-003 is where the invariant is locked with compile-fail doctests (build-time falsification).

## What to Change

### 1. Add `OrderedRanked<'a>` newtype to `ranking.rs`

Introduce the newtype in `crates/worldwake-ai/src/ranking.rs` (near the top of the file, alongside `RankingOutcome`):

```rust
/// A read-only view over `RankedGoal`s ordered by the authoritative preference
/// defined in `ranking::compare_ranked_goals`. Constructible only from within
/// the `ranking` module, through one of two paths that *also perform* the
/// authoritative sort:
///
/// - `ranking::sort_in_place(&mut Vec<RankedGoal>) -> OrderedRanked<'_>`
/// - `RankingOutcome::ordered(&self) -> OrderedRanked<'_>`
///
/// Downstream modules can iterate, filter, and `find` against this view but
/// cannot re-sort it, extract a mutable reference, or construct a new
/// `OrderedRanked` over a slice they sorted themselves. This is the property
/// that prevents S112-class parallel-comparator regressions (FND-27 / FND-28):
/// the only way to hand an `OrderedRanked` to another module is to have
/// produced it through the authoritative sort.
#[derive(Clone, Copy, Debug)]
pub struct OrderedRanked<'a> {
    slice: &'a [RankedGoal],
}

impl<'a> OrderedRanked<'a> {
    fn new(slice: &'a [RankedGoal]) -> Self {
        Self { slice }
    }

    #[cfg(test)]
    pub(crate) fn from_sorted_for_test(slice: &'a [RankedGoal]) -> Self {
        Self { slice }
    }

    pub fn is_empty(&self) -> bool { self.slice.is_empty() }
    pub fn len(&self) -> usize { self.slice.len() }
    pub fn first(&self) -> Option<&RankedGoal> { self.slice.first() }
    pub fn iter(&self) -> std::slice::Iter<'_, RankedGoal> { self.slice.iter() }
    pub fn as_slice(&self) -> &[RankedGoal] { self.slice }
    pub fn find(&self, pred: impl Fn(&RankedGoal) -> bool) -> Option<&RankedGoal> {
        self.slice.iter().find(|g| pred(*g))
    }
}
```

`OrderedRanked::new` has no visibility qualifier — strictly module-private. No `AsMut`, no `DerefMut`, no `sort_*` affordance.

### 2. Add `sort_in_place` entrypoint

```rust
/// Sort a `Vec<RankedGoal>` by authoritative preference and return a view
/// borrowing the sorted storage. This is the only public path for producing
/// an ordered view outside the ranker's own initial sort (`ranking::rank`).
/// Callers typically run this once per tick after feasibility annotation.
pub fn sort_in_place(ranked: &mut Vec<RankedGoal>) -> OrderedRanked<'_> {
    ranked.sort_unstable_by(compare_ranked_goals);
    OrderedRanked::new(ranked.as_slice())
}
```

### 3. Add `RankingOutcome::ordered`

Extend the existing `impl RankingOutcome` block at `ranking.rs:45`:

```rust
impl RankingOutcome {
    /// Consume the outcome, returning only the ranked goals.
    #[must_use]
    pub fn into_ranked(self) -> Vec<RankedGoal> {
        self.ranked
    }

    /// Borrow the outcome's ranked goals as an `OrderedRanked<'_>`. Safe
    /// because `rank_candidates` always returns a `RankingOutcome` whose
    /// `ranked` field was produced by the authoritative sort.
    #[must_use]
    pub fn ordered(&self) -> OrderedRanked<'_> {
        OrderedRanked::new(self.ranked.as_slice())
    }
}
```

### 4. Add in-file unit tests

Add to `ranking.rs`'s existing `#[cfg(test)] mod tests` (alongside existing `super::compare_ranked_goals` tests at lines 7210, 7237, 7260, 7287):

- `ordered_ranked_exposes_len_and_first_in_sorted_order` — build a pre-sorted 3-element slice, wrap via `from_sorted_for_test`, assert `len()`, `is_empty()`, `first()`, and that `iter().cloned().collect::<Vec<_>>()` preserves order.
- `ordered_ranked_find_returns_first_match` — construct a 3-element slice where two elements satisfy a predicate; assert `find` returns the first (matching iteration order).
- `sort_in_place_matches_ranker_output` — shuffle a `Vec<RankedGoal>` built from fixtures, call `sort_in_place`, compare `as_slice()` element-wise against the output of the existing ranker (`ranking::rank` on the same inputs). Ensures `sort_in_place`'s comparator chain is identical to the authoritative ranker.
- `ranking_outcome_ordered_reflects_ranked_field` — run `rank_candidates` on a fixture, call `.ordered().as_slice()`, compare to the outcome's `ranked` field.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — add `OrderedRanked` type, `sort_in_place` fn, `RankingOutcome::ordered` method, 4 unit tests)

## Out of Scope

- Migrating any `&[RankedGoal]` parameter site (scope of S123PREORDAUT-002).
- Demoting `compare_ranked_goals` from `pub(crate)` to file-private (scope of S123PREORDAUT-003).
- Demoting `RankingOutcome.ranked` from `pub` to `pub(crate)` (scope of S123PREORDAUT-003).
- Adding compile-fail doctests or the single-comparator grep regression (scope of S123PREORDAUT-003).
- Renaming `RankedGoal` / `GroundedGoal` / `OrderedRanked` — those live in S115.
- Changing the re-sort at `agent_tick/mod.rs:883` from "a sort exists here" to "no sort here"; the re-sort is required because `FeasibilityHint` legitimately changes within a tick.

## Acceptance Criteria

### Tests That Must Pass

1. New unit tests in `ranking.rs` `#[cfg(test)] mod tests`: `ordered_ranked_exposes_len_and_first_in_sorted_order`, `ordered_ranked_find_returns_first_match`, `sort_in_place_matches_ranker_output`, `ranking_outcome_ordered_reflects_ranked_field`.
2. Existing suite: `cargo test -p worldwake-ai` passes unchanged — every pre-existing test continues to pass with identical behaviour, because the new API has no consumers in this ticket.
3. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings` passes.

### Invariants

1. `OrderedRanked::new` has no visibility qualifier — strictly module-private. Grep for `fn new` inside `impl<'a> OrderedRanked<'a>` confirms no `pub` / `pub(crate)` / `pub(super)`.
2. `OrderedRanked` exposes no mutable or sort-adjacent affordance — no `AsMut<[RankedGoal]>`, no `DerefMut`, no `sort_*` method.
3. `sort_in_place` is the only public function in `ranking.rs` that calls `compare_ranked_goals` *and* hands back an `OrderedRanked`; `RankingOutcome::ordered` wraps `self.ranked` which was already sorted by the authoritative ranker.
4. `cargo test -p worldwake-ai` behavioural output (test counts, golden goldens) is identical to pre-ticket because no downstream code path has changed.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` (`#[cfg(test)] mod tests`, four new tests) — covers shape of `OrderedRanked`, equivalence of `sort_in_place` to the authoritative ranker, and the `.ordered()` accessor on `RankingOutcome`.

### Commands

1. `cargo test -p worldwake-ai ranking::tests`
2. `cargo test -p worldwake-ai`
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
4. `scripts/verify.sh` — before pushing to the shared branch.

## Outcome

Completed on 2026-04-21.

- Added `OrderedRanked<'a>` to `crates/worldwake-ai/src/ranking.rs` with the requested read-only accessors and an `IntoIterator for &OrderedRanked<'_>` impl so the staged API passes the repo's CI-shaped clippy surface.
- Added `ranking::sort_in_place(&mut Vec<RankedGoal>) -> OrderedRanked<'_>` and `RankingOutcome::ordered(&self) -> OrderedRanked<'_>` without migrating any downstream consumers yet; the additive-only ticket boundary stayed in `ranking.rs`.
- Added the four focused `ranking.rs` unit tests requested by the ticket: `ordered_ranked_exposes_len_and_first_in_sorted_order`, `ordered_ranked_find_returns_first_match`, `sort_in_place_matches_ranker_output`, and `ranking_outcome_ordered_reflects_ranked_field`.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib ranking::tests::ordered_ranked_exposes_len_and_first_in_sorted_order -- --exact`
- Passed `cargo test -p worldwake-ai --lib ranking::tests::ordered_ranked_find_returns_first_match -- --exact`
- Passed `cargo test -p worldwake-ai --lib ranking::tests::sort_in_place_matches_ranker_output -- --exact`
- Passed `cargo test -p worldwake-ai --lib ranking::tests::ranking_outcome_ordered_reflects_ranked_field -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
