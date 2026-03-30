# S41BANOFFEME-004: Suite 3 — Wound-Dampened Raid Spiral (Scenario 49)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `crates/worldwake-ai/src/candidate_generation.rs` (wound-aware raid suppression)
**Deps**: S41BANOFFEME-001 (spec reassessment — confirms engine gap), S41BANOFFEME-002 (Suite 1 must pass to confirm basic raid mechanics)

## Problem

No golden test validates FND-10 (Physical Dampeners) for any system. The spec claims wound accumulation should physically dampen raid frequency — bandits who repeatedly raid accumulate wounds, and at some threshold the wound load should suppress further raids. This is the critical feedback-dampening test for the bandit raid amplification loop.

**Engine gap**: As documented in S41BANOFFEME-001, the current code does NOT produce this dampening. See Assumption Reassessment below for the full analysis and the required engine change.

## Assumption Reassessment (2026-03-30)

### Current Architecture (What Exists)

1. **`ReduceDanger` priority** — derived from `danger_class` at ranking.rs:367–369. `danger_class` comes from `derive_danger_pressure()` (pressure.rs:69–71), which returns 0 when `current_attackers.is_empty() && visible_hostiles.is_empty()` (pressure.rs:28–29). **Wounds alone produce zero danger pressure.**
2. **Pain pressure** — `derive_pain_pressure()` (pressure.rs:61–67) sums wound severities. Only feeds `TreatWounds` priority (ranking.rs:370–377). Does not affect `ReduceDanger` or `RaidTarget`.
3. **RaidTarget suppression rule** — `SuppressionRule::Never` (goal_policy.rs:151–156). Raids are never stress-suppressed.
4. **RaidTarget priority class** — always `Medium` (ranking.rs:344). No wound modulation.
5. **RaidTarget candidate-generation guard** — suppressed only when `derive_danger_pressure() >= thresholds.danger.high()` (candidate_generation.rs:1437–1444). Requires visible hostiles or current attackers, not wounds.
6. **`flee_wound_threshold`** — exists on `BanditFactionPolicy` (worldwake-core), already stored per-faction. Currently unused in candidate generation or ranking. It was designed for exactly this purpose but never wired in.

### Required Engine Change

Add wound-aware raid suppression to `emit_raid_target_goals()` in `candidate_generation.rs`. The guard clause at lines 1437–1444 currently checks only danger pressure. Add a second guard:

```
If the agent is a member of a bandit faction with flee_wound_threshold,
AND derive_pain_pressure(view, agent) >= flee_wound_threshold * (1000 - courage) / 1000,
THEN skip raid candidate emission (with diagnostic trace).
```

This uses:
- `flee_wound_threshold` from `BanditFactionPolicy` — per-faction physical threshold
- `derive_pain_pressure()` — existing function summing wound severities
- `courage` from `UtilityProfile` — existing per-agent parameter modulating pain tolerance

The suppression is physical (FND-10): wounds are concrete stored state, pain is derived from wounds, and the threshold is a concrete agent/faction parameter. No numeric cooldown, no abstract fatigue counter.

### Arithmetic for the Golden Test

- `flee_wound_threshold: pm(300)` (from existing T22 bandit faction policy)
- `courage: pm(200)` (low courage)
- Effective threshold: `300 * (1000 - 200) / 1000 = 240 permille`
- Each combat against a weak target produces wounds with total severity ~100–200 permille (depending on combat resolution)
- After 2 combats: cumulative wound severity ~200–400 permille
- At ~240+ permille pain pressure, raid candidates are suppressed

**Survivability check**: Bandits with `CombatProfile` moderate skill vs. weak targets should survive 2–3 combats. The wound capacity (`wound_capacity` in `CombatProfile`) and bleed rates must allow the bandit to accumulate wounds without dying. With `bandit_profile()` having `wound_capacity: nz(4)` and targets being weak, 2–3 non-lethal victories are feasible.

### Scenario Isolation

- **Intended branch**: Wound accumulation suppresses raid candidate generation after 2+ combats.
- **Excluded lawful branches**: TreatWounds (no medicine available), ReduceDanger/flee (no visible hostiles post-combat), production (no workstation at camp for this suite). The only competing goal pressures are hunger-driven consumption and raid.
- **Why exclusions are valid**: The test isolates the physical dampening mechanism. Other dampeners (fleeing from hostiles, treating wounds) are tested elsewhere or are orthogonal.

## Architecture Check

1. The engine change is a ~10-line addition to `emit_raid_target_goals()`, adding a second guard clause after the existing danger-pressure guard. It reads `BanditFactionPolicy.flee_wound_threshold` via the belief view and `UtilityProfile.courage` — both already accessible in the generation context.
2. The mechanism uses existing stored state (`WoundList`, `BanditFactionPolicy`, `UtilityProfile`) and existing derived computations (`derive_pain_pressure()`). No new components, no new systems.
3. Adds a diagnostic trace entry when raid candidates are omitted due to wound load (follows the pattern of `BanditCandidateOmissionReason` already used for rally-belief omissions in T22).
4. No backwards-compatibility shims. The new guard clause is purely additive and does not affect non-bandit agents or bandits without the `flee_wound_threshold` field.

## Verification Layers

1. First raid succeeds with wounds → action trace: `"attack"` committed by bandit + `WoundList` non-empty after combat
2. Second raid occurs despite wounds → decision trace: `RaidTarget` still in `candidates.generated`, `selection.selected_goal` matches `RaidTarget`; ranking shows hunger motive comparison
3. Wound accumulation suppresses raids → decision trace / candidate-generation diagnostics: `RaidTarget` NOT in `candidates.generated` due to wound-load suppression; bandit selects alternative (consume, idle)
4. No numeric cap → wound-load suppression threshold is derived from `flee_wound_threshold * (1000 - courage) / 1000`, verified by inspecting `WoundList` total severity vs. computed threshold
5. Dampening is physical → no `BlockedIntentMemory` for `RaidTarget` exists; suppression is at candidate-generation layer, not blocked-intent filtering
6. Deterministic replay → `hash_world()` + `hash_event_log()` match across two runs with same seed

## What to Change

### 1. Engine: Add wound-aware raid suppression to `emit_raid_target_goals()`

In `crates/worldwake-ai/src/candidate_generation.rs`, after the existing danger-pressure guard (lines 1437–1444), add a second guard that checks pain pressure against the faction's `flee_wound_threshold` modulated by agent courage. Add a new `BanditCandidateOmissionReason::WoundLoadExceedsThreshold` variant for diagnostic tracing.

### 2. Add Suite 3 topology builder

`build_s49_topology()` — 2-place topology: BanditCamp, RoadJunction. Minimal topology to isolate wound accumulation.

### 3. Add Suite 3 setup function

`seed_s49_scenario(h: &mut GoldenHarness) -> S49Ids`:
- 2 bandits at BanditCamp with `BanditFactionPolicy` (`flee_wound_threshold: pm(300)`), `CombatProfile` (moderate — wins but sustains wounds, low guard), `HomeostaticNeeds` (chronic hunger), `MetabolismProfile` (non-zero hunger_rate), `PerceptionProfile`, `UtilityProfile` (`danger_weight >= pm(700)`, `courage <= pm(200)`, `enterprise_weight >= pm(300)`)
- Active `BanditCamp` with minimal supplies
- NO healing items, medicine, or `TreatWounds` capability
- 2–3 non-faction targets arriving sequentially (placed at BanditCamp or traveling in), with weak `CombatProfile`

### 4. Add `run_s49_scenario(seed: Seed)` function

Multi-phase tick loop:
1. Phase 1: First target arrives → bandit raids → combat → wounds sustained → loot
2. Phase 2: Second target arrives → bandit raids despite wounds (hunger still dominates) → more wounds
3. Phase 3: Third target arrives → bandit does NOT raid (wound load exceeds threshold) → selects alternative goal
4. Accumulate assertion flags per phase, assert wound progression and behavioral shift

### 5. Add two test functions

- `golden_wound_dampened_raid_spiral` — calls `run_s49_scenario(Seed([49; 32]))`
- `golden_wound_dampened_raid_spiral_replays_deterministically` — calls twice, asserts hash equality

### 6. Add focused unit test for wound-aware suppression

In `crates/worldwake-ai/src/candidate_generation.rs` tests module, add a focused test:
- Bandit with high wound load + faction `flee_wound_threshold` + low courage → `RaidTarget` NOT emitted
- Same bandit with low wound load → `RaidTarget` emitted
- Non-bandit agent with same wounds → `RaidTarget` not relevant (not a bandit)

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add wound-aware guard clause + focused test)
- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (modify — add Suite 3 tests)

## Out of Scope

- Changes to `worldwake-core` (the `flee_wound_threshold` field already exists on `BanditFactionPolicy`)
- Changes to `worldwake-sim` or `worldwake-systems`
- Modifying ranking.rs or goal_policy.rs — the suppression is at candidate-generation, not ranking
- Modifying existing T22 or Suite 1/2 tests
- Golden inventory updates (S41BANOFFEME-005)
- Adding wound recovery / healing mechanics

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_wound_dampened_raid_spiral -- --exact` — main test passes
2. `cargo test -p worldwake-ai golden_wound_dampened_raid_spiral_replays_deterministically -- --exact` — replay test passes
3. New focused test in `candidate_generation.rs` for wound-aware suppression passes
4. `cargo test -p worldwake-ai` — all existing golden tests still pass (no regressions)
5. `cargo test --workspace` — no regressions workspace-wide from the engine change

### Invariants

1. Wound-based raid suppression uses concrete stored state (`WoundList` severity sum) and per-faction/per-agent parameters (`flee_wound_threshold`, `courage`), not a numeric cooldown or abstract fatigue counter (FND-10).
2. No `BlockedIntentMemory` entry for `RaidTarget` is created — suppression is at candidate generation, not blocked-intent filtering.
3. Non-bandit agents are unaffected by the new guard clause.
4. Bandits without wounds or with wounds below threshold still generate `RaidTarget` candidates normally.
5. Deterministic replay produces identical `StateHash` for world and event log.

## Test Plan

### New/Modified Tests

1. `golden_wound_dampened_raid_spiral` — proves wound accumulation physically dampens raid frequency across 2–3 combat encounters
2. `golden_wound_dampened_raid_spiral_replays_deterministically` — proves deterministic replay invariant for Suite 3
3. Focused unit test in `candidate_generation.rs` — proves wound-aware suppression guard clause in isolation

### Commands

1. `cargo test -p worldwake-ai golden_wound_dampened_raid_spiral` — targeted suite
2. `cargo test -p worldwake-ai` — full AI crate regression
3. `cargo test --workspace` — full workspace regression (engine change)
4. `cargo clippy --workspace` — no warnings
