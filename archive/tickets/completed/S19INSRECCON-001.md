# S19INSRECCON-001: Harness helpers — RULERS_HALL constant + office-register seeding helpers

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — golden harness helper additions and harness tests only
**Deps**: `specs/S19-institutional-record-consultation-golden-suites.md`; E16c (ConsultRecord action, Record entities) — COMPLETED; E16d (golden harness office helpers) — COMPLETED

## Problem

The S19 golden scenarios require harness utilities that do not yet exist:
1. A `RULERS_HALL` constant for the `PrototypePlace::RulersHall` entity, matching the existing `VILLAGE_SQUARE` / `ORCHARD_FARM` / `PUBLIC_LATRINE` pattern.
2. A small office-register helper surface that supports both:
   - appending a vacancy entry to an existing `RecordKind::OfficeRegister`
   - creating or ensuring an `OfficeRegister` at an explicit place for the remote-record scenario

`seed_office()` already creates office-jurisdiction records with empty `entries`, but Scenario 33 also needs an office register at `RULERS_HALL`, not just at the office jurisdiction.

These helpers are prerequisites for all three S19 golden scenarios (Scenarios 32–34).

## Assumption Reassessment (2026-03-22)

1. `seed_office()` at [`golden_harness/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) creates `RecordKind::OfficeRegister` and `RecordKind::SupportLedger` records at the office jurisdiction with `consultation_ticks: 4`, `max_entries_per_consult: 6`, and `entries: Vec::new()`. That confirms the local append case, but not the remote-record case.
2. `WorldTxn::append_record_entry()` at [`world_txn.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs) is the authoritative API for populating record entries. The ticket should keep using that instead of mutating `RecordData.entries` directly.
3. `PrototypePlace::RulersHall` exists in the live prototype topology, and `prototype_place_entity(PrototypePlace::RulersHall)` is the correct constant source. No `RULERS_HALL` harness constant exists yet.
4. `InstitutionalClaim::OfficeHolder { office, holder: None, effective_tick }` is the vacancy claim shape stored in office-register entries. That part of the original ticket is correct.
5. No existing harness helper seeds office-register entries. `seed_office_holder_belief()` writes to an agent belief store, which is a different layer from authoritative record state.
6. The parent S19 spec and Scenario 33 ticket require an office register at `RULERS_HALL`. The original append-only scope was too narrow because it would force later tickets to open-code remote record creation.
7. The original ticket referenced `specs/S19-institutional-recruitment-constraints.md`, but the live spec is [`specs/S19-institutional-record-consultation-golden-suites.md`](/home/joeloverbeck/projects/worldwake/specs/S19-institutional-record-consultation-golden-suites.md).
8. The original “no standalone tests needed” assumption is too weak. This is harness infrastructure that later goldens depend on, so direct harness tests are warranted.
9. The parent spec and follow-on tickets currently assume `consultation_speed_factor: pm(500)` makes consultation slower. Live code in [`action_semantics.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs) does the opposite: duration is `consultation_ticks * factor / 1000`, so `pm(500)` halves a 4-tick consult to 2 ticks. That discrepancy belongs to the later scenario tickets/spec, not to this harness ticket, but it matters for scope hygiene here.
10. This remains a harness-only ticket with no engine changes. It provides reusable setup primitives for S19INSRECCON-002 through S19INSRECCON-004.

## Architecture Check

1. A tiny composable helper surface is cleaner than the original append-only plan:
   - `RULERS_HALL` for place identity
   - `seed_office_register()` to create or ensure the authoritative record entity at an explicit place
   - `seed_office_vacancy_entry()` to append the vacancy claim through `WorldTxn`
2. This keeps later S19 scenarios DRY without forcing them to open-code record creation or duplicate record defaults.
3. This is still minimal. It does not introduce aliasing, compatibility paths, or a broader record abstraction than the tests need.

## Verification Layers

1. `RULERS_HALL` resolves to the live prototype place entity -> direct harness test and downstream scenario use
2. `seed_office_register()` creates or reuses the authoritative `OfficeRegister` at the requested place -> direct harness test against `RecordData`
3. `seed_office_vacancy_entry()` appends an authoritative vacancy claim through `WorldTxn::append_record_entry()` -> direct harness test against stored record entries
4. Downstream S19 goldens still exercise these helpers end-to-end, but they should not be the first place we discover a harness regression

## What to Change

### 1. Add `RULERS_HALL` constant

In `golden_harness/mod.rs`, after the existing place constants (`VILLAGE_SQUARE`, `ORCHARD_FARM`, `PUBLIC_LATRINE`), add:

```rust
/// Ruler's Hall — slot 4.
pub const RULERS_HALL: EntityId = prototype_place_entity(PrototypePlace::RulersHall);
```

### 2. Add `seed_office_register()` helper

In `golden_harness/mod.rs`, near the existing office helpers, add:

```rust
pub fn seed_office_register(
    world: &mut World,
    event_log: &mut EventLog,
    place: EntityId,
) -> EntityId
```

Behavior:
1. If an `OfficeRegister` already exists at `place`, return it.
2. Otherwise create one with the same defaults `seed_office()` uses today:
   - `record_kind: RecordKind::OfficeRegister`
   - `home_place: place`
   - `issuer: place`
   - `consultation_ticks: 4`
   - `max_entries_per_consult: 6`
   - `entries: Vec::new()`
   - `next_entry_id: 0`

This gives Scenario 33 a clean remote-record setup path without changing `seed_office()`.

### 3. Add `seed_office_vacancy_entry()` helper

In `golden_harness/mod.rs`, near the existing `seed_office()` and `seed_office_holder_belief()` helpers, add a function that:
1. Finds the `RecordKind::OfficeRegister` record at a given place using a small lookup helper.
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

**Important nuance**: Keep creation and mutation separate. `seed_office_register()` owns record creation/ensuring; `seed_office_vacancy_entry()` only appends the vacancy entry. That separation is cleaner than making the append helper silently create records as a side effect.

### 4. Add `find_office_register_at()` helper (private, DRY)

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
- No changes to `seed_office()` itself — it already creates local records correctly for co-located scenarios

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai` — existing golden and unit tests must not regress
2. `cargo test --workspace` — full workspace passes
3. `cargo clippy --workspace --all-targets -- -D warnings` — no lint warnings

### Invariants

1. `RULERS_HALL` must equal `prototype_place_entity(PrototypePlace::RulersHall)` — matching slot 4
2. `seed_office_register()` must create records with the same default consultation parameters as `seed_office()` currently uses
3. `seed_office_vacancy_entry()` must use the authoritative `WorldTxn::append_record_entry()` API — no manual `RecordData.entries` mutation
4. `seed_office_vacancy_entry()` must not silently create records — it must find an existing OfficeRegister at the specified place, or panic with a descriptive message
5. All existing golden tests continue to pass unchanged

## Test Plan

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_harness/mod.rs::seed_office_register_reuses_existing_local_register_and_creates_remote_register`  
Rationale: locks the helper contract for both local reuse and remote explicit-record creation without relying on later S19 goldens to detect regressions.
2. `crates/worldwake-ai/tests/golden_harness/mod.rs::seed_office_vacancy_entry_appends_authoritative_office_holder_none_claim`  
Rationale: proves the helper writes the vacancy entry into authoritative record state through the intended path.

### Commands

1. `cargo test -p worldwake-ai seed_office_register_reuses_existing_local_register_and_creates_remote_register`
2. `cargo test -p worldwake-ai seed_office_vacancy_entry_appends_authoritative_office_holder_none_claim`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-22
- What actually changed:
  - added `RULERS_HALL` to the golden harness place constants
  - added `seed_office_register()` so S19 scenarios can reuse the jurisdiction-local office register or create a remote `OfficeRegister` cleanly at an explicit place
  - added `seed_office_vacancy_entry()` so vacancy facts are appended through `WorldTxn::append_record_entry()`
  - added direct golden-harness tests for both helpers instead of relying only on later S19 goldens
- Deviations from original plan:
  - the original ticket’s append-only helper shape was widened into a small composable record-helper surface because Scenario 33 needs explicit remote record creation at `RULERS_HALL`
  - the ticket was corrected to reference the live S19 spec file and to document that `consultation_speed_factor: pm(500)` speeds consultation up in current code rather than slowing it down
  - to satisfy the repo’s required `cargo clippy --workspace --all-targets -- -D warnings` baseline, a few existing over-limit tests/helpers outside this ticket’s harness surface received minimal lint-only annotations or tiny style cleanups; no authoritative behavior changed
- Verification results:
  - `cargo test -p worldwake-ai seed_office_register_reuses_existing_local_register_and_creates_remote_register` passed
  - `cargo test -p worldwake-ai seed_office_vacancy_entry_appends_authoritative_office_holder_none_claim` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
