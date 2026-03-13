# GOLDE2E-015: Wound Bleed → Clotting → Natural Recovery

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None (all engine code exists)

## Problem

The multi-condition wound recovery gate in `combat.rs:229-245` (`recovery_conditions_met()`) is never exercised end-to-end. This function gates recovery on hunger < high, thirst < high, fatigue < high, AND not engaged in combat — a 4-condition AND that could silently regress without detection. The full bleed → clot → recover severity curve has no golden-level proof.

## Assumption Reassessment (2026-03-13)

1. `tick_wounds()` at `combat.rs:192-227` processes each wound: if `bleed_rate > 0`, severity increases by bleed_rate and bleed_rate decreases by `natural_clot_resistance`; once bleed_rate reaches 0 and `can_recover` is true, severity decreases by `natural_recovery_rate`. Confirmed in code.
2. `recovery_conditions_met()` at `combat.rs:229-245` requires: not in combat, needs and thresholds present, hunger < high, thirst < high, fatigue < high. Confirmed in code.
3. Wounds with severity reaching 0 are pruned via `retain()` at `combat.rs:223`. Confirmed.
4. `CombatProfile` contains `natural_clot_resistance: Permille` and `natural_recovery_rate: Permille`. Confirmed in `crates/worldwake-core/src/combat.rs`.
5. No existing golden test exercises the bleed→clot→recover curve. Confirmed by coverage report Part 2 ("Wound bleed → clotting → natural recovery: **No**").

## Architecture Check

1. This test requires zero new engine code. It uses the existing `run_combat_system()` tick, `CombatProfile` parameters, and `WoundList` component. The scenario validates an existing multi-condition code path through emergent behavior, not through unit-level mocking.
2. No backwards-compatibility aliasing or shims introduced.

## What to Change

### 1. New golden test in `golden_combat.rs`

Add `golden_wound_bleed_clotting_natural_recovery` test:

**Setup**:
- Single agent at Village Square, well-fed/hydrated/rested (all needs at pm(0) or very low, well below high thresholds).
- Slow metabolism so needs stay below high thresholds for the entire observation window.
- `CombatProfile` with high `wound_capacity` (e.g. pm(900)) to survive the bleed phase, meaningful `natural_clot_resistance` (e.g. pm(20)/tick so a pm(100) bleed clots in 5 ticks), and meaningful `natural_recovery_rate` (e.g. pm(15)/tick).
- Apply a single bleeding wound via `WoundList` component: initial severity pm(50), bleed_rate pm(100).
- Food/water available locally to keep needs satisfied if metabolism ticks push them up.

**Assertions** (observe over ~60-80 ticks):
1. **Bleed phase**: Wound severity increases over the first ticks (bleed_rate > 0 adds to severity each tick).
2. **Clotting**: `bleed_rate` decreases each tick by `natural_clot_resistance` and eventually reaches pm(0).
3. **Recovery phase**: Once bleed_rate is 0 and recovery conditions are met, severity decreases each tick.
4. **Full recovery or significant reduction**: Severity reaches pm(0) (wound pruned) or is substantially lower than peak.
5. **Conservation**: If any items are present, lot conservation holds every tick.

### 2. Companion deterministic replay test

Add `golden_wound_bleed_clotting_natural_recovery_replays_deterministically`:
- Run the same scenario twice with the same seed.
- Assert identical world and event-log hashes.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add 2 tests)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add helper if needed for wound injection, or use existing component APIs)

## Out of Scope

- Recovery blocked by combat engagement (unit-tested in `combat.rs`)
- Recovery blocked by high needs (unit-tested in `combat.rs`)
- Multi-wound interactions
- Deprivation wound infliction (already proven in Scenario 8)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_wound_bleed_clotting_natural_recovery` — severity rises (bleed), bleed_rate falls (clot), severity falls (recover)
2. `golden_wound_bleed_clotting_natural_recovery_replays_deterministically` — identical hashes across two runs
3. Existing suite: `cargo test -p worldwake-ai --test golden_combat`

### Invariants

1. Conservation holds every tick if items are present
2. Wound is pruned from `WoundList` once severity reaches pm(0)
3. Recovery does not begin until bleed_rate reaches pm(0)
4. Agent remains alive throughout (wound_capacity never exceeded)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs::golden_wound_bleed_clotting_natural_recovery` — proves the full bleed→clot→recover severity curve through the real combat system tick
2. `crates/worldwake-ai/tests/golden_combat.rs::golden_wound_bleed_clotting_natural_recovery_replays_deterministically` — deterministic replay fidelity for the wound recovery path

### Commands

1. `cargo test -p worldwake-ai --test golden_combat -- golden_wound_bleed`
2. `cargo test --workspace && cargo clippy --workspace`
