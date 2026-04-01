# S05MERSTOSTALL-007: Update AI planning for facility-based selling and restocking

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI candidate generation, planner ops, ranking
**Deps**: S05MERSTOSTALL-005, S05MERSTOSTALL-006, S05MERSTOSTALL-011

## Problem

AI agents still need to plan the remaining autonomous store→stage→sell flow for facility-based merchants. Exact facility identity, displayed-lot trading, and the basic `StaffMarket` sell path now work, but the AI still needs cleaner autonomous stock-management planning around when to store, stage, collect, and re-stage facility stock. Any remaining blocker vocabulary should be driven by the exact current stock-management failure surface, not by assumptions from the older place-level model.

## Assumption Reassessment (2026-04-01)

1. Sale visibility evolution (005) is complete — `listed_sale_lots_at` queries display containers, `authorized_seller_for_sale_lot` derives from facility control.
2. MoveCargo evolution (006) is complete — facility restock targets stock containers.
3. Facility identity is now explicit per facility via completed ticket 011. This ticket should build on that exact facility-targeting contract rather than re-solving identity.
4. `SellCommodity` candidate generation and goal satisfaction already use the home-facility place and listed-lot completion, but the at-home candidate path still keys off broad local controlled lots and suppresses when any lot is already listed in `candidate_generation.rs`. Reassess the remaining autonomous staging gap against that live behavior rather than assuming a direct-possession-only model.
5. `StaffMarket` planner op already exists and is the live sell-side progress barrier in `goal_model.rs` / `planner_ops.rs`, but `SELL_OPS` in `goal_dispatch_decl.rs` still omits `StockManagement`, so sell-goal search does not currently admit stock-management operators as part of the sell path.
6. `PlanningState` and `PlanningSnapshot` still model sale readiness through older sale-listing assumptions unless explicitly evolved: sell-side hypothetical search must preserve listing/seller state and stock-container/display-container movement consistently with the facility model.
7. Failure handling already covers `StaffMarket` and generic `StockManagement` plan failures in `failure_handling.rs`. Reassess whether new blocker variants are still required before expanding the vocabulary.
8. Stock action defs and planner-op classification exist (`store_stock`, `collect_display_stock`, `stage_stock_for_sale`, `unstage_stock`), but autonomous affordance exposure and use still need reassessment against the current AI boundary.
9. Partial progress already landed under this ticket: `SELL_OPS` now admits `StockManagement`, sell-side hypothetical planning preserves listing/seller state, and search/golden coverage proves lawful store→stage readiness. The remaining open slice is the coarse at-home `SellCommodity` candidate heuristic in `candidate_generation.rs`, which still suppresses the goal when any local lot is already listed instead of admitting mixed listed+unlisted facility stock.

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

### 1. Evolve the remaining SellCommodity candidate heuristic

In `candidate_generation.rs`: finish the remaining local sell heuristic work so merchants can still admit the right sell/stage work when they have mixed listed + unlisted stock at the home facility. The live heuristic should suppress `SellCommodity` only when all relevant local stock is already sale-ready, not merely because one listed lot exists. Do not reintroduce place-level ambiguity or a direct-possession-only contract.

### 2. Add the missing stock action affordance or admission surface

Completed in the current implementation slice: `SELL_OPS` now admits `StockManagement`, enabling autonomous store/stage/collect/unstage behavior in sell-path search at the merchant's exact home facility.

### 3. Evolve prerequisite stock-management planning around StaffMarket

Completed in the current implementation slice: `planner_ops.rs`, `planning_snapshot.rs`, `planning_state.rs`, and related search surfaces now admit staging and related stock-management steps as prerequisites around the existing `StaffMarket` sell path.

### 4. Add or refine BlockingFact coverage only if the live failure surface requires it

In `failure_handling.rs` and related blocker surfaces: only add new variants when the current stock-management failures cannot be expressed clearly with the existing structured reasons.

### 5. Update search candidates

In `search/candidates.rs`: ensure plan search generates staging steps as prerequisites for sell plans.

## Candidate Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — already completed in this ticket)
- `crates/worldwake-ai/src/goal_model.rs` (modify — already completed in this ticket)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — already completed in this ticket)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — already completed in this ticket)
- `crates/worldwake-ai/src/planning_state.rs` (modify — already completed in this ticket)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — reassess whether still needed after candidate-generation fix)
- `crates/worldwake-ai/src/affordance_query.rs` (modify — reassess whether still needed after candidate-generation fix)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — only if blocker vocabulary truly needs expansion)

## Deferred from S05MERSTOSTALL-005

The previously deferred merchant goldens from 005 no longer fail after the completed 011 work and adjacent test migration. This ticket no longer owns "make the old deferred goldens pass"; it owns only the remaining autonomous stock-management planning gaps that still exist after those suites turned green.

## Out of Scope

- Authorization/theft distinction (008)
- Audit hooks (009)
- New golden test scenarios (010) — but deferred golden test migration from 005 IS in scope

## Acceptance Criteria

### Tests That Must Pass

1. Sell-side candidate generation reflects the exact facility contract rather than a broad local-lot heuristic
2. Merchants with mixed displayed + stored stock at the same facility still admit the correct next sell or staging work
3. Plan search includes staging or related stock-management steps before selling when the live state requires it
4. Any missing stock-action affordance or admission surface for facility controllers is present
5. Dampening applies correctly for facility-based sell cycles
6. Existing merchant goldens stay green while the new behavior is added: `cargo test -p worldwake-ai --test golden_merchant_selling` and `cargo test -p worldwake-ai --test golden_trade`
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Belief-only planning — agent never reads world state directly
2. System decoupling — worldwake-ai depends on core and sim, not systems
3. Affordances reflect actual facility capabilities

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — mixed listed/unlisted stock at the home facility still admits the correct sell or staging work
2. live affordance/candidate surface — any missing stock action exposure for facility controllers
3. `crates/worldwake-ai/src/goal_dispatch_decl.rs` / `planner_ops.rs` / `planning_state.rs` — staging prerequisite in sell plans and facility-consistent hypothetical sell readiness
4. `crates/worldwake-ai/src/failure_handling.rs` — storage/staging failure mapping only if the current reasons are insufficient

### Commands

1. `cargo test -p worldwake-ai -- sell`
2. `cargo test -p worldwake-ai -- stock`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-04-01
- What changed:
  - Admitted `StockManagement` into the `SellCommodity` planner-op family so sell-path search can legally route through store/stage/collect/unstage steps before the existing `StaffMarket` barrier.
  - Extended sell-side hypothetical planning state to preserve sale-listing and seller identity across facility stock transitions, including stage, collect, and unstage transitions.
  - Added focused search and goal-model coverage proving facility-consistent sell readiness and the `stage_stock_for_sale` prerequisite for stored home stock.
  - Tightened the at-home `SellCommodity` candidate heuristic so mixed listed + unlisted home-facility stock still emits the sell goal, while fully sale-ready stock stays suppressed.
  - Updated merchant golden coverage to prove the corrected lawful store->stage readiness contract instead of the older unproductive `staff_market` narrative.
- Deviations from original plan:
  - The ticket was updated mid-implementation when reassessment showed the real remaining gap was narrower than the original broad "facility-based selling and restocking" wording. The completed change did not require `affordance_query.rs`, `failure_handling.rs`, or `search/candidates.rs` edits once the live planner/search boundary was revalidated.
  - The merchant golden change replaced an obsolete scenario invariant rather than merely adjusting timing because the corrected planner now lawfully settles the sell path through stock staging before any futile market-staffing cycle.
- Verification results:
  - `cargo test -p worldwake-ai merchant_at_home_facility_with_only_listed_stock_does_not_emit_sell_commodity -- --nocapture`
  - `cargo test -p worldwake-ai merchant_at_home_facility_with_mixed_listed_and_unlisted_stock_emits_sell_commodity -- --nocapture`
  - `cargo test -p worldwake-ai -- sell`
  - `cargo test -p worldwake-ai --test golden_merchant_selling`
  - `cargo test -p worldwake-ai --test golden_trade`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
