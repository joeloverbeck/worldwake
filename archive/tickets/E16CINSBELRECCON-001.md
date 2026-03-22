# E16CINSBELRECCON-001: Core Institutional Claim and Record Types

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module `institutional.rs` in worldwake-core
**Deps**: None (pure type definitions)

## Problem

E16c requires typed institutional claims, record entries, belief keys, and read-model types. These are foundational types that every subsequent ticket depends on. They must exist before component registration, belief store extension, or action handlers can be written.

## Assumption Reassessment (2026-03-21)

1. No `institutional.rs` module exists in `crates/worldwake-core/src/`, and no `InstitutionalClaim`, `InstitutionalRecordEntry`, `RecordData`, `RecordKind`, `RecordEntryId`, `InstitutionalBeliefKey`, or `InstitutionalBeliefRead` types exist anywhere in the workspace. Current institutional authority still lives only in existing office/faction/support relations and metadata such as [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/offices.rs) and [factions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/factions.rs).
2. The live spec in [E16c-institutional-beliefs-and-record-consultation.md](/home/joeloverbeck/projects/worldwake/specs/E16c-institutional-beliefs-and-record-consultation.md) defines this ticket's shared value layer in deliverables §1–§5. `RecordData` absorbs entries directly; there is no separate `InstitutionalRecord` component.
3. The original ticket overstated two claim shapes. Per spec §2 and §9, `InstitutionalClaim::OfficeHolder` uses `holder: Option<EntityId>` so vacancy is representable, and `InstitutionalClaim::SupportDeclaration` uses `candidate: Option<EntityId>` so support withdrawal / explicit no-candidate state is representable. The ticket scope is corrected below.
4. N/A — no AI layer.
5. N/A — no ordering.
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. Mismatch + correction: current core already has `AgentBeliefStore` and `PerceptionProfile` in [belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs), but E16c's `institutional_beliefs` store extension, `PerceptionProfile` consultation fields, `EntityKind::Record`, component registration, and `WorldTxn` helpers belong to follow-on tickets because they touch component schema, deltas, world factories, and runtime surfaces. This ticket stays intentionally narrow: define the shared institutional value types and their local record-manipulation behavior, then export them from `lib.rs`.
12. N/A.

## Architecture Check

1. A dedicated `institutional.rs` module is cleaner than scattering these cross-cutting types across `offices.rs`, `factions.rs`, and `belief.rs`. Offices/factions remain authoritative domain data; `institutional.rs` becomes the shared boundary for record entries, institutional claims, and institutional belief value types reused later by belief storage, runtime read models, and record actions.
2. No backward-compatibility shims — entirely new types.
3. `RecordData` should own its own small validation error type rather than returning `WorldError`. These methods mutate an in-memory record value, not world authority. Keeping the error local avoids leaking world-transaction semantics into a plain data structure.

## Verification Layers

1. Serialization / deterministic trait contracts for the new value types -> focused unit tests in `institutional.rs`
2. `RecordData` append / supersede / active-entry semantics -> focused unit tests in `institutional.rs`
3. Deterministic ordering for BTree-safe keys and identifiers -> focused unit tests in `institutional.rs`
4. Single-layer ticket — type definitions only, no cross-layer mapping needed.

## What to Change

### 1. New module `crates/worldwake-core/src/institutional.rs`

Define all types from spec §1–§5:

- `RecordKind` enum: `OfficeRegister`, `FactionRoster`, `SupportLedger`
- `RecordEntryId(pub u64)` newtype
- `InstitutionalClaim` enum with variants: `OfficeHolder { office, holder: Option<EntityId>, effective_tick }`, `FactionMembership { faction, member, active, effective_tick }`, `SupportDeclaration { office, supporter, candidate: Option<EntityId>, effective_tick }`
- `InstitutionalRecordEntry` struct: `entry_id`, `claim`, `recorded_tick`, `supersedes`
- `RecordData` struct: `record_kind`, `home_place`, `issuer`, `consultation_ticks`, `max_entries_per_consult`, `entries`, `next_entry_id`
- `InstitutionalRecordError` enum for local record-manipulation failures
- `RecordData` methods: `append_entry()`, `supersede_entry()`, `entries_newest_first()`, `active_entries()`
- `InstitutionalBeliefKey` enum: `OfficeHolderOf { office }`, `FactionMembersOf { faction }`, `SupportFor { supporter, office }`
- `BelievedInstitutionalClaim` struct: `claim`, `source`, `learned_tick`, `learned_at`
- `InstitutionalKnowledgeSource` enum: `WitnessedEvent`, `Report { from, chain_len }`, `RecordConsultation { record, entry_id }`, `SelfDeclaration`
- `InstitutionalBeliefRead<T>` enum: `Unknown`, `Certain(T)`, `Conflicted(Vec<T>)`

All types must derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Types used as BTreeMap keys must additionally derive `Ord, PartialOrd, Hash`.

`RecordData::entries_newest_first()` ordering is append-order / `entry_id` order, not `effective_tick` order. This matches the spec's "record entries are durable world state" rule: later appended entries are newer record statements even when they describe an earlier effective institutional change.

`RecordData::supersede_entry()` must reject a missing `old_id`. It should also reject attempts to supersede the same entry more than once, because duplicate supersession of one entry would create ambiguous "active" lineage inside a single record artifact.

### 2. Register module in `crates/worldwake-core/src/lib.rs`

Add `pub mod institutional;` and re-export key types.

## Files to Touch

- `crates/worldwake-core/src/institutional.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module declaration and re-exports)

## Out of Scope

- `EntityKind::Record` variant and record entity creation
- Component registration for `RecordData` (ticket -002)
- `AgentBeliefStore` extension (ticket -003)
- `PerceptionProfile` extension (ticket -003)
- `WorldTxn` helpers (ticket -004)
- Any action definitions or handlers
- Any AI-layer changes
- `ComponentKind`/`ComponentValue`/delta changes (ticket -002)

## Acceptance Criteria

### Tests That Must Pass

1. `RecordData::append_entry` increments `next_entry_id` and returns correct `RecordEntryId`
2. `RecordData::supersede_entry` appends new entry with `supersedes` pointing to old entry
3. `RecordData::supersede_entry` returns error if `old_id` does not exist
4. `RecordData::supersede_entry` returns error if `old_id` has already been superseded
5. `RecordData::active_entries` excludes entries that have been superseded
6. `RecordData::entries_newest_first` returns entries in reverse append order by `entry_id`
7. All types roundtrip through bincode serialization
8. `InstitutionalBeliefKey` has deterministic `Ord` ordering (BTreeMap-safe)
9. Nullable vacancy / no-candidate claim shapes roundtrip correctly
10. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. All types are `#[derive(Serialize, Deserialize)]` for persistence compatibility
2. No `HashMap`/`HashSet` — only `BTreeMap`/`BTreeSet` or `Vec` for determinism
3. No floats — `Tick` and `Permille` for numeric values
4. `RecordData.next_entry_id` is monotonically increasing within a record
5. A single record entry can have at most one direct superseding successor inside one `RecordData`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/institutional.rs` (new inline `#[cfg(test)] mod tests`) — verifies serde/trait contracts for the new institutional value types
2. `crates/worldwake-core/src/institutional.rs` (same module) — verifies `RecordData` append/supersede semantics, including missing-entry and duplicate-supersede rejection
3. `crates/worldwake-core/src/institutional.rs` (same module) — verifies deterministic ordering and nullable vacancy / no-candidate claim support

### Commands

1. `cargo test -p worldwake-core institutional::tests::`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace && cargo test --workspace`

## Outcome

- Completion date: 2026-03-21
- What actually changed: added `crates/worldwake-core/src/institutional.rs` with shared institutional record/claim/belief value types, local record mutation methods, a local `InstitutionalRecordError`, and focused tests; updated `crates/worldwake-core/src/lib.rs` to export the new module.
- Deviations from original plan: corrected the ticket/spec mismatch for nullable `OfficeHolder.holder` and `SupportDeclaration.candidate`; kept the ticket intentionally value-layer only and did not widen into `EntityKind::Record`, component registration, `AgentBeliefStore`, `PerceptionProfile`, or `WorldTxn` changes because those belong to follow-on E16c tickets.
- Verification results: `cargo test -p worldwake-core institutional::tests::`, `cargo test -p worldwake-core`, `cargo clippy --workspace`, and `cargo test --workspace` all passed.
