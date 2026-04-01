# S04MERSELMAR-006: `PlannerOpKind::StaffMarket` and `SELL_OPS` update

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new planner op kind, updated goal dispatch ops list, search transition semantics
**Deps**: S04MERSELMAR-003

## Problem

The GOAP planner has no `PlannerOpKind` for the `staff_market` action, so it cannot produce plans that include market-presence steps. `SELL_OPS` currently contains `[Travel, Trade, MoveCargo]` — `Trade` must be removed (buyer-side, not seller goal) and `StaffMarket` added so `SellCommodity` goals can plan through `Travel -> MoveCargo -> StaffMarket`.

## Assumption Reassessment (2026-03-31)

1. `PlannerOpKind` enum in `crates/worldwake-ai/src/planner_ops.rs` currently has: `Travel`, `Eat`, `Drink`, `Sleep`, `Relieve`, `Harvest`, `Craft`, `Trade`, `MoveCargo`, `Loot`, `Attack`, `Defend`, `QueueForFacilityUse`, `Tell`, `ConsultRecord`, `Investigate`, `ClaimOffice`, `YieldForceClaim`, `Patrol`, `Accuse`, `PunishAccused`, `EstablishBanditCamp`, `RaidTarget`. Confirmed by grepping.
2. `SELL_OPS` at `crates/worldwake-ai/src/goal_dispatch_decl.rs:93-97` is `[Travel, Trade, MoveCargo]`. Confirmed by read.
3. Classification happens in `planner_ops.rs` via `classify()` or similar function matching `(ActionDomain, action_name)` to `PlannerOpKind`. Confirmed.
4. Search transition semantics for each `PlannerOpKind` are defined in `crates/worldwake-ai/src/search/transition.rs`. Each op kind has rules for how it transforms planning state.
5. `PlannerOpSemantics` struct defines barriers, mid-plan viability, and goal relevance per op kind. Located in `planner_ops.rs`.
6. `staff_market` action definition (ticket 003) must exist before this ticket — the planner op must classify from a real action.
7. No adjacent contradictions found.

## Architecture Check

1. Adding a new `PlannerOpKind` follows the established pattern exactly. Each new action domain gets its own op kind with classification, semantics, and search transition rules.
2. Removing `Trade` from `SELL_OPS` is correct — the seller goal is to establish market presence (StaffMarket), not to execute the trade itself (Trade is buyer-initiated).
3. No backwards-compatibility shims.

## Verification Layers

1. `PlannerOpKind::StaffMarket` classified from `(Trade, "staff_market")` -> focused unit test in planner_ops.rs
2. `SELL_OPS` contains `[Travel, MoveCargo, StaffMarket]` -> compilation + focused assertion
3. Search transition for StaffMarket -> focused planner conformance test
4. `PlannerOpSemantics` for StaffMarket -> focused unit test (barriers, viability checks)

## What to Change

### 1. Add `PlannerOpKind::StaffMarket` variant in `planner_ops.rs`

Add new variant to the enum. Add classification rule: `(ActionDomain::Trade, "staff_market") -> PlannerOpKind::StaffMarket`.

### 2. Define `PlannerOpSemantics` for `StaffMarket`

- Barriers: actor must be at `home_market`, must have local stock of the commodity
- Goal relevance: relevant to `SellCommodity`
- Mid-plan viability: requires actor at home_market with unlisted stock

### 3. Update `SELL_OPS` in `goal_dispatch_decl.rs`

Change from:
```rust
const SELL_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Trade,
    PlannerOpKind::MoveCargo,
];
```
To:
```rust
const SELL_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::MoveCargo,
    PlannerOpKind::StaffMarket,
];
```

### 4. Add search transition for `StaffMarket` in `search/transition.rs`

Define how `StaffMarket` transforms planning state:
- Precondition: actor at home_market, has local unlisted commodity stock
- Effect: marks commodity lots as listed (planning state override)
- This enables the planner to prove that `SellCommodity` is achievable through `Travel -> MoveCargo -> StaffMarket`

### 5. Update planner conformance tests

Add `StaffMarket` to planner conformance test coverage in `crates/worldwake-ai/tests/planner_conformance.rs`.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — add variant, classification, semantics)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — update SELL_OPS)
- `crates/worldwake-ai/src/search/transition.rs` (modify — add transition rules)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — if candidate expansion references op kinds)
- `crates/worldwake-ai/tests/planner_conformance.rs` (modify — add conformance coverage)

## Out of Scope

- `staff_market` action definition and handler (ticket 003 — must be done first)
- Candidate generation for `SellCommodity` (ticket 007)
- Goal satisfaction and feasibility (ticket 007)
- Buyer-side planning changes (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. `PlannerOpKind::StaffMarket` classifies from `(ActionDomain::Trade, "staff_market")`
2. `SELL_OPS` contains `Travel`, `MoveCargo`, `StaffMarket` and does NOT contain `Trade`
3. Planner search can produce a `Travel -> StaffMarket` plan for a merchant away from home_market
4. Planner search can produce a `MoveCargo -> StaffMarket` plan for a merchant with off-site stock
5. Search transition correctly marks lots as listed in planning state
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `Trade` op kind is removed from `SELL_OPS` — seller goals do not plan through buyer-initiated trade
2. `StaffMarket` semantics are terminal for `SellCommodity` — it is the goal-satisfying step
3. No backward-compatibility preserving `Trade` in `SELL_OPS`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — classification unit test for StaffMarket
2. `crates/worldwake-ai/src/planner_ops.rs` — semantics unit test for StaffMarket
3. `crates/worldwake-ai/src/search/transition.rs` — transition unit test for StaffMarket
4. `crates/worldwake-ai/tests/planner_conformance.rs` — conformance test for SellCommodity plans

### Commands

1. `cargo test -p worldwake-ai -- planner_ops`
2. `cargo test -p worldwake-ai -- planner_conformance`
3. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- **Completion date**: 2026-04-01
- **What changed**:
  - `SELL_OPS` updated from `[Travel, Trade, MoveCargo]` to `[Travel, MoveCargo, StaffMarket]` in `goal_dispatch_decl.rs`
  - Added `SellCommodity + StaffMarket` progress barrier in `goal_model.rs` `is_progress_barrier()`
  - 5 new unit tests: classification, semantics, SELL_OPS assertion, progress barrier (positive + negative)
- **Deviations**:
  - Items 1-2 (variant, classification, semantics) were already implemented by S04MERSELMAR-005. This ticket only needed the SELL_OPS update, progress barrier, and tests.
  - Item 4 (search transition in `transition.rs`) required no changes — `StaffMarket` uses `GoalModelFallback` transition, matching the pattern of other terminal ops (Tell, Patrol, Investigate).
  - Conformance test in `planner_conformance.rs` deferred to ticket 007 — conformance requires full candidate generation + handler integration setup.
- **Verification**: `cargo clippy --workspace -- -D warnings` clean, `cargo test --workspace` all pass (0 failures)
