# HARCARGOACON-006: Integration verification tests for cargo goal continuity

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-ai (agent_tick tests), worldwake-systems (integration tests)
**Deps**: HARCARGOACON-001 through HARCARGOACON-005 (all prior tickets)

## Problem

The full cargo-goal continuity architecture needs end-to-end verification: candidate emission → plan search → execution with materialization → dirty replan → goal retention → satisfaction at destination. Individual unit tests from prior tickets verify pieces; this ticket verifies the integrated behavior and the key architectural property: goal stability across replanning after materializing cargo steps.

## Assumption Reassessment (2026-03-12)

1. `agent_tick.rs` has existing cargo-related tests at lines 929, 1122-1154 — confirmed
2. `e10_production_transport_integration.rs` exists for production/transport integration tests — confirmed
3. The spec lists 8 concrete test scenarios in "Concrete Test Scenarios" — confirmed
4. Agent tick `observation_snapshot_changed` mechanism drives replanning — confirmed via spec Section "Agent Runtime Continuity"

## Architecture Check

1. Tests verify the architectural property (goal stability) rather than implementation details
2. No runtime hacks or cargo-specific suppression — continuity works through stable goal identity
3. Tests cover the full agent_tick → candidate → search → execute → replan cycle

## What to Change

### 1. Agent tick continuity tests (agent_tick.rs)

Add or update tests that verify:

**Scenario: Goal stability across replan after materialization**
- Set up agent with local cargo lot and merchandise profile with home_market demand
- Agent tick produces `MoveCargo { commodity, destination }` goal
- Execute `pick_up` (which may split the lot, creating new entity)
- Observation snapshot changes (dirty)
- Next agent tick re-derives the same `MoveCargo { commodity, destination }` goal key
- Plan is retained or rediscovered for the same goal

**Scenario: Satisfaction at destination while carrying**
- Agent arrives at destination carrying the commodity
- `MoveCargo` satisfaction check counts carried-at-destination stock
- Goal is satisfied without requiring explicit `put_down`

### 2. Merchant restock + delivery integration (agent_tick.rs or integration test)

**Scenario: Restock requires delivery to home_market**
- Merchant acquires commodity away from home_market
- `RestockCommodity` may be satisfied (agent has stock)
- But `MoveCargo` is emitted because home_market stock is below demand
- Delivery to home_market dampens cargo pressure

### 3. Conservation and replay verification (integration test)

**Scenario: Conservation invariant across full delivery**
- Set up a cargo delivery sequence: pick_up → travel → (optional put_down)
- Verify `verify_live_lot_conservation` holds at each tick
- Verify lot quantities are preserved through the sequence

**Scenario: Deterministic replay of cargo delivery**
- Record a cargo delivery sequence with seed and inputs
- Replay from same initial state + seed + inputs
- Verify identical per-tick state hashes

### 4. Edge case: zero deliverable suppression

- Agent at capacity → no `MoveCargo` emitted
- Zero restock gap → no `MoveCargo` emitted
- Agent already at destination → no `MoveCargo` emitted

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — add/update integration-style tests)
- `crates/worldwake-systems/tests/e10_production_transport_integration.rs` (modify — add cargo delivery coverage)

## Out of Scope

- Changing any production code (all implementation done in HARCARGOACON-001 through 005)
- Adding new action handlers or action definitions
- Modifying replay/save-load infrastructure
- Performance optimization of observation snapshots (separate hardening)
- Generic hauling jobs or non-merchant cargo delivery

## Acceptance Criteria

### Tests That Must Pass

1. `goal_stability_across_cargo_replan_after_materialization` — same `MoveCargo` goal key re-derived after lot-splitting pick_up
2. `cargo_satisfaction_at_destination_while_carrying` — carried stock at destination satisfies `MoveCargo`
3. `merchant_restock_requires_delivery_to_home_market` — acquiring stock remotely does not suppress `MoveCargo` emission for home_market
4. `no_cargo_candidate_at_full_carry_capacity` — zero `deliverable_quantity` suppresses emission
5. `cargo_delivery_preserves_conservation_invariant` — lot quantities conserved through pick_up → travel → put_down
6. `cargo_delivery_replays_deterministically` — same seed + inputs → identical state hashes
7. All tests from HARCARGOACON-001 through 005 remain passing
8. `cargo test --workspace` and `cargo clippy --workspace` pass

### Invariants

1. Cargo goal identity is stable across lot materialization — replanning finds the same `GoalKind::MoveCargo { commodity, destination }`
2. Conservation holds at every tick during cargo delivery sequences
3. Deterministic replay produces identical results for cargo delivery
4. No cargo-specific runtime suppression or dirtiness hacks exist
5. Enterprise procurement and cargo delivery remain decoupled through shared concrete state

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` — `goal_stability_across_cargo_replan_after_materialization`, `cargo_satisfaction_at_destination_while_carrying`, `merchant_restock_requires_delivery_to_home_market`, `no_cargo_candidate_at_full_carry_capacity`
2. `crates/worldwake-systems/tests/e10_production_transport_integration.rs` — `cargo_delivery_preserves_conservation_invariant`, `cargo_delivery_replays_deterministically`

### Commands

1. `cargo test -p worldwake-ai agent_tick`
2. `cargo test -p worldwake-systems --test e10_production_transport_integration`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
