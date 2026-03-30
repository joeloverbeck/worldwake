# E19GUAPAT-007: Golden tests for patrol cycle, belief-driven intensity, and feedback loop

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only ticket
**Deps**: E19GUAPAT-001 through E19GUAPAT-006 (all patrol infrastructure must be delivered)

## Problem

The patrol system needs end-to-end golden tests verifying: (a) a guard completes a full patrol cycle visiting waypoints in order, (b) patrol urgency scales with the guard's belief state (crime reports, office vacancy), (c) the public order feedback loop converges (more crime → more patrols → fewer crimes → fewer patrols), and (d) route adaptation responds to crime reports. These tests validate that the entire pipeline works together — from candidate generation through action execution to world state changes.

## Assumption Reassessment (2026-03-30)

1. Golden test patterns follow the existing `crates/worldwake-ai/tests/golden_*.rs` files. Tests use a harness (`GoldenTestHarness` or similar) that sets up a world, runs ticks, and asserts outcomes.
2. Decision traces (`h.driver.enable_tracing()`) and action traces (`h.enable_action_tracing()`) are available for debugging.
3. `PerceptionProfile` must be set on agents that need to observe post-production output or any events (per CLAUDE.md: "Golden production tests require PerceptionProfile").
4. 1-tick actions use state-delta observation; multi-tick actions can use `agent_active_action_name()`.
5. Patrol is a multi-tick action (dwell phase) so `agent_active_action_name()` should work for observing active patrol.
6. The feedback loop test requires both guards and a "thief" agent whose behavior is affected by guard presence. This depends on the thief's belief system from E17 — thieves avoid guarded places via their own beliefs.
7. The spec's Canonical Regression Scenario F (Office Vacancy → Succession Delay → Patrol Gap → Route Predation) describes the target golden scenario class.
8. The implemented core patrol route shape is still only `assigned_places` plus `current_index`. Golden tests should assert on that public authoritative contract and observable behavior, not invent stronger expectations about route-entry metadata that the engine does not yet model.
9. No adjacent contradictions found.

## Architecture Check

1. Golden tests as integration tests in `crates/worldwake-ai/tests/` follow the established pattern. These tests exercise the full pipeline: candidate generation → plan search → action execution → world state mutation.
2. Testing the feedback loop requires enough simulation ticks for convergence. The test should assert directional change (patrol motive increases after crime, decreases after period of no crime) rather than exact numeric values.
3. Route assertions should stay at the current authoritative surface: waypoint order membership and `current_index`. If a future patrol ticket upgrades the route model, the golden tests can move with that new contract then.
4. No backwards-compatibility shims.

## Verification Layers

1. Patrol cycle completion → authoritative world state: `PatrolRoute.current_index` advances through all waypoints and wraps
2. Belief-driven intensity → decision trace: patrol motive value is higher with unresolved violations than without
3. Route adaptation → authoritative world state: `PatrolRoute.assigned_places` grows after crime report
4. Public order feedback → derived view: `public_order(place)` rises with guard presence, falls without
5. Information locality → decision trace: guard at remote location has base urgency despite crimes elsewhere

## What to Change

### 1. New golden test file: `crates/worldwake-ai/tests/golden_patrol.rs`

**Test scenarios:**

#### a. `golden_patrol_cycle`
- Setup: 1 guard with 3-waypoint PatrolRoute, PatrolProfile, at waypoint 0
- Run: step ticks until guard completes full cycle (visits all 3 waypoints)
- Assert: `current_index` wraps back to 0 after visiting all waypoints
- Assert: action trace shows Patrol actions committed at each waypoint

#### b. `golden_patrol_interrupted_resumes`
- Setup: 1 guard mid-patrol, introduce critical need (starvation)
- Run: step ticks, guard interrupts patrol to eat, then resumes
- Assert: `current_index` unchanged during interruption, patrol resumes from same waypoint

#### c. `golden_patrol_belief_urgency`
- Setup: 2 guards — one with unresolved violations in ViolationMemory, one without
- Run: step 1 tick of candidate generation
- Assert: decision trace shows guard with violations has higher patrol motive

#### d. `golden_patrol_route_adaptation`
- Setup: 1 guard with 2-waypoint route. Crime report arrives (via Tell) about a 3rd location.
- Run: step enough ticks for route adaptation
- Assert: `PatrolRoute.assigned_places` now includes the 3rd location

#### e. `golden_patrol_feedback_loop`
- Setup: settlement with 1 guard, 1 potential thief, crime-prone location
- Run: extended simulation
- Assert: directional convergence — public_order rises with guard patrols, crime rate decreases

#### f. `golden_patrol_locality` (Principle 7)
- Setup: 1 guard at location A, crime occurs at location B (no information carrier connects them)
- Run: step ticks
- Assert: guard's patrol motive unchanged, route unchanged — no omniscient information

## Files to Touch

- `crates/worldwake-ai/tests/golden_patrol.rs` (new)

## Out of Scope

- Modifying any patrol implementation code (all infrastructure tickets already delivered)
- Thief AI behavior changes (E17 scope)
- Captain-mediated route changes (future epic)
- Performance benchmarks
- Save/load replay tests for patrol (could be a follow-up)
- Asserting on any richer patrol route-entry metadata than the current `assigned_places` + `current_index` model

## Acceptance Criteria

### Tests That Must Pass

1. `golden_patrol_cycle` — guard visits all waypoints in order, index wraps
2. `golden_patrol_interrupted_resumes` — interrupted guard resumes from correct waypoint
3. `golden_patrol_belief_urgency` — crime reports increase patrol motive
4. `golden_patrol_route_adaptation` — crime at new location extends guard's route
5. `golden_patrol_feedback_loop` — public order rises with guard patrols (directional)
6. `golden_patrol_locality` — guard ignorant of remote crimes (Principle 7)
7. Existing suite: `cargo test -p worldwake-ai`
8. `cargo clippy --workspace`

### Invariants

1. Golden tests use decision traces and/or action traces for debugging (per CLAUDE.md)
2. All test agents have `PerceptionProfile` (per CLAUDE.md golden test requirement)
3. Tests are deterministic (seeded RNG, BTreeMap, no floats, no wall-clock time)
4. Tests assert on belief state and authoritative world state, not on internal planning details
5. Feedback loop test asserts directional convergence, not exact values
6. Route assertions stay on the current minimal authoritative contract until a later ticket explicitly upgrades it

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_patrol.rs` — 6 golden test scenarios as described above

### Commands

1. `cargo test -p worldwake-ai -- golden_patrol`
2. `cargo clippy --workspace && cargo test --workspace`
