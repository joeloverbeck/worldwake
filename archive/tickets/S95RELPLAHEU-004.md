# S95RELPLAHEU-004: Search integration — two-pass RPG heuristic and helpful actions

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — search expansion loop in worldwake-ai
**Deps**: archive/tickets/S95RELPLAHEU-001.md, archive/tickets/S95RELPLAHEU-002.md, archive/tickets/S95RELPLAHEU-003.md

## Problem

The RPG algorithm (ticket 003) exists but is not connected to the search loop. This ticket wires `compute_ff_heuristic` into the expansion site, retroactively updates successor heuristic values, uses helpful-action indices for preferred-operator selection, and populates the decision trace fields (ticket 002) with live RPG data.

## Assumption Reassessment (2026-04-12)

1. The expansion loop in `crates/worldwake-ai/src/search/mod.rs:480-529` collects `successor_operators` during the candidate loop. After line 529, landmarks are extracted (lines 612-627) and preferred operators are computed (lines 637-646). The RPG integration follows the same post-loop pattern.
2. `build_successor_detailed` in `transition.rs:199-206` sets `heuristic_ticks = spatial_heuristic.max(landmark_heuristic)` during successor construction. The RPG integration must retroactively update this value after `successor_operators` are fully collected.
3. `preferred_operators()` function at `landmarks.rs:178` returns `BTreeSet<usize>` — the same type as `RelaxedPlanResult.helpful_action_indices`. The substitution is type-compatible.
4. `cognitive.use_ff_heuristic` now exists on `CognitiveProfile` after ticket 001. `SearchExpansionSummary.ff_heuristic` and `.helpful_action_count` now exist after ticket 002.
5. `compute_ff_heuristic` now exists after ticket 003 with signature `(&BTreeSet<PlanningFact>, &BTreeSet<PlanningFact>, &[PlanningOperator]) -> Option<RelaxedPlanResult>`.
6. `planning_facts_from_state` at `landmarks.rs:40` and `tactical_goal.goal_facts()` at `mod.rs:136-154` are already called in the expansion site for landmark extraction — same inputs reused for RPG.
7. `compute_heuristic` at `heuristic.rs:20-33` computes the spatial heuristic. For the retroactive update, each successor needs its spatial-only component. Currently `heuristic_ticks = max(spatial, landmark)` — the spatial component is not stored separately. The retroactive update will need to recompute spatial heuristic per successor or store it during initial construction.
8. `crates/worldwake-ai/src/search/tests.rs` already has owned expansion-summary trace coverage (`search_expansion_summaries_collected_when_tracing_enabled`, `search_trace_metadata_records_two_phase_strategic_and_landmark_details`, `search_trace_metadata_zero_landmarks_reports_zero_counts`, `beam_truncation_visible_in_expansion_summary`), so this ticket can extend those proofs and add only the additional FF-specific behavior cases it still needs.

## Architecture Check

1. The two-pass pattern (build successors → compute RPG → retroactively update) avoids modifying `build_successor_detailed`'s signature, keeping the change localized to `mod.rs`. The alternative (passing h_ff into `build_successor_detailed`) creates a chicken-and-egg problem since the RPG needs `successor_operators` that are only available after successor construction.
2. When FF is enabled and produces a result, `h_ff` replaces `landmark_heuristic` in the formula. This is sound because `h_ff` is strictly more informative than landmark count. When FF returns `None` (dead end), the existing landmark-based heuristic and preferred operators are preserved as fallback.
3. No backward-compatibility shims. The `use_ff_heuristic: false` path is identical to pre-S95 behavior.

## Verification Layers

1. h_ff replaces landmark_heuristic in successor heuristic → integration test asserting `heuristic_ticks = max(spatial, h_ff)` on successor nodes
2. Helpful actions replace preferred_operators when FF active → integration test checking preferred flag assignment
3. FF disabled produces None in trace → integration test with `use_ff_heuristic: false`
4. Dead-end fallback to landmarks → integration test where RPG returns None
5. Decision trace populated → integration test checking `ff_heuristic` and `helpful_action_count` in expansion summaries

## What to Change

### 1. RPG computation after successor collection

In `crates/worldwake-ai/src/search/mod.rs`, after the landmark extraction block (lines ~612-633), add the RPG computation block:

```rust
// Compute FF heuristic when enabled
let ff_result = if cognitive.use_ff_heuristic && !successor_operators.is_empty() {
    let current_facts = planning_facts_from_state(&node.state);
    let goal_facts = tactical_goal
        .as_ref()
        .map(|tg| tg.goal_facts(goal, &node.state, recipes))
        .unwrap_or_default();
    if !goal_facts.is_empty() {
        compute_ff_heuristic(&current_facts, &goal_facts, &successor_operators)
    } else {
        None
    }
} else {
    None
};
```

### 2. Retroactive heuristic update

When `ff_result` is `Some`, iterate over `successors` and recompute each successor's `heuristic_ticks`:

- Recompute spatial heuristic for each successor: call `compute_heuristic(snapshot, &successor.state, &combined_places.places)`.
- Set `successor.heuristic_ticks = spatial_h.max(ff_result.h_ff)`.

This replaces the `landmark_heuristic` component that was set during `build_successor_detailed`.

### 3. Helpful action preferred operator substitution

When `ff_result` is `Some`, replace the `preferred_operators()` call block (lines ~635-646) with:

```rust
for (index, (_, _, _, preferred)) in successors.iter_mut().enumerate() {
    *preferred = ff_result.helpful_action_indices.contains(&index);
}
```

When `ff_result` is `None`, the existing `preferred_operators()` block runs as fallback.

### 4. Decision trace population

In the `SearchExpansionSummary` construction sites within `mod.rs` (lines ~554, ~678), populate:
- `ff_heuristic: ff_result.as_ref().map(|r| r.h_ff)`
- `helpful_action_count: ff_result.as_ref().map_or(0, |r| r.helpful_action_indices.len() as u16)`

### 5. Integration tests

Add tests 7-9 from the spec in the `search/` test module:

7. **FF vs spatial heuristic combination**: Build a scenario where h_ff > spatial_h, verify successor `heuristic_ticks = h_ff`. Build another where spatial_h > h_ff, verify `heuristic_ticks = spatial_h`.
8. **FF disabled via profile**: Agent with `use_ff_heuristic: false` → `ff_heuristic: None` in all expansion summaries.
9. **Fallback on dead end**: Scenario where RPG cannot reach goal facts → search uses landmark-based preferred operators and `ff_heuristic: None`.

## Files to Touch

- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify — integration tests)

## Out of Scope

- Per-successor RPG computation (spec Non-Goal — h_ff is per-expansion)
- Weighted A* or anytime search
- Cached RPGs across expansions
- LMCut or operator-counting heuristics
- Golden test assertions (ticket 005)

## Acceptance Criteria

### Tests That Must Pass

1. Successor heuristic_ticks reflects `max(spatial_h, h_ff)` when FF is enabled and RPG succeeds
2. Helpful action indices correctly mark preferred successors
3. `use_ff_heuristic: false` produces `ff_heuristic: None` in expansion summaries
4. Dead-end fallback uses landmark-based preferred operators
5. Decision trace `ff_heuristic` and `helpful_action_count` populated correctly
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. When `use_ff_heuristic` is `false`, behavior is identical to pre-S95
2. RPG computed per-expansion (not cached across expansions)
3. Determinism preserved — all RPG operations use BTreeSet/BTreeMap
4. Spatial heuristic remains the floor (`max` ensures it's never undercut)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — extend existing expansion-summary trace tests and add focused FF-specific integration tests (heuristic combination, FF disabled, dead-end fallback)

### Commands

1. `cargo test -p worldwake-ai -- search`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-04-12
- Integrated FF relaxed-plan guidance into `crates/worldwake-ai/src/search/mod.rs` with a dedicated post-successor helper that computes `h_ff`, rewrites successor heuristic floors to `max(spatial_h, h_ff)`, and substitutes helpful-action preferred flags whenever FF returns a live result.
- Preserved landmark behavior as the fallback when FF is disabled or the relaxed plan is unreachable for that expansion, while populating live `ff_heuristic` and `helpful_action_count` fields in both expansion-summary construction paths.
- Extended `crates/worldwake-ai/src/search/tests.rs` with direct helper proofs for both `h_ff > spatial_h` and `spatial_h > h_ff`, plus trace-level proofs for FF-enabled population, FF-disabled inert fields, and dead-end fallback to landmark guidance.

## Deviations

- The helper integration landed as a small extracted function (`apply_ff_heuristic_to_successors`) rather than as an inline post-loop block. This kept the two-pass logic localized without widening `build_successor_detailed`.
- The focused verification command from the draft was narrowed with module-qualified exact selectors after `cargo test -p worldwake-ai -- --list` confirmed that bare substring selectors would truthfully compile targets but run zero tests.

## Verification Result

- Passed: `cargo test -p worldwake-ai search::tests::ff_successor_rewrite_uses_relaxed_plan_when_it_exceeds_spatial_heuristic -- --exact`
- Passed: `cargo test -p worldwake-ai search::tests::search_trace_metadata_records_ff_heuristic_and_helpful_actions_when_enabled -- --exact`
- Passed: `cargo test -p worldwake-ai search::tests::ff_successor_rewrite_preserves_spatial_heuristic_when_it_exceeds_relaxed_plan -- --exact`
- Passed: `cargo test -p worldwake-ai`
- Passed: `cargo test --workspace`
- Passed: `cargo clippy --workspace --all-targets -- -D warnings`
