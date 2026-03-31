# E22INTSOATES-008: T22 — Bandit Camp Destruction → Diaspora → Reconstitution → Economic Effect

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None
**Deps**: E22INTSOATES-001

## Problem

The existing `golden_t22_bandit_camp_destruction.rs` covers camp destruction, flee/surrender, rally-point regrouping, and route avoidance. T22 extends the chain with: reconstituted camp → new raids from new location → merchant route adaptation → downstream market supply change. This is the longest causal chain in E22.

## Assumption Reassessment (2026-03-31)

1. `GoalKind::EstablishBanditCamp` exists — confirmed in `crates/worldwake-core/src/goal.rs` and `candidate_generation.rs`.
2. `BanditCamp` component exists — confirmed.
3. `BanditFactionPolicy` with `min_regroup_count`, `flee_wound_threshold` exists — confirmed.
4. `GoalKind::RegroupWithFaction` exists — confirmed.
5. `GoalKind::EngageHostile` exists — confirmed.
6. `BelievedActivity` and `BelievedEntityState` exist for route threat beliefs — confirmed.
7. `MerchandiseProfile` exists — confirmed.
8. `verify_authoritative_conservation` exists — confirmed.
9. Existing T22 golden (`golden_t22_bandit_camp_destruction.rs`) has 4 scenarios covering camp destruction, flee/surrender, rally-point regrouping, route avoidance — confirmed.
10. T22 extension adds: camp reconstitution at rally point via `EstablishBanditCamp`, new raids from new location, merchant route adaptation based on beliefs (not danger score), downstream supply change.
11. Decision trace can show merchant route selection based on `BelievedActivity` or `BelievedEntityState` beliefs — confirmed via route_threat module in worldwake-ai.
12. T22 is the longest chain scenario. Setup reuses existing T22 topology pattern plus adds DownstreamMarket and AlternateRoute.
13. No adjacent contradictions.
14. Tick budget: ≤ 10080 ticks (7 days). Camp destruction → diaspora → regrouping → establishment → raids → merchant adaptation.
15. Conservation must hold for all commodity types throughout.

## Architecture Check

1. T22 extension exercises the same bandit camp lifecycle, establishment, and raid systems as the existing golden. The new chain segment (reconstitution → raids → merchant adaptation → supply change) uses existing goal kinds and action handlers. No scenario-specific logic.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. Camp destruction + regrouping → existing coverage (verify as precondition)
2. New camp establishment → authoritative component state (new `BanditCamp` component on place entity after diaspora)
3. Raids from new camp → event-log delta (combat events from entities associated with new camp faction, not old camp)
4. Merchant route adaptation → decision trace (route selection based on `BelievedActivity`/`BelievedEntityState`, not derived danger cache)
5. Supply change → authoritative world state (market commodity quantities change due to longer alternate route)
6. Conservation → `verify_authoritative_conservation` for all commodity types throughout
7. Determinism → state hash comparison across 2 seeds

## What to Change

### 1. Add T22 scenario to `crates/worldwake-ai/tests/golden_integration.rs`

- Reuse topology pattern from existing T22 golden (BanditCamp, BanditWoods, ForestPath, etc.) plus add DownstreamMarket and AlternateRoute places
- Setup camp destruction preconditions (guard force destroys camp)
- Run through: destruction → flee/regroup → `EstablishBanditCamp` at rally point → new raids → merchant belief update → route change → supply delay
- Enable decision tracing on driver
- Verify: new `BanditCamp` component appears on place entity after diaspora phase
- Verify: raid events from new camp faction entities
- Verify: merchant decision traces show route selection based on beliefs, not danger cache
- Verify: `verify_authoritative_conservation` passes throughout
- `fn run_t22_camp_reconstitution(seed: Seed) -> (StateHash, StateHash)`
- Two `#[test]` functions

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify)

## Out of Scope

- Changes to bandit camp, establishment, raid, or trade systems
- Modifying existing `golden_t22_bandit_camp_destruction.rs` scenarios
- Any engine code changes

## Acceptance Criteria

### Tests That Must Pass

1. `t22_camp_reconstitution_seed_1` — full chain from camp destruction to economic effect
2. `t22_camp_reconstitution_seed_2` — determinism verification
3. New `BanditCamp` component appears on a place entity after diaspora phase
4. Raid events originate from entities associated with new camp faction, not old camp
5. Merchant decision traces show route selection based on `BelievedActivity` or `BelievedEntityState` beliefs — not any derived danger cache
6. `verify_authoritative_conservation` passes for all commodity types throughout
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Camp reconstitution follows from `GoalKind::EstablishBanditCamp` — not forced by test logic
2. Merchant route adaptation is belief-driven (Principle 14) — not omniscient
3. Conservation holds for all commodity types at every tick

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs::t22_camp_reconstitution_seed_1` — proves bandit reconstitution → economic effect chain
2. `crates/worldwake-ai/tests/golden_integration.rs::t22_camp_reconstitution_seed_2` — determinism

### Commands

1. `cargo test -p worldwake-ai --test golden_integration -- t22`
2. `cargo test --workspace`
