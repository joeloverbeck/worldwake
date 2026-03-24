# S23-002: Update failure_handling.rs for compound blocker recording

**Status**: PENDING
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

## Out of Scope

- **No changes to `blocked_intent.rs`** — that is S23-001
- **No changes to `candidate_generation.rs`** — that is S23-003
- **No changes to `search/`** — that is S23-004
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
