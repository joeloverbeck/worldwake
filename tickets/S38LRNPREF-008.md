# S38LRNPREF-008: Golden tests — experience-driven route and source preferences

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S38LRNPREF-004, S38LRNPREF-005, S38LRNPREF-006, S38LRNPREF-007

## Problem

No end-to-end validation exists for the full experience-driven decision cycle: action outcome → experience recording → belief view → ranking/cost influence → different planning outcome. Golden tests verify that the entire chain works together to produce emergent preference behavior.

## Assumption Reassessment (2026-04-02)

1. Golden tests live in `crates/worldwake-ai/tests/` as integration tests — verified pattern from existing golden tests (e.g., `golden_merchant_selling.rs`, `golden_emergent.rs`).
2. Golden tests require `PerceptionProfile` on agents that need to observe post-action output — noted in CLAUDE.md.
3. Golden test setup requires: world with topology (multiple routes), agents with `PreferenceProfile`, action registration, and enough simulation ticks for travel + recording + replanning.
4. All prerequisite tickets (001–007) deliver the infrastructure: types, eviction, belief view, recording, ranking.
5. Deterministic replay requires `ChaCha8Rng` seeded — standard golden test pattern.
6. `SAVE_FORMAT_VERSION` bumped to 13 in S38LRNPREF-001 — golden tests use current format.

## Architecture Check

1. Golden tests exercise the full vertical slice without mocking. They prove that the systems compose correctly through state-mediated interaction (P26), not just that individual units work.
2. Three scenarios cover the spec's key behavioral claims: (a) hostile experience → safer route preference, (b) combat abort → same recording effect, (c) PreferenceProfile diversity → different choices.
3. No backward-compatibility shims.

## Verification Layers

1. Attack → hostile recording → safer route preference → decision trace showing cost penalty influenced route choice
2. Combat abort → hostile recording → safer route preference → decision trace
3. PreferenceProfile diversity → different route choices → decision trace showing different cost penalties for same topology
4. Multi-layer ticket: action trace (recording), authoritative world state (experience components), decision trace (ranking influence), event-log delta (combat events).

## What to Change

### 1. Golden test: attack during travel → safer route preference

Setup:
- World with place A, B, C. Two routes: A→B→C (short, hostile history) and A→D→C (long, no history).
- Agent at A with `PreferenceProfile` (meaningful `route_caution_weight`), `PerceptionProfile`.
- Simulate: agent travels A→B, combat event occurs during travel, travel completes.
- Assert: `RouteExperience` records hostile encounter for A→B edge.
- Simulate: agent needs to travel to C again.
- Assert: agent chooses A→D→C (longer but no hostile experience) over A→B→C.

### 2. Golden test: combat-aborted travel → safer route preference

Setup: Similar to test 1 but travel is aborted (not completed) due to combat.
- Assert: `RouteExperience` still records hostile encounter (P10 — failure is new state).
- Assert: agent prefers safer route on replan.

### 3. Golden test: PreferenceProfile diversity → different route choices

Setup:
- Same topology with one edge having hostile experience recorded for both agents.
- Agent 1: high `route_caution_weight` (Permille(800)).
- Agent 2: low `route_caution_weight` (Permille(100)).
- Both need to travel to the same destination.
- Assert: Agent 1 chooses the longer safe route, Agent 2 chooses the shorter dangerous route (penalty too small to overcome distance advantage).

### 4. Deterministic replay companion

For each golden test, verify that replaying with the same seed produces identical outcomes.

## Files to Touch

- `crates/worldwake-ai/tests/golden_experience_preferences.rs` (new)

## Out of Scope

- Source reliability golden tests (could be added as follow-up — the focused tests in S38LRNPREF-005 and S38LRNPREF-007 cover the unit behavior)
- Post-load pruning integration test (separate follow-up if needed)
- Performance benchmarks for experience-heavy scenarios

## Acceptance Criteria

### Tests That Must Pass

1. Golden: attack during travel → agent prefers safer alternative route
2. Golden: combat-aborted travel → agent prefers safer alternative route
3. Golden: two agents with different `PreferenceProfile` weights make different route choices
4. All golden tests are deterministic replay companions (same seed → same outcome)
5. Existing suite: `cargo test --workspace`

### Invariants

1. Experience recording flows through action handlers → components → belief view → ranking (full vertical slice)
2. Route preference is a tie-breaking influence — does not suppress routes, only adjusts cost
3. Agents without `PreferenceProfile` are unaffected by experience (backward-compatible behavior)
4. All golden tests use seeded `ChaCha8Rng` for determinism

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_experience_preferences.rs` (new) — 3 golden scenarios + replay companions

### Commands

1. `cargo test -p worldwake-ai golden_experience`
2. `cargo test -p worldwake-ai` (full AI suite including all existing golden tests)
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
