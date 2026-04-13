# S80EXPDRI-008: Extend ExploreLocation candidate generation to Dirtiness need

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — candidate_generation.rs (AI layer, candidate emission)
**Deps**: S80 (exploration-drive, archived/implemented)

## Problem

`emit_exploration_candidates()` in `candidate_generation.rs:2362-2372` only iterates over `[Hunger, Thirst]` when deciding whether to emit `ExploreLocation` goals. Dirtiness is excluded. When an agent has critical dirtiness and no local Water for washing, the exploration system never fires because dirtiness is not checked as a motivating need.

In the cli-evaluation scenario (seed 7777), Forager Lina reached dirtiness=1000 over 810 ticks while never leaving Eldergrove Forest. She has an ExplorationProfile (defaults: curiosity_weight=500, need_activation_threshold=400) and dirtiness exceeded the threshold by tick ~400, but ExploreLocation was never emitted for dirtiness.

Fatigue and Bladder are excluded intentionally: Sleep and Relieve are available at any location (no facility or commodity required), so exploration cannot help satisfy them.

## Assumption Reassessment (2026-04-13)

1. `emit_exploration_candidates` at `crates/worldwake-ai/src/candidate_generation.rs:2331-2393` confirmed: the needs loop at line 2362-2372 contains only `HomeostaticNeedId::Hunger` and `HomeostaticNeedId::Thirst`. No other need variants are checked.
2. `relieves_hunger` at line 4896-4901 and `relieves_thirst` at line 4903-4908 use `consumable_profile` to match commodities. Washing uses Water directly — `emit_wash_goal` at line 2568-2601 checks `local_controlled_commodity_evidence(Water)`. A `relieves_dirtiness` function should return `true` for `CommodityKind::Water` since Water is the commodity consumed during washing.
3. This is a single-layer ticket (AI candidate generation). The shared abstraction boundary is the `GoalKind::ExploreLocation { motivating_need }` variant which already accepts any `HomeostaticNeedId`.
4. Not a failing golden scenario — this is a missing candidate generation path identified by diagnostic analysis.
5. Live `GoalKind` is `ExploreLocation { target_place, motivating_need }`. The operator surface is `emit_exploration_candidates` → `select_exploration_target` → `emit_candidate_with_trace`. Adding `Dirtiness` as a `motivating_need` requires no operator changes — the downstream search and action handlers already support arbitrary `HomeostaticNeedId` in the `ExploreLocation` variant.
6. AI regression layer: candidate-generation focused/unit coverage. No `agent_tick` harness needed — the change is entirely within `emit_exploration_candidates`.
7. N/A — no ordering dependency.
8. N/A — no heuristic removal.
9. N/A — not a stale-request ticket.
10. N/A — not a political office ticket.
11. N/A — no ControlSource manipulation.
12. N/A — no scenario isolation concerns.
13. No adjacent contradictions found.
14. No mismatch — ticket scope confirmed against current code.
15. `need_activation_threshold` default is 400‰. Dirtiness rate is 1/tick for Forager Lina. At tick 400, dirtiness reaches 400‰ which crosses the threshold. `any_local_need_relief(relieves_dirtiness)` will check for Water at the agent's location or in inventory. `need_has_known_acquisition_path(relieves_dirtiness)` will check for known Water sources. Both are valid for the dirtiness-as-water-need pattern.

## Architecture Check

1. This extends the existing exploration loop with one additional need variant. The `any_local_need_relief` and `need_has_known_acquisition_path` functions already accept `fn(CommodityKind) -> bool` — adding `relieves_dirtiness` (which checks for Water) slots into the same pattern with no architectural changes. The `ExploreLocation` goal variant already accepts any `HomeostaticNeedId`, so the downstream search and action handlers require no changes.
2. No backwards-compatibility shims. One new function (`relieves_dirtiness`) and one additional entry in the needs loop.

## Verification Layers

1. ExploreLocation candidate emitted for Dirtiness when no local Water and no known acquisition path → focused unit test in `candidate_generation.rs` tests
2. ExploreLocation candidate NOT emitted for Dirtiness when local Water exists → focused unit test
3. ExploreLocation candidate NOT emitted for Dirtiness when known Water acquisition path exists → focused unit test
4. Single-layer ticket — AI candidate generation only. No authoritative action or event-log changes.

## What to Change

### 1. Add `relieves_dirtiness` function

In `crates/worldwake-ai/src/candidate_generation.rs`, after `relieves_thirst` (line ~4908):

```rust
fn relieves_dirtiness(commodity: CommodityKind) -> bool {
    commodity == CommodityKind::Water
}
```

Washing consumes Water (see `emit_wash_goal` at line 2579 which checks for controlled Water). This function does not use `consumable_profile` because Water's dirtiness-relieving property is through the Wash action, not through consumption.

### 2. Extend the needs loop in `emit_exploration_candidates`

At line 2362-2372, add Dirtiness to the loop:

```rust
for (need_id, pressure, matches_need) in [
    (
        HomeostaticNeedId::Hunger,
        needs.hunger,
        relieves_hunger as fn(CommodityKind) -> bool,
    ),
    (
        HomeostaticNeedId::Thirst,
        needs.thirst,
        relieves_thirst as fn(CommodityKind) -> bool,
    ),
    (
        HomeostaticNeedId::Dirtiness,
        needs.dirtiness,
        relieves_dirtiness as fn(CommodityKind) -> bool,
    ),
]
```

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- Adding Fatigue or Bladder to the exploration loop (Sleep/Relieve are location-independent)
- Adding Pain to the exploration loop (separate from homeostatic needs system)
- Changing ExploreLocation action handlers or search operators
- Scenario file changes

## Acceptance Criteria

### Tests That Must Pass

1. New test: ExploreLocation candidate is emitted with `motivating_need: Dirtiness` when dirtiness exceeds `need_activation_threshold` and no local Water and no known Water acquisition path
2. New test: ExploreLocation candidate is NOT emitted for Dirtiness when agent has controlled Water locally
3. New test: ExploreLocation candidate is NOT emitted for Dirtiness when known acquisition path for Water exists
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. ExploreLocation candidates for Dirtiness use the same gating logic as Hunger/Thirst (pressure threshold, local relief check, known path check)
2. No ExploreLocation candidates for Fatigue or Bladder (these needs remain excluded)

## Test Plan

### New/Modified Tests

1. `candidate_generation::tests::generate_candidates_emits_exploration_for_critical_dirtiness_without_water` — proves ExploreLocation fires for Dirtiness when Water is unavailable
2. `candidate_generation::tests::generate_candidates_skips_dirtiness_exploration_when_water_available` — proves local Water suppresses Dirtiness exploration
3. `candidate_generation::tests::generate_candidates_skips_dirtiness_exploration_when_water_path_is_known` — proves known Water source suppresses Dirtiness exploration

### Commands

1. `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_emits_exploration_for_critical_dirtiness_without_water -- --exact`
2. `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_skips_dirtiness_exploration_when_water_available -- --exact`
3. `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_skips_dirtiness_exploration_when_water_path_is_known -- --exact`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-13.

- Extended `emit_exploration_candidates()` to treat `HomeostaticNeedId::Dirtiness` as a lawful `ExploreLocation` motivator using the same threshold, local-relief, and known-acquisition-path gates already used for hunger and thirst.
- Added `relieves_dirtiness()` as the commodity matcher for wash-relevant exploration gating; it maps dirtiness relief to `CommodityKind::Water`, matching the existing wash candidate path.
- Added three focused `candidate_generation` regressions proving dirtiness-driven exploration emission when water is unavailable and suppression when local water or a known water path already exists.
- No deviations from the ticket's planned production scope or proof surface were required.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_emits_exploration_for_critical_dirtiness_without_water -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_skips_dirtiness_exploration_when_water_available -- --exact`
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::generate_candidates_skips_dirtiness_exploration_when_water_path_is_known -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`

## Worktree Notes

- Modified tracked file: `crates/worldwake-ai/src/candidate_generation.rs`
- This ticket file is updated in place under `tickets/`
- Unrelated worktree changes were already present outside this ticket's scope and were left untouched
