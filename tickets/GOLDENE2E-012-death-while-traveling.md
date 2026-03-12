# GOLDENE2E-012: Death While Traveling

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Possible
**Deps**: GOLDENE2E-003 (Multi-Hop Travel Plan — needs multi-tick travel)

## Problem

What happens when an agent dies from deprivation during a multi-tick travel action? The action should terminate, death should be processed, and the corpse should remain at a consistent location. This edge case is untested and could reveal bugs in action termination during death, corpse placement, and item conservation during mid-travel death.

**Coverage gap filled**:
- Cross-system chain: Travel in progress → metabolism → deprivation → wound accumulation → death during active action → action termination → corpse location consistency → item conservation
- Tests action framework's death handling during multi-tick actions

## Assumption Reassessment (2026-03-12)

1. Death processing exists in the needs system (deprivation wounds → wound accumulation → death when total exceeds wound_capacity) — confirmed from Scenario 8.
2. `DeadAt` component marks dead agents (confirmed — `agent_is_dead()` checks this).
3. Action termination on death — the scheduler/action framework should clean up active actions for dead agents. Needs verification during implementation.
4. Corpse location — where does the corpse appear? At departure, destination, or in-transit? This is an engine behavior to discover and document.
5. Item conservation during death — items owned by the dead agent should remain conserved (confirmed from Scenario 8).

## Architecture Check

1. This test validates a critical edge case: what happens to the simulation when an agent dies during a multi-tick action. It ensures no dangling state, no lost items, and no action framework corruption.
2. Depends on GOLDENE2E-003 establishing that multi-hop travel works. This ticket adds death during that travel.
3. Goes in `golden_combat.rs` since it extends death/corpse scenarios.

## What to Change

### 1. Write golden test: `golden_death_while_traveling`

In `golden_combat.rs`:

Setup:
- Fragile agent at Village Square (similar to Scenario 8's fragile victim setup):
  - Very low `wound_capacity: pm(200)`
  - Existing starvation wound `severity: pm(150)`
  - Fast hunger metabolism, `hunger_critical_ticks: 2` in `DeprivationExposure`
  - `hunger: pm(950)` (near-critical)
- Food only available at distant Orchard Farm (agent must travel).
- Give the agent coins to verify item conservation.
- Run simulation for up to 100 ticks.
- Assert: agent starts traveling (leaves Village Square).
- Assert: agent dies during the simulation (wound accumulation from deprivation).
- Assert: item conservation holds every tick (coins never created/destroyed).
- Assert: agent's death does not crash or cause action framework errors.

**Expected emergent chain**: Hunger drives travel to food → travel begins → metabolism continues → deprivation wounds accumulate during travel → death → action terminated → corpse placed → items conserved.

### 2. Document corpse location behavior

In the Outcome section, document where the corpse ends up (departure location, destination, or intermediate). This is an engine behavior discovery — do not assert a specific location unless the engine has a clear rule.

### 3. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P12 from Part 3 to Part 1.
- Update cross-system interactions: "Death during active travel action" now tested.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add test)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Looting the traveler's corpse (already covered by Scenario 8 for non-travelers)
- Death during non-travel actions
- Revival or resurrection mechanics
- Other agents reacting to the death

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

1. `golden_death_while_traveling` — fragile agent dies from deprivation during multi-tick travel
2. Agent starts traveling (leaves starting location or has active travel action)
3. Agent dies during the simulation (`agent_is_dead()` returns true)
4. No panic or action framework error during death processing
5. Conservation: coin quantity is constant every tick
6. Coverage report `reports/golden-e2e-coverage-analysis.md` updated
7. Existing suite: `cargo test -p worldwake-ai --test golden_combat`
8. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation holds for all commodity kinds every tick
3. Determinism: same seed produces same outcome
4. No dangling active actions for dead agents after death tick

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs::golden_death_while_traveling` — proves death handling during active travel

### Commands

1. `cargo test -p worldwake-ai --test golden_combat golden_death_while_traveling`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
