# S15STAFAIEME-002: Local Trade Start Failure Recovers Via Production Fallback

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None expected; golden/harness scope only unless a hidden runtime defect is exposed
**Deps**: `specs/S15-start-failure-emergence-golden-suites.md`, S08 start-failure architecture, Scenario 2b trade path already active

## Problem

The suite proves successful buyer-driven trade, but it does not prove that a lawful local market opportunity can disappear between planning and action start, record `StartFailed`, and push the losing buyer into a non-trade fallback chain. That leaves the S08 economic path unproven in golden coverage.

## Assumption Reassessment (2026-03-19)

1. Current trade goldens in [crates/worldwake-ai/tests/golden_trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs) are `golden_buyer_driven_trade_acquisition`, `golden_buyer_driven_trade_acquisition_replays_deterministically`, `golden_merchant_restock_return_stock`, and its replay companion. No existing trade golden asserts `StartFailed` or next-tick AI reconciliation.
2. Trade affordance and planner seams are already covered in focused/unit tests such as trade affordance expansion in [crates/worldwake-sim/src/affordance_query.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/affordance_query.rs) and trade-barrier search coverage in [crates/worldwake-ai/src/search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs). The missing layer is golden E2E proof.
3. This ticket targets golden E2E coverage. Full action registries are required because the scenario spans needs, trade, production fallback, travel, traces, and authoritative world mutation.
4. Ordering is mixed-layer and not weight-only. The two buyers should be symmetric with respect to the local trade opportunity, then diverge only because one buyer lawfully consumes the seller's last local stock before the other queued trade start resolves.
5. The scenario must explicitly avoid S10 pricing ambiguity. Use a seller with exactly one edible unit and a plainly acceptable 1:1 trade path so stock exhaustion, not valuation failure, is the hinge.
6. Scenario isolation is required: do not seed alternative local food lots, extra sellers, or unrelated local production paths that would let the losing buyer satisfy hunger without proving the intended "local trade fails -> remote production fallback" branch.
7. Scope correction: if the current trade handler cannot produce the needed lawful start rejection without broader engine work, update the ticket before implementation rather than folding that architecture work into this golden ticket.

## Architecture Check

1. A dedicated trade golden is cleaner than extending Scenario 2b because success-path trade and failure-recovery trade prove different contracts and should remain independently reviewable.
2. No backward-compatibility or trade-only recovery shim is allowed. The loser must recover through the same S08 failure handoff used elsewhere.

## Verification Layers

1. Both buyers can lawfully generate the local `AcquireCommodity` trade branch before stock disappears -> decision trace.
2. The losing buyer's queued local trade start is authoritatively rejected after stock is consumed -> action trace `StartFailed` and scheduler start-failure record.
3. The next AI tick consumes the failure and clears the dead local trade branch -> decision trace `planning.action_start_failures` and no stale retained trade step.
4. The losing buyer later travels, acquires food through production, and eats -> authoritative world state plus action-trace commits.
5. The loser does not livelock on repeated dead local trade starts after stock is gone -> decision trace history and negative action-trace check.

## What to Change

### 1. Add the trade golden scenario

Add `golden_local_trade_start_failure_recovers_via_production_fallback` and `golden_local_trade_start_failure_recovers_via_production_fallback_replays_deterministically` in [crates/worldwake-ai/tests/golden_trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs).

The setup should include:

- two hungry buyers
- one local seller with exactly one edible unit
- a guaranteed-acceptable simple trade path
- a distant orchard or equivalent production fallback

### 2. Add minimal harness support only if it shrinks duplication

If needed, add narrow setup helpers in [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) for repeatable buyer/seller/fallback setup. Keep helper scope compositional; no custom trade shortcut paths.

## Files to Touch

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

1. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
2. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback_replays_deterministically -- --exact`
3. Existing guardrail: `cargo test -p worldwake-ai golden_buyer_driven_trade_acquisition -- --exact`
4. Existing guardrail: `cargo test -p worldwake-ai golden_care_pre_start_wound_disappearance_records_blocker -- --exact`
5. Owning binary: `cargo test -p worldwake-ai --test golden_trade`

### Invariants

1. The start failure must come from lawful world drift in local stock, not from pricing failure, arbitrary test mutation, or omniscient AI reads.
2. The losing buyer must not remain stuck on a stale local trade plan once the local stock is gone.
3. Trade and production systems remain decoupled; recovery happens through shared world state and ordinary replanning.
4. Commodity and coin conservation remain explicit and bounded by seeded stock and actual trade transfers.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_trade.rs` — add the local-trade start-failure golden and deterministic replay companion.
2. `crates/worldwake-ai/tests/golden_harness/mod.rs` — optional narrow helper additions only if they reduce duplicated setup.

### Commands

1. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
2. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback_replays_deterministically -- --exact`
3. `cargo test -p worldwake-ai --test golden_trade`
