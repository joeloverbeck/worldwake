# E13DECARC-017: Belief-only audit, Phase 2 gate tests, and invariant enforcement

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — tests and audit only
**Deps**: E13DECARC-016

## Problem

After all E13 subsystems are implemented, we need a comprehensive test suite that verifies: (1) all AI reads go through `&dyn BeliefView`, (2) Phase 2 gate criteria pass, (3) all spec-listed tests pass, and (4) invariants hold under multi-tick simulation.

## Assumption Reassessment (2026-03-11)

1. All prior E13 tickets are implemented — dependency.
2. Phase 2 gate criteria from `specs/IMPLEMENTATION-ORDER.md`: agents eat/drink/sleep/wash/relieve, merchants restock, 24+ hour survival, no deadlock — confirmed.
3. Spec test checklist has 22 items — confirmed.
4. `worldwake-systems` registers all Phase 2 actions — confirmed from prior epics.
5. `build_prototype_world()` creates a test world — confirmed.

## Architecture Check

1. This is a verification-only ticket — no production code changes.
2. Tests serve as the Phase 2 gate acceptance criteria.
3. The belief-only audit is a compile-time/grep verification that `worldwake-ai` never uses `&World`.

## What to Change

### 1. Belief-only audit test

A test that greps/scans `worldwake-ai/src/` for any direct `&World` usage. All AI code must go through `&dyn BeliefView`.

### 2. Phase 2 gate integration tests

Create `crates/worldwake-ai/tests/phase2_gate.rs` (integration test file):

- **Autonomous survival**: Set up agents with needs, food sources, sleep locations. Run 24+ simulated hours. Verify no agent dies from preventable deprivation.
- **Merchant restock**: Set up a merchant with `MerchandiseProfile`. Run ticks. Verify merchant acquires stock via physical procurement path.
- **No thrashing**: Verify agents don't switch between equal plans every tick.
- **No blocked retries**: Verify agents don't retry the same blocked target every tick.
- **No AI deadlock**: Verify the world runs 24+ in-world hours without all agents idling permanently.
- **Survival outranks commerce**: Verify Critical survival needs always produce higher-priority plans than enterprise goals.
- **Combat as leaf**: Verify combat is only selected as a leaf commitment step.

### 3. Spec test checklist

Implement all 22 tests from the spec's "Tests" section:

1. All E13 reads go through `&dyn BeliefView`; no planner code reads `&World`
2. `OmniscientBeliefView` implements all new belief methods
3. `PlannedStep` stores exact ordered `targets` and exact `payload_override`
4. `PlannedStep` converts losslessly to `InputKind::RequestAction`
5. Revalidation is performed by affordance identity, not by duplicate precondition code
6. Search nodes do not clone whole-world state
7. `PlanningState` is transient and never stored as authoritative world state
8. Candidate generation emits only goals grounded by current believed evidence
9. Enterprise goals never outrank `Critical` self-care or danger goals
10. `visible_hostiles_for()` and `current_attackers_of()` are local in semantics
11. Danger is derived from local hostile evidence, never from a stored fear scalar
12. Planner uses only relevant `PlannerOpKind`s for each goal family
13. Planner honors materialization barriers and preserves the top-level goal across them
14. Budget exhaustion returns no invalid partial plan
15. Failure writes a concrete `BlockedIntent`
16. `BlockedIntent` suppresses immediate retries against the same blocker
17. `BlockedIntent` clears on world change or expiry
18. Current valid plan is not replaced by same-class noise unless it beats the switch margin
19. `TradeAcquire`, `Harvest`, `Craft`, and similar barrier steps trigger follow-up replanning
20. Agents prefer controlled stock before external procurement
21. Merchants can fetch their own off-site stock before buying or producing
22. Dead agents generate no goals or plans
23. Human / AI control swap preserves same legal action set

## Files to Touch

- `crates/worldwake-ai/tests/phase2_gate.rs` (new)
- `crates/worldwake-ai/tests/belief_audit.rs` (new)
- `crates/worldwake-ai/tests/spec_checklist.rs` (new)

## Out of Scope

- Fixing any bugs found by these tests (those would be addressed in the relevant ticket)
- E14 per-agent beliefs
- Phase 3+ gate tests
- Performance benchmarking

## Acceptance Criteria

### Tests That Must Pass

1. All 22 spec-listed tests pass
2. Phase 2 gate: agents autonomously eat, drink, sleep, wash, relieve
3. Phase 2 gate: merchants restock via lawful physical paths
4. Phase 2 gate: agents don't thrash between equal plans
5. Phase 2 gate: agents don't retry same blocked target every tick
6. Phase 2 gate: world runs 24+ in-world hours without AI deadlock
7. Phase 2 gate: survival-critical needs consistently outrank commerce and loot
8. Phase 2 gate: combat as leaf commitment only
9. Belief audit: zero direct `&World` references in `worldwake-ai`
10. Existing suite: `cargo test --workspace`

### Invariants

1. No `&World` in `worldwake-ai` source (only `&dyn BeliefView`)
2. `AgentDecisionRuntime` not in component tables
3. `PlanningState`/`PlanningSnapshot` not in component tables
4. All deterministic: BTreeMap/BTreeSet only, no floats, no HashMap/HashSet

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/phase2_gate.rs` — multi-tick integration tests
2. `crates/worldwake-ai/tests/belief_audit.rs` — source code audit
3. `crates/worldwake-ai/tests/spec_checklist.rs` — all 22 spec test items

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
