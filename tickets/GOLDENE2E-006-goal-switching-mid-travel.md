# GOLDENE2E-006: Goal Switching Mid-Travel

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Possible
**Deps**: GOLDENE2E-003 (Multi-Hop Travel Plan — needs multi-hop working)

## Problem

No test verifies that an agent can abandon a travel action mid-journey when a higher-priority need emerges. An agent traveling to a distant food source should interrupt travel if hunger spikes to critical and a local alternative becomes available (e.g., food placed at an intermediate location). This validates the interrupt evaluation system during multi-tick actions.

**Coverage gap filled**:
- Cross-system chain: Travel in progress → metabolism drives need escalation → interrupt evaluation → goal switch → travel abandonment → replan for local alternative
- Tests interrupt evaluation during multi-tick travel actions specifically

## Assumption Reassessment (2026-03-12)

1. `evaluate_interrupt()` in `crates/worldwake-ai/src/interrupts.rs` evaluates whether to interrupt running actions — confirmed.
2. `compare_goal_switch()` in `crates/worldwake-ai/src/goal_switching.rs` handles priority-based goal replacement — confirmed.
3. Travel actions are multi-tick (duration based on edge travel time) — confirmed.
4. Metabolism runs every tick, even during travel — confirmed (needs system runs before action progression).
5. The interrupt system can fire during any active action, not just idle states — needs verification during implementation.

## Architecture Check

1. This test validates a critical emergent property: agents are not slaves to their current plan. When circumstances change mid-execution, the AI should be able to re-evaluate and switch goals. This is essential for believable agent behavior.
2. Depends on GOLDENE2E-003 proving multi-hop travel works. This ticket adds the interrupt dimension.
3. Fits in `golden_ai_decisions.rs` since it tests AI goal-switching behavior.

## What to Change

### 1. Write golden test: `golden_goal_switching_mid_travel`

In `golden_ai_decisions.rs`:
- Agent at a distant location (e.g., BanditCamp) with low hunger, no food.
- Food available at OrchardFarm (distant — requires multi-hop travel).
- Agent also has very fast hunger metabolism (hunger will spike during travel).
- Place bread at an intermediate location along the travel path (e.g., NorthCrossroads or ForestPath).
- Run simulation for up to 150 ticks.
- Assert: agent starts traveling (leaves BanditCamp).
- Assert: agent does NOT complete the full journey to OrchardFarm — instead eats the intermediate bread.
- OR Assert: agent's hunger reaches critical during travel AND agent changes behavior (eats bread at intermediate location or adjusts plan).

**Expected emergent chain**: Low hunger → AcquireCommodity at OrchardFarm → start travel → metabolism spikes hunger → interrupt evaluation fires → replan → eat intermediate bread.

**Note**: The exact assertion depends on whether the planner can detect intermediate food during travel. If the engine always completes travel before replanning, the test should document this behavior and the Engine Discovery Protocol applies.

### 2. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P6 from Part 3 to Part 1.
- Update cross-system interactions: "Goal switching mid-travel" now tested.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify — add test)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Goal switching during non-travel actions (partially covered by Scenario 2)
- Death during travel (that's GOLDENE2E-012)
- Multi-agent travel coordination
- Travel route optimization

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

1. `golden_goal_switching_mid_travel` — agent interrupts travel when higher-priority need emerges
2. Agent starts traveling (leaves starting location)
3. Agent's hunger reaches critical level during the simulation
4. Agent eats food (bread quantity decreases) before completing the originally planned full journey
5. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: goal switching mid-travel cross-system interaction marked as tested
6. Existing suite: `cargo test -p worldwake-ai --test golden_ai_decisions`
7. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation: bread lots never increase
3. Determinism: same seed produces same outcome
4. Agent remains alive throughout

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_goal_switching_mid_travel` — proves mid-travel goal interruption

### Commands

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_goal_switching_mid_travel`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
