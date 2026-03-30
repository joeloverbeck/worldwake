# E20COMBEH-008: Golden tests — social and need continuity (witness, no-witness, continuity)

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — tests only
**Deps**: E20COMBEH-004 (wilderness relief action with SamePlace visibility), E14 (perception system)

## Problem

Wilderness relief social consequences and need continuity need golden E2E coverage to verify that: (a) co-located agents observe wilderness relief events via the perception pipeline, (b) agents who relieve in isolation produce no social consequences, and (c) bladder resets are exact and dirtiness penalties are correctly applied across all relief paths. These are spec-required golden tests T-WitnessObservation, T-NoWitness, and T-NeedContinuity.

## Assumption Reassessment (2026-03-30)

1. **Perception system (E14)**: Processes events with `VisibilitySpec::SamePlace` by checking for agents co-located with the event's place. Agents with `PerceptionProfile` form beliefs about observed events. This is existing E14 infrastructure.
2. **PerceptionProfile requirement**: Golden tests that need agents to observe events MUST assign `PerceptionProfile` to those agents (per CLAUDE.md: "Golden production tests require `PerceptionProfile` on agents that need to observe post-production output"). T-WitnessObservation requires this on the witness agent.
3. **VisibilitySpec::SamePlace**: Confirmed on `relieve_wilderness` action (set in E20COMBEH-004).
4. **Belief formation**: After perception processes an event, the observing agent gains a belief about the event (event tag, actor, place, tick). Belief queries available via the belief view system.
5. **Need continuity**: After any relief (toilet, wilderness, accident), bladder must be exactly `Permille(0)`. Dirtiness must reflect the appropriate penalty: 0 for toilet, `wilderness_relief_dirtiness_penalty` for wilderness, max for accident.
6. **Scenario isolation for T-WitnessObservation**: Two agents at the same outdoor place. One relieves. The other (with PerceptionProfile) should form a belief. For T-NoWitness: one agent alone at outdoor place relieves. No other agent present. No beliefs formed elsewhere.

## Architecture Check

1. Three focused golden tests. T-WitnessObservation and T-NoWitness test the perception pipeline's handling of the new action — they are the social consequence verification layer. T-NeedContinuity is a cross-cutting invariant check across all relief paths.
2. No backward-compatibility shims. These are new tests.

## Verification Layers

1. Witness observation → belief state on observer agent (belief about WildernessRelief event exists)
2. No-witness isolation → belief state on all other agents (no WildernessRelief belief exists)
3. Need continuity → authoritative world state (exact bladder = 0, exact dirtiness delta after each relief type)
4. These are golden E2E tests — they verify the full perception + needs pipeline.

## What to Change

### 1. T-WitnessObservation golden test

**Setup**: Two agents at the same outdoor place (e.g., `Forest`). Agent A has high bladder. Agent B has `PerceptionProfile` and is idle (no action). Agent A's bladder crosses critical → planner chooses `relieve_wilderness` → action executes.

**Assert**: After the event commits and perception system runs:
- Agent B has a belief about a `WildernessRelief` event involving Agent A at the current place.
- Agent A's bladder = `Permille(0)`.
- Waste entity exists at the place.

### 2. T-NoWitness golden test

**Setup**: One agent alone at an outdoor place (e.g., `Trail`). No other agents at the same place. Agent has high bladder → `relieve_wilderness` executes.

**Assert**: After the event commits:
- No other agent has any belief about the `WildernessRelief` event.
- Physical consequences still exist: waste at place, dirtiness on agent.
- Agent's bladder = `Permille(0)`.

### 3. T-NeedContinuity golden test

**Setup**: Three scenarios in one test (or three sub-tests):
- (a) Agent uses `toilet` → bladder = 0, dirtiness unchanged.
- (b) Agent uses `relieve_wilderness` → bladder = 0, dirtiness += penalty.
- (c) Agent suffers deprivation accident → bladder = 0 (or handled by needs system), dirtiness at max consequence level.

**Assert**: In all three cases, bladder is exactly `Permille(0)` after relief. No partial resets. Dirtiness reflects the exact expected penalty for each path.

## Files to Touch

- `crates/worldwake-ai/src/golden_tests/` (new test file or addition to existing golden test module)

## Out of Scope

- Travel physiology golden tests (E20COMBEH-006)
- Relief fallback golden tests (E20COMBEH-007)
- Unit tests for individual components (covered in E20COMBEH-001 through E20COMBEH-005)
- Changes to production code (this ticket is tests only)
- Tell action propagation (E15 — mentioned in spec as a downstream mechanism but not tested here; belief formation from direct observation is sufficient)
- Belief provenance metadata verification (beyond scope — existence of belief is sufficient)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_witness_observation` — co-located agent with PerceptionProfile forms belief about WildernessRelief event
2. `golden_no_witness` — agent alone produces no beliefs in other agents; physical consequences still exist
3. `golden_need_continuity` — bladder is exactly Permille(0) after toilet, wilderness, and accident; dirtiness reflects correct penalty per path
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. VisibilitySpec::SamePlace is respected — only co-located agents observe
2. Perception requires PerceptionProfile — agents without it never form beliefs
3. Bladder is never partially reset — always exactly Permille(0) after any relief
4. Dirtiness penalty is path-specific: 0 for toilet, wilderness_relief_dirtiness_penalty for wilderness, deprivation consequence level for accident

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/golden_tests/` — `golden_witness_observation` — perception pipeline for wilderness relief
2. `crates/worldwake-ai/src/golden_tests/` — `golden_no_witness` — isolation / no social consequence
3. `crates/worldwake-ai/src/golden_tests/` — `golden_need_continuity` — cross-path bladder and dirtiness invariants

### Commands

1. `cargo test -p worldwake-ai golden_witness`
2. `cargo test -p worldwake-ai golden_no_witness`
3. `cargo test -p worldwake-ai golden_need_continuity`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
