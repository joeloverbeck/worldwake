# S44GENCONSUB-002: Agent-side contention types

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new ECS components in worldwake-core
**Deps**: S44GENCONSUB-001

## Problem

The contention substrate needs per-agent tracking of which entities an agent is contending for (`ContentionIntents`) and per-agent patience configuration (`ContentionDispositionProfile`). These generalize `FacilityQueueIntents` and `FacilityQueueDispositionProfile` respectively.

## Assumption Reassessment (2026-04-03)

Direct correction applied during implementation:
- ticket says: `Deps: None`
- live code has: `crates/worldwake-core/src/contention.rs` already created and registered by archived `S44GENCONSUB-001`, and this ticket's `What to Change` explicitly extends that file and those registrations
- correction applied: `Deps` updated to `S44GENCONSUB-001`
- why safe: this is a factual prerequisite correction, not a scope change

1. `FacilityQueueIntents` exists at `crates/worldwake-core/src/intention.rs:35-38` with `intents: BTreeMap<EntityId, QueuedFacilityIntent>`. Confirmed.
2. `FacilityQueueDispositionProfile` exists at `crates/worldwake-core/src/facility_queue.rs:17-21` with `queue_patience_ticks: Option<NonZeroU32>`. Confirmed.
3. Both implement `Component`. Confirmed.
4. `ContentionDispositionProfile` is role-specific per spec-drafting-rules section 5 — applied via `if let Some(...)` in `spawn_agent()`. Scenario wiring deferred to S44GENCONSUB-003.
5. `ContentionIntents` is runtime-generated state (like `ActiveGoal`) — exempt from scenario profile contract.
6. No existing types named `ContentionIntents` or `ContentionDispositionProfile` in codebase. Confirmed.
7. This ticket adds new types only — old types coexist until S44GENCONSUB-003.

## Architecture Check

1. New types mirror proven patterns from existing `FacilityQueueIntents` and `FacilityQueueDispositionProfile`. Structural rename with no semantic change.
2. No backward-compatibility shims — new types stand alone.

## Verification Layers

1. Component trait satisfaction → compile-time bounds tests
2. Serialization → bincode round-trip
3. Single-layer ticket (worldwake-core types only).

## What to Change

### 1. Add agent-side types to contention module

Add to `crates/worldwake-core/src/contention.rs` (created in S44GENCONSUB-001):
- `ContentionIntents` struct with `intents: BTreeMap<EntityId, QueuedContentionIntent>`, derives Default
- `QueuedContentionIntent` struct (mirroring `QueuedFacilityIntent` fields)
- `ContentionDispositionProfile` struct with `queue_patience_ticks: Option<NonZeroU32>`
- `impl Component` for both
- Unit tests: bounds, serialization

### 2. Register components

Add `ContentionIntents` and `ContentionDispositionProfile` to component_schema.rs, delta.rs, component_tables.rs, world.rs. Entity kind constraint for both: `|kind| kind == EntityKind::Agent`.

### 3. Re-export from lib.rs

Add re-exports for `ContentionIntents`, `QueuedContentionIntent`, `ContentionDispositionProfile`.

## Files to Touch

- `crates/worldwake-core/src/contention.rs` (modify — add types)
- `crates/worldwake-core/src/lib.rs` (modify — add re-exports)
- `crates/worldwake-core/src/component_schema.rs` (modify — register)
- `crates/worldwake-core/src/delta.rs` (modify — add variants)
- `crates/worldwake-core/src/component_tables.rs` (modify — add entries + imports)
- `crates/worldwake-core/src/world.rs` (modify — add imports)

## Out of Scope

- Removing old `FacilityQueueIntents` / `FacilityQueueDispositionProfile` (S44GENCONSUB-003)
- Scenario system wiring (S44GENCONSUB-003)
- Contention system logic (S44GENCONSUB-004)

## Acceptance Criteria

### Tests That Must Pass

1. `ContentionIntents` and `ContentionDispositionProfile` satisfy Component bounds
2. Bincode round-trip for all new types
3. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `ContentionIntents` uses `BTreeMap` for deterministic iteration
2. No old facility queue types are modified or removed

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/contention.rs` (inline tests) — bounds and serialization for agent-side types

### Commands

1. `cargo test -p worldwake-core contention`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

Completed: 2026-04-04

What changed:
- Extended `crates/worldwake-core/src/contention.rs` with the new agent-side generalized contention types: `ContentionIntents`, `QueuedContentionIntent`, and `ContentionDispositionProfile`.
- Registered `ContentionIntents` and `ContentionDispositionProfile` across the authoritative core schema/materialization surfaces in `component_schema.rs`, `delta.rs`, `component_tables.rs`, and `world.rs`.
- Re-exported the new types from `crates/worldwake-core/src/lib.rs`.
- Added representative fixture support in `crates/worldwake-core/src/test_utils.rs`.
- Added focused tests covering component/value bounds, bincode round-trips, and component-table insert/get/remove cycles for the new agent-side contention state.

Deviations from original plan:
- No production-scope deviation. The only factual ticket correction was updating `Deps` from `None` to `S44GENCONSUB-001`, because this ticket's own `What to Change` already depended on the `contention.rs` substrate created by the earlier archived ticket.
- Verification was run as separate sequential commands rather than one combined shell line so each proof surface and CI gate result stayed explicit.

Verification results:
- `cargo test -p worldwake-core contention`
- `cargo test -p worldwake-core`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
