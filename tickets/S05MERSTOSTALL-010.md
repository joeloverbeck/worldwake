# S05MERSTOSTALL-010: Golden E2E tests for merchant stock storage lifecycle

**Status**: PENDING
**Priority**: LOW
**Effort**: Large
**Engine Changes**: None — golden test additions only
**Deps**: S05MERSTOSTALL-005, S05MERSTOSTALL-006, S05MERSTOSTALL-007

## Problem

The full facility stock lifecycle (store→stage→sell, restock delivery, unstage round-trip) needs golden E2E test coverage to ensure cross-system emergent behavior works correctly and remains stable under replay.

## Assumption Reassessment (2026-04-01)

1. Golden test harness exists in `golden_harness/mod.rs` — check for facility creation helpers or whether they need to be added to the harness.
2. `golden_merchant_selling.rs` exists with merchant selling scenarios — check current scenarios and what new ones are needed.
3. Replay companions (deterministic replay) are the standard for golden tests — check existing replay companion pattern.
4. `PerceptionProfile` required on agents that need to observe post-production output — confirmed in CLAUDE.md.
5. All prerequisite systems (stock actions, sale visibility, MoveCargo evolution, AI planning) are complete via dependencies.
6. Golden test setup was partially migrated in ticket 005: `seed_merchant` creates facilities with display containers, `seed_merchant_with_stored_stock` creates unstaged stock, Scenario 75 rewritten. If ticket 007 resolves all deferred golden test failures, this ticket focuses on NEW scenarios only. If any deferred failures remain after 007, address them here first.

## Architecture Check

1. Golden tests validate emergent cross-system behavior — they test the full agent decision cycle, not individual components. Each scenario exercises a distinct lifecycle path through the facility stock system.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Autonomous store→stage→sell lifecycle → golden E2E test with replay companion
2. Buyer trades against displayed lot → golden E2E test with replay companion
3. Carrier delivers to facility without becoming seller → golden E2E test with replay companion
4. Unstage preserves ownership and item integrity → golden E2E test with replay companion
5. All scenarios deterministically replay → replay companion verification
6. Single-layer ticket (golden E2E only) — additional layer mapping not applicable.

## What to Change

### 1. Add facility helper to golden harness

In `golden_harness/mod.rs`: add a helper for creating test facilities with stock/display containers, using the creation helpers from ticket 002.

### 2. Add store→stage→sell scenario

Merchant autonomously stores goods, stages for sale, buyer purchases. Verify the full lifecycle through event log.

### 3. Add buyer trade against displayed lot scenario

Buyer agent arrives at facility place, perceives displayed lots, initiates trade. Verify trade completes against displayed (not possessed) stock.

### 4. Add carrier delivery scenario

Carrier agent delivers goods to facility via MoveCargo. Verify goods end up in stock container and carrier does not become the seller.

### 5. Add unstage round-trip scenario

Merchant stages then unstages goods. Verify ownership preserved, item integrity maintained, SaleListing cleared.

### 6. Add replay companions

Each scenario gets a deterministic replay companion verifying identical outcomes.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add facility helper)
- `crates/worldwake-ai/tests/golden_merchant_selling.rs` (modify — add scenarios)

## Out of Scope

- Theft golden tests (depends on 008, separate coverage)
- Audit golden tests (depends on 009, separate coverage)
- Institutional or multi-merchant scenarios

## Acceptance Criteria

### Tests That Must Pass

1. Autonomous store→stage→sell lifecycle completes successfully
2. Buyer trades against displayed lot (not possessed lot)
3. Carrier delivers to facility stock container without becoming seller
4. Unstage round-trip preserves ownership and clears SaleListing
5. All scenarios replay deterministically
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Conservation — items never created/destroyed across all scenarios
2. Determinism — replay produces identical event logs
3. Belief-only planning — agents plan from beliefs in all scenarios
4. Unique location — every entity in exactly one place throughout

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — store→stage→sell golden scenario
2. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — buyer trade against displayed lot
3. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — carrier delivery to facility
4. `crates/worldwake-ai/tests/golden_merchant_selling.rs` — unstage round-trip preservation
5. `crates/worldwake-ai/tests/golden_harness/mod.rs` — facility helper for golden tests

### Commands

1. `cargo test -p worldwake-ai -- golden_merchant`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
