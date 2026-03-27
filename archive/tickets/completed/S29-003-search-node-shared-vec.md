# S29-003: Migrate SearchNode Steps to SharedVec

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S29-001

## Problem

`SearchNode.steps: Vec<PlannedStep>` is deep-cloned at `search/transition.rs:124` for every successor node. The Vec grows linearly with search depth (up to max_plan_depth=8, or 12 in expanded-budget tests). Wrapping it in `SharedVec<PlannedStep>` avoids deep-cloning the accumulated step history — only the final `push` triggers a CoW clone.

## Assumption Reassessment (2026-03-27)

1. Shared abstraction boundary under audit: the transient step-history path from `SearchNode<'snapshot>` in `crates/worldwake-ai/src/search/mod.rs`, through successor construction in `crates/worldwake-ai/src/search/transition.rs`, to final owned `PlannedPlan.steps: Vec<PlannedStep>` in `crates/worldwake-ai/src/planner_ops.rs`. This is a purely internal search-memory optimization boundary, not a gameplay or cross-crate contract.
2. `SearchNode` in `crates/worldwake-ai/src/search/mod.rs` currently stores `steps: Vec<PlannedStep>`, and `crates/worldwake-ai/src/search/transition.rs` still deep-clones that `Vec` at the single live clone site: `let mut steps = node.steps.clone(); steps.push(step);`.
3. `PlannedPlan::new()` in `crates/worldwake-ai/src/planner_ops.rs` still takes `steps: Vec<PlannedStep>`. The four live plan-construction sites in `crates/worldwake-ai/src/search/mod.rs` currently pass `node.steps` or `successor.steps` directly, so they are the exact search-boundary conversions that must change to `into_vec()`.
4. `best_barrier` in `crates/worldwake-ai/src/search/mod.rs` stores `Option<PlannedPlan>`, so the ownership conversion belongs only at `PlannedPlan::new(...)`; no shared wrapper should leak into barrier storage or any downstream API.
5. `root_node()` in `crates/worldwake-ai/src/search/heuristic.rs`, not `search_plan()` itself, is the live root initialization site for `SearchNode.steps`.
6. `crates/worldwake-ai/src/search/frontier.rs::compare_search_nodes` currently uses both `left.steps.len()` and lexicographic `left.steps.cmp(&right.steps)` as deterministic tie-breakers. After the field-type swap, this comparison should stay on slice contents (`as_slice()`) rather than broadening `SharedVec` into a more general ordering abstraction than search actually needs.
7. `SearchNode` remains private to the `search` module, but `crates/worldwake-ai/src/search/tests.rs` constructs `SearchNode` literals directly in multiple focused tests. This ticket therefore needs limited test-file updates even if no planner behavior changes.
8. S29-001 is completed. `SharedVec` already exists as a crate-private type in `crates/worldwake-ai/src/shared_collections.rs`, and this ticket should consume that exact type rather than widening the abstraction surface or adding a parallel alias path.
9. The dead-code assumption in the earlier draft was inaccurate. The live code does not use a module-level `#![allow(dead_code)]`; instead `crates/worldwake-ai/src/shared_collections.rs` has item-level `#[allow(dead_code)]` on `SharedVec` and some wrapper methods. Once `SearchNode` consumes `SharedVec`, this ticket should remove or narrow the `SharedVec`-specific allowances while leaving any still-needed `SharedMap` / `SharedSet` allowances to the actual live consumers.
10. This ticket is still architecturally independent of S29-002. Both depend on S29-001, but S29-002 changes `PlanningState` collection storage while this ticket changes only the search step-history transport.
11. Verification commands must use real current targets. `cargo test -p worldwake-ai -- --list` confirms the live unit/integration inventory, so this ticket should reference exact new test names plus real crate/workspace targets instead of approximate `search` / `golden` substring filters.

## Architecture Check

1. This remains a good architectural change. The current `Vec<PlannedStep>` clone-per-successor path is pure transient planner overhead, while `SharedVec` gives structural sharing exactly where the search tree branches without changing any world meaning, planner surface, or downstream storage contract.
2. The clean boundary is to keep `SharedVec` strictly internal to search and tests that exercise search internals. `PlannedPlan` should continue to own a plain `Vec<PlannedStep>` so the optimization does not leak into planner-runtime or action-execution boundaries.
3. This is cleaner than introducing extra conversion helpers, alias types, or trait-heavy wrapper ergonomics. The ideal architecture here is a narrow optimization seam: `SharedVec` inside the search tree, `Vec` at the plan boundary, and no duplicate transport paths.
4. No backwards-compatibility shims. Replace the internal field outright; do not keep parallel `Vec` and `SharedVec` representations alive together.

## Verification Layers

1. Successor step accumulation and parent/child independence at the search boundary -> focused `search::tests` coverage around `build_successor`
2. Search ordering and final plan materialization stay identical after the internal field-type swap -> focused `search::tests` coverage plus `cargo test -p worldwake-ai --lib`
3. Crate-level planner and golden behavior remain unchanged -> `cargo test -p worldwake-ai`
4. Workspace determinism/lint hygiene and staged-wrapper dead-code cleanup remain intact -> `cargo clippy --workspace` and `cargo test --workspace`

## What to Change

### 1. Update `SearchNode` struct

Change `steps: Vec<PlannedStep>` to `steps: SharedVec<PlannedStep>` in `search/mod.rs`.

### 2. Update initial node construction

In `root_node()` at `search/heuristic.rs`, the initial `SearchNode` construction currently uses `steps: Vec::new()` — change that to `steps: SharedVec::new()`.

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

Add `use crate::shared_collections::SharedVec;` where the type is named directly (`search/mod.rs`, `search/heuristic.rs`, and `search/tests.rs`). `search/transition.rs` does not need a new import unless its local code starts naming the type explicitly.

### 6. Narrow `SharedVec` dead-code allowances

Remove or narrow the `SharedVec`-specific `#[allow(dead_code)]` attributes in `crates/worldwake-ai/src/shared_collections.rs` once `SearchNode` becomes a real consumer. Do not broaden this into unrelated wrapper API cleanup.

## Files to Touch

- `crates/worldwake-ai/src/search/mod.rs` (modify — struct field, plan construction sites, import)
- `crates/worldwake-ai/src/search/frontier.rs` (modify — keep deterministic tie-break ordering on step contents without widening `SharedVec` ordering surface)
- `crates/worldwake-ai/src/search/transition.rs` (modify — clone+push path now operates on `SharedVec`)
- `crates/worldwake-ai/src/search/heuristic.rs` (modify — root node initialization, import)
- `crates/worldwake-ai/src/search/tests.rs` (modify — direct `SearchNode` test fixtures and any focused search-boundary assertions)
- `crates/worldwake-ai/src/shared_collections.rs` (modify — narrow/remove `SharedVec` dead-code allowances now that it has a live consumer)

## Out of Scope

- `PlanningState` field migration (that is S29-002).
- `planning_state.rs` changes.
- `PlannedPlan` struct or its `new()` signature — it still takes `Vec<PlannedStep>`.
- Benchmarking (that is S29-004).
- Any behavior change to planner goal selection, heuristic ranking, or terminal semantics.
- Broad wrapper API expansion beyond the methods already required by S29-001/S29-003.

## Acceptance Criteria

### Tests That Must Pass

1. Focused search-boundary tests covering the changed seam
2. `cargo test -p worldwake-ai --lib`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

### Invariants

1. `SearchNode::clone()` (including steps) is O(1) when steps have not been modified since last clone.
2. Plans produced by `search_plan` are identical before and after (same steps, same order, same terminal kinds).
3. Golden test hash values are unchanged (determinism preserved).
4. `PlannedPlan.steps` is always a `Vec<PlannedStep>` — the `SharedVec` is internal to search only.
5. `SharedVec` no longer carries dead-code suppression for code paths that now have live search consumers.

## Test Plan

### New/Modified Tests

1. `search::tests::build_successor_preserves_parent_steps_when_appending_child_step` — proves the copy-on-write boundary directly at the only live `SearchNode.steps.clone(); push(...)` seam, so this ticket does not rely only on indirect plan-level regressions.
2. Existing `search::tests` helpers and `SearchNode` fixture sites will be updated to construct `SharedVec<PlannedStep>` instead of raw `Vec<PlannedStep>`.

### Commands

1. `cargo test -p worldwake-ai --lib search::tests::build_successor_preserves_parent_steps_when_appending_child_step`
2. `cargo test -p worldwake-ai --lib`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome

Completed: 2026-03-27

1. Replaced `SearchNode.steps: Vec<PlannedStep>` with `SharedVec<PlannedStep>` across the search tree, keeping `PlannedPlan.steps` as an owned `Vec<PlannedStep>` at the plan boundary via `into_vec()`.
2. Updated `root_node()` and successor construction to use the shared step-history path, and kept deterministic frontier ordering by comparing `steps.as_slice()` in `search/frontier.rs` rather than broadening `SharedVec` with extra ordering behavior.
3. Narrowed the temporary `SharedVec` dead-code suppression in `shared_collections.rs` now that search is a live consumer.
4. Updated focused search tests to build `SearchNode` fixtures with `SharedVec`, and added a dedicated regression test that proves child-step appends do not mutate the parent node's accumulated step history.

Deviations from original plan:

1. `crates/worldwake-ai/src/search/frontier.rs` also needed a small change. The earlier ticket draft had missed that `compare_search_nodes` used lexicographic step ordering as a deterministic tie-breaker.
2. Existing `search/tests.rs` fixture code required modification after reassessment; this was not optional if the ticket was going to stay aligned with the real private-module test surface.

Verification results:

1. `cargo test -p worldwake-ai --lib search::tests::build_successor_preserves_parent_steps_when_appending_child_step` passed.
2. `cargo test -p worldwake-ai --lib` passed.
3. `cargo test -p worldwake-ai` passed.
4. `cargo clippy --workspace` passed.
5. `cargo test --workspace` passed.
