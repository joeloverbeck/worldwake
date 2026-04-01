# S04MERSELMAR-014: Remote merchant autonomously travels to home market to sell

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — candidate generation and goal dispatch for remote SellCommodity
**Deps**: S04MERSELMAR-011

## Problem

A merchant at a non-home-market place with stock and demand memory for their home market cannot autonomously plan travel to the home market to sell. `SellCommodity` only emits when the agent is already at `home_market` (candidate_generation.rs `emit_sell_goals` checks `current_place != home_market` and returns early). `RestockCommodity`/`MoveCargo` are designed for restocking (moving goods *to* a market where demand was observed), not for seller relocation. The golden test `move_cargo_then_sell_commodity_plan_shape` in `golden_merchant_selling.rs` is currently `#[ignore = "remote-merchant travel+sell pipeline not yet wired"]` because of this gap.

## Assumption Reassessment (2026-04-01)

1. `emit_sell_goals` at `crates/worldwake-ai/src/candidate_generation.rs:2390` checks `if current_place != home_market { return; }`. This prevents `SellCommodity` from being emitted when the merchant is at a remote place. Confirmed.
2. `goal_relevant_places` for `SellCommodity` at `crates/worldwake-ai/src/goal_model.rs:790` falls through to the catch-all which returns the current state — no special place logic directs the merchant toward `home_market`. Confirmed.
3. The S04 spec (Section 8) says `SellCommodity` planning semantics include `Travel` as a relevant op to get the merchant to `home_market`. The spec's `goal_relevant_places` section says `SellCommodity` must return `[home_market]` from `MerchandiseProfile`.
4. `SELL_OPS` in `goal_dispatch_decl.rs:96` includes `Travel`, `MoveCargo`, and `StaffMarket`. The search can plan Travel steps if the candidate is emitted.
5. GoalKind: `SellCommodity { commodity }`. Operator surface: `PlannerOpKind::StaffMarket` (terminal) + `PlannerOpKind::Travel` (auxiliary). The search is wired for multi-step plans through these ops.
6. The fix has two parts: (a) emit `SellCommodity` even when not at `home_market`, and (b) make `goal_relevant_places` return `home_market` so the search plans Travel + StaffMarket.
7. No adjacent contradictions. `RestockCommodity` has its own candidate emission and place logic — not affected.

## Architecture Check

1. Emitting `SellCommodity` from a remote place and letting the planner search Travel + StaffMarket is the cleanest approach. It follows the same pattern as other goals that require travel (e.g., `ClaimOffice` plans Travel to the office jurisdiction). No new goal kinds or special-purpose planner ops needed.
2. `goal_relevant_places` should return `[home_market]` for `SellCommodity`, matching the spec's Section 8 design.
3. No backwards-compatibility shims.

## Verification Layers

1. `SellCommodity` emitted when merchant is at remote place with stock -> focused candidate generation test
2. Plan search finds Travel + StaffMarket multi-step plan -> focused search test or decision trace
3. Merchant autonomously travels to home_market and starts staff_market -> un-ignore golden test 9
4. Single-layer ticket (AI candidate generation + goal model); authoritative action framework not modified.

## What to Change

### 1. Allow `SellCommodity` emission from remote places

In `crates/worldwake-ai/src/candidate_generation.rs`, modify `emit_sell_goals` to emit `SellCommodity` candidates even when `current_place != home_market`. The candidate should still require that the merchant has stock of the commodity (at any place they control, not necessarily locally).

### 2. Update `goal_relevant_places` for `SellCommodity`

In `crates/worldwake-ai/src/goal_model.rs`, add a `SellCommodity` arm to `goal_relevant_places` that returns `[home_market]` from `MerchandiseProfile`. This directs the planner's Travel heuristic toward the correct destination.

### 3. Un-ignore golden test 9

In `crates/worldwake-ai/tests/golden_merchant_selling.rs`, remove the `#[ignore]` from `move_cargo_then_sell_commodity_plan_shape` and verify it passes with the autonomous Travel + StaffMarket pipeline.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — emit SellCommodity from remote places)
- `crates/worldwake-ai/src/goal_model.rs` (modify — goal_relevant_places returns home_market)
- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — un-ignore test 9)

## Out of Scope

- `MoveCargo` candidate generation for seller relocation (not needed if SellCommodity + Travel works)
- Demand memory changes
- Valuation changes
- `RestockCommodity` modifications

## Acceptance Criteria

### Tests That Must Pass

1. `SellCommodity` candidate emitted when merchant is at a remote place with stock
2. Plan search finds Travel + StaffMarket for remote merchant
3. Golden test `move_cargo_then_sell_commodity_plan_shape` passes (un-ignored)
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `SellCommodity` candidate still emitted at home_market (existing behavior preserved)
2. Remote emission requires stock (no stock = no candidate, regardless of location)
3. Travel heuristic directs toward `home_market`, not arbitrary places
4. Enterprise goals never overpower survival-class goals

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused test: SellCommodity emitted at remote place
2. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — un-ignore test 9

### Commands

1. `cargo test -p worldwake-ai -- candidate_generation`
2. `cargo test -p worldwake-ai --test golden_merchant_selling`
3. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- **Completion date**: 2026-04-01
- **What changed**:
  - `emit_sell_goals` now emits `SellCommodity` from remote places (anchored at `home_market`) when the merchant has stock — removed `current_place != home_market` early return
  - `goal_relevant_places` already returned `[home_market]` for `SellCommodity` (no change needed — was done by a prior ticket)
  - Un-ignored `move_cargo_then_sell_commodity_plan_shape` golden test — now passes
  - Updated focused test `merchant_not_at_home_market_does_not_emit_sell_commodity` → `merchant_not_at_home_market_emits_sell_commodity_anchored_at_home` to assert new behavior
- **Deviations from original plan**:
  - Deliverable 2 (goal_relevant_places) was already implemented — no change needed
- **Verification**: `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo test --workspace` all tests pass, 29 golden merchant selling tests (0 ignored)
