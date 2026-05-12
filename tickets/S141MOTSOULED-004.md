# S141MOTSOULED-004: `GoalOffer.motive_sources` + `motive_score` body refactor + mapping helper

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `GoalOffer` extension, `motive_score` body refactor, new mapping helper module, populates `RankedGoalSummary.motive_source_contributions`
**Deps**: `archive/tickets/S141MOTSOULED-001.md` (uses `MotiveSource`, `MotiveSourceRef`), `archive/tickets/S141MOTSOULED-002.md` (reads new `UtilityProfile` weights), `archive/tickets/S141MOTSOULED-003.md` (declares `motive_source_contributions`)

## Problem

This is the S141 critical-path "switchover" ticket. Two coupled deliverables land together because splitting them creates a transient "carrier with no consumer" state in a live ranking-authority path (FND-28-driven combining per `tickets/_TEMPLATE.md` review guidance):

- **D2**: Add `motive_sources: Vec<MotiveSourceRef>` to `GoalOffer` as a required non-empty field; populate it at every construction site via a new `derive_default_motive_sources(GoalKind, OpportunityAnchor)` helper.
- **D3**: Refactor the body of `motive_score` (`crates/worldwake-ai/src/ranking.rs:1007`) from `match candidate.key.goal_kind { ... }` to `candidate.motive_sources.iter().map(score_motive_source).sum()`. Per-variant scoring helpers extract today's `motive_score` match arms.

The acceptance gate is **score parity**: every existing 1440-tick survival golden produces bitwise-identical `motive_score` values pre/post-S141 for every commit. This is the strongest regression guard against derivation drift.

## Assumption Reassessment (2026-05-12)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `GoalOffer` lives in `worldwake-ai` at `crates/worldwake-ai/src/goal_model.rs:2038` and derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` (per S141 reassessment). 13 struct-literal construction sites exist workspace-wide (per Step 2 sub-check (d)): 3 production paths in `crates/worldwake-ai/src/candidate_generation.rs` (lines 554, 4808, 5420) plus 10 sites across `crates/worldwake-ai/src/goal_model.rs`, `agent_tick/planning.rs`, `search/strategic.rs`, `ranking.rs`, `plan_selection.rs`, `source_composite.rs`, and tests. All 13 must populate `motive_sources` because the field is required non-empty post-S141.
2. `motive_score` currently lives at `crates/worldwake-ai/src/ranking.rs:1007` with signature `fn motive_score(candidate: &GoalOffer, context: &RankingContext<'_>) -> u32` and dispatches on `GoalKind` to per-family helpers (`drive_score`, `enterprise_score`, `raid_target_motive`, etc.). `compare_ranked_goals` at `ranking.rs:2615` is the file-private comparator per S123; its identity is preserved by this ticket. 15 existing tests in `crates/worldwake-ai/src/ranking.rs#[cfg(test)]` exercise motive_score/ranking behavior and must continue to pass:
   - `source_composite_tiebreaker_fires_when_motive_score_tied_and_peer_keys_match` (line 802)
   - `crime_goals_use_profile_driven_motive_scores` (line 1388)
   - `ranking_context_prunes_stale_obligation_execution_ticks` (line 2034)
   - `motive_score_falls_back_to_success_ratio_for_acquire_commodity` (line 3386)
   - `ranking_is_deterministic_for_identical_inputs` (line 4896)
   - `ranking_outcome_ordered_reflects_ranked_field` (line 6113)
   - `compare_ranked_goals_is_the_only_impl_in_crate` (line 6144)
   - `sleep_ranking_biases_against_dirty_place` (line 6288)
   - `explore_location_ranking_is_not_biased_by_place_dirtiness` (line 6336)
   - `ranking_pushes_damping_entry_when_explore_location_is_damped` (line 6496)
   - `wash_ranking_biases_toward_clean_water_basin` (line 6559)
   - `wash_ranking_biases_against_dirty_basin` (line 6605)
   - `relieve_ranking_prefers_under_threshold_latrine_over_wilderness` (line 6653)
   - `relieve_ranking_falls_through_to_wilderness_when_all_latrines_critical` (line 6687)
   - `sleep_ranking_unchanged_at_zero_dirtiness` (line 6721)
3. Shared abstraction boundary: the `GoalKind → MotiveSource` mapping (in the new `derive_default_motive_sources` helper) and the `MotiveSource → score` mapping (in the new `score_motive_source` dispatch) jointly preserve the existing per-`GoalKind` score arithmetic. Score parity is the contract; any drift between today's match-on-GoalKind and the post-refactor sum-over-motive-sources is a bug to fix, never a test to weaken (FND-3, FND-28). Per `docs/precision-rules.md` Rule 7 (cumulative arithmetic), the per-variant scoring helpers must reproduce today's exact `u32` arithmetic — equal-weight assumptions are not enough; the full active substrate (pressure, weights, hygiene modifiers, memory bonuses) must be partitioned across the new helpers.
4. The `derive_default_motive_sources` mapping is the load-bearing structural change. For each existing `GoalKind` variant, the helper returns a `Vec<MotiveSourceRef>` whose per-variant contributions, summed by `score_motive_source`, exactly equal today's `motive_score` body arm for that `GoalKind`. Ambiguous mappings (enterprise → `Greed`?, crime → `Revenge` with the right `ViolationId`?) are part of this ticket's scope and must be audited per `GoalKind`.
5. The 13 GoalOffer construction sites use field-by-field enumeration (no `..Default::default()` spread); each site must explicitly add `motive_sources: derive_default_motive_sources(&goal_kind, &anchor)` (or pass an explicit override where the site has richer context). Test-build `debug_assert!(!offer.motive_sources.is_empty())` will fire if a site forgets to populate.
6. Ranking-sensitive precision (per `docs/precision-rules.md` Rule 5): the divergence driver for `motive_score` is the per-`MotiveSource` weight × strength product. After refactor, two agents with identical world state but different per-`MotiveSource`-class weights on `UtilityProfile` (from `archive/tickets/S141MOTSOULED-002.md`) must produce different `motive_score` values — this is FND-22 diversity. Verify against `crime_goals_use_profile_driven_motive_scores` (line 1388), which already asserts this for crime goals; extend coverage to the new motive classes in 007.

## Architecture Check

1. The per-variant scoring helpers are direct extractions of today's `motive_score` body — no semantic change, only structural partitioning along the `MotiveSource` axis instead of the `GoalKind` axis. This satisfies FND-3 (concrete state over abstract scores) because the score's contribution sources are now per-motive-class, traceable to per-agent state, and inspectable via `RankedGoalSummary.motive_source_contributions`.
2. Co-locating D2 and D3 in one ticket avoids the FND-28 transient state where `motive_sources` exists as a populated but unread carrier. Score parity is testable in one shot.
3. The new `motive_source_mapping.rs` module is the single authoritative `GoalKind → MotiveSource` mapping. The 3 helper sites in `candidate_generation.rs` (lines 554, 4808, 5420) call it; per-emitter overrides are permitted only when the emitter has richer context (e.g., a recorded-violation emitter knows the exact `ViolationId`). FND-26 (systems interact through state) is preserved — the mapping reads `GoalKind` + `OpportunityAnchor` from the offer, no cross-system calls.
4. `compare_ranked_goals` (file-private per S123) is unchanged in identity; only the body of its callee `motive_score` is partitioned. The "one comparator" invariant tested at `ranking.rs:6144` (`compare_ranked_goals_is_the_only_impl_in_crate`) remains valid.

## Verification Layers

1. Score parity → every existing 1440-tick survival golden produces bitwise-identical `motive_score` values pre/post-S141 for every commit. Verified via `cargo test --workspace` showing no regression in `crates/worldwake-ai/tests/golden_survival_*.rs`.
2. Per-variant dispatch correctness → focused unit tests in `crates/worldwake-ai/src/ranking.rs#[cfg(test)]` for each new helper (`score_need_pressure`, `score_pain`, `score_office_duty`, `score_loyalty`, `score_greed`, `score_shame`, `score_revenge`).
3. Mapping correctness → focused unit tests in `crates/worldwake-ai/src/motive_source_mapping.rs#[cfg(test)]` covering every active `GoalKind` variant.
4. Trace population → `RankedGoalSummary.motive_source_contributions` is non-empty for every ranked candidate; verified by extending the existing decision-trace assertions in goldens that already inspect `RankedGoalSummary`.
5. Empty-vec invariant → test-build `debug_assert!(!offer.motive_sources.is_empty())` fires on any construction site that forgets to populate; covered by a focused test that constructs an offer with empty motive_sources and expects panic.
6. Backward compatibility → none; per FND-28, post-S141 offers without explicit `motive_sources` are invalid. There is no fallback path to today's `match goal_kind` body — the old body is removed.

## What to Change

### 1. Extend `GoalOffer` struct

At `crates/worldwake-ai/src/goal_model.rs:2038` add the new field:

```rust
pub struct GoalOffer {
    // existing fields preserved
    pub motive_sources: Vec<MotiveSourceRef>,
}
```

Insert `use worldwake_core::motive_source::MotiveSourceRef;` at the top.

### 2. New module `crates/worldwake-ai/src/motive_source_mapping.rs`

```rust
use worldwake_core::motive_source::{MotiveSource, MotiveSourceRef};

pub fn derive_default_motive_sources(
    goal_kind: &GoalKind,
    anchor: &OpportunityAnchor,
    introduced_tick: Tick,
) -> Vec<MotiveSourceRef> { /* … per-GoalKind mapping … */ }
```

The mapping partitions every active `GoalKind` variant into 1–N `MotiveSource` references whose sum equals today's `motive_score` arm for that `GoalKind`. The implementation phase reads today's `motive_score` body and constructs the mapping by direct extraction.

### 3. Populate `motive_sources` at the 3 candidate-generation helper sites

At `crates/worldwake-ai/src/candidate_generation.rs` lines 554, 4808, 5420, attach `motive_sources: derive_default_motive_sources(&goal_kind, &anchor, current_tick)` to the `GoalOffer { … }` literal. All 53 `emit_*_candidates` functions route through one of these three sites, so the central mapping is exercised uniformly.

### 4. Populate `motive_sources` at the 10 remaining `GoalOffer { … }` literal sites

Across `goal_model.rs`, `agent_tick/planning.rs`, `search/strategic.rs`, `ranking.rs`, `plan_selection.rs`, `source_composite.rs`, and the relevant tests. Use the mapping helper or an explicit context-derived override.

### 5. Add test-build empty-vec debug assertion

Define a `pub fn new(...) -> GoalOffer` constructor (or extend an existing helper) that calls `debug_assert!(!motive_sources.is_empty(), "GoalOffer.motive_sources must be non-empty post-S141")`. Construction sites that bypass the constructor still must produce non-empty motive_sources; the assertion catches mistakes in test builds.

### 6. Refactor `motive_score` body

At `crates/worldwake-ai/src/ranking.rs:1007`:

```rust
fn motive_score(candidate: &GoalOffer, context: &RankingContext<'_>) -> u32 {
    candidate
        .motive_sources
        .iter()
        .map(|src| score_motive_source(src, context))
        .sum()
}

fn score_motive_source(src: &MotiveSourceRef, context: &RankingContext<'_>) -> u32 {
    match &src.source {
        MotiveSource::NeedPressure { need } => {
            let pressure = context
                .needs
                .map(|n| need_pressure_for_id(n, *need))
                .unwrap_or(Permille::zero());
            let weight = utility_weight_for_need(context.utility, *need);
            score_from_pressure_and_weight(pressure, weight)
        }
        MotiveSource::Pain { wound } => score_pain_from_wound(context, *wound, context.utility.pain_weight),
        MotiveSource::OfficeDuty { office } => score_office_duty(context, *office, context.utility.office_duty_weight),
        MotiveSource::Loyalty { other } => score_loyalty(context, *other, context.utility.loyalty_weight),
        MotiveSource::Greed { opportunity } => score_greed(context, opportunity, context.utility.greed_weight),
        MotiveSource::Shame { reputation_record } => score_shame(context, *reputation_record, context.utility.shame_weight),
        MotiveSource::Revenge { violation } => score_revenge(context, *violation, context.utility.revenge_weight),
    }
}
```

Each per-variant helper (`score_need_pressure`, `score_pain_from_wound`, `score_office_duty`, `score_loyalty`, `score_greed`, `score_shame`, `score_revenge`) extracts the corresponding fragment of today's `motive_score` body. The exact split must reproduce today's `u32` arithmetic per-`GoalKind`. Reuse existing helpers where they already extract per-kind logic (e.g., `need_pressure_for_id` at `ranking.rs:1322`, `utility_weight_for_need` at `ranking.rs:1292`).

### 7. Populate `RankedGoalSummary.motive_source_contributions`

Wherever `motive_score` is called during ranking and the result is stored in `RankedGoalSummary`, capture each `(MotiveSourceRef, u32)` contribution alongside the sum. Extend `score_motive_source` (or wrap its callers) to return both the sum and the per-source breakdown; pipe the breakdown into the existing `RankedGoalSummary` construction sites.

### 8. Remove the old `motive_score` match-on-GoalKind body

Per FND-28, the old `match candidate.key.goal_kind { ... }` body is removed entirely; no fallback path remains. Helpers that were called only by the old match arms (`drive_score`, `enterprise_score`, `raid_target_motive`, etc.) are either renamed/reused as per-`MotiveSource`-variant helpers or removed if their logic is fully absorbed.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — `GoalOffer` field, construction sites)
- `crates/worldwake-ai/src/motive_source_mapping.rs` (new — mapping helper module)
- `crates/worldwake-ai/src/lib.rs` (modify — `pub mod motive_source_mapping;`)
- `crates/worldwake-ai/src/ranking.rs` (modify — `motive_score` body refactor, new per-variant helpers, remove old match arms)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — populate `motive_sources` at the 3 helper sites)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — construction sites)
- `crates/worldwake-ai/src/search/strategic.rs` (modify — construction sites)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — construction sites)
- `crates/worldwake-ai/src/source_composite.rs` (modify — construction sites)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — wire `motive_source_contributions` population alongside `motive_score`)
- Any test fixtures that build `GoalOffer { … }` literally — confirm with `rg -n "GoalOffer\s*\{" crates/worldwake-ai/` during reassessment.

## Out of Scope

- The 5 new `UtilityProfile` weight fields — owned by `archive/tickets/S141MOTSOULED-002.md`.
- `MotiveSource` / `MotiveSourceRef` type definitions — owned by `archive/tickets/S141MOTSOULED-001.md`.
- `RankedGoalSummary.motive_source_contributions` field declaration — owned by `archive/tickets/S141MOTSOULED-003.md` (this ticket only populates it).
- `GoalCommittedPayload.decisive_motive_sources` — owned by 005.
- Observer rendering of motive sources — owned by 006.
- New goldens for motive-source behavior — owned by 007.
- The 5 deferred `MotiveSource` variants (`Fear`, `Obligation`, `Debt`, `Habit`, `Curiosity`) — Phase 12 follow-ups per spec's Deferred Variants table; never reach `score_motive_source` because they don't exist in the live enum.

## Acceptance Criteria

### Tests That Must Pass

1. **Score parity gate**: every existing 1440-tick survival golden in `crates/worldwake-ai/tests/golden_survival_*.rs` passes without modification. The decision-event payloads they assert against carry identical `motive_score` values per commit (bitwise) pre/post-this-ticket.
2. All 15 existing `ranking.rs` `#[cfg(test)]` tests named in Assumption Reassessment item 2 continue to pass without modification.
3. Per-variant focused tests: 7 unit tests in `ranking.rs#[cfg(test)]`, one per `MotiveSource` variant, asserting the helper produces a known value for a constructed input.
4. Mapping focused tests: in `motive_source_mapping.rs#[cfg(test)]`, one test per active `GoalKind` variant asserting `derive_default_motive_sources` returns the expected variant set.
5. `debug_assert!(!motive_sources.is_empty())` fires on a synthetic construction site that intentionally passes an empty vec — covered by a `#[should_panic]` test.
6. Existing suite: `cargo test --workspace`

### Invariants

1. **Score parity**: for every commit produced by every existing 1440-tick survival golden, `motive_score(offer)` post-refactor equals `motive_score(offer)` pre-refactor bitwise. Any drift is a bug, never a test relaxation.
2. **Required non-empty `motive_sources`**: every `GoalOffer` constructed at any of the 13 workspace-wide sites carries at least one `MotiveSourceRef`. Test builds enforce this via `debug_assert`; release builds rely on the conformance test in 007.
3. **Single comparator**: `compare_ranked_goals_is_the_only_impl_in_crate` (`ranking.rs:6144`) continues to pass — only `compare_ranked_goals` is defined in `ranking.rs`, and the per-variant helpers it transitively calls are file-private to `ranking.rs`.
4. **No backward-compat fallback**: the old `match candidate.key.goal_kind { ... }` body is removed; no `_ =>` arm or environment-flag-gated alternative remains (FND-28).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs#[cfg(test)]` — 7 per-variant scoring tests + 1 empty-motive-sources `#[should_panic]` test.
2. `crates/worldwake-ai/src/motive_source_mapping.rs#[cfg(test)]` — per-`GoalKind` mapping correctness tests covering every active variant.
3. The 15 named existing `ranking.rs` tests do NOT change — they validate score parity at the comparator level. Per `docs/precision-rules.md` Rule 3 (coverage-gap classification), this is the existing focused/unit coverage; the score-parity gate is the existing golden/E2E coverage.

### Commands

1. `cargo test -p worldwake-ai motive_source_mapping`
2. `cargo test -p worldwake-ai ranking`
3. `cargo test -p worldwake-ai --tests` (full crate sweep including all `tests/golden_*.rs` files — proves the score-parity gate)
4. `cargo test --workspace` (full workspace verification)
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `./scripts/verify.sh` before push (fmt + workspace tests + clippy at the level CI enforces)
