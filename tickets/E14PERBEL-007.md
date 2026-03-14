# E14PERBEL-007: Belief Isolation Integration Tests

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only ticket
**Deps**: E14PERBEL-005 (perception system running), E14PERBEL-006 (migration complete, OmniscientBeliefView deleted)

## Problem

The Phase 3 gate requires T10 (belief isolation) and several other integration tests that verify end-to-end behavior: agents must NOT react to events they haven't perceived, beliefs must be traceable, and the full perception→belief→planning pipeline must work correctly. These are acceptance tests for the entire E14 epic.

## Assumption Reassessment (2026-03-14)

1. Phase 3 gate includes "T10: Belief isolation — agent does not react to unseen theft, death, or camp migration" — confirmed in IMPLEMENTATION-ORDER.md.
2. The spec lists 11 test requirements in the "Tests" section — confirmed.
3. Integration tests need a full `SimulationState` with agents, places, events, perception system, and AI planning — this requires all prior E14 tickets to be complete.
4. Deterministic RNG ensures test reproducibility — `ChaCha8Rng` with fixed seed.

## Architecture Check

1. Integration tests live alongside the crate they primarily exercise — for cross-crate tests, a dedicated integration test file may be needed.
2. Tests verify behavior, not implementation — they set up scenarios and assert outcomes.

## What to Change

### 1. T10: Belief isolation — unseen theft

Set up: Agent A and Agent B at Place 1. Agent C at Place 2. Agent C steals from (or kills) Agent B. Verify Agent A does not know about the event — Agent A's `AgentBeliefStore` has no record of the theft/death, and Agent A's planning does not react to it.

### 2. T10: Belief isolation — unseen death

Set up: Agent A at Place 1. Agent B at Place 2. Agent B dies (deprivation or combat). Verify Agent A does not know Agent B is dead — `BelievedEntityState.alive` remains `true` (or entity unknown).

### 3. T10: Belief isolation — unseen camp migration

Set up: Agent A at Place 1. Agent B moves from Place 2 to Place 3. Verify Agent A still believes Agent B is at Place 2 (stale belief) or has no belief about Agent B's location.

### 4. Direct perception test

Set up: Agent A and Agent B at same place. Event occurs involving Agent B. Verify Agent A perceives it — `AgentBeliefStore` updated with `DirectObservation` source and current tick.

### 5. Agent at different place does not perceive

Set up: Agent A at Place 1. Event occurs at Place 2. Verify Agent A's belief store is unchanged.

### 6. Observation fidelity test

Set up: Agent with `observation_fidelity = Permille(500)` at same place as event. Run many times with different RNG seeds. Verify approximately 50% perception rate (within statistical bounds for deterministic seeds).

### 7. Memory capacity test

Set up: Agent with `memory_capacity = 3`. Agent observes 5 different entities. Verify belief store contains only the 3 most recently observed.

### 8. Social observation recording

Set up: Two agents trade at a place where a third agent is present. Verify the observing agent records a `SocialObservation` with `kind: WitnessedCooperation`.

Set up: Two agents fight at a place where a third agent is present. Verify `WitnessedConflict` recorded.

### 9. Stale beliefs not auto-updated

Set up: Agent A observes Agent B at Place 1 at tick 10. Agent B moves to Place 2 at tick 20 (Agent A not present). At tick 30, verify Agent A still believes Agent B is at Place 1.

### 10. Belief conflict resolution — newer supersedes older

Set up: Agent A observes Agent B at Place 1 at tick 10. Agent A later observes Agent B at Place 2 at tick 20. Verify belief store shows Place 2 with `observed_tick = 20`.

### 11. PerAgentBeliefView enforces separation

Verify: After E14PERBEL-006, no code path in workspace references `OmniscientBeliefView`. This is a compile-time check (if it compiles, it passes) plus a grep verification.

### 12. Full pipeline test: perception → belief → planning

Set up: Agent A (merchant) at market place. Agent B (customer) arrives at same place. Perception system runs. Verify Agent A's belief store knows about Agent B. Verify Agent A's planning (via `PerAgentBeliefView`) can see Agent B as a potential trade partner.

## Files to Touch

- `crates/worldwake-systems/tests/perception_integration.rs` (new — integration tests for perception system)
- `crates/worldwake-ai/tests/belief_isolation.rs` (new — T10 tests for belief-based planning isolation)

## Out of Scope

- Unit tests for individual components (covered in E14PERBEL-002 through -005)
- Report/rumor propagation tests (E15 scope)
- Office/faction tests (E16 scope)
- Crime discovery tests (E17 scope — T25)
- Performance benchmarks
- Soak tests (E22 scope)
- Modifying any production code

## Acceptance Criteria

### Tests That Must Pass

1. All 12 test scenarios described above pass
2. `cargo test --workspace` — full green
3. `cargo clippy --workspace` — no warnings
4. Grep for `OmniscientBeliefView` returns zero matches in non-archived code

### Invariants

1. T10 (belief isolation) passes — Phase 3 gate requirement
2. All beliefs traceable to `PerceptionSource::DirectObservation` (no untraceable beliefs)
3. No agent reacts to information it hasn't perceived
4. Stale beliefs persist until re-observation
5. Social observations captured for cooperation/conflict events
6. Memory capacity and retention correctly enforced
7. Determinism: same seed produces same test outcomes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/tests/perception_integration.rs` — scenarios 1-10
2. `crates/worldwake-ai/tests/belief_isolation.rs` — scenarios 1-3 (T10), 11-12

### Commands

1. `cargo test -p worldwake-systems -- perception_integration`
2. `cargo test -p worldwake-ai -- belief_isolation`
3. `cargo clippy --workspace`
4. `cargo test --workspace`
