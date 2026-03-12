# GOLDENE2E-010: Three-Way Need Competition

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Possible
**Deps**: None

## Problem

No test exercises an agent with multiple simultaneous critical needs. Scenario 2 tests a two-way competition (fatigue vs. hunger), but a three-way competition among hunger, thirst, and fatigue tests that the ranking system correctly selects the highest-utility need when all cross thresholds at once.

**Coverage gap filled**:
- Cross-system chain: Multiple simultaneous critical needs → ranking system → highest-utility need selected → agent addresses it first → re-ranks remaining needs
- Tests the full ranking pipeline under maximum pressure

## Assumption Reassessment (2026-03-12)

1. `rank_candidates()` in `crates/worldwake-ai/src/ranking.rs` scores goals by priority class and motive value (confirmed).
2. `UtilityProfile` has per-need weights (hunger_weight, thirst_weight, fatigue_weight, etc.) (confirmed).
3. Candidate generation emits goals for all needs crossing their thresholds simultaneously (confirmed — each need pathway is independent).
4. The ranking system correctly prioritizes based on `UtilityProfile` weights — this is what we're testing.
5. Sleep, eat, and drink actions all exist and can execute from the same location if resources are available.

## Architecture Check

1. This test validates the ranking algorithm under maximum contention — the most demanding scenario for goal selection. It proves the system doesn't deadlock, oscillate, or make arbitrary choices under multi-need pressure.
2. Fits in `golden_ai_decisions.rs` since it tests AI ranking and decision-making.
3. Simple setup: one agent, three resources, three critical needs.

## What to Change

### 1. Write golden test: `golden_three_way_need_competition`

In `golden_ai_decisions.rs`:

Setup:
- Single agent at Village Square with all three critical needs:
  - Hunger: `pm(900)` (critical)
  - Thirst: `pm(900)` (critical)
  - Fatigue: `pm(900)` (critical)
- Agent has `Quantity(1)` Bread and `Quantity(1)` Water in inventory.
- `UtilityProfile` with distinct weights so ranking is deterministic:
  - `hunger_weight: pm(800)` (highest)
  - `thirst_weight: pm(600)` (middle)
  - `fatigue_weight: pm(400)` (lowest)
- Run simulation for up to 100 ticks.
- Assert: agent addresses hunger first (bread consumed before water).
- Assert: agent addresses thirst second (water consumed after bread).
- Assert: agent eventually sleeps (fatigue addressed last).
- Conservation: bread and water lots never increase.

**Expected emergent chain**: Three critical needs → ranking selects hunger (highest weight) → eat bread → re-rank → thirst next → drink water → re-rank → sleep.

### 2. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P10 from Part 3 to Part 1.
- Update cross-system interactions: "Multiple competing needs" now tested.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify — add test)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Four-way or five-way need competition
- Need competition with travel requirements (combining this with travel would be a separate scenario)
- Interrupt-based switching during need satisfaction (covered by Scenario 2)
- Bladder and dirtiness needs (separate tickets)

## Engine Discovery Protocol

This ticket is a golden e2e test that exercises emergent behavior through the real AI loop.
If implementation reveals that the engine cannot produce the expected emergent behavior,
the following protocol applies:

1. **Diagnose**: Identify the specific engine limitation (missing candidate generation path, planner op gap, action handler deficiency, belief view gap, etc.).
2. **Do not downgrade the test**: The test scenario defines the desired emergent behavior. Do not weaken assertions or remove expected behaviors to work around engine gaps.
3. **Fix forward**: Implement the minimal, architecturally sound engine change that enables the emergent behavior. Document the change in a new "Engine Changes Made" subsection under "What to Change". Each fix must:
   - Follow existing patterns in the affected module
   - Include focused unit tests for the engine change itself
   - Not introduce compatibility shims or special-case logic
4. **Scope guard**: If the required engine change exceeds this ticket's effort rating by more than one level (e.g., a Small ticket needs a Large engine change), stop and apply the 1-3-1 rule: describe the problem, present 3 options, recommend one, and wait for user confirmation before proceeding.
5. **Document**: Record all engine discoveries and fixes in the ticket's Outcome section upon completion, regardless of whether fixes were needed.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_three_way_need_competition` — agent with three critical needs addresses them in utility-weight order
2. Bread consumed before water (hunger prioritized over thirst)
3. Water consumed before agent starts sleeping (thirst prioritized over fatigue)
4. Agent eventually sleeps (fatigue eventually addressed)
5. No deadlock: agent does not stall with all three needs unaddressed
6. Coverage report `reports/golden-e2e-coverage-analysis.md` updated
7. Existing suite: `cargo test -p worldwake-ai --test golden_ai_decisions`
8. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation: bread and water lots never increase
3. Determinism: same seed produces same outcome
4. Agent remains alive throughout

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_three_way_need_competition` — proves priority ranking under multi-need pressure

### Commands

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
