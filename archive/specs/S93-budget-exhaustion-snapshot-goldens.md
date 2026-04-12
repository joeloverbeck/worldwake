# S93 — Budget Exhaustion Snapshot Golden Tests

**Status**: COMPLETED

## Problem Statement

Despite five specs (S83, S88, S89, S90, S91/S92) addressing GOAP planner budget exhaustion through belief-informed pruning, two-phase strategic/tactical decomposition, mandatory tactical scoping, and prerequisite guidance, the cli-evaluation.ron simulation (seed 7777, 1440 ticks) still produces 96+ budget-exhausted plan attempts across 3 of 4 agents. Guard Theron dies of hunger at tick 422 because `AcquireCommodity(Water)` generates 2085 candidates at depth 5 and exceeds the 224-node expansion budget — at a location where Water is physically present.

S91's golden test `cross_location_water_acquisition_succeeds_without_budget_exhaustion` passes because it seeds clean beliefs with only the relevant entities (well and village). The real simulation has 12–16 known entities (including Waste, weapons, coins, other agents, and facilities) that multiply candidate branches beyond what S91 exercises.

This spec captures the **exact planner input state** at each distinct budget-exhaustion moment from the cli-evaluation simulation and turns each into a golden e2e test. Where a hand-seeded static fixture is sufficient, the test distills the snapshot directly; where early-run accumulated scenario history materially affects the planner surface, the test replays `cli-evaluation.ron` to the recorded tick and asserts on that live tick state. These tests serve as falsification criteria: the planner must handle realistic entity densities, not just clean-room isolation.

**Evidence source**: `reports/simulation-observer-report.md` (observer run with seed 7777, 1440 ticks). Budget exhaustion snapshots captured via Section 8 of the enhanced observer dump.

## Scope

- 6 golden e2e tests in `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs`
- No planner changes — this spec captures and proves the failures
- Planner fix is a separate follow-up spec

## Golden Test Specifications

Each test follows the S91 dual-phase assertion pattern:
1. **Phase 1 (bug reproduction)**: Assert `search_plan` returns `BudgetExhausted` with the exact snapshot state (reproduces the bug under current code)
2. **Phase 2 (fix verification)**: After the planner fix lands, flip to assert `Found` and verify the action chain executes (marked `#[ignore]` until fix lands, then un-ignored)

All tests use the cli-evaluation topology: 5 places (Thornwall Village, Eldergrove Forest, Dusty Trail, Hearthstone Inn, Golden Fields) with 6 edges matching `scenarios/cli-evaluation.ron`. Tests 1 and 2 replay the live scenario from seed `7777` to ticks 11 and 25 before invoking `search_plan`; Tests 3-6 use focused file-local reconstruction on the shared snapshot harness.

### Test 1: `merchant_vara_water_at_thornwall_budgets_exhaust`

**Snapshot tick**: 11
**Agent**: Merchant Vara at Thornwall Village
**Goal**: `AcquireCommodity { commodity: Water, purpose: SelfConsume }`

**Setup**:
- Needs: hunger=224, thirst=236, fatigue=124, bladder=148, dirtiness=112
- Inventory: empty
- Cognitive profile: max_node_expansions=300, max_plan_depth=10, max_candidates_per_expansion=150, beam_width=10, preferred_operator_boost=3
- Known entities (12): Thornwall Village, Kael, self, Guard Theron, 20×Coin, 4×Water, 4×Bread, 1×Sword, 1×Bow, Mill, Loom, Well
- Believed locations: all 12 entities at Thornwall Village
- Place contents at Thornwall Village: 3 agents, Mill, Loom, Well (workstations), 1×Bow, 4×Bread, 20×Coin, 10×Grain, 1×Sword, 4×Water
- Adjacent: Eldergrove (Forager Lina, ChoppingBlock, OrchardRow, 3×Apple, 5×Water), Golden Fields (FieldPlot, GravePlot), Hearthstone Inn (Forge, WashBasin, 3×Firewood, 2×Medicine)

**Observed failure**: 300 expansions, depth 9, 1483 total candidates

**Key diagnostic question**: Water is physically present at the agent's location. Why does `AcquireCommodity(Water)` generate 1483 candidates and budget-exhaust instead of finding a 1-step pick-up-and-consume plan?

### Test 2: `guard_theron_water_at_thornwall_budgets_exhaust`

**Snapshot tick**: 25
**Agent**: Guard Theron at Thornwall Village
**Goal**: `AcquireCommodity { commodity: Water, purpose: SelfConsume }`

**Setup**:
- Needs: hunger=352, thirst=378, fatigue=152, bladder=204, dirtiness=126
- Inventory: 1×Bow, 1×Sword
- Cognitive profile: max_node_expansions=224, max_plan_depth=8, max_candidates_per_expansion=200, beam_width=8, preferred_operator_boost=2
- Known entities (14): Thornwall Village, Dusty Trail, Kael, self, Merchant Vara, 20×Coin, 4×Water, 10×Grain, 3×Bread, 1×Sword, 1×Bow, Mill, Loom, Well
- Believed locations: 12 entities at Thornwall Village, Dusty Trail known but no believed contents
- Place contents at Thornwall Village: Merchant Vara, Guard Theron, Mill, Loom, Well, 1×Bow, 10×Grain, 1×Sword
- Note: Kael has already left for Dusty Trail, so Thornwall has fewer entities than tick 11
- Adjacent: Dusty Trail (Kael, 3×Bread, 20×Coin, 1×Waste, 4×Water), Eldergrove (Forager Lina, ChoppingBlock, OrchardRow, 2×Apple, 1×Waste, 5×Water), Golden Fields, Hearthstone Inn

**Observed failure**: 224 expansions, depth 5, 2085 total candidates

**Key diagnostic question**: Same as Test 1 — Water present locally, but 2085 candidates generated. Different cognitive profile (default CognitiveProfile) vs. Merchant Vara's custom one. Larger candidate explosion despite lower max_candidates_per_expansion, suggesting the issue is in the number of action operators × target entities, not per-expansion limits.

### Test 3: `merchant_vara_apple_at_dusty_trail_budgets_exhaust`

**Snapshot tick**: 85
**Agent**: Merchant Vara at Dusty Trail
**Goal**: `AcquireCommodity { commodity: Apple, purpose: SelfConsume }`

**Setup**:
- Needs: hunger=192, thirst=458, fatigue=272, bladder=76, dirtiness=186
- Inventory: 9×Grain
- Cognitive profile: same as Test 1 (Merchant Vara's custom profile)
- Known entities (12): Thornwall Village (place, no contents known), Dusty Trail (place), Kael, self, Guard Theron, 3×Bread, 1×Waste×3, Mill, Loom, Well
- Place contents at Dusty Trail: Kael, Merchant Vara, 3×Bread, 20×Coin, 9×Grain, 4×Waste, 3×Water
- Adjacent: Thornwall Village (Guard Theron, Mill, Loom, Well, 1×Bow, 1×Sword)

**Observed failure**: 300 expansions, depth 4, 2511 total candidates

**Key diagnostic question**: Apples are at Eldergrove Forest (not adjacent to Dusty Trail — only reachable via Thornwall Village). The plan requires travel through Thornwall to Eldergrove. But the agent doesn't know about Eldergrove or its apples — their beliefs show only Thornwall Village and Dusty Trail as known places. How is `AcquireCommodity(Apple)` even generating 2511 candidates if the agent doesn't know where apples are? Is speculative acquisition active for this agent? (Merchant Vara has `speculative_acquisition: true` in the scenario.)

### Test 4: `kael_water_at_thornwall_late_game_budgets_exhaust`

**Snapshot tick**: 411
**Agent**: Kael at Thornwall Village
**Goal**: `AcquireCommodity { commodity: Water, purpose: SelfConsume }`

**Setup**:
- Needs: hunger=42, thirst=411, fatigue=284, bladder=156, dirtiness=512
- Inventory: 20×Coin
- Cognitive profile: default (max_node_expansions=224, max_plan_depth=8, max_candidates_per_expansion=200, beam_width=8, preferred_operator_boost=2)
- Known entities (16): Thornwall Village, Dusty Trail, self, Merchant Vara, Guard Theron, 7×Waste, 20×Coin, Mill, Loom, Well
- Believed locations: 7 Waste + 3 agents at Dusty Trail; 20×Coin + Mill + Loom + Well at Thornwall Village
- Place contents at Thornwall Village: Kael, Mill, Loom, Well, 20×Coin
- Adjacent: Dusty Trail (Merchant Vara, Guard Theron, 1×Bow, 5×Grain, 1×Sword, 14×Waste), Eldergrove, Golden Fields, Hearthstone Inn

**Observed failure**: 224 expansions, depth 6, 2657 total candidates

**Key diagnostic question**: Thornwall Village has a Well (workstation) but NO Water items on the ground. The Well is a resource source that regenerates Water — but Water acquisition through a Well requires `queue_for_facility_use`, not `pick_up`. Does the planner's `AcquireCommodity` goal correctly handle Well-based water acquisition? The agent knows the Well exists (in beliefs). Also notable: 7 Waste entities in beliefs inflate the candidate set.

### Test 5: `merchant_vara_treat_wounds_at_dusty_trail_budgets_exhaust`

**Snapshot tick**: 456
**Agent**: Merchant Vara at Dusty Trail
**Goal**: `TreatWounds { patient: self }`

**Setup**:
- Needs: hunger=214, thirst=1000, fatigue=294, bladder=120, dirtiness=557
- Inventory: 5×Grain
- Cognitive profile: Merchant Vara's custom profile
- Known entities (12): Dusty Trail, self, Kael, 9×Waste
- Believed locations: all at Dusty Trail (Kael + self + 9×Waste)
- Place contents at Dusty Trail: Merchant Vara, Guard Theron, 1×Bow, 5×Grain, 1×Sword, 15×Waste
- Adjacent: Thornwall Village (Kael, Mill, Loom, Well, 20×Coin)

**Observed failure**: 300 expansions, depth 3, 5739 total candidates

**Key diagnostic question**: TreatWounds requires Medicine (at Hearthstone Inn, 2 travel hops away). The agent doesn't even know about Hearthstone Inn or Medicine — beliefs show only Dusty Trail and its contents. With 12 known entities (9 of which are Waste), 5739 candidates at depth 3 implies massive branching per expansion. This is the worst candidate explosion: 5739/300 ≈ 19 candidates per expansion on average, but the root expansion likely generates far more (the action operator × target entity product).

### Test 6: `kael_treat_wounds_vara_at_dusty_trail_budgets_exhaust`

**Snapshot tick**: 471
**Agent**: Kael at Dusty Trail
**Goal**: `TreatWounds { patient: Merchant Vara }`

**Setup**:
- Needs: hunger=162, thirst=591, fatigue=304, bladder=8, dirtiness=572
- Inventory: 20×Coin
- Cognitive profile: default
- Known entities (16): Thornwall Village, Dusty Trail, self, Merchant Vara, 9×Waste, Mill, Loom, Well
- Believed locations: self + Merchant Vara + 9×Waste at Dusty Trail; Mill + Loom + Well at Thornwall Village
- Place contents at Dusty Trail: Kael, Merchant Vara, Guard Theron, 1×Bow, 20×Coin, 5×Grain, 1×Sword, 16×Waste
- Adjacent: Thornwall Village (Mill, Loom, Well)

**Observed failure**: 224 expansions, depth 3, 4151 total candidates

**Key diagnostic question**: Same pattern as Test 5 but with Kael's default cognitive profile (fewer max_node_expansions). 16×Waste at the place further inflates the candidate set. The patient (Merchant Vara) is co-located, but Medicine is at Hearthstone Inn (2 hops). The plan chain is: travel to Thornwall → travel to Hearthstone → pick_up Medicine → travel back → treat. This 5-step chain with branching at each step creates the explosion.

## Cross-Test Diagnostic Patterns

### Pattern A: Local Resource Budget Exhaustion (Tests 1, 2, 4)

In all three tests, `AcquireCommodity(Water)` budget-exhausts at a location that **has or can produce Water**. Tests 1 and 2 have Water items physically on the ground. Test 4 has a Well (resource source) but no Water items. The two-phase planning from S88/S89 should decompose this into a tactical-only plan (no strategic travel needed), but the candidate count (1483–2657) suggests the tactical search is still exploring too many operator × target combinations.

**Hypothesis**: The planner's `AcquireCommodity` is not correctly identifying that Water is locally available and restricting the search to local operators (pick_up + consume). Instead, it's expanding all acquisition operators (Trade, QueueForFacilityUse, Harvest, Craft, MoveCargo, PickUp) × all known entities, creating a combinatorial explosion.

### Pattern B: Remote Resource Budget Exhaustion (Test 3)

`AcquireCommodity(Apple)` at Dusty Trail when apples are at Eldergrove Forest (not directly adjacent). The agent has `speculative_acquisition: true`, allowing it to consider places without current positive resource evidence. This generates candidates for every known place × every acquisition operator.

### Pattern C: Multi-Hop Acquisition Budget Exhaustion (Tests 5, 6)

`TreatWounds` requires Medicine at a location 2 hops away. The plan chain is at least 5 steps deep. With 12–16 known entities (many Waste), each expansion generates hundreds of candidates. The two-phase planning should handle this via strategic decomposition (plan travel to Hearthstone Inn, then tactical acquisition of Medicine), but either the decomposition isn't firing or the tactical search is still too broad.

### Pattern D: Waste Entity Inflation

Across all snapshots, agents accumulate 7–16 Waste entities in their beliefs. Each Waste entity adds target candidates for actions like `pick_up`, `drop_item`, `store_stock`, `stage_stock_for_sale`, etc. The candidate count growth between tick 11 (1483 candidates, 3 Waste believed) and tick 471 (4151 candidates, 9 Waste believed) correlates with Waste accumulation.

## Section H — Causal Hooks (FND-01)

### H.1 Information-Path Analysis

No new information paths introduced. This spec adds tests, not systems.

### H.2 Positive-Feedback Analysis

No positive feedback loops introduced. However, the Waste accumulation pattern (Pattern D) reveals an existing positive feedback loop: consumption → Waste creation → more belief entities → larger candidate sets → budget exhaustion → inability to consume → more Waste accumulation. The physical dampener is carry capacity (agents eventually can't hold more Waste), but the planner impact occurs well before that physical limit.

### H.3 Stored State vs. Derived Read-Model

No new stored state introduced. All test assertions read existing planner output (`PlanSearchResult`).

## Deliverables

1. `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` — 6 golden tests
2. Each test builds `GoldenHarness` with cli-evaluation topology and proves the recorded planner state either by replaying the live scenario to the observer tick or by focused file-local snapshot reconstruction, then asserts `BudgetExhausted` (Phase 1)
3. Phase 2 assertions (`#[ignore]` until fix) flip to `Found` and verify need reduction

## Verification

```bash
# Phase 1: All 6 tests reproduce the budget exhaustion
cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots -- --nocapture

# Phase 2 (after planner fix): Un-ignore and verify
cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots -- --ignored --nocapture
```

## Outcome

**Completion date**: 2026-04-11

Implemented the dedicated S93 golden snapshot file at `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs`, covering all six observer-backed budget-exhaustion scenarios: four `AcquireCommodity` reproductions and two `TreatWounds` reproductions. The file includes ignored phase-2 `Found` follow-ups for each scenario so the planner-fix proof can be activated in place later.

**Deviations from original plan**:
- The early Thornwall water failures were not faithfully reproducible from static hand-seeded fixtures alone on the current branch. The landed tests therefore use mixed reconstruction: the tick-11 and tick-25 water cases replay `cli-evaluation.ron` from seed `7777` to the recorded observer tick before calling `search_plan`, while the remaining four scenarios use focused file-local snapshot reconstruction.

**Verification results**:
- `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots -- --nocapture`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
