# S123: Preference-Ordering Authority

## Summary

Make `ranking::compare_ranked_goals` the sole authoritative total order on `RankedGoal` by introducing a module-owned `OrderedRanked<'a>` newtype over `&[RankedGoal]`, constructible only through `ranking::sort_in_place` (which sorts a `Vec<RankedGoal>` and returns an `OrderedRanked<'_>` borrowing it) or `RankingOutcome::ordered` (which borrows the outcome's already-sorted `ranked` field). The newtype's constructor is strictly module-private, which is what closes the gap: downstream modules inside `worldwake-ai` cannot synthesise an `OrderedRanked` over a slice they sorted themselves, because the only paths that produce an `OrderedRanked` also perform the authoritative sort. S112 regressed `golden-survival / baseline` precisely because `agent_tick/portfolio.rs` shipped a parallel comparator whose tie-breaker (goal-key discriminant) silently disagreed with ranking's authoritative tie-breaker chain (feasibility → specificity → opportunity strength). The comparator divergence was not visible at review time, did not fail any unit test at land, and only surfaced after 1440-tick behavioural golden runs. This spec closes the structural path that allowed the divergence to exist.

## Phase and Status

Phase 8 Adjunct: Belief-First Continual Planning Foundation. Status: Draft.

## Crates

- `worldwake-ai` — `ranking.rs` gains `OrderedRanked<'a>` and `sort_in_place(&mut Vec<RankedGoal>)`; `compare_ranked_goals` downgraded from `pub(crate)` to file-private. All `&[RankedGoal]` parameter sites in `agent_tick/planning.rs`, `agent_tick/portfolio.rs`, `agent_tick/active_action.rs`, `agent_tick/frame.rs`, `plan_selection.rs`, `interrupts.rs`, `side_benefit.rs` migrate to `&OrderedRanked<'_>`.
- `worldwake-core` — no changes (the newtype lives in `worldwake-ai`).
- `worldwake-sim` — no changes.
- `worldwake-cli` — no changes.

## Dependencies

- `archive/specs/S112-portfolio-planning.md` — the incident that motivated this spec. The portfolio module is the call site that introduced the second comparator; the migration here consumes `OrderedRanked` at `assemble_portfolio`'s boundary. Hard (shape of the public signature).
- `archive/specs/S74-intention-commitment-under-needs-fluctuation.md` — `explain_ranked_goal_order` stays public for margin-based commit's comparison output. Soft.
- `specs/S115-agenda-manager.md` — **forward dependency.** S115 owns the `RankedGoal` → `AgendaEntry` / `GroundedGoal` → `GoalOffer` renames and will rename `OrderedRanked` → `OrderedAgenda` (or equivalent) as part of the agenda-lifecycle migration. S123 must land before S115 so the rename is a single atomic edit against an established authoritative-view framework. FND-28 forbids a shim or dual-name compatibility window. Soft (direction of land order, not shape).

## Motivating Evidence

From the S112 incident resolution (landed via PR #61):

- `agent_tick/portfolio.rs` had shipped a parallel `compare_ranked_goals` that used `left.motive_score.cmp(&right.motive_score)` forward and then `right.grounded.key.cmp(&left.grounded.key)` as a reverse tie-break; `ranking::ranked_goal_ordering` uses `right.motive_score.cmp(&left.motive_score)` reverse and then `left.feasibility.cmp(&right.feasibility)` forward as its next discriminant. For Agent B, tick 239 of `survival-baseline.ron` (seed 104004), the two inputs were Water@Riverside and Apple@FertileFields with motive 160 300 each. Ranking preferred Water (higher feasibility → Likely); portfolio preferred Apple (smaller `CommodityKind` discriminant). Agent B never committed a `drink` action across 1440 ticks; `all_agents_perform_survival_actions` failed. After the S112 fix the parallel definition is gone — only explanatory comments in `portfolio.rs` (at lines 119, 288, 480) remain, referencing the authoritative `ranking::compare_ranked_goals`.
- `agent_tick/planning.rs::build_candidate_plans` had prepended plausible slot winners to `search_order` before appending admitted ranked order, making Survival-slot winners win the first search slot even when a higher-motive non-survival candidate was immediately behind them. At Agent A, tick 3 of the same scenario, `ConsumeOwnedCommodity(Water)` (survival slot) was searched and committed before `ExploreLocation(FertileFields)` (higher motive) despite ranking ordering the latter first. The resulting 13-tick delay to Fertile Fields collided with Agent B's harvest reservation at tick 16 and produced a 21-tick idle window that failed `no_stuck_idle_windows_with_elevated_needs`. Current `build_candidate_plans` constructs `search_order` from `admitted_candidates` filtered by rejected opportunities — ranking-order-respecting, slot-agnostic.

Both regressions were fixed by making portfolio consume the admitted order via `find` and by replacing slot-first `search_order` construction with "admitted order, skipping only probe-rejected opportunities." The underlying architectural gap — *nothing prevents the next S112-class change from re-introducing a parallel comparator or a slot-based re-sort* — is what this spec addresses.

## Design Goals

- `RankedGoal` has exactly one total order reachable from any call site: the one produced by `ranking`. Downstream modules can filter, iterate, or find against an `OrderedRanked<'_>` view, but cannot re-sort it and cannot synthesise an alternative comparator that Rust will compile against `&[RankedGoal]` or `&OrderedRanked<'_>`. The invariant must hold against intra-crate regressions (the S112 class), not only against external-crate imports.
- The only paths that yield an `OrderedRanked<'_>` also perform the authoritative sort: `ranking::sort_in_place(&mut Vec<RankedGoal>) -> OrderedRanked<'_>` and `RankingOutcome::ordered(&self) -> OrderedRanked<'_>`. `OrderedRanked`'s constructor is strictly module-private (no visibility qualifier), so no module outside `ranking` can wrap an externally-sorted slice.
- The re-sort that `agent_tick::mod` runs after feasibility annotation (current `agent_tick/mod.rs:883`) survives as a public entrypoint but is explicitly named — `ranking::sort_in_place` — so every occurrence of "the ranking invariant is re-established here" is greppable.
- The migration is mechanical, not semantic. Post-migration behaviour is identical to the post-S112-fix behaviour currently in `main`; the spec's purpose is to ensure a future edit cannot regress back to the S112-class bug without Rust refusing to compile.
- Test fixtures keep a narrow escape hatch (`OrderedRanked::from_sorted_for_test`, `pub(crate)`) so unit tests can build ranked slices manually, with a doctest demonstrating that the escape hatch is not reachable outside `worldwake-ai`.
- `explain_ranked_goal_order` stays `pub(crate)` because margin-based commit (S74) and decision-trace rendering need to surface "why X outranked Y" without re-sorting.

## Non-Goals

- Renaming `RankedGoal` → `AgendaEntry` or `GroundedGoal` → `GoalOffer`. Those renames live in S115 and must land as part of the agenda-lifecycle migration.
- Embedding slot priority into motive score as a pre-ranking adjustment (the "Option C" sketch from the S112 post-mortem). That is a larger redesign that S116's `DriveEscalationProfile` pattern points at; S123 stays strictly a structural-invariant spec.
- Removing or changing the re-sort at `agent_tick/mod.rs:883` after feasibility annotation. The feasibility hint is a field on `RankedGoal` that legitimately changes within a tick (after blocker/discrepancy memory reads); re-sort is correct.
- Unifying `ranking::sort` with `goal_model.rs`'s unrelated place sort (`goal_model.rs:2125`, a `Vec<EntityId>` sort by travel cost). Different type, different concern.
- Runtime lints or Clippy custom lints. The invariant is enforced by the Rust type system; a clippy rule would be redundant.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-14 (World State Is Not Belief State) | The belief-derived preference ordering is produced by a single authoritative function that reads the agent's current beliefs. Parallel comparators bypassing that function effectively introduce a second "truth" about what the agent prefers; the newtype makes that impossible. |
| FND-20 (Resource-Bounded Practical Reasoning) | Candidate selection collapses to "traverse `OrderedRanked` in order and take the first satisfying item." No re-evaluation of preference inside selection. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | `OrderedRanked` is a derived view on `&[RankedGoal]` with read-only accessors. The ordering is never cached across ticks. Because `OrderedRanked::new` is module-private, the only paths that produce an `OrderedRanked<'_>` (`sort_in_place` and `RankingOutcome::ordered`) also perform the authoritative sort — a downstream module cannot re-derive the ordering with a different comparator and hand the result to a consumer that accepts `&OrderedRanked<'_>`. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The migration replaces the existing `&[RankedGoal]` signature surface in one atomic rename; no compatibility shim, no deprecated path. |
| FND-31 (Validation and Falsification Are First-Class) | The compile-fail doctest (D5) turns the invariant into a build-time falsification: any future PR that tries to construct `OrderedRanked` externally or call `sort_by(compare_ranked_goals)` directly fails `cargo test --doc`. |

## Deliverables

### D1: `OrderedRanked<'a>` newtype

New type in `crates/worldwake-ai/src/ranking.rs`:

```rust
/// A read-only view over `RankedGoal`s ordered by the authoritative preference
/// defined in `ranking::compare_ranked_goals`. Constructible only from within
/// the `ranking` module, through one of two paths that *also perform* the
/// authoritative sort:
///
/// - `ranking::sort_in_place(&mut Vec<RankedGoal>) -> OrderedRanked<'_>`
///   (sorts the vec in place and returns a view borrowing it).
/// - `RankingOutcome::ordered(&self) -> OrderedRanked<'_>`
///   (borrows the already-sorted `ranked` field).
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
    /// Authoritative constructor. Strictly module-private — no visibility
    /// qualifier — so no call site outside `ranking` can wrap a slice that
    /// was sorted by any other comparator. Used internally by `sort_in_place`
    /// and `RankingOutcome::ordered`.
    fn new(slice: &'a [RankedGoal]) -> Self {
        Self { slice }
    }

    /// Test-only construction path. `pub(crate)` so that unit tests inside
    /// `worldwake-ai` can build fixtures; a compile-fail doctest (D5) proves
    /// this is not reachable from outside the crate.
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

`OrderedRanked` does not expose `AsMut<[RankedGoal]>`, `DerefMut`, or any `sort_*` affordance. `as_slice` returns `&[RankedGoal]` (immutable); a downstream module that sorts a copy obtained from `as_slice().to_vec()` is re-creating a *different* owned collection, which is acceptable — the point of the invariant is that such a copy can never be wrapped back into an `OrderedRanked` and handed to modules that accept `&OrderedRanked<'_>`.

### D2: `sort_in_place` and `RankingOutcome::ordered` entrypoints

```rust
/// Sort a `Vec<RankedGoal>` by authoritative preference and return a view
/// borrowing the sorted storage. This is the only public path for producing
/// an ordered view outside the ranker's own initial sort (`ranking::rank`).
/// Callers typically run this once per tick after feasibility annotation;
/// its existence is load-bearing because `FeasibilityHint` can change
/// mid-tick and affects sort order.
pub fn sort_in_place(ranked: &mut Vec<RankedGoal>) -> OrderedRanked<'_> {
    ranked.sort_unstable_by(compare_ranked_goals);
    OrderedRanked::new(ranked.as_slice())
}

impl RankingOutcome {
    /// Borrow the outcome's ranked goals as an `OrderedRanked<'_>`. Safe
    /// because `rank_candidates` always returns a `RankingOutcome` whose
    /// `ranked` field was produced by the authoritative sort.
    #[must_use]
    pub fn ordered(&self) -> OrderedRanked<'_> {
        OrderedRanked::new(self.ranked.as_slice())
    }
}
```

`compare_ranked_goals` is demoted from `pub(crate)` to file-private (no visibility qualifier). The only external entrypoints to the ordering are `sort_in_place`, `RankingOutcome::ordered`, and `explain_ranked_goal_order`. Every in-tree test that currently calls `compare_ranked_goals` directly migrates to `sort_in_place` or `OrderedRanked::from_sorted_for_test`.

`RankingOutcome.ranked` is demoted from `pub` to `pub(crate)` in the same edit. External callers that previously read the field directly must now use `RankingOutcome::ordered()` (for a read-only view) or `RankingOutcome::into_ranked()` (to consume the outcome and take ownership of the sorted `Vec`). This closes the raw-`Vec<RankedGoal>` leak the field currently exposes: without the demotion, an external caller could clone `outcome.ranked`, sort it by a different comparator, and pass the result to any consumer accepting `&[RankedGoal]`. Grep confirms that no consumer outside `worldwake-ai` currently reads this field (zero matches in `crates/worldwake-ai/tests/` and in sibling crates), so the demotion has no blast radius beyond the in-crate reader at `agent_tick/observation.rs:274`, which remains `pub(crate)`-visible.

### D3: Call-site migration

All `&[RankedGoal]` parameters in the following files migrate to `&OrderedRanked<'_>`:

- `agent_tick/planning.rs` — 11 parameter sites at lines 232, 342, 655, 666, 714, 856, 931, 1101, 1134, 1162, 1325 (functions: `selected_plan_value`, `build_candidate_plans`, `ranked_goal_for_opportunity`, `summarize_snapshot_continuation`, `try_continue_snapshot_plan`, `build_rejected_alternatives`, `emit_plan_selection_events`, `adopt_selected_plan`, `clear_current_plan`, `plan_and_validate_next_step`, and `plan_and_validate_next_step_traced`'s helper pair).
- `agent_tick/portfolio.rs` — the `pub(crate) fn assemble_portfolio` public entrypoint (line 34) and its file-private helpers `select_commitment_candidate` (line 97) and `select_best_candidate` (line 127). Only `assemble_portfolio` is a cross-module callsite; the helpers migrate because their `&'a [RankedGoal]` parameter flows directly from the public signature.
- `agent_tick/active_action.rs` — `handle_active_action_phase` at line 50.
- `agent_tick/frame.rs` — `evaluate_assumptions` at line 339, whose parameter is `Option<&[RankedGoal]>` (not a plain slice); the migration target is `Option<&OrderedRanked<'_>>` and the `None` case stays as-is.
- `plan_selection.rs` — `select_best_plan` entrypoint at line 26.
- `interrupts.rs` — 4 sites at lines 35, 135, 280, 290 (`evaluate_interrupt`, `best_challenger`, `interrupt_freely`, `current_priority`).
- `side_benefit.rs` — 2 sites at lines 24 and 79 (`detect_side_benefits`, `build_plan_value`).

`agent_tick/mod.rs:883` changes from `ranked_candidates.sort_by(crate::ranking::compare_ranked_goals);` to `let ordered = crate::ranking::sort_in_place(&mut ranked_candidates);`. The returned `OrderedRanked<'_>` is then passed down to each migrated callsite. Because `OrderedRanked::new` is module-private, no call site can construct the view independently; the only way to hand one to a consumer is to go through `sort_in_place` (or `RankingOutcome::ordered` at earlier points in the ranking pipeline).

Internal helpers that iterate `OrderedRanked` use `.iter()` or `.find()`. Helpers that need index-style access use `.as_slice()` explicitly at the use site, which documents the read-only intent.

### D4: Test-fixture migration

In-tree tests that construct `Vec<RankedGoal>` manually (e.g. `agent_tick/portfolio.rs::tests` helpers at `ranked_goal(...)` on line 243 and the portfolio/planning fixtures updated in the S112 incident fix) switch to:

```rust
let mut ranked = vec![
    ranked_goal(/* … */),
    ranked_goal(/* … */),
];
// sort_in_place returns the ordered view directly
let ordered = ranking::sort_in_place(&mut ranked);
```

If a test needs to skip the sort (e.g., it constructs an already-sorted fixture) it uses the test-only escape hatch:

```rust
let ordered = ranking::OrderedRanked::from_sorted_for_test(&ranked);
```

This keeps the existing "pass pre-sorted input" invariant from the S112 fix but makes it mechanical rather than a comment. Tests inside `ranking.rs` itself continue to call `compare_ranked_goals` directly (the file-private visibility does not block in-file access) and do not need the escape hatch.

### D5: Compile-fail doctests

Two doctests in `crates/worldwake-ai/src/ranking.rs` document and enforce the boundary:

```rust
/// Outside the crate, `OrderedRanked::from_sorted_for_test` is not reachable:
///
/// ```compile_fail
/// use worldwake_ai::ranking::OrderedRanked;
/// use worldwake_ai::RankedGoal;
/// let empty: &[RankedGoal] = &[];
/// let _ = OrderedRanked::from_sorted_for_test(empty);
/// ```
///
/// Outside the crate, `compare_ranked_goals` is not reachable:
///
/// ```compile_fail
/// use worldwake_ai::ranking::compare_ranked_goals;
/// ```
```

Any future PR that tries to synthesise `OrderedRanked` externally or import `compare_ranked_goals` to power a second comparator fails `cargo test --doc -p worldwake-ai`.

### D6: Single-comparator grep regression

Add a build-time regression at `crates/worldwake-ai/src/ranking.rs` tests module:

```rust
#[test]
fn compare_ranked_goals_is_the_only_impl_in_crate() {
    // Walk every `*.rs` under `src/` (excluding `ranking.rs`) and fail if
    // any file contains the byte sequence `fn compare_ranked_goals`. This
    // is a belt-and-suspenders check: the newtype makes the parallel
    // comparator useless, but the grep prevents a dead-but-confusing
    // parallel definition from living in-tree.
    // …implementation reads CARGO_MANIFEST_DIR and walks the directory…
}
```

The closest existing in-repo precedent for compile-time-enforced structural invariants in the AI crate is the `compile_fail` doctest block in `crates/worldwake-ai/src/planning_snapshot.rs` (at lines 399 and 407), which uses the doctest mechanism to gate external access to private fields and types — same intent (build-time falsification of an invariant future code could otherwise violate), different mechanism. D5 adopts the doctest half of that precedent; D6's grep walk over `src/` is net-new tooling for this crate. Co-locating the grep regression in `ranking.rs`'s test module keeps the check alongside the authoritative ordering.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: The comparator reads only data the ranker itself produced from belief-view queries — the fields on `RankedGoal` (priority class, motive score, feasibility hint) and derivations from its `grounded` contents (goal specificity and opportunity strength, both computed by helper functions in `ranking.rs`). No cross-agent information flow; no omniscient access. The newtype is a read-only view over in-memory data already computed for this agent this tick.
2. **Positive-feedback analysis**: None. The ordering is produced per-tick and discarded; `OrderedRanked` borrows a `Vec<RankedGoal>` that lives only for the planning call stack. No cross-tick reinforcement.
3. **Concrete dampeners**: None needed — the invariant is structural, not dynamic.
4. **Stored state vs. derived read-model**: `OrderedRanked` is the canonical derived read-model. It holds no stored state. `Vec<RankedGoal>` is the storage (transient, per-tick) the ranker mutates; the newtype wraps a borrow.

## SystemFn Integration

None. This spec is a type-system refactor inside `worldwake-ai`; no new `SystemFn`, no new system-manifest entry, no scheduler change.

## Component Registration

None.

## Cross-System Interactions

- **Ranking ↔ candidate generation**: Unchanged. Candidate generation feeds `Vec<GroundedGoal>` to the ranker; the ranker returns a `RankingOutcome` whose `ranked` field is already sorted. Downstream callers read the ordering through `RankingOutcome::ordered()`.
- **Ranking ↔ portfolio (S112)**: Post-migration, `assemble_portfolio` takes `&OrderedRanked<'_>` and calls `find` on the first matching candidate per slot. The old `max_by` tie-break is impossible because `OrderedRanked` exposes no sort or max-by affordance, and because the caller cannot construct an `OrderedRanked` over a locally re-sorted copy.
- **Ranking ↔ agenda manager (S115)**: S115's `tick_agenda` merges fresh offers with existing agenda entries, then ranks the merged pool. Post-S123, the ranker returns a `RankingOutcome` whose `.ordered()` view S115's commit-decision reads. S115 does not gain the ability to re-order the pool; it makes a single-pass commit decision. As noted in Dependencies, the eventual rename `OrderedRanked → OrderedAgenda` (or equivalent) lands as part of S115's rename wave, not here.
- **Ranking ↔ margin-based commit (S74)**: `explain_ranked_goal_order` stays the public way to compute "why winner outranks loser" for trace output. No change.

## Profile-Driven Parameters

None. The preference ordering is not per-agent; it is the single authoritative ordering on which motive / priority / feasibility trade-offs land. Per-agent variation lives in the motive-score and priority-class inputs (which already carry per-agent weights via `UtilityProfile`, `DriveEscalationProfile`, etc.), not in the comparator.

## Validation and Falsification

### Unit tests

1. `OrderedRanked::find` returns the first-ranked matching candidate on a hand-constructed pre-sorted slice.
2. `sort_in_place` on a shuffled `Vec<RankedGoal>` produces the same order as the in-tree ranker (`ranking::rank`) on the same inputs.
3. The D6 grep regression passes when only `ranking.rs` defines `fn compare_ranked_goals`, and fails when any other file in `src/` does.
4. The portfolio-tie regression from the S112 incident: construct admitted `[Water@Riverside (motive 160 300, feasibility Likely), Apple@Fertile (motive 160 300, feasibility Uncertain)]`, run `assemble_portfolio`, and assert the Survival slot is Water@Riverside. (Equivalent test already exists post-fix; this spec cements the contract.)

### Integration tests

5. `survival-baseline.ron`, `survival-contested.ron`, `survival-scattered.ron` goldens pass unchanged.
6. `crates/worldwake-ai/tests/golden_portfolio_planning.rs::portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` passes unchanged. (This is currently the only test in the file; the portfolio-tie regression is covered by Unit test #4 above rather than by a duplicate golden.)
7. `cargo test --doc -p worldwake-ai` passes (the D5 compile-fail blocks succeed).

### Golden test

8. No new golden scenario is added. The invariant is structural; its falsification channel is the compile-fail doctest and the grep regression. Behavioural goldens protect the *current* behaviour, which this spec preserves exactly.

## Outcome

To be filled in at completion.
