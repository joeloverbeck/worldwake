# S88TWOPHALAN-004: Implement DualFrontier with preferred operator boosting

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — replaces internal planner data structure
**Deps**: None

## Problem

The current planner uses a single `BinaryHeap<FrontierEntry>` (search/mod.rs:104) which treats all candidates equally. The dual open list (S88 D6) separates preferred operators (landmark-achieving) from regular candidates, allowing the search to focus on subgoal-relevant actions while maintaining completeness via alternation.

## Assumption Reassessment (2026-04-11)

1. `FrontierEntry` exists at `crates/worldwake-ai/src/search/frontier.rs:4` as a wrapper around `SearchNode<'snapshot>` with `Ord` impl based on `compare_search_nodes`. The `BinaryHeap` is created at `search/mod.rs:104` and popped at line 114. No other frontier data structures exist.
2. `compare_search_nodes` at `frontier.rs:38` uses f-value (search_cost + heuristic), then tiebreaks on search_cost, total_estimated_ticks, and step count. This ordering logic is unchanged by this ticket.
3. The frontier is `pub(super)` scoped — only used within the `search` module. No external consumers.

## Architecture Check

1. Replacing the single heap with a `DualFrontier` struct that encapsulates two heaps and alternation logic is cleaner than adding conditional logic around a single heap. The `DualFrontier` exposes `push_regular`, `push_preferred`, and `pop` — the alternation logic is internal.
2. No backwards-compatibility shims. The old single-heap pattern is replaced entirely.

## Verification Layers

1. Alternation correctness → focused unit tests (preferred/regular alternation, boost behavior)
2. Fallback behavior → focused unit tests (empty preferred falls through to regular)
3. Ordering preservation → focused unit tests (within each queue, ordering matches `compare_search_nodes`)
4. Single-layer ticket (planner-internal data structure) — no cross-layer mapping needed.

## What to Change

### 1. Add `DualFrontier` to `crates/worldwake-ai/src/search/frontier.rs`

```rust
pub(super) struct DualFrontier<'snapshot> {
    regular: BinaryHeap<FrontierEntry<'snapshot>>,
    preferred: BinaryHeap<FrontierEntry<'snapshot>>,
    boost_remaining: u8,
    preferred_operator_boost: u8,
    use_preferred_next: bool,
}
```

Methods:
- `new(preferred_operator_boost: u8) -> Self` — initializes with empty heaps, `use_preferred_next = true`
- `push_regular(entry: FrontierEntry<'snapshot>)` — inserts into regular queue
- `push_preferred(entry: FrontierEntry<'snapshot>)` — inserts into preferred queue
- `push_both(entry: FrontierEntry<'snapshot>)` — clone into both queues (for preferred candidates that also go into regular)
- `pop() -> Option<SearchNode<'snapshot>>` — implements alternation: if `boost_remaining > 0` or `use_preferred_next`, pop from preferred (decrement boost); else pop from regular. Toggle `use_preferred_next` after each pop. Fall through to regular if preferred is empty.
- `trigger_boost(&mut self)` — sets `boost_remaining = preferred_operator_boost` when search progress is detected
- `is_empty() -> bool`
- `push(entry: FrontierEntry<'snapshot>)` — backwards-compatible push into regular (used during integration transition)

### 2. Write focused unit tests

Tests within `frontier.rs`:

- `test_dual_frontier_alternates` — preferred, regular, preferred, regular pattern
- `test_dual_frontier_boost` — after `trigger_boost()`, preferred is popped `boost + 1` times before regular
- `test_dual_frontier_preferred_empty_falls_through` — when preferred is empty, regular is popped regardless of alternation state
- `test_dual_frontier_both_empty_returns_none`
- `test_dual_frontier_ordering_preserved` — lower f-value nodes are popped first within each queue
- `test_dual_frontier_zero_boost` — with `preferred_operator_boost = 0`, alternates 1:1

Note: The existing `FrontierEntry` struct and its `Ord` impl are unchanged. `DualFrontier` composes two heaps of the same entry type.

## Files to Touch

- `crates/worldwake-ai/src/search/frontier.rs` (modify)

## Out of Scope

- Replacing the `BinaryHeap` in `search_plan()` with `DualFrontier` (S88TWOPHALAN-007)
- Determining which candidates are preferred (landmark logic from S88TWOPHALAN-003)
- Strategic planner (S88TWOPHALAN-006)

## Acceptance Criteria

### Tests That Must Pass

1. All 6+ focused unit tests for DualFrontier alternation, boost, fallback, and ordering
2. Existing suite: `cargo test -p worldwake-ai -- frontier`

### Invariants

1. When both queues are empty, `pop()` returns `None`
2. When preferred is empty, `pop()` always falls through to regular
3. Ordering within each queue is identical to the existing `FrontierEntry::Ord`
4. `boost_remaining` never exceeds `preferred_operator_boost`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/frontier.rs` (inline tests) — DualFrontier alternation, boost, fallback, ordering

### Commands

1. `cargo test -p worldwake-ai -- frontier`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
