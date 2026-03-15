# E15BSOCAIGOA-007: Golden social tests T1–T4 (Tell mechanics & belief transfer)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test file only
**Deps**: E15BSOCAIGOA-006

## Problem

The golden E2E test suite has zero coverage of E15 Tell features. These Tier 1 tests validate Tell mechanics, rumor degradation, discovery-triggered replanning, and skeptical rejection — all using existing code (no spec changes needed, InputQueue-injected Tell actions).

## Assumption Reassessment (2026-03-15)

1. `golden_social.rs` does not exist yet. Confirmed.
2. Tell action handler exists in `crates/worldwake-systems/` and is tested in systems-level tests — these golden tests exercise the full stack (Tell → belief → AI → planner).
3. `InputQueue` supports `InputKind::RequestAction` for injecting Tell actions manually.
4. Discovery events (InventoryDiscrepancy, EntityMissing) exist from E15 and fire on belief/observation mismatch.
5. `acceptance_fidelity: Permille(0)` should cause listener to reject all told beliefs.

## Architecture Check

1. New test file paralleling existing golden_*.rs files.
2. Each test is self-contained: setup → inject → step → assert.
3. Tests use existing Tell handler and belief system — no new production code.

## What to Change

### 1. Create golden_social.rs

Create `crates/worldwake-ai/tests/golden_social.rs` with 4 tests:

**T1: `golden_tell_transmits_belief_and_listener_replans`**
- Setup: Alice and Bob at Village Square. Bob hungry, no food, no knowledge of orchard. Orchard Farm has apples. Alice has DirectObservation belief about orchard.
- Inject Tell(Alice → Bob) via InputQueue.
- Assert: Bob receives Report belief about orchard → generates AcquireCommodity goal → plans Travel to Orchard Farm.
- Checks: Conservation (`verify_live_lot_conservation`), determinism (replay produces identical hashes).

**T2: `golden_rumor_chain_degrades_through_three_agents`**
- Setup: 3 agents (Alice, Bob, Carol) co-located. Alice has DirectObservation.
- Inject Tell(Alice→Bob), then Tell(Bob→Carol).
- Assert: Alice DirectObservation → Bob Report{chain_len:1} → Carol Rumor{chain_len:2}. Confidence ordering preserved.
- Checks: Determinism.

**T3: `golden_discovery_depleted_resource_triggers_replan`**
- Setup: Agent at Orchard Farm with stale belief orchard has apples (Quantity(10)). Actually Quantity(0). Agent hungry.
- Step simulation — agent attempts harvest.
- Assert: Passive observation fires → InventoryDiscrepancy Discovery event → agent replans.
- Checks: Conservation, determinism.

**T4: `golden_skeptical_listener_rejects_told_belief`**
- Setup: 2 agents co-located. Listener has `acceptance_fidelity: Permille(0)`. Speaker has fresh belief.
- Inject Tell(Speaker→Listener).
- Assert: Tell completes but listener's belief store unchanged. No travel goals generated.
- Checks: Determinism.

## Files to Touch

- `crates/worldwake-ai/tests/golden_social.rs` (new)

## Out of Scope

- Tests T5–T7 (E15BSOCAIGOA-008)
- Tests T8–T13 (E15BSOCAIGOA-009, E15BSOCAIGOA-010)
- Production code changes
- Golden harness modifications (E15BSOCAIGOA-006)
- Coverage report updates (E15BSOCAIGOA-010)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_tell_transmits_belief_and_listener_replans` — full Tell→belief→replan chain
2. `golden_rumor_chain_degrades_through_three_agents` — source degradation across relay
3. `golden_discovery_depleted_resource_triggers_replan` — mismatch triggers Discovery and replan
4. `golden_skeptical_listener_rejects_told_belief` — zero acceptance blocks belief transfer
5. All 4 tests verify determinism (replay with same seed produces identical state hashes)
6. T1, T3 verify conservation (`verify_live_lot_conservation` per tick)
7. Existing suite: `cargo test -p worldwake-ai` — no regressions

### Invariants

1. Tests exercise the full simulation stack (not unit-level mocks)
2. All tests use InputQueue injection (Tier 1 — no autonomous AI social goals needed)
3. Conservation verified every tick where items exist
4. Deterministic replay verified for every test

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` — 4 new golden E2E tests

### Commands

1. `cargo test -p worldwake-ai --test golden_social`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
