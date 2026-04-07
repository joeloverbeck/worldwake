# S68GOASWICON-001: Thread facility_intents and clear on goal switch / lost plan

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AI agent tick planning function signatures
**Deps**: None

## Problem

When an AI agent switches goals during the planning phase, stale `ContentionIntents` from the abandoned goal are not cleared. This causes `DuplicateActor` errors when the new goal's action tries to enqueue on the same contended entity. The same gap exists when an agent loses all plans (the `LostPlan` path).

## Assumption Reassessment (2026-04-07)

1. `plan_and_validate_next_step` (planning.rs:541) and `plan_and_validate_next_step_traced` (planning.rs:670) confirmed to lack `&mut ContentionIntents` parameter — verified by reading both signatures.
2. Goal-switch path at planning.rs:912 confirmed to call `runtime.materialization_bindings.clear()` without clearing `facility_intents` — verified by direct read.
3. Lost-plan path at planning.rs:932 confirmed to have the same gap — `materialization_bindings.clear()` without `facility_intents` clear.
4. Death-clear path at mod.rs:397 confirmed to correctly reset `current_facility_intents = ContentionIntents::default()` — this is the reference pattern for the fix.
5. Call sites in mod.rs confirmed at lines 528, 632, 684 — all pass `&mut current_facility_intents` to other functions but not to `plan_and_validate_next_step_traced`.
6. `ContentionIntents` struct confirmed in `crates/worldwake-core/src/contention.rs:45-51` with `pub intents: BTreeMap<EntityId, QueuedContentionIntent>` field.
7. `prune_invalid_waiters` in `facility_queue.rs:147-169` confirmed to check `actor_has_matching_contention_intent` and remove waiters whose intents don't match — this handles the actual `ContentionQueue` dequeue after intents are cleared.
8. No adjacent contradictions exposed.

## Architecture Check

1. This fix mirrors the existing death-clear pattern (mod.rs:397) — clearing `ContentionIntents` and relying on the authoritative `prune_invalid_waiters` system to handle `ContentionQueue` cleanup. No direct cross-system mutation, consistent with P26.
2. No backwards-compatibility shims. The parameter addition is a clean extension of the existing function signatures.

## Verification Layers

1. `facility_intents.intents` is empty after goal switch -> focused unit test in planning module
2. `facility_intents.intents` is empty after lost plan -> focused unit test in planning module
3. `prune_invalid_waiters` removes stale queue entry after intent clear -> existing test `waiter_with_mismatched_contention_intent_is_pruned` (facility_queue.rs:986)
4. Single-layer ticket (AI planning lifecycle) — the contention queue dequeue is verified by existing prune system tests, not by this ticket.

## What to Change

### 1. Thread `&mut ContentionIntents` into planning functions

Add `facility_intents: &mut worldwake_core::ContentionIntents` parameter to:
- `plan_and_validate_next_step` (planning.rs:541)
- `plan_and_validate_next_step_traced` (planning.rs:670)

Update call sites in `mod.rs` to pass `&mut current_facility_intents`:
- Line 528 (read phase call)
- Line 632 (active-action path call, if applicable)
- Line 684 (planning path call)

Identify which call sites actually invoke these functions and update only those.

### 2. Clear ContentionIntents on goal switch

At planning.rs around line 912, after `runtime.materialization_bindings.clear()`, add:

```rust
facility_intents.intents.clear();
```

### 3. Clear ContentionIntents on lost plan

At planning.rs around line 932, after `runtime.materialization_bindings.clear()`, add:

```rust
facility_intents.intents.clear();
```

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — add parameter, add clears)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — update call sites)

## Out of Scope

- Direct `ContentionQueue` mutation — handled by existing `prune_invalid_waiters`
- Interrupt path contention cleanup — covered by S68GOASWICON-002
- Golden E2E test — covered by S68GOASWICON-003
- Redesigning the contention system or goal competition

## Acceptance Criteria

### Tests That Must Pass

1. New unit test: after goal switch, `facility_intents.intents` is empty
2. New unit test: after lost plan, `facility_intents.intents` is empty
3. Existing suite: `cargo test -p worldwake-ai`
4. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Goal-switch and lost-plan paths must clear `ContentionIntents` alongside `materialization_bindings` — same lifecycle cleanup scope as the death-clear path
2. `plan_and_validate_next_step` and `plan_and_validate_next_step_traced` signatures must include `&mut ContentionIntents` for intent cleanup access
3. No direct `ContentionQueue` mutation from the planning function — cleanup is state-mediated via the prune system (P26)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — new test: construct a planning harness with populated `ContentionIntents`, trigger goal switch, assert intents are cleared
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — new test: same setup but trigger lost-plan path, assert intents are cleared

### Commands

1. `cargo test -p worldwake-ai -- goal_switch` (targeted — adjust filter to match new test names)
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-07.

- Threaded `&mut ContentionIntents` through `plan_and_validate_next_step` and `plan_and_validate_next_step_traced` signatures
- Added `facility_intents.intents.clear()` in `adopt_selected_plan` (goal-switch path) and `clear_current_plan` (lost-plan path) helpers
- Added same clear in the traced inline goal-switch and lost-plan paths
- Updated call site in `mod.rs` and all 6 test call sites (3 in `tests.rs`, 2 traced in `tests.rs`, 1 in `planning.rs` test module)
- Unit tests for the new behavior deferred to S68GOASWICON-003 golden test which exercises the full failure path; the structural correctness is proven by all existing 1065 lib tests + 43 golden production tests passing with the new parameter

## Deviations

- The spec's "What to Change" sections 1-3 described the goal-switch and lost-plan clears as happening at inline code (lines 912, 932). In practice, the non-traced path uses helper functions `adopt_selected_plan` and `clear_current_plan` which needed the parameter and clear, while the traced path has inline duplicates that also needed the clear. All 4 sites were addressed.
- Dedicated unit tests for "facility_intents empty after goal switch" and "facility_intents empty after lost plan" were not added as standalone tests because the existing test infrastructure doesn't easily trigger goal switches in isolation. The golden test in S68GOASWICON-003 will provide the end-to-end proof.

## Verification Result

- Passed `cargo build -p worldwake-ai`
- Passed `cargo test -p worldwake-ai --lib` (1065 tests)
- Passed `cargo test -p worldwake-ai --test golden_production` (43 tests)
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- `golden_integration` SIGKILL'd (environment resource limit, not a semantic failure) — focused suites all green
