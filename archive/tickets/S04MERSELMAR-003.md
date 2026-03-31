# S04MERSELMAR-003: `staff_market` action definition and handler

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new trade-domain action with start/tick/commit/abort lifecycle
**Deps**: S04MERSELMAR-001, S04MERSELMAR-002

## Problem

Merchants have no concrete action to establish sale presence at a market. `MerchandiseProfile` implies passive availability but no actual time-consuming market-staffing behavior exists. A `staff_market` action is needed so merchants spend bounded time listing lots for sale, making sale availability a real time-cost activity rather than a free side effect of profile state.

## Assumption Reassessment (2026-03-31)

1. Trade action registration follows the pattern in `crates/worldwake-systems/src/trade_actions.rs:14` — `register_trade_action()` registers def + handler pair and returns `ActionDefId`. The new action needs its own `register_staff_market_action()` following the same pattern.
2. Action handler lifecycle is `start / tick / commit / abort` as defined by `ActionHandler::new()`. Confirmed in `trade_actions.rs:18-19`.
3. `ActionDef` fields include `name`, `domain`, `target_spec`, `interruptibility`, `duration_expr`, `preconditions`, `visibility`, `body_costs`, `payload_entity_roles`. Confirmed via `crates/worldwake-sim/src/action_def.rs`.
4. `ActionDomain::Trade` exists and is used by the existing trade action. Confirmed at `trade_actions.rs:29`.
5. `SaleListing` component (ticket 001) and `StaffMarketPayload` (ticket 002) must exist before this ticket.
6. `MerchandiseProfile` has `sale_kinds: BTreeSet<CommodityKind>` and `home_market: EntityId`. Confirmed in `crates/worldwake-core/src/trade.rs`.
7. Action registration happens in `crates/worldwake-systems/src/action_registry.rs` which calls individual `register_*` functions. Line 31 shows `register_trade_action`.
8. `DurationExpr` variants include `Finite`, `ActorTradeDisposition`, etc. The spec says duration is `market_presence_ticks` from `TradeDispositionProfile`. A new `DurationExpr` variant or a `Finite` resolved from the profile is needed.
9. No adjacent contradictions found.

## Architecture Check

1. Follows the established action registration pattern exactly: separate `register_staff_market_action()` function, `ActionDef` with trade domain, handler with start/tick/commit/abort. Same shape as `register_trade_action`, `register_patrol_action`, etc.
2. No backwards-compatibility shims. This is a new action, not a modification of existing trade action.
3. Start behavior adds `SaleListing` to eligible lots. Commit/abort removes listings. This keeps listing state tightly coupled to the action lifecycle rather than requiring external cleanup for normal flows.

## Verification Layers

1. Listing attachment on start -> action trace + world state (SaleListing present on eligible lots after start)
2. Listing removal on commit/abort -> action trace + world state (SaleListing absent after commit/abort)
3. Precondition enforcement -> focused unit test (alive, at home_market, has MerchandiseProfile, commodity in sale_kinds, has local stock)
4. Duration from profile -> focused unit test (resolves to market_presence_ticks)

## What to Change

### 1. Add `register_staff_market_action()` in `trade_actions.rs`

New public function following the `register_trade_action` pattern:
- `ActionDef` with name `"staff_market"`, domain `ActionDomain::Trade`, `TargetSpec::Untargeted`, `Interruptibility::FreelyInterruptible`, visibility `VisibilitySpec::SamePlace`
- Duration resolves from `TradeDispositionProfile.market_presence_ticks`. Either add a new `DurationExpr` variant (e.g., `DurationExpr::ActorMarketPresence`) or resolve at start time to a `Finite` value.
- Preconditions: actor alive, not in transit, at home_market, has MerchandiseProfile, payload commodity in sale_kinds, controls at least one local lot of payload commodity

### 2. Implement `start_staff_market`

- Extract `StaffMarketPayload` from action instance
- Find all local lots directly possessed by actor matching the payload commodity
- Add `SaleListing { listed_at: current_tick }` to each eligible lot not already listed
- Return `Ok(())` or appropriate start result

### 3. Implement `tick_staff_market`

- No special mutation. Return normal tick progression.

### 4. Implement `commit_staff_market`

- Remove `SaleListing` from any still-controlled local lots of the payload commodity that were listed by this cycle
- Record `WantedToSellButNoBuyer` demand observation if no trades occurred during the cycle (lots that were traded away are no longer possessed, so remaining listed lots indicate unproductive presence)

### 5. Implement `abort_staff_market`

- Same listing removal as commit (remove `SaleListing` from controlled lots)
- No demand observation on abort (interrupted, not completed)

### 6. Wire registration in `action_registry.rs`

Add `register_staff_market_action(defs, handlers)` call in the central registration function.

### 7. Export from `lib.rs`

Add `register_staff_market_action` to `crates/worldwake-systems/src/lib.rs` exports.

## Files to Touch

- `crates/worldwake-systems/src/trade_actions.rs` (modify — add registration, def, handler functions)
- `crates/worldwake-systems/src/action_registry.rs` (modify — wire registration)
- `crates/worldwake-systems/src/lib.rs` (modify — export)
- `crates/worldwake-sim/src/action_semantics.rs` (modify — if new `DurationExpr` variant needed)

## Out of Scope

- Planner op registration (ticket 006)
- AI candidate generation for `SellCommodity` (ticket 007)
- Listing cleanup for edge cases (dead seller, left place) — that is ticket 005
- Trade commit rules for listed lots (ticket 009)
- Buyer-side changes (ticket 008)

## Acceptance Criteria

### Tests That Must Pass

1. `staff_market` start attaches `SaleListing` to all eligible local lots of the payload commodity
2. `staff_market` start does not list lots already listed
3. `staff_market` commit removes `SaleListing` from still-possessed lots
4. `staff_market` abort removes `SaleListing` from still-possessed lots
5. `staff_market` start fails if actor not at `home_market`
6. `staff_market` start fails if commodity not in `sale_kinds`
7. `staff_market` commit records `WantedToSellButNoBuyer` when no lots were traded during the cycle
8. Duration resolves to `market_presence_ticks` from `TradeDispositionProfile`
9. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. `SaleListing` is only added during `staff_market` start — no other code path creates listings
2. `staff_market` does not execute the trade itself — it only establishes sale visibility
3. Action is `FreelyInterruptible` — can be abandoned for higher-priority goals
4. Conservation: no items created or destroyed by `staff_market`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/trade_actions.rs` — focused unit tests for start/commit/abort listing lifecycle
2. `crates/worldwake-systems/src/trade_actions.rs` — precondition failure tests (wrong place, missing profile, wrong commodity)
3. `crates/worldwake-systems/src/trade_actions.rs` — demand observation recording on unproductive commit

### Commands

1. `cargo test -p worldwake-systems -- trade_action`
2. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-31
- **What changed**:
  - Added `DurationExpr::ActorMarketPresence` variant in `action_semantics.rs`, resolving to `market_presence_ticks` from `TradeDispositionProfile`
  - Implemented `register_staff_market_action()` in `trade_actions.rs` with full start/tick/commit/abort lifecycle
  - Start attaches `SaleListing` to eligible local lots; commit/abort removes them
  - Commit records `WantedToSellButNoBuyer` demand observation on unproductive cycles
  - Wired registration in `action_registry.rs`, exported from `lib.rs`
  - Added `PlannerOpKind::StaffMarket` in `planner_ops.rs` with non-barrier semantics
  - Updated exhaustive matches in `belief_view.rs`, `planner_duration_contract.rs`, `planning_state.rs`, `goal_model.rs`, `failure_handling.rs`, `observation.rs`
- **Deviations**: Used `clear_component_sale_listing` (WorldTxn API) instead of `remove_component_sale_listing` (World-only). Used `txn.tick()` instead of `current_tick()`. Both are API naming differences, not behavioral deviations.
- **Verification**: 9 new staff_market tests pass. Full workspace: clippy clean, all tests pass (0 failures)
