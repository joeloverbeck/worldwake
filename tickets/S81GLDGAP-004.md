# S81GLDGAP-004: Golden test S81-A -- multi-agent convergence

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: None

## Problem

No golden test verifies multi-agent behavior under resource scarcity at scale. S76-B (`golden_max_idle_under_remote_resource_scarcity`) tests 1 agent for 300 ticks. The observer report shows qualitatively different failure at 3+ agents / 600+ ticks due to candidate explosion and contention effects. This gap means multi-agent behavioral collapse (prolonged sleep+relieve loops) could regress undetected.

## Assumption Reassessment (2026-04-09)

1. S76-B test exists at `crates/worldwake-ai/tests/golden_simulation_gaps.rs:387` (`golden_max_idle_under_remote_resource_scarcity`). Helper at line 203 (`run_max_idle_under_remote_resource_scarcity`). Confirmed via grep. Uses 1 agent, 300 ticks, single remote resource location.
2. Golden test infrastructure: `golden_simulation_gaps.rs` already exists and is the correct file for S81 golden tests. S81-A test will be added to this file.
3. `GoalKind::AcquireCommodity`, `ConsumeOwnedCommodity`, `Sleep`, `Relieve` all exist in `crates/worldwake-core/src/goal.rs`. Confirmed via grep.
4. `ActionDomain::Travel`, `Production`, `Needs` all exist in `crates/worldwake-core/src/action_domain.rs`. Confirmed via grep.
5. `WorkstationTag::Well` and `OrchardRow` exist in `crates/worldwake-core/src/production.rs`. `KnownRecipes` component exists for recipe knowledge. Confirmed via grep.
12. Scenario isolation: this test intentionally seeds beliefs about remote resources to isolate the travel-decision branch. Without seeded beliefs, agents cannot know about remote resources (P14) and would only sleep/relieve. The test verifies agents ACT on those beliefs, not that they discover resources independently (that is S80 exploration mechanics).

## Architecture Check

1. Multi-agent golden test extends the existing S76-B pattern. Reuses the same golden test infrastructure (world builder, harness, assertion helpers). No new framework needed.
2. No backward-compatibility shims.

## Verification Layers

1. No agent enters >200 consecutive idle ticks -> authoritative action trace (tick-by-tick action domain tracking)
2. At least one agent starts travel within 300 ticks -> action trace (ActionDomain::Travel committed)
3. At least one agent reaches resource location by tick 600 -> authoritative world state (effective_place query)
4. Single-layer golden E2E ticket: the contract is emergent multi-system behavior, not a single system invariant.

## What to Change

### 1. Add S81-A golden test to golden_simulation_gaps.rs

In `crates/worldwake-ai/tests/golden_simulation_gaps.rs`, add:

- Helper function `run_multi_agent_convergence(seed: Seed)`:
  - Create 3 agents at a barren indoor location (e.g., VILLAGE_SQUARE or equivalent with no food/water sources)
  - Create remote places with food (OrchardRow + Apple source) and water (Well + Water source) resource sources
  - Seed each agent with beliefs about at least one remote resource location
  - Give each agent `KnownRecipes` for harvest actions
  - Set elevated hunger/thirst needs
  - Run for 600 ticks
  - Track per-agent consecutive idle ticks (only sleep/relieve actions)
  - Assert: no agent exceeds 200 consecutive idle ticks
  - Assert: at least one travel action started within 300 ticks
  - Assert: at least one agent at a resource-bearing location by tick 600

- Test function `golden_multi_agent_convergence()` with `#[test]`
- Deterministic replay test `golden_multi_agent_convergence_replays_deterministically()` following the existing pattern

### 2. Follow existing S76-B setup patterns

Use the same world-builder utilities, topology setup, and assertion patterns from `run_max_idle_under_remote_resource_scarcity` as a template. Scale up: 3 agents instead of 1, 600 ticks instead of 300, multiple remote resource locations.

## Files to Touch

- `crates/worldwake-ai/tests/golden_simulation_gaps.rs` (modify -- add S81-A test)

## Out of Scope

- Fixing root causes of idle loops (S79, S80)
- Plan search budget tuning (CognitiveProfile already supports this)
- Observer tooling improvements
- Single-agent scarcity testing (already covered by S76-B)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_multi_agent_convergence` -- no agent >200 consecutive idle ticks, travel within 300 ticks, resource location reached by 600
2. `golden_multi_agent_convergence_replays_deterministically` -- same seed produces same outcome
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Agents plan from beliefs only, never world state (P14) -- beliefs are seeded, not derived from omniscient access
2. All agent actions use the same lawful affordances (P19 agent symmetry)
3. Deterministic replay under same seed (ChaCha8Rng, BTreeMap)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_simulation_gaps.rs` -- 2 new test functions (golden + replay)

### Commands

1. `cargo test -p worldwake-ai -- golden_multi_agent_convergence`
2. `cargo test -p worldwake-ai`
