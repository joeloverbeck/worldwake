# S25-002: Integrate feasibility into candidate ordering

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — modifies ranking comparator and agent_tick integration point
**Deps**: S25-001

## Problem

After S25-001 adds `FeasibilityHint` and `feasibility_hint()`, the annotation must be wired into `process_agent()` so candidates are reordered by feasibility within each priority class before the top `max_candidates_to_plan` are fed to the GOAP planner.

## Assumption Reassessment (2026-03-25)

1. `compare_ranked_goals()` in `crates/worldwake-ai/src/ranking.rs:688-705` is module-private (`fn`, not `pub`). It sorts by: priority_class → motive_score → discriminant → commodity → entity → place. S25-002 inserts feasibility between priority_class and motive_score. The function must be made `pub(crate)` for reuse in `agent_tick/mod.rs`.
2. `process_agent()` in `crates/worldwake-ai/src/agent_tick/mod.rs:384-490`. The ranked candidates are produced at line 405 (`read_result.ranked`). The deferred NoCriticalThreat evaluation runs at lines 407-439. The active-action phase starts at line 449. The feasibility annotation goes between lines 439 and 441 per spec.
3. `runtime_belief_view()` is already called at line 419 and line 444 — it constructs references without allocation. A third call for feasibility annotation is cheap.
4. `blocked_memory` (type `BlockedIntentMemory`) is a local mutable variable in `process_agent()`, available at the insertion point.
5. `current_frame` (type `Option<IntentionFrame>`) is a local variable, available at the insertion point.
6. The ordering layer being modified is the sort key in `compare_ranked_goals`. The change is additive (inserting a new comparison tier), not removing or weakening an existing one. Existing behavior is preserved when all feasibility hints are `Uncertain` (the initial state before annotation).

## Architecture Check

1. Making `compare_ranked_goals` `pub(crate)` is cleaner than duplicating the sort logic. The function is already the single source of truth for ranking order.
2. No backward-compatibility shims. The initial `Uncertain` feasibility on all goals means the first sort in `rank_candidates()` is unchanged. Only the re-sort after annotation reorders.

## Verification Layers

1. Feasibility inserted in sort order: focused unit test in `ranking.rs` confirming `Likely` same-class goal outranks `Unlikely` same-class goal, but `Unlikely` Critical still outranks `Likely` Low
2. Agent selects local food over remote food: golden test behavior verification (existing goldens should improve or stay the same)
3. Re-sort happens after deferred NoCriticalThreat evaluation: code-path inspection (insertion point is between lines 439 and 441)

## What to Change

### 1. Update `compare_ranked_goals()` in `ranking.rs`

- Make `compare_ranked_goals` `pub(crate)`.
- Insert `.then_with(|| left.feasibility.cmp(&right.feasibility))` between the `priority_class` comparison and the `motive_score` comparison. Since `Likely < Uncertain < Unlikely` (derived Ord), and we want `Likely` first, the natural ascending order on feasibility is correct here (lower enum value = better = sorts first).

### 2. Annotate and re-sort in `process_agent()` in `agent_tick/mod.rs`

After the deferred NoCriticalThreat evaluation block (after line 439), before `let active_action = ...` (line 441):

```rust
// ── Feasibility annotation and re-sort ──
{
    let view = runtime_belief_view(agent, ctx.world, ctx.scheduler, action_defs);
    for ranked in &mut ranked_candidates {
        ranked.feasibility = feasibility_hint(
            &view, agent, ranked, &blocked_memory,
            current_frame.as_ref(), tick,
        );
    }
    ranked_candidates.sort_by(compare_ranked_goals);
}
```

This requires importing `feasibility_hint` and `compare_ranked_goals` into `agent_tick/mod.rs`.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — update `compare_ranked_goals` visibility and sort key)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — add feasibility annotation block)

## Out of Scope

- The `FeasibilityHint` enum and `feasibility_hint()` function (S25-001)
- Decision trace integration (S25-003)
- Budget allocation changes for Unlikely goals (future spec, per S25 spec)
- Golden test documentation changes (S25-004)
- Changes to `rank_candidates()` itself — the initial sort is unchanged since all goals start as `Uncertain`

## Acceptance Criteria

### Tests That Must Pass

1. `test_feasibility_tiebreak_within_priority_class` — within same priority class: Likely(motive=600) outranks Unlikely(motive=900)
2. `test_feasibility_does_not_cross_priority_class` — Critical+Unlikely outranks Low+Likely
3. `test_same_feasibility_falls_through_to_motive` — within same priority class and same feasibility: higher motive wins
4. Existing suite: `cargo test -p worldwake-ai` — all tests pass (golden tests may show improved behavior but must not regress)

### Invariants

1. Feasibility reordering is strictly within a `GoalPriorityClass` — never across classes
2. All goals remain in the candidate list — no goal is excluded by feasibility
3. The annotation runs after deferred NoCriticalThreat evaluation but before active-action evaluation
4. `rank_candidates()` initial sort is unaffected (all `Uncertain`)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` (inline tests) — 3 focused sort-order tests verifying feasibility tiebreaking within/across priority classes

### Commands

1. `cargo test -p worldwake-ai ranking` — run ranking tests including new ones
2. `cargo test -p worldwake-ai` — full AI crate (includes all golden tests)
3. `cargo clippy -p worldwake-ai` — no new warnings

## Outcome

- **Completion date**: 2026-03-25
- **What changed**:
  - `ranking.rs`: `compare_ranked_goals` made `pub(crate)`, feasibility tier inserted between `priority_class` and `motive_score` in sort comparator.
  - `agent_tick/mod.rs`: Feasibility annotation block added after deferred NoCriticalThreat evaluation, before active-action phase. Annotates each ranked candidate via `feasibility_hint()` then re-sorts.
  - 3 new focused sort-order tests added in `ranking.rs`.
- **Deviations from original plan**: None.
- **Verification**: 955 tests pass (0 failures), clippy clean. All golden tests pass with no regressions.
