# S04MERSELMAR-004: Replace `agents_selling_at` with listed-lot belief queries

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — trait method replacement across RuntimeBeliefView and GoalBeliefView
**Deps**: S04MERSELMAR-001

## Problem

Buyer discovery currently finds sellers by querying `agents_selling_at(place, commodity)`, which filters agents by `MerchandiseProfile` presence — conflating enterprise intent with active sell availability. This must be replaced with concrete lot-based queries that only surface lots with `SaleListing` components. This is the central architectural cleanup of S04.

## Assumption Reassessment (2026-03-31)

1. `agents_selling_at` is defined on `RuntimeBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:229` and on `GoalBeliefView` at line 552. Confirmed.
2. Primary runtime implementation is in `crates/worldwake-sim/src/per_agent_belief_view.rs:996` — filters entities at place by `EntityKind::Agent` and `MerchandiseProfile.sale_kinds`. Confirmed.
3. `PlanningState` implementation at `crates/worldwake-ai/src/planning_state.rs:1688` has a full implementation that queries entities and filters by merchandise profile. Confirmed.
4. Test stubs returning `Vec::new()` exist in 13 files: `pressure.rs`, `feasibility.rs`, `affordance_query.rs`, `trade_valuation.rs`, `ranking.rs`, `failure_handling.rs`, `goal_explanation.rs`, `enterprise.rs`, `agent_tick/frame.rs`, `pursuit_belief.rs`, `agent_tick/tests.rs`, `planner_ops.rs`, `plan_revalidation.rs`, `exhaustion.rs`, `planning_snapshot.rs`. Confirmed via grep.
5. `candidate_generation.rs`, `goal_model.rs`, and `search/tests.rs` have non-trivial implementations that filter by merchandise profile. Confirmed.
6. No `omniscient_belief_view.rs` standalone file exists — the omniscient view is generated via macro in `belief_view.rs` (the `mock_belief_view!` or similar macro). The spec reference to this file is cosmetic.
7. `SaleListing` component must exist (ticket 001) before this ticket can be implemented.
8. No adjacent contradictions found.

## Architecture Check

1. Replacing one trait method with two methods (`listed_sale_lots_at` + `seller_for_sale_lot`) is cleaner because it separates lot discovery from seller derivation. Consumers that need the seller can call the second method; those that only need lots (e.g., trade payload assembly) only call the first.
2. No backwards-compatibility shims. The old `agents_selling_at` method is removed entirely from both traits and all implementations.
3. The replacement is mechanical for stub implementations (return `Vec::new()`) and requires real logic only in `per_agent_belief_view.rs` and `planning_state.rs`.

## Verification Layers

1. `listed_sale_lots_at` returns only lots with `SaleListing` -> focused unit test on `per_agent_belief_view`
2. `seller_for_sale_lot` derives seller from direct possessor -> focused unit test
3. Invalid listings not surfaced (no possessor, dead seller, wrong place) -> focused unit test
4. `agents_selling_at` fully removed -> compilation (any leftover call is a compile error)
5. Cross-system: planning_state uses new methods -> planning_state focused tests

## What to Change

### 1. Replace trait methods on `RuntimeBeliefView` in `belief_view.rs`

Remove:
```rust
fn agents_selling_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
```

Add:
```rust
fn listed_sale_lots_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId>;
fn seller_for_sale_lot(&self, lot: EntityId) -> Option<EntityId>;
```

### 2. Replace trait methods on `GoalBeliefView` in `belief_view.rs`

Same replacement as above.

### 3. Update `per_agent_belief_view.rs` implementation

Replace `agents_selling_at` at line 996 with:
- `listed_sale_lots_at`: query lots at place with matching commodity that have `SaleListing`, are directly possessed, and whose possessor is alive, capable, and co-located.
- `seller_for_sale_lot`: return direct possessor of the lot if valid.

### 4. Update `planning_state.rs` implementations

Replace both `PlanningState` impl at line 1688 and the test stub impl at line 2281. `PlanningState` must support hypothetical listing state for plan search.

### 5. Update all stub implementations (~13 files)

Replace `agents_selling_at` returning `Vec::new()` with the two new methods returning `Vec::new()` / `None` respectively in:
- `pressure.rs`, `feasibility.rs`, `affordance_query.rs`, `trade_valuation.rs`, `ranking.rs`, `failure_handling.rs`, `goal_explanation.rs`, `enterprise.rs`, `agent_tick/frame.rs`, `pursuit_belief.rs`, `agent_tick/tests.rs`, `planner_ops.rs`, `plan_revalidation.rs`, `exhaustion.rs`, `planning_snapshot.rs`

### 6. Update callers of `agents_selling_at`

Find all call sites (candidate_generation, goal_model, search/tests, affordance_query, planning_state, tell_actions) and migrate to use the new methods. The exact caller migration depends on context:
- Candidate generation for `AcquireCommodity` should use `listed_sale_lots_at` + `seller_for_sale_lot` (detailed in ticket 008)
- Search transition for Trade should use `listed_sale_lots_at` to find counterparty lots
- Tell actions stubs just need the method signature update

### 7. Update macro-generated belief view in `belief_view.rs`

The `mock_belief_view!` or delegate macro that generates the omniscient/macro view must be updated to delegate the new methods instead of the old one.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — trait definitions + macro)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — primary impl)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — stub + caller)
- `crates/worldwake-sim/src/trade_valuation.rs` (modify — stub)
- `crates/worldwake-ai/src/planning_state.rs` (modify — real impl + test stub)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — stub)
- `crates/worldwake-ai/src/pressure.rs` (modify — stub)
- `crates/worldwake-ai/src/feasibility.rs` (modify — stub)
- `crates/worldwake-ai/src/ranking.rs` (modify — stub)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — stub)
- `crates/worldwake-ai/src/goal_explanation.rs` (modify — stub)
- `crates/worldwake-ai/src/enterprise.rs` (modify — stub)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — stub)
- `crates/worldwake-ai/src/pursuit_belief.rs` (modify — stub)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — stub)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — stub)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — stub)
- `crates/worldwake-ai/src/exhaustion.rs` (modify — stub)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — caller)
- `crates/worldwake-ai/src/goal_model.rs` (modify — caller)
- `crates/worldwake-ai/src/search/tests.rs` (modify — test impl + callers)
- `crates/worldwake-systems/src/tell_actions.rs` (modify — stub)

## Out of Scope

- `SaleListing` component definition (ticket 001)
- `staff_market` action (ticket 003)
- `AcquireCommodity` evidence rework details (ticket 008 — this ticket provides the trait; 008 updates the AI usage)
- Trade commit validation against listed lots (ticket 009)
- Listing cleanup logic (ticket 005)

## Acceptance Criteria

### Tests That Must Pass

1. `listed_sale_lots_at` returns only lots with `SaleListing` at the queried place and commodity
2. `listed_sale_lots_at` excludes lots without `SaleListing` even if possessed by a merchant
3. `listed_sale_lots_at` excludes lots whose possessor is dead, incapacitated, or at a different place
4. `seller_for_sale_lot` returns the direct possessor when valid
5. `seller_for_sale_lot` returns `None` for unowned or invalid lots
6. `agents_selling_at` does not exist anywhere (compilation proves removal)
7. Existing suite: `cargo test --workspace`

### Invariants

1. No backward-compatibility shim preserves `agents_selling_at` — it is fully removed
2. Buyer discovery depends on concrete `SaleListing` state, not `MerchandiseProfile` alone
3. `MerchandiseProfile` remains enterprise intent — it is read by `staff_market` preconditions and candidate generation, but not by buyer discovery

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — focused test for `listed_sale_lots_at` with listed and unlisted lots
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` — focused test for `seller_for_sale_lot` with valid and invalid lots
3. `crates/worldwake-ai/src/planning_state.rs` — test `PlanningState` listed-lot query with hypothetical state
4. Multiple existing test files updated to compile with new trait signature

### Commands

1. `cargo test -p worldwake-sim -- belief_view`
2. `cargo test -p worldwake-ai -- planning_state`
3. `cargo clippy --workspace && cargo test --workspace`
