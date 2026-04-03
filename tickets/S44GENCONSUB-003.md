# S44GENCONSUB-003: P28 migration — replace all facility queue types

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — component removal/replacement across all crates, SystemId rename, scenario system update
**Deps**: S44GENCONSUB-001, S44GENCONSUB-002

## Problem

Per FOUNDATIONS Principle 28 (No Backward Compatibility), the old facility-specific types (`FacilityUseQueue`, `ExclusiveFacilityPolicy`, `FacilityQueueIntents`, `FacilityQueueDispositionProfile`, `QueuedFacilityUse`, `GrantedFacilityUse`, `FacilityQueueError`) must be removed entirely and replaced by the generalized contention types. Two live authoritative representations of the same concept cannot coexist.

## Assumption Reassessment (2026-04-03)

1. Old types defined in `crates/worldwake-core/src/facility_queue.rs` (lines 10-56) and `crates/worldwake-core/src/intention.rs` (FacilityQueueIntents, lines 35-38). Confirmed.
2. Component registrations in `component_schema.rs`: `ExclusiveFacilityPolicy` (lines 934-956), `FacilityUseQueue` (lines 957-981), `FacilityQueueDispositionProfile` (lines 158-181). Confirmed.
3. `SystemId::FacilityQueue` in `system_manifest.rs:57`. Canonical execution order places it after BanditCamp, before Politics. Confirmed.
4. `AgentDef.facility_queue_disposition: Option<FacilityQueueDispositionProfile>` at `scenario/types.rs:104`. Confirmed.
5. `spawn_agent()` applies `FacilityQueueDispositionProfile` at `scenario/mod.rs:1153`. Confirmed.
6. `FacilityQueueIntents` is NOT in the scenario system (runtime-generated, exempt). Confirmed.
7. Downstream consumers span worldwake-core (delta.rs, component_tables.rs, world.rs, lib.rs), worldwake-systems (facility_queue.rs system), worldwake-sim (action pipeline), and worldwake-cli (scenario).
8. Migration defaults for `ExclusiveFacilityPolicy` → `ContentionPolicy`: `{ grant_hold_ticks: <existing>, auto_promote: true, max_waiters: None }`.
9. For component registration, all macro expansion sites (delta.rs, world.rs, component_tables.rs) must import new types and remove old type imports per ticket README check 13.

## Architecture Check

1. Single atomic migration ensures no dead types or dual representations persist. P28 mandates this approach over incremental deprecation.
2. No backward-compatibility shims — old types are deleted, not aliased.

## Verification Layers

1. All old type names absent from codebase → `grep -r FacilityUseQueue` returns zero hits
2. All existing facility queue tests still pass under new type names → `cargo test -p worldwake-core`
3. Scenario system correctly applies `ContentionDispositionProfile` → scenario round-trip test
4. SystemId rename does not break system dispatch → `cargo test -p worldwake-sim system_manifest`
5. Cross-crate ticket — verification covers core (types), systems (queue system), sim (validation), cli (scenario).

## What to Change

### 1. Remove old types from worldwake-core

- Delete `FacilityUseQueue`, `ExclusiveFacilityPolicy`, `QueuedFacilityUse`, `GrantedFacilityUse`, `FacilityQueueError` from `facility_queue.rs`. Either delete the file entirely or keep it as a thin re-export redirect (prefer deletion per P28).
- Delete `FacilityQueueIntents`, `QueuedFacilityIntent` from `intention.rs`.
- Remove old component registrations from `component_schema.rs`.
- Remove old `ComponentValue`/`ComponentKind` variants from `delta.rs`.
- Remove old table entries from `component_tables.rs`.
- Remove old re-exports from `lib.rs`.
- Update `world.rs` imports.

### 2. Rename SystemId

In `system_manifest.rs`: rename `SystemId::FacilityQueue` → `SystemId::Contention`. Update `as_str()` to return `"contention"`. Update canonical execution order. Update all tests.

### 3. Update worldwake-systems consumers

- Rename `facility_queue.rs` → `contention.rs` (or update in place).
- Update `facility_queue_system()` → `contention_system()` signature.
- Replace all `FacilityUseQueue` → `ContentionQueue`, `ExclusiveFacilityPolicy` → `ContentionPolicy`, etc. in function bodies.
- Update module declaration in `lib.rs`.
- Update system dispatch registration.

### 4. Update worldwake-sim consumers

- Update `action_validation.rs` and any file importing old types.
- Update `start_gate.rs` facility queue references.

### 5. Update worldwake-ai consumers

- Update any files importing old types for affordance or planning logic.

### 6. Update worldwake-cli scenario system

- In `scenario/types.rs`: rename `facility_queue_disposition` → `contention_disposition`, change type to `Option<ContentionDispositionProfile>`.
- In `scenario/mod.rs`: update `spawn_agent()` to use `contention_disposition` field and `ContentionDispositionProfile` type.
- Update all `.ron` scenario files that reference `facility_queue_disposition`.

## Files to Touch

- `crates/worldwake-core/src/facility_queue.rs` (delete or gut)
- `crates/worldwake-core/src/intention.rs` (modify — remove FacilityQueueIntents)
- `crates/worldwake-core/src/lib.rs` (modify — remove old re-exports)
- `crates/worldwake-core/src/component_schema.rs` (modify — remove old registrations)
- `crates/worldwake-core/src/delta.rs` (modify — remove old variants)
- `crates/worldwake-core/src/component_tables.rs` (modify — remove old entries)
- `crates/worldwake-core/src/world.rs` (modify — remove old imports)
- `crates/worldwake-sim/src/system_manifest.rs` (modify — rename SystemId)
- `crates/worldwake-systems/src/facility_queue.rs` (rename to contention.rs or modify)
- `crates/worldwake-systems/src/lib.rs` (modify — update module name)
- `crates/worldwake-sim/src/action_validation.rs` (modify — update imports)
- `crates/worldwake-sim/src/start_gate.rs` (modify — update references)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — rename field + type)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — update spawn_agent)
- `scenarios/*.ron` (modify — rename field if referenced)

## Out of Scope

- Generalizing contention_system() logic (S44GENCONSUB-004)
- Adding ContentionStatus to Affordance (S44GENCONSUB-005)
- Adding contention to new domains (S44GENCONSUB-007)

## Acceptance Criteria

### Tests That Must Pass

1. Zero grep hits for `FacilityUseQueue`, `ExclusiveFacilityPolicy`, `FacilityQueueIntents`, `FacilityQueueDispositionProfile` in source files
2. `SystemId::Contention` exists and `SystemId::FacilityQueue` does not
3. Scenario files parse with renamed `contention_disposition` field
4. Existing suite: `cargo test --workspace`

### Invariants

1. No old type names remain in the codebase (P28)
2. Canonical system execution order unchanged in semantics (only name changed)
3. Existing facility queue behavior preserved through generalized types

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/contention.rs` — migrate existing facility_queue.rs tests to use new types
2. `crates/worldwake-sim/src/system_manifest.rs` — update SystemId display and ordering tests
3. `crates/worldwake-systems/src/contention.rs` — update system function tests

### Commands

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
