# E14PERBEL-003: Register AgentBeliefStore and PerceptionProfile in ComponentTables

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — component registration in ECS layer
**Deps**: E14PERBEL-002 (types must exist before registration)

## Problem

`AgentBeliefStore` and `PerceptionProfile` exist as authoritative belief-domain types in `worldwake-core`, but they are not yet registered in the ECS component schema. Until they are registered, no generated `World` / `WorldTxn` storage API exists for them, so later E14 work cannot attach or mutate them on agent entities.

## Assumption Reassessment (2026-03-14)

1. Component registration is driven by `with_component_schema_entries!` in `crates/worldwake-core/src/component_schema.rs` — confirmed.
2. That macro fan-out is the authoritative source for `ComponentTables`, `World` component APIs, `WorldTxn` set/clear APIs, and `ComponentKind` / `ComponentValue` variants — confirmed across `component_tables.rs`, `world.rs`, `world_txn.rs`, and `delta.rs`.
3. Both components are `EntityKind::Agent`-only — confirmed by the E14 spec and by the existing schema pattern for agent-only components.
4. `canonical.rs` does not enumerate component types manually. World hashing is generic `Serialize` over `World`, so schema registration is sufficient for canonical inclusion.
5. `crates/worldwake-core/src/lib.rs` already re-exports `AgentBeliefStore` and `PerceptionProfile` from `belief.rs`; no export work is needed here.
6. Existing component regression coverage lives in `world.rs` and `world_txn.rs`, not in `component_schema.rs`. This ticket should extend those existing test patterns instead of introducing a new dedicated schema test module.

## Architecture Check

1. Registering these components in the existing macro-based ECS path is the clean architectural move. It keeps belief state inside the same deterministic authoritative storage model as the rest of the world rather than creating a parallel subsystem.
2. EntityKind guards must restrict these components to agents only. Belief memory and perception parameters are per-agent state; allowing them on other entity kinds would weaken invariants and create future cleanup work.
3. The proposed change is better than the current architecture because the current state leaves the belief/perception types as inert domain structs. Registration makes them first-class authoritative world state without wrappers, aliases, or compatibility shims.
4. No broader rewrite is justified in this ticket. Default attachment on agent creation belongs to the later migration/system tickets that actually consume these components.

## What to Change

### 1. Add entries to `component_schema.rs`

Add two new entries to the `with_component_schema_entries!` macro following the existing agent-only pattern:

- `AgentBeliefStore` — field name `agent_belief_stores`, registered for `EntityKind::Agent`
- `PerceptionProfile` — field name `perception_profiles`, registered for `EntityKind::Agent`

Each entry must define:

- Field name and type
- Table methods (`insert`, `get`, `get_mut`, `remove`, `has`, `iter`)
- `World` methods
- `WorldTxn` methods (`set`, `clear`, `has`, `entities`, `query`, `count`)
- Display name string
- EntityKind predicate: `|kind| kind == EntityKind::Agent`
- Component variant name used by generated `ComponentKind` / `ComponentValue`

### 2. Add focused regression tests

Add or extend tests in the existing core test locations to prove the generated API works end to end:

- `World` insert/get/remove/count/query coverage for both new components
- non-agent rejection coverage for both new components
- `WorldTxn` set/clear delta coverage for both new components

Do not add manual enum wiring or canonical hashing code unless inspection shows the macro path is insufficient.

## Files to Touch

- `crates/worldwake-core/src/component_schema.rs` (modify — add 2 component entries)
- `crates/worldwake-core/src/component_tables.rs` (modify only if imports/tests need the new component types)
- `crates/worldwake-core/src/world.rs` (modify — imports/tests for generated `World` API)
- `crates/worldwake-core/src/world_txn.rs` (modify — imports/tests for generated `WorldTxn` API)
- `crates/worldwake-core/src/delta.rs` (modify only if imports/tests need the new component types for generated enums)

## Out of Scope

- Defining the types themselves (done in E14PERBEL-002)
- Implementing `PerAgentBeliefView` (E14PERBEL-004)
- Implementing `perception_system()` (E14PERBEL-005)
- Adding `AgentBeliefStore` or `PerceptionProfile` to existing agent creation code (belongs to the later migration/system tickets)
- Modifying `BeliefView` trait
- Introducing a separate belief storage subsystem outside `ComponentTables`
- Changing any existing component registrations beyond the two new entries

## Acceptance Criteria

### Tests That Must Pass

1. `World` exposes generated component APIs for both `AgentBeliefStore` and `PerceptionProfile`
2. Inserting either component on an agent succeeds; querying and removal roundtrip correctly
3. Inserting either component on a non-agent fails with the existing EntityKind-guarded invalid-operation path
4. `WorldTxn` set/clear flows emit the correct `ComponentDelta` / `ComponentKind` / `ComponentValue` variants for both components and commit correctly
5. Query/count helpers include the new components and stay deterministic
6. `cargo test -p worldwake-core` — all existing tests pass
7. `cargo clippy --workspace`
8. `cargo test --workspace`

### Invariants

1. Components are only attachable to `EntityKind::Agent` entities
2. Component tables remain deterministic (`BTreeMap`-backed)
3. Canonical hashing continues to include all registered components through `World` serialization, with no bespoke hashing path added
4. Existing component registrations remain unchanged aside from the two new entries

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` — add `World` roundtrip, non-agent guard, query, and count coverage for `AgentBeliefStore` and `PerceptionProfile`
2. `crates/worldwake-core/src/world_txn.rs` — add set/clear delta coverage for both components

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed: registered `AgentBeliefStore` and `PerceptionProfile` in both authoritative component-schema macro expansion paths so `ComponentTables`, `World`, `WorldTxn`, `ComponentKind`, and `ComponentValue` all generate the expected APIs and variants; added/updated regression coverage in `world.rs`, `world_txn.rs`, `component_tables.rs`, `delta.rs`, and updated the downstream schema inventory assertion in `crates/worldwake-systems/tests/e09_needs_integration.rs`.
- Deviations from original plan: no `canonical.rs` or `lib.rs` changes were needed because hashing already flows through generic `World` serialization and the belief types were already re-exported; no manual enum wiring was needed because `ComponentKind` / `ComponentValue` are macro-derived.
- Verification results: `cargo test -p worldwake-core` passed; `cargo clippy --workspace` passed; `cargo test --workspace` passed.
