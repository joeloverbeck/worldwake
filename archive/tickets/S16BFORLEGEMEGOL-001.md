# S16BFORLEGEMEGOL-001: Promote force-control harness helpers to shared golden_harness

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S16b spec (S16b-force-legitimacy-emergence-golden-suites.md)

## Problem

`seed_known_office_at_place` and `seed_force_controller_belief` are defined as file-local functions in `golden_offices.rs`. Suites 10–12 in `golden_emergent.rs` need them. Duplicating these helpers violates DRY and diverges maintenance. Promote them to the shared harness.

## Assumption Reassessment (2026-03-22)

1. `seed_known_office_at_place` exists at `golden_offices.rs:229` — confirmed via grep. It seeds a `BelievedEntityState` with `last_known_place` for an office entity. No equivalent exists in `golden_harness/mod.rs`.
2. `seed_force_controller_belief` exists at `golden_offices.rs:257` — confirmed via grep. It seeds an `InstitutionalClaim::ForceControl` belief into an agent's belief store. No equivalent exists in `golden_harness/mod.rs`.
3. Both helpers use only public API from `worldwake_core` (`AgentBeliefStore`, `InstitutionalBeliefKey`, `InstitutionalClaim`, `BelievedInstitutionalClaim`, `BelievedEntityState`, `PerceptionSource`, `PerceptionProfile`). All types are already imported in `golden_harness/mod.rs`.
4. No `set_office_controller` direct harness helper exists — Suite 10 will use `txn.add_force_claim()` + tick progression or direct relation API, but that's Suite 10's concern, not this ticket.
5. Not a planner/AI ticket — pure test infrastructure refactor.

## Architecture Check

1. Moving helpers to the shared harness is the standard pattern; all other shared helpers (seed_office, seed_office_holder_belief, add_hostility, etc.) already live in `golden_harness/mod.rs`.
2. No backward-compatibility shims — the originals in `golden_offices.rs` will be replaced with imports from the shared module.

## Verification Layers

1. All existing golden_offices tests pass unchanged → `cargo test -p worldwake-ai --test golden_offices`
2. All existing golden_emergent tests pass unchanged → `cargo test -p worldwake-ai --test golden_emergent`
3. Single-layer ticket: pure refactor with no behavioral change. Additional verification layers not applicable.

## What to Change

### 1. Move `seed_known_office_at_place` to `golden_harness/mod.rs`

Copy the function from `golden_offices.rs:229-254` into `golden_harness/mod.rs` in the "Office / Faction / Political helpers" section (after `seed_office_vacancy_entry`). Make it `pub`. Ensure the required imports (`BelievedEntityState`, `PerceptionSource`) are present (they already are).

### 2. Move `seed_force_controller_belief` to `golden_harness/mod.rs`

Copy the function from `golden_offices.rs:257-294` into `golden_harness/mod.rs` after `seed_known_office_at_place`. Make it `pub`. Ensure required imports (`InstitutionalBeliefKey::ForceControllerOf`, `InstitutionalClaim::ForceControl`) are present (they already are in the `use worldwake_core` block).

### 3. Remove originals from `golden_offices.rs`

Delete the two function definitions from `golden_offices.rs`. Since the file already does `use golden_harness::*`, the moved helpers will be available without additional import changes.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add 2 pub helpers)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify — remove 2 file-local functions)

## Out of Scope

- Any new test logic or new test functions
- Changes to `golden_emergent.rs` (consumers come in later tickets)
- Adding a `set_office_controller` helper (deferred to Suite 10 ticket if needed)
- Modifying any production crate code

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_offices` — all existing office tests pass unchanged
2. `cargo test -p worldwake-ai --test golden_emergent` — all existing emergent tests pass unchanged
3. `cargo clippy -p worldwake-ai` — no new warnings

### Invariants

1. No behavioral change to any existing test — identical assertions, identical outcomes
2. Both promoted helpers are `pub` and accessible via `use golden_harness::*`

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-ai --test golden_offices`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo clippy -p worldwake-ai`

## Outcome

- **Completion date**: 2026-03-22
- **What changed**: Promoted `seed_known_office_at_place` and `seed_force_controller_belief` from file-local functions in `golden_offices.rs` to `pub` helpers in `golden_harness/mod.rs`. Added `BTreeMap` import to harness. Removed unused `BTreeMap` and `BelievedEntityState` imports from `golden_offices.rs`.
- **Deviations**: None.
- **Verification**: golden_offices 35/35 passed, golden_emergent 38/38 passed, clippy clean.
