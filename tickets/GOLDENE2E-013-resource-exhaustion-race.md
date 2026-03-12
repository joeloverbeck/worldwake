# GOLDENE2E-013: Resource Exhaustion Race

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Possible
**Deps**: None

## Problem

Current tests use at most 2 agents. When 4+ hungry agents compete for a limited resource source, the system must handle concurrent harvest attempts, resource exhaustion, and graceful degradation as resources run out. This tests reservation/contention logic, multi-agent scheduling fairness, and plan failure recovery at scale.

**Coverage gap filled**:
- Cross-system chain: Multiple agents → simultaneous needs pressure → concurrent harvest attempts → resource depletion → some agents fail → seek alternatives or wait for regeneration → conservation holds → no deadlocks
- Scale testing: 4 agents (2x previous maximum)

## Assumption Reassessment (2026-03-12)

1. `ResourceSource` tracks `available_quantity` which decreases on harvest (confirmed).
2. The action framework serializes actions per tick via the scheduler — agents take turns (confirmed).
3. Harvest action validation checks `available_quantity > 0` before allowing harvest (confirmed — action validation in `crates/worldwake-sim/src/action_validation.rs`).
4. `total_live_lot_quantity` and `verify_authoritative_conservation` enforce conservation regardless of agent count (confirmed).
5. Plan failure handling in `crates/worldwake-ai/src/failure_handling.rs` handles failed harvest attempts — agents should replan (confirmed).

## Architecture Check

1. This test validates system behavior under contention — a property that only emerges with more agents than resources. It catches bugs in: resource reservation, action validation race conditions, plan failure cascades, and conservation under contention.
2. Fits in `golden_production.rs` since it tests production system contention.
3. Straightforward setup: 4 agents, 1 resource source with limited supply.

## What to Change

### 1. Write golden test: `golden_resource_exhaustion_race`

In `golden_production.rs`:

Setup:
- 4 agents at Orchard Farm, all critically hungry (`pm(900)`), no food in inventory.
- OrchardRow workstation with `Quantity(4)` apples in ResourceSource (just enough for 2 harvests of 2 each).
- No resource regeneration (`regeneration_ticks_per_unit: None`).
- Run simulation for up to 150 ticks.
- Assert: at least 2 agents harvest successfully (total apple lots appear).
- Assert: at least 1 agent cannot harvest (resource exhausted before they can).
- Assert: conservation — total apple authoritative quantity never exceeds 4.
- Assert: no deadlock — simulation completes without hanging.
- Assert: no panic from concurrent access or failed plans.

**Expected emergent chain**: 4 agents race for 4 apples → 2 harvest cycles (2 apples each) exhaust the source → remaining agents fail to harvest → they replan (possibly blocked intent, or seek alternatives) → system remains stable.

### 2. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P13 from Part 3 to Part 1.
- Update cross-system interactions: "Resource exhaustion race with 4+ agents" now tested.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify — add test)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Agents traveling to alternative resource sources (they can if the topology supports it, but not asserted)
- Resource regeneration during the race (no regen configured)
- Agent-to-agent trading to share harvested resources
- More than 4 agents
- Fairness guarantees (which agents get resources first is seed-dependent)

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

1. `golden_resource_exhaustion_race` — 4 agents compete for limited resources, system handles exhaustion gracefully
2. At least 2 agents acquire apples (successful harvests occur)
3. Resource source depletes to 0 available quantity
4. Conservation: total apple authoritative quantity never exceeds initial 4
5. No panics, no deadlocks — simulation completes within tick limit
6. Coverage report `reports/golden-e2e-coverage-analysis.md` updated
7. Existing suite: `cargo test -p worldwake-ai --test golden_production`
8. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation holds for all commodity kinds every tick
3. Determinism: same seed produces same outcome
4. No agent has more apples than physically possible given resource source limits

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs::golden_resource_exhaustion_race` — proves multi-agent resource contention

### Commands

1. `cargo test -p worldwake-ai --test golden_production golden_resource_exhaustion_race`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
