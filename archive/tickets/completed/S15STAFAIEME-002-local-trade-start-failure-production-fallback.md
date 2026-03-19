# S15STAFAIEME-002: Local Trade Start Failure Recovers Via Production Fallback

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — shared `BestEffort` request/start handling in `crates/worldwake-sim/src/tick_step.rs` plus `crates/worldwake-systems/src/trade_actions.rs` must route stale concrete trade requests into authoritative start rejection, with focused and golden coverage
**Deps**: `specs/S15-start-failure-emergence-golden-suites.md`, S08 start-failure architecture, Scenario 2b trade path already active

## Problem

The suite proves successful buyer-driven trade, but it does not prove that a lawful local market opportunity can disappear between planning and action start, record `StartFailed`, and push the losing buyer into a non-trade fallback chain. That leaves the S08 economic path unproven in golden coverage.

## Assumption Reassessment (2026-03-19)

1. Current trade goldens in [crates/worldwake-ai/tests/golden_trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs) are `golden_buyer_driven_trade_acquisition`, `golden_buyer_driven_trade_acquisition_replays_deterministically`, `golden_merchant_restock_return_stock`, and its replay companion. No existing trade golden asserts `StartFailed` or next-tick AI reconciliation for trade.
2. Current focused coverage proves trade affordance expansion and planner seams, including trade affordance payload expansion in [crates/worldwake-sim/src/affordance_query.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/affordance_query.rs), trade barrier search in [crates/worldwake-ai/src/search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs), and commit-time trade behavior in [crates/worldwake-systems/src/trade_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs). There is currently no focused test proving authoritative trade start rejection when the bundle is already stale at start.
3. The ticket cannot remain golden-only. In the live architecture, `start_trade` in [crates/worldwake-systems/src/trade_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs) only validates payload shape and returns `Ok(Some(ActionState::Empty))`, while authoritative stock/payment/acceptance checks run in `commit_trade`. As written, the requested trade stock race would become a commit-time abort, not a `StartFailed` path.
4. This ticket therefore targets both production code and golden E2E coverage. Full action registries are still required for the golden scenario because it spans needs, trade, production fallback, travel, traces, and authoritative world mutation.
5. Ordering is mixed-layer and not weight-only. The two buyers should be symmetric with respect to the local trade opportunity, then diverge only because one buyer lawfully consumes the seller's last local stock before the other buyer's trade start is attempted in the same authoritative world.
6. The scenario must explicitly avoid S10 pricing ambiguity. Use a seller with exactly one edible unit and a plainly acceptable 1:1 trade path so stock exhaustion, not valuation failure, is the hinge.
7. Scenario isolation is required: do not seed alternative local food lots, extra sellers, or unrelated local production paths that would let the losing buyer satisfy hunger without proving the intended "local trade start fails -> remote production fallback" branch.
8. Additional mismatch discovered during implementation: shared input handling in [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) currently calls `resolve_affordance` before `start_affordance`, and stale trade requests fail there as `RequestedAffordanceUnavailable` instead of reaching `start_trade`. The ticket therefore cannot be satisfied by `trade_actions` alone.
9. Scope correction: the required architectural substrate is a shared `BestEffort` path that can carry a concrete requested action through input revalidation and into authoritative start, where domain handlers return structured `ActionStartFailureReason`s. Trade still needs authoritative start-time validation in [crates/worldwake-systems/src/trade_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs), but the request-to-start handoff belongs in shared runtime code, not a trade-only exception.

## Architecture Check

1. A dedicated trade golden is cleaner than extending Scenario 2b because success-path trade and failure-recovery trade prove different contracts and should remain independently reviewable.
2. Validating concrete trade context at both start and commit is cleaner than leaving stale bundles to fail only at commit or introducing reservation-based trade exclusivity. Start-time validation gives the scheduler a truthful `StartFailed` boundary when the trade is already impossible, while commit-time validation still preserves lawful drift during negotiation ticks.
3. The shared runtime must own the `BestEffort` handoff from requested action to authoritative start. If stale concrete requests are rejected earlier as generic affordance mismatches, each domain would need its own workaround and the architecture would drift away from S08’s shared failure contract.
4. No backward-compatibility or trade-only recovery shim is allowed. The loser must recover through the same S08 failure handoff used elsewhere, and the trade action must continue to use ordinary authoritative state rather than hidden reservation aliases.

## Verification Layers

1. Both buyers can lawfully generate the local `AcquireCommodity` trade branch before stock disappears -> decision trace.
2. A stale concrete `BestEffort` trade request survives shared input resolution long enough to be rejected at authoritative start instead of dying as `RequestedAffordanceUnavailable` -> focused runtime test in `tick_step`.
3. The losing buyer's queued local trade start is authoritatively rejected after stock is consumed -> focused trade-action/runtime test plus action trace `StartFailed` and scheduler start-failure record.
4. The next AI tick consumes the failure and clears the dead local trade branch -> decision trace `planning.action_start_failures` and no stale retained trade step.
5. The losing buyer later travels, acquires food through production, and eats -> authoritative world state plus action-trace commits.
6. The loser does not livelock on repeated dead local trade starts after stock is gone -> decision trace history and negative action-trace check.
7. Commit-time trade revalidation still aborts lawfully if stock disappears after a successful start -> focused trade-action test.

## What to Change

### 1. Extend shared `BestEffort` request handling to preserve authoritative start-failure semantics

Update [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) so `BestEffort` action requests with a concrete requested def/targets/payload can still reach authoritative start validation even when the current belief-view affordance enumeration no longer reproduces the exact affordance. The shared path must remain generic, not trade-specific.

### 2. Add authoritative start validation to trade actions

Update [crates/worldwake-systems/src/trade_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs) so `start_trade` validates concrete trade context, accessible quantities, and bundle acceptance against authoritative state before the action is allowed to start. Preserve `commit_trade` revalidation so multi-tick negotiation still respects world drift after start.

### 3. Add focused runtime and trade-action coverage

Add narrow tests in [crates/worldwake-sim/src/tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs) and [crates/worldwake-systems/src/trade_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs) that prove:

- stale concrete `BestEffort` trade requests become `StartFailed` instead of `RequestedAffordanceUnavailable`
- trade start rejects when the requested commodity is already gone before start
- trade still aborts cleanly at commit if stock disappears after a valid start

### 4. Add the trade golden scenario

Add `golden_local_trade_start_failure_recovers_via_production_fallback` and `golden_local_trade_start_failure_recovers_via_production_fallback_replays_deterministically` in [crates/worldwake-ai/tests/golden_trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs).

The setup should include:

- two hungry buyers
- one local seller with exactly one edible unit
- a guaranteed-acceptable simple trade path
- a distant orchard or equivalent production fallback

### 5. Add minimal harness support only if it shrinks duplication

If needed, add narrow setup helpers in [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) for repeatable buyer/seller/fallback setup. Keep helper scope compositional; no custom trade shortcut paths.

## Files to Touch

- `crates/worldwake-sim/src/tick_step.rs` (modify)
- `crates/worldwake-systems/src/trade_actions.rs` (modify)
- `crates/worldwake-ai/tests/golden_trade.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, only if needed)

## Out of Scope

- `crates/worldwake-ai/tests/golden_production.rs`
- `crates/worldwake-ai/tests/golden_emergent.rs`
- S10 bilateral pricing or trade valuation redesign
- adding new market-information propagation systems
- weakening the ticket to a pure "trade failed somehow" assertion without S08 trace-layer proof

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim best_effort_trade_request_records_start_failure_when_affordance_goes_stale -- --exact`
2. `cargo test -p worldwake-systems trade_start_rejects_when_counterparty_lacks_requested_commodity -- --exact`
3. `cargo test -p worldwake-systems trade_aborts_when_counterparty_loses_requested_commodity_before_commit -- --exact`
4. `cargo test -p worldwake-ai derive_blocking_fact_uses_authoritative_trade_start_failure_when_belief_is_stale -- --exact`
5. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
6. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback_replays_deterministically -- --exact`
7. Existing guardrail: `cargo test -p worldwake-ai golden_buyer_driven_trade_acquisition -- --exact`
8. Existing guardrail: `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
9. Owning binaries: `cargo test -p worldwake-sim tick_step -- --nocapture`, `cargo test -p worldwake-systems trade_actions -- --nocapture`, and `cargo test -p worldwake-ai --test golden_trade`

### Invariants

1. The start failure must come from lawful world drift in local stock, not from pricing failure, arbitrary test mutation, or omniscient AI reads.
2. Shared `BestEffort` handling must not collapse stale concrete requests into generic affordance mismatch errors when the domain can provide a lawful authoritative start rejection.
3. Trade must reject already-impossible bundles at authoritative start without introducing reservation-based exclusivity or other hidden trade locks.
4. The losing buyer must not remain stuck on a stale local trade plan once the local stock is gone.
5. Trade and production systems remain decoupled; recovery happens through shared world state and ordinary replanning.
6. Commodity and coin conservation remain explicit and bounded by seeded stock and actual trade transfers.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/tick_step.rs` — add focused coverage for stale `BestEffort` trade requests reaching authoritative start failure.
2. `crates/worldwake-systems/src/trade_actions.rs` — add focused start-time and commit-time drift coverage for authoritative trade validation.
3. `crates/worldwake-ai/src/failure_handling.rs` — add/retain focused classification coverage for stale trade start failures mapping to `SellerOutOfStock`.
4. `crates/worldwake-ai/tests/golden_trade.rs` — add the local-trade start-failure golden and deterministic replay companion.
5. `crates/worldwake-ai/tests/golden_harness/mod.rs` — optional narrow helper additions only if they reduce duplicated setup.

### Commands

1. `cargo test -p worldwake-sim best_effort_trade_request_records_start_failure_when_affordance_goes_stale -- --exact`
2. `cargo test -p worldwake-systems trade_start_rejects_when_counterparty_lacks_requested_commodity -- --exact`
3. `cargo test -p worldwake-systems trade_aborts_when_counterparty_loses_requested_commodity_before_commit -- --exact`
4. `cargo test -p worldwake-ai derive_blocking_fact_uses_authoritative_trade_start_failure_when_belief_is_stale -- --exact`
5. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
6. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback_replays_deterministically -- --exact`
7. `cargo test -p worldwake-ai --test golden_trade`

## Outcome

- Completion date: 2026-03-19
- What actually changed:
  - shared `BestEffort` request handling in `crates/worldwake-sim/src/tick_step.rs` now preserves concrete stale requests long enough to reach authoritative start validation instead of collapsing them into generic `RequestedAffordanceUnavailable` errors
  - `crates/worldwake-systems/src/trade_actions.rs` now validates concrete trade context at `start_trade` and still revalidates at commit
  - `crates/worldwake-ai/src/failure_handling.rs` now classifies authoritative stale-trade start failures as `SellerOutOfStock` instead of misclassifying them as missing buyer input
  - `crates/worldwake-ai/tests/golden_trade.rs` now includes the S15 local-trade start-failure recovery golden and deterministic replay coverage
- Deviations from original plan:
  - the ticket started as a golden-only trade scenario, but current architecture required shared runtime work in `tick_step` before trade could participate in the S08 start-failure contract cleanly
  - the delivered golden uses lawful world drift plus a queued stale concrete request after an occupied self-care warmup, rather than relying on same-tick dual-AI trade selection, because the shared runtime now cleanly proves the stale-request-to-start-failure handoff
- Verification results:
  - `cargo test -p worldwake-sim best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches -- --exact`
  - `cargo test -p worldwake-systems trade_start_rejects_when_counterparty_lacks_requested_commodity`
  - `cargo test -p worldwake-systems trade_aborts_when_counterparty_loses_requested_commodity_before_commit`
  - `cargo test -p worldwake-ai derive_blocking_fact_uses_authoritative_trade_start_failure_when_belief_is_stale`
  - `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
  - `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback_replays_deterministically -- --exact`
  - `cargo test -p worldwake-ai golden_buyer_driven_trade_acquisition -- --exact`
  - `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
  - `cargo test -p worldwake-ai --test golden_trade`
  - `cargo clippy --workspace`
