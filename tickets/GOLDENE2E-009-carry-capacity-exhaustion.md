# GOLDENE2E-009: Carry Capacity Exhaustion

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Possible
**Deps**: None

## Problem

No test verifies behavior when an agent's `CarryCapacity` (`LoadUnits(50)` default) is fully consumed. An agent should be unable to pick up more items when at capacity and must either consume items to free space or choose lighter alternatives. Scenario 6c tests partial capacity (1 load unit), but full exhaustion from accumulated inventory is untested.

**Coverage gap filled**:
- Cross-system chain: Production → transport → carry capacity check → plan failure or alternative selection → replanning around constraint
- Tests the load accounting system under genuine exhaustion

## Assumption Reassessment (2026-03-12)

1. `CarryCapacity` component exists with `LoadUnits` newtype (confirmed in `crates/worldwake-core/src/load.rs`).
2. Transport actions (pick-up) check capacity before execution (confirmed — action validation enforces this).
3. Load accounting in `crates/worldwake-systems/src/inventory.rs` tracks per-agent load (confirmed).
4. The default `CarryCapacity(LoadUnits(50))` is set in `seed_agent` harness helper (confirmed in harness).
5. Per-unit load values exist for each `CommodityKind` — needs verification to determine how many items fill 50 load units.

## Architecture Check

1. This test validates graceful degradation when a physical constraint (carry capacity) blocks the standard plan. The agent must adapt — either consume to free space or find lighter alternatives.
2. Fits in `golden_production.rs` since it tests production + transport interaction under load constraints.
3. Uses a very low `CarryCapacity` to force exhaustion quickly without needing massive inventory.

## What to Change

### 1. Write golden test: `golden_carry_capacity_exhaustion`

In `golden_production.rs`:

Setup:
- Agent at Orchard Farm, critically hungry, with `CarryCapacity(LoadUnits(2))` (very low).
- Orchard workstation with `Quantity(20)` apples in ResourceSource.
- Harvest recipe outputs `Quantity(2)` apples per harvest.
- Run simulation for up to 100 ticks.
- Assert: agent harvests apples.
- Assert: agent cannot carry all harvested apples (some remain on ground).
- Assert: agent consumes some apples (hunger decreases).
- Assert: conservation holds.

**Expected emergent chain**: Harvest → apples on ground → pick up limited by capacity → consume carried apples → potentially re-harvest or pick up remaining.

### 2. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P9 from Part 3 to Part 1.
- Update cross-system interactions: carry capacity exhaustion now tested.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify — add test)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Container-based inventory management
- Put-down actions to free capacity
- Multiple agents competing for capacity-limited resources
- Load unit values per commodity kind (use whatever exists)

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

1. `golden_carry_capacity_exhaustion` — agent with limited carry capacity harvests, picks up what fits, and consumes
2. Agent's hunger decreases (consumption occurred)
3. Some apple lots remain on the ground (capacity prevented full pickup)
4. Conservation: total apple quantity never exceeds initial resource source amount
5. Coverage report `reports/golden-e2e-coverage-analysis.md` updated
6. Existing suite: `cargo test -p worldwake-ai --test golden_production`
7. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation holds every tick
3. Determinism: same seed produces same outcome
4. Carry capacity is never exceeded (load accounting is authoritative)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs::golden_carry_capacity_exhaustion` — proves capacity-constrained behavior

### Commands

1. `cargo test -p worldwake-ai --test golden_production golden_carry_capacity_exhaustion`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
