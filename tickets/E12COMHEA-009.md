# E12COMHEA-009: Wound progression — bleeding, natural clotting, and recovery

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-systems combat module
**Deps**: E12COMHEA-001 (bleed_rate_per_tick), E12COMHEA-002 (CombatProfile), E12COMHEA-006 (wound helpers)

## Problem

Each wound progresses independently each tick. Bleeding wounds increase in severity. Natural clotting reduces bleed rate over time. Non-bleeding wounds can recover under acceptable conditions. This is the physical dampener for the wound-spiral feedback loop (Principle 8).

## Assumption Reassessment (2026-03-11)

1. `Wound.bleed_rate_per_tick` will be `Permille` after E12COMHEA-001 — confirmed by spec.
2. `Wound.inflicted_at` is `Tick` — elapsed time = `current_tick - inflicted_at`.
3. `CombatProfile.natural_clot_resistance` controls clotting speed — higher = faster clotting.
4. `CombatProfile.natural_recovery_rate` controls severity reduction per tick under recovery conditions.
5. Recovery requires: not bleeding, alive, not in combat, acceptable hunger/thirst/fatigue.
6. Checking "not in combat" requires querying active actions — state-mediated per Principle 12.
7. Checking acceptable physiology requires reading `HomeostaticNeeds` — state-mediated per Principle 12.

## Architecture Check

1. Wound progression runs per-tick for all living agents with wounds, implemented as a function callable from the combat system tick.
2. Per-wound processing: iterate `WoundList.wounds`, modify each wound's `bleed_rate_per_tick` and `severity` in place.
3. Natural clotting formula: `bleed_rate_per_tick` decreases based on elapsed time and `natural_clot_resistance`. This models blood coagulation (physical process, not a numerical clamp per Principle 8).
4. Recovery formula: if not bleeding and conditions met, `severity` decreases by `natural_recovery_rate` per tick, floored at 0. Remove wounds with severity 0.

## What to Change

### 1. Implement per-wound bleeding progression

For each wound where `bleed_rate_per_tick > 0`:
- Increase `severity` by `bleed_rate_per_tick`
- Cap severity increase at `wound_capacity` (via wound load check later)

### 2. Implement natural clotting

For each wound where `bleed_rate_per_tick > 0`:
- Calculate elapsed ticks: `current_tick - inflicted_at`
- Reduce `bleed_rate_per_tick` based on elapsed time and `natural_clot_resistance`
- When `bleed_rate_per_tick` reaches 0, wound transitions to recovery phase

### 3. Implement natural recovery

For each non-bleeding wound on a living agent:
- Check recovery conditions (not in active combat, acceptable physiology)
- If conditions met: reduce `severity` by `natural_recovery_rate` per tick
- If `severity` reaches 0: remove wound from list

### 4. Clean up fully healed wounds

Remove wounds with `severity == Permille(0)` from the `WoundList`.

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (new or modify — wound progression functions)

## Out of Scope

- Hit resolution / combat damage (E12COMHEA-010)
- Death detection (E12COMHEA-008 — called after progression)
- Heal action (E12COMHEA-013 — accelerates this process)
- Combat system tick wiring (E12COMHEA-014)
- Medicine profiles
- E09 HomeostaticNeeds logic changes

## Acceptance Criteria

### Tests That Must Pass

1. Bleeding wound increases severity by `bleed_rate_per_tick` each tick
2. Natural clotting reduces `bleed_rate_per_tick` over elapsed time
3. Higher `natural_clot_resistance` produces faster clotting
4. Wound with `bleed_rate_per_tick = 0` does not increase in severity
5. Non-bleeding wound recovers under acceptable conditions
6. Recovery only occurs when agent is not actively in combat
7. Recovery only occurs when physiological conditions are acceptable
8. Wound with severity reduced to 0 is removed from WoundList
9. Deprivation wounds (bleed_rate = 0) coexist with combat wounds
10. Different `CombatProfile` values produce different progression rates
11. Progression is deterministic (no RNG involved in clotting/recovery)
12. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Principle 8: wound loops have physical dampeners (clotting = coagulation process)
2. Principle 12: reads HomeostaticNeeds and active actions from shared state, no direct system calls
3. No `f32`/`f64` — `Permille` arithmetic only
4. Deterministic progression given same inputs

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` — unit tests for bleeding, clotting, recovery

### Commands

1. `cargo test -p worldwake-systems -- combat`
2. `cargo test --workspace && cargo clippy --workspace`
