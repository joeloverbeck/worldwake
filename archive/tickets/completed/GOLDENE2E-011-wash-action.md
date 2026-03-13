# GOLDENE2E-011: Wash Action

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Possible
**Deps**: None

## Problem

The dedicated golden E2E proof for the Wash pathway is missing. The wash pathway requires dirtiness above threshold and locally controlled Water, but the current ticket overstates the engine gap: wash candidate generation and wash action handling already have focused lower-level coverage. What is missing is an end-to-end emergent scenario through the real AI loop.

**Coverage gap filled**:
- GoalKind: `Wash` (golden E2E untested)
- Need: Dirtiness (as driver)
- Cross-system chain: Metabolism → dirtiness escalation → Wash goal → wash action → dirtiness decreases + water consumed

## Assumption Reassessment (2026-03-12)

1. `GoalKind::Wash` exists (confirmed in `crates/worldwake-core/src/goal.rs`).
2. `HomeostaticNeeds` has a `dirtiness` field (confirmed).
3. `MetabolismProfile` has a `dirtiness_rate` field (confirmed).
4. Candidate generation already emits `GoalKind::Wash` when dirtiness reaches the low threshold and the agent locally controls Water (`crates/worldwake-ai/src/candidate_generation.rs`, `wash_requires_dirtiness_and_local_water`).
5. A concrete `wash` needs action already exists in `crates/worldwake-systems/src/needs_actions.rs`, with a `Water` target precondition and commit logic that consumes Water and reduces dirtiness (`wash_consumes_water_and_clears_dirtiness` covers this at the unit level).
6. The ticket's workstation assumption was incorrect: the current architecture does not require `WorkstationTag::WashBasin` or any place tag for washing. Wash is a local self-care action gated by controlled Water, not by facility access.
7. Dirtiness is not entirely absent from the golden suite today: `golden_bladder_relief_with_travel` already observes dirtiness dropping after latrine relief. What remains unproven is the dedicated `Wash` goal/action chain.
8. The real gap is therefore one golden E2E scenario plus harness/report updates, not a likely engine feature build.

## Architecture Check

1. The current architecture is already the right shape for this behavior: candidate generation, planning, and needs actions model wash as a normal needs-driven path with no special cases. Adding golden coverage is more valuable than refactoring because the architecture is already clean, explicit, and consistent with Principle 12.
2. A workstation-gated redesign would only be justified if washing should become facility-based everywhere in the simulation. That would be a broader design change, not something this ticket should smuggle in. For the current architecture, the cleaner scope is to prove the existing local-water wash path end-to-end.
3. This test completes the golden homeostatic-needs coverage set once added. It fits in `golden_ai_decisions.rs` alongside thirst/bladder because it is the same class of needs-driven AI behavior.

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
- Agent at Village Square with high dirtiness (`pm(800)`).
- Agent has `Quantity(1)` Water.
- All other needs low.
- Run simulation for up to 80 ticks.
- Assert: dirtiness decreases from initial high value.
- Assert: Water quantity decreases (water consumed for washing).
- Assert: total live Water quantity never increases (conservation).

**Expected emergent chain**: High dirtiness above threshold + locally controlled Water → Wash goal generated → wash action executes → dirtiness decreases + water consumed.

### 3. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P11 from Part 3 to Part 1.
- Update Part 2: Dirtiness need now tested, Wash GoalKind tested.
- With this ticket, all 5 needs have golden coverage.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add `agent_dirtiness()`)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify — add test)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)
- `tickets/GOLDENE2E-000-index.md` (modify — move this ticket to completed/archive state and refresh summary counts)

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

1. `golden_wash_action` — agent with high dirtiness and Water washes, reducing dirtiness
2. Agent's dirtiness decreases from initial high value
3. Water quantity decreases (consumed for washing)
4. Total live Water quantity never increases during the scenario
5. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: Dirtiness need and Wash GoalKind marked as tested
6. Existing suite: `cargo test -p worldwake-ai --test golden_ai_decisions`
7. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

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

## Outcome

### Completion date

2026-03-13

### What actually changed

- Added `GoldenHarness::agent_dirtiness()` in `crates/worldwake-ai/tests/golden_harness/mod.rs`.
- Added `golden_wash_action` in `crates/worldwake-ai/tests/golden_ai_decisions.rs`.
- Updated `reports/golden-e2e-coverage-analysis.md` to record the new wash coverage and refreshed suite totals.
- Updated `tickets/GOLDENE2E-000-index.md` so the initiative summary and ticket list reflect this ticket's completion.

### Engine changes made

- None. The existing wash architecture was already clean and sufficient: AI candidate generation, planner semantics, and needs-action execution already modeled wash as a normal local self-care path gated by controlled Water.

### Deviations from the original plan

- Corrected the ticket's stale assumptions before implementation. Wash was not an engine-gap feature: it already had focused AI/action coverage and did not require a `WashBasin` or place-specific facility.
- Scoped the implementation to the missing golden proof rather than expanding into unnecessary architectural changes.
- Tightened the expected scenario around the actual invariant that matters here: Water consumption and dirtiness relief under the real AI loop, with live Water conservation checks.

### Verification results

- `cargo test -p worldwake-ai --test golden_ai_decisions golden_wash_action` passed.
- `cargo test -p worldwake-ai --test golden_ai_decisions` passed.
- `cargo test --workspace` passed.
- `cargo clippy --workspace` passed.
