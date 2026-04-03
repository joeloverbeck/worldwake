# PERAGRSTY-003: Golden test proving reasoning style diversity

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: PERAGRSTY-002

## Problem

After PERAGRSTY-001 and -002, agents can have different `ReasoningProfile` values, but no test proves that these differences produce observable behavioral divergence. Without a golden test, a future regression could silently ignore per-agent profiles and nobody would notice. This ticket adds at least one golden E2E test proving that two agents with different reasoning profiles make different decisions from identical starting conditions.

## Assumption Reassessment (2026-04-03)

1. After PERAGRSTY-002, `ReasoningProfile` is resolved per-agent from the world's component tables. Agents without an explicit profile get `ReasoningProfile::default()`.
2. Golden tests live in `crates/worldwake-ai/tests/` as `golden_*.rs` files. They use `build_prototype_world()` from `crates/worldwake-core/src/topology.rs:442` and construct agents with various profiles.
3. `PerceptionProfile` is required on agents that need to observe post-production output (per CLAUDE.md golden test note). Any agent in the golden test that must perceive the world needs this profile.
4. `compare_goal_switch()` at `crates/worldwake-ai/src/goal_switching.rs:10` uses margin parameter from `goal_switch_margin_details()`. With different `switch_margin` values, agents will switch goals at different thresholds.
5. `search_plan()` at `crates/worldwake-ai/src/search/mod.rs:77` uses `max_node_expansions` (line 123) as the primary budget cutoff. An agent with 32 expansions will exhaust its budget much sooner than one with 224, potentially failing to find longer plans.
6. Deterministic replay requires `ChaCha8Rng` seeding. Golden tests use deterministic seeds.
7. Not a stale-request/contested-affordance/political/ControlSource/heuristic-removal ticket — domain-specific precision items 8-15 are N/A.
8. Golden scenario design: both scenarios isolate the `ReasoningProfile` difference as the sole variable. Same place, same needs, same beliefs, same action registries. The only difference is the profile values.

## Architecture Check

1. Golden tests are the established E2E proof surface for agent behavioral contracts in this project. A reasoning-diversity golden test fits naturally alongside existing golden tests for production, combat, trade, etc.
2. No backward-compatibility shims. This is a pure test addition.

## Verification Layers

1. Agent A (flighty, `switch_margin: 50`) switches goals when a higher-motive challenger appears -> decision trace shows goal switch
2. Agent B (stubborn, `switch_margin: 300`) stays committed under the same conditions -> decision trace shows no switch
3. Agent C (impulsive, `max_node_expansions: 32`) fails to find a 4-step plan or finds a shorter fallback -> decision trace shows search exhaustion or shorter plan
4. Agent D (thorough, `max_node_expansions: 224`) finds the full plan -> decision trace shows successful plan with expected steps
5. Deterministic replay -> same seed produces identical outcomes on re-run

## What to Change

### 1. Add golden test file

Create `crates/worldwake-ai/tests/golden_reasoning_diversity.rs`:

**Scenario 1 — Switch margin divergence:**
- Place two agents (A and B) at the same location with identical needs and beliefs.
- Agent A: `ReasoningProfile { switch_margin: Permille(50), ..Default::default() }` (flighty).
- Agent B: `ReasoningProfile { switch_margin: Permille(300), ..Default::default() }` (stubborn).
- Both agents should have `PerceptionProfile` so they can observe the world.
- Set up initial conditions where both agents adopt a goal (e.g., eat food at the market).
- After initial goal adoption, introduce a condition that creates a higher-motive challenger goal (e.g., danger appears, creating a flee motive).
- Tick enough steps for the goal-switching evaluation to fire.
- Assert: Agent A switches to the new goal. Agent B stays committed to the original.

**Scenario 2 — Search depth divergence:**
- Place two agents (C and D) at the same location with identical needs and beliefs.
- Agent C: `ReasoningProfile { max_node_expansions: 32, ..Default::default() }` (impulsive).
- Agent D: `ReasoningProfile::default()` (thorough, 224 expansions).
- Present a goal that requires a multi-step plan (e.g., acquire an item that requires traveling to another place, then purchasing).
- Tick through the planning phase.
- Assert: Agent D finds a plan with the expected number of steps. Agent C either fails to find a plan (budget exhaustion) or finds a shorter alternative.

Both scenarios use deterministic `ChaCha8Rng` seeding for replay.

### 2. Register test in workspace

Ensure the new test file is picked up by `cargo test -p worldwake-ai`. No `Cargo.toml` changes needed — files in `tests/` are auto-discovered.

## Files to Touch

- `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` (new)

## Out of Scope

- Testing all 12 `ReasoningProfile` fields individually (this proves the mechanism works; exhaustive field coverage is future work)
- Adding non-default profiles to agents in the CLI or scenario files (separate spec scope)
- Modifying `IntentionDispositionProfile` interaction (unchanged, already tested by existing goal-switching tests)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_reasoning_diversity::switch_margin_divergence` — flighty agent switches, stubborn agent stays
2. `golden_reasoning_diversity::search_depth_divergence` — impulsive agent fails/falls back, thorough agent succeeds
3. Deterministic replay: running twice with the same seed produces identical outcomes
4. Existing suite: `cargo test --workspace`

### Invariants

1. Only `ReasoningProfile` values differ between paired agents — all other setup is identical
2. Both scenarios are deterministic and replay-safe (seeded RNG, `BTreeMap` state)
3. `PerceptionProfile` is attached to all agents that need to observe the world

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` — two golden scenarios proving per-agent reasoning style produces observable behavioral divergence

### Commands

1. `cargo test -p worldwake-ai golden_reasoning_diversity`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
