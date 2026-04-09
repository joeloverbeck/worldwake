# AIDECREG-003: Reassess and fix `golden_trade_rejection_reroutes_to_reliable_seller`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No — stale golden setup only
**Deps**: AIDECREG-002

## Problem

After `AIDECREG-002` fixed `golden_witnessed_theft_accusation_chain`, the broader `cargo test -p worldwake-ai` rerun exposed a different real failure in `crates/worldwake-ai/tests/golden_trade.rs::golden_trade_rejection_reroutes_to_reliable_seller`. The failing assertion is `after learning the local seller's rejection, the buyer should reroute to the remote seller`. This now blocks honest same-crate full-suite verification.

## Assumption Reassessment (2026-04-09)

1. The failure reproduces in isolation with `cargo test -p worldwake-ai golden_trade_rejection_reroutes_to_reliable_seller -- --nocapture`, so it is not a broad-suite artifact.
2. Archived tickets [S48GOLGAP-001](/home/joeloverbeck/projects/worldwake/archive/tickets/S48GOLGAP-001.md) and [S55CAUBLOINV-003](/home/joeloverbeck/projects/worldwake/archive/tickets/S55CAUBLOINV-003.md) show this scenario has prior ownership/history, but there is no active ticket currently owning a new regression here.
3. The original lower-layer suspicion from `archive/tickets/S48GOLGAP-001.md` is no longer live. `cargo test -p worldwake-ai search_candidates_from_affordance_rejects_trade_for_wrong_seller_opportunity -- --nocapture` passes, so seller-binding in `search/candidates.rs` still matches the archived fix.
4. Temporary trace instrumentation showed that after the first `TradeBundleRejected(InsufficientPayment)` abort, later ticks produced `selected=none`, `candidates=0`, `plans_found=0`.
5. That disappearance was not caused by a new production blocker-memory regression. The buyer's `BlockedIntentMemory` remains empty after the rejection, so the remote branch is not being suppressed by a stored seller-scoped blocker.
6. The live contradiction is stale golden setup: after the first perception refresh, the buyer no longer lawfully retains the seeded remote seller/lot belief because the scenario never assigned an explicit `PerceptionProfile` with enough memory retention to keep the remote trade knowledge alive. Once the remote belief disappears, the reroute candidate cannot be regenerated.

## Architecture Check

1. A bounded reassessment ticket is cleaner than folding the newly exposed trade regression into `AIDECREG-002`, which owns the theft/tell handoff only.
2. The ticket should fix the earliest concrete contradiction: stale setup/proof if the reroute contract is still lawful, or production behavior if seller-rejection learning or reroute selection has regressed.
3. The earliest live contradiction is setup-only. The canonical path remains unchanged: retained remote seller belief -> source-reliability discount on the rejecting local seller -> later planning pass reroutes to the remote seller.

## Verification Layers

1. Buyer learns the local seller rejection through the live aftermath path -> authoritative belief/learned-state proof and/or focused lower-layer proof
2. Buyer lawfully retains the seeded remote seller/lot belief across the first perception refresh -> scenario-setup proof
3. Candidate generation/search later prefers the reliable remote seller -> decision trace and/or focused planner/runtime proof
4. Golden trade reroute contract remains valid -> `golden_trade_rejection_reroutes_to_reliable_seller`

## What to Change

### 1. Correct the stale scenario setup

- Update `crates/worldwake-ai/tests/golden_trade.rs` so the buyer has an explicit `PerceptionProfile` with sufficient retention/capacity to lawfully keep the seeded remote seller and sale-lot beliefs across the first refresh.

### 2. Keep the golden honest

- Remove temporary trace-only debugging scaffolding from `crates/worldwake-ai/tests/golden_trade.rs`.
- Keep the golden proving the full causal chain: local rejection, retained remote seller knowledge, learned seller unreliability, and reroute to the remote seller.

## Files to Touch

- `crates/worldwake-ai/tests/golden_trade.rs` (modify)

## Out of Scope

- Further work on `golden_witnessed_theft_accusation_chain`
- Broad trade-suite cleanup unrelated to seller-rejection rerouting

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_trade_rejection_reroutes_to_reliable_seller -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The final fix preserves the honest learned-rejection reroute contract rather than papering over the failure.
2. If the golden changes, its reroute/proof surface matches the live causal boundary.
3. The reroute remains attributable to retained lawful remote knowledge plus source unreliability, not to fabricated post-failure reseeding.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_trade.rs::golden_trade_rejection_reroutes_to_reliable_seller` — repaired broader-suite blocker

### Commands

1. `cargo test -p worldwake-ai golden_trade_rejection_reroutes_to_reliable_seller -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-04-09
- What changed:
  - corrected the golden setup in `crates/worldwake-ai/tests/golden_trade.rs` by assigning the buyer an explicit `PerceptionProfile` with enough retention/capacity to preserve the seeded remote seller belief through the first perception refresh
  - removed temporary reassessment-only trace/debug scaffolding once the live cause was proved
- Deviations from original plan:
  - the ticket started as a possible mixed-layer AI regression, but live reassessment proved the broader rerun failure was stale golden setup rather than a new production contradiction
  - no `worldwake-ai` production files changed
- Verification results:
  - `cargo test -p worldwake-ai golden_trade_rejection_reroutes_to_reliable_seller -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
