# S76GOLGAPSIOBS-003: Golden S76-D — utility profile diversity drives different actions

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

The simulation observer found all 3 AI agents collapsed into identical sleep+relieve patterns despite having different `UtilityProfile` weights (Finding 2). No golden test verifies that utility-profile diversity produces behavioral diversity. `golden_reasoning_diversity.rs` tests search-depth divergence (`CognitiveProfile.max_node_expansions`) but NOT utility-profile divergence. Without this regression guard, changes to ranking or candidate generation could silently collapse agent diversity.

## Assumption Reassessment (2026-04-09)

1. `UtilityProfile` exists at `crates/worldwake-core/src/utility_profile.rs:8-24` with 15 `Permille` weight fields including `hunger_weight`, `thirst_weight`, `fatigue_weight`. These are the parameters varied across agents.
2. `golden_reasoning_diversity.rs` (228 lines) contains `search_depth_divergence()` which varies `CognitiveProfile.max_node_expansions`. S76-D is distinct: it varies `UtilityProfile` weights to test goal-ranking diversity, not search-depth diversity.
3. Shared boundary: golden harness + ranking system. No production code changes.
5. GoalKinds under test: `ConsumeOwnedCommodity` (eat/drink), `Sleep`, `Relieve`. `generate_candidates()` at `candidate_generation.rs:187` produces these. `rank_candidates()` at `ranking.rs:81` scores them using `UtilityProfile` weights — this is the divergence point.
12. Scenario isolation: all 3 agents start at the same place with same need levels. Only `UtilityProfile` weights differ. Resources are sufficient for all agents so scarcity contention doesn't mask the diversity signal.

## Architecture Check

1. Adding to `golden_reasoning_diversity.rs` (228 lines) is appropriate — same domain (reasoning diversity), small file. Both search-depth and utility-profile diversity tests belong together.
2. No backwards-compatibility shims. Tests only.

## Verification Layers

1. Different agents produce different action sequences -> action trace (per-agent action distributions are not identical)
2. Utility profile weights drive the divergence -> decision trace (different `rank_candidates()` scores for the same `GroundedGoal` set)
3. Deterministic replay -> authoritative world state equality across two runs with same seed
6. Single-layer ticket (golden E2E tests only). No production code changes.

## What to Change

### 1. Implement S76-D scenario runner

Add to `crates/worldwake-ai/tests/golden_reasoning_diversity.rs`:

Create `run_utility_profile_diversity(seed: Seed)` returning an observation struct:

- Build 1 place with limited resources: 3 apple lots, 2 water lots, 1 bed.
- Spawn 3 AI agents with different `UtilityProfile` weights:
  - Agent A: hunger-prioritizing (high `hunger_weight`, e.g., `pm(900)`, others at `pm(300)`)
  - Agent B: thirst-prioritizing (high `thirst_weight`, e.g., `pm(900)`, others at `pm(300)`)
  - Agent C: fatigue-prioritizing (high `fatigue_weight`, e.g., `pm(900)`, others at `pm(300)`)
- All agents have `PerceptionProfile`, `CognitiveProfile` (identical across agents).
- All agents start with moderate levels of all needs: `pm(500)` each.
- Run for 200 ticks.
- Collect: per-agent action counts (eat, drink, sleep, relieve) and first non-relieve action.

### 2. Implement S76-D test and replay companion

```rust
// Scenario S76-D: Different Utility Profiles Produce Different Goal Orderings
#[test]
fn golden_utility_profile_diversity() { ... }

#[test]
fn golden_utility_profile_diversity_replays_deterministically() { ... }
```

Use `Seed([179; 32])`. Assert the 3 agents do NOT produce identical action sequences. At minimum, their first non-relieve action should differ, or their action distribution (eat vs. drink vs. sleep counts) should show measurable variance. Identical `sleep*N + relieve*M` patterns across all 3 agents fail the test.

## Files to Touch

- `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` (modify)

## Out of Scope

- Fixing the planner or ranking system — this test guards existing behavior
- Planner fallback testing (S76GOLGAPSIOBS-001)
- Perception belief coverage (S76GOLGAPSIOBS-002)
- Search-depth diversity (already covered by existing `search_depth_divergence()`)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_utility_profile_diversity` — 3 agents with different UtilityProfiles produce non-identical action sequences over 200 ticks
2. `golden_utility_profile_diversity_replays_deterministically` — identical observations across two runs
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No production code changes — engine behavior is unchanged
2. Deterministic replay: same seed produces identical observation structs
3. Behavioral diversity: different `UtilityProfile` weights produce measurably different action distributions (P22)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_reasoning_diversity.rs::golden_utility_profile_diversity` — regression guard for utility-profile-driven behavioral diversity
2. `crates/worldwake-ai/tests/golden_reasoning_diversity.rs::golden_utility_profile_diversity_replays_deterministically` — determinism guard

### Commands

1. `cargo test -p worldwake-ai golden_utility_profile_diversity`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
