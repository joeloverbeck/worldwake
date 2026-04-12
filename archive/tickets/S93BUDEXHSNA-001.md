# S93BUDEXHSNA-001: AcquireCommodity budget exhaustion snapshot golden tests

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — tests only
**Deps**: None

## Problem

S91's golden test for budget exhaustion passes under clean-room conditions (2-3 known entities) but the cli-evaluation simulation produces 96+ budget-exhausted plan attempts with 12-16 known entities. The gap is that no golden test reproduces the exact planner input state (belief density, inventory, place contents) from the real simulation. This ticket creates 4 golden tests that capture the exact `AcquireCommodity` budget-exhaustion snapshots from the cli-evaluation run (seed 7777), proving the failure exists under realistic entity densities.

## Assumption Reassessment (2026-04-11)

1. **`GoldenHarness`** exists at `crates/worldwake-ai/tests/golden_harness/mod.rs:1089`. `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` now exists from S93BUDEXHSNA-002 and already carries the local cli-evaluation topology builder plus TreatWounds snapshot helpers. This ticket now owns extending that existing file with the four AcquireCommodity scenarios rather than creating the file from scratch.
2. **`PlanSearchResult::BudgetExhausted`** exists at `crates/worldwake-ai/src/search/mod.rs:195`. `PlanSearchOutcome::BudgetExhausted` exists at `crates/worldwake-ai/src/decision_trace.rs:895`.
3. This is a single-layer ticket (AI planner tests only). The shared abstraction boundary is `search_plan` — the function under test. No authoritative/system layer is involved.
4. N/A — no failing golden motivates this ticket; the simulation observer report motivates it.
5. Live `GoalKind` under test: `AcquireCommodity { commodity, purpose: SelfConsume }`. The planner routes this through `search_plan` with operators including `pick_up`, `eat`/`drink`, `trade`, `queue_for_facility_use`, `harvest`, `craft`, `move_cargo`. The snapshot data shows 1483-2657 candidates generated from these operators × known entities.
6. The observer-backed Thornwall water snapshots at ticks 11 and 25 were not faithfully reproducible from a hand-seeded static harness alone. The landed tests therefore reconstruct those two cases from the exact `cli-evaluation.ron` start state, advance the simulation to the observer tick under seed `7777`, and then call `search_plan` on the live belief surface at that tick. The Dusty Trail Apple and late-game Kael water cases remain file-local snapshot reconstructions.

## Architecture Check

1. Reusing the existing file-local cli-evaluation topology scaffolding in `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` keeps this ticket bounded to the remaining AcquireCommodity scenarios. Extracting that scaffolding into `golden_harness` would still be a separate refactoring concern.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. Budget exhaustion reproduction → `search_plan` returns `BudgetExhausted` with matching `expansions_used` (decision trace / planner output)
2. Phase 2 fix verification → `search_plan` returns `Found`, action chain executes, need decreases (decision trace + authoritative world state)
3. Single-layer ticket — additional layer mapping not applicable. All assertions operate on planner output (`PlanSearchResult`).

## What to Change

### 1. Extend the existing snapshot test file with AcquireCommodity coverage

Modify `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` to add the four AcquireCommodity scenarios and any additional file-local setup helpers they need. Reuse the existing cli-evaluation topology builder and current helper scaffolding where lawful instead of recreating the file from scratch.

### 2. Test 1: `merchant_vara_water_at_thornwall_budgets_exhaust`

Snapshot tick 11. Merchant Vara at Thornwall Village, goal `AcquireCommodity(Water, SelfConsume)`.

Setup per spec:
- Needs: hunger=224, thirst=236, fatigue=124, bladder=148, dirtiness=112
- Inventory: empty
- Cognitive profile: max_node_expansions=300, max_plan_depth=10, max_candidates_per_expansion=150
- Execution budget: beam_width=10, preferred_operator_boost=3, max_prerequisite_locations=3
- 12 known entities at Thornwall Village (3 agents, 6 items, 3 workstations)
- Place contents: 3 agents, Mill, Loom, Well, 1×Bow, 4×Bread, 20×Coin, 10×Grain, 1×Sword, 4×Water

Phase 1 assertion: `search_plan` returns `BudgetExhausted`.
Phase 2 (`#[ignore]`): `search_plan` returns `Found`, thirst decreases after execution.

### 3. Test 2: `guard_theron_water_at_thornwall_budgets_exhaust`

Snapshot tick 25. Guard Theron at Thornwall Village, goal `AcquireCommodity(Water, SelfConsume)`.

Setup per spec:
- Needs: hunger=352, thirst=378, fatigue=152, bladder=204, dirtiness=126
- Inventory: 1×Bow, 1×Sword
- Cognitive profile: default (max_node_expansions=224, max_plan_depth=8, max_candidates_per_expansion=200)
- Execution budget: default (beam_width=8, preferred_operator_boost=2, max_prerequisite_locations=3)
- 14 known entities: 12 at Thornwall Village, Dusty Trail known (no contents)
- Place contents: Merchant Vara, Guard Theron, Mill, Loom, Well, 1×Bow, 10×Grain, 1×Sword

Phase 1 assertion: `search_plan` returns `BudgetExhausted`.
Phase 2 (`#[ignore]`): `search_plan` returns `Found`, thirst decreases.

### 4. Test 3: `merchant_vara_apple_at_dusty_trail_budgets_exhaust`

Snapshot tick 85. Merchant Vara at Dusty Trail, goal `AcquireCommodity(Apple, SelfConsume)`.

Setup per spec:
- Needs: hunger=192, thirst=458, fatigue=272, bladder=76, dirtiness=186
- Inventory: 9×Grain
- Cognitive profile: Merchant Vara's custom (max_node_expansions=300, speculative_acquisition=true)
- 12 known entities: Dusty Trail + Thornwall Village (places), Kael, self, Guard Theron, 3×Bread, 3×Waste, Mill, Loom, Well
- Place contents at Dusty Trail: Kael, Merchant Vara, 3×Bread, 20×Coin, 9×Grain, 4×Waste, 3×Water

Phase 1 assertion: `search_plan` returns `BudgetExhausted`.
Phase 2 (`#[ignore]`): `search_plan` returns `Found`, hunger decreases.

### 5. Test 4: `kael_water_at_thornwall_late_game_budgets_exhaust`

Snapshot tick 411. Kael at Thornwall Village, goal `AcquireCommodity(Water, SelfConsume)`.

Setup per spec:
- Needs: hunger=42, thirst=411, fatigue=284, bladder=156, dirtiness=512
- Inventory: 20×Coin
- Cognitive profile: default
- 16 known entities: 7×Waste + 3 agents at Dusty Trail, 20×Coin + Mill + Loom + Well at Thornwall Village
- Place contents at Thornwall Village: Kael, Mill, Loom, Well, 20×Coin (NO Water items — Well is resource source)

Phase 1 assertion: `search_plan` returns `BudgetExhausted`.
Phase 2 (`#[ignore]`): `search_plan` returns `Found`, thirst decreases.

## Files to Touch

- `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` (modify)

## Out of Scope

- Planner changes — this ticket captures failures, not fixes
- Extracting topology helpers to `golden_harness` (separate refactoring)
- TreatWounds tests (ticket 002)
- Phase 2 fix verification (un-ignoring `#[ignore]` tests — done when planner fix lands)

## Acceptance Criteria

### Tests That Must Pass

1. `merchant_vara_water_at_thornwall_budgets_exhaust` — asserts `BudgetExhausted` with ≥300 expansions
2. `guard_theron_water_at_thornwall_budgets_exhaust` — asserts `BudgetExhausted` with ≥224 expansions
3. `merchant_vara_apple_at_dusty_trail_budgets_exhaust` — asserts `BudgetExhausted` with ≥300 expansions
4. `kael_water_at_thornwall_late_game_budgets_exhaust` — asserts `BudgetExhausted` with ≥224 expansions
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Each test reproduces the exact budget-exhaustion signature from the simulation snapshot (goal, location, cognitive profile match)
2. Phase 2 `#[ignore]` tests compile but do not run by default

## Outcome

**Completion date**: 2026-04-11

Added the four AcquireCommodity reproductions to `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` plus four ignored `Found` follow-ups. The two early Thornwall water cases now rebuild the live `cli-evaluation.ron` scenario substrate and step to ticks 11 and 25 before invoking `search_plan`, which was necessary to reproduce the observer-reported budget exhaustion under the current branch. The Dusty Trail Apple and late-game Kael water cases use focused file-local snapshot setup on the shared S93 harness.

**Deviations from original plan**:
- The original ticket narrative assumed all four cases could be reproduced from static hand-seeded snapshots. The landed implementation uses mixed reconstruction: two observer-backed Thornwall water cases replay the live scenario to the recorded tick, while the other two cases remain distilled file-local snapshots.

## Verification Result

1. `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` — 4 new golden tests capturing AcquireCommodity budget exhaustion from cli-evaluation simulation snapshots

### Commands

1. `cargo test -p worldwake-ai --test golden_budget_exhaustion_snapshots -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
