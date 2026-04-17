# S107PRODIV-007: Golden tests — proactive diversification discovery, need-slack veto, cooldown

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — tests only
**Deps**: archive/tickets/S107PRODIV-005.md, archive/tickets/S107PRODIV-006.md

## Problem

Three golden E2E test scenarios are needed to validate proactive diversification behavior end-to-end: (1) an agent with DiversificationProfile discovers an unvisited place when needs are comfortable, (2) an agent with the profile but high needs never explores proactively, (3) exploration attempts are spaced by the cooldown parameter.

## Assumption Reassessment (2026-04-17)

1. Golden test pattern: `crates/worldwake-ai/tests/golden_exploration.rs` — existing exploration golden tests use scenario setup with TestBeliefView, run agent ticks, assert on goal kinds and target places. 19 ExploreLocation references.
2. Scenario infrastructure: golden tests construct worlds programmatically (not from RON files). They set up places, connections, facilities, agents, and profiles, then run `agent_tick` for N ticks and inspect results.
3. CLI wiring (ticket 005) ensures agents can be spawned with DiversificationProfile from RON scenarios, but golden tests construct agents programmatically and don't depend on CLI.
4. Golden tests DO depend on CLI for scenario-based E2E tests that load RON files. For S107, the golden tests can be either programmatic (like existing golden_exploration.rs tests) or RON-scenario-based. The spec describes 3 scenarios with specific assertions.

## Architecture Check

1. Golden tests exercise the full agent decision cycle end-to-end: candidate generation → ranking → goal selection → plan search → action execution → perception. They prove emergent behavior, not individual function correctness.
2. No backward-compatibility shims — new test file.

## Verification Layers

1. Proactive diversification discovery → golden E2E: agent with profile visits unvisited place after needs stabilize
2. Need-slack veto → golden E2E: agent with profile + high needs never emits proactive ExploreLocation
3. Cooldown enforcement → golden E2E: exploration attempts spaced by cooldown_ticks
4. Control case → golden E2E: agent WITHOUT profile never visits the unvisited place
5. Multi-dependency golden tests exercise tickets 001-006 simultaneously

## What to Change

### 1. Create golden test file

New file `crates/worldwake-ai/tests/golden_proactive_diversification.rs`.

### 2. Scenario: Proactive Diversification Discovery

Setup:
- 3 places: Home (satisfies all needs), Nearby (1 hop, no resources), Far (2 hops, alternative food)
- 2 agents: Explorer (with DiversificationProfile), Settler (without)
- Run for enough ticks that Explorer's needs stabilize and curiosity accumulates

Assertions:
- Explorer visits Far within N ticks after needs drop below comfort_threshold
- Settler never visits Far (needs are met at Home)

### 3. Scenario: Need-Slack Veto

Setup:
- 2 places with insufficient resources — needs always above comfort_threshold
- 1 agent with DiversificationProfile

Assertions:
- Zero proactive ExploreLocation goals emitted across entire run
- Agent still explores reactively when S80/S102 triggers fire

### 4. Scenario: Cooldown Enforcement

Setup:
- Multiple reachable places, agent with short cooldown (e.g., 10 ticks)
- Run for enough ticks to attempt multiple explorations

Assertions:
- Consecutive proactive exploration goals are spaced by at least exploration_cooldown_ticks

## Files to Touch

- `crates/worldwake-ai/tests/golden_proactive_diversification.rs` (new) — 3 golden test scenarios

## Out of Scope

- Focused unit tests for familiarity/novelty computation (ticket 006)
- RON scenario files for non-test use
- Observer binary integration

## Acceptance Criteria

### Tests That Must Pass

1. `golden_proactive_diversification::proactive_discovery` — explorer visits unvisited place, settler does not
2. `golden_proactive_diversification::need_slack_veto` — no proactive exploration under high need pressure
3. `golden_proactive_diversification::cooldown_enforcement` — exploration attempts respect cooldown interval
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Proactive exploration is agent-diversity-dependent (FND-22) — identical scenarios produce different behavior based on DiversificationProfile presence/absence
2. Need-slack veto is absolute — no proactive exploration when any need exceeds comfort_threshold
3. Cooldown is per-agent — no interaction between agents' exploration timing

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_proactive_diversification.rs` — 3 new golden E2E tests validating emergent proactive exploration behavior

### Commands

1. `cargo test -p worldwake-ai -- golden_proactive_diversification`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
