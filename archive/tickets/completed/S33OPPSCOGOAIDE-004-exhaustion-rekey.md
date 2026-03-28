# S33OPPSCOGOAIDE-004: Re-key exhaustion cache from GoalKey to OpportunityKey

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `AgentDecisionRuntime.exhaustion_cache`, planning result keying, invalidation iteration, save/load pruning, runtime fixtures/helpers
**Deps**: S33OPPSCOGOAIDE-002

## Problem

`AgentDecisionRuntime.exhaustion_cache` is still keyed by `GoalKey`, so exhausting one concrete opportunity can still suppress planning for sibling opportunities that share the same desire. After S33OPPSCOGOAIDE-002, candidate generation already emits per-opportunity `GroundedGoal` values, so the remaining contradiction is specifically in the exhaustion layer.

## Assumption Reassessment (2026-03-28)

1. `GroundedGoal` now carries `anchor` in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), and candidate generation already emits per-opportunity candidates in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs).
2. `AgentDecisionRuntime.exhaustion_cache` is still `BTreeMap<GoalKey, ExhaustionEntry>` in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs).
3. `build_candidate_plans()` and `record_exhausted_goals()` in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) still operate on desire-level exhaustion keys and still deduplicate planning by first-seen `GoalKey`.
4. `invalidate_exhausted_goals()` in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs) and `post_load_validate()` in [`crates/worldwake-ai/src/agent_tick/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs) still assume `GoalKey`-keyed exhaustion entries, so a correct re-key cannot stop in `planning.rs`.
5. The exact shared abstraction boundary under audit is: `GroundedGoal { key, anchor }` -> `OpportunityKey { goal_key, anchor }` -> planning attempt key -> `AgentDecisionRuntime.exhaustion_cache`.
6. Live code currently contains a temporary first-per-`GoalKey` planning dedup to stabilize search budget after `002`. This ticket must not replace that dedup policy; it only fixes exhaustion identity under the current temporary selection rule.
7. The live `GoalKind` surface most sensitive to this contradiction is multi-opportunity `AcquireCommodity`, `ProduceCommodity`, and other place-anchored acquisition/production goals where one failed source should not poison its siblings.
8. Planning-snapshot isolation no longer remains merged today: archived `S33OPPSCOGOAIDE-010` moved `build_candidate_plans()` in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) to candidate-local snapshot construction. This ticket therefore stays focused strictly on exhaustion identity.
9. Mismatch + correction: the old ticket understated the verification and file scope. Live save/load tests in [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs), golden save/load coverage in [`crates/worldwake-ai/tests/golden_harness/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs), and the unrelated-commodity exhaustion golden in [`crates/worldwake-ai/tests/golden_ai_decisions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs) all still encode `GoalKey` exhaustion entries and must be updated in-scope.
10. Mismatch + correction: the old ticket assumed this was the next immediate step after `002`. In live code, `005` still remains after this lands, but candidate-local snapshot scope is already complete and no longer a dependency or follow-up concern here.

## Architecture Check

1. Re-keying the cache is the clean architectural move. Keeping a parallel `GoalKey` cache, mixed lookup helpers, or dual serialization paths would violate P26 by preserving the old alias boundary.
2. `ExhaustionEntry` semantics from S31 should remain unchanged. The point of this ticket is identity granularity, not a second redesign of invalidation or retry policy.
3. The current architecture still leaves one awkward seam: `build_candidate_plans()` deduplicates by `GoalKey` before looking at per-opportunity exhaustion. That means only the first ranked sibling opportunity gets considered during a planning pass. This ticket should preserve that behavior because S33OPPSCOGOAIDE-005 owns the selector redesign, but the ticket should document that this remains a known architectural limitation rather than implying full per-opportunity fallthrough after re-keying.

## Verification Layers

1. Exhausting one opportunity leaves sibling opportunities retained in runtime state and not suppressed by cache lookup -> focused planning/runtime test
2. Invalidation clears only the matching exhausted opportunity entry -> focused planning/runtime test
3. Save/load and post-load pruning preserve/prune `OpportunityKey` entries correctly -> focused runtime persistence test plus golden harness save/load test
4. Budget-retry state remains attached to the exhausted opportunity only -> focused planning/runtime test
5. Golden proof that unrelated invalidation still preserves exhaustion now uses `OpportunityKey`-scoped entries -> targeted golden AI test

## What to Change

### 1. Re-key the runtime exhaustion cache

Change `AgentDecisionRuntime.exhaustion_cache` from `BTreeMap<GoalKey, ExhaustionEntry>` to `BTreeMap<OpportunityKey, ExhaustionEntry>`.

### 2. Re-key planning attempt results

In [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), change planning-result tuples and exhaustion insertion paths from `GoalKey` to `OpportunityKey`, constructed from `grounded.key` plus `grounded.anchor`.

### 3. Update invalidation iteration

`invalidate_exhausted_goals()` in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs) must iterate per `OpportunityKey` while preserving S31 invalidation-condition semantics. This ticket changes scope, not the conditions themselves.

### 4. Update persistence and post-load pruning

`post_load_validate()` and all runtime serialization fixtures/helpers must prune dead-entity `OpportunityKey` anchors correctly and preserve live entries without introducing an alias migration path.

### 5. Keep temporary dedup behavior untouched

Do not absorb post-rank selection policy here. If the current temporary first-per-`GoalKey` dedup remains until S33OPPSCOGOAIDE-005, this ticket should work with it rather than silently redesign it.

## Files to Touch

- [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) (modify)
- [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) (modify)
- [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs) (modify)
- [`crates/worldwake-ai/src/agent_tick/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs) (modify)
- [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) (modify)
- [`crates/worldwake-ai/tests/golden_harness/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) (modify)
- [`crates/worldwake-ai/tests/golden_ai_decisions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs) (modify)
- directly coupled planning/exhaustion unit tests in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) (modify)

## Out of Scope

- replacing temporary planning dedup with real post-rank opportunity selection
- adding `PlannedPlan.opportunity`
- per-opportunity planning-snapshot isolation (already delivered by archived `S33OPPSCOGOAIDE-010`)
- save/load version bump
- decision-trace schema changes

## Acceptance Criteria

### Tests That Must Pass

1. Exhausting `OpportunityKey { goal_key: AcquireCommodity(...), anchor: Place(orchard) }` leaves the sibling market opportunity plannable.
2. Invalidation clears the specific exhausted `OpportunityKey`, not all same-goal siblings.
3. Budget retry state remains per-opportunity.
4. Save/load round-trip preserves live `OpportunityKey` exhaustion entries and prunes dead-anchor entries.
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo clippy --workspace`

### Invariants

1. No `GoalKey`-scoped exhaustion cache remains in live runtime state.
2. S31 invalidation semantics are preserved; only the key granularity changes.
3. The temporary first-per-`GoalKey` planning dedup remains in place until S33OPPSCOGOAIDE-005; this ticket does not claim cross-sibling fallthrough within the same ranked planning pass.

## Test Plan

### New/Modified Tests

1. `agent_tick::planning::tests::record_exhausted_goals_removes_only_successful_opportunity_entry` in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) — prove success clears only the solved exhausted opportunity and preserves a sibling opportunity entry with the same `GoalKey`.
2. `agent_tick::planning::tests::record_exhausted_goals_replaces_frontier_suppression_with_budget_retry_state` and `agent_tick::planning::tests::record_exhausted_goals_records_frontier_exhaustion_as_suppressing_retry_state` in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) — prove budget/frontier retry state attaches to `OpportunityKey`, not the bare desire.
3. `exhaustion::tests::invalidate_exhausted_goals_clears_only_matching_opportunity_for_same_goal` in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs) — prove invalidation clears only the matching sibling opportunity while preserving another cache entry with the same `GoalKey`.
4. `agent_tick::tests::save_runtime_state_serializes_persisted_driver_state`, `agent_tick::tests::from_saved_runtime_restores_and_validates_driver_state`, and `agent_tick::tests::post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty` in [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) — prove runtime persistence and dead-anchor pruning now use `OpportunityKey`.
5. `golden_harness::tests::save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation` in [`crates/worldwake-ai/tests/golden_harness/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) — keep save/load golden coverage aligned with live runtime shape.
6. `golden_unrelated_commodity_change_preserves_frontier_exhaustion` in [`crates/worldwake-ai/tests/golden_ai_decisions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs) — prove unrelated invalidation still preserves exhaustion under `OpportunityKey` keying.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai agent_tick::planning::tests::`
3. `cargo test -p worldwake-ai invalidate_exhausted_goals_clears_only_matching_opportunity_for_same_goal`
4. `cargo test -p worldwake-ai save_runtime_state_serializes_persisted_driver_state`
5. `cargo test -p worldwake-ai from_saved_runtime_restores_and_validates_driver_state`
6. `cargo test -p worldwake-ai post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
7. `cargo test -p worldwake-ai golden_unrelated_commodity_change_preserves_frontier_exhaustion`
8. `cargo test -p worldwake-ai save_load_roundtrip_prunes_stale_runtime_state_via_post_load_validation`
9. `cargo test -p worldwake-ai`
10. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-28
- Actual changes:
  - re-keyed `AgentDecisionRuntime.exhaustion_cache` from `GoalKey` to `OpportunityKey`
  - re-keyed planning attempt tuples and exhaustion recording/removal paths to `OpportunityKey`
  - updated exhaustion invalidation and post-load pruning to reason about both `goal_key` and anchor liveness
  - updated save/load fixtures and golden helpers to persist the new runtime shape without any compatibility alias path
  - strengthened focused coverage with a sibling-opportunity invalidation test
- Deviations from original plan:
  - no `PlannedPlan.opportunity` field was added; that remains correctly deferred
  - the temporary first-per-`GoalKey` planning dedup was intentionally preserved; the architecture still needs S33OPPSCOGOAIDE-005 to provide real per-opportunity fallthrough inside a planning pass
  - `worldwake-ai` re-exported `OpportunityAnchor` and `OpportunityKey` to keep test/runtime call sites clean and avoid ad hoc import paths
- Verification results:
  - focused planning, invalidation, persistence, and golden regression tests all passed
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy --workspace` passed
