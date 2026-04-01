# S05MERSTOSTALL-010: Golden E2E tests for merchant stock storage lifecycle

**Status**: PENDING
**Priority**: LOW
**Effort**: Large
**Engine Changes**: None — golden test additions only
**Deps**: S05MERSTOSTALL-005, S05MERSTOSTALL-006, S05MERSTOSTALL-007, S05MERSTOSTALL-011

## Problem

The remaining uncovered parts of the facility stock lifecycle still need golden E2E coverage so the exact-facility merchant model stays stable under replay. Buyer trade against displayed lots, listing persistence, and merchant restock-return coverage now exist; this ticket should focus only on the lifecycle paths that still lack dedicated golden proof.

## Assumption Reassessment (2026-04-01)

1. Golden test harness exists in `golden_harness/mod.rs` — check for facility creation helpers or whether they need to be added to the harness.
2. `golden_merchant_selling.rs` already covers displayed listing persistence, buyer trade against listed lots, and `move_cargo`-to-sell plan shape — check only what lifecycle gaps remain after those completed scenarios.
3. Replay companions (deterministic replay) are the standard for golden tests — check existing replay companion pattern.
4. `PerceptionProfile` is required on agents that need to observe produced or newly materialized output — confirmed in `AGENTS.md` under `Authoritative-To-AI Impact Rule`.
5. Exact facility identity is now complete via 011 and should be treated as part of the live contract under test.
6. The previously deferred merchant golden failures are already resolved: `cargo test -p worldwake-ai --test golden_merchant_selling` and `cargo test -p worldwake-ai --test golden_trade` are green. This ticket should focus only on genuinely new scenario coverage, not on re-owning those older migrations.

## Architecture Check

1. Golden tests validate emergent cross-system behavior — they test the full agent decision cycle, not individual components. Each scenario exercises a distinct lifecycle path through the facility stock system.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Autonomous store→stage→sell lifecycle → golden E2E test with replay companion
2. Carrier delivers to facility without becoming seller → golden E2E test with replay companion
3. Unstage preserves ownership and item integrity → golden E2E test with replay companion
4. All new scenarios deterministically replay → replay companion verification
5. Existing merchant goldens stay green while the new scenarios are added
6. Single-layer ticket (golden E2E only) — additional layer mapping not applicable.

## What to Change

### 1. Add facility helper to golden harness

In `golden_harness/mod.rs`: add a helper for creating test facilities with stock/display containers, using the creation helpers from ticket 002.

### 2. Add autonomous store→stage→sell scenario

Merchant autonomously stores goods, stages for sale, buyer purchases. Verify the full lifecycle through event log.

### 3. Add carrier delivery scenario

Carrier agent delivers goods to facility via MoveCargo. Verify goods end up in stock container and carrier does not become the seller.

### 4. Add unstage round-trip scenario

Merchant stages then unstages goods. Verify ownership preserved, item integrity maintained, SaleListing cleared.

### 5. Add replay companions

Each scenario gets a deterministic replay companion verifying identical outcomes.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add facility helper)
- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — add only the remaining uncovered scenarios)

## Out of Scope

- Theft golden tests (depends on 008, separate coverage)
- Audit golden tests (depends on 009, separate coverage)
- Institutional or multi-merchant scenarios

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

1. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — store→stage→sell golden scenario
2. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — carrier delivery to facility
3. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — unstage round-trip preservation
4. replay companions for each new scenario
5. `crates/worldwake-ai/tests/golden_harness/mod.rs` — facility helper only if a remaining scenario truly needs one

### Commands

1. `cargo test -p worldwake-ai -- golden_merchant`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
