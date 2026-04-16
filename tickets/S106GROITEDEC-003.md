# S106GROITEDEC-003: item_decay_system implementation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `item_decay_system` replaces stub with decay logic
**Deps**: archive/tickets/S106GROITEDEC-001.md, archive/tickets/S106GROITEDEC-002.md

## Problem

Ground items accumulate without bound (FND-11 violation). The infrastructure (GroundSince component from 001, SystemId/EventTag/decay map from 002) is in place. This ticket implements the actual decay logic: each tick, check ground items whose elapsed time exceeds their commodity's decay threshold and archive them with event log entries.

## Assumption Reassessment (2026-04-16)

1. `evidence_decay_system` at `crates/worldwake-systems/src/evidence_decay.rs:7-26` provides the structural pattern: `SystemExecutionContext` destructuring, collect-then-apply loop, `WorldTxn::new` with `CauseRef::SystemTick(tick)`, `txn.add_tag(...)`, `txn.commit(event_log)`. The item decay system follows this pattern exactly.
2. `WorldTxn::archive_entity` at `world_txn.rs:429-434` returns `Result<(), WorldError>`. It checks for archive dependencies (world.rs:407-417). Ground items without possessors or containers should have no blocking dependencies, but the system must handle errors gracefully.
3. `ItemLot` struct at `items.rs:317-321` uses field `commodity: CommodityKind` (not `commodity_kind`). The query method is `world.query_ground_since()` for iteration and `world.get_component_item_lot(entity)` for lookup.
4. `Tick(pub u64)` at `ids.rs:55-77`. Decay threshold is `NonZeroU32`. Comparison requires `u64::from(decay_ticks.get())` for type widening.
5. The stub `item_decay_system` from ticket 002 exists in `crates/worldwake-systems/src/item_decay.rs` — this ticket replaces the stub body.

## Architecture Check

1. Following the `evidence_decay_system` pattern ensures consistency across cleanup systems. Both collect candidates in a read pass, then apply mutations in a write pass — avoiding iterator invalidation.
2. No backward-compatibility shims. The stub is replaced entirely.

## Verification Layers

1. Items decay at correct tick boundary → focused unit test (place at tick 10, decay threshold 50, check at tick 59 vs 60)
2. Items without decay map entry never decay → focused unit test (Sword with no entry survives indefinitely)
3. Archive dependencies handled gracefully → focused unit test (item with archive dependency is skipped, not panicked)
4. Event tags on decay events → focused unit test (verify `EventTag::ItemDecay` and `EventTag::WorldMutation`)
5. Archived items excluded from query_ground_since → authoritative world state (archived entities not returned by active queries)

## What to Change

### 1. Replace stub with real implementation

In `crates/worldwake-systems/src/item_decay.rs`, replace the stub body with the implementation from the spec's pseudocode:

- Destructure `SystemExecutionContext` to get `world`, `event_log`, `tick`
- Read the `CommodityDecayMap` from world config (however 002 stored it)
- Collect phase: iterate `world.query_ground_since()`, for each entity check `get_component_item_lot`, look up commodity in decay map, compute elapsed time, collect entities exceeding threshold
- Apply phase: for each collected entity, create `WorldTxn` with `CauseRef::SystemTick(tick)`, add `EventTag::ItemDecay` and `EventTag::WorldMutation` tags, call `txn.archive_entity(entity)`. On error, skip (continue). On success, `txn.commit(event_log)`.

### 2. Unit tests

Add tests in `crates/worldwake-systems/src/item_decay.rs` (test module):

- `waste_decays_at_threshold_tick`: Create Waste on ground at tick 10, decay threshold 50. Run system at tick 59 → survives. Run at tick 60 → archived.
- `multi_commodity_selective_decay`: Create Waste (threshold 50), Apple (threshold 100), Sword (no entry) all at tick 10. Run at tick 60 → only Waste archived. Run at tick 110 → Apple also archived. Sword survives.
- `no_decay_for_missing_commodity`: Create Water on ground (no decay entry). Run system at tick 10000 → still alive.
- `archive_dependency_skips_gracefully`: If possible to create an item with an archive dependency on ground, verify the system skips it without error.
- `decay_event_has_correct_tags`: Verify decayed item event has both `EventTag::ItemDecay` and `EventTag::WorldMutation`.
- `dispatch_table_routes_item_decay`: Verify `dispatch_table().get(SystemId::ItemDecay)` executes the real system (update from 002's stub test if needed).

## Files to Touch

- `crates/worldwake-systems/src/item_decay.rs` (modify — replace stub with implementation + tests)

## Out of Scope

- GroundSince component registration and lifecycle (ticket 001)
- SystemId, EventTag, decay map infrastructure (ticket 002)
- Golden E2E tests and conservation tests (ticket 004)
- Decay for carried or stored items (spec non-goal)
- Multi-stage decomposition chains (spec non-goal)

## Acceptance Criteria

### Tests That Must Pass

1. `waste_decays_at_threshold_tick` — boundary-tick precision
2. `multi_commodity_selective_decay` — per-commodity independence
3. `no_decay_for_missing_commodity` — opt-in decay only
4. `decay_event_has_correct_tags` — traceability (FND-4)
5. Existing suite: `cargo test -p worldwake-systems` — all tests pass

### Invariants

1. Items not in the `CommodityDecayMap` never decay (opt-in only).
2. Archived items produce event log entries with `EventTag::ItemDecay` and `EventTag::WorldMutation`.
3. `archive_entity` errors are handled gracefully (skip, no panic).
4. The system reads only `GroundSince`, `ItemLot`, and the decay map — no cross-system calls (FND-26).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/item_decay.rs` (test module) — 5-6 focused unit tests as described above

### Commands

1. `cargo test -p worldwake-systems item_decay` — targeted tests
2. `cargo test -p worldwake-systems` — full crate suite
3. `cargo clippy --workspace --all-targets -- -D warnings` — lint
