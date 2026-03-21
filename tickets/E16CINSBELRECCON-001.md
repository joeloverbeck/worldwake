# E16CINSBELRECCON-001: Core Institutional Claim and Record Types

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new module `institutional.rs` in worldwake-core
**Deps**: None (pure type definitions)

## Problem

E16c requires typed institutional claims, record entries, belief keys, and read-model types. These are foundational types that every subsequent ticket depends on. They must exist before component registration, belief store extension, or action handlers can be written.

## Assumption Reassessment (2026-03-21)

1. No `institutional.rs` module exists in `crates/worldwake-core/src/` — confirmed via grep. No `InstitutionalClaim`, `RecordData`, `RecordKind`, `RecordEntryId` types exist anywhere.
2. Spec deliverables §1–§5 define the types. `RecordData` absorbs entries (no separate `InstitutionalRecord` component).
3. Not a planner/golden ticket — pure type definitions.
4. N/A — no AI layer.
5. N/A — no ordering.
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatch — types do not exist yet.
12. N/A.

## Architecture Check

1. Single module for all institutional types keeps related definitions together and avoids scattering across entity.rs, belief.rs, etc. Types are `pub` and re-exported from `lib.rs`.
2. No backward-compatibility shims — entirely new types.

## Verification Layers

1. All types derive `Serialize`/`Deserialize` → bincode roundtrip tests
2. `RecordData` methods (append, supersede, active_entries, entries_newest_first) → focused unit tests
3. `InstitutionalBeliefRead` enum → unit tests for Unknown/Certain/Conflicted construction
4. Single-layer ticket — type definitions only, no cross-layer mapping needed.

## What to Change

### 1. New module `crates/worldwake-core/src/institutional.rs`

Define all types from spec §1–§5:

- `RecordKind` enum: `OfficeRegister`, `FactionRoster`, `SupportLedger`
- `RecordEntryId(pub u64)` newtype
- `InstitutionalClaim` enum with variants: `OfficeHolder { office, holder, effective_tick }`, `FactionMembership { faction, member, active, effective_tick }`, `SupportDeclaration { office, supporter, candidate, effective_tick }`
- `InstitutionalRecordEntry` struct: `entry_id`, `claim`, `recorded_tick`, `supersedes`
- `RecordData` struct: `record_kind`, `home_place`, `issuer`, `consultation_ticks`, `max_entries_per_consult`, `entries`, `next_entry_id`
- `RecordData` methods: `append_entry()`, `supersede_entry()`, `entries_newest_first()`, `active_entries()`
- `InstitutionalBeliefKey` enum: `OfficeHolderOf { office }`, `FactionMembersOf { faction }`, `SupportFor { supporter, office }`
- `BelievedInstitutionalClaim` struct: `claim`, `source`, `learned_tick`, `learned_at`
- `InstitutionalKnowledgeSource` enum: `WitnessedEvent`, `Report { from, chain_len }`, `RecordConsultation { record, entry_id }`, `SelfDeclaration`
- `InstitutionalBeliefRead<T>` enum: `Unknown`, `Certain(T)`, `Conflicted(Vec<T>)`

All types must derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Types used as BTreeMap keys must additionally derive `Ord, PartialOrd, Hash`.

### 2. Register module in `crates/worldwake-core/src/lib.rs`

Add `pub mod institutional;` and re-export key types.

## Files to Touch

- `crates/worldwake-core/src/institutional.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module declaration and re-exports)

## Out of Scope

- `EntityKind::Record` variant (ticket -002)
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
4. `RecordData::active_entries` excludes entries that have been superseded
5. `RecordData::entries_newest_first` returns entries in reverse chronological order by `entry_id`
6. All types roundtrip through bincode serialization
7. `InstitutionalBeliefKey` has deterministic `Ord` ordering (BTreeMap-safe)
8. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. All types are `#[derive(Serialize, Deserialize)]` for persistence compatibility
2. No `HashMap`/`HashSet` — only `BTreeMap`/`BTreeSet` or `Vec` for determinism
3. No floats — `Tick` and `Permille` for numeric values
4. `RecordData.next_entry_id` is monotonically increasing within a record

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/institutional.rs` (inline `#[cfg(test)] mod tests`) — roundtrip serialization, append/supersede logic, active_entries filtering, ordering guarantees

### Commands

1. `cargo test -p worldwake-core institutional`
2. `cargo clippy --workspace && cargo test --workspace`
