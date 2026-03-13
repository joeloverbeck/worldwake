# GOLDE2E-016: Loot/Bury Suppression Under Self-Care Pressure

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: None (all engine code exists)

## Problem

The `is_suppressed()` ranking filter at `ranking.rs:93-98` is never exercised in any golden test. This filter prevents starving/dehydrated agents from prioritizing loot or burial over self-care. A regression removing or weakening this filter would silently allow critically hungry agents to loot corpses instead of eating available food — breaking the emergent priority system. The suppression-then-lift ordering (eat first, then loot) has no end-to-end proof.

## Assumption Reassessment (2026-03-13)

1. `is_suppressed()` at `ranking.rs:93-98` matches `LootCorpse` and `BuryCorpse` goal kinds, returning true when `self_care_high_or_above()` or `danger_high_or_above()`. Confirmed in code.
2. `self_care_high_or_above()` at `ranking.rs:57-59` checks whether `max_self_care_class() >= GoalPriorityClass::High`. Confirmed in code.
3. `rank_candidates()` filters out suppressed candidates before ranking. Confirmed by `ranking.rs` flow.
4. Scenario 8 (`golden_death_cascade_and_opportunistic_loot`) proves loot works when the looter is healthy. But no test proves loot is *suppressed* when the agent has critical self-care needs. Confirmed by coverage report Part 2 ("Loot/bury suppression under self-care pressure: **No**").
5. The `LootCorpse` goal is emitted by candidate generation when a corpse with lootable items is co-located. Confirmed in `candidate_generation.rs`.

## Architecture Check

1. This test validates an existing ranking filter through emergent behavior. The scenario needs a critically hungry agent co-located with both food and a lootable corpse. The eat-before-loot ordering must emerge from the real AI ranking, not from manual action queueing. No new engine code required.
2. No backwards-compatibility aliasing or shims introduced.

## What to Change

### 1. New golden test in `golden_combat.rs`

Add `golden_loot_suppressed_under_self_care_pressure` test:

**Setup**:
- Two agents at Village Square: a living agent (Scavenger) and a dead agent (Victim).
- Scavenger is critically hungry (hunger at or above critical threshold).
- Scavenger has bread in inventory (or bread is locally available for immediate consumption).
- Victim (corpse) has coins that are lootable.
- Scavenger has no other competing high-priority needs (thirst/fatigue/bladder low).

**Assertions** (observe over ~40-60 ticks):
1. **Suppression phase**: The first self-care action started by the AI is `eat` (consuming bread), NOT `loot`. This proves `is_suppressed()` filtered the loot candidate while self-care was high.
2. **Suppression lift**: After hunger is relieved below the high threshold, the agent proceeds to loot the corpse within the observation window. This proves the suppression lifts once self-care drops below high.
3. **Ordering**: The eat action completes (or at least starts) before any loot action starts.
4. **Conservation**: Coin lot totals remain conserved throughout the scenario.

### 2. Companion deterministic replay test

Add `golden_loot_suppressed_under_self_care_pressure_replays_deterministically`:
- Run the same scenario twice with the same seed.
- Assert identical world and event-log hashes.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add 2 tests)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add helper if needed for corpse setup, or reuse existing death/corpse patterns from Scenario 8)

## Out of Scope

- Bury suppression under self-care (same `is_suppressed()` code path, redundant)
- Loot suppression under danger (requires active combat setup, different scenario)
- Multiple corpses or multiple lootable items
- Corpse creation mechanics (already proven in Scenario 8)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_loot_suppressed_under_self_care_pressure` — eat starts before loot; loot completes after hunger relief
2. `golden_loot_suppressed_under_self_care_pressure_replays_deterministically` — identical hashes across two runs
3. Existing suite: `cargo test -p worldwake-ai --test golden_combat`

### Invariants

1. Coin conservation holds every tick
2. `is_suppressed()` prevents `LootCorpse` from ranking while self-care is high or above
3. Suppression lifts once self-care drops below high threshold
4. Agent eats before looting — ordering is emergent, not manually sequenced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs::golden_loot_suppressed_under_self_care_pressure` — proves the eat-before-loot ordering emerges from the `is_suppressed()` ranking filter
2. `crates/worldwake-ai/tests/golden_combat.rs::golden_loot_suppressed_under_self_care_pressure_replays_deterministically` — deterministic replay fidelity for the suppression-then-lift path

### Commands

1. `cargo test -p worldwake-ai --test golden_combat -- golden_loot_suppressed`
2. `cargo test --workspace && cargo clippy --workspace`
