# S05MERSTOSTALL-010: Golden E2E tests for merchant stock storage lifecycle

**Status**: ✅ COMPLETED
**Priority**: LOW
**Effort**: Large
**Engine Changes**: None — golden test additions only
**Deps**: S05MERSTOSTALL-005, S05MERSTOSTALL-006, S05MERSTOSTALL-007, S05MERSTOSTALL-011

## Problem

The remaining uncovered parts of the facility stock lifecycle still need golden E2E coverage so the exact-facility merchant model stays stable under replay. Buyer trade against displayed lots, listing persistence, merchant restock-return coverage, and loose-stock store-then-stage sell readiness already exist; this ticket should focus only on the lifecycle paths that still lack dedicated golden proof.

## Assumption Reassessment (2026-04-01)

1. `golden_merchant_selling.rs` already covers displayed listing persistence, buyer trade against listed lots, loose home stock being stored and staged before sell readiness, and `move_cargo`-to-sell plan shape — check only what lifecycle gaps remain after those completed scenarios.
2. `golden_trade.rs` already covers merchant restock-return and its deterministic replay companion; do not duplicate that proof under a second scenario name.
3. Replay companions (deterministic replay) are the standard for golden tests — check existing replay companion pattern.
4. `PerceptionProfile` is required on agents that need to observe produced or newly materialized output — confirmed in `AGENTS.md` under `Authoritative-To-AI Impact Rule`.
5. Exact facility identity is now complete via 011 and should be treated as part of the live contract under test.
6. The real remaining gaps are narrower: there is no dedicated golden proving `unstage_stock` returns displayed stock to facility storage while clearing `SaleListing`, and there is no dedicated non-selling carrier-delivery lifecycle golden distinct from the merchant-owned restock-return scenario.
7. The relevant live boundaries are merchant facility lifecycle goldens in `crates/worldwake-ai/tests/golden_merchant_selling.rs` and trade-domain transport lifecycle goldens in `crates/worldwake-ai/tests/golden_trade.rs`; no harness helper is currently blocking those additions because both suites already create facilities directly with `create_merchant_facility(...)`.

## Architecture Check

1. Golden tests validate emergent cross-system behavior — they test the full agent decision cycle or explicitly requested action path under the real harness, not individual components. Each scenario must exercise a distinct lifecycle path through the facility stock system.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Carrier delivery into facility stock does not make the carrier the seller or facility owner → golden E2E test with replay companion
2. `unstage_stock` returns displayed stock to facility storage and clears `SaleListing` without changing ownership → golden E2E test with replay companion
3. All new scenarios deterministically replay → replay companion verification
4. All new scenarios deterministically replay → replay companion verification
5. Existing merchant goldens stay green while the new scenarios are added
6. Single-layer ticket (golden E2E only) — additional layer mapping not applicable.

## What to Change

### 1. Add carrier delivery lifecycle scenario

In `golden_trade.rs`: add a distinct golden proving a non-selling carrier can deliver stock into a merchant-controlled facility and that the delivered lot ends in facility stock custody without transferring seller identity or facility ownership to the carrier.

### 2. Add unstage round-trip scenario

In `golden_merchant_selling.rs`: add a golden proving displayed stock can be unstaged back into the facility stock container, with `SaleListing` cleared and ownership preserved.

### 3. Add replay companions

Each newly added scenario gets a deterministic replay companion verifying identical outcomes.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add facility helper)
- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — add unstage lifecycle golden + replay)
- `crates/worldwake-ai/tests/golden_trade.rs` (modify — add carrier delivery lifecycle golden + replay)

## Out of Scope

- Theft golden tests (depends on 008, separate coverage)
- Audit golden tests (depends on 009, separate coverage)
- Institutional or multi-merchant scenarios
- Re-adding proof that already exists for loose-stock staging or merchant restock-return

## Acceptance Criteria

### Tests That Must Pass

1. Autonomous store→stage→sell lifecycle completes successfully
2. Carrier delivers to facility stock container without becoming seller
3. Unstage round-trip preserves ownership and clears `SaleListing`
4. All new scenarios replay deterministically
5. Existing merchant goldens stay green: `cargo test -p worldwake-ai --test golden_merchant_selling` and `cargo test -p worldwake-ai --test golden_trade`
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Conservation — items never created/destroyed across all scenarios
2. Determinism — replay produces identical event logs
3. Belief-only planning — agents plan from beliefs in all scenarios
4. Unique location — every entity in exactly one place throughout

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_trade.rs` — carrier delivery to facility stock without seller identity transfer
2. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — unstage round-trip preservation
3. replay companions for each new scenario

### Commands

1. `cargo test -p worldwake-ai -- golden_merchant`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-04-01
- What changed:
  - corrected the ticket to the live golden gap surface before coding: loose-stock store-then-stage proof and merchant restock-return were already covered, so the remaining work narrowed to a dedicated `unstage_stock` round-trip golden and a distinct carrier-delivery-to-facility golden
  - added `unstage_round_trip_preserves_storage_contract` plus deterministic replay coverage in `crates/worldwake-ai/tests/golden_merchant_selling.rs`, proving displayed stock returns to facility storage, ownership is preserved, and `SaleListing` is cleared
  - added `golden_carrier_delivery_to_facility_preserves_seller_identity` plus deterministic replay coverage in `crates/worldwake-ai/tests/golden_trade.rs`, proving a non-selling carrier can deliver stock into facility custody without becoming the later seller
  - refreshed the generated golden inventory and coverage docs after resolving the scenario-id collision introduced by the new trade scenario
- Deviations from original plan:
  - no `golden_harness/mod.rs` helper change was needed because the relevant merchant facility setup already existed in the live golden suites
  - the original ticket scope was stale and was narrowed before implementation to avoid duplicating already-covered store-stage-sell and merchant restock-return proof surfaces
- Verification results:
  - `cargo test -p worldwake-ai --test golden_merchant_selling unstage_round_trip_preserves_storage_contract -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_trade golden_carrier_delivery_to_facility_preserves_seller_identity -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_merchant_selling`
  - `cargo test -p worldwake-ai --test golden_trade`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `python3 scripts/golden_inventory.py --write --check-docs`
