# S01PROOUTOWNCLA-009a: Migrate test fixtures to declare explicit ownership policies

**Status**: ✅ COMPLETED
**Priority**: CRITICAL (blocks -004 through -008)
**Effort**: Small–Medium
**Engine Changes**: No — test fixtures and helpers only
**Deps**: S01PROOUTOWNCLA-001 (types exist), S01PROOUTOWNCLA-002 (helper exists)

## Problem

All existing test fixtures that create workstations or resource sources lack `ProductionOutputOwnershipPolicy`. Once -004 makes `commit_harvest` fail on missing policies, every existing harvest test breaks. The same applies to -005 (craft) and -007 (pickup validation).

This ticket migrates all test fixtures to declare explicit ownership policies BEFORE the engine changes land.

## What to Change

### 1. Golden test harness helpers (`crates/worldwake-ai/tests/golden_harness/mod.rs`)

Update these three helper functions to accept and set `ProductionOutputOwnershipPolicy`:

| Function | Line | Change |
|----------|------|--------|
| `place_workstation_with_source()` | ~358 | Add `policy: ProductionOutputOwnershipPolicy` parameter, call `set_component_production_output_ownership_policy()` |
| `place_exclusive_workstation_with_source()` | ~375 | Same pattern |
| `place_workstation()` | ~397 | Add optional policy parameter (workstations without resource sources may not produce, but if they have a `WorkstationTag` they may be used for crafting) |

Default convention for callers:
- Personal workstations → `ProductionOutputOwner::Actor`
- Public resource sources → `ProductionOutputOwner::Unowned`
- Faction/office-owned facilities → `ProductionOutputOwner::ProducerOwner`

### 2. Update all golden test callers

Update every call site of the modified helpers across:
- `golden_production.rs` (~13 call sites)
- `golden_trade.rs` (~2 call sites)
- `golden_determinism.rs` (~1 call site)
- `golden_social.rs` (~6 call sites)
- `golden_ai_decisions.rs` (~4 call sites)
- `golden_combat.rs` (~2 call sites)

Most callers should pass `ProductionOutputOwner::Actor` (personal production scenarios).

### 3. Update `setup_world()` in `production_actions.rs` tests (~line 709)

After setting `WorkstationMarker` and `ResourceSource`, add:
```rust
txn.set_component_production_output_ownership_policy(
    workstation,
    ProductionOutputOwnershipPolicy { output_owner: ProductionOutputOwner::Actor },
).unwrap();
```

### 4. Update `e10_production_transport_integration.rs` (~line 164)

The `Harness::new` function creates orchard and mill workstations directly. Add policy assignment for both.

### 5. Audit remaining test files

Grep all test code for `set_component_workstation_marker` or `set_component_resource_source` and verify each co-located with a policy assignment. Files to check:
- `transport_actions.rs` tests
- `needs_actions.rs` tests
- Any other integration test files in `crates/worldwake-systems/tests/`

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — helper signatures)
- `crates/worldwake-ai/tests/golden_production.rs` (modify — caller updates)
- `crates/worldwake-ai/tests/golden_trade.rs` (modify — caller updates)
- `crates/worldwake-ai/tests/golden_determinism.rs` (modify — caller updates)
- `crates/worldwake-ai/tests/golden_social.rs` (modify — caller updates)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify — caller updates)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify — caller updates)
- `crates/worldwake-systems/src/production_actions.rs` (modify — `setup_world` test helper)
- `crates/worldwake-systems/tests/e10_production_transport_integration.rs` (modify — `Harness::new`)

## Out of Scope

- Engine changes to `commit_harvest` or `commit_craft` (S01-004, -005)
- Belief view changes (S01-006)
- Pickup validation changes (S01-007, -008)
- Golden integration tests proving ownership lifecycle (S01-009b / original -009)
- Changes to `build_prototype_world()` in `topology.rs` (only needed if prototype world creates Facility entities — research indicates it only creates Places and TravelEdges)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` passes with zero failures
2. `cargo clippy --workspace` passes with no warnings
3. Every Facility or Place entity that has a `WorkstationMarker` or `ResourceSource` in test code also has an explicit `ProductionOutputOwnershipPolicy`
4. No backward-compatibility aliases or default policies — every producer declares intent

### Invariants

1. No behavioral changes to production — this is purely additive component assignment
2. Deterministic replay consistency maintained (policy is deterministic component state)
3. No test logic changes — only fixture setup additions
4. Conservation invariant unaffected

## Test Plan

### Commands

1. `cargo test --workspace`
2. `cargo clippy --workspace`
3. Grep audit: zero instances of `set_component_workstation_marker` without a co-located `set_component_production_output_ownership_policy` in test code

## Outcome

**Completion date**: 2026-03-16

**What changed** (14 files, +136 -18):
- `golden_harness/mod.rs`: 3 helper functions (`place_workstation_with_source`, `place_exclusive_workstation_with_source`, `place_workstation`) updated with `ownership_policy: ProductionOutputOwner` parameter + `pub use` re-export
- `golden_production.rs`: 13 call sites updated with `ProductionOutputOwner::Actor`
- `golden_social.rs`: 6 call sites updated
- `golden_ai_decisions.rs`: 4 call sites updated
- `golden_trade.rs`: 2 call sites updated
- `golden_combat.rs`: 2 call sites updated
- `golden_determinism.rs`: 1 call site updated
- `production_actions.rs`: `setup_world()` test helper updated with `Actor` policy
- `e10_production_transport_integration.rs`: `Harness::new` orchard + mill updated with `Actor` policy
- `facility_queue.rs`: test `setup_world()` updated with `Actor` policy
- `facility_queue_actions.rs`: test `setup_world()` updated with `Actor` policy
- `perception.rs`: test workstation fixture updated with `Actor` policy
- `production.rs`: `seed_source_on_first_place()` updated with `Unowned` policy (public Place resource)
- `combat.rs`: GravePlot workstation updated with `Unowned` policy

**Deviations from plan**:
- Also fixed 5 non-producing workstation fixtures (facility_queue, facility_queue_actions, perception, production, combat) not listed in the original ticket but required by the AC "every Facility/Place with WorkstationMarker or ResourceSource must have explicit policy"

**Verification**: `cargo test --workspace` (all pass), `cargo clippy --workspace` (clean)
