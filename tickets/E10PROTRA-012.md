# E10PROTRA-012: E10 integration tests — conservation, no-teleportation, no-infinite-harvest

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — tests only
**Deps**: E10PROTRA-007 through E10PROTRA-011 (all action tickets must be complete)

## Problem

The E10 spec defines several cross-cutting invariants that no single unit test can verify. These integration tests validate that the production and transport systems work together correctly and that no combination of actions violates the foundational invariants.

## Assumption Reassessment (2026-03-10)

1. All E10 components and actions will exist after tickets 001-011 — confirmed.
2. `verify_conservation()` exists in `worldwake-core/src/conservation.rs` — it can be used to verify commodity conservation across the full world state.
3. The deterministic replay system exists — integration tests can run multi-tick scenarios and verify replay consistency.
4. `build_prototype_world()` exists in topology — it can be extended or supplemented with production/transport test worlds.

## Architecture Check

1. Integration tests live in `crates/worldwake-systems/tests/` (integration test directory) or in a dedicated test module.
2. Each test sets up a minimal world with specific production/transport configuration and runs through multiple ticks.
3. Tests verify invariants after each tick, not just at the end.
4. These tests complement the per-action unit tests — they test emergent behavior from action combinations.

## What to Change

### 1. Integration test module

Create `crates/worldwake-systems/tests/e10_integration.rs` (or add to existing integration test file).

### 2. Test scenarios

Each scenario sets up world state, runs actions through the tick loop, and verifies invariants.

## Files to Touch

- `crates/worldwake-systems/tests/e10_integration.rs` (new)

## Out of Scope

- Unit tests for individual actions (covered in E10PROTRA-007 through E10PROTRA-011)
- AI-driven scenarios (E13)
- Trade scenarios (E11)
- Combat encounters during travel (E12)
- Performance benchmarks
- Soak tests (E22)

## Acceptance Criteria

### Tests That Must Pass

1. **No production path creates infinite goods from a tag alone**: set up a ResourceSource with limited stock, harvest until empty, verify harvest fails when empty, verify total goods created = total stock consumed.
2. **No teleportation path moves goods without a carrier or explicit container chain**: attempt to move goods between two places — verify goods only arrive at destination after an agent carries them through a travel action.
3. **Conservation across harvest→carry→travel→deliver**: harvest goods, pick up, travel, put down — verify `total_commodity_quantity` is unchanged throughout.
4. **Conservation across craft (inputs→outputs)**: craft an item, verify input commodities consumed + output commodities created match recipe exactly.
5. **Interrupted craft leaves WIP**: start craft, interrupt mid-way, verify `ProductionJob` persists, verify staged inputs still exist in staged container.
6. **Resource regeneration + harvest cycle**: deplete a source, wait for regeneration ticks, verify source regenerates, harvest again, verify conservation.
7. **Concurrent workstation access**: two agents at same place with one workstation — first reserves and harvests, second's harvest attempt fails due to occupied workstation.
8. **Carried items travel with carrier**: agent picks up items, travels, verify items are at destination after arrival.
9. **Route occupancy is concrete**: agent starts travel, check `InTransitOnEdge` exists during transit, check it is removed on arrival, check agent is NOT at origin during transit.
10. **Multi-step transport**: harvest at source → pick up → travel edge 1 → travel edge 2 → put down at destination. Verify conservation at every step.
11. Existing suite: `cargo test --workspace`

### Invariants

1. `verify_conservation()` passes after every tick in every scenario.
2. No entity exists at two places simultaneously.
3. No entity is both `LocatedIn` a place and `InTransitOnEdge` simultaneously.
4. All events have causal linkage.
5. Replay determinism holds for all integration scenarios.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/tests/e10_integration.rs` — all 10 scenarios above

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
