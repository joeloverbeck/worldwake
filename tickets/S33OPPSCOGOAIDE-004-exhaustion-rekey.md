# S33OPPSCOGOAIDE-004: Re-key exhaustion cache from GoalKey to OpportunityKey

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `AgentDecisionRuntime.exhaustion_cache`, planning result keying, invalidation iteration
**Deps**: S33OPPSCOGOAIDE-002

## Problem

`AgentDecisionRuntime.exhaustion_cache` is still keyed by `GoalKey`, so exhausting one concrete opportunity can still suppress planning for sibling opportunities that share the same desire. After S33OPPSCOGOAIDE-002, candidate generation already emits per-opportunity `GroundedGoal` values, so the remaining contradiction is specifically in the exhaustion layer.

## Assumption Reassessment (2026-03-28)

1. `GroundedGoal` now carries `anchor` in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), and candidate generation already emits per-opportunity candidates in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs).
2. `AgentDecisionRuntime.exhaustion_cache` is still `BTreeMap<GoalKey, ExhaustionEntry>` in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs).
3. `build_candidate_plans()` and `record_exhausted_goals()` in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) still operate on desire-level exhaustion keys.
4. The exact shared abstraction boundary under audit is: `GroundedGoal { key, anchor }` -> planning attempt key -> `AgentDecisionRuntime.exhaustion_cache`.
5. Live code currently contains a temporary first-per-`GoalKey` planning dedup to stabilize search budget after `002`. This ticket must not replace that dedup policy; it only fixes exhaustion identity.
6. The live `GoalKind` surface most sensitive to this contradiction is multi-opportunity `AcquireCommodity`, `ProduceCommodity`, and other place-anchored acquisition/production goals where one failed source should not poison its siblings.
7. Planning-snapshot isolation no longer remains merged today: archived `S33OPPSCOGOAIDE-010` moved `build_candidate_plans()` in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) to candidate-local snapshot construction. This ticket therefore stays focused strictly on exhaustion identity.
8. Mismatch + correction: the old ticket assumed this was the next immediate step after `002`. In live code, `005` still remains after this lands, but candidate-local snapshot scope is already complete and no longer a dependency or follow-up concern here.

## Architecture Check

1. Re-keying the cache is the clean architectural move. Keeping a parallel `GoalKey` cache or mixed lookup path would violate P26 by preserving the old alias boundary.
2. `ExhaustionEntry` semantics from S31 should remain unchanged. The point of this ticket is identity granularity, not a second redesign of invalidation or retry policy.

## Verification Layers

1. Exhausting one opportunity leaves sibling opportunities plannable -> focused planning/runtime test
2. Invalidation clears only the matching exhausted opportunity entry -> focused planning/runtime test
3. Budget-retry state remains attached to the exhausted opportunity only -> focused planning/runtime test
4. Single-layer runtime-state ticket; golden proof is deferred to S33OPPSCOGOAIDE-009

## What to Change

### 1. Re-key the runtime exhaustion cache

Change `AgentDecisionRuntime.exhaustion_cache` from `BTreeMap<GoalKey, ExhaustionEntry>` to `BTreeMap<OpportunityKey, ExhaustionEntry>`.

### 2. Re-key planning attempt results

In [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), change planning-result tuples and exhaustion insertion paths from `GoalKey` to `OpportunityKey`, constructed from `grounded.key` plus `grounded.anchor`.

### 3. Update invalidation iteration

`invalidate_exhausted_goals()` must iterate per `OpportunityKey` while preserving S31 invalidation-condition semantics. This ticket changes scope, not the conditions themselves.

### 4. Keep temporary dedup behavior untouched

Do not absorb post-rank selection policy here. If the current temporary first-per-`GoalKey` dedup remains until S33OPPSCOGOAIDE-005, this ticket should work with it rather than silently redesign it.

## Files to Touch

- [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) (modify)
- [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) (modify)
- any directly coupled tests around exhaustion invalidation and planning retries

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
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy --workspace`

### Invariants

1. No `GoalKey`-scoped exhaustion cache remains in live runtime state.
2. S31 invalidation semantics are preserved; only the key granularity changes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — exhaust one place-anchored opportunity and prove the sibling remains plannable.
2. `crates/worldwake-ai/src/agent_tick/planning.rs` — invalidate one `OpportunityKey` entry without clearing its siblings.
3. `crates/worldwake-ai/src/agent_tick/planning.rs` — prove budget-retry counters remain scoped per opportunity.

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai -- planning`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`
