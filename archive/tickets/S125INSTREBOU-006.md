# S125INSTREBOU-006: emit_bounty_posting_candidates funding-aware emission

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AI candidate emitter consumes belief-view accessor
**Deps**: [S125INSTREBOU-004](S125INSTREBOU-004.md)

## Problem

`emit_bounty_posting_candidates` (`crates/worldwake-ai/src/candidate_generation.rs:765-878`) hard-codes `RewardSource::InstitutionalTreasury { treasury_entity: office }` at lines 867-868 without consulting fund availability. This bypasses FND-7/FND-14: the AI emits a candidate whose authoritative validation may immediately reject (no funds), wasting a planner cycle and producing a misleading decision trace. S125 Deliverable D4 requires the emitter to consult the belief-view accessor (delivered in ticket 004) and skip emission when no lawful funded source exists.

## Assumption Reassessment (2026-04-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `emit_bounty_posting_candidates` at `crates/worldwake-ai/src/candidate_generation.rs:765-878`. Reads `bounty_posting_weight` (line 771), consults `ctx.view.known_institutional_beliefs(ctx.agent)`, filters office holders with JurisdictionalAuthority. The hard-coded reward source is at lines 867-868. Existing tests for PostBounty motive: `post_bounty_goal_has_non_zero_motive_for_live_accusation_case` (`crates/worldwake-ai/src/ranking.rs:3633`), `post_bounty_goal_applies_obligation_satiation_decay` (`ranking.rs:3722`), `post_bounty_goal_is_zero_motive_when_bounty_weight_is_zero` (`ranking.rs:3817`). No focused test for the candidate emitter itself; ranking tests exercise upstream gating, not the reward-source decision.
2. S125 §5 (AI Candidate Generation) specifies the emitter must call `ctx.view.actor_lawful_reward_source_for_case(...)` and skip emission when it returns `None`. Live `GoalKind` under test: `PostBounty` (already exists; no GoalKind variant addition).
3. Shared abstraction boundary: `GenerationContext` exposes `view: &dyn GoalBeliefView` (verify the exact field name during implementation). The accessor introduced in ticket 004 is callable through this surface.
4. Live planner surface: candidate emitter for `GoalKind::PostBounty`, ranked via `post_bounty_motive` at `ranking.rs:1380`. No GoalKind variant or operator surface change.
5. Adjacent contradictions: none. Ranking continues to operate on the emitted candidates unchanged; suppression of empty cases moves from "emit then reject at validation" to "skip at emission" — strictly less work and a cleaner decision trace.

## Architecture Check

1. The emitter becomes a pure consumer of the belief-view accessor: no duplicate world-state reads, no duplicate authoritative validation logic. This keeps the AI crate's read path lawful (FND-7, FND-14) and authoritative validation centralized in `worldwake-systems` (FND-26).
2. No backward compat: hard-coded source is replaced, not aliased.

## Verification Layers

1. Candidate absence when accessor returns `None` → decision-trace assertion that no `PostBounty` candidate is emitted for the relevant accusation/office pair (preferred per `docs/precision-rules.md` Rule 6: decision-trace over indirect evidence).
2. Candidate emission with the accessor's returned reward source when `Some` → decision-trace assertion + focused unit coverage on the emission path.
3. Existing tests continue to pass → confirms ranking and motive math are not regressed by the emitter change.

## What to Change

### 1. Replace hard-coded reward source

In `emit_bounty_posting_candidates` (lines 867-868), replace the literal `RewardSource::InstitutionalTreasury { treasury_entity: office }` with a call to `ctx.view.actor_lawful_reward_source_for_case(actor, &accusation_case)` (verify the precise parameter shape against ticket 004's delivered signature).

### 2. Skip emission on `None`

When the accessor returns `None`, do not emit a candidate for that accusation/office pair. Record the skip via the existing diagnostics surface (`CandidateGenerationDiagnostics`) so decision traces can explain the absence.

### 3. Use accessor result

When the accessor returns `Some(reward_source)`, use that value when constructing the candidate's `BountyTerms.reward_source`.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (add trace reason for funded bounty omission)
- `crates/worldwake-ai/src/planner_ops.rs` (test inventory fallout: include already-live `WithdrawBounty` in the count)
- `archive/specs/S125-institutional-treasuries-and-bounty-funding.md` (mark D4 done)

## Out of Scope

- Belief-view accessor implementation — ticket 004.
- Authoritative validation re-check at start/commit — ticket 005.
- Ranking changes — `post_bounty_motive` continues to operate on the emitted candidate set unchanged.
- Stale-balance memory for non-co-located holders — S125 OQ3, deferred.

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: `emit_bounty_posting_candidates_skips_when_accessor_returns_none`.
2. New focused test: `emit_bounty_posting_candidates_uses_accessor_returned_reward_source`.
3. Existing tests must continue to pass: `post_bounty_goal_has_non_zero_motive_for_live_accusation_case`, `post_bounty_goal_applies_obligation_satiation_decay`, `post_bounty_goal_is_zero_motive_when_bounty_weight_is_zero`, `fulfill_post_bounty_search_finds_travel_then_post_bounty_progress_barrier` (`crates/worldwake-ai/src/search/tests.rs:13069`).
4. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. Emitter performs no direct world-state read for funded-ness (FND-7/FND-14): the accessor is the only fund-availability path the emitter consults.
2. Decision trace records the accessor outcome (Some/None) so post-hoc analysis can explain candidate presence or absence.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (existing `#[cfg(test)]` block — verify location during implementation) — two new emitter tests.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `scripts/verify.sh`

## Outcome

Completed on 2026-04-25.

- `emit_bounty_posting_candidates` now calls `ctx.view.actor_lawful_reward_source_for_case(ctx.agent, &belief)` before emitting `GoalKind::PostBounty`.
- When the accessor returns `None`, the emitter skips the candidate and records an omitted `PostBounty` trace as `PoliticalGoalFamily::PostBounty` / `PoliticalCandidateOmissionReason::NoLawfulRewardSource`.
- When the accessor returns `Some(reward_source)`, the emitted `BountyTerms.reward_source` uses that returned value instead of constructing a hard-coded institutional treasury source in the emitter.
- Existing positive candidate-generation fixture setup now seeds local controlled coin so it proves the funded accessor path instead of relying on the old hard-coded source.
- S125 Deliverable D4 is marked done in the active spec.

## Deviations

- The drafted `Files to Touch` only named `candidate_generation.rs`; implementation also touched `decision_trace.rs` so the accessor-`None` outcome reaches the public candidate trace, and `planner_ops.rs` because `cargo test -p worldwake-ai` exposed a stale exact planner-op inventory count after the already-live `WithdrawBounty` operator.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::emit_bounty_posting_candidates_skips_when_accessor_returns_none -- --exact`.
- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::emit_bounty_posting_candidates_uses_accessor_returned_reward_source -- --exact`.
- Passed `cargo test -p worldwake-ai --lib ranking::tests::post_bounty_goal_has_non_zero_motive_for_live_accusation_case -- --exact`.
- Passed `cargo test -p worldwake-ai --lib ranking::tests::post_bounty_goal_applies_obligation_satiation_decay -- --exact`.
- Passed `cargo test -p worldwake-ai --lib ranking::tests::post_bounty_goal_is_zero_motive_when_bounty_weight_is_zero -- --exact`.
- Passed `cargo test -p worldwake-ai --lib search::tests::fulfill_post_bounty_search_finds_travel_then_post_bounty_progress_barrier -- --exact`.
- Passed `cargo test -p worldwake-ai --lib planner_ops::tests::planner_op_kind_covers_exactly_current_phase_two_families -- --exact`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh` (live script gates: `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p worldwake-cli --bin scenario-coverage -- --check`).
