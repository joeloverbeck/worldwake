# E12COMHEA-006: Wound helper functions (wound_load, is_incapacitated, fatality/bleeding helpers)

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-core wounds module
**Deps**: E12COMHEA-001 (Wound.bleed_rate_per_tick), E12COMHEA-002 (CombatProfile, DeadAt)

## Problem

Authoritative checks for wound load, incapacitation, and wound-load fatality are derived from `WoundList` + `CombatProfile`. These derived computations need shared helper functions so that combat, scheduling, action validation, and AI all use the same logic. Per Principle 3, these are read-model functions, never stored state. Actual death state remains the stored `DeadAt` component rather than a duplicated `is_dead` derivation helper.

## Assumption Reassessment (2026-03-11)

1. `WoundList.wounds` is a `Vec<Wound>` — confirmed.
2. `Wound.severity` is `Permille` — confirmed.
3. `CombatProfile` will have `wound_capacity` and `incapacitation_threshold` as `Permille` fields — per E12COMHEA-002.
4. `DeadAt` exists as a component after E12COMHEA-002, so this ticket should not add an `is_dead()` helper that aliases stored death state.
5. Wound load = sum of all wound severities. Must handle potential overflow (sum of Permille values can exceed 1000).

## Architecture Check

1. Helper functions live on `WoundList` as methods or as free functions in the wounds module — keeps them co-located with the data they operate on.
2. These are pure derivation functions: `&WoundList` + `&CombatProfile` → bool/Permille. No side effects.
3. Wound load can exceed `Permille(1000)` since it's a sum of multiple wounds. Use `u32` for the sum, or a dedicated type. The comparison against `CombatProfile` thresholds (which are `Permille`) must handle this.
4. Bleeding detection belongs on `WoundList`, because it derives strictly from the contained wounds and keeps later wound-progression systems from reimplementing the same scan.

## What to Change

### 1. Add `wound_load()` method to `WoundList`

Returns the total severity across all wounds as a `u32` (sum of `Permille` inner values). Cannot be `Permille` since it may exceed 1000.

### 2. Add `is_incapacitated()` function

Takes `&WoundList` and `&CombatProfile`, returns `bool`. True when `wound_load >= incapacitation_threshold`.

### 3. Add `is_wound_load_fatal()` function

Takes `&WoundList` and `&CombatProfile`, returns `bool`. True when `wound_load >= wound_capacity`.

### 4. Add `has_bleeding_wounds()` method to `WoundList`

Returns `bool` — true if any wound has `bleed_rate_per_tick > Permille(0)`.

## Files to Touch

- `crates/worldwake-core/src/wounds.rs` (modify — add methods/functions)

## Out of Scope

- Death triggering logic (E12COMHEA-008)
- Wound progression / bleeding / clotting (E12COMHEA-009)
- Scheduler exclusion (E12COMHEA-008)
- Combat system tick (E12COMHEA-014)
- Any action definitions or handlers

## Acceptance Criteria

### Tests That Must Pass

1. `wound_load()` on empty `WoundList` returns 0
2. `wound_load()` correctly sums multiple wound severities
3. `wound_load()` can exceed 1000 (multiple wounds)
4. `is_incapacitated()` returns false when wound load < incapacitation_threshold
5. `is_incapacitated()` returns true when wound load >= incapacitation_threshold
6. `is_wound_load_fatal()` returns false when wound load < wound_capacity
7. `is_wound_load_fatal()` returns true when wound load >= wound_capacity
8. `has_bleeding_wounds()` returns false on empty list
9. `has_bleeding_wounds()` returns false when all wounds have `bleed_rate_per_tick = Permille(0)`
10. `has_bleeding_wounds()` returns true when any wound bleeds
11. Different `CombatProfile` values produce different incapacitation/death results
12. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. No stored state — these are pure derivation functions
2. No `f32`/`f64` — integer arithmetic only
3. Functions are deterministic (no RNG, no side effects)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/wounds.rs` — unit tests for all helper functions

### Commands

1. `cargo test -p worldwake-core -- wounds`
2. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - added `WoundList::wound_load() -> u32`
  - added `WoundList::has_bleeding_wounds() -> bool`
  - added `is_incapacitated(&WoundList, &CombatProfile) -> bool`
  - added `is_wound_load_fatal(&WoundList, &CombatProfile) -> bool`
  - exported the new wound helpers from `worldwake-core`
  - added direct unit coverage for empty, summed, bleeding, incapacitation, and fatality cases
- Deviations from original plan:
  - corrected the ticket to avoid adding an `is_dead()` helper; death is modeled by the stored `DeadAt` component, so a helper with the same semantic would have been redundant and architecturally weaker
- Verification results:
  - `cargo test -p worldwake-core -- wounds` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
