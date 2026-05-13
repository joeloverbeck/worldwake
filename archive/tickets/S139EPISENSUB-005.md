# S139EPISENSUB-005: Ranking integration for GoalKind::AskWitness

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` (`ranking.rs`)
**Deps**: archive/tickets/S139EPISENSUB-001.md, archive/tickets/S139EPISENSUB-002.md, archive/tickets/S139EPISENSUB-003.md

## Problem

Before this ticket, ticket 001 had left placeholder priority (`GoalPriorityClass::Background`) and `motive_score` (`0`) arms for `GoalKind::AskWitness` in `ranking.rs` to keep the workspace compiling. This ticket replaced those placeholders with the ranking contract: priority class `Low`, and a `motive_score` formula combining (a) the confidence gap below `stale_evidence_barrier_threshold`, (b) a recency bonus weighted by `witness_recency_preference` (added in ticket 002), and (c) `LearnedOpportunityMemory` dampening for repeated fruitless asks.

## Assumption Reassessment (2026-05-13)

1. `GoalPriorityClass` in `crates/worldwake-ai/src/ranking.rs` already had an exhaustive `GoalKind` match. The owned change was the `AskWitness` arm only: `Background` became `Low` because epistemic detours rank below productive economic goals but above pure background polling.
2. `motive_score` computation flows through `rank_candidates_with_memories` into `score_goal_kind_motive`. The existing `RankingContext` already carries `LearnedOpportunityMemory`, so the dampening path did not require a new memory type or a new belief-view accessor.
3. Shared abstraction boundary under audit: the priority-class match and `score_goal_kind_motive` match in `ranking.rs`. Both are exhaustive matches over `GoalKind`.
4. Live `GoalKind` under test: `GoalKind::AskWitness` (added by ticket 001). The structural analog for branch comparison is `GoalKind::ShareBelief`, which shares social weighting but scores known-confidence pressure rather than a threshold gap.
5. `EpistemicDispositionProfile.witness_recency_preference` is read through `context.view.epistemic_disposition_profile(context.agent)`. The implementation uses integer `Permille` arithmetic plus tick deltas; no floats or wall-clock time were introduced.
6. The drafted accessors `entity_belief_confidence`, `entity_belief_last_observed_tick`, `Permille::from_ratio_clamped`, and `LearnedOpportunityMemory::damping_for` were absent in the live code. The landed implementation uses `GoalBeliefView::entity_beliefs_sourced_from_witness`, `GoalBeliefView::known_entity_beliefs`, `belief_confidence`, and the existing `LearnedOpportunityMemory.opportunities` map.
7. Ranking-sensitive branch symmetry was tested directly: `AskWitness` and `ShareBelief` diverge for the same topic pressure because `AskWitness` scores the threshold gap while `ShareBelief` scores known-confidence pressure.

## Architecture Check

1. The `motive_score` formula respects determinism: all scaling stays in `Permille` and integer tick deltas. The recency bonus is computed from `current_tick - last_observed_tick`, normalized over `ASK_WITNESS_STALENESS_NORMALIZATION_TICKS`, and scaled by `witness_recency_preference`.
2. Dampening through `LearnedOpportunityMemory` reuses the existing ranking context parameter. No new memory type, no parallel damping state.
3. The two staged placeholder markers placed by ticket 001 were removed from `ranking.rs`.

## Verification Layers

1. `ask_witness_priority_class_is_low` asserts `GoalKind::AskWitness { .. }` ranks as `GoalPriorityClass::Low`.
2. `ask_witness_motive_score_rises_with_confidence_gap` compares high-gap and low-gap report fixtures and asserts the higher gap ranks first.
3. `ask_witness_motive_score_is_damped_by_learned_opportunity_memory` injects an unexpired `LearnedOpportunityMemory` entry for the same opportunity and asserts the damped score is strictly lower.
4. `ask_witness_and_share_belief_scores_diverge_for_same_topic_pressure` compares `AskWitness` and `ShareBelief` under the same topic pressure and locks the expected formula divergence.
5. Authoritative ranking ordering → action trace surface is not relevant here; the contract is decision-trace at the ranking layer.

## What Changed

### 1. Replace placeholder priority class arm

In `crates/worldwake-ai/src/ranking.rs`, the `GoalKind::AskWitness { .. }` priority-class arm now returns `GoalPriorityClass::Low`.

### 2. Replace placeholder motive_score arm

In `score_goal_kind_motive`, `GoalKind::AskWitness { witness, topic }` now delegates to `ask_witness_motive`. That helper supports `TellTopic::EntityBelief` and returns zero for unsupported topics. It computes witness-sourced confidence via `entity_beliefs_sourced_from_witness`, computes a gap below `stale_evidence_barrier_threshold`, adds staleness signal scaled by `witness_recency_preference`, applies social utility weighting, and applies AskWitness-specific dampening for unexpired learned-opportunity entries.

### 3. Remove the two TODO markers placed by ticket 001

`TODO(S139EPISENSUB-005)` has zero matches in `ranking.rs`.

## Files to Touch

- `crates/worldwake-ai/src/ranking.rs` (modify — replace 2 placeholder arms + add module-private formula constants)

## Out of Scope

- Calibration of `ASK_WITNESS_GAP_WEIGHT` / `ASK_WITNESS_STALENESS_NORMALIZATION_TICKS` against multi-scenario goldens — initial values chosen by analog to the existing social-weight scale; calibration remains a follow-up if ticket 006 surfaces imbalance.
- Cross-witness topic-disagreement scoring — deferred until `TellTopic::SocialObservation` / `InstitutionalClaim` variants are supported.
- Goldens — ticket 006.

## Acceptance Criteria

### Tests That Must Pass

1. Focused unit test `ask_witness_priority_class_is_low` asserts `GoalKind::AskWitness { .. }` maps to `GoalPriorityClass::Low`.
2. Focused unit test `ask_witness_motive_score_rises_with_confidence_gap` asserts `AskWitness` scoring rises with the confidence gap below threshold.
3. Focused unit test `ask_witness_motive_score_is_damped_by_learned_opportunity_memory` asserts learned-opportunity dampening strictly reduces the score for a repeated ask.
4. Focused unit test `ask_witness_and_share_belief_scores_diverge_for_same_topic_pressure` documents that `AskWitness` and `ShareBelief` formulas diverge under identical topic pressure.
5. Existing suite: `cargo test -p worldwake-ai` passed.
6. Grep for the S139EPISENSUB-005 placeholder marker in `ranking.rs` returned zero matches.

### Invariants

1. `motive_score` for `GoalKind::AskWitness` is expressible in `Permille` arithmetic — no floats, no wall-clock time (CLAUDE.md Critical Invariants).
2. `motive_score` is monotonically non-decreasing in confidence-gap (holding all other inputs constant) — verified by focused unit test.
3. `LearnedOpportunityMemory` damping is the single damping path for repeated asks — no parallel damping state introduced.

## Verification Plan

### Focused Tests

1. `crates/worldwake-ai/src/ranking.rs` includes four focused AskWitness ranking tests listed in Acceptance Criteria.

### Commands

1. `cargo test -p worldwake-ai --lib ranking::tests::ask_witness_` — targeted ranking test run.
2. `cargo test -p worldwake-ai`.
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
4. `./scripts/verify.sh` — full pre-PR gate.

## Outcome

`GoalKind::AskWitness` now has production ranking behavior in `crates/worldwake-ai/src/ranking.rs`. Priority class is `Low`; motive scoring uses witness-sourced belief confidence, threshold gap, staleness/recency preference, social utility, and existing learned-opportunity memory.

## Deviations

The draft expected new `GoalBeliefView` confidence/tick helpers and a `Permille::from_ratio_clamped` helper. Live code already exposed enough substrate through `entity_beliefs_sourced_from_witness`, `known_entity_beliefs`, and `belief_confidence`, so no `worldwake-sim` or `worldwake-core` files were changed.

The draft referenced a `LearnedOpportunityMemory::damping_for` API. Live code stores opportunities directly in `LearnedOpportunityMemory.opportunities`; `AskWitness` applies its family-specific dampening by checking an unexpired matching `OpportunityKey` in the existing ranking context.

## Verification Result

1. Passed `cargo test -p worldwake-ai --lib ask_witness_ -- --list`.
2. Passed `cargo test -p worldwake-ai --lib ranking::tests::ask_witness_`.
3. Passed `rg -n 'TODO\(S139EPISENSUB-005\)' crates/worldwake-ai/src/ranking.rs` as a zero-match check.
4. Passed `cargo test -p worldwake-ai`.
5. Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
6. Passed `./scripts/verify.sh`.
