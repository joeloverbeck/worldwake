# S05MERSTOSTALL-007: Update AI planning for facility-based selling and restocking

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI candidate generation, planner ops, ranking
**Deps**: S05MERSTOSTALL-005, S05MERSTOSTALL-006, S05MERSTOSTALL-011

## Problem

AI agents need to plan the full store→stage→sell flow for facility-based merchants. `SellCommodity` must originate from displayed stock, affordances must include stock actions, and `StaffMarket` planner ops must evolve for the facility model. Blocking facts must cover storage/staging failures.

## Assumption Reassessment (2026-04-01)

1. Sale visibility evolution (005) is complete — `listed_sale_lots_at` queries display containers, `authorized_seller_for_sale_lot` derives from facility control.
2. MoveCargo evolution (006) is complete — facility restock targets stock containers.
3. Facility identity is still place-level, not explicit per facility — see S05MERSTOSTALL-011. This ticket should build on the exact facility-targeting contract rather than extending the current "any controlled facility at the place" behavior.
4. `SellCommodity` candidate generation exists — check how it currently identifies sellable lots and whether it assumes direct possession.
5. `StaffMarket` planner op exists — check current implementation and whether it needs evolution or replacement for facility model.
6. `BlockingFact` variants exist for plan failure handling — check `blocked_intent.rs` for existing variants and where storage/staging failures should be added.
7. Affordance generation for stock actions (store, stage, collect, unstage) does not yet exist — must be added to `affordance_query.rs`.

## Architecture Check

1. Extends existing AI planning infrastructure rather than introducing new planning paradigms. Stock actions become additional affordances, staging becomes a prerequisite step in sell plans, and blocking facts extend the existing failure vocabulary.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Sell candidates derive from displayed stock → candidate generation test
2. Staging appears as plan step before selling → plan search test
3. Stock action affordances generated for facility controllers → affordance query test
4. Dampening works for facility-based selling → ranking test
5. BlockingFact variants fire on storage/staging failures → failure handling test

## What to Change

### 1. Evolve SellCommodity candidate generation

In `candidate_generation.rs`: `SellCommodity` candidates must originate from lots with `StockAssignment::Displayed` in display containers, not from direct possession.

### 2. Add stock action affordances

In `affordance_query.rs`: generate affordances for `store_stock`, `collect_display_stock`, `stage_stock_for_sale`, `unstage_stock` when agent controls a facility.

### 3. Evolve StaffMarket planner op

In `planner_ops.rs`: evolve `StaffMarket` to work with the facility-based model — staging as a prerequisite for selling.

### 4. Add BlockingFact variants

In `blocked_intent.rs` and `failure_handling.rs`: add variants for storage failures (no facility, no stock container) and staging failures (no display container, lot not stored).

### 5. Update search candidates

In `search/candidates.rs`: ensure plan search generates staging steps as prerequisites for sell plans.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/search/candidates.rs` (modify)
- `crates/worldwake-ai/src/affordance_query.rs` (modify)
- `crates/worldwake-ai/src/blocked_intent.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)

## Deferred from S05MERSTOSTALL-005

Ticket 005 was completed partially — all production code and focused tests pass, but 11 golden tests fail because the AI planner does not yet support facility-based selling. The golden test setup (helpers, test bodies) was already migrated in 005:
- `seed_merchant` now creates a facility with display container and stages stock
- `seed_merchant_with_stored_stock` creates unstaged stock in the stock container
- Scenario 75 rewritten to test presence-only staff_market behavior
- `golden_trade.rs` trade harness updated with facility setup

**Once this ticket (007) provides AI planning support for displayed stock, the 11 failing golden tests should pass.** Verify by running `cargo test -p worldwake-ai --test golden_merchant_selling` and `cargo test -p worldwake-ai --test golden_trade` after implementation. Any remaining failures belong in ticket 010.

## Out of Scope

- Authorization/theft distinction (008)
- Audit hooks (009)
- New golden test scenarios (010) — but deferred golden test migration from 005 IS in scope

## Acceptance Criteria

### Tests That Must Pass

1. Sell candidates derive from displayed stock, not direct possession
2. Plan search includes staging step before selling
3. Stock action affordances generated for facility controllers
4. Dampening applies correctly for facility-based sell cycles
5. BlockingFact variants produced on storage/staging failures
6. Deferred golden tests from 005 pass: `cargo test -p worldwake-ai --test golden_merchant_selling` and `cargo test -p worldwake-ai --test golden_trade`
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Belief-only planning — agent never reads world state directly
2. System decoupling — worldwake-ai depends on core and sim, not systems
3. Affordances reflect actual facility capabilities

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — sell candidates from displayed stock
2. `crates/worldwake-ai/src/affordance_query.rs` — stock action affordances for facility controllers
3. `crates/worldwake-ai/src/planner_ops.rs` — staging prerequisite in sell plans
4. `crates/worldwake-ai/src/blocked_intent.rs` — storage/staging blocking fact variants

### Commands

1. `cargo test -p worldwake-ai -- sell`
2. `cargo test -p worldwake-ai -- stock`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
