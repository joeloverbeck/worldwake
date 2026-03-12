# GOLDENE2E-001: Thirst-Driven Acquisition

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Possible
**Deps**: None

## Problem

Thirst is the only hunger-like need with consume actions, but it has zero golden coverage. The `relieves_thirst` predicate and `CommodityKind::Water` consumption pathway are completely untested at the E2E level. This is a trivial gap to close — clone the Scenario 7 (Deprivation Cascade) pattern with thirst instead of hunger.

**Coverage gap filled**:
- GoalKind: `ConsumeOwnedCommodity { commodity: Water }` (thirst pathway)
- Need: Thirst (as driver)
- Cross-system chain: Metabolism → thirst escalation → AI threshold detection → goal generation → drink action

## Assumption Reassessment (2026-03-12)

1. `HomeostaticNeeds` has a `thirst` field (confirmed in `crates/worldwake-core/src/needs.rs`).
2. `MetabolismProfile` has a `thirst_rate` field that drives thirst accumulation per tick (confirmed).
3. `CommodityKind::Water` exists (confirmed in `crates/worldwake-core/src/items.rs`).
4. Candidate generation in `crates/worldwake-ai/src/candidate_generation.rs` already emits thirst-driven `ConsumeOwnedCommodity { commodity: Water }` and `AcquireCommodity { commodity: Water, purpose: SelfConsume }` candidates once `needs.thirst >= thresholds.thirst.low()` (confirmed).
5. Needs actions in `crates/worldwake-systems/src/needs_actions.rs` already register a concrete `drink` action with `ConsumableEffect::Thirst` preconditions and `commit_drink` consumption logic (confirmed).
6. The ticket's original threshold assumption was stale: default thirst low threshold is `pm(200)`, not `pm(250)` (`DriveThresholds::default()` in `crates/worldwake-core/src/drives.rs`).
7. Existing golden coverage already proves the analogous hunger pathway via `golden_deprivation_cascade`; this ticket is additive coverage for the parallel thirst path, not an engine-gap investigation.

## Architecture Check

1. The current architecture is already the right shape for this behavior: need-driven candidate generation is commodity-agnostic, and needs actions split `eat` and `drink` cleanly by consumable effect. Extending coverage is more valuable than refactoring because the code path is already explicit, decoupled, and consistent with Principle 12.
2. The clean change is additive: reuse the existing golden harness, add a single thirst accessor, and add one focused candidate-generation unit test to lock in the threshold-to-goal mapping at the AI layer.
3. No new test files or compatibility layers are warranted. This belongs in `golden_ai_decisions.rs` plus the existing `candidate_generation.rs` unit-test module.

## What to Change

### 1. Add harness helper: `agent_thirst()`

In `golden_harness/mod.rs`, add:
```rust
pub fn agent_thirst(&self, agent: EntityId) -> Permille {
    self.world
        .get_component_homeostatic_needs(agent)
        .map_or(pm(0), |n| n.thirst)
}
```

### 2. Write golden test: `golden_thirst_driven_acquisition`

In `golden_ai_decisions.rs`:
- Create agent with `pm(0)` thirst, fast thirst metabolism (e.g., `thirst_rate: pm(20)`).
- Give agent `Quantity(1)` Water.
- Run simulation for up to 80 ticks.
- Assert: thirst crosses the agent's default low threshold (`pm(200)`).
- Assert: agent consumes the Water (quantity decreases).

**Expected emergent chain**: Metabolism system → thirst increases → crosses threshold → AI generates `ConsumeOwnedCommodity { commodity: Water }` → agent drinks.

### 3. Strengthen AI-layer coverage with a focused unit test

In `crates/worldwake-ai/src/candidate_generation.rs`:
- Add a unit test proving an owned Water lot emits `ConsumeOwnedCommodity { commodity: Water }` once thirst reaches the low threshold.
- Keep the setup minimal and local to candidate generation so golden coverage remains end-to-end while the unit test isolates the AI invariant.

### 4. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P1 (Thirst-Driven Acquisition) from Part 3 backlog to Part 1 with test name and file.
- Update Part 2 matrices: Thirst need now tested as driver, Water commodity exercised.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — add `agent_thirst()`)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify — add test)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add focused thirst candidate-generation unit test)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Testing thirst as an interrupt trigger (that's a separate scenario)
- Testing water acquisition via travel (covered by other tickets)
- Adding new commodity kinds or needs

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

1. `golden_thirst_driven_acquisition` — agent with fast thirst metabolism and Water in inventory drinks when thirst crosses threshold
2. Thirst crosses low threshold (`pm(200)`) via metabolism before agent acts
3. Agent's Water quantity decreases (consumption occurred)
4. `crates/worldwake-ai/src/candidate_generation.rs` contains a focused unit test for thirst-driven Water consume-goal emission
5. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: Part 2 matrices reflect Thirst as tested need, scenario moves from Part 3 to Part 1
6. Existing suite: `cargo test -p worldwake-ai --test golden_ai_decisions`
7. Relevant unit coverage: `cargo test -p worldwake-ai candidate_generation`
8. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation: Water lots never increase
3. Determinism: same seed produces same outcome

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_thirst_driven_acquisition` — proves thirst-driven consumption pathway
2. `crates/worldwake-ai/src/candidate_generation.rs` unit test for owned Water under thirst pressure — locks in AI candidate emission at the low threshold

### Commands

1. `cargo test -p worldwake-ai --test golden_ai_decisions golden_thirst_driven_acquisition`
2. `cargo test -p worldwake-ai candidate_generation`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

**Completion date**: 2026-03-12

**What actually changed**:
- Added `GoldenHarness::agent_thirst()` in `crates/worldwake-ai/tests/golden_harness/mod.rs`.
- Added `golden_thirst_driven_acquisition` in `crates/worldwake-ai/tests/golden_ai_decisions.rs`.
- Added `owned_water_emits_consume_goal_when_thirsty` in `crates/worldwake-ai/src/candidate_generation.rs`.
- Updated `reports/golden-e2e-coverage-analysis.md` to record the delivered thirst scenario and refreshed coverage notes.

**Deviations from original plan**:
- Corrected the ticket's stale threshold assumption from `pm(250)` to the actual default thirst low threshold `pm(200)`.
- Confirmed that thirst candidate generation and the `drink` action already existed, so no engine changes were needed.
- Strengthened scope with a focused AI unit test in addition to the golden E2E scenario because the AI layer had hunger-path coverage but no parallel thirst-path unit coverage.

**Verification results**:
- `cargo test -p worldwake-ai --test golden_ai_decisions golden_thirst_driven_acquisition --offline -- --exact --nocapture` passed.
- `cargo test -p worldwake-ai candidate_generation --offline -- --nocapture` passed.
- `cargo test -p worldwake-ai --test golden_ai_decisions --offline -- --nocapture` passed.
- `cargo test --workspace` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
