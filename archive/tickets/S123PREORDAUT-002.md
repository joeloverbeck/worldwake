# S123PREORDAUT-002: Migrate call sites from &[RankedGoal] to &OrderedRanked<'_>

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — mechanical parameter migration across 8 production files + 1 test-helper in `worldwake-ai` (no behaviour change; signatures only)
**Deps**: archive/tickets/S123PREORDAUT-001.md

## Outcome

- Migrated the remaining `worldwake-ai` ordered-ranking consumer surface from raw `&[RankedGoal]` / `Option<&[RankedGoal]>` to `&OrderedRanked<'_>` / `Option<&OrderedRanked<'_>>` across `plan_selection.rs`, `side_benefit.rs`, `interrupts.rs`, `agent_tick/{active_action,frame,planning,portfolio,mod}.rs`, and the in-tree helper in `agent_tick/tests.rs`.
- Replaced the post-feasibility `sort_by(compare_ranked_goals)` in `agent_tick/mod.rs` with `ranking::sort_in_place(&mut ranked_candidates)` and threaded the ordered view through interrupt evaluation, planning, tracing, and goal-switch inference. The deferred `NoCriticalThreat` assumption now also evaluates against an ordered view before feasibility mutation, then the authoritative post-feasibility re-sort rebuilds the ordered view for the rest of the tick.
- Updated non-`ranking.rs` tests to construct ordered fixtures through `sort_in_place` or `OrderedRanked::from_sorted_for_test`, preserving the production invariant without widening the public construction surface.

## Verification Result

- Passed `cargo fmt --all`
- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-ai --test golden_portfolio_planning`
- Passed `cargo test -p worldwake-ai --test golden_ai_decisions`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`

## Problem

S123PREORDAUT-001 introduces `OrderedRanked<'a>`, `ranking::sort_in_place`, and `RankingOutcome::ordered` — but until every `&[RankedGoal]` consumer inside `worldwake-ai` migrates to `&OrderedRanked<'_>`, nothing in the crate actually consumes the new API. A module could still synthesise a `Vec<RankedGoal>`, sort it with its own comparator, and hand `&vec[..]` to any of the 25+ parameter sites that currently accept `&[RankedGoal]`. The S112 regression was precisely that class: `agent_tick/portfolio.rs` shipped a parallel comparator and the divergence silently passed review.

This ticket flips every `&[RankedGoal]` production parameter in `worldwake-ai` to `&OrderedRanked<'_>` (and `Option<&[RankedGoal]>` → `Option<&OrderedRanked<'_>>` in the one site that uses the optional shape). It also updates `agent_tick/mod.rs:883` from the bare `sort_by(compare_ranked_goals)` call to `let ordered = ranking::sort_in_place(&mut ranked_candidates);` and threads `&ordered` down to every downstream helper, plus migrates in-tree test fixtures outside `ranking.rs` to construct ordered views through `sort_in_place` or `from_sorted_for_test`. After this ticket `compare_ranked_goals` is no longer called from any `worldwake-ai` module other than `ranking.rs` itself, setting the stage for S123PREORDAUT-003's visibility demotion + invariant-lock.

The migration is strictly mechanical — post-migration behaviour is identical to `main`. FND-28 forbids a dual-signature shim, so every site flips in this ticket.

## Assumption Reassessment (2026-04-21)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Authoritative enumeration of `&[RankedGoal]` parameter sites in `worldwake-ai/src` via `grep '\[(crate::)?RankedGoal\]'`:
   - `plan_selection.rs:26` — `select_best_plan`
   - `side_benefit.rs:24, 79` — `detect_side_benefits`, `build_plan_value`
   - `agent_tick/planning.rs:232, 342, 655, 666, 714, 856, 931, 1101, 1134, 1162, 1325` — 11 sites (`selected_plan_value`, `build_candidate_plans`, `ranked_goal_for_opportunity`, `summarize_snapshot_continuation`, `try_continue_snapshot_plan`, `build_rejected_alternatives`, `emit_plan_selection_events`, `adopt_selected_plan`, `clear_current_plan`, `plan_and_validate_next_step`, traced helper pair)
   - `agent_tick/active_action.rs:59` — `handle_active_action_phase` (spec line D3 said line 50; drift)
   - `agent_tick/mod.rs:334` — `infer_goal_switch_reason` (spec did not enumerate this site; added here)
   - `agent_tick/frame.rs:343` — `evaluate_assumptions`, `Option<&[RankedGoal]>` shape (spec said line 339; drift)
   - `interrupts.rs:35, 135, 222, 280, 290` — 5 sites (`evaluate_interrupt`, `interrupt_freely`, `relation_aware_interrupt_candidate`, `best_challenger`, `current_priority`). Spec line D3 enumerated only 4; it missed line 222 (`relation_aware_interrupt_candidate`).
   - `agent_tick/portfolio.rs:35, 98, 128` — `assemble_portfolio` + two file-private helpers (spec said 34, 97, 127; drift)
   - `agent_tick/tests.rs:1097` — `has_goal` test helper (not enumerated by spec; migrated for consistency)
   Total: 25 production sites + 1 test-helper = 26 sites. Two of these (`interrupts.rs:222`, `agent_tick/mod.rs:334`) are additions beyond the spec's enumeration.
2. Cross-crate reader scan for `RankingOutcome` / `outcome.ranked`: zero matches in `crates/worldwake-sim`, `crates/worldwake-cli`, and `crates/worldwake-ai/tests/`. The field remains `pub` after this ticket; demotion to `pub(crate)` is in S123PREORDAUT-003.
3. Shared abstraction boundary under audit: the `&[RankedGoal]` parameter shape at every consumer site. Post-ticket, the sole producer paths are `ranking::sort_in_place` at `agent_tick/mod.rs:883` and (later) `RankingOutcome::ordered` at points where the outcome is still in scope. `observation.rs:274` continues to move `outcome.ranked` into `ReadPhaseResult.ranked`, which remains a `Vec<RankedGoal>` (storage, not view); downstream consumers call `ranking::sort_in_place` on the stored vec when they need an ordered view. The re-sort at `agent_tick/mod.rs:883` is preserved because `FeasibilityHint` legitimately mutates within a tick.
4. Mismatch + correction — spec drift and undercount: (a) line numbers in `portfolio.rs`, `active_action.rs`, and `frame.rs` drifted since the spec was last edited; (b) the spec missed `interrupts.rs:222` and `agent_tick/mod.rs:334`. Both missed sites fall within the spec's stated intent ("every `&[RankedGoal]` parameter site ... migrate to `&OrderedRanked<'_>`"), so they are required consequences of this ticket — not separate bugs, not future cleanup.
5. Existing tests exercising the migrating functions (checked in `ranking.rs` `#[cfg(test)] mod tests` + sibling test modules): `test_feasibility_does_not_cross_priority_class` (ranking.rs:7220), `test_same_feasibility_falls_through_to_motive` (ranking.rs:7243), `critical_remote_food_can_outrank_local_wash_on_motive` (ranking.rs:7266), `survival_slot_picks_highest_motive_survival` (portfolio.rs:285), the portfolio-tie regression tests added in the S112 fix, and the golden suite at `crates/worldwake-ai/tests/golden_portfolio_planning.rs` (`portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal`). All use `super::compare_ranked_goals` from inside `ranking.rs`'s own test module (lines 5265, 5322, 7210, 7237, 7260, 7287) — those calls remain valid and unchanged because `compare_ranked_goals` is still `pub(crate)` after this ticket (demotion to file-private is S123PREORDAUT-003).

## Architecture Check

1. Why one ticket, not seven (one per file): FND-28 forbids a dual-signature shim. An intermediate state where some consumers take `&OrderedRanked<'_>` and others still take `&[RankedGoal]` would require `OrderedRanked::as_slice()` bridges at every mixed boundary — defeating the invariant exactly where migration is incomplete. A single mechanical migration ticket compiles cleanly at both endpoints; the diff is large but uniformly mechanical (pattern: `&[RankedGoal]` → `&OrderedRanked<'_>` + iteration-site adjustments `.iter()`, `.first()`, `.find(...)`, `.as_slice()` for index-style access, `.is_empty()`, `.len()`).
2. Why this ticket does not include the visibility demotion: keeping the migration atomic and separating the invariant-lock keeps the review surfaces distinct. The migration is a 400-line mechanical diff; the lock is a 3-line visibility tweak plus compile-fail doctests and a grep regression. Bundling them loses review signal on the invariant boundary.
3. No backwards-compatibility aliasing: every migrated signature flips in this ticket. No `impl From<&[RankedGoal]> for OrderedRanked` bridge, no `as_slice()` wrapper at module boundaries to preserve old signatures.

## Verification Layers

1. Behavioural equivalence to pre-ticket `main` -> `cargo test -p worldwake-ai` (all 700+ existing unit and integration tests) passes identically. Golden suites (`survival-baseline.ron`, `survival-contested.ron`, `survival-scattered.ron`, `golden_portfolio_planning.rs`) pass unchanged.
2. Signature boundary under audit: every `&[RankedGoal]` parameter site in `worldwake-ai/src/**/*.rs` (excluding `ranking.rs`'s own internal helpers) has migrated -> grep audit: `grep -R '\[(crate::)?RankedGoal\]' crates/worldwake-ai/src/` returns zero matches in production code outside `ranking.rs` after this ticket. Single matches may remain in comments/docstrings; none in `fn` signatures.
3. No same-tick semantic drift: the re-sort at `agent_tick/mod.rs:883` is replaced by `ranking::sort_in_place(&mut ranked_candidates)`, which is defined as `ranked.sort_unstable_by(compare_ranked_goals); OrderedRanked::new(...)`. Stability change (`sort_by` → `sort_unstable_by`) does not affect output because the comparator chain is total (no ties below the `OpportunityKey` tiebreaker in `ranked_goal_ordering`). Verified by existing in-file `sort_in_place_matches_ranker_output` test from S123PREORDAUT-001.
4. Single-layer ticket on semantics, multi-layer on files: this is a type-signature refactor spanning 8 files in `worldwake-ai`; no decision-trace, action-trace, event-log, or authoritative-state mutation is touched. The only proof surface required beyond unit tests is the behavioural golden suite, because the rationale for the ticket is "the S112 regression survived until golden goldens ran." Goldens protect the exact post-S112-fix behaviour this ticket must preserve.

## What to Change

### 1. `agent_tick/mod.rs:883` — feasibility-annotation re-sort

Replace:
```rust
ranked_candidates.sort_by(crate::ranking::compare_ranked_goals);
```
with:
```rust
let ordered = crate::ranking::sort_in_place(&mut ranked_candidates);
```

Thread `&ordered` down to every migrated callsite in `agent_tick/mod.rs` that currently passes `&ranked_candidates` (approx. lines 840 `evaluate_assumptions` pre-sort call, 912, 994, 1334). Call sites that iterate (`ranked_candidates.first()`, `ranked_candidates.iter()`) migrate to `ordered.first()`, `ordered.iter()`. Indexing (`ranked_candidates.get(1)`, used at line 1112 for top-ranked comparison) migrates to `ordered.as_slice().get(1)` — explicit `.as_slice()` documents the read-only index intent.

`observation.rs:274` (`ranked: outcome.ranked,`) is unchanged — `ReadPhaseResult.ranked` stores the `Vec<RankedGoal>` for downstream `sort_in_place` consumption.

### 2. `agent_tick/planning.rs` — 11 parameter sites

Migrate 232, 342, 655, 666, 714, 856, 931, 1101, 1134, 1162, 1325 from `ranked_candidates: &[RankedGoal]` to `ranked_candidates: &OrderedRanked<'_>`. Internal uses of `ranked_candidates.iter()`, `ranked_candidates.first()`, `ranked_candidates.is_empty()`, `ranked_candidates.len()` require no change (identical method surface). Sites that call `ranked_candidates.as_ref()` or index directly need `.as_slice()`.

### 3. `agent_tick/portfolio.rs` — 3 parameter sites

Migrate line 35 (`assemble_portfolio`, the `pub(crate)` entrypoint) and the file-private helpers at lines 98 (`select_commitment_candidate`) and 128 (`select_best_candidate`). The spec comment at line 119 ("`ranked` is pre-sorted by `ranking::compare_ranked_goals`") is strengthened by the type signature; the comment can stay as context. Test fixtures inside `#[cfg(test)] mod tests` (e.g. `survival_slot_picks_highest_motive_survival` at line 285) that build a `Vec<RankedGoal>` manually via the `ranked_goal(...)` helper at line 243 switch to:

```rust
let mut ranked = vec![ /* … */ ];
let ordered = ranking::sort_in_place(&mut ranked);
assemble_portfolio(&ordered, committed, probe);
```

Or, for tests that assert a specific pre-sorted input order:

```rust
let ordered = ranking::OrderedRanked::from_sorted_for_test(&ranked);
```

### 4. `agent_tick/active_action.rs:59` — `handle_active_action_phase`

Migrate the `ranked_candidates: &[RankedGoal]` parameter to `&OrderedRanked<'_>`. No other surface in this file reads the slice; call-site adjustments propagate naturally from `agent_tick/mod.rs`.

### 5. `agent_tick/mod.rs:334` — `infer_goal_switch_reason` (spec-undercounted site)

Migrate `ranked_candidates: &[crate::RankedGoal]` to `ranked_candidates: &OrderedRanked<'_>`. The function body uses `.iter().find(...)` twice — identical method surface post-migration.

### 6. `agent_tick/frame.rs:343` — `evaluate_assumptions`

Migrate `ranked_candidates: Option<&[RankedGoal]>` to `ranked_candidates: Option<&OrderedRanked<'_>>`. The `None` case is unchanged; the `Some(slice)` case accesses `.iter()` or `.find()` on the inner reference — method surface is identical.

### 7. `plan_selection.rs:26` — `select_best_plan`

Migrate `candidates: &[RankedGoal]` to `candidates: &OrderedRanked<'_>`.

### 8. `interrupts.rs` — 5 parameter sites (not 4)

Migrate lines 35 (`evaluate_interrupt`), 135 (`interrupt_freely`), **222 (`relation_aware_interrupt_candidate` — spec-undercounted site)**, 280 (`best_challenger`), 290 (`current_priority`). Line 222 is a file-private helper called from `interrupt_freely` at line 160; migrating it preserves the invariant at every boundary within `interrupts.rs`. The `&'a [RankedGoal]` at line 222 becomes `&OrderedRanked<'a>` (lifetime parameter retained for the shared-borrow return).

### 9. `side_benefit.rs` — 2 parameter sites

Migrate lines 24 (`detect_side_benefits`) and 79 (`build_plan_value`).

### 10. `agent_tick/tests.rs:1097` — `has_goal` test helper

Migrate `fn has_goal(ranked: &[RankedGoal], goal: GoalKind) -> bool` to `fn has_goal(ranked: &OrderedRanked<'_>, goal: GoalKind) -> bool` for consistency across the test surface. All callers sort via `sort_in_place` before comparison.

### 11. Test fixtures outside `ranking.rs`

Any `#[cfg(test)]` helper that constructs `Vec<RankedGoal>` manually outside `ranking.rs` switches to `ranking::sort_in_place(&mut vec)` (if wanting authoritative sort) or `ranking::OrderedRanked::from_sorted_for_test(&vec)` (if asserting a specific pre-sorted input). Tests inside `ranking.rs`'s own `#[cfg(test)] mod tests` continue to call `super::compare_ranked_goals` directly — file-private visibility does not block child-module access.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — replace `sort_by` with `sort_in_place`; thread `&ordered` through downstream calls; migrate `infer_goal_switch_reason` at line 334)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — 11 parameter sites)
- `crates/worldwake-ai/src/agent_tick/portfolio.rs` (modify — 3 parameter sites + test fixtures)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify — 1 parameter site)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — 1 parameter site; `Option<…>` shape)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — `has_goal` test helper signature + any manual-Vec fixtures that now need `sort_in_place` or `from_sorted_for_test`)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — 1 parameter site)
- `crates/worldwake-ai/src/interrupts.rs` (modify — 5 parameter sites, incl. spec-undercounted line 222)
- `crates/worldwake-ai/src/side_benefit.rs` (modify — 2 parameter sites)

## Out of Scope

- Adding `OrderedRanked`, `sort_in_place`, `RankingOutcome::ordered` (scope of S123PREORDAUT-001, already landed by Deps).
- Demoting `compare_ranked_goals` from `pub(crate)` to file-private (scope of S123PREORDAUT-003).
- Demoting `RankingOutcome.ranked` from `pub` to `pub(crate)` (scope of S123PREORDAUT-003).
- Adding compile-fail doctests or the D6 grep regression (scope of S123PREORDAUT-003).
- Any rename (`RankedGoal` → `AgendaEntry`, `GroundedGoal` → `GoalOffer`, `OrderedRanked` → `OrderedAgenda`) — lives in S115.
- Changing the behaviour of `compare_ranked_goals` or the comparator chain — this is a pure parameter-shape migration.
- Removing the re-sort at `agent_tick/mod.rs:883` — it is required because `FeasibilityHint` mutates mid-tick.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — every pre-existing unit, integration, and golden test passes unchanged. Count and names identical to pre-ticket.
2. `cargo test -p worldwake-ai --test golden_portfolio_planning` — `portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` passes.
3. Golden behavioural suites: `survival-baseline.ron`, `survival-contested.ron`, `survival-scattered.ron` pass unchanged (these are the scenarios the S112 regression originally broke).
4. `cargo clippy --workspace --all-targets -- -D warnings` passes.

### Invariants

1. `grep -R '\[(crate::)?RankedGoal\]' crates/worldwake-ai/src/` returns zero matches in `fn` parameter positions outside `ranking.rs` (matches inside comments/docstrings permitted; matches inside `ranking.rs` itself permitted for internal helpers such as `rank_candidates_with_memories` which continue to accept `&[GroundedGoal]` and internally produce `Vec<RankedGoal>`).
2. `compare_ranked_goals` is called from exactly two locations post-ticket: (a) `ranking::sort_in_place` at the new entrypoint, (b) the in-file test module at the 6 pre-existing sites (ranking.rs:5265, 5322, 7210, 7237, 7260, 7287). Zero calls from `agent_tick/mod.rs` or any other file outside `ranking.rs`.
3. `OrderedRanked::new` retains no visibility qualifier — no call site outside `ranking::sort_in_place` or `RankingOutcome::ordered` constructs it.
4. Behavioural output of all golden scenarios and all pre-existing unit tests is identical to pre-ticket — this is a mechanical refactor, not a semantic change.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — `has_goal` helper signature migrates; no new test cases added.
2. `crates/worldwake-ai/src/agent_tick/portfolio.rs` (`#[cfg(test)] mod tests`) — fixture helpers that build `Vec<RankedGoal>` manually now wrap via `sort_in_place` or `from_sorted_for_test` before calling `assemble_portfolio`. Test assertions unchanged.
3. No new test cases — this ticket is a signature migration; behavioural coverage is already comprehensive via existing units + goldens.

### Commands

1. `cargo test -p worldwake-ai` — full AI-crate suite including goldens.
2. `cargo test -p worldwake-ai --test golden_portfolio_planning` — explicit check of the S112-regression golden.
3. `cargo test -p worldwake-ai --test golden_ai_decisions` — broader golden coverage.
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `scripts/verify.sh` — before pushing.
