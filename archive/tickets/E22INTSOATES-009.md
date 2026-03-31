# E22INTSOATES-009: T30 — Seven-Day Autoplay Soak Test

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None
**Deps**: E22INTSOATES-001, E22INTSOATES-002, E22INTSOATES-003, E22INTSOATES-004, E22INTSOATES-005, E22INTSOATES-006, E22INTSOATES-007, E22INTSOATES-008

## Problem

No existing test exercises the full simulation stack under extended autonomous play with a diverse population. T30 is a 20-seed, 10080-tick (7-day) soak test that verifies per-tick invariants (conservation, needs bounds, dead agent inactivity, unique placement, tick monotonicity, causal link integrity), per-run invariants (emergence occurs), and cross-run diversity (emergence is seed-sensitive). Marked `#[ignore]` for CI.

## Assumption Reassessment (2026-03-31)

1. `verify_authoritative_conservation` exists for specific `CommodityKind` values — confirmed.
2. `HomeostaticNeeds` fields are `Permille` bounded by `Permille(1000)` — confirmed.
3. `DeadAt` component exists — confirmed.
4. `Scheduler.current_tick()` API exists — confirmed.
5. `EventLog` with `CauseRef` link integrity — confirmed; each `EventRecord` has a `CauseRef`.
6. `hash_world()` and `hash_event_log()` exist — confirmed.
7. `Seed` type for deterministic RNG — confirmed.
8. All agent profile types referenced in spec (OfficeData, PatrolRoute, BanditCamp, MerchandiseProfile, MetabolismProfile, etc.) exist — confirmed.
9. `build_full_action_registries()` produces the same registries used by all golden tests — confirmed.
10. `PlaceTag` variants (Village, Road, Forest, Farm, Camp, Store) exist — confirmed.
11. T30 depends on all integration scenarios working (001–008) because it exercises the full system stack. If any domain is broken, T30 will surface it.
12. `GoalKind::AcquireCommodity`, `GoalKind::ShareBelief`, `GoalKind::ClaimOffice`, `GoalKind::StealItem` all exist — confirmed.
13. `#[ignore]` attribute — standard Rust test mechanism, run via `cargo test -- --ignored`.
14. No adjacent contradictions.
15. 20 seeded runs × 10080 ticks each. Population of 15–25 agents per run with diverse profiles.

## Architecture Check

1. T30 uses the standard `GoldenHarness` and `build_full_action_registries()`. The soak test adds no special infrastructure — it just runs longer with more agents and checks invariants per-tick. The `#[ignore]` attribute keeps it out of normal CI.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. Conservation per-tick → `verify_authoritative_conservation` for Apple, Grain, Bread, Coin at every tick
2. Needs bounds per-tick → authoritative component read (no `HomeostaticNeeds` field exceeds `Permille(1000)`)
3. Dead agent inactivity → action trace or scheduler check (no action started/completed after `DeadAt` tick)
4. Unique placement → relation query (every agent in exactly one existing place)
5. Tick monotonicity → `Scheduler.current_tick()` strictly increases
6. Causal link integrity → event-log scan (every `CauseRef` references an existing event)
7. Emergence per-run → event-log scan (≥ 1 death, ≥ 1 acquire goal, ≥ 1 travel, ≥ 1 share belief)
8. State change per-run → hash comparison (tick 10080 hash differs from tick 0)
9. Cross-run diversity → hash comparison (not all 20 final hashes identical)
10. Political emergence → event-log scan (≥ 3 runs produce ClaimOffice)
11. Crime emergence → event-log scan (≥ 3 runs produce StealItem)

## What to Change

### 1. Add T30 soak test to `crates/worldwake-ai/tests/golden_integration.rs`

- World builder function creating 8–12 place topology with mixed `PlaceTag`s and varied `TravelEdge` travel_ticks
- Population builder creating 15–25 agents with concrete profiles:
  - 1 ruler (OfficeData, OfficeForceProfile, CombatProfile, UtilityProfile)
  - 2 office claimants (faction membership, UtilityProfile)
  - 1 merchant (MerchandiseProfile, TradeDispositionProfile, MetabolismProfile)
  - 1 carrier (MetabolismProfile)
  - 3 guards (PatrolRoute, PatrolProfile, CombatProfile)
  - 3 bandits (BanditCamp, BanditFactionPolicy, CombatProfile, PursuitProfile)
  - 4+ civilians (HomeostaticNeeds, UtilityProfile, ViolationDispositionProfile, PerceptionProfile, TellProfile)
- `fn run_t30_soak(seed: Seed) -> SoakResult` containing:
  - Per-tick invariant checks (all 6 per-tick invariants)
  - Per-run invariant checks (all 5 per-run invariants)
  - Final state hash
- `fn collect_cross_run_results(seeds: &[Seed]) -> Vec<SoakResult>` running 20 seeds
- Cross-run diversity assertions
- Single `#[test]` `#[ignore]` function: `t30_seven_day_soak`

### 2. Soak result type

Small struct to collect per-run results (final hash, emergence flags, invariant pass/fail) for cross-run analysis.

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify)

## Out of Scope

- Performance optimization of the soak test
- Changes to any engine or system code
- Non-`#[ignore]` test variants

## Acceptance Criteria

### Tests That Must Pass

1. `t30_seven_day_soak` (run via `cargo test -p worldwake-ai --test golden_integration -- --ignored t30`) — 20 seeds complete with zero per-tick invariant violations
2. Per-tick: conservation holds for Apple, Grain, Bread, Coin
3. Per-tick: no `HomeostaticNeeds` field exceeds `Permille(1000)`
4. Per-tick: no dead agent has action started/completed after death tick
5. Per-tick: every agent in exactly one existing place
6. Per-tick: `Scheduler.current_tick()` strictly increases
7. Per-tick: every `CauseRef` references an existing event
8. Per-run: ≥ 1 death, ≥ 1 acquire goal, ≥ 1 travel, ≥ 1 share belief
9. Per-run: state hash at tick 10080 differs from tick 0
10. Cross-run: not all 20 final hashes identical
11. Cross-run: ≥ 3 runs produce `ClaimOffice`
12. Cross-run: ≥ 3 runs produce `StealItem`
13. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Conservation holds for all tracked commodities at every tick of every run
2. World runs without observers (Principle 6): zero human input, meaningful autonomous play
3. Emergence is seed-sensitive: different seeds produce different histories

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs::t30_seven_day_soak` — 20-seed 7-day soak with per-tick/per-run/cross-run invariants

### Commands

1. `cargo test -p worldwake-ai --test golden_integration -- --ignored t30`
2. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-31
- **What changed**: Added `t30_seven_day_soak` test to `crates/worldwake-ai/tests/golden_integration.rs` — 20-seed, 10080-tick soak test with per-tick invariant checks (conservation, needs bounds, dead agent inactivity, unique placement, tick monotonicity, causal link integrity), per-run emergence checks, and cross-run diversity assertions. Marked `#[ignore]` for CI.
- **Deviations from original plan**: None.
- **Verification**: `cargo test --workspace` passes; soak test runnable via `cargo test -p worldwake-ai --test golden_integration -- --ignored t30`.
