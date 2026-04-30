# S129PLADIRFAC-010: Hygiene ranking integration

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — extends `motive_score` / candidate ranking in `crates/worldwake-ai/src/ranking.rs` for Sleep, ExploreLocation, Wash, Relieve goals
**Deps**: archive/tickets/S129PLADIRFAC-001.md, archive/tickets/S129PLADIRFAC-004.md, S129PLADIRFAC-009

## Problem

Without hygiene-aware ranking, the per-basin and per-place-latrine candidates produced by ticket 009 carry no scoring differentiation — every basin scores the same regardless of `clean_water_units`, every latrine scores the same regardless of `LatrineFullness.fill`, every place scores the same for sleep regardless of `PlaceDirtiness.value`. The S129 emergent chain ("dirty place → bad sleep → travel decision") requires ranking to read these new components and bias scores accordingly. This ticket lands the ranking arithmetic for all four affected goal kinds.

## Assumption Reassessment (2026-04-29)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `motive_score` function in `crates/worldwake-ai/src/ranking.rs` is match-based at lines ~985–1170 (verified during reassessment). Sleep / Wash / Relieve / ExploreLocation arms exist and reference per-need `drive_score` calls. Sleep-specific post-processing at lines ~1649–1665 multiplies the base by `recovery_modifier` from `SleepQualityProfile` — this is the integration anchor for `PlaceDirtiness`.
2. `GoalKind` variants `Sleep`, `Wash`, `Relieve`, `ExploreLocation` exist at `crates/worldwake-core/src/goal.rs:60–62` (and surrounding). Confirm `ExploreLocation` exact name during implementation — it may also be called `Explore` or live in a parent enum.
3. `ProfileBeliefView` accessors `place_dirtiness`, `latrine_fullness`, `wash_basin_state` (from ticket 004) are auto-forwarded to `GoalBeliefView` consumers via the blanket impl at `belief_view.rs:1359`. Ranking reads `context.view.place_dirtiness(agent, place)` etc.
4. Per spec D10's saturating arithmetic: `(1000 - place_dirtiness.value) / 1000` is the dirtiness multiplier for Sleep and ExploreLocation. To avoid overflow with `u32` intermediates: `base.saturating_mul(u32::from((Permille::FULL.value() - dirtiness.value().min(1000)))) / 1000`. (Note: `Permille::FULL` doesn't exist in the codebase — use literal `1000` instead per ticket 005's pattern.) Ranking-sensitive precision (precision-rules §13): validate the live arithmetic during implementation; equal weights alone aren't sufficient.
5. Per Q2=(a), wash candidate ranking: each per-basin candidate scored by basin state. Spec D10 says "biases toward basins with higher `clean_water_units` and lower `dirtiness_level`". Concrete formula to confirm during implementation; a reasonable shape is `base_score * (clean_water_units / max_clean_water) * (1000 - dirtiness_level) / 1000`. Capture the exact formula during the implementation reassessment per precision-rules §7 (cumulative arithmetic).
6. Per Q3=(a), relieve candidate ranking: each per-place-latrine candidate scored against `latrine_fullness.fill`. Latrines below `critical_threshold` rank above the wilderness fallback; if all latrines are at or over `critical_threshold`, the wilderness candidate ranks highest (the spec's natural fallback path). Concrete shape: candidates with `fill < critical_threshold` get a positive bias; candidates with `fill >= critical_threshold` get a penalty bringing them below the wilderness candidate. Validate the exact formula during implementation.
7. Existing focused/unit coverage: grep `ranking.rs`'s test module for tests asserting Sleep / Wash / Relieve / ExploreLocation `motive_score` outcomes. Tests will need extending to cover the new component-aware paths. The `place_sleep_quality_profile` integration at line ~1664 has existing test coverage that this ticket's changes preserve (the dirtiness multiplier is applied *after* the recovery modifier per spec D10).
8. Behavioral claim validation (codebase-validation §3.9): the ranking-side reads happen at runtime; verified that ProfileBeliefView accessors at `belief_view.rs:758` are the canonical read surface (no test-only accessor leakage).

## Architecture Check

1. Multiplicative integration of `(1000 - place_dirtiness.value) / 1000` *after* `recovery_modifier` (rather than as a sibling additive penalty) preserves the S128-established convention: place quality is a chained multiplier on the base motive score. A pristine place (`value=0`) yields full multiplier; a fully-dirty place (`value=1000`) yields zero — both extremes match spec D10's intent. Adding the dirtiness multiplier at the same stage as `recovery_modifier` keeps ranking arithmetic compositional rather than special-casing dirtiness.
2. Per-basin and per-place-latrine candidates, by virtue of ticket 009's per-anchor split, allow ranking to read state directly through the candidate's anchor — no candidate-time iteration over sibling basins/latrines, no second-tier disambiguation. Per Candidate Scoring Architecture rule, scoring lives in ranking, not in emission.
3. No backward-compat shim. The existing Sleep `recovery_modifier` integration is preserved unchanged; new dirtiness multiplier is additive at the same stage. Wash and Relieve `motive_score` arms gain new state reads but no parallel "old ranking path" remains.

## Verification Layers

1. Sleep ranking biases away from dirty places → focused unit test seeding two candidate places, identical `SleepQualityProfile`, place A with `PlaceDirtiness.value = pm(800)`, place B with `value = pm(100)`. Compute `motive_score` for each. Assert place B's score > place A's score.
2. ExploreLocation ranking biases the same way → analogous focused test.
3. Wash ranking biases toward basins with more clean water → focused unit test seeding two basin candidates at the same place, basin A with `clean_water_units: 1`, basin B with `clean_water_units: 9`. Assert basin B's score > basin A's score.
4. Wash ranking biases toward basins with lower dirtiness → seeded `clean_water_units: 5` for both, basin A `dirtiness_level: pm(800)`, basin B `dirtiness_level: pm(100)`. Assert basin B's score > basin A's score.
5. Relieve ranking prefers under-threshold latrines over wilderness → focused unit test seeding one latrine candidate `fill: pm(400), critical_threshold: pm(800)` and a wilderness fallback. Assert latrine score > wilderness score.
6. Relieve ranking falls through to wilderness when all latrines critical → focused unit test seeding one latrine `fill: pm(900), critical_threshold: pm(800)` and a wilderness fallback. Assert wilderness score > latrine score.
7. Existing Sleep `recovery_modifier` integration unchanged in zero-dirtiness case → focused unit test confirming that with `PlaceDirtiness::default()` (value=0), the resulting score equals the pre-ticket `recovery_modifier`-only score.

## What to Change

### 1. Sleep ranking — extend `recovery_modifier` integration site

At `ranking.rs:1649–1665` (the existing `recovery_modifier` post-processing block), append a `PlaceDirtiness` multiplier:

```rust
let recovery = view.place_sleep_quality_profile(agent, place).recovery_modifier;
let dirtiness = view.place_dirtiness(agent, place).value;

let after_recovery = base
    .saturating_mul(u32::from(recovery.value()))
    / 1000;
let dirtiness_factor = u32::from(1000_u16.saturating_sub(dirtiness.value().min(1000)));
let after_dirtiness = after_recovery
    .saturating_mul(dirtiness_factor)
    / 1000;
after_dirtiness
```

Confirm exact `base` type (`u32` per existing code) during implementation.

### 2. ExploreLocation ranking — apply the same dirtiness multiplier

Locate the ExploreLocation arm in the `motive_score` match (search for the `ExploreLocation` variant). After whatever existing place-quality input it computes, multiply by the same `dirtiness_factor`. If ExploreLocation doesn't currently read place quality, this is the introduction site for that read.

### 3. Wash ranking — score per-basin candidate by basin state

Locate the Wash arm of `motive_score`. Today it scores via `drive_score` against `HomeostaticNeedId::Dirtiness`. Extend to read the candidate's anchor (`OpportunityAnchor::Facility(basin_id)`) and compute:

```rust
let basin_state = view.wash_basin_state(agent, basin_id);
let water_factor = if basin_state.units_per_full_wash == 0 {
    0
} else {
    (u32::from(basin_state.clean_water_units) * 1000) / u32::from(basin_state.units_per_full_wash).max(1)
}.min(1000);  // cap at 1.0 — basins with more than units_per_full_wash water are equally good

let dirtiness_factor = u32::from(1000_u16.saturating_sub(basin_state.dirtiness_level.value().min(1000)));

let basin_quality = water_factor.saturating_mul(dirtiness_factor) / 1000;

base_dirtiness_drive_score.saturating_mul(basin_quality) / 1000
```

(Verify exact arithmetic with the implementation reassessment; the formula should match spec D10's "biases toward basins with higher `clean_water_units` and lower `dirtiness_level`" intent.)

### 4. Relieve ranking — score per-place-latrine candidate against fullness

Locate the Relieve arm. Today it scores via `drive_score` against `HomeostaticNeedId::Bladder`. Extend to read the candidate's anchor:

- If `OpportunityAnchor::Place(place_id)` (a latrine candidate): read `view.latrine_fullness(agent, place_id)`. If `fill < critical_threshold`, apply a positive multiplier (e.g., `1000`); if `fill >= critical_threshold`, apply a penalty multiplier that brings the score below the wilderness candidate's baseline.
- If `OpportunityAnchor::None` (the wilderness fallback): apply a baseline penalty (e.g., a fixed reduction to reflect "wilderness relief is the worst-case fallback").

The exact penalty/bonus values are spec-D10's "prefer below `critical_threshold` over wilderness; if all critical, fall through to wilderness". Validate the live arithmetic during implementation per precision-rules §13.

### 5. Update existing tests

Existing Sleep/Wash/Relieve/ExploreLocation `motive_score` tests — extend assertions to seed the new components and verify the ranking biases.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — Sleep arm at ~1664, ExploreLocation arm at ~1026 or wherever it sits, Wash arm at ~1032, Relieve arm at ~1037; add `view.place_dirtiness` etc. reads; new tests in the inline test module)

## Out of Scope

- Per-basin / per-place-latrine candidate emission — landed in ticket 009.
- Belief-view accessors — landed in ticket 004.
- Component definitions — landed in ticket 001.
- Authoritative outcome (action commit) — landed in tickets 005/006/007.
- Maintenance pass (decay/refill) — landed in ticket 008.
- Golden coverage — deferred to ticket 012.

## Acceptance Criteria

### Tests That Must Pass

1. New focused test `sleep_ranking_biases_against_dirty_place` — clean place outranks dirty place.
2. New focused test `explore_location_ranking_biases_against_dirty_place` — analogous.
3. New focused test `wash_ranking_biases_toward_clean_water_basin` — basin with more `clean_water_units` outranks basin with less.
4. New focused test `wash_ranking_biases_against_dirty_basin` — basin with lower `dirtiness_level` outranks basin with higher.
5. New focused test `relieve_ranking_prefers_under_threshold_latrine_over_wilderness` — latrine with `fill < critical_threshold` outranks wilderness candidate.
6. New focused test `relieve_ranking_falls_through_to_wilderness_when_all_latrines_critical` — wilderness candidate outranks latrines all over `critical_threshold`.
7. New focused test `sleep_ranking_unchanged_at_zero_dirtiness` — regression guard: pre-ticket score parity at default `PlaceDirtiness`.
8. Existing `motive_score` tests in `ranking.rs`'s inline test module continue to pass (with updated seed assertions where the new components are now read).
9. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. Sleep score at `PlaceDirtiness::default()` (value=0) equals the pre-ticket Sleep score (no regression in clean-place scoring).
2. Sleep score at `PlaceDirtiness.value = pm(1000)` equals zero (fully-dirty multiplier zeroes the recovery contribution).
3. Wash candidate ranking is monotonic in `clean_water_units` (more water → higher score, all else equal) and in `1000 - dirtiness_level` (cleaner basin → higher score).
4. Relieve candidate ranking has the wilderness fallback strictly outranked by any under-`critical_threshold` latrine candidate, and strictly outranking any at-or-over-`critical_threshold` latrine candidate (per spec D10 + Q3=(a)).
5. Ranking is FND-7 / FND-14A-compliant: every read goes through `ProfileBeliefView` accessors (which already encode co-located perception), never through direct world reads.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` test module — seven new focused tests; existing motive_score tests updated to seed the new components.

### Commands

1. `cargo test -p worldwake-ai ranking`
2. `cargo test -p worldwake-ai`
3. `cargo build --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
