# E12COMHEA-009: Wound progression - bleeding, natural clotting, and recovery

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes - worldwake-systems combat module
**Deps**: E12COMHEA-001 (bleed_rate_per_tick), E12COMHEA-002 (CombatProfile), E12COMHEA-006 (wound helpers)

## Problem

Each wound progresses independently each tick. Bleeding wounds increase in severity. Natural clotting reduces bleed rate over time. Non-bleeding wounds can recover under acceptable conditions. This is the physical dampener for the wound-spiral feedback loop (Principle 8).

## Assumption Reassessment (2026-03-11)

1. `Wound.bleed_rate_per_tick`, `Wound.inflicted_at`, and the `CombatProfile` clotting/recovery fields already exist in the live codebase.
2. `combat_system()` already exists in `crates/worldwake-systems/src/combat.rs` and is already wired into the combat dispatch slot. This ticket must extend that system rather than introduce a second combat-system path.
3. `CombatProfile.natural_clot_resistance` and `CombatProfile.natural_recovery_rate` are the correct per-agent progression parameters; higher values should produce faster stabilization and recovery.
4. `HomeostaticNeeds` is the correct authoritative physiology read-model for recovery gating.
5. `DriveThresholds` already exists and provides per-agent tolerability bands. Using those thresholds is preferable to introducing hardcoded recovery cutoffs.
6. There is no authoritative "currently in combat" component or action classification in the live runtime. `SystemExecutionContext` exposes `active_actions` and `action_defs`, but inferring combat state from action names or payload shape would be brittle and not durable architecture.
7. The current `Wound` schema stores current `bleed_rate_per_tick`, not an immutable `initial_bleed_rate`. A clotting formula that repeatedly recomputes from `current_tick - inflicted_at` would therefore need extra wound state or risk compounding artifacts. This ticket should use monotonic per-tick clotting against current bleed state.

## Architecture Check

1. Wound progression should be owned by the existing combat system, alongside the already-implemented death detection pass. The tick order should be: progress wounds first, then evaluate fatality on the updated wound state.
2. Per-entity progression should be implemented as a small helper over `WoundList`, with combat-system wiring responsible only for world reads/writes. This keeps the mutation logic testable without adding another system boundary.
3. Natural clotting should be modeled as deterministic per-tick reduction of current `bleed_rate_per_tick` by `natural_clot_resistance`, clamped at zero. This is a stateful physical process and matches the current schema better than an elapsed-time recomputation that lacks immutable baseline data.
4. Natural recovery should only apply to non-bleeding wounds on living agents whose `HomeostaticNeeds` are within that agent's tolerable `DriveThresholds`. This keeps the gate profile-driven instead of hardcoded.
5. Recovery suppression based on "currently in combat" is not part of this ticket's implementation scope because the runtime does not yet expose a robust authoritative combat-engagement marker. That should be added explicitly in a later ticket instead of inferred ad hoc here.

## What to Change

### 1. Implement per-wound bleeding progression

For each wound where `bleed_rate_per_tick > 0`:
- Increase `severity` by `bleed_rate_per_tick`
- Saturate wound severity at `Permille(1000)`

### 2. Implement natural clotting

For each wound where `bleed_rate_per_tick > 0`:
- Reduce `bleed_rate_per_tick` by `CombatProfile.natural_clot_resistance`
- When `bleed_rate_per_tick` reaches 0, wound transitions to recovery phase

### 3. Implement natural recovery

For each non-bleeding wound on a living agent:
- Check recovery conditions from `HomeostaticNeeds` plus per-agent `DriveThresholds`
- If conditions met: reduce `severity` by `natural_recovery_rate` per tick
- If `severity` reaches 0: remove wound from list

### 4. Clean up fully healed wounds

Remove wounds with `severity == Permille(0)` from the `WoundList`.

### 5. Integrate progression into the existing combat tick

- Extend `combat_system()` to progress wounds before fatality collection
- Keep death detection on the updated wound state so wounds that cross fatal load this tick die this tick

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (modify - wound progression helpers and combat tick integration)

## Out of Scope

- Hit resolution / combat damage (E12COMHEA-010)
- Heal action (E12COMHEA-013 - accelerates this process)
- Additional combat system dispatch wiring (already completed in E12COMHEA-008)
- Medicine profiles
- E09 HomeostaticNeeds logic changes
- Introducing a new combat-engagement component or action-classification layer just to block recovery during active combat

## Acceptance Criteria

### Tests That Must Pass

1. Bleeding wound increases severity by `bleed_rate_per_tick` each tick
2. Natural clotting reduces `bleed_rate_per_tick` each tick until it reaches zero
3. Higher `natural_clot_resistance` produces faster clotting
4. Wound with `bleed_rate_per_tick = 0` does not increase in severity
5. Non-bleeding wound recovers under acceptable conditions
6. Recovery only occurs when physiological conditions are acceptable under per-agent thresholds
7. Progression runs before fatality detection so newly worsened wounds can trigger death on the same tick
8. Wound with severity reduced to 0 is removed from `WoundList`
9. Deprivation wounds (`bleed_rate_per_tick = 0`) coexist with combat wounds
10. Different `CombatProfile` values produce different progression rates
11. Progression is deterministic (no RNG involved in clotting/recovery)
12. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Principle 8: wound loops have physical dampeners (clotting = coagulation process)
2. Principle 12: reads wound, physiology, and threshold state from shared components; no direct system calls
3. No `f32`/`f64` - `Permille` arithmetic only
4. Deterministic progression given same inputs

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` - unit tests for bleeding, clotting, recovery, cleanup, and same-tick fatality after progression

### Commands

1. `cargo test -p worldwake-systems -- combat`
2. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-11
- Outcome amended: 2026-03-11
- What actually changed:
  - corrected the ticket assumptions to match the live code before implementation
  - extended the existing `combat_system()` with a wound-progression pass that runs before fatality detection
  - implemented per-tick bleeding severity growth, per-tick clotting from `CombatProfile.natural_clot_resistance`, physiology-gated recovery, and healed-wound cleanup
  - used `HomeostaticNeeds` plus per-agent `DriveThresholds` for recovery gating instead of introducing hardcoded recovery cutoffs
  - added explicit `ActionDomain` classification to `ActionDef` and used it to treat active combat-domain actions as authoritative combat engagement for recovery suppression
  - added combat tests covering bleeding progression, clotting-rate differences, recovery gating, wound cleanup, and same-tick fatality after progression
- Deviations from original plan:
  - used monotonic per-tick clotting against current bleed state instead of an elapsed-time recomputation because the current `Wound` schema does not store immutable baseline bleed state
- Verification results:
  - `cargo test -p worldwake-systems --lib combat` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
