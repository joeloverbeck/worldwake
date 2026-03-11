# E13DECARC-006: Derived pressure functions (pain, danger)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — AI-layer pure functions
**Deps**: E13DECARC-005

## Problem

Pain and danger are transient derived values, never stored as authoritative state. The decision architecture needs pure functions that compute them each tick from current beliefs. Danger derivation must be monotone and local, using only believed hostile/attacker evidence.

## Assumption Reassessment (2026-03-11)

1. `Wound.severity` is `Permille` — confirmed in `worldwake-core::wounds`.
2. `DriveThresholds` has `pain` and `danger` fields, each a `ThresholdBand` — confirmed.
3. `ThresholdBand` exposes `.low()`, `.medium()`, `.high()`, `.critical()` — confirmed.
4. `BeliefView` already has `wounds()`, `visible_hostiles_for()`, `current_attackers_of()`, `has_wounds()`, and `is_incapacitated()` in `worldwake-sim::belief_view`; this ticket does not need to add or wait on those APIs.
5. `OmniscientBeliefView` already implements those belief queries with the locality semantics described in the E13 spec.
6. `worldwake-ai` already depends on `worldwake-sim`; this ticket does not need any Cargo dependency change.
7. `Permille::new_unchecked()` exists and is the appropriate zero-allocation constructor for fixed literals in this layer.
8. `worldwake-ai/src/pressure.rs` does not exist yet. This ticket should add that module rather than treat it as an existing stub.

## Architecture Check

1. These are pure functions `(view, agent) -> Permille`, not stored components. Correct per Principle 3.
2. Danger derivation uses monotone bands from the agent's own `DriveThresholds`, not arbitrary numbers.
3. No stored "fear scalar" or "danger score" — derived fresh each decision pass.
4. Centralizing these derivations in one AI module is better than duplicating threshold math across future candidate generation and ranking code. It improves determinism and keeps E13 extensible without introducing a new abstract state layer.
5. This ticket should stay narrow. Do not generalize into a trait-heavy "pressure engine" before more derived pressures exist; that would add indirection without present benefit.

## What to Change

### 1. Implement pressure derivation in `worldwake-ai/src/pressure.rs`

```rust
pub fn derive_pain_pressure(view: &dyn BeliefView, agent: EntityId) -> Permille {
    // sum all wound severities, cap at Permille(1000)
}

pub fn derive_danger_pressure(
    view: &dyn BeliefView,
    agent: EntityId,
) -> Permille {
    // Uses thresholds from view.drive_thresholds(agent)
    // no hostiles and no attackers -> Permille(0)
    // hostile presence without active attack -> at least danger medium band
    // active attacker present -> at least danger high band
    // multiple attackers, or any attacker while wounded/incapacitated -> at least danger critical band
}
```

### 2. Add band classification helper

```rust
pub fn classify_band(value: Permille, band: &ThresholdBand) -> GoalPriorityClass {
    // value >= critical -> Critical
    // value >= high -> High
    // value >= medium -> Medium
    // value >= low -> Low
    // else -> Background
}
```

This maps any pressure + threshold band to a `GoalPriorityClass` for use in candidate ranking.

## Files to Touch

- `crates/worldwake-ai/src/pressure.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — export the new module and helpers)

## Out of Scope

- Storing pain or danger as components (they are derived, never stored)
- Homeostatic pressure derivation (those already exist as `HomeostaticNeeds` fields)
- Candidate generation logic — E13DECARC-007
- Enterprise opportunity signal derivation — E13DECARC-008
- Any change to `BeliefView`, `OmniscientBeliefView`, or `worldwake-sim` threat-locality behavior unless implementation proves the current APIs are insufficient

## Acceptance Criteria

### Tests That Must Pass

1. `derive_pain_pressure()` returns `Permille(0)` for agent with no wounds
2. `derive_pain_pressure()` returns sum of wound severities (e.g., two wounds at 300 each = 600)
3. `derive_pain_pressure()` caps at `Permille(1000)` when wounds exceed it
4. `derive_danger_pressure()` returns `Permille(0)` when no hostiles and no attackers
5. `derive_danger_pressure()` returns at least danger medium band when hostiles present but no attackers
6. `derive_danger_pressure()` returns at least danger high band when one attacker present
7. `derive_danger_pressure()` returns at least danger critical band when multiple attackers present
8. `derive_danger_pressure()` returns at least danger critical band when one attacker and agent is wounded
9. `derive_danger_pressure()` returns `Permille(0)` when agent has no `DriveThresholds`
10. `classify_band()` correctly maps values to priority classes against a threshold band
11. `worldwake-ai` publicly re-exports the new pressure helpers needed by follow-up E13 tickets
12. Existing suite: `cargo test --workspace`

### Invariants

1. Pain and danger are never stored as authoritative state
2. Danger derivation is monotone: more threat -> equal or higher danger
3. Danger derivation is local: uses only `visible_hostiles_for()` and `current_attackers_of()`, never global queries
4. No floats — all `Permille` arithmetic

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/pressure.rs` — new unit tests using a minimal mock `BeliefView`
2. `crates/worldwake-ai/src/lib.rs` — update crate-surface test only if needed for re-exports

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Added `crates/worldwake-ai/src/pressure.rs` with pure `derive_pain_pressure()`, `derive_danger_pressure()`, and `classify_band()` helpers.
  - Re-exported the pressure helpers from `crates/worldwake-ai/src/lib.rs` for follow-up E13 work.
  - Added focused unit coverage for pain aggregation, danger band derivation, wounded/incapacitated escalation, and threshold-band classification.
  - Corrected this ticket's assumptions before implementation to match the current codebase: `BeliefView` and `OmniscientBeliefView` already had the required APIs, `worldwake-ai` already depended on `worldwake-sim`, and `pressure.rs` had to be added rather than modified.
- Deviations from original plan:
  - No `worldwake-sim` or Cargo dependency changes were needed.
  - No pre-existing stub file was modified; the pressure module was created fresh.
  - The work stayed narrowly scoped to shared pure derivations rather than introducing a broader abstraction layer.
- Verification results:
  - `cargo test -p worldwake-ai` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
