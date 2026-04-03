# S44GENCONSUB-001: Core contention types + queue operations

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new ECS components in worldwake-core
**Deps**: None

## Problem

The codebase has only one contention mechanism (`FacilityUseQueue`) locked to `EntityKind::Facility`. FOUNDATIONS P8 and P9 require all contested affordances to resolve through explicit world processes. A domain-agnostic contention substrate is needed as the foundation for generalizing contention across entity kinds.

## Assumption Reassessment (2026-04-03)

1. `FacilityUseQueue` exists at `crates/worldwake-core/src/facility_queue.rs:25-30` with fields `next_ordinal: u32`, `waiting: BTreeMap<u32, QueuedFacilityUse>`, `granted: Option<GrantedFacilityUse>`. Confirmed via session grep.
2. `ExclusiveFacilityPolicy` at same file, line 10, has only `grant_hold_ticks: NonZeroU32`. Confirmed.
3. Queue methods (enqueue, position_of, has_actor, remove_actor, promote_head, clear_grant, grant_expired) all exist on `FacilityUseQueue` impl block lines 58-152. Confirmed.
4. `FacilityQueueError` at lines 52-56 has `DuplicateActor(EntityId)` and `OrdinalOverflow`. Confirmed.
5. Component trait requires `'static + Send + Sync + Clone + Debug + Serialize + DeserializeOwned` per `traits.rs:15`. Confirmed.
6. `component_schema.rs` uses entity kind constraints; `FacilityUseQueue` is constrained to `|kind| kind == EntityKind::Facility`. New contention types need `|kind| kind == EntityKind::Agent || kind == EntityKind::Facility`.
7. No existing `ContentionQueue` or `ContentionPolicy` types in the codebase (would conflict). Confirmed via session grep.
8. This ticket adds new types only — old types coexist until S44GENCONSUB-003 removes them per P28.

## Architecture Check

1. New types mirror proven `FacilityUseQueue` API surface but are domain-agnostic. The `ContentionPolicy` adds `auto_promote` and `max_waiters` fields enabling both queue and race semantics through configuration rather than separate mechanisms.
2. No backward-compatibility shims — new types stand alone. Old types are untouched in this ticket and removed atomically in S44GENCONSUB-003.

## Verification Layers

1. Queue enqueue/position/promote semantics → focused unit tests on `ContentionQueue` methods
2. Component trait satisfaction → compile-time `assert_component_bounds` tests
3. Serialization round-trip → bincode serialize/deserialize unit tests
4. Error variants (DuplicateActor, OrdinalOverflow, QueueFull) → focused unit tests
5. Single-layer ticket (worldwake-core types only) — no cross-system verification needed.

## What to Change

### 1. Add contention types module

Create `crates/worldwake-core/src/contention.rs` with:
- `ContentionQueue` struct (next_ordinal, waiting, granted)
- `ContentionWaiter` struct (actor, intended_action, queued_at)
- `ContentionGrant` struct (actor, intended_action, granted_at, expires_at)
- `ContentionPolicy` struct (grant_hold_ticks, auto_promote, max_waiters)
- `ContentionError` enum (DuplicateActor, OrdinalOverflow, QueueFull)
- All derive macros matching Component trait bounds
- `impl Component` for `ContentionQueue` and `ContentionPolicy`
- Full method surface on `ContentionQueue`: enqueue (with max_waiters enforcement), position_of, has_actor, remove_actor, promote_head, clear_grant, grant_expired
- Unit tests: enqueue ordering, duplicate rejection, position tracking, promote semantics, grant expiry, QueueFull enforcement, bincode round-trip, component bounds

### 2. Register in module tree

Add `pub mod contention;` to `crates/worldwake-core/src/lib.rs` and re-export all public types.

### 3. Register components in component_schema.rs

Add `ContentionQueue` and `ContentionPolicy` entries to `with_component_schema_entries!` macro with entity kind constraint `|kind| kind == EntityKind::Agent || kind == EntityKind::Facility`.

### 4. Register in delta.rs, component_tables.rs, world.rs

Add `ComponentValue` variants, `ComponentKind` variants, table entries, and import statements for the new types at all macro expansion sites.

## Files to Touch

- `crates/worldwake-core/src/contention.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)
- `crates/worldwake-core/src/component_schema.rs` (modify — register new components)
- `crates/worldwake-core/src/delta.rs` (modify — add ComponentValue/ComponentKind variants)
- `crates/worldwake-core/src/component_tables.rs` (modify — add table entries + imports)
- `crates/worldwake-core/src/world.rs` (modify — add imports)

## Out of Scope

- Removing old `FacilityUseQueue` types (S44GENCONSUB-003)
- Agent-side contention types (S44GENCONSUB-002)
- Contention system generalization (S44GENCONSUB-004)
- Affordance annotation (S44GENCONSUB-005)

## Acceptance Criteria

### Tests That Must Pass

1. `ContentionQueue::enqueue` returns incrementing ordinals and rejects duplicates
2. `ContentionQueue::promote_head` moves head to granted with correct expiry
3. `ContentionQueue::enqueue` returns `QueueFull` when `max_waiters` limit reached
4. `ContentionQueue::enqueue` returns `QueueFull` immediately in race mode (`max_waiters: Some(0)`) when grant exists
5. All types satisfy `Component` trait bounds
6. Bincode round-trip for all contention types
7. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `ContentionQueue` methods preserve BTreeMap ordering (determinism)
2. `ContentionPolicy` with `max_waiters: Some(0)` enforces race semantics — no waiters, only grant-or-reject
3. No old facility queue types are modified or removed

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/contention.rs` (inline `#[cfg(test)]` module) — unit tests for queue operations, error cases, serialization, component bounds

### Commands

1. `cargo test -p worldwake-core contention`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

Completed: 2026-04-03

What changed:
- Added the new authoritative generalized contention substrate in `crates/worldwake-core/src/contention.rs`: `ContentionQueue`, `ContentionWaiter`, `ContentionGrant`, `ContentionPolicy`, and `ContentionError`.
- Implemented the full queue method surface on `ContentionQueue`, including `max_waiters` enforcement and race-mode `QueueFull` rejection.
- Registered `ContentionQueue` and `ContentionPolicy` as authoritative components for `EntityKind::Agent` and `EntityKind::Facility` in the core schema/materialization surfaces.
- Re-exported the new contention types from `worldwake-core`.
- Added focused core tests for queue semantics, trait bounds, serialization, and component-table registration.

Deviations from original plan:
- No architectural scope deviation. The only implementation fallout beyond the ticket prose was one existing `delta.rs` authoritative component-order inventory test that needed to be updated to include the newly registered component kinds in schema order.
- Verification expanded slightly beyond the ticket's single-line combined command and was run as separate sequential commands to match current repo guidance and CI shape.

Verification results:
- `cargo test -p worldwake-core contention`
- `cargo test -p worldwake-core`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
