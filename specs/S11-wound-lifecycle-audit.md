**Status**: PENDING

# Wound Lifecycle Audit

## Summary

Investigate and fix an observed wound lifecycle anomaly: wounds with `natural_recovery_rate: pm(0)` on the agent's `CombatProfile` were observed to disappear (wound_load drops to 0, wound pruned from `WoundList`) when they should have persisted indefinitely. This either indicates an undocumented pruning mechanism, an arithmetic edge case in the bleed→clot→recovery pipeline, or a component-override ordering issue.

This is an investigation-first spec. The deliverables are diagnostic tests that reproduce the anomaly and a targeted fix once the root cause is identified.

## Discovered Via

Golden E2E emergent tests (S07 care interaction coverage). A fighter wounded during combat had `natural_recovery_rate: pm(0)` set via combat profile override, yet `wound_load` returned to 0 within ~100 ticks. The wound was observed (`fighter_wounded=true`) at an intermediate tick but absent at the scenario's end.

## Foundation Alignment

- **Principle 3** (Concrete State Over Abstract Scores): Wounds are concrete state. If wound state evolves unpredictably — disappearing despite zero recovery rate — agents cannot make reliable decisions about care, danger, or retreat. The wound system must be fully deterministic and traceable.
- **Principle 9** (Outcomes Are Granular and Leave Aftermath): Combat wounds are aftermath. If they silently vanish, the aftermath is lost and downstream emergence (care goals, danger pressure, loot priority) breaks.

## Phase

Phase 3: Information & Politics (investigation, no phase dependency)

## Crates

- `worldwake-core` (wound types, `WoundList`)
- `worldwake-systems` (wound processing in `needs_system`, combat wound creation)

## Dependencies

None. This is an investigation of existing code.

## Hypotheses

### H1: Wound pruning has a severity floor

The wound system may prune wounds when severity reaches 0 or falls below some epsilon, even if `natural_recovery_rate` is 0. If the bleed→clot cycle causes severity to transiently reach 0 through arithmetic (e.g., severity starts at 150, bleeds for 2 ticks adding ~35, clots, then... somehow decreases), the wound could be pruned.

**Test**: Create a wound with severity pm(200), bleed_rate pm(0) (already clotted), recovery_rate pm(0). Tick 50 times. Assert severity unchanged and wound not pruned.

### H2: Bleed/clot arithmetic underflow

The bleed rate decreases by `natural_clot_resistance` per tick. If clot resistance is applied to severity instead of bleed_rate, or if there's a subtraction ordering issue, severity could decrease when it shouldn't.

**Test**: Create a wound with severity pm(100), bleed_rate pm(50), clot_resistance pm(25), recovery_rate pm(0). Tick until bleed_rate reaches 0. Assert severity equals initial + total_bleed_accumulated. Assert severity does not decrease after clotting.

### H3: Combat profile override not taking effect

`seed_agent_with_recipes` sets `default_combat_profile()` (recovery_rate pm(18)). The test then calls `set_component_combat_profile` in a separate transaction to override. If the component table merge has ordering issues, the override might not persist.

**Test**: Set combat profile via override transaction, then read it back immediately. Assert recovery_rate equals the overridden value.

### H4: Wound system has minimum recovery rate

The wound processing code may clamp `natural_recovery_rate` to a minimum of 1 or apply recovery regardless of the profile value.

**Test**: Inspect wound system source code for recovery application logic. Verify it respects pm(0) as "no recovery."

## Deliverables

### 1. Diagnostic Unit Tests

Add focused unit tests in `crates/worldwake-systems/tests/` (or inline in `needs.rs`) that isolate each hypothesis:

- **`wound_persists_with_zero_recovery_rate`**: Clotted wound, pm(0) recovery. Tick N times. Assert wound unchanged.
- **`wound_bleed_clot_arithmetic_is_exact`**: Bleeding wound, known parameters. Assert severity after clotting equals expected value.
- **`combat_profile_override_takes_effect`**: Override profile in separate transaction. Read back. Assert match.
- **`wound_pruning_threshold_is_zero_severity`**: Wound at severity pm(1), recovery pm(1). After 1 tick, severity should be pm(0) and wound pruned. Wound at severity pm(1), recovery pm(0) — wound should NOT be pruned.

### 2. Root Cause Fix

Once a hypothesis is confirmed, apply the minimal fix:

- **H1 confirmed**: If pruning triggers at severity > 0, fix the pruning threshold to be exactly `severity == pm(0)`.
- **H2 confirmed**: Fix the arithmetic ordering in the wound tick processing.
- **H3 confirmed**: Document or fix the component override ordering guarantee.
- **H4 confirmed**: Remove the minimum recovery floor, or if it exists for a valid reason, document it and adjust `CombatProfile` semantics.

### 3. Golden Test Cleanup

If the root cause is fixed, update `golden_emergent.rs` to remove the `no_recovery_combat_profile()` workaround where it was applied solely to avoid this anomaly (vs. where it serves the test's design intent of ensuring wounds only heal through medicine).

## Risks

Low. This is an investigation with targeted fixes. No architectural changes required — the wound system's design is sound, only its implementation may have an edge case.

## Information-Path Analysis (FND-01 Section H)

Not applicable — wound processing is local per-agent state manipulation, not information flow.

## Stored State vs. Derived

- **Stored**: `WoundList` (authoritative per-agent wound state), `CombatProfile` (authoritative per-agent combat parameters)
- **Derived**: `wound_load()` (sum of wound severities — derived read-only accessor)
