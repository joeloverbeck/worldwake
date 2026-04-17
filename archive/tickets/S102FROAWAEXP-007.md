# S102FROAWAEXP-007: Golden tests for S102 frontier-aware exploration

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — test-only
**Deps**: archive/tickets/S102FROAWAEXP-001.md, archive/tickets/S102FROAWAEXP-002.md, archive/tickets/S102FROAWAEXP-003.md, archive/tickets/S102FROAWAEXP-004.md, archive/tickets/S102FROAWAEXP-005.md, archive/tickets/S102FROAWAEXP-006.md

## Problem

The S102 feature set (gate bypass, multi-hop BFS, belief protection, counter reset) needs end-to-end golden tests validating the complete agent decision cycle — from unmet need through exploration to resource discovery. Focused unit tests in tickets 001-006 verify individual components; golden tests verify they compose correctly into emergent exploration chains.

## Assumption Reassessment (2026-04-14)

1. Existing golden exploration coverage already lives in `crates/worldwake-ai/tests/golden_exploration.rs` with four S80 scenarios (`golden_exploration_triggers_on_need_and_ignorance`, `golden_exploration_is_suppressed_when_known_satisfaction_path_exists`, `golden_exploration_consecutive_cap_is_respected`, `golden_exploration_arrival_unlocks_beliefs_and_concrete_relief`). This ticket extends that same file instead of creating a new golden family.
2. The live golden harness is `crates/worldwake-ai/tests/golden_harness/mod.rs`, not `TestBeliefView`. It already provides the reusable surfaces needed here: custom topology via `GoldenHarness::with_recipes()`, profile overrides through `set_agent_perception_profile()` / `set_agent_cognitive_profile()` / direct `WorldTxn`, authoritative belief seeding via `seed_belief_from_world()`, and decision/action/perception trace sinks.
3. `select_exploration_target()` in `crates/worldwake-ai/src/candidate_generation.rs` emits one ranked `ExploreLocation` target per motivating need, not a frontier-set payload. Golden proof must therefore assert staged target progression across ticks (for example first `Village`, later `Fields`), not "both frontier places appear as generated candidates at once."
4. The original gate-unlock sketch was internally inconsistent: it said the actor believed `Grain` existed at a `Village` that only contained a `Well`. The honest motivating path is a known remote food source that matches the believed commodity and recipe surface, then budget exhaustion on the remote acquire/produce plan, then S102 exploration bypass despite that known path.
5. A simple three-place chain does not make `frontier_depth: 2` uniquely necessary for eventual discovery, because depth-1 exploration can still chain across rounds once intermediate places become known. The honest golden proof for ticket 005 at this layer is staged multi-hop composition under the live S102 stack, not a false "depth 1 can never reach the second hop" claim.
6. Belief-persistence proof does need a sharper setup than the original ticket claimed. If `frontier_depth: 2`, the agent can infer the second hop from the start place without relying on the intermediate-place belief. The golden persistence scenario must therefore use `frontier_depth: 1` plus an aggressively pruning `PerceptionProfile.entity_activation_threshold` so the arrival boost is causally necessary.
7. Because this ticket adds new `golden_*` scenarios and source-declared `// Scenario ...` blocks, broadened verification must include `python3 scripts/golden_inventory.py --write --check-docs` and review the generated docs fallout in `docs/generated/`.
8. A second reassessment was required during implementation: under the live golden cadence, the zero-boost control still reaches the second hop in the simple 1-hop chain. The honest Test 3 proof at this layer is therefore comparative reinforcement of the intermediate-place belief state plus successful boosted second-hop discovery, not a false stronger contract that zero boost must stall the chain entirely.

## Architecture Check

1. Golden tests exercise the complete agent decision cycle (candidate generation → ranking → plan search → action execution → belief update). This is the appropriate proof surface for S102's emergent behavior: tickets 001-006 proved the local substrates, and this ticket proves the composed frontier behavior end to end.
2. Extending `golden_exploration.rs` is cleaner than creating a second frontier-specific golden file because the existing file already owns exploration-only scenario isolation, helper topology, and scenario-doc inventory entries.
3. No backward-compatibility shims. This is test-only work against the landed S102 behavior.

## Verification Layers

1. Gate unlock after budget exhaustion -> decision trace + action trace + authoritative tracker state: repeated budget-exhausted acquire attempts accumulate hunger exhaustion, then `ExploreLocation` is generated/selected and travel commits
2. Multi-hop staged discovery -> decision trace + action trace + authoritative belief state: exploration first selects the intermediate place, later selects the second-hop place, then perception unlocks concrete relief there
3. Belief persistence via arrival boost -> comparative golden E2E plus authoritative belief state: boosted run retains the intermediate place belief long enough for the second exploration round; unboosted control loses that belief under the sharpened activation profile
4. Counter reset after satisfaction -> authoritative tracker state + post-satisfaction decision trace: hunger relief drops below threshold, the lazy reset clears the tracker on the next generation pass, and exploration does not spuriously re-fire for the satisfied need
5. Inventory/doc handoff -> golden inventory refresh: generated docs in `docs/generated/` reflect the added scenarios and test names

## What to Change

### 1. Golden Test 1: Planner Failure Unlocks Exploration

Setup:
- 2 places: Trail (nothing), Village (remote food source)
- Agent at Trail, hunger=800‰
- Agent already believes the matching remote food source exists at Village (so `need_has_known_acquisition_path()` is true)
- CognitiveProfile with low `max_node_expansions` to force `PlanSearchOutcome::BudgetExhausted` on the remote acquire path
- `acquisition_failure_threshold: 3`

Assert:
- The first repeated planning attempts select the concrete acquire goal and budget-exhaust
- After 3 budget-exhausted attempts, `ExploreLocation` is generated/selected for the Village despite the still-known path
- Travel commits to Village and the authoritative exhaustion tracker reaches the threshold before the bypass fires

### 2. Golden Test 2: Multi-Hop Frontier Discovery

Setup:
- 3 places: Forest → Village → Fields (with FieldPlot/ResourceSource)
- Agent at Forest, hunger=800‰, known_recipes=["Harvest Grain"]
- Agent knows only Forest
- `frontier_depth: 2`
- Need high enough to trigger exploration immediately

Assert:
- First exploration selects Village, then a later exploration round selects Fields
- At Fields, discovers FieldPlot, harvests Grain
- The golden proves staged multi-hop composition under the live S102 stack; it does not claim the false stronger contract that depth-1 can never reach the second hop

### 3. Golden Test 3: Exploration-Chain Belief Persistence

Setup:
- 3 places: Forest → Village → Inn (with WashBasin)
- Agent at Forest, dirtiness=800‰
- `frontier_depth: 1`
- `exploration_arrival_boost: 500` (4 synthetic ticks)
- Aggressive `entity_activation_threshold` so a single stale place observation decays away quickly without the boost
- Parallel unboosted control run with otherwise identical setup

Assert:
- Agent explores Village (1 hop)
- Boosted run records a stronger retained Village place belief than the unboosted control
- Boosted run explores Inn (2nd hop) and discovers the WashBasin there

### 4. Golden Test 4: Counter Reset on Need Satisfaction

Setup:
- Same as Test 1 but extended through concrete food acquisition and eating
- Hunger drops below `need_activation_threshold`

Assert:
- `AcquisitionExhaustionTracker` count for Hunger resets to 0 on the next candidate-generation pass
- Once hunger is satisfied, no new hunger-motivated exploration fires

## Files to Touch

- `crates/worldwake-ai/tests/golden_exploration.rs` (modify — add S102 scenarios, helpers, and source-declared scenario docs)
- `docs/generated/golden-e2e-inventory.md` (generated)
- `docs/generated/golden-scenario-index.md` (generated)
- `docs/generated/golden-scenario-details/golden_exploration.md` (generated)

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
6. Golden inventory/docs refresh stays aligned with the new scenario metadata

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

1. `cargo test -p worldwake-ai --test golden_exploration -- --list`
2. `cargo test -p worldwake-ai --test golden_exploration golden_s102_gate_unlock_after_budget_exhaustion -- --exact`
3. `cargo test -p worldwake-ai --test golden_exploration golden_s102_multi_hop_frontier_discovery -- --exact`
4. `cargo test -p worldwake-ai --test golden_exploration golden_s102_exploration_chain_belief_persistence -- --exact`
5. `cargo test -p worldwake-ai --test golden_exploration golden_s102_counter_reset_on_need_satisfaction -- --exact`
6. `python3 scripts/golden_inventory.py --write --check-docs`
7. `cargo test -p worldwake-ai`
8. `cargo build --workspace`
9. `cargo test --workspace`
10. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-04-14
- What changed: added four S102 golden scenarios to `crates/worldwake-ai/tests/golden_exploration.rs` covering budget-exhaustion unlock, staged multi-hop discovery, arrival-boost belief reinforcement, and lazy exhaustion-counter reset; added local topology/trace helpers in the same file to prove those contracts through decision traces, action traces, authoritative belief state, and tracker state.
- Deviations from original plan: the belief-persistence scenario was narrowed again during implementation when the zero-boost control still completed the second hop under the live golden cadence; the final proof compares strengthened intermediate-place belief state plus boosted-run second-hop discovery instead of asserting total chain failure in the control run. Golden inventory refresh also produced broader generated-doc fallout than the original file list anticipated, including `docs/generated/golden-coverage-matrix.md`, `docs/generated/golden-scenario-details/activation-decay.md`, and `docs/generated/golden-scenario-details/planner-pathology.md`.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_exploration -- --list`
  - `cargo test -p worldwake-ai --test golden_exploration golden_s102_gate_unlock_after_budget_exhaustion -- --exact`
  - `cargo test -p worldwake-ai --test golden_exploration golden_s102_multi_hop_frontier_discovery -- --exact`
  - `cargo test -p worldwake-ai --test golden_exploration golden_s102_exploration_chain_belief_persistence -- --exact`
  - `cargo test -p worldwake-ai --test golden_exploration golden_s102_counter_reset_on_need_satisfaction -- --exact`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo test -p worldwake-ai`
  - `cargo build --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
