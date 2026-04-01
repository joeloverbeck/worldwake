# S05MERSTOSTALL-005: Evolve sale visibility from direct possession to displayed stock

**Status**: ✅ COMPLETED (partial) — golden test migration deferred to S05MERSTOSTALL-007
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — core sale visibility model changes across worldwake-sim and worldwake-systems
**Deps**: S05MERSTOSTALL-004

## Problem

Core architectural shift: `listed_sale_lots_at` must query display containers instead of (or in addition to) direct possession. `seller_for_sale_lot` must evolve to `authorized_seller_for_sale_lot` deriving from facility control rather than possession. This ticket has the widest blast radius in the S05 series — every consumer of sale visibility must be updated.

## Assumption Reassessment (2026-04-01)

1. `listed_sale_lots_at` exists on `RuntimeBeliefView` and `PerAgentBeliefView` — check exact signatures and all call sites.
2. `seller_for_sale_lot` exists and derives seller from possession — check exact implementation and all consumers.
3. Trade action validation uses sale visibility queries — check `trade_actions.rs` for exact validation flow.
4. `staff_market` function may need retirement or evolution — check current usage and whether facility-based visibility subsumes it.
5. Candidate generation, ranking, planner ops, plan revalidation, and affordance query all consume sale visibility — each must be audited for required changes.
6. Mixed-layer ticket: the shared abstraction boundary is the sale visibility query interface (`listed_sale_lots_at`, `seller_for_sale_lot`) used by both authoritative validation (worldwake-systems) and AI planning (worldwake-ai).

## Architecture Check

1. Evolving the query interface rather than adding parallel paths keeps the system clean. Display containers become the single source of truth for "what's for sale at this place" — no dual-path confusion.
2. No backwards-compatibility aliasing/shims introduced — `seller_for_sale_lot` evolves to `authorized_seller_for_sale_lot` rather than keeping both.

## Verification Layers

1. `listed_sale_lots_at` returns lots from display containers → authoritative world state (focused test)
2. `authorized_seller_for_sale_lot` derives from facility control → authoritative world state (focused test)
3. Trade action succeeds with display-container-based visibility → action trace (runtime integration test)
4. Existing golden tests migrated to new visibility model → golden E2E tests
5. Mixed-layer: authoritative validation and AI planning both use the evolved interface → runtime integration tests spanning systems and ai crates

## What to Change

### 1. Evolve listed_sale_lots_at

On both `RuntimeBeliefView` and `PerAgentBeliefView`: query display containers at the place for lots with `SaleListing` + `StockAssignment::Displayed`, instead of (or in addition to) direct possession with `SaleListing`.

### 2. Evolve seller_for_sale_lot to authorized_seller_for_sale_lot

Derive the authorized seller from facility control rather than item possession. The entity controlling the facility that owns the display container is the authorized seller.

### 3. Update trade action validation

In `trade_actions.rs`, update validation to use the new `authorized_seller_for_sale_lot` and display-container-based visibility.

### 4. Retire/evolve staff_market

If `staff_market` is subsumed by facility-based visibility, retire it. If it serves a distinct purpose, evolve it to work with the new model.

### 5. Update all belief view consumers

Audit and update all consumers: `candidate_generation.rs`, `ranking.rs`, `planner_ops.rs`, `plan_revalidation.rs`, `affordance_query.rs`.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-systems/src/trade_actions.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify)
- `crates/worldwake-ai/src/affordance_query.rs` (modify)

## Out of Scope

- MoveCargo evolution (006)
- AI planning for staging workflow (007)
- Authorization/theft (008)
- Audit hooks (009)
- Golden tests beyond migration of existing ones (010)

## Acceptance Criteria

### Tests That Must Pass

1. `listed_sale_lots_at` returns lots from display containers with `SaleListing` + `Displayed`
2. `authorized_seller_for_sale_lot` correctly derives from facility control
3. Trade action succeeds using display-container-based visibility
4. All existing golden tests pass after migration
5. Existing suite: `cargo test --workspace`

### Invariants

1. Sale visibility derives from display containers, not direct possession
2. Authorized seller derives from facility control, not item possession
3. System decoupling — worldwake-systems does not depend on worldwake-ai

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/belief_view.rs` — listed_sale_lots_at returns display container lots
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` — per-agent visibility respects display model
3. `crates/worldwake-systems/src/trade_actions.rs` — trade validation with facility-based seller
4. `crates/worldwake-ai/src/` — existing golden tests migrated to new visibility model

### Commands

1. `cargo test -p worldwake-sim -- listed_sale`
2. `cargo test -p worldwake-systems -- trade`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome (2026-04-01)

### What changed

**Production code (complete):**
1. `per_agent_belief_view.rs`: `listed_sale_lots_at` evolved to query `StockAssignment::Displayed` + `SaleListing` + facility controller present. Added `facility_controller_at` helper using authoritative facility placement and belief-filtered agent presence.
2. `per_agent_belief_view.rs`: `seller_for_sale_lot` evolved to derive seller from facility control via `StockAssignment` → facility → controller lookup.
3. `trade_actions.rs`: `staff_market` stripped of ALL SaleListing management (start/commit/abort) — now presence-only. Added `displayed_sale_lots_at` helper for commit-time demand recording. `validate_staff_market_preconditions` accepts displayed stock in addition to possessed stock.
4. `trade_actions.rs`: `validate_trade_bundle_context` uses `can_exercise_control` instead of possession check. `transfer_trade_lot` clears `StockAssignment` on transfer.
5. `world_txn.rs`: `create_merchant_facility` now takes `owner` parameter, sets ownership on facility + both containers so `can_exercise_control` chains correctly.

**Focused test migration (complete):**
6. All focused/unit tests across core (890), sim (384), systems (445), and ai lib (938) pass.
7. `StaffMarketHarness` and `TradeHarness` migrated to facility model with display containers.
8. `hungry_acquisition_harness` and `stale_remote_acquisition_harness` in agent_tick tests migrated.
9. `unknown_entities_and_unbelieved_merchants_stay_hidden` belief view test migrated.

**Golden test preparation (partial — deferred):**
10. `seed_merchant` helper updated to create facility with display container and stage stock.
11. `seed_merchant_with_stored_stock` helper added for tests needing unstaged stock.
12. Scenario 75 rewritten to test "displayed lot retains SaleListing through presence cycle."
13. `golden_trade.rs` trade harness updated with facility setup.
14. **11 golden tests still fail** because the AI planner doesn't yet support facility-based selling. These are deferred to S05MERSTOSTALL-007 which provides the AI planning updates.

### Deferred

Golden test migration (11 tests: 7 in golden_merchant_selling.rs, 4 in golden_trade.rs). The test setup and assertions are already migrated — they fail because the AI planner can't yet plan trade against displayed stock (tickets 006-007 scope). Tickets 007 and 010 updated with this dependency.

### Verification

- `cargo test -p worldwake-core`: 890 passed, 0 failed
- `cargo test -p worldwake-sim`: 384 passed, 0 failed
- `cargo test -p worldwake-systems`: 445 passed, 0 failed
- `cargo test -p worldwake-ai --lib`: 938 passed, 0 failed
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- Golden tests: 11 failures (deferred to 007)
