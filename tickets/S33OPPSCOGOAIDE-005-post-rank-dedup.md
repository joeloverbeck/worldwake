# S33OPPSCOGOAIDE-005: Post-rank deduplication with exhaustion fallthrough

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new dedup step between ranking and plan search
**Deps**: S33OPPSCOGOAIDE-002, S33OPPSCOGOAIDE-004

## Problem

After S33OPPSCOGOAIDE-002, candidate generation emits multiple `GroundedGoal` per `GoalKey` (one per opportunity). Ranking assigns identical priority/motive to all opportunities for the same `GoalKey`. Without dedup, plan search would waste budget searching multiple opportunities for the same desire. The spec requires a post-rank dedup step that selects the top non-exhausted opportunity per `GoalKey`, falling through to the next if the top is exhausted.

## Assumption Reassessment (2026-03-28)

1. Before S33, dedup happened implicitly via `BTreeMap<GoalKey, GroundedGoal>` merging in candidate generation. After S33OPPSCOGOAIDE-002, candidates are `Vec<GroundedGoal>` — no implicit dedup.
2. `rank_candidates()` at `crates/worldwake-ai/src/ranking.rs:70` returns `Vec<RankedGoal>` sorted by `(priority_class, motive_score)`.
3. `RankedGoal` at `ranking.rs:1702` contains `grounded: GroundedGoal` — after S33OPPSCOGOAIDE-002, each `GroundedGoal` has an `anchor` field.
4. `build_candidate_plans()` at `planning.rs:146` takes `ranked_candidates: &[RankedGoal]` — it expects one candidate per `GoalKey` for budget efficiency.
5. After S33OPPSCOGOAIDE-004, exhaustion is keyed by `OpportunityKey`. The dedup step needs the exhaustion cache to check which opportunities are exhausted.
6. This is an AI-pipeline ticket. The shared boundary is between ranking output and plan-search input.

## Architecture Check

1. Post-rank dedup is cleaner than pre-rank dedup because: (a) ranking is desire-level (all opportunities for same GoalKey get same score), so filtering first would require running ranking separately; (b) post-rank dedup preserves the full ranked list for diagnostics/tracing before narrowing.
2. The alternative — dedup during ranking — mixes concern levels (ranking shouldn't know about exhaustion). Post-rank dedup is a separate, testable pass.
3. No backward-compatibility shims.

## Verification Layers

1. Top non-exhausted opportunity selected → focused unit test: two opportunities ranked, top exhausted, second selected.
2. All-exhausted GoalKey yields no candidate → focused unit test: both opportunities exhausted, GoalKey absent from dedup output.
3. Single-opportunity GoalKey unaffected → focused unit test: no regression for single-source scenarios.

## What to Change

### 1. Add `dedup_ranked_by_goal_key()` function

In a suitable location (e.g., `crates/worldwake-ai/src/ranking.rs` or a new submodule of `agent_tick`):

```rust
fn dedup_ranked_by_goal_key(
    ranked: &[RankedGoal],
    exhaustion_cache: &BTreeMap<OpportunityKey, ExhaustionEntry>,
) -> Vec<&RankedGoal>
```

Logic:
1. Iterate ranked candidates in order (already sorted by priority_class, motive_score).
2. Group by `GoalKey` (via `grounded.key`).
3. Within each group, select the first entry whose `OpportunityKey` is NOT exhausted (`suppresses_planning` returns false).
4. If all entries for a `GoalKey` are exhausted, emit nothing for that `GoalKey`.

### 2. Integrate dedup into planning pipeline

In `crates/worldwake-ai/src/agent_tick/planning.rs` or `candidates.rs`, call `dedup_ranked_by_goal_key()` after ranking and before `build_candidate_plans()`. Pass the deduped list to `build_candidate_plans()`.

### 3. Remove exhaustion filter from `build_candidate_plans()`

The exhaustion check currently inside `build_candidate_plans()` (lines 170-174) is superseded by the dedup step. Remove it to avoid double-filtering.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` or `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — add `dedup_ranked_by_goal_key()`)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — integrate dedup, remove inline exhaustion filter from `build_candidate_plans()`)
- `crates/worldwake-ai/src/agent_tick/candidates.rs` (modify — if pipeline integration happens here)

## Out of Scope

- Ranking algorithm changes (ranking remains at GoalKey level)
- Candidate generation changes (S33OPPSCOGOAIDE-002)
- Two-pass blocker filtering (S33OPPSCOGOAIDE-003)
- `PlannedPlan` changes (S33OPPSCOGOAIDE-006)
- Decision trace changes (S33OPPSCOGOAIDE-007)
- Golden tests (S33OPPSCOGOAIDE-009)

## Acceptance Criteria

### Tests That Must Pass

1. Post-rank dedup selects top non-exhausted opportunity per `GoalKey`.
2. Post-rank dedup falls through to next opportunity when top is exhausted.
3. When all opportunities for a `GoalKey` are exhausted, no candidate proceeds to plan search.
4. Single-opportunity GoalKey (most current scenarios) works identically to before.
5. Dedup output preserves rank ordering across different GoalKeys.
6. Existing suite: `cargo test -p worldwake-ai`
7. Existing suite: `cargo clippy --workspace`

### Invariants

1. Only one opportunity per `GoalKey` proceeds to plan search (budget conservation).
2. The selected opportunity is the highest-ranked non-exhausted one.
3. Determinism: when multiple opportunities have identical rank, selection is stable (deterministic via `Ord` on `OpportunityKey`).
4. No exhaustion check remains inside `build_candidate_plans()` — dedup is the sole exhaustion gate.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` or `ranking.rs` — `test_dedup_selects_top_non_exhausted` — first opportunity exhausted, second selected.
2. `test_dedup_all_exhausted_drops_goal` — all opportunities exhausted, GoalKey absent.
3. `test_dedup_single_opportunity_passthrough` — single-source scenarios unchanged.
4. `test_dedup_preserves_cross_goalkey_ordering` — rank order across different GoalKeys preserved.
5. Existing ranking and planning tests updated.

### Commands

1. `cargo test -p worldwake-ai -- dedup`
2. `cargo test -p worldwake-ai -- planning`
3. `cargo clippy --workspace && cargo test --workspace`
