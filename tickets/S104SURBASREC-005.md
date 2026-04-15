# S104SURBASREC-005: Layer 0 survival baseline golden tests

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: archive/tickets/S104SURBASREC-001.md, archive/tickets/S104SURBASREC-002.md, S104SURBASREC-004

## Problem

After golden test triage (001, 002) removes hash-dependent tests and the survival scenario (004) proves agents can survive, the project needs a permanent regression test that pins survival behavior with invariant-based assertions. This is Layer 0 of the golden test rebuild — the foundation that all subsequent layers build upon. Without it, future changes could silently break survival again.

## Assumption Reassessment (2026-04-15)

1. Golden test infrastructure exists and is unchanged: `golden_harness/mod.rs`, `golden_harness/soak_world.rs`, `golden_harness/timeline.rs` — confirmed during reassessment. The harness provides `GoldenTestBuilder`, scenario loading, tick advancement, and assertion utilities.
2. `scenarios/survival-baseline.ron` will exist after S104SURBASREC-004 — this ticket depends on it.
3. After S104SURBASREC-001 and S104SURBASREC-002, the golden test file namespace has room for `golden_survival_baseline.rs`.
4. `HomeostaticNeedId` variants (Hunger, Thirst, Fatigue, Bladder, Dirtiness) and their accessors are available through the belief view and world state for assertion purposes.
5. The spec mandates NO StateHash assertions in Layer 0 tests — only structural invariants.
6. Factual follow-up from archived `S104SURBASREC-001`: `crates/worldwake-ai/tests/` uses file-based Rust integration tests. A new `golden_survival_baseline.rs` file should declare its own local `mod golden_harness;` like the surviving golden files, not rely on a shared test-harness entry point.

## Architecture Check

1. Invariant-based assertions are more resilient to behavioral changes than hash-based assertions. This is the core lesson from the golden test triage — tests should verify "agents survive" not "agents take exactly this sequence of actions."
2. No backwards-compatibility shims. New test file following existing golden test conventions.

## Verification Layers

1. All agents alive at tick 1440 → authoritative world state assertion
2. No need saturated above critical for extended periods → authoritative world state sampling
3. All agents performed survival actions → event-log / action trace assertions
4. Agent B explored and discovered food → belief state / event-log assertion
5. Single-layer ticket — test infrastructure only, verifying emergent survival behavior.

## What to Change

### 1. Create `golden_survival_baseline.rs`

New file: `crates/worldwake-ai/tests/golden_survival_baseline.rs`

Tests (3-4, all 1440-tick runs using `survival-baseline.ron`):

**Test 1: `all_agents_survive_1440_ticks`**
- Load `survival-baseline.ron`, run 1440 ticks
- Assert: zero deaths (all 3 agents alive at end)
- Assert: no agent's any need was above Permille 750 for more than 100 consecutive ticks

**Test 2: `all_agents_perform_survival_actions`**
- Load `survival-baseline.ron`, run 1440 ticks
- Assert: every agent executed at least one Eat, Drink, Sleep, Relieve, and Wash action
- Use event log or action trace to verify action types occurred

**Test 3: `explorer_discovers_food_source`**
- Load `survival-baseline.ron`, run 1440 ticks
- Assert: Agent B (starts knowing only Riverside Camp) discovers and travels to a place with food production capability
- Verify via belief state: Agent B knows at least one place with FieldPlot or OrchardRow

**Test 4: `no_budget_exhaustion_on_survival_goals`**
- Load `survival-baseline.ron`, run 1440 ticks
- Assert: no planner budget exhaustion events for survival-related goal kinds (AcquireCommodity with SelfConsume purpose, ConsumeOwnedCommodity, Sleep, Relieve, ExploreLocation)

**All tests**: NO StateHash assertions. Only structural invariants.

### 2. Wire the new file like existing golden integration tests

Give `golden_survival_baseline.rs` its own local `mod golden_harness;` declaration and imports consistent with the surviving golden files in `crates/worldwake-ai/tests/`.

## Files to Touch

- `crates/worldwake-ai/tests/golden_survival_baseline.rs` (new)

## Out of Scope

- Modifying the survival baseline scenario (authored in S104SURBASREC-004)
- Adding system-specific tests (Layer 1 — S104SURBASREC-006)
- Adding StateHash or determinism assertions (Layer 3 — S104SURBASREC-006)
- Modifying golden harness infrastructure
- Any production code changes

## Acceptance Criteria

### Tests That Must Pass

1. `all_agents_survive_1440_ticks` — zero deaths, no critical need saturation
2. `all_agents_perform_survival_actions` — all 5 action types observed per agent
3. `explorer_discovers_food_source` — Agent B discovers food via exploration
4. `no_budget_exhaustion_on_survival_goals` — no planner exhaustion on survival goals
5. Existing suite: `cargo test -p worldwake-ai` — all tests pass (KEEP + TRIAGE survivors + new Layer 0)

### Invariants

1. No StateHash calls in this test file — invariant-only assertions
2. Tests use `survival-baseline.ron` scenario, not custom-built sterile setups
3. All assertions verify emergent behavior from realistic starting conditions (FND-01, FND-31)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_baseline.rs` — Layer 0 survival regression tests proving agents can bootstrap survival from realistic starting conditions with only survival profiles

### Commands

1. `cargo test -p worldwake-ai -- golden_survival_baseline` — targeted Layer 0 tests
2. `cargo test -p worldwake-ai` — full AI crate suite
3. `cargo clippy --workspace --all-targets -- -D warnings` — clean
