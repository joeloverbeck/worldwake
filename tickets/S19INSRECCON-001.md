# S19INSRECCON-001: Harness helpers — RULERS_HALL constant + seed_office_vacancy_entry()

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — golden harness helper additions only
**Deps**: E16c (ConsultRecord action, Record entities) — COMPLETED; E16d (golden harness office helpers) — COMPLETED

## Problem

The S19 golden scenarios require two harness utilities that do not yet exist:
1. A `RULERS_HALL` constant for the `PrototypePlace::RulersHall` entity, matching the existing `VILLAGE_SQUARE` / `ORCHARD_FARM` / `PUBLIC_LATRINE` pattern.
2. A `seed_office_vacancy_entry()` helper that appends an `InstitutionalClaim::OfficeHolder { office, holder: None }` entry to an existing `RecordKind::OfficeRegister` record. `seed_office()` already creates the record entities with empty entries (`entries: Vec::new()`); this helper populates the vacancy information that the ConsultRecord handler will later read.

These helpers are prerequisites for all three S19 golden scenarios (Scenarios 32–34).

## Assumption Reassessment (2026-03-22)

1. `seed_office()` at `golden_harness/mod.rs:741` creates `RecordKind::OfficeRegister` and `RecordKind::SupportLedger` records at the office's jurisdiction with `consultation_ticks: 4`, `max_entries_per_consult: 6`, and `entries: Vec::new()`. Confirmed by reading the code.
2. `WorldTxn::append_record_entry()` at `world_txn.rs:247` accepts a record `EntityId` and `InstitutionalClaim`, appending an `InstitutionalRecordEntry` to the record's `RecordData.entries`. This is the authoritative API for populating records.
3. `PrototypePlace::RulersHall` exists at `topology.rs:54` with slot index 4. The existing `prototype_place_entity()` function (used for `VILLAGE_SQUARE`, `ORCHARD_FARM`, `PUBLIC_LATRINE`) converts a `PrototypePlace` to `EntityId`. No `RULERS_HALL` constant exists yet in the harness.
4. The `InstitutionalClaim::OfficeHolder { office, holder: None, effective_tick }` variant is the claim type used for vacancy entries, confirmed at `component_tables.rs:256` and `golden_harness/mod.rs:300`.
5. No existing helper seeds vacancy entries into records — `seed_office_holder_belief()` seeds beliefs into agent belief stores, not record entries. These are distinct: one is authoritative world state (record), the other is agent knowledge (belief store).
10. This is a harness-only ticket with no scenario isolation concerns. It provides building blocks for S19INSRECCON-002 through S19INSRECCON-004.

## Architecture Check

1. Adding a `const` and a helper function to the golden harness is the simplest possible approach. The alternative — inlining the record entry seeding in each scenario — would duplicate 8–10 lines across 3 tests.
2. No backward-compatibility shims introduced. This is additive-only.

## Verification Layers

1. `RULERS_HALL` constant → compile-time verification (if the slot doesn't match, `prototype_place_entity` would produce wrong IDs, caught by downstream scenario tests)
2. `seed_office_vacancy_entry()` → downstream golden tests in S19INSRECCON-002..004 exercise it end-to-end; no standalone unit test needed for a 10-line helper
5. Single-layer ticket: this is infrastructure, verified through consumption in later tickets.

## What to Change

### 1. Add `RULERS_HALL` constant

In `golden_harness/mod.rs`, after the existing place constants (`VILLAGE_SQUARE`, `ORCHARD_FARM`, `PUBLIC_LATRINE`), add:

```rust
/// Ruler's Hall — slot 4.
pub const RULERS_HALL: EntityId = prototype_place_entity(PrototypePlace::RulersHall);
```

### 2. Add `seed_office_vacancy_entry()` helper

In `golden_harness/mod.rs`, near the existing `seed_office()` and `seed_office_holder_belief()` helpers, add a function that:
1. Finds the `RecordKind::OfficeRegister` record at a given place using `world.query_record_data()`.
2. Calls `WorldTxn::append_record_entry(record_entity, InstitutionalClaim::OfficeHolder { office, holder: None, effective_tick: Tick(0) })`.
3. Panics with a clear message if no OfficeRegister is found at the given place.

Signature:
```rust
pub fn seed_office_vacancy_entry(
    world: &mut World,
    event_log: &mut EventLog,
    office: EntityId,
    record_place: EntityId,
)
```

The `record_place` parameter allows Scenario 33 to place the record at `RULERS_HALL` (different from the office's jurisdiction at `VILLAGE_SQUARE`). For Scenarios 32 and 34, `record_place` equals the office jurisdiction.

**Important nuance**: For Scenario 33, the record must be at `RULERS_HALL`, not at the office jurisdiction (`VILLAGE_SQUARE`). This means we cannot rely on `seed_office()` to create the record at the right place for Scenario 33. The helper must find (or create) a record at `record_place`. Two approaches:
- Option A: `seed_office_vacancy_entry` only appends to an existing record at `record_place`. Scenario 33's setup creates a separate record at `RULERS_HALL` before calling this helper.
- Option B: `seed_office_vacancy_entry` creates a new record at `record_place` if none exists there.

**Recommendation**: Option A — keep it simple. The helper appends entries; record creation is either handled by `seed_office()` (for co-located scenarios) or by explicit `WorldTxn::create_record()` in the scenario setup (for remote scenarios). This avoids hidden side effects.

### 3. Add `find_office_register_at()` helper (optional, DRY)

A small private helper to locate the OfficeRegister record entity at a given place:

```rust
fn find_office_register_at(world: &World, place: EntityId) -> EntityId
```

Used by `seed_office_vacancy_entry()` and potentially by assertion code in scenarios.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)

## Out of Scope

- No new golden test scenarios (those are S19INSRECCON-002..004)
- No changes to engine code (`worldwake-core`, `worldwake-sim`, `worldwake-systems`)
- No changes to `golden_offices.rs`
- No changes to documentation files
- No changes to `seed_office()` itself — it already creates records correctly for co-located scenarios

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — existing golden and unit tests must not regress
2. `cargo test --workspace` — full workspace passes
3. `cargo clippy --workspace --all-targets -- -D warnings` — no lint warnings

### Invariants

1. `RULERS_HALL` must equal `prototype_place_entity(PrototypePlace::RulersHall)` — matching slot 4
2. `seed_office_vacancy_entry()` must use the authoritative `WorldTxn::append_record_entry()` API — no manual `RecordData.entries` mutation
3. The helper must not silently create records — it must find an existing OfficeRegister at the specified place, or panic with a descriptive message
4. All existing golden tests continue to pass unchanged

## Test Plan

### New/Modified Tests

1. None — infrastructure-only ticket; verification is through compile success and downstream consumption in S19INSRECCON-002..004. Existing golden tests serve as regression guard.

### Commands

1. `cargo test -p worldwake-ai` — AI crate (includes all goldens)
2. `cargo test --workspace` — full workspace
3. `cargo clippy --workspace --all-targets -- -D warnings` — lint
