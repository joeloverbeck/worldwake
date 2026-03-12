# GOLDENE2E-011: Wash Action

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Possible
**Deps**: None

## Problem

The Wash goal and dirtiness need are completely untested. The wash pathway requires dirtiness above threshold AND Water in inventory. This is the last untested homeostatic need pathway.

**Coverage gap filled**:
- GoalKind: `Wash` (completely untested)
- Need: Dirtiness (as driver)
- Cross-system chain: Metabolism → dirtiness escalation → Wash goal → wash action → dirtiness decreases + water consumed

## Assumption Reassessment (2026-03-12)

1. `GoalKind::Wash` exists (confirmed in `crates/worldwake-core/src/goal.rs`).
2. `HomeostaticNeeds` has a `dirtiness` field (confirmed).
3. `MetabolismProfile` has a `dirtiness_rate` field (confirmed).
4. Candidate generation for Wash goals — needs verification. Should emit when dirtiness crosses threshold and agent has Water.
5. A wash action in `crates/worldwake-systems/src/needs_actions.rs` — needs verification. Should consume Water and reduce dirtiness.
6. Wash may require being at a specific location (WashBasin workstation) or may work anywhere with Water — needs verification.

## Architecture Check

1. This test completes the homeostatic needs coverage (with 001 covering thirst and 005 covering bladder, this covers dirtiness — fatigue and hunger are already tested).
2. Fits in `golden_ai_decisions.rs` since it tests needs-driven AI behavior.
3. Simple setup similar to thirst (GOLDENE2E-001).

## What to Change

### 1. Add harness helper: `agent_dirtiness()`

In `golden_harness/mod.rs`:
```rust
pub fn agent_dirtiness(&self, agent: EntityId) -> Permille {
    self.world
        .get_component_homeostatic_needs(agent)
        .map_or(pm(0), |n| n.dirtiness)
}
```

### 2. Write golden test: `golden_wash_action`

In `golden_ai_decisions.rs`:

Setup:
- Agent at Village Square with high dirtiness (`pm(800)`), fast dirtiness metabolism.
- Agent has `Quantity(1)` Water.
- All other needs low.
- If wash requires a WashBasin workstation at the location, place one.
- Run simulation for up to 80 ticks.
- Assert: dirtiness decreases from initial high value.
- Assert: Water quantity decreases (water consumed for washing).

**Expected emergent chain**: Dirtiness escalation → Wash goal generated → wash action executes → dirtiness decreases + water consumed.

### 3. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P11 from Part 3 to Part 1.
- Update Part 2: Dirtiness need now tested, Wash GoalKind tested.
- With this ticket, all 5 needs have golden coverage.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add `agent_dirtiness()`)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify — add test)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Wash action at specific workstation types (if not required)
- Dirtiness as interrupt trigger
- Water acquisition for washing (agent starts with water)
- Dirtiness competing with other needs (covered by GOLDENE2E-010 pattern)

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

1. `golden_wash_action` — agent with high dirtiness and water washes, reducing dirtiness
2. Agent's dirtiness decreases from initial high value
3. Water quantity decreases (consumed for washing)
4. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: Dirtiness need and Wash GoalKind marked as tested
5. Existing suite: `cargo test -p worldwake-ai --test golden_ai_decisions`
6. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation: Water lots never increase
3. Determinism: same seed produces same outcome
4. Agent remains alive throughout

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_wash_action` — proves wash/dirtiness pathway

### Commands

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
