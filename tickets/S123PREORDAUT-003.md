# S123PREORDAUT-003: Lock preference-ordering invariant — demote visibilities, add compile-fail doctests and grep regression

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — visibility demotions in `worldwake-ai::ranking` (`compare_ranked_goals`: `pub(crate)` → file-private; `RankingOutcome.ranked`: `pub` → `pub(crate)`) + new build-time falsification tests
**Deps**: archive/tickets/S123PREORDAUT-002.md

## Problem

After S123PREORDAUT-001 and -002, `OrderedRanked<'a>` exists and every production consumer in `worldwake-ai` accepts `&OrderedRanked<'_>` — but the invariant is still enforced only socially. Three gaps remain:

1. **Intra-crate direct sort**: `compare_ranked_goals` is `pub(crate)`, so any future module inside `worldwake-ai` can call `my_vec.sort_by(ranking::compare_ranked_goals)` and pass `&my_vec` to a consumer that takes `&OrderedRanked<'_>` by re-constructing it through... well, they *can't*, because `OrderedRanked::new` is module-private. But they can still call `compare_ranked_goals` directly and operate on raw `Vec<RankedGoal>` internally, creating a second comparator call site that is harder to audit. This is exactly the failure mode the S112 incident exhibited.
2. **Parallel comparator definitions**: nothing prevents a file under `crates/worldwake-ai/src/` other than `ranking.rs` from defining its own `fn compare_ranked_goals(left: &RankedGoal, right: &RankedGoal) -> Ordering`. The type system does not enforce "only `ranking::compare_ranked_goals` exists."
3. **Extra-crate synthesis**: `OrderedRanked::from_sorted_for_test` is `#[cfg(test)] pub(crate)`, which is correct — but without a compile-fail doctest, a reviewer has to trust the visibility qualifier rather than verify it. Similarly, `compare_ranked_goals` is `pub(crate)` today (not publicly importable), but demoting it to file-private tightens the contract and the compile-fail doctest makes the new contract falsifiable at build time.

This ticket closes all three gaps: (a) demotes `compare_ranked_goals` from `pub(crate)` to file-private so only `ranking.rs` (including its `#[cfg(test)] mod tests` child) can reach it; (b) demotes `RankingOutcome.ranked` from `pub` to `pub(crate)` (grep-verified to have zero external readers, so the demotion has zero blast radius); (c) adds two `compile_fail` doctests on `ranking.rs` proving `OrderedRanked::from_sorted_for_test` and `compare_ranked_goals` are unreachable from outside the crate; (d) adds a build-time regression test that walks `crates/worldwake-ai/src/` and fails if any file other than `ranking.rs` contains the byte sequence `fn compare_ranked_goals`.

After this ticket, the S112 failure mode is structurally impossible: the Rust type system + the grep regression + the compile-fail doctests jointly guarantee that only `ranking::compare_ranked_goals` defines the preference ordering and only `sort_in_place` / `RankingOutcome::ordered` can produce an `OrderedRanked` a downstream module will accept.

## Assumption Reassessment (2026-04-21)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Post-S123PREORDAUT-002 code state (assumed by Deps): every production `&[RankedGoal]` parameter in `worldwake-ai/src/**/*.rs` outside `ranking.rs` has migrated to `&OrderedRanked<'_>`. `compare_ranked_goals` is called from exactly `sort_in_place` + the in-file test module's 6 sites (ranking.rs:5265, 5322, 7210, 7237, 7260, 7287). `RankingOutcome.ranked` is still `pub` but is read only by in-crate callers (`observation.rs:274`, `decision_trace.rs:154/1587`, the many `ranking.rs` test assertions, and `crates/worldwake-ai/tests/golden_ai_decisions.rs:1670`). Verified by re-grep at 2026-04-21: zero `RankingOutcome` / `outcome.ranked` matches outside `worldwake-ai` today; this is stable because S123PREORDAUT-002 does not change external boundary.
2. Compile-fail doctest precedent: `crates/worldwake-ai/src/planning_snapshot.rs:399` and `:407` are the in-crate reference for this pattern. Both use `/// \`\`\`compile_fail` blocks that `cargo test --doc -p worldwake-ai` runs. The new doctests co-locate on `ranking.rs`'s module-level documentation.
3. Shared abstraction boundary under audit: the **visibility** contract on `compare_ranked_goals` and `RankingOutcome.ranked`, plus the **uniqueness** contract that only `ranking.rs` defines `fn compare_ranked_goals`. The type-system already prevents external `OrderedRanked::new` construction (from S123PREORDAUT-001's no-visibility-qualifier constructor); this ticket adds belt-and-suspenders falsification for the two remaining invariants.
4. Existing tests exercising the demoted-visibility symbols: the six in-file calls to `super::compare_ranked_goals` at `ranking.rs:5265, 5322, 7210, 7237, 7260, 7287` continue to compile because file-private items are accessible to any child module of the defining module (Rust visibility rule); `#[cfg(test)] mod tests` is such a child. Multiple `outcome.ranked[i].motive_score`-style assertions across `ranking.rs` tests (e.g. 3264, 3360, 3488, 3542, 3740, 4249, 4284) continue to compile because they are in-crate readers; `pub(crate)` still permits in-crate reads. No test modifications required.
5. Heuristic-removal / substrate check (per precision rule 12): this ticket does not remove any heuristic. The demotions tighten visibility around an already-authoritative comparator; no substrate is being replaced. The grep regression is net-new tooling whose closest in-repo precedent is the compile-fail pattern in `planning_snapshot.rs` — the grep walk adds a category of build-time check not previously used in this crate.

## Architecture Check

1. Why this belongs in a separate ticket from the migration: reviewer attention. The migration in S123PREORDAUT-002 is a ~400-line mechanical diff across 8 files; the invariant-lock is a ~20-line visibility tweak plus compile-fail doctests and a grep regression. Combining them buries the invariant boundary inside the mechanical diff. Splitting keeps the two review surfaces distinct.
2. No backwards-compatibility shims: visibility demotion replaces the old contract outright. No deprecated alias, no re-export at `worldwake-ai::ranking::compare_ranked_goals`, no `pub use` bridge.
3. Why grep regression over Clippy custom lint: Clippy lints require an external crate, custom lint infrastructure, and a toolchain dependency that this project explicitly avoids per `specs/S123-preference-ordering-authority.md` Non-Goals. A `#[test]` that walks `CARGO_MANIFEST_DIR` and fails the build on a byte-sequence match is ~15 lines of `std::fs` and runs under the ordinary `cargo test` harness. It co-locates with `ranking.rs` where the authoritative comparator lives, making the enforcement surface impossible to miss in review.
4. Why compile-fail doctests rather than `#[deny]` or `#[forbid]` attributes: the invariant "outside code cannot import `compare_ranked_goals`" is a visibility property of the symbol, not a lint category. The Rust compiler already enforces it once the symbol is file-private; the compile-fail doctest turns the enforcement into a falsifiable build-time check (FND-31) that fails `cargo test --doc` if someone re-exports or re-promotes the symbol.

## Verification Layers

1. Visibility of `compare_ranked_goals` post-demotion -> compile-fail doctest block in `ranking.rs` module docs: `use worldwake_ai::ranking::compare_ranked_goals;` must not compile.
2. Visibility of `OrderedRanked::from_sorted_for_test` from outside the crate -> compile-fail doctest: `use worldwake_ai::ranking::OrderedRanked; OrderedRanked::from_sorted_for_test(&[])` must not compile.
3. Uniqueness of `fn compare_ranked_goals` in `worldwake-ai/src/` -> D6 grep regression test (`compare_ranked_goals_is_the_only_impl_in_crate`) walks every `.rs` under `CARGO_MANIFEST_DIR/src/` (excluding `ranking.rs`) and fails if the byte sequence `fn compare_ranked_goals` appears.
4. Visibility of `RankingOutcome.ranked` -> no external-reader regression possible because grep confirmed zero extra-crate readers before demotion; in-crate readers remain valid under `pub(crate)`. Verified by `cargo test --workspace` passing.
5. Single-layer ticket on production behaviour: no decision-trace, action-trace, event-log, or authoritative-state change. The proof surfaces for this ticket are (a) `cargo test --doc -p worldwake-ai` running the compile-fail blocks, (b) `cargo test -p worldwake-ai ranking::tests::compare_ranked_goals_is_the_only_impl_in_crate`, (c) `cargo test --workspace` passing unchanged (demotions have zero behavioural effect).

## What to Change

### 1. Demote `compare_ranked_goals` to file-private

At `crates/worldwake-ai/src/ranking.rs:1923`, change:
```rust
pub(crate) fn compare_ranked_goals(left: &RankedGoal, right: &RankedGoal) -> Ordering {
```
to:
```rust
fn compare_ranked_goals(left: &RankedGoal, right: &RankedGoal) -> Ordering {
```

No changes required at the six in-file test call sites (ranking.rs:5265, 5322, 7210, 7237, 7260, 7287) — file-private items are visible to `#[cfg(test)] mod tests` as a child module.

### 2. Demote `RankingOutcome.ranked` to `pub(crate)`

At `crates/worldwake-ai/src/ranking.rs:38`, change:
```rust
pub struct RankingOutcome {
    /// Ranked goals after all filters (sorted by ranking order).
    pub ranked: Vec<RankedGoal>,
    pub(crate) suppressed: Vec<crate::candidate_generation::CandidateSuppressionDiagnostic>,
    pub zero_motive: Vec<GoalKey>,
}
```
to:
```rust
pub struct RankingOutcome {
    /// Ranked goals after all filters (sorted by ranking order).
    pub(crate) ranked: Vec<RankedGoal>,
    pub(crate) suppressed: Vec<crate::candidate_generation::CandidateSuppressionDiagnostic>,
    pub zero_motive: Vec<GoalKey>,
}
```

`zero_motive` stays `pub` because it is not an ordering-bearing field and the scope-tightening on `ranked` is enough. `RankingOutcome::into_ranked` (existing at ranking.rs:48) and `RankingOutcome::ordered` (added in S123PREORDAUT-001) remain `pub` and provide the external-consumer surface; neither currently has extra-crate callers, but the API stays available for future use via the authoritative paths only.

### 3. Add compile-fail doctests to `ranking.rs` module docs

Add at the top-of-file module docstring (or just above `OrderedRanked`'s definition, matching the `planning_snapshot.rs` precedent):

```rust
//! # Preference-ordering authority
//!
//! `ranking::compare_ranked_goals` is the sole authoritative total order on
//! `RankedGoal`. It is file-private and therefore unreachable from outside
//! this module.
//!
//! ```compile_fail
//! use worldwake_ai::ranking::compare_ranked_goals;
//! ```
//!
//! `OrderedRanked::from_sorted_for_test` is the in-crate test escape hatch
//! and is not reachable from outside `worldwake-ai`.
//!
//! ```compile_fail
//! use worldwake_ai::ranking::OrderedRanked;
//! use worldwake_ai::RankedGoal;
//! let empty: &[RankedGoal] = &[];
//! let _ = OrderedRanked::from_sorted_for_test(empty);
//! ```
```

### 4. Add D6 single-comparator grep regression

Add to `ranking.rs`'s `#[cfg(test)] mod tests`:

```rust
#[test]
fn compare_ranked_goals_is_the_only_impl_in_crate() {
    use std::fs;
    use std::path::Path;

    let src_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let needle = b"fn compare_ranked_goals";
    let mut offending = Vec::new();

    fn walk(dir: &Path, offending: &mut Vec<String>, needle: &[u8]) {
        for entry in fs::read_dir(dir).expect("read_dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, offending, needle);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                // Skip ranking.rs itself — it IS allowed to define the fn.
                if path.file_name().and_then(|s| s.to_str()) == Some("ranking.rs") {
                    continue;
                }
                let bytes = fs::read(&path).expect("read file");
                if bytes.windows(needle.len()).any(|w| w == needle) {
                    offending.push(path.display().to_string());
                }
            }
        }
    }

    walk(&src_root, &mut offending, needle);

    assert!(
        offending.is_empty(),
        "`fn compare_ranked_goals` must only be defined in ranking.rs; \
         found parallel definitions in: {offending:?}"
    );
}
```

Belt-and-suspenders with the newtype: a future PR that tries to re-introduce a parallel comparator fails this test at `cargo test -p worldwake-ai ranking::tests::compare_ranked_goals_is_the_only_impl_in_crate`.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — demote `compare_ranked_goals` visibility, demote `RankingOutcome.ranked` visibility, add 2 compile-fail doctests to module docs, add `compare_ranked_goals_is_the_only_impl_in_crate` test)

## Out of Scope

- Any further parameter migration (complete in S123PREORDAUT-002).
- Demoting `RankingOutcome::into_ranked`, `RankingOutcome::ordered`, `RankingOutcome::zero_motive`, or any other pub API beyond the two named fields.
- Adding similar invariant locks for unrelated orderings (e.g., `goal_model.rs:2125`'s unrelated place sort).
- Clippy custom lints (rejected in spec Non-Goals).
- Renaming `OrderedRanked` → `OrderedAgenda` — lives in S115.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --doc -p worldwake-ai` — the two new `compile_fail` doctests pass (they compile successfully when their body does NOT compile, which is the expected property).
2. `cargo test -p worldwake-ai ranking::tests::compare_ranked_goals_is_the_only_impl_in_crate` — passes (no file under `src/` other than `ranking.rs` contains `fn compare_ranked_goals`).
3. `cargo test -p worldwake-ai` — every pre-existing test passes unchanged. The six in-file uses of `super::compare_ranked_goals` in `ranking.rs`'s test module continue to work (file-private visibility allows child-module access).
4. `cargo test --workspace` — passes. Zero external readers of `RankingOutcome.ranked` exist, so the `pub` → `pub(crate)` demotion has no cross-crate blast radius.
5. `cargo clippy --workspace --all-targets -- -D warnings` — passes.

### Invariants

1. `compare_ranked_goals` is file-private after this ticket. An external import (`use worldwake_ai::ranking::compare_ranked_goals;`) fails to compile.
2. `OrderedRanked::from_sorted_for_test` is unreachable from outside the crate. External construction fails to compile.
3. No file under `crates/worldwake-ai/src/` other than `ranking.rs` contains `fn compare_ranked_goals` (enforced at `cargo test` time by `compare_ranked_goals_is_the_only_impl_in_crate`).
4. `RankingOutcome.ranked` is `pub(crate)`. External code must use `RankingOutcome::ordered()` (read-only view) or `RankingOutcome::into_ranked()` (consume and take ownership).
5. Behaviour of every pre-existing unit, integration, and golden test is identical to pre-ticket — the demotions affect only visibility, not runtime semantics.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` module docs — two new `compile_fail` doctest blocks proving external unreachability of `compare_ranked_goals` and `OrderedRanked::from_sorted_for_test`.
2. `crates/worldwake-ai/src/ranking.rs` (`#[cfg(test)] mod tests`) — `compare_ranked_goals_is_the_only_impl_in_crate` walks `CARGO_MANIFEST_DIR/src/` and fails on parallel `fn compare_ranked_goals` definitions.

### Commands

1. `cargo test --doc -p worldwake-ai` — runs the compile-fail blocks.
2. `cargo test -p worldwake-ai ranking::tests::compare_ranked_goals_is_the_only_impl_in_crate` — runs the single-comparator grep regression.
3. `cargo test -p worldwake-ai` — full crate suite (unit + integration + golden).
4. `cargo test --workspace` — whole-workspace build and test (proves the `pub(crate)` demotion has zero external blast radius).
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `scripts/verify.sh` — before pushing.
