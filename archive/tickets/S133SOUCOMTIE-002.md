# S133SOUCOMTIE-002: Source composite rank module and factor derivation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `worldwake-ai::source_composite` module
**Deps**: archive/tickets/S133SOUCOMTIE-001.md

## Problem

Spec D2 defines `SourceCompositeRank` as the per-candidate derived view that the comparator (ticket 003) and decision trace (ticket 004) consume. Without this module, no consumer of `PreferenceProfile.capacity_observation_weight` exists and the same-commodity tiebreaker has no scoring artifact. Per FND-27 the composite must be a derived per-tick read model; per FND-3 it must be expressed as integer permille math over the existing `ReliabilityRecord` substrate.

## Assumption Reassessment (2026-05-03)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `RankingContext` (`crates/worldwake-ai/src/ranking.rs:206`) exposes `view`, `agent`, `current_tick`, and `view.preference_profile(agent)`/`view.source_reliability(agent)` — verified at `ranking.rs:429-430` where the existing `apply_source_reliability_discount` already reads both. The new module reuses this contract verbatim. `source_reliability_discount_scope` (`ranking.rs:609`) returns the single-source `(EntityId, CommodityKind)` for `AcquireCommodity`/`RestockCommodity`, with the same single-evidence guard the new module needs (FND-15: per-agent local read).
2. Spec D2 dictates the factor formulas, the integer-permille discipline, the 800 wait-cap and the 60-tick wait normalizer. `ReliabilityRecord` exposes `average_wait_ticks`, `wait_observation_count`, `last_observed_capacity`, `last_observed_capacity_tick`, `successful_acquisitions`, `failed_attempts` (verified `crates/worldwake-core/src/experience.rs::ReliabilityRecord`).
3. Shared abstraction boundary under audit: `RankingContext<'_>` and `BeliefView::preference_profile/source_reliability` — the module must compute purely over those reads and never touch authoritative world state.
4. Live `GoalKind` under test: the spec restricts to `AcquireCommodity { commodity, purpose, .. }` and `RestockCommodity { commodity }` — both confirmed live in `crates/worldwake-core/src/goal.rs` and exercised by the existing `source_reliability_discount_scope` helper.
5. Live visibility correction (2026-05-04): `RankingContext` and `source_reliability_discount_scope` are currently file-private in `ranking.rs`. A separate `source_composite.rs` module cannot lawfully reuse them without a narrow crate-visible internal surface. The implementation must update only the needed `ranking.rs` visibility, keeping the new computation read-only and non-public outside `worldwake-ai`.

## Architecture Check

1. Encapsulating the factor math in a dedicated module keeps `ranking.rs` (which is already large at >9000 lines) focused on goal-set assembly. Per FND-3, the factor formulas express integer permille arithmetic with named structural constants (`WAIT_NORMALIZER_TICKS`, `WAIT_PENALTY_CAP_PERMILLE`); this is cleaner than embedding magic numbers inline at the comparator site.
2. No backward-compat shim. The module is net-new; nothing wraps a prior implementation.

## Verification Layers

1. Trust factor neutrality on no-failure record → focused unit test in `source_composite::tests`.
2. Trust factor floors at zero on full-failure record → focused unit test.
3. Wait factor floors at 200 permille under extreme contention (cap = 800) → focused unit test.
4. Capacity factor neutral when stale (`current_tick - last_observed_capacity_tick > memory_retention_ticks`) → focused unit test.
5. Capacity factor neutral when never observed (`wait_observation_count == 0 && last_observed_capacity == 0`) → focused unit test.
6. Capacity factor floors at 500 for empty-but-fresh observations → focused unit test.
7. `compose_factors` clamps at 2000 permille → focused unit test.
8. `source_composite_rank` returns `None` for non-acquisition goals → focused unit test (this is the cross-category-neutrality contract at the source).

## What to Change

### 1. New module `crates/worldwake-ai/src/source_composite.rs`

Public API:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct SourceCompositeRank {
    pub source_entity: EntityId,
    pub commodity: CommodityKind,
    pub trust_factor_permille: u32,
    pub wait_factor_permille: u32,
    pub capacity_factor_permille: u32,
    pub composite_permille: u32,
}

pub(crate) fn source_composite_rank(
    candidate: &GoalOffer,
    context: &RankingContext<'_>,
) -> Option<SourceCompositeRank>;
```

Internal factor functions:
- `trust_factor_permille(record: &ReliabilityRecord, profile: PreferenceProfile) -> u32`: `1000 - (failure_ratio_permille(record) * profile.source_trust_weight.value() as u32) / 1000`, clamped to `[0, 1000]`.
- `wait_factor_permille(record: &ReliabilityRecord, profile: PreferenceProfile) -> u32`: `1000 - min(WAIT_PENALTY_CAP_PERMILLE, (record.average_wait_ticks * profile.wait_sensitivity_weight.value() as u32) / WAIT_NORMALIZER_TICKS as u32)`. Floors at 200.
- `capacity_factor_permille(record: &ReliabilityRecord, profile: PreferenceProfile, current_tick: Tick) -> u32`:
  - When stale (`current_tick.0 - record.last_observed_capacity_tick.0 > profile.memory_retention_ticks`): return 1000 (neutral).
  - When never observed (`record.wait_observation_count == 0 && record.last_observed_capacity == 0`): return 1000.
  - When fresh + empty (`record.last_observed_capacity == 0` and within `memory_retention_ticks`): return `1000 - freshness_factor_permille / 2`, floored at 500.
  - Otherwise: compute `freshness_factor_permille = 1000 - (capacity_age_ticks * 1000 / memory_retention_ticks)` clamped to `[0, 1000]`; `capacity_signal_permille = min(1000, (last_observed_capacity as u32 * 1000) / profile.capacity_observation_weight.value() as u32)`; `bonus = (capacity_signal_permille * freshness_factor_permille) / 1000`; return `1000 + bonus` clamped at 2000.
- `compose_factors(t: u32, w: u32, c: u32) -> u32`: `((t * w / 1000) * c / 1000).min(2000)`.

Private constants:
- `const WAIT_NORMALIZER_TICKS: u64 = 60;` — structural unit conversion: one in-sim hour at the live tick scale (FND-3, not a designer dial).
- `const WAIT_PENALTY_CAP_PERMILLE: u32 = 800;` — guarantees wait factor floors at 200 permille so contention never zeroes the composite.

### 2. Wire the module into the AI crate

`crates/worldwake-ai/src/lib.rs` — add `mod source_composite;` and `pub use source_composite::SourceCompositeRank;` for public surface (the `source_composite_rank` function stays `pub(crate)`).

## Files to Touch

- `crates/worldwake-ai/src/source_composite.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — module declaration and public re-export)
- `crates/worldwake-ai/src/ranking.rs` (modify — crate-visible `RankingContext`/source-scope access for the new sibling module)

## Out of Scope

- Wiring `source_composite_rank` into the comparator and `AgendaEntry` — ticket 003 (placeholder, replaced by ticket 003: this module is invoked but no caller exists yet).
- Trace text formatting — ticket 004.
- Vestigial-field removal on `SourceReliabilityDiscount` — ticket 005.

## Acceptance Criteria

### Tests That Must Pass

1. `trust_factor_neutral_without_failures` — record with `failed_attempts == 0` returns 1000.
2. `trust_factor_floors_at_zero_with_total_failures` — `failure_ratio_permille == 1000` and `source_trust_weight == 1000` returns 0.
3. `wait_factor_caps_at_floor_200_under_extreme_contention` — large `average_wait_ticks` × large `wait_sensitivity_weight` produces 200, never below.
4. `capacity_factor_neutral_for_stale_observation` — stale capacity returns 1000.
5. `capacity_factor_neutral_for_never_observed` — `wait_observation_count == 0 && last_observed_capacity == 0` returns 1000.
6. `capacity_factor_floors_at_500_for_empty_fresh_observation` — fresh `last_observed_capacity == 0` returns 500.
7. `capacity_factor_returns_bonus_for_fresh_full_observation` — capacity == `capacity_observation_weight.value()` and zero age returns 2000.
8. `compose_factors_clamps_at_2000_permille` — three 2000-permille factors compose to 2000, not 8000.
9. `source_composite_rank_returns_none_for_non_acquisition_goal` — `Sleep`, `Wash`, `Patrol` etc. return `None`.
10. `source_composite_rank_returns_none_without_reliability_record` — no `(source_entity, commodity)` record in `SourceReliability.sources` returns `None`.
11. Existing suite: `cargo test --workspace`.

### Invariants

1. All factor math uses `u32`/`u64` integer arithmetic; no floats (CLAUDE.md determinism invariant; FND-3).
2. Module reads only from `RankingContext.view.{source_reliability, preference_profile}` for the agent under rank — no global state read (FND-15).
3. Constants are private to the module; not exposed as designer dials (FND-3).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/source_composite.rs::tests` — all 10 focused tests above.
2. `crates/worldwake-ai/src/lib.rs` — module declaration; no test changes.

### Commands

1. `cargo test -p worldwake-ai source_composite::` (focused).
2. `cargo test -p worldwake-ai` (broader regression).
3. `cargo test --workspace` (full regression — confirms no other crate breaks).

## Outcome

Completed on 2026-05-04.

- Added `crates/worldwake-ai/src/source_composite.rs` with `SourceCompositeRank`, integer-permille trust/wait/capacity factor derivation, composite multiplication/clamping, and the crate-private `source_composite_rank` entry point.
- Re-exported `SourceCompositeRank` from `worldwake-ai` and kept `source_composite_rank` crate-private for ticket 003's comparator integration.
- Made only the required `ranking.rs` internals crate-visible (`RankingContext` plus the existing single-source scope helper) so the new sibling module reuses the live source-reliability boundary instead of duplicating it.
- Added 10 focused unit tests covering the factor math and the non-acquisition / missing-record `None` cases.
- Truth-synced sibling ticket `archive/tickets/S133SOUCOMTIE-005.md` because this ticket already corrected the stale `S132` comment while opening the module boundary.

## Deviations

- The drafted file list omitted `crates/worldwake-ai/src/ranking.rs`; live Rust privacy made a narrow internal visibility edit necessary.
- `source_composite_rank` is staged for ticket 003, so it carries a local `#[allow(dead_code)]` with an explicit comment until the comparator calls it.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib source_composite:: -- --list` (confirmed 10 focused tests).
- Passed `cargo test -p worldwake-ai --lib source_composite::`.
- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo test --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
