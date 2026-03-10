# E09NEEMET-008: E09 integration tests and negative assertions

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — tests only
**Deps**: E09NEEMET-001 through E09NEEMET-007 (all prior tickets)

## Problem

E09 requires integration tests that verify the full physiology loop works end-to-end and negative assertions that confirm forbidden patterns (no stored fear, no stored wellness scores) are absent. This ticket also covers the spec's explicit test list items that span multiple subsystems.

## Assumption Reassessment (2026-03-10)

1. The spec lists 15 tests in its "Tests" section — some are unit-level (covered by prior tickets), some require integration across components + system + actions.
2. Phase 2 gate test T15 ("Need progression") requires values evolve by simulation tick, not frame rate — needs integration with scheduler.
3. T26 ("Camera independence") requires physiology not reset on visibility change — needs to verify no external reset path exists.
4. The spec requires negative assertions: no stored `fear` component, no stored `AgentCondition` component.

## Architecture Check

1. Integration tests live in `crates/worldwake-systems/tests/` or as `#[cfg(test)]` modules.
2. Tests must create a minimal `World` + `EventLog` + `Scheduler` setup with agents that have all physiology components, then step ticks and verify outcomes.
3. Negative assertions use grep-style compile-time or runtime checks that certain types/components do not exist.

## What to Change

### 1. Integration test: full metabolism cycle

Create an agent with `HomeostaticNeeds::new_sated()`, default `MetabolismProfile`, `DriveThresholds`, empty `WoundList`, zeroed `DeprivationExposure`. Run N ticks of the needs system. Assert:
- All drives have increased from 0
- Rates match `MetabolismProfile` * ticks
- No wounds yet (not at critical long enough)

### 2. Integration test: eat-drink cycle restores needs

Create hungry/thirsty agent with food + water at same location. Execute Eat action to completion. Assert hunger decreased by commodity profile amount. Execute Drink. Assert thirst decreased.

### 3. Integration test: starvation → wound

Create agent at max hunger. Run needs system for `starvation_tolerance_ticks`. Assert deprivation wound added to `WoundList`.

### 4. Integration test: fatigue → forced collapse

Create agent at max fatigue. Run for `exhaustion_collapse_ticks`. Assert collapse signal emitted.

### 5. Integration test: bladder accident creates waste

Create agent at max bladder. Run for `bladder_accident_tolerance_ticks`. Assert bladder reset, waste entity exists at location, dirtiness increased.

### 6. Integration test: different MetabolismProfiles diverge

Create two agents with different `MetabolismProfile` values (e.g., hunger_rate 2 vs 5). Run same ticks. Assert hunger differs.

### 7. Negative assertion: no stored fear component

Assert that `component_schema.rs` does not register a "fear" or "Fear" component. Assert no `struct Fear` or `fear: Permille` field exists in authoritative component types (excluding `DriveThresholds` which has a threshold band for danger/fear but that's a threshold, not stored state).

### 8. Negative assertion: no stored AgentCondition component

Assert that no `AgentCondition` or `wellness` stored component exists.

### 9. Integration test: sleep without bed reduces fatigue

Create fatigued agent at a location with no bed entity. Start Sleep action. Tick. Assert fatigue decreases.

### 10. Integration test: sleep with bed is better

Create fatigued agent with bed reservation. Start Sleep action. Tick same number. Assert fatigue is lower than ground-sleep variant.

## Files to Touch

- `crates/worldwake-systems/tests/needs_integration.rs` (new)
- `crates/worldwake-systems/tests/needs_negative_assertions.rs` (new)

## Out of Scope

- E12 combat wound tests
- E13 AI decision tests
- Performance benchmarks
- Save/load round-trip of physiology state (covered by existing E08 patterns)
- Multi-agent interaction tests

## Acceptance Criteria

### Tests That Must Pass

1. T15: Need progression — values evolve by simulation tick.
2. T26: Camera independence — physiology does not reset on visibility change.
3. Eating consumes food and applies commodity-defined relief.
4. Drinking consumes water and applies commodity-defined relief.
5. Sleep reduces fatigue even without a bed; beds improve recovery rate.
6. Toilet reduces bladder and creates waste entity.
7. Wash reduces dirtiness and consumes water when applicable.
8. Active action body costs increase fatigue/thirst deterministically.
9. Sustained critical hunger adds deprivation wound(s).
10. Sustained critical thirst adds deprivation wound(s).
11. Sustained critical fatigue triggers forced collapse/sleep.
12. Sustained critical bladder triggers involuntary relief.
13. Need values stay within Permille range.
14. Different MetabolismProfile values produce different progression/tolerance behavior.
15. DriveThresholds are per-drive and per-agent, not global constants.
16. There is no stored fear component and no stored AgentCondition component.
17. Existing suite: `cargo test --workspace`

### Invariants

1. All spec-listed tests from E09 "Tests" section are covered.
2. Conservation invariant holds through all eat/drink/toilet cycles.
3. No floating-point in any test setup or assertion.
4. Determinism: same seed + same inputs = same physiology outcomes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/tests/needs_integration.rs` — full cycle, eat/drink, starvation, collapse, bladder, divergent profiles, sleep variants
2. `crates/worldwake-systems/tests/needs_negative_assertions.rs` — no stored fear, no stored wellness/condition

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
