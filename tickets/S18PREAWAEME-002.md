# S18PREAWAEME-002: Golden test — merchant restocks via prerequisite-aware craft

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only
**Deps**: S18PREAWAEME-001 (prerequisite_places extension for RestockCommodity)

## Problem

No golden test exercises the emergent chain: enterprise demand signal → `RestockCommodity` goal → prerequisite-aware plan search → travel to remote resource → harvest/pick-up → return to workstation → craft → merchant inventory restocked. The existing `golden_supply_chain.rs` tests cover only harvest-based restock (travel→harvest→return). This chain validates Principles 1 (emergent local causality), 5 (concrete carriers), and 24 (state-mediated system interaction).

## Assumption Reassessment (2026-03-21)

1. `golden_supply_chain.rs` currently has 4 active tests: `run_merchant_restock_with_traces` (harvest-based), `run_consumer_trade_with_traces`, and their replay companions. Two ignored full-chain tests exist (blocked on S10). None test craft-based restock. Confirmed by reading the file.
2. The golden harness (`golden_harness/mod.rs`) provides: `build_bake_bread_recipe()` (Firewood→Bread at Mill), `build_multi_recipe_registry()` (includes Bake Bread), `place_workstation_with_source()`, `seed_agent_with_recipes()`, `seed_actor_beliefs()`, `seed_actor_local_beliefs()`. All needed infrastructure exists.
3. The `Bake Bread` recipe in the harness uses Firewood(1)→Bread(1) at `WorkstationTag::Mill` with `work_ticks: 3`. This differs from the default `simulation_state.rs` version (Grain→Bread). The golden harness version is authoritative for tests.
4. `CommodityKind::Bread` and `CommodityKind::Firewood` both exist in `worldwake-core/src/items.rs`.
5. `DemandObservationReason::WantedToBuyButSellerOutOfStock` is the trigger for enterprise restock — used in existing merchant restock tests (lines 164–171 of `golden_supply_chain.rs`).
6. `PerceptionProfile` must be set on the merchant for belief observation. `KnownRecipes` must include the Bake Bread recipe for the planner to consider craft actions.
7. This is a golden E2E test; the intended layer is golden coverage with decision trace + action trace + conservation assertions.
9. Isolation: only one agent (merchant) is AI-controlled. No competing affordance contention. Metabolism suppressed via default `HomeostaticNeeds` to ensure enterprise goal dominates.

## Architecture Check

1. Follows the established golden test pattern from existing `run_merchant_restock_with_traces`: same harness, same assertion hierarchy (world state → action traces → decision traces → conservation).
2. No backward-compatibility shims. New test file content only.

## Verification Layers

1. Merchant owns ≥1 Bread at General Store → authoritative world state assertion
2. Firewood quantity at Orchard Farm decreased → authoritative world state assertion
3. Travel to Orchard Farm started/committed → action trace
4. Harvest or PickUp committed at Orchard Farm → action trace
5. Travel back to Village Square committed → action trace
6. Craft committed (craft:Bake Bread) → action trace
7. `RestockCommodity{Bread}` selected in first 20 ticks → decision trace
8. `prerequisite_places` non-empty during plan search → decision trace (if `SearchExpansionSummary` exposes this)
9. `verify_live_lot_conservation()` and `verify_authoritative_conservation()` per tick → conservation invariant
10. Deterministic replay: same seed → identical `(hash_world, hash_event_log)` → replay companion test

## What to Change

### 1. Add `run_merchant_restocks_via_prerequisite_aware_craft` function

In `crates/worldwake-ai/tests/golden_supply_chain.rs`, add a new test runner function following the pattern of `run_merchant_restock_with_traces`. Setup:

- Merchant at General Store with `MerchandiseProfile` advertising `Bread`
- `DemandMemory` with `WantedToBuyButSellerOutOfStock` for Bread → triggers `RestockCommodity{Bread}`
- High `enterprise_weight` in `UtilityProfile` (e.g., `pm(900)`)
- Sated merchant (default `HomeostaticNeeds` to suppress survival pressure)
- Mill workstation at Village Square with `WorkstationTag::Mill`
- `build_multi_recipe_registry()` or custom registry containing `Bake Bread` recipe
- Firewood available ONLY at Orchard Farm via `ResourceSource` — no firewood at Village Square or General Store
- Merchant has `PerceptionProfile` and seeded beliefs about Orchard Farm resource source and Mill workstation
- Merchant has `KnownRecipes` including the Bake Bread recipe

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

- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify — add test runner + 2 test functions)

## Out of Scope

- Changes to `goal_model.rs` (covered by S18PREAWAEME-001)
- Changes to `golden_harness/mod.rs` (use existing helpers; if a minor helper is needed, that's acceptable but should be minimal)
- Consumer trade tests (existing and blocked on S10)
- Stale belief golden test (covered by S18PREAWAEME-003)
- Changes to any crate source code — this is a test-only ticket
- Action trace infrastructure changes
- Decision trace infrastructure changes

## Acceptance Criteria

### Tests That Must Pass

1. `golden_merchant_restocks_via_prerequisite_aware_craft` — merchant ends with Bread at General Store
2. `golden_merchant_restocks_via_prerequisite_aware_craft_replays_deterministically` — same seed produces identical hashes
3. Existing suite: `cargo test -p worldwake-ai -- golden_supply_chain`

### Invariants

1. Conservation: `verify_live_lot_conservation()` holds every tick during the test
2. Conservation: `verify_authoritative_conservation()` holds every tick during the test
3. Existing `golden_supply_chain` tests remain green — no setup pollution
4. Merchant plans from beliefs only, never from authoritative world state (Principle 12)
5. All information reaches merchant through seeded beliefs or perception (Principle 7)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_supply_chain.rs::golden_merchant_restocks_via_prerequisite_aware_craft` — proves enterprise restock → prerequisite-aware craft chain
2. `crates/worldwake-ai/tests/golden_supply_chain.rs::golden_merchant_restocks_via_prerequisite_aware_craft_replays_deterministically` — proves determinism

### Commands

1. `cargo test -p worldwake-ai -- golden_merchant_restocks_via_prerequisite_aware_craft`
2. `cargo test -p worldwake-ai -- golden_supply_chain`
3. `cargo test -p worldwake-ai`
