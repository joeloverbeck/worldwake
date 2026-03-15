# E16OFFSUCFAC-002: Register OfficeData and FactionData in Component Tables and Schema

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — component_tables macro expansion, component_schema entries
**Deps**: E16OFFSUCFAC-001

## Problem

`OfficeData` and `FactionData` types exist (from ticket 001) but cannot be attached to entities because they are not registered in the component storage system. The `with_component_schema_entries!` macro must include them, and `ComponentTables` must have typed storage fields, following the exact pattern used by `CombatProfile`, `UtilityProfile`, etc.

## Assumption Reassessment (2026-03-15)

1. `component_tables.rs` uses a macro-generated `ComponentTables` struct with `BTreeMap<EntityId, T>` fields for each component — confirmed.
2. `component_schema.rs` uses `with_component_schema_entries!` to declare which entity kinds accept which components — confirmed.
3. The macro generates insert/get/get_mut/remove/has/iter methods plus `WorldTxn` staging — confirmed.
4. `EntityKind::Office` and `EntityKind::Faction` already exist as valid kind values for the predicate lambdas — confirmed.
5. The `World` struct exposes component operations through the tables — confirmed.

## Architecture Check

1. Following the exact existing macro pattern ensures all generated methods (insert, get, get_mut, remove, has, iter, txn staging) work automatically.
2. No custom code needed — the macro does all the work.
3. No backward-compatibility shims.

## What to Change

### 1. Add storage fields to `ComponentTables` via `with_component_schema_entries!`

In `crates/worldwake-core/src/component_schema.rs`, add two new entries:

| Field | Type | Kind Predicate |
|-------|------|----------------|
| `office_data` | `OfficeData` | `kind == EntityKind::Office` |
| `faction_data` | `FactionData` | `kind == EntityKind::Faction` |

This follows the same pattern as e.g. `combat_profiles` -> `CombatProfile` -> `kind == EntityKind::Agent`.

### 2. Add necessary imports

Ensure `OfficeData` and `FactionData` are imported in the component schema/tables modules.

### 3. Verify `World` and `WorldTxn` operations work

The macro should auto-generate all needed methods. Verify with tests that:
- `world.insert_office_data(entity, data)` works for `Office` entities
- `world.get_office_data(entity)` retrieves it
- Same for `faction_data`
- Kind mismatch is rejected (inserting `OfficeData` on an `Agent` entity fails)

## Files to Touch

- `crates/worldwake-core/src/component_schema.rs` (modify — add 2 entries to macro)
- `crates/worldwake-core/src/component_tables.rs` (modify — imports if needed)

## Out of Scope

- The `OfficeData` and `FactionData` type definitions themselves (E16OFFSUCFAC-001)
- Relation storage for `support_declarations` (E16OFFSUCFAC-003)
- Any action definitions or handlers
- Any system functions
- Modifying existing component registrations

## Acceptance Criteria

### Tests That Must Pass

1. `insert_office_data` succeeds for an `EntityKind::Office` entity.
2. `get_office_data` retrieves the inserted component.
3. `insert_office_data` is rejected for non-Office entity kinds.
4. `insert_faction_data` succeeds for an `EntityKind::Faction` entity.
5. `get_faction_data` retrieves the inserted component.
6. `insert_faction_data` is rejected for non-Faction entity kinds.
7. `WorldTxn` staging works for both components (stage insert, commit, verify present).
8. `verify_completeness()` still passes.
9. `cargo clippy --workspace --all-targets -- -D warnings`
10. `cargo test --workspace`

### Invariants

1. No existing component registrations change behavior.
2. Component storage uses `BTreeMap<EntityId, T>` for determinism.
3. Kind predicates enforce Office-only and Faction-only attachment.
4. Save/load roundtrip preserves new components (bincode serialization via existing derives).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/component_tables.rs` or `component_schema.rs` — add insert/get/reject tests for `OfficeData` and `FactionData` matching the existing pattern for `CombatProfile`.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
