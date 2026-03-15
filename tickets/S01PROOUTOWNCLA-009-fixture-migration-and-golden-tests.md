# S01PROOUTOWNCLA-009: Migrate fixtures and add golden integration tests

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — test fixtures, prototype world setup, integration tests
**Deps**: S01PROOUTOWNCLA-001 through S01PROOUTOWNCLA-008 (all prior tickets)

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

### 1. Update `build_prototype_world()` in `topology.rs`

Add `ProductionOutputOwnershipPolicy` to all Facility and Place entities that have `ResourceSource` or `WorkstationTag`:
- Personal workstations → `ProductionOutputOwner::Actor`
- Public resource sources (berry bushes, etc.) → `ProductionOutputOwner::Unowned`
- Any faction/office-owned facilities → `ProductionOutputOwner::ProducerOwner`

### 2. Update test utilities

Any test helper that creates workstations or resource sources must also assign a `ProductionOutputOwnershipPolicy`. Update `test_utils.rs` and any inline test setup in `production_actions.rs`, `transport_actions.rs`, etc.

### 3. Update existing production tests

All existing harvest/craft tests must add explicit policy to their test workstations. The policy should match the test's intent (usually `Actor` for simple tests).

### 4. Add golden integration tests

Add integration tests that exercise the full production → ownership → pickup lifecycle:
- Harvest with `Actor` policy → agent picks up own goods
- Harvest with `ProducerOwner` policy → faction member picks up faction goods
- Craft with `Actor` policy → golden craft/barrier scenario still works
- Ownership survives travel (possessed lots retain ownership)
- `put_down` + `pick_up` cycle preserves ownership

### 5. Verify deterministic replay

Run replay tests to confirm deterministic replay remains unchanged after policy migration. The policy adds deterministic component state that should hash consistently.

## Files to Touch

- `crates/worldwake-core/src/topology.rs` (modify — `build_prototype_world` adds policies)
- `crates/worldwake-core/src/test_utils.rs` (modify — test helpers assign policies)
- `crates/worldwake-systems/src/production_actions.rs` (modify — test fixtures)
- `crates/worldwake-systems/src/transport_actions.rs` (modify — test fixtures)
- `crates/worldwake-systems/src/needs_actions.rs` (modify — test fixtures if production-related)
- `crates/worldwake-ai/` test modules (modify — golden E2E test fixtures)
- Integration test files as needed (new or modify)

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
