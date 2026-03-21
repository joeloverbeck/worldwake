# S18PREAWAEME-003: Golden test — stale prerequisite belief discovery and replan

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only
**Deps**: S18PREAWAEME-001 (prerequisite_places extension for RestockCommodity — needed for craft-based variant, but this test uses ProduceCommodity which already has prerequisite_places support)

## Problem

No golden test exercises the emergent chain: stale belief about resource location → travel to believed location → perception corrects belief on arrival → failed acquisition → `BlockedIntent` recorded → replan toward alternative source → successful acquisition. All infrastructure exists (perception system from E14, belief stores, failure handling, blocked intent from E13, prerequisite_places from S12), but no golden test proves the stale-belief-through-prerequisite-places path end-to-end. This chain validates Principles 7 (locality), 12 (belief ≠ truth), 14 (ignorance is first-class), 15 (violated expectation), and 19 (revisable commitments).

## Assumption Reassessment (2026-03-21)

1. `ProduceCommodity { recipe_id }` already has full `prerequisite_places()` support (goal_model.rs lines 734–759). This test exercises the existing `ProduceCommodity` path with stale beliefs — it does NOT depend on S18PREAWAEME-001's `RestockCommodity` extension.
2. `BlockedIntentMemory` is initialized by `seed_agent()` and `seed_agent_with_recipes()` in the golden harness (mod.rs line 477). `BlockedIntent` records are created by `handle_plan_failure()` in `failure_handling.rs`.
3. The perception system runs on arrival at a place, updating beliefs based on direct observation. Agents with `PerceptionProfile` will observe entities at their current location.
4. `ResourceSource` depletion happens when harvest actions consume `available_quantity`. Alice's scripted harvesting will deplete Orchard Farm firewood before Bob arrives.
5. Two-agent setup: Alice (scripted depletion agent) and Bob (AI agent exercising the stale belief path). Alice can be human-controlled with queued harvest inputs, or AI-controlled with high enough hunger to prioritize local harvest.
6. `seed_actor_beliefs()` can seed Bob's beliefs about specific entities (e.g., Orchard Farm's ResourceSource, Forest Path's ResourceSource) at tick 0. After Alice depletes Orchard Farm, Bob's belief becomes stale.
7. The golden harness has `ORCHARD_FARM` and `FOREST_PATH` as place constants. Both can host `ResourceSource` for firewood.
8. This is a golden E2E test; the intended layer is golden coverage with decision trace + action trace + blocked intent + conservation assertions.
9. Isolation: Bob is the primary AI agent under test. Alice is a depletion agent (AI or scripted). No trade or combat contention.
11. Survivability: Bob needs enough ticks to travel to Orchard Farm (wasted trip), discover depletion, replan, travel to Forest Path, acquire firewood, return, craft, and eat. With default travel times (~6 ticks per leg) and 4 legs + craft + harvest, ~40–60 ticks needed. Bob's metabolism must be slow enough to survive but fast enough to generate hunger pressure. `MetabolismProfile` with moderate hunger rate and high initial hunger should work.

## Architecture Check

1. Follows established golden test patterns: same harness, same assertion hierarchy. The novel element is the two-phase travel pattern (wasted trip → successful trip) validated via action traces.
2. No backward-compatibility shims. New test content only.

## Verification Layers

1. Bob has bread (or has eaten it, reducing hunger) → authoritative world state
2. Firewood at Forest Path consumed → authoritative world state
3. Firewood at Orchard Farm remains at 0 → authoritative world state
4. Two distinct travel sequences — first to Orchard Farm, then to Forest Path → action trace
5. Harvest/PickUp at Orchard Farm results in `StartFailed` or is never attempted (resource depleted) → action trace
6. Craft committed (craft:Bake Bread) after Forest Path acquisition → action trace
7. `BlockedIntentMemory` contains entry related to failed Orchard Farm acquisition → authoritative component state
8. First plan search shows `prerequisite_places` including Orchard Farm → decision trace
9. Second plan search (after replan) shows Forest Path as primary prerequisite place → decision trace
10. `verify_live_lot_conservation()` holds every tick → conservation invariant
11. Deterministic replay: same seed → identical hashes → replay companion test

## What to Change

### 1. Add `run_stale_prerequisite_belief_discovery_replan` function

In `crates/worldwake-ai/tests/golden_supply_chain.rs`, add a new test runner function. Setup:

**World layout**:
- Village Square: Mill workstation (`WorkstationTag::Mill`)
- Orchard Farm: `ResourceSource { commodity: Firewood, available_quantity: Quantity(5), ... }` — will be depleted by Alice
- Forest Path: `ResourceSource { commodity: Firewood, available_quantity: Quantity(5), ... }` — Bob's fallback

**Agent Alice** (depletion agent):
- Starts at Orchard Farm with `PerceptionProfile`
- AI-controlled with high hunger or scripted with queued harvest inputs
- Purpose: deplete all firewood at Orchard Farm in the first ~10 ticks
- Sated (no competing needs) with `KnownRecipes` for harvest

**Agent Bob** (primary test subject):
- Starts at Village Square (or repositioned there after initial belief seeding)
- `PerceptionProfile` with `DirectObservation` for observation on arrival
- `KnownRecipes` including Bake Bread recipe
- Beliefs seeded at tick 0:
  - Firewood at Orchard Farm (from direct observation or inference) — becomes stale after Alice depletes
  - Firewood at Forest Path (from inference) — remains valid
  - Mill workstation at Village Square
- `HomeostaticNeeds` with hunger pressure (e.g., `pm(700)` hunger) to drive `ProduceCommodity{Bread}` goal
- `MetabolismProfile` with slow enough rate to survive the full chain

**Tick budget**: ~80–120 ticks to allow full two-trip chain.

Assertions follow the hierarchy in Verification Layers above.

### 2. Add `golden_stale_prerequisite_belief_discovery_replan` test

```rust
#[test]
fn golden_stale_prerequisite_belief_discovery_replan() {
    let (wh, eh) = run_stale_prerequisite_belief_discovery_replan(Seed(/* chosen seed */));
    // hashes recorded after first green run
}
```

### 3. Add deterministic replay companion

```rust
#[test]
fn golden_stale_prerequisite_belief_discovery_replan_replays_deterministically() {
    let seed = Seed(/* same seed */);
    let (wh1, eh1) = run_stale_prerequisite_belief_discovery_replan(seed);
    let (wh2, eh2) = run_stale_prerequisite_belief_discovery_replan(seed);
    assert_eq!(wh1, wh2, "world hash diverged");
    assert_eq!(eh1, eh2, "event log hash diverged");
}
```

## Files to Touch

- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify — add test runner + 2 test functions)

## Out of Scope

- Changes to `goal_model.rs` (covered by S18PREAWAEME-001)
- Changes to `golden_harness/mod.rs` (use existing helpers; if a `FOREST_PATH` constant or similar minor addition is needed, that's acceptable)
- Changes to perception system, belief stores, or failure handling code
- Changes to `BlockedIntentMemory` or `BlockedIntent` types
- Merchant restock golden test (covered by S18PREAWAEME-002)
- Changes to any crate source code — this is a test-only ticket
- Testing `RestockCommodity` stale belief path (this test uses `ProduceCommodity` which already has full prerequisite_places support)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_stale_prerequisite_belief_discovery_replan` — Bob ends with bread (or reduced hunger); Orchard Farm firewood at 0; Forest Path firewood consumed
2. `golden_stale_prerequisite_belief_discovery_replan_replays_deterministically` — same seed produces identical hashes
3. Existing suite: `cargo test -p worldwake-ai -- golden_supply_chain`

### Invariants

1. Conservation: `verify_live_lot_conservation()` holds every tick
2. Bob plans from beliefs only, never from authoritative world state (Principle 12)
3. Belief correction happens ONLY through perception on arrival, not through omniscient update (Principle 7)
4. `BlockedIntent` machinery prevents re-attempting failed Orchard Farm acquisition within blocking period (Principle 19)
5. Existing `golden_supply_chain` tests remain green — no setup pollution
6. Alice's depletion is a lawful world process (harvest actions), not a scripted state mutation

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_supply_chain.rs::golden_stale_prerequisite_belief_discovery_replan` — proves stale belief → wasted trip → perception correction → replan → successful alternative acquisition
2. `crates/worldwake-ai/tests/golden_supply_chain.rs::golden_stale_prerequisite_belief_discovery_replan_replays_deterministically` — proves determinism

### Commands

1. `cargo test -p worldwake-ai -- golden_stale_prerequisite_belief_discovery_replan`
2. `cargo test -p worldwake-ai -- golden_supply_chain`
3. `cargo test -p worldwake-ai`
