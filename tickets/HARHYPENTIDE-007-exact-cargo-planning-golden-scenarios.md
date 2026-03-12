# HARHYPENTIDE-007: Exact cargo planning tests and golden scenario

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — test-only ticket
**Deps**: HARHYPENTIDE-001 through HARHYPENTIDE-006 (all preceding tickets)
**Spec Reference**: HARDENING-hypothetical-entity-identity.md, Section E.5, Tests section

## Problem

After the hypothetical entity identity system is in place, there is no end-to-end proof that the full pipeline works: planner creates hypothetical entities during search → runtime binds them to authoritative entities after commit → later steps target the bound entities correctly → deterministic replay produces identical results.

## Assumption Reassessment (2026-03-12)

1. `crates/worldwake-ai/tests/golden_e2e.rs` exists with deterministic golden scenarios — confirmed.
2. No current test covers partial pickup → travel → put-down as a multi-step plan — confirmed.
3. No current test exercises hypothetical-to-authoritative binding across step boundaries — confirmed.
4. Deterministic replay infrastructure exists in `worldwake-sim` (`replay_and_verify()`) — confirmed.
5. Conservation verification (`verify_live_lot_conservation`, `verify_authoritative_conservation`) exists in `worldwake-core` — confirmed.

## Architecture Check

1. This is a test-only ticket. No production code changes.
2. Golden scenarios must be fully deterministic (seeded RNG, `BTreeMap` ordering, no floats).
3. Conservation must be verified at explicit phase boundaries.
4. Replay verification must confirm identical results for identical seeds.

## What to Change

### 1. Add search/planner-level multi-step cargo tests

In `crates/worldwake-ai/src/planner_ops.rs` or `search.rs` tests:

- Planner produces a plan with partial `pick_up` followed by `travel` — plan contains hypothetical target in later steps.
- Planner produces a plan with partial `pick_up` followed by `put_down` of the split-off lot — put-down step targets `PlanningEntityRef::Hypothetical(...)`.
- Verify generic non-materializing actions (eat, travel, sleep) still work with no behavior regression.

### 2. Add transport/action-level binding tests

In `crates/worldwake-systems/src/transport_actions.rs` tests or a new integration test:

- Authoritative partial `pick_up` returns `CommitOutcome` with `SplitOffLot` → runtime binds `H1 → real EntityId`.
- `put_down` after exact partial pickup resolves against the bound authoritative entity.

### 3. Add golden end-to-end scenario

In `crates/worldwake-ai/tests/golden_e2e.rs`:

**Scenario**: Actor partially picks up cargo, travels to another place, and puts it down.

Setup:
- Ground lot: `Water x 10` at Place A
- Actor at Place A with `CarryCapacity(LoadUnits(8))` and `Water` load-per-unit `LoadUnits(2)` → fits 4 units
- Place B adjacent to Place A

Expected behavior:
1. Agent plans: partial `pick_up(Water lot)` → `travel(Place B)` → `put_down(split-off lot)`
2. Partial pickup: original lot reduced to `Water x 6`, new lot `Water x 4` created in actor possession
3. Travel: actor moves to Place B with carried lot
4. Put-down: carried lot placed on ground at Place B
5. Conservation: total Water quantity remains 10 throughout

Assertions:
- Conservation verified at each phase boundary
- Deterministic replay yields identical state hashes
- Correct quantities at correct places after completion

### 4. Add replay verification for the golden scenario

After running the scenario, replay from the same seed and initial state. Verify all per-tick state hashes match.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify tests)
- `crates/worldwake-ai/src/search.rs` (modify tests)
- `crates/worldwake-ai/tests/golden_e2e.rs` (modify — new golden scenario)
- `crates/worldwake-systems/src/transport_actions.rs` (modify tests)

## Out of Scope

- Any production code changes
- New action families or new commodity types
- Carry-capacity rule changes
- Changes to replay infrastructure
- Changes to conservation verification infrastructure

## Acceptance Criteria

### Tests That Must Pass

1. Planner produces plan with partial `pick_up` followed by `travel` targeting hypothetical lot.
2. Planner produces plan with partial `pick_up` followed by `put_down` of hypothetical lot.
3. Generic non-materializing actions (eat, travel, sleep) show no behavior regression.
4. Authoritative partial `pick_up` → `CommitOutcome` → binding → `put_down` resolves correctly.
5. Golden scenario: actor partially picks up, travels, delivers — conservation holds throughout.
6. Deterministic replay of golden scenario yields identical results.
7. Existing suite: `cargo test --workspace`
8. Existing lint: `cargo clippy --workspace`

### Invariants

1. Conservation invariants hold at every explicit checkpoint.
2. Deterministic replay is stable (identical seeds → identical hashes).
3. No hypothetical entity leaks into authoritative world state.
4. All planner entity references are properly typed (no raw `EntityId` in plan targets).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — multi-step cargo plan search tests.
2. `crates/worldwake-ai/src/search.rs` — search produces plans with hypothetical targets.
3. `crates/worldwake-systems/src/transport_actions.rs` — binding integration tests.
4. `crates/worldwake-ai/tests/golden_e2e.rs` — `golden_partial_pickup_travel_deliver` scenario.

### Commands

1. `cargo test -p worldwake-ai planner_ops`
2. `cargo test -p worldwake-ai search`
3. `cargo test -p worldwake-ai --test golden_e2e`
4. `cargo test -p worldwake-systems transport`
5. `cargo test --workspace`
6. `cargo clippy --workspace`
