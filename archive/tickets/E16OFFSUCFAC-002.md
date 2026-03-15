# E16OFFSUCFAC-002: Register OfficeData and FactionData in Component Tables and Schema

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — component_tables macro expansion, component_schema entries
**Deps**: E16OFFSUCFAC-001

## Problem

`OfficeData` and `FactionData` exist as authoritative component types, but they are not registered in the authoritative component pipeline. As a result, office/faction entities cannot yet carry those components through the same typed path used by the rest of the world: schema expansion, typed storage, `World` insert/get/query helpers, `ComponentKind`/`ComponentValue`, and `WorldTxn` set/clear delta recording.

## Assumption Reassessment (2026-03-15)

1. `OfficeData` and `FactionData` already exist in dedicated modules (`offices.rs`, `factions.rs`) and already satisfy component + bincode bounds — confirmed.
2. `component_schema.rs` is the single source of truth for authoritative component registration. Adding entries there fans out into `ComponentTables`, `World` helpers, `WorldTxn` setters, and delta/component enums via macro expansion — confirmed.
3. `ComponentTables` stores authoritative components in deterministic `BTreeMap<EntityId, T>` fields generated from the schema — confirmed.
4. `EntityKind::Office` and `EntityKind::Faction` already exist and are used elsewhere in world/verification logic — confirmed.
5. `OfficeData` and `FactionData` are not currently present in `with_component_schema_entries!`, so the generated `insert_component_office_data`, `get_component_office_data`, `set_component_office_data`, and faction equivalents do not exist yet — confirmed.
6. `verify_completeness()` validates event-log/world consistency and relation invariants, but it is not the primary proof that component registration is correct. Registration must be proven through direct world/txn/component tests — corrected.

## Architecture Check

1. The correct architectural move is to register both components in the existing authoritative schema rather than adding bespoke office/faction storage paths.
2. This keeps component registration centralized and lets `ComponentTables`, `World`, `WorldTxn`, `ComponentKind`, and `ComponentValue` stay derived from one declaration source.
3. No alias APIs, fallback paths, or compatibility shims are acceptable here. If callers need the generated APIs, they should use the new canonical names directly.
4. Factory helpers such as `create_office()` and `create_faction()` should remain lightweight; they should not grow special-case default component attachment in this ticket.

## What to Change

### 1. Register both components in the authoritative schema

In `crates/worldwake-core/src/component_schema.rs`, add schema entries for:

| Field | Type | Kind Predicate |
|-------|------|----------------|
| `office_data` | `OfficeData` | `kind == EntityKind::Office` |
| `faction_data` | `FactionData` | `kind == EntityKind::Faction` |

This follows the same pattern as existing kind-gated authoritative components such as `combat_profiles` and `resource_sources`.

### 2. Wire the generated storage/types cleanly

Update any authoritative-component modules that need concrete type imports once the schema expands, especially `component_tables.rs`.

### 3. Verify the full generated API surface that this registration unlocks

The schema change should auto-generate all needed methods and enum variants. Verify with tests that:
- `ComponentTables` can store and roundtrip both components deterministically
- `world.insert_office_data(entity, data)` works for `Office` entities
- `world.get_office_data(entity)` retrieves it
- `world.query_office_data()` / `count_with_office_data()` behave like other authoritative components
- Same coverage for `FactionData`
- Kind mismatch is rejected (inserting `OfficeData` on an `Agent` entity fails)
- `WorldTxn::set_component_office_data` / `clear_component_office_data` record correct deltas and commit cleanly
- Same coverage for `FactionData`

## Files to Touch

- `crates/worldwake-core/src/component_schema.rs` (modify — add 2 entries to macro)
- `crates/worldwake-core/src/component_tables.rs` (modify — imports if needed)
- `crates/worldwake-core/src/world.rs` (tests via generated API)
- `crates/worldwake-core/src/world_txn.rs` (tests via generated txn setters)

## Out of Scope

- The `OfficeData` and `FactionData` type definitions themselves (E16OFFSUCFAC-001)
- Changing `create_office()` / `create_faction()` to auto-attach these components
- Relation storage for `support_declarations` (E16OFFSUCFAC-003)
- Any action definitions or handlers
- Any system functions
- Any office/faction gameplay semantics beyond authoritative component registration

## Acceptance Criteria

### Tests That Must Pass

1. `ComponentTables` exposes typed insert/get/has/iter methods for `OfficeData` and `FactionData`.
2. `insert_component_office_data` succeeds for an `EntityKind::Office` entity.
3. `insert_component_office_data` is rejected for non-Office entity kinds.
4. `query_office_data()` and `count_with_office_data()` only report live office entities.
5. `insert_component_faction_data` succeeds for an `EntityKind::Faction` entity.
6. `insert_component_faction_data` is rejected for non-Faction entity kinds.
7. `query_faction_data()` and `count_with_faction_data()` only report live faction entities.
8. `WorldTxn` `set_component_*` and `clear_component_*` paths work for both components and emit the expected `ComponentDelta`.
9. `cargo clippy --workspace --all-targets -- -D warnings`
10. `cargo test --workspace`

### Invariants

1. No existing component registrations change behavior.
2. Component storage uses `BTreeMap<EntityId, T>` for determinism.
3. Kind predicates enforce Office-only and Faction-only attachment.
4. Save/load and delta/component serialization paths continue to include the new authoritative components through the shared schema.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/component_tables.rs` — add typed storage and roundtrip coverage for `OfficeData` and `FactionData`.
2. `crates/worldwake-core/src/world.rs` — add world-level roundtrip/query/rejection tests matching the existing component pattern.
3. `crates/worldwake-core/src/world_txn.rs` — add set/clear delta coverage for both components.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-15
- What actually changed: `OfficeData` and `FactionData` were registered in the shared authoritative component schema, which generated canonical support across `ComponentTables`, `World`, `WorldTxn`, `ComponentKind`, and `ComponentValue`. Targeted tests were added for typed storage, world-level kind gating/query behavior, and transaction delta recording/commit behavior.
- Deviations from original plan: the ticket originally framed this as a schema-plus-storage tweak and mentioned `verify_completeness()` as part of proof. In practice the correct scope was the full shared registration fanout, and the meaningful verification came from direct component/world/txn tests rather than `verify_completeness()`.
- Verification results: `cargo test -p worldwake-core`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` all passed.
