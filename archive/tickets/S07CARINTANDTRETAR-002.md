# S07CARINTANDTRETAR-002: Add care_weight to UtilityProfile

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — component schema in worldwake-core
**Deps**: None (independent of 001, but logically part of core changes)

## Problem

The current ranking for treatment uses `treatment_pain()` which conflates self-pain and local patient pain with no per-agent care sensitivity. The spec requires a `care_weight: Permille` field in `UtilityProfile` so that third-party care urgency is governed by a separate, per-agent weight (Principle 20 diversity).

## Assumption Reassessment (2026-03-17)

1. `UtilityProfile` exists in `utility_profile.rs` with 10 fields — confirmed
2. `Default` impl uses `Permille::new_unchecked(500)` for most weights, `200` for `social_weight` — confirmed
3. `UtilityProfile` implements `Component`, `Clone`, `Debug`, `Eq`, `PartialEq`, `Serialize`, `Deserialize` — confirmed
4. Existing bincode roundtrip test covers all current fields — confirmed
5. No `care_weight` field exists yet — confirmed

## Architecture Check

1. Adding a field to `UtilityProfile` is the correct place — this is where all per-agent decision weights live.
2. Default `Permille(200)` matches spec: low baseline, most agents prioritize self over others. Same as `social_weight` default.
3. No shim or migration — the field is simply added. Deserialization of old save files will fail (acceptable per Principle 26).

## What to Change

### 1. Add `care_weight` field to `UtilityProfile`

```rust
pub care_weight: Permille,
```

### 2. Update `Default` impl

Set `care_weight: Permille::new_unchecked(200)` (low baseline per spec D06).

### 3. Update tests

- Update `utility_profile_default_is_balanced` to assert `care_weight.value() == 200`
- Update `utility_profile_roundtrips_through_bincode` to set a non-default `care_weight` and verify roundtrip

## Files to Touch

- `crates/worldwake-core/src/utility_profile.rs` (modify)

## Out of Scope

- Ranking logic changes that use `care_weight` (ticket 006)
- Candidate generation changes (ticket 005)
- Any other crate changes
- Updating golden tests or harness helpers that construct `UtilityProfile`

## Acceptance Criteria

### Tests That Must Pass

1. `UtilityProfile` with `care_weight` defaults to `Permille(200)` — updated default test
2. `UtilityProfile` with custom `care_weight` roundtrips through bincode — updated roundtrip test
3. `UtilityProfile` still satisfies component and value bounds — existing bounds test
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `UtilityProfile` has exactly 11 fields (added `care_weight`)
2. `care_weight` default is `Permille(200)` — low baseline per spec
3. `UtilityProfile` still implements `Component`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/utility_profile.rs` — modify `utility_profile_default_is_balanced` to assert `care_weight`
2. `crates/worldwake-core/src/utility_profile.rs` — modify roundtrip test to include non-default `care_weight`

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy -p worldwake-core`

## Outcome

- **Completion date**: 2026-03-18
- **What changed**: Added `care_weight: Permille` field to `UtilityProfile` (now 11 fields) with default `Permille(200)`. Updated all explicit struct-literal construction sites across `utility_profile.rs`, `test_utils.rs`, `ranking.rs`, and `goal_explanation.rs`.
- **Deviations**: Ticket marked golden tests/harness helpers as out of scope, but `test_utils::sample_utility_profile()` and two test `utility()` helpers in `worldwake-ai` required updating to compile. No behavioral deviations.
- **Verification**: All 8 utility_profile tests pass, clippy clean on worldwake-core.
