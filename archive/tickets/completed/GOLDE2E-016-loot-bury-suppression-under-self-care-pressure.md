# GOLDE2E-016: Loot/Bury Suppression Under Self-Care Pressure

**Status**: ✅ COMPLETED
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
4. Scenario 8 (`golden_death_cascade_and_opportunistic_loot`) proves loot works once a corpse exists, but it does not prove the suppression contract under competing self-care pressure. Confirmed by coverage report Part 2 ("Loot/bury suppression under self-care pressure: **No**").
5. `golden_bury_corpse` already seeds a dead agent directly by attaching `DeadAt(Tick(0))`. The suppression scenario does not need to reuse deprivation-death setup from Scenario 8; a pre-seeded corpse is the cleaner and narrower fixture.
6. The `LootCorpse` goal is emitted by candidate generation when a corpse with direct possessions is co-located. Confirmed in `candidate_generation.rs`.
7. Bread relieves `pm(260)` hunger per unit, while default hunger `high` is `pm(750)`. A scavenger starting at `pm(800)` hunger with one carried bread should cross from high to medium after eating, making the suppression-lift phase observable without extra engine changes.
8. A golden test can prove the externally visible behavior contract: no loot action starts while self-care remains high-or-above, then loot starts after hunger relief. It cannot directly prove in isolation that `is_suppressed()` rather than some other ranking effect caused that outcome, so the ticket must avoid overclaiming.

## Architecture Check

1. The current architecture is directionally sound: corpse interaction remains ordinary world-state-driven behavior, and self-care suppression is expressed in ranking rather than via special-case action denial. A golden should reinforce that contract without introducing new abstractions.
2. The narrowest durable proof is a pre-seeded corpse plus a hungry scavenger with carried bread. Reusing the deprivation-death cascade would couple this ticket to unrelated wound/death timing and make failures harder to localize.
3. The test should assert behavior, not internals: the scavenger begins `eat`, never begins `loot` while hunger is still high-or-above, then begins `loot` after hunger relief. No manual action queueing, no direct ranking pokes, no engine changes required.
4. No backwards-compatibility aliasing or shims introduced.

## What to Change

### 1. New golden test in `golden_combat.rs`

Add `golden_loot_suppressed_under_self_care_pressure` test:

**Setup**:
- Two agents at Village Square: a living agent (Scavenger) and a pre-seeded corpse.
- Scavenger starts at hunger `pm(800)` or another default-threshold value that is clearly high-but-subcritical, so one bread is enough to drop hunger below `high`.
- Scavenger has one carried bread for immediate `ConsumeOwnedCommodity`.
- Corpse has lootable coins in direct possession.
- Scavenger has no other competing high-priority needs (thirst/fatigue/bladder low).

**Assertions** (observe over ~20-40 ticks):
1. **Suppression phase**: While the scavenger's hunger remains at `high` or above, no `loot` action starts.
2. **Self-care first**: The scavenger begins `eat` before any `loot` action begins.
3. **Suppression lift**: After hunger falls below the `high` threshold, the scavenger begins `loot` within the observation window.
4. **Ordering**: The first observed `loot` start happens strictly after the first observed hunger-below-high tick.
4. **Conservation**: Coin lot totals remain conserved throughout the scenario.

### 2. Companion deterministic replay test

Add `golden_loot_suppressed_under_self_care_pressure_replays_deterministically`:
- Run the same scenario twice with the same seed.
- Assert identical world and event-log hashes.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add scenario helper + 2 tests)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify only if a reusable corpse-seeding helper becomes clearly beneficial; otherwise keep setup local to avoid growing the shared harness for a single scenario)

## Out of Scope

- Bury suppression under self-care (same `is_suppressed()` code path, redundant)
- Loot suppression under danger (requires active combat setup, different scenario)
- Multiple corpses or multiple lootable items
- Corpse creation mechanics (already proven in Scenario 8)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_loot_suppressed_under_self_care_pressure` — no loot starts while hunger is high-or-above; `eat` starts first; `loot` starts after hunger relief
2. `golden_loot_suppressed_under_self_care_pressure_replays_deterministically` — identical hashes across two runs
3. Existing suite: `cargo test -p worldwake-ai --test golden_combat`

### Invariants

1. Coin conservation holds every tick
2. Loot remains behaviorally unavailable while self-care is high or above
3. Suppression lifts once self-care drops below the high threshold
4. Agent eats before looting — ordering is emergent, not manually sequenced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs::golden_loot_suppressed_under_self_care_pressure` — proves the eat-before-loot ordering emerges from the `is_suppressed()` ranking filter
2. `crates/worldwake-ai/tests/golden_combat.rs::golden_loot_suppressed_under_self_care_pressure_replays_deterministically` — deterministic replay fidelity for the suppression-then-lift path

### Commands

1. `cargo test -p worldwake-ai --test golden_combat -- golden_loot_suppressed`
2. `cargo test --workspace && cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-13
- **What actually changed**:
  - Added `golden_loot_suppressed_under_self_care_pressure`
  - Added `golden_loot_suppressed_under_self_care_pressure_replays_deterministically`
  - Updated `reports/golden-e2e-coverage-analysis.md` to record the scenario as proven and remove the remaining backlog gap
- **Deviations from original plan**:
  - The shipped scenario seeds a corpse directly instead of reusing the deprivation-death cascade, keeping the proof focused on loot suppression rather than corpse creation timing
  - No `golden_harness` changes were needed; the setup stayed local to `golden_combat.rs`
  - The assertions prove the externally visible contract (`no loot while hunger is high`, then loot after relief) rather than claiming direct isolated proof of the internal ranking helper
- **Verification results**:
  - `cargo test -p worldwake-ai --test golden_combat -- golden_loot_suppressed_under_self_care_pressure --nocapture` passed
  - `cargo test -p worldwake-ai --test golden_combat` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
