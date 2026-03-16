# S01PROOUTOWNCLA-009: Golden integration tests for ownership lifecycle

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No — integration tests only
**Deps**: S01PROOUTOWNCLA-009a (fixture migration), S01PROOUTOWNCLA-004 through S01PROOUTOWNCLA-008 (all engine tickets)

> **Note**: This ticket was split. Fixture migration moved to S01PROOUTOWNCLA-009a (must run BEFORE engine tickets -004 through -008). This ticket now covers only golden integration tests and replay verification, which require all engine changes to be in place. See `tickets/S01-IMPLEMENTATION-ORDER.md` for the corrected sequence.

## Problem

All existing production scenarios (harvest, craft, trade tests, golden E2E tests) use the pre-S01 setup where workstations/sources lack `ProductionOutputOwnershipPolicy`. After S01PROOUTOWNCLA-001 makes the policy mandatory (no fallback defaults), these scenarios will fail unless migrated to declare explicit ownership policies. Additionally, the spec requires golden integration tests proving the full ownership lifecycle.

## Assumption Reassessment (2026-03-15)

1. `build_prototype_world()` in `topology.rs` sets up prototype places and facilities — confirmed
2. Test helpers in `test_utils.rs` create test workstations — confirmed
3. Golden E2E tests exist in `crates/worldwake-ai/` and `crates/worldwake-systems/` — confirmed
4. Existing production tests will break without explicit policy assignment — confirmed (by design)
5. Deterministic replay must remain unchanged after migration — confirmed

## Architecture Check

1. No backward-compatibility aliases — explicit policy on every producer
2. Migration cost is deliberate: every test fixture must declare its ownership intent
3. Prototype world defaults: personal workstations use `Actor`, unowned natural sources use `Unowned`
4. No new abstractions — just adding component assignments to existing setup code

## What to Change

> Fixture migration (build_prototype_world, test helpers, existing test fixtures) is handled by S01PROOUTOWNCLA-009a.

### 1. Add golden integration tests

Add integration tests that exercise the full production → ownership → pickup lifecycle:
- Harvest with `Actor` policy → agent picks up own goods
- Harvest with `ProducerOwner` policy → faction member picks up faction goods
- Craft with `Actor` policy → golden craft/barrier scenario still works
- Ownership survives travel (possessed lots retain ownership)
- `put_down` + `pick_up` cycle preserves ownership

### 2. Verify deterministic replay

Run replay tests to confirm deterministic replay remains unchanged after policy migration. The policy adds deterministic component state that should hash consistently.

## Files to Touch

- `crates/worldwake-systems/tests/` or `crates/worldwake-ai/tests/` (new — golden integration test files)
- Existing integration test files as needed for replay verification

## Out of Scope

- Adding new ownership variants beyond Actor/ProducerOwner/Unowned
- Theft implementation (E17)
- Merchant stock custody semantics (S05)
- E16b force legitimacy integration

## Acceptance Criteria

### Tests That Must Pass

1. Golden craft/barrier scenarios still work under explicit actor-owned output
2. Deterministic replay remains unchanged after policy migration
3. All existing production tests pass with explicit policies
4. Integration: harvest → actor-owned lot → lawful pickup → possession
5. Integration: harvest at faction workstation → faction-owned lot → faction member pickup
6. Integration: harvest at unowned source → unowned lot → free pickup
7. Integration: owned lot put down → still owned → original owner can pick up
8. Full workspace: `cargo test --workspace` passes
9. Full workspace: `cargo clippy --workspace` passes with no warnings

### Invariants

1. No producer entity exists without a `ProductionOutputOwnershipPolicy` (except entities that don't produce)
2. No backward-compatibility aliases or silent defaults
3. Conservation invariant holds across all migrated scenarios
4. Deterministic replay consistency maintained
5. All 17+ spec test cases from S01 spec lines 329-348 are covered

## Test Plan

### New/Modified Tests

1. Golden integration tests in `crates/worldwake-systems/` or `crates/worldwake-ai/` — full lifecycle coverage
2. Updated fixture tests across all crates that create production entities

### Commands

1. `cargo test --workspace`
2. `cargo clippy --workspace`
3. `cargo test -p worldwake-sim replay` (replay consistency)

## Outcome

- **Completion date**: 2026-03-16
- **What changed**: Golden integration tests for the full ownership lifecycle were implemented across multiple test files:
  - `crates/worldwake-ai/tests/golden_production.rs`: `golden_materialized_output_ownership_prevents_theft` (actor-owned output prevents theft, crafter eats own bread, thief must use orchard), `golden_materialization_barrier_chain` (harvest→pickup→eat with actor policy)
  - `crates/worldwake-systems/src/production_actions.rs`: Unit tests for all three ownership variants (Actor, ProducerOwner, Unowned) for both harvest and craft, plus ProducerOwner-on-ownerless-producer failure tests, ownership relation delta verification, and `craft_golden_scenario_works_with_actor_owned_output`
  - `crates/worldwake-systems/src/transport_actions.rs`: `pick_up_succeeds_for_actor_owned_lot`, `pick_up_succeeds_for_unowned_lot`
  - `crates/worldwake-systems/tests/e10_production_transport_integration.rs`: Full lifecycle (harvest→pickup→travel→put_down) with Actor policy, deterministic replay verification, craft conservation
  - `crates/worldwake-ai/tests/golden_production.rs`: 4 deterministic replay verification tests
  - `crates/worldwake-sim/src/affordance_query.rs`: Pickup affordance tests for unowned/controlled lots
- **Deviations**: Tests were distributed across existing test files rather than a single new golden integration test file, which provides better locality and coverage
- **Verification**: `cargo test --workspace` passes, `cargo clippy --workspace` clean, all 9 acceptance criteria satisfied
