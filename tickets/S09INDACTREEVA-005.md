# S09INDACTREEVA-005: Golden E2E test for finite defend re-evaluation cycle

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S09INDACTREEVA-004 (full `Indefinite` removal must be complete)

## Problem

The original bug (discovered in S07 golden tests) was: a pre-wounded fighter near a hostile target selected `ReduceDanger -> defend` and ran indefinitely — never attacking, looting, or self-healing. The agent was permanently deadlocked. A golden E2E test must prove this deadlock no longer occurs: after defend expires, the agent re-enters the decision cycle and transitions to a different goal.

## Assumption Reassessment (2026-03-20)

1. Existing golden combat tests in `crates/worldwake-ai/tests/golden_combat.rs` cover attack, loot, and defensive mitigation scenarios (verified: 4 `CombatProfile::new()` calls).
2. `golden_reduce_danger_defensive_mitigation` may need tick budget adjustment if it previously relied on indefinite defend. This test must be checked during implementation.
3. This is an AI golden test ticket. The intended layer is golden E2E in `worldwake-ai`. Full action registries are required (the golden harness provides them).
4. The ordering contract is: defend starts -> defend commits after `defend_stance_ticks` ticks -> agent re-enters decision cycle -> agent selects a different goal (attack, loot, self-care, or idle depending on state). This is action-lifecycle ordering verified via decision trace and/or action trace.
5. No heuristic removal — this ticket adds coverage, not changes.
6. Not a stale-request, political, or ControlSource ticket.
7. Golden scenario isolation: the test should set up a pre-wounded agent near a hostile target with `defend_stance_ticks` short enough (e.g., `nz(5)`) to observe re-evaluation within a reasonable tick budget. Unrelated affordances (trade, production) should be excluded from the scenario setup.
8. No mismatch — spec matches codebase.

## Architecture Check

1. A dedicated golden test is the correct proof surface for the original bug. It demonstrates the full emergent cycle: pressure -> goal selection -> defend -> defend expires -> re-evaluation -> new goal.
2. No backwards-compatibility shims.

## Verification Layers

1. Agent selects defend initially (danger pressure is highest) -> decision trace: `DecisionOutcome::Planning` with `ReduceDanger` goal
2. Defend action starts and runs for `defend_stance_ticks` ticks -> action trace: `Started` at tick T, `Committed` at tick T + defend_stance_ticks
3. After defend expires, agent re-evaluates and selects a different action (attack, loot, self-care) -> decision trace: new goal != `ReduceDanger.defend` OR new action != defend
4. Agent is NOT stuck in an infinite defend loop -> world state after N ticks shows the agent has performed at least one non-defend action

## What to Change

### 1. Add golden E2E test for defend re-evaluation

In `crates/worldwake-ai/tests/golden_combat.rs` (or a new test in the same file):

**Scenario setup**:
- Two agents: a fighter and a hostile target
- Fighter has a `CombatProfile` with short `defend_stance_ticks` (e.g., `nz(5)`)
- Fighter is pre-wounded (triggers `ReduceDanger` pressure)
- Hostile target is nearby (triggers danger perception)
- Fighter should initially select defend due to wound pressure + danger

**Assertions**:
- Fighter enters defend within the first few ticks
- Defend commits after exactly `defend_stance_ticks` ticks (not indefinite)
- After defend expires, fighter transitions to a different action (attack, loot, self-heal, or another defend cycle)
- Within a reasonable tick budget (e.g., 30 ticks), the fighter has performed at least one non-defend action — proving the deadlock is broken

**Instrumentation**:
- Enable decision tracing (`h.driver.enable_tracing()`) and/or action tracing (`h.enable_action_tracing()`)
- Use trace queries to verify the defend-then-transition sequence

### 2. Check `golden_reduce_danger_defensive_mitigation`

Review the existing test. If it relied on defend running indefinitely (asserting the agent stays in defend forever), update its assertions to expect finite defend with re-evaluation. If it only checks that defend is selected (not duration), it may pass unchanged.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add new test, possibly adjust existing)

## Out of Scope

- Any engine changes (all engine work is in tickets 001–004)
- Changes to `CombatProfile`, `DurationExpr`, `ActionDuration`, or any production code
- Non-combat golden tests
- Planner changes

## Acceptance Criteria

### Tests That Must Pass

1. New: `golden_defend_finite_reevaluation` (or similar name) — agent defends, defend expires, agent transitions to non-defend action
2. Existing: `golden_reduce_danger_defensive_mitigation` — still passes (adjusted if needed)
3. Existing suite: `cargo test -p worldwake-ai` — all AI tests pass
4. Existing suite: `cargo test --workspace` — no regressions

### Invariants

1. No agent can be permanently stuck in a defend action — defend always expires after `defend_stance_ticks` ticks
2. After defend expires, the agent re-enters the full decision pipeline (candidate generation -> ranking -> plan search -> selection)
3. The `FreelyInterruptible` flag on defend still allows early interruption if a higher-priority goal appears before expiry
4. Decision traces and/or action traces provide complete observability of the defend lifecycle

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs` — new `golden_defend_finite_reevaluation` test proving the original deadlock scenario is resolved
2. `crates/worldwake-ai/tests/golden_combat.rs` — possible adjustment to `golden_reduce_danger_defensive_mitigation` if tick budget or assertions assumed indefinite defend

### Commands

1. `cargo test -p worldwake-ai golden_defend` — targeted new test
2. `cargo test -p worldwake-ai` — full AI test suite
3. `cargo test --workspace` — full workspace regression
