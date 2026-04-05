# S44GENCONSUB-003: Remove facility-only contention types after generalized substrate lands

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — component removal/replacement across all crates, SystemId rename, scenario system update
**Deps**: S44GENCONSUB-001, S44GENCONSUB-002

## Problem

Per the repo no-backward-compatibility rule in `AGENTS.md` and the S44 spec’s generalized-contention contract, the old facility-specific contention types (`FacilityUseQueue`, `ExclusiveFacilityPolicy`, `FacilityQueueIntents`, `FacilityQueueDispositionProfile`, `QueuedFacilityUse`, `GrantedFacilityUse`, `FacilityQueueError`) must be removed entirely and replaced by the generalized contention types. Two live authoritative representations of the same concept cannot coexist.

## Assumption Reassessment (2026-04-04)

1. The old facility-only types still exist in live core at `crates/worldwake-core/src/facility_queue.rs` and `crates/worldwake-core/src/intention.rs`, while the generalized replacements already exist in `crates/worldwake-core/src/contention.rs`. Confirmed.
2. The old component registrations also still exist in `component_schema.rs`, `delta.rs`, `component_tables.rs`, and `world.rs`. This ticket must delete those duplicate authoritative surfaces, not just stop using them at call sites. Confirmed.
3. `SystemId::FacilityQueue` still exists in `system_manifest.rs`, and the systems dispatch table still routes through `facility_queue_system()`. The rename to `SystemId::Contention` is in-scope and semantically load-bearing because canonical ordering must remain unchanged apart from the identifier rename. Confirmed.
4. The scenario schema still exposes `AgentDef.facility_queue_disposition`, and `spawn_agent()` still applies the old profile component. Confirmed.
5. `FacilityQueueIntents` is runtime-generated state and remains exempt from the scenario contract; only the type/component name changes here. Confirmed.
6. The live downstream migration surface is broader than the original ticket implied in one place and narrower in another:
   - broader: AI planning/runtime tests, planning snapshot/state helpers, golden harnesses, and same-crate CLI constructor sites all still mention the old types
   - narrower: `worldwake-sim` does not currently have old-type fallout in `action_validation.rs` or `start_gate.rs`; the real sim migration surface is `system_manifest.rs` plus belief-view helpers that expose queue/grant state
7. Facility semantics that previously used `ExclusiveFacilityPolicy` should migrate to `ContentionPolicy { grant_hold_ticks: <existing>, auto_promote: true, max_waiters: None }` so the facility queue behavior remains unchanged in this removal step.

## Architecture Check

1. Single atomic migration ensures no dead types or dual representations persist. The repo no-backward-compatibility rule mandates this approach over incremental deprecation.
2. No backward-compatibility shims — old types are deleted, not aliased.

## Verification Layers

1. All old type names absent from codebase → `grep -r FacilityUseQueue` returns zero hits
2. All existing facility queue tests still pass under new type names → `cargo test -p worldwake-core`
3. Scenario system correctly applies `ContentionDispositionProfile` → scenario round-trip test
4. SystemId rename does not break system dispatch → `cargo test -p worldwake-sim system_manifest`
5. Cross-crate ticket — verification covers core (types/registrations), systems (contention queue system + facility consumers), sim (system manifest + belief-view surface), AI (queue/grant runtime/planning fallout), and cli (scenario rename + constructor fallout).

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

- Rename `SystemId::FacilityQueue` → `SystemId::Contention` in `system_manifest.rs`.
- Update belief-view helper imports and return types that still mention the old grant/profile types.

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
- `crates/worldwake-sim/src/belief_view.rs` (modify — update grant/profile types)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — update grant/profile types)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — update grant/profile types)
- `crates/worldwake-ai/src/planning_state.rs` (modify — update grant/profile types)
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

1. No old type names remain in the codebase
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

## Outcome

Completed: 2026-04-04

What changed:
- Removed the old facility-only contention substrate entirely by deleting `crates/worldwake-core/src/facility_queue.rs` and removing the remaining facility-only contention types from `intention.rs`, core exports, component registration, delta/component-table surfaces, and world imports.
- Kept `crates/worldwake-core/src/contention.rs` as the sole authoritative contention substrate and migrated downstream consumers across `worldwake-sim`, `worldwake-systems`, `worldwake-ai`, and `worldwake-cli` to the generalized contention names and component accessors.
- Renamed `SystemId::FacilityQueue` to `SystemId::Contention`, updated the stable system string to `"contention"`, and preserved canonical ordering semantics.
- Renamed the scenario field `facility_queue_disposition` to `contention_disposition` and updated scenario spawn plus same-crate CLI constructor/test fallout accordingly.

Deviations from original plan:
- The real downstream migration surface was broader than the original ticket claimed in AI/golden-harness and same-crate CLI constructor sites, and narrower in `worldwake-sim`: the relevant sim fallout was `system_manifest.rs` plus belief-view surfaces rather than `action_validation.rs` or `start_gate.rs`.
- The systems implementation was migrated in place in `crates/worldwake-systems/src/facility_queue.rs` rather than renaming the file in this ticket; the functional/system identity still moved to generalized contention names.
- Workspace verification exposed one additional owned cleanup in `crates/worldwake-core/src/delta.rs`: duplicate `ContentionPolicy` / `ContentionQueue` sample entries had to be removed so `component_value_reports_matching_component_kind` matched the live schema inventory.

Verification results:
- `cargo test -p worldwake-core contention`
- `cargo test -p worldwake-sim system_manifest`
- `cargo test -p worldwake-systems`
- `cargo test -p worldwake-cli scenario`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
