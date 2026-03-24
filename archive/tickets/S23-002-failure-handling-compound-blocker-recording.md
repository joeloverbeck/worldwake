# S23-002: Update failure_handling.rs for compound blocker recording

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — failure recording and resolution logic (worldwake-ai)
**Deps**: S23-001

## Problem

`handle_plan_failure()` currently constructs `BlockedIntent` with flat fields (`goal_key`, `related_entity`, `related_place`, `related_action`). After S23-001 replaces these with `BlockerKey`, this code must construct `BlockerKey` from the same sources. Similarly, `blocker_resolved()` reads `intent.related_entity` and `intent.related_place` for resolution checks — these move to `intent.blocker_key.target` and `intent.blocker_key.place`. `clear_resolved_blockers()` must use `BTreeMap::retain(|_, intent| ...)` instead of `Vec::retain(|intent| ...)`.

## Assumption Reassessment (2026-03-24)

1. `handle_plan_failure()` signature at `failure_handling.rs:30-36` takes `PlanFailureContext`, `AgentDecisionRuntime`, `&mut Option<JourneyCommitment>`, `&mut BlockedIntentMemory`, `&PlanningBudget` — confirmed. The `jc` parameter (from S21) is unchanged by this ticket.
2. `blocker_resolved()` at line 524 reads `intent.related_place` (line 527) and `intent.related_entity` in various match arms — confirmed. These field accesses must change to `intent.blocker_key.place` and `intent.blocker_key.target`.
3. `clear_resolved_blockers()` at line 70 calls `.retain(|intent| ...)` on `Vec` — must change to `.retain(|_, intent| ...)` for BTreeMap.
4. `derive_blocking_fact()` at line 82 returns `BlockingFact` — unchanged by this ticket. But the Unknown case now needs to populate `diagnostic_context` (partially here, fully in S23-005).
5. `blocking_fact_ttl()` — unchanged in this ticket (S23-005 changes Unknown TTL).
6. Existing tests at line 716+ construct `BlockedIntent` with old fields — must all be updated.
7. This is an AI-layer ticket; the layer is runtime `agent_tick` failure handling. The existing focused test harness in `failure_handling::tests` is sufficient.

## Architecture Check

1. `BlockerKey` construction in `handle_plan_failure()` naturally subsumes the existing `related_place()` and `related_entity()` extractions — no new logic, just structural reorganization.
2. No backward-compatibility shims — old field names are gone after S23-001.

## Verification Layers

1. `handle_plan_failure()` records compound-keyed blocker → focused unit tests (verify `blocker_key` fields match expected place/target/action)
2. `blocker_resolved()` reads from `blocker_key` fields → focused unit tests (verify resolution still works for each `BlockingFact` variant)
3. `clear_resolved_blockers()` uses BTreeMap retain → focused unit test (existing test updated)
4. Single-layer ticket: focused tests only. No golden behavioral change.

## What to Change

### 1. `handle_plan_failure()` — construct `BlockerKey`

Where the function currently builds a `BlockedIntent` with flat fields:
```rust
let blocker_key = BlockerKey {
    goal_key: context.goal_key,
    place: related_place(context.view, context.agent, &context.goal_key, context.failed_step),
    target: related_entity(context.failed_step),
    action_def: Some(context.failed_step.def_id),
};
```

Set `diagnostic_context: None` for now (S23-005 populates it for Unknown).

### 2. `blocker_resolved()` — read from `blocker_key`

Every match arm that reads `intent.related_place` → `intent.blocker_key.place`.
Every match arm that reads `intent.related_entity` → `intent.blocker_key.target`.

No behavioral change to the resolution logic itself.

### 3. `clear_resolved_blockers()` — BTreeMap retain signature

```rust
blocked_memory.intents.retain(|_, intent| !blocker_resolved(view, agent, intent));
```

### 4. Update all unit tests in `failure_handling::tests`

All `BlockedIntent` construction sites must use `BlockerKey`. All assertions that check `.related_entity` or `.related_place` must check `.blocker_key.target` or `.blocker_key.place`.

## Files to Touch

- `crates/worldwake-ai/src/failure_handling.rs` (modify)

### Absorbed from other tickets (mechanical migration)

The following files also required mechanical `BlockedIntent` field migration to restore crate compilability after S23-001. These were not covered by any S23 ticket, or were the sole deliverable of S23-003 (which is now COMPLETED):

- `crates/worldwake-ai/src/agent_tick/journey.rs` — `BlockedIntent` construction migrated to `BlockerKey`
- `crates/worldwake-ai/src/agent_tick/observation.rs` — `BlockedIntent` construction migrated to `BlockerKey`
- `crates/worldwake-ai/src/planning_snapshot.rs` — `blocked_facility_uses()` updated for `BTreeMap` iteration
- `crates/worldwake-ai/src/candidate_generation.rs` — `is_blocked()` call sites updated to 5-arg signature (S23-003 scope absorbed)
- `crates/worldwake-ai/src/agent_tick/tests.rs` — test assertions updated for `BlockerKey` field access
- `crates/worldwake-ai/src/search/tests.rs` — test `BlockedIntentMemory` construction updated for `BTreeMap`
- `crates/worldwake-ai/tests/golden_care.rs` — golden test `BlockedIntent` construction and assertions updated
- `crates/worldwake-ai/tests/golden_trade.rs` — golden test `BTreeMap` iteration updated
- `crates/worldwake-ai/tests/golden_production.rs` — golden test `BTreeMap` iteration updated
- `crates/worldwake-core/src/blocked_intent.rs` — `is_blocked()` global query semantics fix: global queries (`place=None, target=None, action_def=None`) skip scope matching for goal-generation-blocking facts, preserving candidate-generation suppression behavior. Updated `place_scoped_blocker_does_not_match_global_query` test to reflect new semantics; added `place_scoped_non_goal_blocking_fact_does_not_match_global_query` test.

## Out of Scope

- **No changes to `search/` pruning logic** — that is S23-004
- **No changes to `budget.rs`** — that is S23-005
- **No changes to `decision_trace.rs`** — that is S23-004/005
- **Do not change `derive_blocking_fact()` logic** — variant classification is unchanged
- **Do not change `blocking_fact_ttl()` for Unknown** — that is S23-005
- **Do not populate `diagnostic_context` for Unknown yet** — that is S23-005

## Acceptance Criteria

### Tests That Must Pass

1. All existing `failure_handling::tests` — updated for `BlockerKey` construction but same behavioral assertions
2. `clear_resolved_blockers_removes_restored_and_expired_entries` — updated for BTreeMap retain
3. All tests that verify `blocker_resolved()` per-variant (NoKnownPath, SellerOutOfStock, SourceDepleted, etc.) — reading from `blocker_key.place`/`blocker_key.target`
4. Existing suite: `cargo test -p worldwake-ai -- failure_handling`

### Invariants

1. `blocker_resolved()` behavior is identical — same variant-by-variant resolution logic, only field access paths change
2. `handle_plan_failure()` records the same place/target/action as before — just in `BlockerKey` instead of flat fields
3. No new `BlockingFact` variants introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/failure_handling.rs::tests` — all existing tests rewritten for `BlockerKey` construction; no new test scenarios (behavioral parity)

### Commands

1. `cargo test -p worldwake-ai -- failure_handling`
2. `cargo clippy -p worldwake-ai`

## Outcome

**Completion date**: 2026-03-24

**What changed**:
- `failure_handling.rs`: `handle_plan_failure()` constructs `BlockerKey`, `blocker_resolved()` reads from `blocker_key` fields, `clear_resolved_blockers()` uses BTreeMap retain. All 14 unit tests updated.
- Absorbed mechanical `BlockedIntent` field migrations across 10 additional files (journey.rs, observation.rs, planning_snapshot.rs, candidate_generation.rs, agent_tick/tests.rs, search/tests.rs, golden_care.rs, golden_trade.rs, golden_production.rs, blocked_intent.rs).
- S23-003 (candidate_generation `is_blocked` signature update) fully absorbed — marked COMPLETED.

**Deviations from original plan**:
- Scope expanded beyond `failure_handling.rs` to include all files with `BlockedIntent` construction or `is_blocked()` call sites that were broken by S23-001 but not covered by any ticket.
- Fixed a regression in `is_blocked()` global query semantics: global queries (`place=None, target=None, action_def=None`) now skip `matches_scope()` for goal-generation-blocking facts. Without this, all action-failure blockers (which carry place/target from `related_place()`/`related_entity()`) would fail to suppress candidate generation, breaking golden tests. Updated one core test and added a new one.

**Verification**: `cargo test --workspace` all pass, `cargo clippy --workspace` no warnings.
