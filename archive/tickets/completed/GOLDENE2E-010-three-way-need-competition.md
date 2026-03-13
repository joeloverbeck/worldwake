# GOLDENE2E-010: Three-Way Need Competition

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Possible
**Deps**: None

## Problem

No test exercises an agent with multiple simultaneous critical needs. Scenario 2 tests a two-way competition (fatigue vs. hunger), but a three-way competition among hunger, thirst, and fatigue tests that the ranking system correctly selects the highest-utility need when all cross thresholds at once.

**Coverage gap filled**:
- Cross-system chain: Multiple simultaneous critical needs → ranking system → highest-utility need selected → agent addresses it first → re-ranks remaining needs
- Tests the full ranking pipeline under maximum pressure

## Assumption Reassessment (2026-03-13)

1. `rank_candidates()` in `crates/worldwake-ai/src/ranking.rs` orders candidates by priority class first, then motive score, then stable tie-breakers (confirmed).
2. `UtilityProfile` has distinct per-need weights and those weights directly affect motive scores for hunger, thirst, and fatigue (confirmed).
3. Candidate generation emits `ConsumeOwnedCommodity` goals for controlled bread/water lots and emits `Sleep` when fatigue reaches the fatigue **low** threshold, not only when fatigue is critical (confirmed in `candidate_generation.rs`).
4. Sleep is currently location-independent: the real `sleep` action requires only `ActorAlive` and does not seek a bed or a tagged place (confirmed in `needs_actions.rs`).
5. `ConsumeOwnedCommodity` goals are satisfied when the relevant need drops below its medium threshold. From the proposed starting pressures, one bread and one water are insufficient to satisfy those goals, so the original ticket setup would accidentally test longer-horizon plan completion rather than pure local ranking.
6. Because sleep is always locally executable and emitted at the low threshold, the ticket should validate observable ordering in the real loop without over-specifying an exact uninterrupted action sequence between all three needs.
7. Commit order is not a safe proxy for first choice here: `eat` takes longer than `drink`, and the runtime may legitimately interrupt a longer self-care action before commit. The clean observable for initial prioritization is the first started self-care action.

## Architecture Check

1. This test validates the ranking algorithm under simultaneous self-care contention in the real AI loop. That is still architecturally valuable because it proves the ranking + replanning pipeline stays deterministic when multiple local needs are actionable at once.
2. Fits in `golden_ai_decisions.rs` since it tests AI ranking and decision-making.
3. The clean architecture target here is not a scripted queue of `eat -> drink -> sleep`, but a robust ranking system that produces the right first choice, then re-ranks from updated concrete state without compatibility shims or scenario-specific logic.
4. Scope correction: this scenario should assert concrete signals the engine exposes cleanly:
   - the first started self-care action is `eat`
   - bread and water are both eventually consumed in the same local scenario
   - fatigue is eventually reduced after the agent has addressed hunger and thirst pressure
   It should not require bed-seeking, fixed commit ordering between `eat` and `drink`, or a guarantee that fatigue never begins to change until both consumables are gone.

## What to Change

### 1. Write golden test: `golden_three_way_need_competition`

In `golden_ai_decisions.rs`:

Setup:
- Single agent at Village Square with all three needs above their critical thresholds:
  - Hunger: `pm(900)` (critical)
  - Thirst: `pm(900)` (critical)
  - Fatigue: `pm(920)` (critical for default thresholds)
- Agent has `Quantity(2)` Bread and `Quantity(2)` Water in inventory.
- `UtilityProfile` with distinct weights so ranking is deterministic:
  - `hunger_weight: pm(800)` (highest)
  - `thirst_weight: pm(600)` (middle)
  - `fatigue_weight: pm(400)` (lowest)
- Run simulation for up to 100 ticks.
- Assert: the first started self-care action is `eat`.
- Assert: the agent also consumes water from the same local option set during the scenario.
- Assert: after the consumables are exhausted, fatigue is eventually reduced by rest.
- Conservation: bread and water lots never increase.

**Expected emergent chain**: Three simultaneous self-care pressures with fully plannable local solutions → ranking selects hunger first, so the first started self-care action is `eat` → the runtime continues reprioritizing as state changes and may commit `drink` sooner because it is shorter → later rest reduces fatigue.

### Engine Changes Made

- Tighten planner consume transitions so a `drink` step cannot satisfy a bread-consume goal (or vice versa) in hypothetical planning. This preserves commodity-specific causality in the planner and prevents invalid cross-commodity consume plans.

### 2. Add focused ranking regression coverage

In `crates/worldwake-ai/src/ranking.rs` tests:

- Add a unit test covering the direct ranking invariant for simultaneous critical hunger, thirst, and fatigue.
- Assert the ranked order is bread consume goal, then water consume goal, then sleep, given the ticket utility weights.

This captures the architectural contract directly while the golden test proves the same ordering survives the real controller/runtime path.

### 3. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P10 from Part 3 to Part 1.
- Update cross-system interactions: "Multiple competing needs" now tested.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify — add test)
- `crates/worldwake-ai/src/ranking.rs` (modify — add focused ranking regression test)
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

1. `golden_three_way_need_competition` — agent with three critical needs starts with the hunger path under the configured weights
2. The first started self-care action is `eat`
3. Bread and water are both eventually consumed in the scenario
4. Agent eventually reduces fatigue after the consumables are handled
5. No deadlock: agent does not stall with all three needs unaddressed
6. Focused unit coverage added in `crates/worldwake-ai/src/ranking.rs`
7. Coverage report `reports/golden-e2e-coverage-analysis.md` updated
8. Existing suite: `cargo test -p worldwake-ai --test golden_ai_decisions`
9. Focused unit suite: `cargo test -p worldwake-ai ranking`
10. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation: bread and water lots never increase
3. Determinism: same seed produces same outcome
4. Agent remains alive throughout

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_three_way_need_competition` — proves the real AI loop re-ranks local hunger/thirst/fatigue competition correctly
2. `crates/worldwake-ai/src/ranking.rs::<new simultaneous-needs ranking test>` — proves the direct ranking contract independent of runtime noise

### Commands

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_three_way_need_competition`
2. `cargo test -p worldwake-ai ranking`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-13
- **What actually changed**:
  - Added `golden_three_way_need_competition` to `crates/worldwake-ai/tests/golden_ai_decisions.rs`.
  - Added focused ranking coverage in `crates/worldwake-ai/src/ranking.rs` for simultaneous critical hunger/thirst/fatigue ordering.
  - Fixed planner consume transitions in `crates/worldwake-ai/src/planner_ops.rs` so a consume step must match the goal commodity.
  - Updated `reports/golden-e2e-coverage-analysis.md` to record the new scenario and closed cross-system gap.
- **Deviations from original plan**:
  - The ticket originally assumed commit order (`bread` before `water`) was the right observable. The runtime showed that commit order is distorted by action duration and interrupts, so the final golden assertion uses the first started self-care action (`eat`) plus eventual bread/water consumption and later fatigue relief.
  - The original setup used `Quantity(1)` bread and water. The final scenario uses `Quantity(2)` of each so the local hunger/thirst paths are genuinely plannable from the start.
  - Implementation exposed a real planner bug, so the completed work includes an engine fix rather than test-only changes.
- **Verification results**:
  - `cargo test -p worldwake-ai --test golden_ai_decisions`
  - `cargo test -p worldwake-ai simultaneous_critical_self_care_needs_rank_by_weighted_order`
  - `cargo test -p worldwake-ai consume_transition_rejects_mismatched_target_commodity`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
