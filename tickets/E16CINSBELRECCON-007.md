# E16CINSBELRECCON-007: Atomic Record Mutation in Office/Faction/Support Handlers

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extend existing political action handlers in worldwake-systems
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-002, E16CINSBELRECCON-004

## Problem

When authoritative institutional truth changes (office installed, support declared, faction membership changed), the responsible handler must atomically append or supersede the corresponding record entry in the same `WorldTxn` commit. Without this, records become stale caches that drift from truth, violating Principle 25 (records are world artifacts, not derived caches).

## Assumption Reassessment (2026-03-21)

1. `office_actions.rs` in worldwake-systems handles support declaration actions. `offices.rs` handles office vacancy/installation logic. Both mutate authoritative state through `WorldTxn`.
2. The spec requires that when an office is installed or vacated, the office register record entry is appended/superseded. When support is declared, the support ledger entry is appended/superseded.
3. Records must already exist at the relevant place (created during world setup or by a prior system). The handlers must look up the relevant record by `RecordKind` at the institutional entity's home place.
4. N/A — not a planner ticket.
5. N/A — no ordering.
6. N/A — no heuristic removal.
7. N/A.
8. Closure boundary: office-holder mutation and support-declaration mutation. The handlers being modified are `office_actions.rs` (declare_support handler) and `offices.rs` (install/vacate logic).
9. N/A.
10. N/A.
11. No mismatch.
12. N/A.

## Architecture Check

1. Co-locating record mutation with authoritative state mutation in the same handler/txn is the only way to guarantee atomicity. A separate reconciliation system would demote records to caches (spec §8 explicitly forbids this).
2. No backward-compatibility shims.

## Verification Layers

1. Office installation appends OfficeHolder entry → inspect RecordData after commit
2. Support declaration appends/supersedes SupportDeclaration entry → inspect RecordData
3. Record mutation is atomic with authoritative mutation → single WorldTxn commit check
4. Supersession: new entry has `supersedes: Some(old_id)` when replacing prior entry → RecordData.active_entries() check

## What to Change

### 1. Extend office installation/vacancy in `offices.rs`

When `WorldTxn` commits an office holder change:
- Find the `OfficeRegister` record at the office's home place (via relation query)
- If a prior entry exists for this office, supersede it with the new holder state
- If no prior entry exists, append a fresh entry
- Use `WorldTxn::append_record_entry()` / `supersede_record_entry()`

### 2. Extend support declaration handler in `office_actions.rs`

When `WorldTxn` commits a support declaration:
- Find the `SupportLedger` record at the office's home place
- If a prior entry exists for this (supporter, office) pair, supersede it
- If no prior entry exists, append a fresh entry
- Use `WorldTxn::append_record_entry()` / `supersede_record_entry()`

### 3. Helper: find record by kind at place

Add a utility function (either on `WorldTxn` or as a free function in worldwake-systems) that finds a record entity of a given `RecordKind` at a given place. This avoids duplicating the lookup logic across multiple handlers.

## Files to Touch

- `crates/worldwake-systems/src/offices.rs` (modify — append/supersede office register entries on install/vacate)
- `crates/worldwake-systems/src/office_actions.rs` (modify — append/supersede support ledger entries on support declaration)

## Out of Scope

- Creating record entities (records are created during world setup — the prototype world builder should be extended separately if needed)
- Faction roster mutations (no faction membership action handlers exist yet — deferred until faction actions are implemented)
- ConsultRecord action (ticket -005)
- Perception projection (ticket -006)
- AI reading records (Phase B2 tickets)
- E16b force-claim record mutations (deferred to E16b)

## Acceptance Criteria

### Tests That Must Pass

1. After office installation, the office register record at the office's home place contains an `OfficeHolder` entry with the new holder
2. After second office installation (succession), the old entry is superseded and the new entry is active
3. After support declaration, the support ledger record contains a `SupportDeclaration` entry
4. After re-declaration of support (changed candidate), the old entry is superseded
5. Record mutation happens in the same `WorldTxn` commit as the authoritative state change (no separate tick needed)
6. If no record of the correct kind exists at the place, the handler emits a warning or error (not a silent no-op)
7. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Records are never regenerated from authoritative state — only appended/superseded by handlers (Principle 25)
2. Record mutations are atomic with authoritative mutations (same WorldTxn)
3. `RecordData.active_entries()` reflects current truth after handler commit

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs` — office install creates record entry, succession supersedes, vacancy recorded
2. `crates/worldwake-systems/src/office_actions.rs` — support declaration creates entry, re-declaration supersedes

### Commands

1. `cargo test -p worldwake-systems offices`
2. `cargo test -p worldwake-systems office_actions`
3. `cargo clippy --workspace && cargo test --workspace`
