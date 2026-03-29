# S37COOBASEXH-003: Remove budget reduction in planning — use full budget on every retry

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — planning budget application in build_candidate_plans
**Deps**: S37COOBASEXH-002 (ExhaustionEntry no longer has effective_max_expansions)

## Problem

`build_candidate_plans()` in `planning.rs` reduces the search budget for budget-retry entries via `effective_max_expansions()`. With cooldown-based exhaustion, every retry should use the full `max_node_expansions` budget. The cooldown gate (ticket -004) ensures retries are spaced out instead.

## Assumption Reassessment (2026-03-29)

1. Budget reduction block is at `crates/worldwake-ai/src/agent_tick/planning.rs:238-252`. It calls `entry.is_budget_retry_pending()` and `entry.effective_max_expansions()` — both removed by S37COOBASEXH-002.
2. Spec S37 Section 3 specifies removing the budget-reduction block entirely. The comment at lines 238-241 about "Exponential backoff on search budget" becomes stale.
3. Single-layer ticket: only touches the budget application code path in `build_candidate_plans`. No cross-system boundary.
4. N/A — no golden scenario.
5. N/A — not directly planner-driven.
6. N/A — not an AI regression.
7. N/A — no ordering dependency.
8. Removing the budget-halving block: the cooldown gate in S37COOBASEXH-004 is the replacement mechanism that prevents excessive retry cost. Without the cooldown gate, agents would retry every tick at full budget — but -004 lands before any golden tests run.
9. N/A — not a stale-request ticket.
10-12. N/A.
13. No adjacent contradictions.
14. No mismatch.
15. N/A — no cumulative arithmetic.

## Architecture Check

1. Removing a conditional budget clone and replacing with a simple clone is strictly simpler. The cooldown mechanism (time-gating) replaces the budget-reduction mechanism (search-shallowing). This is the core design change of S37.
2. No backward-compatibility shims.

## Verification Layers

1. Full `max_node_expansions` used on every retry → existing `build_candidate_plans` tests updated to verify no budget reduction
2. Single-layer ticket: the budget application path. Integration behavior tested by golden tests after full chain lands.

## What to Change

### 1. Remove budget reduction block in `build_candidate_plans()`

In `crates/worldwake-ai/src/agent_tick/planning.rs`, replace lines 238-252:

Remove the comment about exponential backoff, the `opportunity` key construction (lines 240-243), and the `match exhaustion_cache.get(&opportunity)` block (lines 244-252).

Replace with:
```rust
let effective_budget = budget.clone();
```

The `opportunity` key variable constructed at lines 240-243 is only used by the budget-reduction match. If no other code in this function uses it, remove it entirely. (The candidate-filter already constructs its own `OpportunityKey` at lines 198-201.)

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)

## Out of Scope

- Candidate filtering changes (S37COOBASEXH-004)
- `has_pending_budget_retry` changes (S37COOBASEXH-004)
- Exhaustion recording changes (S37COOBASEXH-005)
- `ExhaustionEntry` struct changes (S37COOBASEXH-002)
- `PlanningBudget` changes (S37COOBASEXH-001)
- Decision trace or save/load changes (S37COOBASEXH-006, -007)
- Golden test adjustments

## Acceptance Criteria

### Tests That Must Pass

1. `build_candidate_plans` returns plans searched with full `max_node_expansions` regardless of exhaustion state
2. No reference to `effective_max_expansions` remains in `planning.rs`
3. Existing suite: `cargo test -p worldwake-ai -- build_candidate`

### Invariants

1. Search budget is always `PlanningBudget::max_node_expansions` (224 by default) — never halved
2. `build_candidate_plans` function signature unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — update or remove tests that asserted budget halving behavior
2. Any test referencing `effective_max_expansions` in planning context must be updated

### Commands

1. `cargo test -p worldwake-ai -- build_candidate`
2. `cargo clippy --workspace && cargo test -p worldwake-ai`
