# S102FROAWAEXP-007: Golden tests for S102 frontier-aware exploration

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — test-only
**Deps**: archive/tickets/S102FROAWAEXP-001.md, archive/tickets/S102FROAWAEXP-002.md, archive/tickets/S102FROAWAEXP-003.md, archive/tickets/S102FROAWAEXP-004.md, archive/tickets/S102FROAWAEXP-005.md, archive/tickets/S102FROAWAEXP-006.md

## Problem

The S102 feature set (gate bypass, multi-hop BFS, belief protection, counter reset) needs end-to-end golden tests validating the complete agent decision cycle — from unmet need through exploration to resource discovery. Focused unit tests in tickets 001-006 verify individual components; golden tests verify they compose correctly into emergent exploration chains.

## Assumption Reassessment (2026-04-14)

1. Existing golden exploration tests at `crates/worldwake-ai/tests/golden_exploration.rs` — 2 `ExplorationProfile { ... }` construction sites. New fields from ticket 001 need spread or explicit values in test fixtures.
2. Golden test infrastructure: tests use `ProfileFixture` or direct `TestBeliefView` setup. The `TestBeliefView` in `candidate_generation.rs` (test module starting line 4921) provides `adjacent_places_with_travel_ticks()` with default 1-tick travel. Golden tests use the full action registry and `run_ticks()` harness.
3. `PerceptionProfile` is required on agents that need to observe post-production output (per CLAUDE.md: "Golden production tests require PerceptionProfile on agents that need to observe newly created entities").
4. S102 spec defines 4 golden test scenarios: (1) gate unlock after budget exhaustion, (2) multi-hop frontier discovery, (3) belief persistence across exploration rounds, (4) counter reset on need satisfaction.

## Architecture Check

1. Golden tests exercise the complete agent decision cycle (candidate generation → ranking → plan search → action execution → belief update). This is the appropriate proof surface for S102's emergent behavior — focused tests prove components, golden tests prove composition (FND-31).
2. No backward-compatibility shims. New golden tests, no modification of existing ones.

## Verification Layers

1. Gate unlock after budget exhaustion → golden E2E: agent transitions from repeated Sleep to ExploreLocation after N failures
2. Multi-hop discovery → golden E2E: agent discovers places at frontier_depth 2 and acquires resource
3. Belief persistence → golden E2E: explored place belief survives across exploration rounds
4. Counter reset → golden E2E: tracker count returns to 0 after need satisfaction
5. Verification is golden/E2E layer — appropriate because the contract is emergent composition, not individual function behavior

## What to Change

### 1. Golden Test 1: Planner Failure Unlocks Exploration

Setup:
- 2 places: Village (with Well), Trail (nothing)
- Agent at Trail, hunger=800‰
- Agent believes Grain exists at Village (seeds `need_has_known_acquisition_path()` true)
- CognitiveProfile with low `max_node_expansions` to force BudgetExhausted on AcquireCommodity
- `acquisition_failure_threshold: 3`

Assert:
- After 3 budget-exhausted plan attempts, ExploreLocation fires (decision trace)
- Agent travels to Village, discovers Grain, satisfies hunger
- Without S102 gate bypass, agent would loop Sleep indefinitely

### 2. Golden Test 2: Multi-Hop Frontier Discovery

Setup:
- 3 places: Forest → Village → Fields (with FieldPlot/ResourceSource)
- Agent at Forest, hunger=800‰, known_recipes=["Harvest Grain"]
- Agent knows only Forest
- `frontier_depth: 2`
- Need high enough to trigger exploration immediately

Assert:
- Exploration candidates include Village (1 hop) and Fields (2 hops)
- Agent explores Village first (closer), then Fields
- At Fields, discovers FieldPlot, harvests Grain
- With `frontier_depth: 1`, agent cannot reach Fields (regression guard)

### 3. Golden Test 3: Exploration-Chain Belief Persistence

Setup:
- 3 places: Forest → Village → Inn (with WashBasin)
- Agent at Forest, dirtiness=800‰
- `exploration_arrival_boost: 500` (4 synthetic ticks)
- `frontier_depth: 2`
- S101 decay active

Assert:
- Agent explores Village (1 hop)
- Village belief persists through next decision cycle (synthetic ticks resist decay)
- Agent explores Inn (2nd hop), discovers WashBasin, washes

### 4. Golden Test 4: Counter Reset on Need Satisfaction

Setup:
- Same as Test 1 but extended: after agent finds food and eats
- Hunger drops below `need_activation_threshold`

Assert:
- `AcquisitionExhaustionTracker` count for Hunger resets to 0
- Agent does not spuriously explore for food when hunger is satisfied

## Files to Touch

- `crates/worldwake-ai/tests/golden_exploration.rs` (modify — add 4 new test functions)

## Out of Scope

- Need-directed exploration targeting
- Changing ExploreLocation dispatch, plan, or execution mechanics
- Modifying S101 decay rates or S100 retention windows
- "Scout" or "cartography" actions

## Acceptance Criteria

### Tests That Must Pass

1. `golden_s102_gate_unlock_after_budget_exhaustion` — agent explores after N failures
2. `golden_s102_multi_hop_frontier_discovery` — agent discovers resources at depth 2
3. `golden_s102_exploration_chain_belief_persistence` — belief survives decay for multi-hop chain
4. `golden_s102_counter_reset_on_need_satisfaction` — tracker resets when need satisfied
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Agents plan from beliefs only, never world state (FND-14)
2. All resource discovery occurs through physical travel and local perception (FND-07, FND-15)
3. Exploration competes in normal ranking pipeline — not force-activated (FND-20)
4. Agent diversity: different `frontier_depth` / `acquisition_failure_threshold` values produce different exploration behavior (FND-22)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_exploration.rs::golden_s102_gate_unlock_after_budget_exhaustion` — validates the core S102 motivating scenario
2. `crates/worldwake-ai/tests/golden_exploration.rs::golden_s102_multi_hop_frontier_discovery` — validates multi-hop BFS target selection end-to-end
3. `crates/worldwake-ai/tests/golden_exploration.rs::golden_s102_exploration_chain_belief_persistence` — validates belief protection enables multi-round exploration
4. `crates/worldwake-ai/tests/golden_exploration.rs::golden_s102_counter_reset_on_need_satisfaction` — validates counter lifecycle

### Commands

1. `cargo test -p worldwake-ai -- golden_s102`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
