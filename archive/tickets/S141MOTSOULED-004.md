# S141MOTSOULED-004: `GoalOffer.motive_sources` + `motive_score` body refactor + mapping helper

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `GoalOffer` extension, `motive_score` body refactor, new mapping helper module, populates `RankedGoalSummary.motive_source_contributions`
**Deps**: `archive/tickets/S141MOTSOULED-001.md` (uses `MotiveSource`, `MotiveSourceRef`), `archive/tickets/S141MOTSOULED-002.md` (reads new `UtilityProfile` weights), `archive/tickets/S141MOTSOULED-003.md` (declares `motive_source_contributions`)

## Problem

This is the S141 critical-path "switchover" ticket. Two coupled deliverables landed together because splitting them would create a transient "carrier with no consumer" state in a live ranking-authority path (FND-28-driven combining per `tickets/_TEMPLATE.md` review guidance):

- **D2**: Added `motive_sources: Vec<MotiveSourceRef>` to `GoalOffer`; production candidate helpers and the current-plan reinstatement helper populate it through `derive_default_motive_sources(goal_kind, anchor, introduced_tick)`.
- **D3**: Refactored `motive_score` (`crates/worldwake-ai/src/ranking.rs:1007`) to iterate `candidate.motive_sources` through `score_motive_source`. The landed dispatch preserves exact prior score arithmetic by delegating each current source variant to the extracted `score_goal_kind_motive` body; richer independent per-source arithmetic remains owned by 007.

The acceptance gate was **score parity**: existing `worldwake-ai` golden and ranking suites passed unchanged after the refactor. The full non-empty production conformance and multi-source behavioral proof remain owned by 007.

## Assumption Reassessment (2026-05-12)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `GoalOffer` lives in `worldwake-ai` at `crates/worldwake-ai/src/goal_model.rs:2038` and derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` (per S141 reassessment). Live reassessment found the production agenda-emitter path is centralized in the three `crates/worldwake-ai/src/candidate_generation.rs` construction helpers, plus one reinstatement helper in `agent_tick/observation.rs`; those production-style helpers now populate `motive_sources` via `derive_default_motive_sources`. Many additional explicit `GoalOffer { ... }` literals exist in focused test and synthetic helper code; this ticket makes those compile with explicit fixture vectors, while 007 remains the enforcement owner for whole-planner non-empty conformance.
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
3. Shared abstraction boundary: the `GoalKind -> MotiveSource` mapping (in `derive_default_motive_sources`) and the `MotiveSource -> score` dispatch now preserve the existing per-`GoalKind` score arithmetic. The landed scoring dispatch is intentionally parity-preserving: each source dispatches through the extracted `score_goal_kind_motive` body so the existing score contract is unchanged while the carrier path becomes first-class. Richer independent per-source arithmetic remains a 007/golden-conformance concern.
4. The `derive_default_motive_sources` mapping is the load-bearing structural change. It maps core need goals to `NeedPressure`, office support to `OfficeDuty`/`Loyalty`, violation goals to `Revenge`, and other active goals to opportunity-backed `Greed`. This is a default source ledger for current emitters, not a final proof that every future/multi-source behavior has independent contribution arithmetic.
5. `GoalOffer::assert_motive_sources_present()` exists for debug/test enforcement at explicit validation points. Production candidate-generation helpers populate the field. Synthetic tests and fixture helpers may still use `Vec::new()` and are covered by a `#[cfg(test)]` parity fallback in `ranking.rs`; 007 owns the full conformance test that makes omitted motive sources fail at the assembly boundary.
6. Ranking-sensitive precision (per `docs/precision-rules.md` Rule 5): the divergence driver for `motive_score` is the per-`MotiveSource` weight × strength product. After refactor, two agents with identical world state but different per-`MotiveSource`-class weights on `UtilityProfile` (from `archive/tickets/S141MOTSOULED-002.md`) must produce different `motive_score` values — this is FND-22 diversity. Verify against `crime_goals_use_profile_driven_motive_scores` (line 1388), which already asserts this for crime goals; extend coverage to the new motive classes in 007.

## Architecture Check

1. The scoring refactor is parity-preserving: `motive_score` now iterates `candidate.motive_sources` and dispatches via `score_motive_source`, while the extracted `score_goal_kind_motive` body preserves today's exact arithmetic. This satisfies the first S141 carrier switchover without changing ranking behavior.
2. Co-locating D2 and D3 in one ticket avoids the FND-28 transient state where `motive_sources` exists as a populated but unread carrier. Score parity is testable in one shot.
3. The new `motive_source_mapping.rs` module is the single authoritative `GoalKind → MotiveSource` mapping. The 3 helper sites in `candidate_generation.rs` (lines 554, 4808, 5420) call it; per-emitter overrides are permitted only when the emitter has richer context (e.g., a recorded-violation emitter knows the exact `ViolationId`). FND-26 (systems interact through state) is preserved — the mapping reads `GoalKind` + `OpportunityAnchor` from the offer, no cross-system calls.
4. `compare_ranked_goals` (file-private per S123) is unchanged in identity; only the body of its callee `motive_score` is partitioned. The "one comparator" invariant tested at `ranking.rs:6144` (`compare_ranked_goals_is_the_only_impl_in_crate`) remains valid.

## Verification Layers

1. Score parity: `cargo test -p worldwake-ai --tests` and `cargo test --workspace` pass with the existing golden suite unchanged.
2. Dispatch correctness: `cargo test -p worldwake-ai --lib ranking` passes with `motive_score` iterating `motive_sources` and preserving the old score arithmetic through `score_goal_kind_motive`.
3. Mapping correctness: `cargo test -p worldwake-ai --lib motive_source_mapping` covers the current default mappings for core needs, social/violation goals, and fallback opportunity-backed greed.
4. Trace population: `cargo test -p worldwake-ai --lib agent_tick::planning::tests::summarize_ranked_goal_populates_motive_source_contributions -- --exact` covers summary contribution population.
5. Empty-vec invariant: this ticket exposes `GoalOffer::assert_motive_sources_present()` and keeps synthetic empty fixture vectors out of production candidate generation. The full workspace conformance check is left to 007.
6. Backward compatibility: no persisted compatibility path was added. The only compatibility relief is a `#[cfg(test)]` fixture fallback in `ranking.rs` so the existing large synthetic test surface can keep asserting ranking parity.

## Landed Changes

### 1. `GoalOffer` struct extension

At `crates/worldwake-ai/src/goal_model.rs:2038`, `GoalOffer` now carries:

```rust
pub struct GoalOffer {
    // existing fields preserved
    pub motive_sources: Vec<MotiveSourceRef>,
}
```

`GoalOffer::assert_motive_sources_present()` provides the debug assertion helper for validation points. Production helpers populate the field; synthetic focused-test fixtures may use explicit empty vectors until 007 installs whole-planner conformance.

### 2. New module `crates/worldwake-ai/src/motive_source_mapping.rs`

```rust
use worldwake_core::motive_source::{MotiveSource, MotiveSourceRef};

pub fn derive_default_motive_sources(
    goal_kind: &GoalKind,
    anchor: &OpportunityAnchor,
    introduced_tick: Tick,
) -> Vec<MotiveSourceRef> { /* per-GoalKind default mapping */ }
```

The mapping returns one default `MotiveSourceRef` for the current production offer surface: core need goals map to `NeedPressure`, office/social/violation goals map to their specific current variants, and remaining active goals map to opportunity-backed `Greed`.

### 3. Populated `motive_sources` at production-style helper sites

The three `crates/worldwake-ai/src/candidate_generation.rs` helper sites now attach `motive_sources: derive_default_motive_sources(...)` to emitted `GoalOffer`s. `agent_tick/observation.rs` also derives sources when reinstating a current-plan candidate.

### 4. Made explicit fixture fallout compile

The live repo had many more explicit `GoalOffer { ... }` literals than the drafted 13-site count. Focused tests and synthetic helper literals now enumerate `motive_sources` explicitly, usually as `Vec::new()` fixture scaffolding where the test is not proving source population.

### 5. Added debug assertion helper

`GoalOffer::assert_motive_sources_present()` calls `debug_assert!(!motive_sources.is_empty(), "GoalOffer.motive_sources must be non-empty post-S141")`. Constructor-wide enforcement is deferred to 007's conformance/golden pass so this ticket can preserve the existing broad synthetic ranking test surface.

### 6. Refactored `motive_score` body

At `crates/worldwake-ai/src/ranking.rs:1007`, `motive_score` now chooses a source slice and sums `score_motive_source` over it:

```rust
fn motive_score(candidate: &GoalOffer, context: &RankingContext<'_>) -> u32 {
    // test-only fixture fallback omitted here for brevity
    candidate
        .motive_sources
        .iter()
        .map(|src| score_motive_source(src, candidate, context))
        .sum()
}

fn score_motive_source(
    src: &MotiveSourceRef,
    candidate: &GoalOffer,
    context: &RankingContext<'_>,
) -> u32 {
    match &src.source {
        MotiveSource::NeedPressure { .. }
        | MotiveSource::Pain { .. }
        | MotiveSource::OfficeDuty { .. }
        | MotiveSource::Loyalty { .. }
        | MotiveSource::Greed { .. }
        | MotiveSource::Shame { .. }
        | MotiveSource::Revenge { .. } => score_goal_kind_motive(candidate, context),
    }
}
```

The old `GoalKind` match body was extracted into `score_goal_kind_motive` to preserve score parity. 007 owns the later proof and implementation work for independent per-source arithmetic.

### 7. Populated `RankedGoalSummary.motive_source_contributions`

`agent_tick::planning::summarize_ranked_goal` now projects the ranked offer's motive sources into `RankedGoalSummary.motive_source_contributions`. Because the current default mapping emits one source per production offer, the first source receives the aggregate motive score and later sources, if any, receive zero until 007 lands richer decomposition.

### 8. Kept no separate live fallback path

There is no persisted compatibility path and no alternate production ranking mode. The only relief path is a `#[cfg(test)]` fallback inside `motive_score` that derives default sources for legacy synthetic fixtures with empty `motive_sources`.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — `GoalOffer` field, construction sites)
- `crates/worldwake-ai/src/motive_source_mapping.rs` (new — mapping helper module)
- `crates/worldwake-ai/src/lib.rs` (modify — `pub mod motive_source_mapping;`)
- `crates/worldwake-ai/src/ranking.rs` (modify — `motive_score` body refactor, parity-preserving `score_motive_source`, extracted `score_goal_kind_motive`)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — populate `motive_sources` at the 3 helper sites)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — construction sites)
- `crates/worldwake-ai/src/search/strategic.rs` (modify — construction sites)
- `crates/worldwake-ai/src/plan_selection.rs` (modify — construction sites)
- `crates/worldwake-ai/src/source_composite.rs` (modify — construction sites)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — reinstate helper derives motive sources)
- Any test fixtures that build `GoalOffer { … }` literally — confirm with `rg -n "GoalOffer\s*\{" crates/worldwake-ai/` during reassessment.

## Out of Scope

- The 5 new `UtilityProfile` weight fields — owned by `archive/tickets/S141MOTSOULED-002.md`.
- `MotiveSource` / `MotiveSourceRef` type definitions — owned by `archive/tickets/S141MOTSOULED-001.md`.
- `RankedGoalSummary.motive_source_contributions` field declaration — owned by `archive/tickets/S141MOTSOULED-003.md` (this ticket only populates it).
- `GoalCommittedPayload.decisive_motive_sources` — owned by 005.
- Observer rendering of motive sources — owned by `archive/tickets/S141MOTSOULED-006.md`.
- New goldens for motive-source behavior — owned by 007.
- The 5 deferred `MotiveSource` variants (`Fear`, `Obligation`, `Debt`, `Habit`, `Curiosity`) — Phase 12 follow-ups per spec's Deferred Variants table; never reach `score_motive_source` because they don't exist in the live enum.

## Acceptance Criteria

### Tests That Passed

1. **Score parity gate**: `cargo test -p worldwake-ai --tests` passed with the existing golden suite unchanged.
2. Existing ranking behavior: `cargo test -p worldwake-ai --lib ranking` passed.
3. Mapping focused coverage: `cargo test -p worldwake-ai --lib motive_source_mapping` passed.
4. Trace focused coverage: `cargo test -p worldwake-ai --lib agent_tick::planning::tests::summarize_ranked_goal_populates_motive_source_contributions -- --exact` passed.
5. Existing suite: `cargo test --workspace` passed.

### Invariants

1. **Score parity**: the refactor preserves existing ranking arithmetic by routing each motive source through the extracted `score_goal_kind_motive` body.
2. **Production source population**: the central candidate-generation helpers and reinstatement helper populate `motive_sources`; synthetic fixture literals remain explicit test fixtures.
3. **Single comparator**: `compare_ranked_goals_is_the_only_impl_in_crate` continues to pass through the `ranking` test target.
4. **No persisted compatibility shim**: no save or runtime alias path was added. The only relief path is `#[cfg(test)]` fixture derivation inside `motive_score`.

## Test Plan

### Focused Coverage

1. `crates/worldwake-ai/src/motive_source_mapping.rs#[cfg(test)]` covers default source derivation for need, social/violation, and fallback greed mappings.
2. `crates/worldwake-ai/src/agent_tick/planning.rs#[cfg(test)]` covers `RankedGoalSummary.motive_source_contributions` population.
3. Existing `ranking.rs` tests validate score parity at the comparator level.

### Commands

1. `cargo test -p worldwake-ai --lib motive_source_mapping`
2. `cargo test -p worldwake-ai --lib ranking`
3. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::summarize_ranked_goal_populates_motive_source_contributions -- --exact`
4. `cargo test -p worldwake-ai --tests`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `python3 .codex/skills/implement-ticket/scripts/check_closeout.py archive/tickets/S141MOTSOULED-004.md`

## Outcome

Implemented the first S141 motive-source switchover seam:

1. Added `GoalOffer.motive_sources` and `GoalOffer::assert_motive_sources_present()`.
2. Added `worldwake_ai::motive_source_mapping::derive_default_motive_sources`.
3. Populated production candidate-generation helpers and the current-plan reinstatement helper with derived motive sources.
4. Refactored `motive_score` to iterate motive sources through `score_motive_source`, preserving prior score arithmetic through `score_goal_kind_motive`.
5. Populated `RankedGoalSummary.motive_source_contributions` for ranked summaries.

## Deviations

1. The live repo had many more explicit `GoalOffer` literals than the stale 13-site reassessment claimed. Production-style emitter paths now derive motive sources; synthetic fixtures use explicit `Vec::new()` where they are only scaffolding for focused tests.
2. Per-source scoring is a parity-preserving dispatch layer, not final independent per-motive arithmetic. 007/golden conformance remains responsible for richer multi-source behavioral proof.
3. Empty-source enforcement is not a constructor-wide panic. The landed enforcement is an assertion helper plus production helper population; 007 owns whole-planner conformance.
4. `./scripts/verify.sh` was not run in this ticket pass; the final proof used the narrow and broad commands listed below.

## Verification Result

1. Passed `cargo test -p worldwake-ai --lib motive_source_mapping`
2. Passed `cargo test -p worldwake-ai --lib ranking`
3. Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::summarize_ranked_goal_populates_motive_source_contributions -- --exact`
4. Passed `cargo test -p worldwake-ai --tests`
5. Passed `cargo test --workspace`
6. Passed `cargo clippy --workspace --all-targets -- -D warnings`
7. Passed `python3 .codex/skills/implement-ticket/scripts/check_closeout.py archive/tickets/S141MOTSOULED-004.md`
