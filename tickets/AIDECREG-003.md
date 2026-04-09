# AIDECREG-003: Reassess and fix `golden_trade_rejection_reroutes_to_reliable_seller`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — exact owning layer depends on reassessment of seller-rejection learning and remote reroute selection
**Deps**: AIDECREG-002

## Problem

After `AIDECREG-002` fixed `golden_witnessed_theft_accusation_chain`, the broader `cargo test -p worldwake-ai` rerun exposed a different real failure in `crates/worldwake-ai/tests/golden_trade.rs::golden_trade_rejection_reroutes_to_reliable_seller`. The failing assertion is `after learning the local seller's rejection, the buyer should reroute to the remote seller`. This now blocks honest same-crate full-suite verification.

## Assumption Reassessment (2026-04-09)

1. The failure reproduces in isolation with `cargo test -p worldwake-ai golden_trade_rejection_reroutes_to_reliable_seller -- --nocapture`, so it is not a broad-suite artifact.
2. Archived tickets [S48GOLGAP-001](/home/joeloverbeck/projects/worldwake/archive/tickets/S48GOLGAP-001.md) and [S55CAUBLOINV-003](/home/joeloverbeck/projects/worldwake/archive/tickets/S55CAUBLOINV-003.md) show this scenario has prior ownership/history, but there is no active ticket currently owning a new regression here.
3. The live boundary under audit is mixed-layer: seller-rejection aftermath, buyer belief/internalized preference update, and later trade candidate rerouting toward the reliable remote seller.

## Architecture Check

1. A bounded reassessment ticket is cleaner than folding the newly exposed trade regression into `AIDECREG-002`, which owns the theft/tell handoff only.
2. The ticket should fix the earliest concrete contradiction: stale setup/proof if the reroute contract is still lawful, or production behavior if seller-rejection learning or reroute selection has regressed.

## Verification Layers

1. Buyer learns the local seller rejection through the live aftermath path -> authoritative belief/learned-state proof and/or focused lower-layer proof
2. Candidate generation/search later prefers the reliable remote seller -> decision trace and/or focused planner/runtime proof
3. Golden trade reroute contract remains valid -> `golden_trade_rejection_reroutes_to_reliable_seller`

## What to Change

### 1. Reassess the failing trade golden against live code

- Name the exact seller-rejection, learning, and reroute-selection symbols under audit.
- Determine whether the failure is stale setup, stale proof surface, or a production regression.

### 2. Land the smallest honest fix

- If the golden setup or assertion surface is stale, update it to match the live rejection-learning contract.
- If production behavior regressed, fix the earliest concrete layer and keep the golden honest.

## Files to Touch

- `crates/worldwake-ai/tests/golden_trade.rs` (modify) and/or the exact owning production files revealed by reassessment

## Out of Scope

- Further work on `golden_witnessed_theft_accusation_chain`
- Broad trade-suite cleanup unrelated to seller-rejection rerouting

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_trade_rejection_reroutes_to_reliable_seller -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The final fix preserves the honest learned-rejection reroute contract rather than papering over the failure
2. If the golden changes, its reroute/proof surface matches the live causal boundary

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_trade.rs::golden_trade_rejection_reroutes_to_reliable_seller` — repaired broader-suite blocker
2. Additional focused lower-layer tests only if reassessment proves the current golden lacks enough provenance

### Commands

1. `cargo test -p worldwake-ai golden_trade_rejection_reroutes_to_reliable_seller -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
