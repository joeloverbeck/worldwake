# S18PREAWAEME-002: Golden test — merchant restocks via prerequisite-aware craft

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only
**Deps**: None active — prerequisite `RestockCommodity` recipe-input support is already implemented and covered in focused tests

## Problem

No golden test exercises the emergent chain: enterprise demand signal → `RestockCommodity` goal → prerequisite-aware plan search → travel to remote recipe input source → acquire input → return to workstation → craft → merchant inventory restocked. The existing `golden_supply_chain.rs` tests cover harvest-based restock, consumer trade, and the currently-active full-chain trade scenario, but none isolate craft-based restock as its own golden. This chain validates Principles 1 (emergent local causality), 5 (concrete carriers), 7 (belief-seeded locality), and 24 (state-mediated system interaction).

## Assumption Reassessment (2026-03-21)

1. `crates/worldwake-ai/tests/golden_supply_chain.rs` currently exposes six scenario tests in the `golden_supply_chain` binary: merchant restock, consumer trade, full supply chain, and a replay companion for each. None covers craft-based restock. Verified with `cargo test -p worldwake-ai --test golden_supply_chain -- --list`.
2. The prerequisite architecture this ticket originally depended on is already live:
   - `GoalKind::RestockCommodity` delegates to recipe-input prerequisite discovery in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs).
   - Focused unit coverage already exists in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) for `prerequisite_places_restock_commodity_include_missing_recipe_input_places` and the matching empty-case tests.
   - Search-trace plumbing for `SearchExpansionSummary.prerequisite_places_count` already exists in [`crates/worldwake-ai/src/search.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs).
3. The golden harness in [`crates/worldwake-ai/tests/golden_harness/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) already provides the needed setup pieces: `build_bake_bread_recipe()`, `build_multi_recipe_registry()`, `place_workstation()`, `place_workstation_with_source()`, `seed_agent_with_recipes()`, `seed_actor_beliefs()`, `set_agent_perception_profile()`, and action-trace helpers on `GoldenHarness`.
4. The harness `Bake Bread` recipe is currently `Firewood(1) -> Bread(1)` at `WorkstationTag::Mill` with `work_ticks: 3`. That is the authoritative contract for this golden and must be cited explicitly rather than inferred from other registries.
5. `DemandObservationReason::WantedToBuyButSellerOutOfStock` is still the concrete enterprise trigger used by the existing merchant-restock scenario in [`crates/worldwake-ai/tests/golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs).
6. `KnownRecipes` must include the bread recipe id for the merchant, and the merchant needs belief/perception setup for the remote firewood source and local mill. This is a belief-planning golden, so locality must be proven from seeded beliefs plus ordinary perception rather than omniscient world reads.
7. Scenario isolation should remain narrow: one AI merchant, no competing sellers or alternate firewood sources, no local firewood at the home market, and low competing survival pressure. The contract under test is route reachability and execution of the craft-restock chain, not multi-agent contention.

## Architecture Check

1. The current architecture is already the right one: `RestockCommodity` stays a goal-level enterprise intent, recipe-input discovery stays inside shared planner prerequisite logic, and the golden should prove that those existing layers compose. Reopening production/search architecture here would duplicate delivered behavior instead of improving it.
2. The beneficial change is therefore narrower than the original ticket implied: add the missing golden that locks the existing architecture in place. That improves long-term robustness without adding aliases, special cases, or one-off test helpers in live authority paths.
3. The new golden should follow the repo’s assertion hierarchy from `docs/golden-e2e-testing.md`: durable outcome in authoritative world state, action lifecycle in action traces, AI reason/route evidence in decision traces, and conservation checks each tick.

## Verification Layers

1. Merchant holds Bread at the home market after the scenario → authoritative world state
2. Remote firewood availability decreases from its initial authoritative quantity → authoritative world state
3. Travel to the remote prerequisite place is part of the chosen plan and later commits → decision trace for selected path, action trace for execution
4. Firewood acquisition at the remote place commits as `harvest` or `pickup` depending on the live affordance path → action trace
5. Return travel toward the mill/home market commits → action trace
6. `craft:Bake Bread` commits at the mill → action trace
7. `RestockCommodity { Bread }` is generated and selected early in the scenario → decision trace
8. At least one recorded search expansion reports `prerequisite_places_count > 0` for the merchant’s restock search → decision trace
9. `verify_live_lot_conservation()` and `verify_authoritative_conservation()` hold across the commodities under test during the scenario → authoritative conservation helpers
10. Same seed yields identical `(hash_world, hash_event_log)` on replay → deterministic replay companion

## What to Change

### 1. Add `run_merchant_restocks_via_prerequisite_aware_craft` scenario runner

In `crates/worldwake-ai/tests/golden_supply_chain.rs`, add a new test runner function following the pattern of `run_merchant_restock_with_traces`. Setup:

- Merchant at General Store with `MerchandiseProfile` advertising `Bread`
- `DemandMemory` with `WantedToBuyButSellerOutOfStock` for Bread → triggers `RestockCommodity{Bread}`
- High `enterprise_weight` in `UtilityProfile` (e.g., `pm(900)`)
- Sated merchant (default `HomeostaticNeeds` to suppress survival pressure)
- Mill workstation at Village Square or General Store with `WorkstationTag::Mill`; the ticket contract is "remote input -> local craft at merchant home market", not a specific place alias
- Custom recipe registry is acceptable when the live shared golden registry does not already expose the exact harvest affordance needed for the scenario
- Firewood available ONLY at Orchard Farm via `ResourceSource` — no firewood at Village Square or General Store
- Merchant has `PerceptionProfile` and seeded beliefs about Orchard Farm resource source and Mill workstation
- Merchant has `KnownRecipes` covering every live action the scenario requires, not just the terminal craft recipe
- Decision tracing and action tracing enabled for the scenario runner

Assertions follow the hierarchy in Verification Layers above.

### 2. Add `golden_merchant_restocks_via_prerequisite_aware_craft` test

```rust
#[test]
fn golden_merchant_restocks_via_prerequisite_aware_craft() {
    let (wh, eh) = run_merchant_restocks_via_prerequisite_aware_craft(Seed(/* chosen seed */));
    // hashes are recorded after first green run
}
```

### 3. Add deterministic replay companion

```rust
#[test]
fn golden_merchant_restocks_via_prerequisite_aware_craft_replays_deterministically() {
    let seed = Seed(/* same seed */);
    let (wh1, eh1) = run_merchant_restocks_via_prerequisite_aware_craft(seed);
    let (wh2, eh2) = run_merchant_restocks_via_prerequisite_aware_craft(seed);
    assert_eq!(wh1, wh2, "world hash diverged");
    assert_eq!(eh1, eh2, "event log hash diverged");
}
```

## Files to Touch

- [`crates/worldwake-ai/tests/golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs) (modify — add scenario runner + 2 tests)

## Out of Scope

- Changes to `goal_model.rs` or `search.rs` prerequisite logic — already delivered and covered by focused tests
- Changes to `golden_harness/mod.rs` unless the new golden exposes a genuinely missing reusable helper
- Consumer trade tests
- Stale belief golden test (covered by S18PREAWAEME-003)
- Changes to any crate source code in `src/` — this remains a golden-only ticket unless the live code contradicts the ticket during implementation
- Action trace infrastructure changes
- Decision trace infrastructure changes

## Acceptance Criteria

### Tests That Must Pass

1. `golden_merchant_restocks_via_prerequisite_aware_craft` — bread stock exists at the merchant home market after the remote prerequisite chain completes
2. `golden_merchant_restocks_via_prerequisite_aware_craft_replays_deterministically` — same seed produces identical hashes
3. Existing suite: `cargo test -p worldwake-ai --test golden_supply_chain`

### Invariants

1. Conservation: `verify_live_lot_conservation()` holds every tick during the test
2. Conservation: `verify_authoritative_conservation()` holds every tick during the test
3. Existing `golden_supply_chain` tests remain green — no setup pollution
4. Merchant plans from beliefs only, never from authoritative world state (Principle 7 / repo belief-only planning rule)
5. All information reaches merchant through seeded beliefs or perception (Principle 7)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_supply_chain.rs::golden_merchant_restocks_via_prerequisite_aware_craft` — proves enterprise restock selects a remote-input craft path and completes it
2. `crates/worldwake-ai/tests/golden_supply_chain.rs::golden_merchant_restocks_via_prerequisite_aware_craft_replays_deterministically` — proves deterministic replay for the same seed

### Rationale Per Test

1. `golden_merchant_restocks_via_prerequisite_aware_craft` — closes the missing golden coverage gap between the focused prerequisite logic already covered in `goal_model.rs` and the existing supply-chain goldens that only prove harvest-based restock.
2. `golden_merchant_restocks_via_prerequisite_aware_craft_replays_deterministically` — locks the scenario against nondeterministic planner or scheduler regressions, which is especially important for a multi-hop, mixed-layer golden.

### Commands

1. `cargo test -p worldwake-ai --test golden_supply_chain -- --list`
2. `cargo test -p worldwake-ai --test golden_supply_chain golden_merchant_restocks_via_prerequisite_aware_craft -- --exact`
3. `cargo test -p worldwake-ai --test golden_supply_chain golden_merchant_restocks_via_prerequisite_aware_craft_replays_deterministically -- --exact`
4. `cargo test -p worldwake-ai --test golden_supply_chain`
5. `cargo test -p worldwake-ai`

## Outcome

- Completion date: 2026-03-21
- Actual changes:
  - Added `golden_merchant_restocks_via_prerequisite_aware_craft` in `crates/worldwake-ai/tests/golden_supply_chain.rs`
  - Added `golden_merchant_restocks_via_prerequisite_aware_craft_replays_deterministically`
  - Added a tiny test-local recipe registry for `Harvest Firewood` + `Bake Bread` so the golden exercises a real remote `ResourceSource` path under the current action registry
- Deviations from original plan:
  - No production-source code changes were needed because `RestockCommodity` prerequisite discovery had already shipped in `goal_model.rs` with focused unit coverage
  - The durable success boundary is home-market bread stock, not "bread remains in merchant inventory"; the live architecture lawfully materializes the restocked bread as local market stock after the craft chain
  - The initial tick-0 selected plan was asserted at the honest earlier boundary: remote travel guidance via prerequisite places. The later harvest/craft chain is proven through action traces and authoritative end state instead of overstating what tick-0 selection guarantees
- Verification results:
  - `cargo test -p worldwake-ai --test golden_supply_chain golden_merchant_restocks_via_prerequisite_aware_craft -- --exact`
  - `cargo test -p worldwake-ai --test golden_supply_chain golden_merchant_restocks_via_prerequisite_aware_craft_replays_deterministically -- --exact`
  - `cargo test -p worldwake-ai --test golden_supply_chain`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
