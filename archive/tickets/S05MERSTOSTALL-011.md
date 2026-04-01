# S05MERSTOSTALL-011: Bind merchant stock logistics to explicit facility identity

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — merchant stock ownership/intent model, AI planning targets, and facility resolution
**Deps**: S05MERSTOSTALL-005, S05MERSTOSTALL-006

## Problem

Merchant stock custody is now facility-aware, but merchant intent is still keyed only by `MerchandiseProfile.home_market: Place`. The completed 006 work proves facility-custody delivery, yet the live planner and authoritative stock actions still resolve against "any controlled facility at the place" instead of an explicit merchant facility identity. That keeps stock transfer lawful but leaves the destination abstraction too coarse when one merchant can control multiple facilities at the same market or shop place.

## Assumption Reassessment (2026-04-01)

1. `MerchandiseProfile` still stores only `home_market: Option<EntityId>` with no facility reference in [`crates/worldwake-core/src/trade.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/trade.rs). Existing call sites already overload that field with both place ids and facility ids, which confirms the abstraction drift this ticket is correcting.
2. The completed 006 implementation made `MoveCargo` facility-custody aware, but the destination proof still starts from place-level intent: `restock_gap_at_destination` discovers controlled facilities by iterating entities at the destination place and aggregating their custody containers in [`crates/worldwake-ai/src/enterprise.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/enterprise.rs).
3. Hypothetical storage after travel still chooses an arbitrary controlled stock container at the destination place: `controlled_stock_containers_at_place(...).into_iter().next()` in [`crates/worldwake-ai/src/planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs).
4. Authoritative stock actions resolve the target facility the same way: `resolve_controlled_facility` returns the first controlled facility with `StockStoragePolicy` at the actor's place in [`crates/worldwake-systems/src/stock_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/stock_actions.rs).
5. This is a mixed-layer ticket. The shared abstraction boundary under audit is merchant stock destination identity: merchant intent/profile data, AI restock planning targets, and authoritative facility resolution must all agree on which exact facility receives stock.
6. Existing active tickets 007, 008, 009, and 010 assume the place-level facility model and do not currently resolve this identity ambiguity. Ticket 007 would deepen the ambiguity if it extends facility-based selling and restocking without first naming an exact facility target.
7. The adjacent contradiction exposed by 006 is a required architectural consequence, not a separate bug: facility-custody state became explicit, but merchant intent still collapses distinct facilities into one place-level endpoint.

## Architecture Check

1. Introducing explicit facility identity for merchant stock intent is cleaner than continuing to infer "the right facility" from place-local control order. It preserves concrete custody state and stable object identity rather than aggregating multiple lawful stock locations into one place-level abstraction.
2. This follows `FOUNDATIONS` Principle 4 (persistent identity and explicit transfer) and Principle 24 (ownership, custody, access, and jurisdiction are distinct). A merchant with multiple facilities at one place must be able to restock, stage, audit, and sell against one exact facility without relying on iteration order or "any controlled facility" fallback.
3. No backwards-compatibility aliasing/shims introduced. The old place-only path should be removed or fully migrated rather than kept in parallel.

## Verification Layers

1. Merchant stock intent names one exact facility, not just a place -> focused unit/runtime tests on the merchant profile / destination-identification surface
2. `MoveCargo` and follow-on stock actions target that exact facility -> planner search tests plus hypothetical transition tests
3. Authoritative `store_stock` / related stock actions resolve the same exact facility identity -> action/runtime tests in systems layer
4. Place-level ambiguity is eliminated for multi-facility merchants -> focused mixed-layer tests proving no iteration-order-dependent behavior remains
5. Ticket 007 handoff stays coherent after the identity change -> ticket reassessment and focused AI planning coverage

## What to Change

### 1. Replace the overloaded home-market field with explicit facility identity

Migrate `MerchandiseProfile` from place-level `home_market` semantics to an explicit facility reference. The chosen model must let merchant planning and runtime logic name one exact facility while deriving the relevant place from that facility's location where place-based travel, demand memory, or action anchoring still need it.

### 2. Update AI restock planning to target exact facilities

Remove "any controlled facility at destination place" behavior from the restock-gap and hypothetical storage path. `MoveCargo` must know which facility it is restocking, and the planner must route `store_stock` to that exact facility.

### 3. Update authoritative stock-action resolution to use exact facility identity

`store_stock`, `stage_stock_for_sale`, `unstage_stock`, and `collect_display_stock` should not resolve by first matching controlled facility at actor place when a stronger facility identity is available. The runtime path should either target the exact facility directly or prove why the binding is unambiguous.

### 4. Reassess downstream S05 tickets against the new facility identity contract

Update active tickets that currently assume place-only merchant stock intent, starting with 007, so later work does not reintroduce the ambiguity.

## Files to Touch

- `crates/worldwake-core/src/trade.rs` (modify)
- `crates/worldwake-ai/src/enterprise.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/feasibility.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-systems/src/stock_actions.rs` (modify)
- `crates/worldwake-systems/src/trade_actions.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify)
- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `tickets/S05MERSTOSTALL-007.md` (modify)

## Out of Scope

- Theft/crime classification itself (008)
- Audit mismatch detection flow itself (009)
- New golden scenarios beyond those required to keep the explicit-facility contract provable

## Acceptance Criteria

### Tests That Must Pass

1. Merchant stock intent identifies one exact facility rather than only a place
2. `MoveCargo` targets and stores into the intended facility even when multiple controlled facilities exist at the same place
3. Authoritative stock actions resolve the same exact facility identity as the planner path
4. Multi-facility merchant behavior does not depend on iteration order
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Facility stock custody remains concrete and tied to explicit facility identity
2. Belief-only planning preserved — AI reasons from belief-accessible facility bindings, not omniscient world lookup
3. No place-level ambiguity between multiple controlled facilities at the same market/shop place

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — multi-facility merchant restock targets the intended facility
2. `crates/worldwake-ai/src/goal_model.rs` / `enterprise.rs` — restock satisfaction is keyed to the intended facility, not aggregated place custody
3. `crates/worldwake-systems/src/stock_actions.rs` — authoritative stock actions use the same exact facility identity

### Commands

1. `cargo test -p worldwake-ai -- facility`
2. `cargo test -p worldwake-systems -- stock`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-04-01
- What changed:
  - Replaced the overloaded merchant home endpoint with explicit `MerchandiseProfile.home_facility` identity and propagated that contract through core fixtures, CLI scenario definitions, AI candidate generation, goal satisfaction, feasibility checks, planner transitions, belief/snapshot surfaces, and authoritative stock/trade actions.
  - Removed place-level "any controlled facility at destination" restock routing for merchant logistics by targeting the exact merchant facility in `MoveCargo`, `store_stock`, display staging, and related runtime validation.
  - Reassessed downstream ticket handoff and updated `S05MERSTOSTALL-007` to depend on the new exact-facility contract.
  - Refreshed focused and golden coverage so merchant restock, merchant selling, buyer-driven trade, and stale local-trade start-failure flows all prove the exact-facility model.
- Deviations from original plan:
  - The live codebase already used the old `home_market` field as both place and facility identity. The ticket was corrected before implementation to reflect the real symbol boundary and the broader touched files needed to remove that ambiguity safely.
  - Golden trade coverage required adapting stale scenario assumptions around listed/displayed stock and stale sale-lot failures. The final assertions now prove the current lawful failure family instead of assuming one older abort reason.
- Verification results:
  - `cargo test -p worldwake-ai -- facility`
  - `cargo test -p worldwake-ai goal_stability_across_cargo_materialization_continuity -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_merchant_selling`
  - `cargo test -p worldwake-ai --test golden_trade`
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-systems -- stock`
  - `cargo clippy --workspace --all-targets -- -D warnings`
