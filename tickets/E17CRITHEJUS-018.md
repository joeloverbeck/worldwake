# E17CRITHEJUS-018: Strengthen ShareBelief ranking and Tell traceability

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — decision-trace summaries in `worldwake-ai` and Tell action-trace surfaces in `worldwake-sim` / `worldwake-systems`
**Deps**: E17CRITHEJUS-017

## Problem

`E17CRITHEJUS-017` exposed a traceability gap rather than a remaining simulation-law bug. The cleaned institutional Tell boundary now behaves correctly, but the trace surfaces still make some cross-layer failures harder to explain than they should be.

During the refactor, decision traces could show that `GoalKind::ShareBelief { .. }` was generated or selected, and action traces could show that a Tell committed. But two architecturally important explanations still required source inspection or lower-layer bespoke assertions:

1. why an actionable political goal such as `GoalKind::SupportCandidateForOffice { .. }` lost to Tell chatter inside `compare_ranked_goals()`
2. what concrete Tell mutation class actually landed when a Tell committed, without unpacking lower-layer storage state by hand

That violates the project’s explainable-emergence standard. `docs/FOUNDATIONS.md` requires fully traceable local information flow and explainable decisions, especially for social artifacts, institutional claims, and political action.

## Assumption Reassessment (2026-03-26)

1. Decision traces already persist ranked-goal summaries in `crates/worldwake-ai/src/decision_trace.rs` via `PlanningPipelineTrace.ranking.ranked`, `PlanningPipelineTrace.selection.top_challenger`, and `DecisionTraceSink::goal_history_for()`. Current coverage includes `decision_trace::tests::goal_status_distinguishes_omitted_suppressed_zero_motive_ranked_and_selected`, `decision_trace::tests::goal_status_reports_social_omission_reason`, `agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning`, and golden trace scenarios such as `golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry`.
2. `crates/worldwake-ai/src/ranking.rs` currently resolves final ordering in `compare_ranked_goals()`, using `priority_class`, `feasibility`, `motive_score`, `compare_share_belief_topics()`, then the remaining key tiebreakers. The live regression from `E17CRITHEJUS-017` came from this ranking substrate, not from candidate absence or authoritative start failure.
3. Action traces already carry Tell commit outcome data through `CommitTraceData::Tell(TellCommitTrace)` in `crates/worldwake-sim/src/action_handler.rs` and `crates/worldwake-sim/src/action_trace.rs`. `TellCommitTrace` includes `result` and `belief_delta`, and focused tests already cover formatting through `action_trace::tests::summary_includes_tell_commit_trace_when_present`.
4. The remaining gap is not raw data absence in every path. It is that the fastest, most commonly queried trace surfaces still emphasize `ActionTraceDetail::Tell { listener, topic }` identity and selected-goal summaries more than the ranking-comparison and mutation-class explanations that mattered during the E17 refactor.
5. The exact shared abstraction boundary under audit is the social-decision explanation path: `GoalKind::ShareBelief { listener, topic }` ranking in `crates/worldwake-ai/src/ranking.rs` -> ranked-goal capture in `crates/worldwake-ai/src/decision_trace.rs` -> Tell authoritative commit in `crates/worldwake-systems/src/tell_actions.rs` -> Tell commit trace serialization/summary in `crates/worldwake-sim/src/action_trace.rs`.
6. The live `GoalKind` family under test is mixed: `GoalKind::ShareBelief { .. }` versus political goals including `GoalKind::SupportCandidateForOffice { .. }` and `GoalKind::ClaimOffice { .. }`. The immediate failure mode from `E17CRITHEJUS-017` was ranking divergence driven by priority class and final ordering, not by plan search infeasibility or authoritative rejection.
7. Ordering matters here, but the contract is decision-layer ranking ordering and action-lifecycle ordering, not strict tick separation. The compared branches are not symmetric under the live architecture because `SupportCandidateForOffice` and `ShareBelief` can differ by priority class, motive score, and topic tiebreakers.
8. No heuristic removal is proposed. This ticket strengthens explanation surfaces around existing ranking and commit semantics so debugging no longer requires source-diving or custom state assertions to understand lawful behavior.
9. This is not a stale-request or start-failure ticket. The first boundary under audit is decision ranking (`compare_ranked_goals()`), followed by authoritative Tell commit trace reporting (`tell_trace()` / `TellCommitTrace`).
10. Political precision: the motivating office scenario involved support declaration and office succession follow-through. The immediate closure boundary that became opaque was not office-holder mutation itself, but why `SupportCandidateForOffice` lost earlier in AI ranking and why Tell commits changed listener knowledge in a specific belief lane.
11. Existing tests prove outcome, but not always the most useful provenance:
    - ranking outcome -> `ranking::tests::support_candidate_uses_social_weight_times_loyalty`
    - social omission/explanation -> `agent_tick::tests::trace_social_resend_omission_reason`
    - same-tick Tell ordering -> `tick_step::tests::action_trace_records_tell_detail_without_disturbing_ordering`
    - political goldens -> `golden_bribe_support_coalition`, `golden_force_control_locality_requires_tell`, `golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact`
12. `docs/FOUNDATIONS.md` already requires this work: Principle 7 (local communication paths), Principle 13 (information-path legibility), Principle 16 (memories and records as world state), Principle 18 (explainable decisions), Principle 23 (social artifacts are first-class), and Principle 24 (systems interact through state, not hidden commands).
13. Adjacent contradictions exposed during reassessment:
    - required consequence of this ticket: decision traces should explain ranked-goal loss reasons at the same abstraction level they explain candidate omission reasons today
    - separate future cleanup: broader trace UX improvements outside Tell/political scenarios do not belong in this ticket
14. Mismatch + correction: the problem is not “action traces lack Tell delta data.” The lower-level data already exists; the ticket scope is to expose and verify that data on the high-signal debugging surfaces actually used in mixed-layer investigations.

## Architecture Check

1. The clean architecture is to make the social-decision path self-explanatory at the same level as the world-state path. If information-carrying artifacts and ranking choices drive emergence, the trace model must expose those causes directly rather than relying on source inspection or ad-hoc debugging.
2. This is cleaner than adding scenario-specific debug output or more golden-only assertions. The trace surfaces become a reusable debugging contract for all future social and political refactors.
3. No backwards-compatibility aliasing or alternate tell paths are introduced. The ticket only enriches explanation and verification around the single cleaned `TellTopic` / `TellCommitTrace` architecture.

## Verification Layers

1. Ranked-goal loss explanation for `SupportCandidateForOffice` versus `ShareBelief` -> focused `decision_trace.rs` and/or `agent_tick` trace tests
2. Tell commit mutation class (`EntityBelief` / `InstitutionalBelief` / `SocialObservation` / `Mixed` / `None`) remains visible in the most useful trace summaries -> focused `action_trace.rs` tests
3. Tell action lifecycle ordering remains unchanged while adding richer tell summaries -> focused `tick_step` action-trace ordering tests
4. Political golden scenarios still remain explainable after the trace additions -> targeted `golden_offices.rs` and `golden_emergent.rs` trace assertions
5. Strongest lower-layer proof surface remains the existing authoritative `tell_actions.rs` tests; this ticket should not weaken those into trace-only proof
6. Mixed-layer ticket; explicit layer mapping is required because the regression surfaced in AI ranking but was investigated through Tell action outcomes

## What to Change

### 1. Add ranked-goal comparison explanations to decision traces

- Extend `crates/worldwake-ai/src/decision_trace.rs` so the selected goal and top challenger expose why the challenger lost at the final comparison boundary.
- Capture the decisive comparison dimensions from `crates/worldwake-ai/src/ranking.rs::compare_ranked_goals()`: priority class, feasibility, motive score, share-belief topic ordering, and any later deterministic tiebreakers when they are the actual cause.
- Make the explanation queryable in structured form, not only rendered text, so focused tests can assert on it directly.

### 2. Expose Tell commit mutation class more directly in action traces

- Keep `TellCommitTrace` as the source of truth, but make the high-signal summary/query surface expose `belief_delta` and `result` without forcing tests or debugging helpers to unpack nested commit payloads manually.
- Update `crates/worldwake-sim/src/action_trace.rs` summary helpers and any related typed-query helpers accordingly.
- Do not duplicate authoritative state inside the trace; expose the existing explicit mutation classification already produced by `crates/worldwake-systems/src/tell_actions.rs`.

### 3. Add focused trace tests for the exact E17 failure mode

- Add a focused decision-trace test proving a political goal loses because of explicit ranking comparison data, not merely because it was absent from the selected slot.
- Add a focused action-trace test proving institutional Tell commits advertise `TellBeliefDeltaKind::InstitutionalBelief` at the summary/query layer.
- Add or strengthen a targeted golden assertion in the bribe/support or force-control path so the political scenario can be debugged via trace surfaces alone before dropping to lower-layer state tests.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify if structured comparison metadata must be surfaced)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-sim/src/action_trace.rs` (modify)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify if the strongest targeted trace assertion lives there)

## Out of Scope

- Reopening the institutional Tell architecture from `E17CRITHEJUS-017`
- Changing political ranking semantics beyond what is necessary to expose comparison provenance
- Replacing authoritative `tell_actions.rs` state assertions with trace-only assertions
- Generic trace UX redesign unrelated to social-information or political-goal debugging

## Acceptance Criteria

### Tests That Must Pass

1. Decision traces can explain why a political goal lost to a `ShareBelief` goal using structured comparison data, not only selected-goal absence
2. Action traces can expose Tell `belief_delta` and `result` through a high-signal summary/query surface without hiding the authoritative `TellCommitTrace`
3. Existing Tell action-trace ordering tests still pass without contract changes
4. Existing political and social trace-driven goldens still pass
5. Existing suite: `cargo test -p worldwake-ai decision_trace::tests:: -- --nocapture`
6. Existing suite: `cargo test -p worldwake-sim action_trace::tests:: -- --nocapture`
7. Existing suite: `cargo test -p worldwake-ai agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning -- --exact`
8. Existing suite: `cargo test -p worldwake-ai ranking::tests::support_candidate_uses_social_weight_times_loyalty -- --exact`
9. Existing suite: `cargo test -p worldwake-ai --test golden_offices golden_bribe_support_coalition -- --exact`
10. Existing suite: `cargo test -p worldwake-ai --test golden_offices golden_force_control_locality_requires_tell -- --exact`
11. Existing suite: `cargo test -p worldwake-ai --test golden_emergent golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact -- --exact`

### Invariants

1. Social-information and political-goal behavior remains explainable as local belief-driven ranking plus explicit authoritative state mutation, consistent with Principles 7, 13, 18, 23, and 24
2. Trace additions remain derived views over existing concrete state and outcomes, not parallel truth channels, consistent with Principles 3, 16, and 25

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — add focused structured assertions for final ranked-goal comparison rationale. Rationale: prove traces expose the actual cause of political-goal loss.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — add a runtime trace test that exercises a real planning tick where a political goal loses to a social goal for an explainable reason. Rationale: prove the structured comparison data survives the live agent-tick pipeline.
3. `crates/worldwake-sim/src/action_trace.rs` — add focused tests for Tell summary/query access to `belief_delta` and `result`. Rationale: prove Tell mutation-class visibility without unpacking nested commit payloads manually.
4. `crates/worldwake-ai/tests/golden_offices.rs` — strengthen one political golden to assert on the improved trace surface before falling back to world-state checks. Rationale: make the trace layer a first-class debugging contract for office/political regressions.

### Commands

1. `cargo test -p worldwake-ai decision_trace::tests:: -- --nocapture`
2. `cargo test -p worldwake-sim action_trace::tests:: -- --nocapture`
3. `cargo test -p worldwake-ai agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning -- --exact`
4. `cargo test -p worldwake-ai ranking::tests::support_candidate_uses_social_weight_times_loyalty -- --exact`
5. `cargo test -p worldwake-ai --test golden_offices golden_bribe_support_coalition -- --exact`
6. `cargo test -p worldwake-ai --test golden_offices golden_force_control_locality_requires_tell -- --exact`
7. `cargo test -p worldwake-ai --test golden_emergent golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact -- --exact`
