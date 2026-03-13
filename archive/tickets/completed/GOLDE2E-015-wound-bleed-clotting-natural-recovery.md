# GOLDE2E-015: Wound Bleed → Clotting → Natural Recovery

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None (all engine code exists)

## Problem

The multi-condition wound recovery gate in `crates/worldwake-systems/src/combat.rs:229-245` (`recovery_conditions_met()`) is not exercised by the golden suite. This function gates recovery on hunger < high, thirst < high, fatigue < high, and not being engaged in combat. The full bleed → clot → recover severity curve is covered by focused combat-system tests, but it still lacks golden-level proof through the real AI/scheduler/system stack.

## Assumption Reassessment (2026-03-13)

1. `progress_wounds()` at `crates/worldwake-systems/src/combat.rs:192-227` processes each wound: if `bleed_rate > 0`, severity increases by bleed rate and bleed rate decreases by `natural_clot_resistance`; once bleed rate reaches 0 and `can_recover` is true, severity decreases by `natural_recovery_rate`. Confirmed in code.
2. `recovery_conditions_met()` at `crates/worldwake-systems/src/combat.rs:229-245` requires: not in combat, needs and thresholds present, hunger < high, thirst < high, fatigue < high. Confirmed in code.
3. Wounds with severity reaching 0 are pruned via `retain()` in `progress_wounds()`. Confirmed.
4. `CombatProfile` contains `natural_clot_resistance: Permille` and `natural_recovery_rate: Permille`. Confirmed in `crates/worldwake-core/src/combat.rs`.
5. Lower-layer coverage already exists in `crates/worldwake-systems/src/combat.rs` for: bleeding progression + clotting, faster clotting under higher resistance, non-bleeding recovery, and blocked recovery during active combat / high needs / missing thresholds. The missing coverage is specifically a golden end-to-end scenario that proves the combined curve under the real tick loop.
6. No existing golden test exercises the bleed→clot→recover curve. Confirmed by coverage report Part 2 ("Wound bleed → clotting → natural recovery: **No**").

## Architecture Check

1. This should remain test-only work. The current wound architecture is already clean: concrete wound state lives in `WoundList`, progression is authoritative in the combat system, and recovery depends on concrete physiology + active combat state rather than a special recovery subsystem.
2. The beneficial change is adding a golden proof for the existing architecture, not changing the architecture. A new engine abstraction, alias path, or recovery-specific helper would make the design worse by duplicating a path already modeled cleanly in authoritative state.
3. The scenario should avoid unrelated setup. It does not need local food/water, custom action scripting, or new harness abstractions unless a real gap appears while implementing.
4. No backwards-compatibility aliasing or shims introduced.

## What to Change

### 1. New golden test in `golden_combat.rs`

Add `golden_wound_bleed_clotting_natural_recovery` test:

**Setup**:
- Single agent at Village Square, well-fed/hydrated/rested (all needs at pm(0) or very low, well below high thresholds).
- Default metabolism is already sufficient to keep hunger/thirst/fatigue below their default `high()` thresholds over the expected observation window. Do not add local food/water unless implementation proves it is necessary.
- Use either the existing default combat profile from the golden harness or a minimal targeted override only if it makes the scenario materially clearer. Prefer the existing profile if it already yields a short deterministic bleed→clot→recover curve.
- Apply a single bleeding wound via `WoundList` component: initial severity pm(50), bleed_rate pm(100).

**Assertions** (observe over the minimum deterministic window that reaches recovery completion):
1. **Bleed phase**: Wound severity increases over the first ticks (bleed_rate > 0 adds to severity each tick).
2. **Clotting**: `bleed_rate` decreases each tick by `natural_clot_resistance` and eventually reaches pm(0).
3. **Recovery phase**: Recovery does not begin before bleed rate reaches pm(0); once clotting completes and recovery conditions remain true, severity decreases.
4. **Pruning**: Severity eventually reaches pm(0) and the wound is removed from `WoundList`.
5. **Safety**: Agent remains alive throughout.

### 2. Companion deterministic replay test

Add `golden_wound_bleed_clotting_natural_recovery_replays_deterministically`:
- Run the same scenario twice with the same seed.
- Assert identical world and event-log hashes.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add 2 tests)
- `reports/golden-e2e-coverage-analysis.md` (modify after implementation to remove the gap and record the new scenario)

`crates/worldwake-ai/tests/golden_harness/mod.rs` should stay untouched unless implementation reveals a real missing helper. Current component APIs appear sufficient.

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
4. Broader verification: `cargo test --workspace` and `cargo clippy --workspace`

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

## Outcome

- **Completion date**: 2026-03-13
- **What actually changed**:
  - Added `golden_wound_bleed_clotting_natural_recovery`
  - Added `golden_wound_bleed_clotting_natural_recovery_replays_deterministically`
  - Updated `reports/golden-e2e-coverage-analysis.md` to record the new proven scenario and remove the backlog gap
- **Deviations from original plan**:
  - No `golden_harness` changes were needed; existing component APIs were sufficient
  - The shipped scenario used the existing default metabolism/combat profile instead of extra local food/water scaffolding or custom profile overrides
  - The report update also corrected the architectural framing: the missing coverage was golden-only, not a lack of lower-layer wound tests
- **Verification results**:
  - `cargo test -p worldwake-ai --test golden_combat -- golden_wound_bleed` passed
  - `cargo test -p worldwake-ai --test golden_combat` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
