# GOLDENE2E-005: Bladder Relief with Travel

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Possible
**Deps**: None

## Problem

The Relieve goal and PublicLatrine place are completely untested. An agent with high bladder pressure should recognize the need, travel to the latrine, and relieve. This tests the bladder need pathway end-to-end, including the travel-to-facility sub-plan.

**Coverage gap filled**:
- GoalKind: `Relieve` (completely untested)
- Need: Bladder (as driver)
- Topology: PublicLatrine (unused place)
- Cross-system chain: Metabolism → bladder escalation → Relieve goal → travel to PublicLatrine → relieve action → bladder decreases

## Assumption Reassessment (2026-03-12)

1. `GoalKind::Relieve` exists in `crates/worldwake-core/src/goal.rs` (confirmed).
2. `HomeostaticNeeds` has a `bladder` field (confirmed).
3. `MetabolismProfile` has a `bladder_rate` field (confirmed).
4. `PrototypePlace::PublicLatrine` exists in the topology (confirmed).
5. Candidate generation emits Relieve goals when bladder crosses threshold — needs verification during implementation.
6. A relieve action exists in `crates/worldwake-systems/src/needs_actions.rs` — needs verification during implementation.
7. The relieve action requires the agent to be at a place with appropriate facilities (PublicLatrine) — needs verification.

## Architecture Check

1. This test validates a need pathway that requires travel to a specific facility type. Unlike eating (which can happen anywhere if you have food), relieving may require being at a specific place, testing the AI's ability to plan travel as a sub-goal.
2. Fits in `golden_ai_decisions.rs` since it tests needs-driven AI behavior.
3. No shims needed.

## What to Change

### 1. Add harness constants and helper

In `golden_harness/mod.rs`:
```rust
pub const PUBLIC_LATRINE: EntityId = prototype_place_entity(PrototypePlace::PublicLatrine);

// In impl GoldenHarness:
pub fn agent_bladder(&self, agent: EntityId) -> Permille {
    self.world
        .get_component_homeostatic_needs(agent)
        .map_or(pm(0), |n| n.bladder)
}
```

### 2. Write golden test: `golden_bladder_relief_with_travel`

In `golden_ai_decisions.rs`:
- Agent at Village Square with high bladder pressure (e.g., `pm(800)`), fast bladder metabolism.
- All other needs low.
- PublicLatrine is a separate place connected to Village Square.
- Run simulation for up to 100 ticks.
- Assert: agent's bladder decreases from initial high value.
- Assert: agent visits PublicLatrine at some point (or bladder decreases, proving relief occurred).

**Expected emergent chain**: Bladder pressure → Relieve goal → plan includes travel to PublicLatrine → travel action → relieve action → bladder decreases.

### 3. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P5 from Part 3 to Part 1.
- Update Part 2: Bladder need now tested, Relieve GoalKind tested, PublicLatrine used.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add `PUBLIC_LATRINE`, `agent_bladder()`)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify — add test)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Bladder as interrupt trigger during other actions
- Waste commodity production from relief (if applicable)
- Multiple relief cycles
- Bladder pressure competing with other critical needs (that's GOLDENE2E-010)

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

1. `golden_bladder_relief_with_travel` — agent with high bladder pressure travels to latrine and relieves
2. Agent's bladder value decreases from initial high value
3. Simulation completes within 100 ticks
4. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: Bladder need tested, Relieve GoalKind tested, PublicLatrine marked as used
5. Existing suite: `cargo test -p worldwake-ai --test golden_ai_decisions`
6. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Determinism: same seed produces same outcome
3. Agent remains alive throughout

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_bladder_relief_with_travel` — proves bladder relief pathway with travel

### Commands

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_bladder_relief_with_travel`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
