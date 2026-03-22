# E16CINSBELRECCON-002: Wire `RecordData` Into Core ECS + World/Txn Creation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `EntityKind`, component-schema registration, delta plumbing, world/txn creation helpers
**Deps**: `specs/E16c-institutional-beliefs-and-record-consultation.md`

## Problem

`RecordData` and the typed institutional record model already exist in `crates/worldwake-core/src/institutional.rs`, but records are still not first-class authoritative entities in the ECS/runtime surface:

- `EntityKind` has no `Record` variant, so records cannot be classified as durable world entities.
- `RecordData` is not registered in the component schema/tables, so it cannot participate in the normal typed component APIs.
- `ComponentKind` / `ComponentValue` do not include `RecordData`, so record mutations cannot flow through event-log component deltas.
- `World` / `WorldTxn` do not expose record creation helpers, which would force callers toward ad hoc `create_entity + insert_component` sequences instead of the standard creation path.

This is the remaining core-plumbing gap between the E16c record architecture and the live authoritative engine.

## Assumption Reassessment (2026-03-21)

1. `RecordData`, `RecordKind`, `RecordEntryId`, `InstitutionalRecordEntry`, `InstitutionalClaim`, `InstitutionalBeliefKey`, and related tests already exist in [crates/worldwake-core/src/institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs). The ticket no longer needs to create those types; it needs to integrate them.
2. `EntityKind` currently has 10 variants in [crates/worldwake-core/src/entity.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/entity.rs), and `ALL_ENTITY_KINDS` still reflects that 10-variant list. This is the live enum surface that must change.
3. `component_schema.rs` is the source of truth for authoritative component registration via `with_component_schema_entries!`. `component_tables.rs`, `delta.rs`, `world.rs`, and `world_txn.rs` project from that manifest. The ticket should target the schema entry, not describe each downstream macro output as an independent architectural decision.
4. `canonical.rs` does not maintain a per-component hashing traversal. `hash_world()` hashes the whole serialized `World` in [crates/worldwake-core/src/canonical.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/canonical.rs). No production canonical code change is required once `RecordData` becomes part of `World`; only coverage should prove that record mutations affect the world hash.
5. `WorldTxn` already mirrors the `World` creation helpers for agents, factions, offices, items, unique items, and containers in [crates/worldwake-core/src/world_txn.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs). Leaving record creation out of `WorldTxn` would create an inconsistent, leak-prone creation path, so txn creation is in scope for this ticket.
6. `cargo test -p worldwake-core -- --list` confirms the current focused verification surfaces exist and the command is real. The ticket’s test plan should use concrete existing targets plus the full crate suite, not a speculative workspace command chain as the primary gate.

## Architecture Check

1. Integrating `RecordData` through the existing component manifest is cleaner than adding bespoke record storage because records are ordinary authoritative world artifacts under E16c, not a special side registry.
2. Adding both `World::create_record()` and `WorldTxn::create_record()` is more robust than leaving creation to raw entity/component calls. It preserves the existing “typed creation helper records the right deltas” architecture and avoids duplicate call-site assembly.
3. No backward-compatibility shims are needed. The correct architecture is to make records real entities now and let downstream code adopt that single authoritative path.
4. This is more beneficial than the current architecture because the current state already contains the record domain model but strands it outside the authoritative ECS. Wiring it through makes records usable by future consultation, placement, custody, and event-log workflows without introducing parallel storage paths.

## Verification Layers

1. Entity classification and deterministic ordering -> focused `entity.rs` roundtrip/ordering coverage
2. Authoritative component storage and kind gating -> focused `component_tables.rs` / `world.rs` component CRUD and wrong-kind rejection coverage
3. Event-log component delta typing -> focused `delta.rs` and `world_txn.rs` delta tests
4. World creation helper semantics -> focused `world.rs` / `world_txn.rs` creation tests
5. Canonical world hashing includes record state through whole-world serialization -> focused `canonical.rs` hash-difference test

## What to Change

### 1. Add `EntityKind::Record`

Extend [crates/worldwake-core/src/entity.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/entity.rs) with `Record` and update the canonical test list accordingly.

### 2. Register `RecordData` in the authoritative component manifest

Add a `RecordData` entry to [crates/worldwake-core/src/component_schema.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_schema.rs) with the kind predicate `|kind| kind == EntityKind::Record`.

This single manifest change should drive:

- typed storage in `ComponentTables`
- typed delta variants in `delta.rs`
- typed world component accessors
- typed txn set/clear helpers where applicable

### 3. Add typed record creation helpers

Add:

- `World::create_record(record: RecordData, tick: Tick) -> Result<EntityId, WorldError>`
- `WorldTxn::create_record(record: RecordData) -> Result<EntityId, WorldError>`

These should mirror existing typed creation helpers: allocate the correct entity kind, attach the component, and in the txn layer record the created-entity delta.

### 4. Strengthen focused coverage instead of adding bespoke hashing logic

Do not add special-case canonical production code. Add or update tests proving that:

- a record component can be stored and queried through the standard APIs
- wrong-kind insertion is rejected
- record component deltas roundtrip with the generated `ComponentKind::RecordData`
- world hashing changes when record state changes after ECS integration

## Files to Touch

- `crates/worldwake-core/src/entity.rs`
- `crates/worldwake-core/src/component_schema.rs`
- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/world_txn.rs`
- `crates/worldwake-core/src/canonical.rs`

## Out of Scope

- New institutional types or belief types in `institutional.rs`
- Record consultation actions or affordances
- `AgentBeliefStore` institutional-belief integration (ticket `-003`)
- Record mutation helpers beyond core creation/plumbing
- AI-layer planning changes

## Acceptance Criteria

### Tests That Must Pass

1. `EntityKind::Record` roundtrips and participates in deterministic ordering
2. `RecordData` is available through the normal component table/world APIs only for `EntityKind::Record`
3. `ComponentValue::RecordData` roundtrips through bincode and reports `ComponentKind::RecordData`
4. `World::create_record()` creates a `Record` entity with attached `RecordData`
5. `WorldTxn::create_record()` records entity/component deltas and commits a valid record entity
6. `hash_world()` changes when integrated `RecordData` changes
7. `cargo test -p worldwake-core` passes

### Invariants

1. Records are modeled as ordinary authoritative entities, not a side table
2. `RecordData` cannot be attached to non-record entities
3. No compatibility alias or duplicate record-creation path is introduced

## Tests

### New/Modified Tests

1. `crates/worldwake-core/src/entity.rs` — extend the canonical variant list and ordering/roundtrip coverage to include `Record`.
   Rationale: proves the authoritative entity classification surface changed cleanly and deterministically.
2. `crates/worldwake-core/src/component_tables.rs` and/or `crates/worldwake-core/src/world.rs` — add focused `RecordData` insert/get/remove and wrong-kind rejection coverage.
   Rationale: proves records are wired into the same authoritative component architecture as other first-class entities.
3. `crates/worldwake-core/src/delta.rs` — add a `RecordData` sample and assert it participates in `ComponentKind::ALL`.
   Rationale: proves event-log deltas can carry record component mutations without bespoke plumbing.
4. `crates/worldwake-core/src/world.rs` — add `create_record()` coverage.
   Rationale: proves the authoritative world exposes a clean typed creation path.
5. `crates/worldwake-core/src/world_txn.rs` — add `create_record()` delta/commit coverage.
   Rationale: proves transactional creation preserves the existing event-log architecture rather than forcing ad hoc caller assembly.
6. `crates/worldwake-core/src/canonical.rs` — add a focused hash-difference test for record mutation.
   Rationale: proves whole-world canonical hashing naturally includes record state once ECS integration is complete.

### Commands

1. `cargo test -p worldwake-core create_record`
2. `cargo test -p worldwake-core component_tables::tests::insert_and_get_record_data`
3. `cargo test -p worldwake-core delta::tests::component_kind_variants_match_authoritative_components`
4. `cargo test -p worldwake-core canonical::tests::hash_world_changes_when_record_data_changes`
5. `cargo test -p worldwake-core`
6. `cargo clippy --workspace`
7. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-21
- Actual changes:
  - Added `EntityKind::Record`
  - Registered `RecordData` in the authoritative component manifest so tables, world APIs, deltas, and txn simple-set helpers are generated through the existing architecture
  - Added `World::create_record()` and `WorldTxn::create_record()`
  - Treated records as physically placeable entities by routing them through the existing in-transit-on-create path
  - Added focused coverage for record component storage, wrong-kind rejection, txn deltas, and canonical hash sensitivity
- Deviations from original plan:
  - No production change was needed in `canonical.rs`; whole-world hashing already covered record state once `RecordData` became part of `World`
  - `WorldTxn` record creation was brought into scope because omitting it would have left an inconsistent, weaker creation path than the rest of the architecture
- Verification results:
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace` passed
  - `cargo test --workspace` passed
