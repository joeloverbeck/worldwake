# S29-003: Migrate SearchNode Steps to SharedVec

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S29-001

## Problem

`SearchNode.steps: Vec<PlannedStep>` is deep-cloned at `search/transition.rs:124` for every successor node. The Vec grows linearly with search depth (up to max_plan_depth=8, or 12 in expanded-budget tests). Wrapping it in `SharedVec<PlannedStep>` avoids deep-cloning the accumulated step history — only the final `push` triggers a CoW clone.

## Assumption Reassessment (2026-03-27)

1. `SearchNode` at `search/mod.rs:27-36` has `steps: Vec<PlannedStep>`. Confirmed by reading the struct.
2. Clone site at `search/transition.rs:124`: `let mut steps = node.steps.clone(); steps.push(step);`. This is the only clone site for `steps`.
3. `PlannedPlan::new()` at `planner_ops.rs:791` takes `steps: Vec<PlannedStep>`. The final plan construction at `search/mod.rs:101-104`, `:234-237`, `:245`, `:277-280` all pass `node.steps` or `successor.steps` directly.
4. With `SharedVec`, the plan construction sites need `into_vec()` to convert back to `Vec<PlannedStep>` for `PlannedPlan::new()`.
5. The `best_barrier` variable at `search/mod.rs:97` stores `Option<PlannedPlan>` which already has `Vec<PlannedStep>` — the conversion happens at construction time, not storage time.
6. `SearchNode` is private to the search module. No external consumers.
7. This ticket is independent of S29-002 (both depend on S29-001 but not on each other). They can be done in parallel.

## Architecture Check

1. Minimal change: one field type change, one clone+push pattern change, and `into_vec()` calls at plan construction boundaries. The `SharedVec` API from S29-001 provides all needed operations.
2. No backwards-compatibility shims. `Vec<PlannedStep>` is replaced, not aliased.

## Verification Layers

1. Step accumulation correctness → existing search unit tests
2. Plan construction correctness → existing golden tests (plans are the end product)
3. Determinism preserved → golden hash comparisons
4. Single-module ticket: changes are contained within `search/`.

## What to Change

### 1. Update `SearchNode` struct

Change `steps: Vec<PlannedStep>` to `steps: SharedVec<PlannedStep>` in `search/mod.rs`.

### 2. Update initial node construction

In `search_plan()` at `search/mod.rs`, the initial `SearchNode` construction uses `steps: Vec::new()` — change to `steps: SharedVec::new()`.

### 3. Update transition clone+push

At `search/transition.rs:124`, change:
```rust
let mut steps = node.steps.clone();  // now O(1) via Rc
steps.push(step);                    // triggers CoW clone + push
```
No API change needed — `SharedVec::clone()` is O(1) and `SharedVec::push()` handles CoW.

### 4. Update plan construction sites

At each `PlannedPlan::new(goal.key, node.steps, ...)` or `PlannedPlan::new(goal.key, successor.steps, ...)` call, change to `node.steps.into_vec()` or `successor.steps.into_vec()`.

Sites (all in `search/mod.rs`):
- Line ~101-104: goal-satisfied terminal
- Line ~234-237: terminal found during expansion
- Line ~245: progress-barrier best_barrier
- Line ~277-280: forced-auxiliary terminal

### 5. Update imports

Add `use crate::shared_collections::SharedVec;` in `search/mod.rs` and `search/transition.rs`.

## Files to Touch

- `crates/worldwake-ai/src/search/mod.rs` (modify — struct field, initial construction, plan construction sites, import)
- `crates/worldwake-ai/src/search/transition.rs` (modify — import only; clone+push pattern is API-compatible)

## Out of Scope

- `PlanningState` field migration (that is S29-002).
- `planning_state.rs` changes.
- `PlannedPlan` struct or its `new()` signature — it still takes `Vec<PlannedStep>`.
- Benchmarking (that is S29-004).
- Any file outside the `search/` module.

## Acceptance Criteria

### Tests That Must Pass

1. All existing search unit tests: `cargo test -p worldwake-ai search`
2. All existing golden tests: `cargo test -p worldwake-ai golden`
3. Full workspace: `cargo test --workspace`
4. `cargo clippy --workspace` — no new warnings.

### Invariants

1. `SearchNode::clone()` (including steps) is O(1) when steps have not been modified since last clone.
2. Plans produced by `search_plan` are identical before and after (same steps, same order, same terminal kinds).
3. Golden test hash values are unchanged (determinism preserved).
4. `PlannedPlan.steps` is always a `Vec<PlannedStep>` — the `SharedVec` is internal to search only.

## Test Plan

### New/Modified Tests

1. None — this is a pure type-substitution refactor within the search module. All behavioral verification comes from existing search and golden test suites. The `SharedVec` unit tests from S29-001 cover the wrapper's correctness.

### Commands

1. `cargo test -p worldwake-ai search` (targeted)
2. `cargo test -p worldwake-ai golden` (golden regression)
3. `cargo clippy --workspace && cargo test --workspace` (full verification)
