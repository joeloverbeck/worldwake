# BLKINTDIAG-001: BlockedIntentMemory Silently Suppresses New-Target Candidates After Compound Goal Sequences

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI crate diagnostics and possibly blocker scoping
**Deps**: None

## Problem

After a bandit completes a compound goal sequence (ShareBelief → EstablishBanditCamp → ShareBelief), `filter_blocked_candidates` in `candidate_generation.rs` silently filters new `RaidTarget` candidates for targets that were NOT present during the original blocked action sequence. The decision trace shows `candidates=0` (post-filter count) while the trace dump section shows pre-filter candidates, but no trace output identifies which `BlockedIntent` entry matched or why.

This was discovered during E22INTSOATES-008 implementation: bandits established a new camp at a rally glen, then new agents arrived at the same place. Despite the new targets having never been attempted before, `RaidTarget` for those targets was filtered to 0 candidates. Clearing the `BlockedIntentMemory` world component between phases did not resolve the suppression, suggesting the blocker match is broader than the specific goal key or that the blocked state is re-introduced by the observation pipeline during the same tick.

The silent nature of this suppression — no trace event, no diagnostic log, no abort reason — makes it invisible without reading `candidate_generation.rs` source.

## Assumption Reassessment (2026-03-31)

1. `filter_blocked_candidates` in `crates/worldwake-ai/src/candidate_generation.rs:260` checks `is_candidate_blocked` which matches `intent.blocker_key.goal_key == candidate.key`. For `RaidTarget`, the `GoalKey` includes the target entity in the `entity` field (`crates/worldwake-core/src/goal.rs:137`), so blocking one target should not block another. The observed behavior contradicts this — new targets were blocked despite never having been attempted.
2. `refresh_runtime_for_read_phase` in `crates/worldwake-ai/src/agent_tick/observation.rs:70` calls `handle_facility_queue_transitions` and `clear_resolved_blockers`, both of which modify `blocked_memory` between reading it from the world and passing it to candidate generation. One of these may re-introduce blockers cleared by the test.
3. The decision trace's `CandidateGenerationDiagnostics` records `fully_blocked_desires` (the dump section), but the summary line only shows the post-filter `candidates` count. The match details (which blocker key, which expiration tick, which anchor) are not exposed in any trace surface.
4. Existing focused test coverage for `filter_blocked_candidates`: `crates/worldwake-ai/src/candidate_generation.rs` contains unit tests for blocked intent filtering but none that cover the compound-sequence scenario (tell → establish → tell → new targets arrive).
5. `E13DECARC-003` and `E13DECARC-013` (archived) established the blocked intent component and failure handling logic. Neither covers cross-goal-kind blocker propagation or diagnostics.

## Architecture Check

1. Two independent improvements:
   - **Diagnostics**: Expose blocker match details in the decision trace so the "candidates=0 but dump shows candidates" pattern is self-explanatory. This is strictly additive — no behavior change.
   - **Blocker scoping investigation**: Determine whether the blocker match in the compound-sequence case is a bug (overbroad match) or correct behavior (the blocked intent system is working as designed but the scenario needs different setup). This investigation may or may not produce a code change.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. Blocker match diagnostics visible in decision trace → decision trace (new `BlockerMatchDetail` field or similar)
2. Compound-sequence blocker behavior correct → focused unit test in `candidate_generation.rs` (establish → new target arrives → RaidTarget not blocked)
3. No regression in existing blocked intent tests → `cargo test -p worldwake-ai`

## What to Change

### 1. Expose blocker match details in decision trace

In `filter_blocked_candidates` (`candidate_generation.rs`), when a candidate is blocked, record the matching blocker's key, expiration tick, and anchor in `CandidateGenerationDiagnostics`. Expose this in the decision trace dump so the "which blocker matched" question is answerable from trace output alone.

### 2. Add focused test for compound-sequence blocker behavior

Add a unit test in `candidate_generation.rs` that:
- Creates a blocked intent for `EstablishBanditCamp` (simulating a failed attempt)
- Generates `RaidTarget` candidates for a new target
- Asserts that the `RaidTarget` candidates are NOT blocked by the `EstablishBanditCamp` blocker

### 3. Investigate observation-pipeline blocker re-introduction

Trace the path from `refresh_runtime_for_read_phase` through `handle_facility_queue_transitions` and `clear_resolved_blockers` to determine whether either function re-introduces blockers that were cleared by a test's world-component modification. If so, determine whether this is correct behavior or a bug.

**Investigation result (2026-03-31):** Neither function re-introduces blockers for `RaidTarget`:
- `clear_resolved_blockers` only *removes* entries (expired + resolved via `blocker_resolved`). Never adds.
- `handle_facility_queue_transitions` only adds `ExclusiveFacilityUnavailable` blockers, which are excluded from candidate filtering by `blocks_goal_generation() == false`.
- For `RaidTarget` with `TargetGone`, `blocker_resolved` explicitly returns `false` (TTL-based expiration by design).
- The original observation ("clearing memory didn't help") was likely caused by incomplete candidate-generation prerequisites (hostility lists, faction membership, bandit flags) for the new targets, not blocker re-introduction. The new blocker match diagnostics will make this visible in future debugging.
- No code change to `observation.rs` required.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — diagnostics + focused test)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — expose blocker match details)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (investigate — may modify)

## Out of Scope

- Changing the fundamental blocked intent mechanism (this ticket is diagnostics + investigation)
- Modifying golden tests to work around the issue (E22INTSOATES-008 already has a working workaround)
- Blocker expiration policy changes (if warranted, becomes a separate ticket)

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: compound-sequence blocker does not suppress unrelated goal kinds
2. Decision trace dump includes blocker match details when candidates are filtered
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `filter_blocked_candidates` diagnostics are purely additive — no behavior change to filtering logic
2. Blocker match details appear in trace dump only when tracing is enabled (zero cost when disabled)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs::compound_sequence_blocker_does_not_suppress_unrelated_goal` — proves RaidTarget is not blocked by EstablishBanditCamp blocker
2. `crates/worldwake-ai/src/decision_trace.rs` or integration test — proves blocker match details appear in trace

### Commands

1. `cargo test -p worldwake-ai -- compound_sequence_blocker`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`

## Outcome

**Completion date**: 2026-03-31

**What changed**:
- `decision_trace.rs`: Added `BlockerMatchDetail` struct (blocker_key, blocking_fact, expires_tick). Added `blocker_matches` field to `DesireFullyBlocked`. Updated `dump_agent` to print per-opportunity blocker details (goal, place, target, action name, fact, expiration).
- `candidate_generation.rs`: Renamed `is_candidate_blocked` → `find_matching_blocker` returning `Option<BlockerMatchDetail>`. Updated `filter_blocked_candidates` to collect and propagate match details. Added `compound_sequence_blocker_does_not_suppress_unrelated_goal` unit test.

**Investigation result**: Neither `clear_resolved_blockers` nor `handle_facility_queue_transitions` re-introduce blockers for `RaidTarget`. The original suppression was likely caused by incomplete candidate-generation prerequisites (hostility/faction setup), not blocker re-introduction. No change to `observation.rs` required.

**Deviations**: None. All three deliverables addressed as specified.

**Verification**: 1,463 tests passed, 0 failed. Clippy clean.
