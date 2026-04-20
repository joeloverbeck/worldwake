# S110DECHISEVE-004: Decision event emission from AI pipeline

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — 9 integration points across `worldwake-ai` emit new `EventTag` + `DecisionEventPayload` pairs inline at existing decision sites
**Deps**: archive/tickets/S110DECHISEVE-002.md (`EventTag` variants, `DecisionEventPayload`, and `EventPayload::decision_payload` field must exist), archive/tickets/S110DECHISEVE-003.md (`CognitiveProfile::decision_history_alternatives` must exist for `GoalCommittedPayload` truncation)

## Problem

Tickets 002 and 003 ship the schema; this ticket wires emission. Every agent decision — commit, suppress, suspend, abandon, adopt, invalidate, expectation-mismatch, repair, replan, blocker-record — must emit a typed event to the authoritative append-only log so FND-29 ("why did this agent do that?") and FND-29A (causal history is authoritative and queryable) can be answered from world history without opt-in `AgentDecisionTrace` sinks. Emission happens at existing call sites where the planner already makes these decisions; no new SystemFn is introduced.

## Assumption Reassessment (2026-04-20)

1. Ranking function is named `rank_candidates` and `rank_candidates_with_memories` at `crates/worldwake-ai/src/ranking.rs:86` and `:108` (not `rank_goals` — corrected in the spec during reassessment). Failure-handling entry is `handle_plan_failure` at `crates/worldwake-ai/src/failure_handling.rs:35`. Candidate-emission function is `emit_candidate_with_trace` at `crates/worldwake-ai/src/candidate_generation.rs:4130`. Planning commit/adoption lives in `crates/worldwake-ai/src/agent_tick/planning.rs`. Goal-switch path uses `goal_switching.rs` and the `agent_tick` module boundary. Plan-revalidation is `crates/worldwake-ai/src/plan_revalidation.rs`. `handle_plan_failure` is invoked from `agent_tick/active_action.rs:292`.
2. Existing trace-building code in the AI crate already assembles the data each payload needs: `emit_candidate_with_trace` produces `GroundedGoal` with evidence sets (`EvidenceSummary` source), `rank_candidates_with_memories` captures rejected alternatives (source for `GoalCommittedPayload::rejected_alternatives`), `plan_revalidation.rs` already matches on invalidation reasons (source for `PlanInvalidationReason`), and `failure_handling.rs` already constructs `Discrepancy` / `BlockingFact` records (source for `BlockerRecordedPayload`). The ticket's work is to add `EventLog::emit(PendingEvent::from_payload(EventPayload { …, decision_payload: Some(DecisionEventPayload::…), tags: BTreeSet::from([EventTag::…]), … }))` calls at each site, constructing the payload from data already available at that call site. No new data plumbing is needed.
3. Shared abstraction boundary under audit: the `EventLog` write path from the AI crate. The planner already writes `ActionStarted` / `ActionCommitted` events to the same log via `EventLog::emit`, so decision-event emission rides the same authoritative write contract. FND-26 preserved — no privileged cross-system call; the event log is shared authoritative state.
4. No failing golden motivates this ticket; S110's spec is a proactive FND-29/29A enablement. Existing goldens (`golden_healer_acquires_remote_ground_medicine_for_patient`, `golden_survival_baseline`, etc.) will begin to contain the new events but must not regress in their existing assertions — tests that filter on specific `EventTag` values continue to work because the new events have distinct tags.
5. Live planner surface: all 11 decision kinds map to existing planner code paths — no `GoalKind` change, no new operator, no new affordance. The emission is additive.
6. This is a runtime `agent_tick` regression target surface. Full action registries are required for the integration tests — not a needs-only harness.
7. Ordering: decision events are emitted inline at the decision site's tick. Same-tick multiple-decision ordering follows existing event-log insertion order (event-log ordering, per FND-9 tie-break rule). `GoalOffered` for a candidate precedes `GoalCommitted` for the winning candidate within the same planning frame because candidate generation runs before ranking/commit in the current `agent_tick/planning.rs` pipeline. The compared branches are naturally asymmetric (offered → ranked → committed), so divergence is driven by pipeline-stage ordering, not motive score.
8. No heuristic is removed or weakened. This ticket adds record-only side effects; it does not change candidate gating, ranking formulas, or plan-search control flow.
9. Not a stale-request, contested-affordance, or start-failure ticket.
13. Adjacent contradictions from reassessment: `PlanExpectation` is an S114 future type and not yet landed. This ticket fires `ExpectationMismatch` only for existing `expected_materializations` mismatches (already detected in `plan_revalidation.rs`); S114 will later widen the trigger set and the `ExpectationMismatchPayload` shape. The pre-S114 payload holds `Vec<MaterializationTag>` (from ticket 001's core relocation); S114's widening is a separate ticket in a future spec and is not in-scope here.
15. `rejected_alternatives` truncation: at emission time in `agent_tick/planning.rs` (commit path), sort the captured alternatives by `(motive_score desc, goal_key asc)` and take the first `agent.cognitive_profile.decision_history_alternatives as usize` entries. Secondary `GoalKey` sort breaks ties deterministically per CLAUDE.md's determinism invariant (`BTreeMap`/`BTreeSet` only in authoritative state, no `HashMap`/`HashSet`).

## Architecture Check

1. Inline emission at existing decision sites is cleaner than a centralized "decision-event bus" because the data needed for each payload is already in local scope at each call site — a bus would force every decision site to serialize into a common intermediate representation, adding cost and indirection. The existing `ActionStarted` / `ActionCommitted` pattern is exactly this: inline `EventLog::emit` at the handler. Decision events use the same idiom.
2. No backwards-compat layer. Pre-S110 event logs are not decodable by post-S110 code (ticket 002 bumps `SAVE_FORMAT_VERSION`); emission follows the same contract.
3. No SystemFn — this preserves the spec's Non-Goal of avoiding a query API over the log. Emission is a pure side effect at the decision site; consumers read the log sequentially.

## Verification Layers

1. Decision-event emission per kind → integration test: for a golden scenario tick, assert that `EventLog::events_by_tag(EventTag::GoalCommitted)` returns the expected count and that `GoalCommittedPayload::goal_key` matches the agent's committed goal for that tick. Separate per-variant assertions rather than one aggregated test so each emission path proves independently.
2. `rejected_alternatives` truncation correctness → focused unit test in `agent_tick/planning.rs` `#[cfg(test)]`: construct a ranking scenario with 10 alternatives, set `decision_history_alternatives = 3`, commit a goal, and assert the emitted `GoalCommittedPayload::rejected_alternatives.len() == 3` with the expected `(motive_score, goal_key)` ordering.
3. `PlanInvalidationReason` mapping → focused unit test in `plan_revalidation.rs` `#[cfg(test)]`: for each invalidation code path, assert the emitted `PlanInvalidatedPayload::reason` variant matches. This is the critical correctness surface because multiple invalidation reasons can fire at distinct call sites within `plan_revalidation.rs`.
4. `BlockerRecordedPayload` memory-class routing → focused unit test in `failure_handling.rs` `#[cfg(test)]`: for a `Discrepancy` record, assert `BlockerRecordedPayload::discrepancy == Some(…)` and `blocking_fact == None`; for a `BlockingFact` record (contention loss path), assert the inverse. Exactly one of the two is `Some`.
5. Existing golden traces (no regression) → full `cargo test -p worldwake-ai` suite passes. Existing assertions on `ActionStarted` / `ActionCommitted` events continue to hold because the new events have distinct tags.
6. Authoritative log ordering within a tick → event-log delta assertion in an integration test: for a scenario where one agent commits a goal and adopts a plan in the same tick, assert `EventTag::GoalCommitted` precedes `EventTag::PlanAdopted` in `EventLog::events_at_tick(t)`. This is the inline-emission contract: pipeline order → event-log order.

## What to Change

### 1. `GoalOffered` emission in `candidate_generation.rs`

In `emit_candidate_with_trace` (line 4130), after the `GroundedGoal` is constructed and before the function returns, call `EventLog::emit` with:

- `tags: BTreeSet::from([EventTag::GoalOffered])`
- `decision_payload: Some(DecisionEventPayload::GoalOffered(GoalOfferedPayload { agent, goal_key, emitter, source_evidence }))`
- `agent` from the call context; `goal_key` from the `GroundedGoal`; `emitter` mapped from the AI-internal emitter identifier to the core `EmitterTag` variant (add a private mapping helper in `candidate_generation.rs`); `source_evidence` built from the evidence-set counts via an `EvidenceSummary::from_grounded_goal(&grounded_goal) -> EvidenceSummary` helper (new, in core or ai — decide at implementation time based on what types are in scope).
- `actor_id: Some(agent)`, `tick` from the context, `cause: CauseRef::SystemTick(tick)`, all other `EventPayload` fields default to their empty values.

### 2. `GoalSuppressed` emission in `ranking.rs`

In `rank_candidates_with_memories` (line 108), identify the suppression branches (blocker-memory filter, discrepancy filter, contention-preempt filter). At each branch, emit `EventTag::GoalSuppressed` with `GoalSuppressedPayload { agent, goal_key, reason: GoalRejectionReason::… }` using the variant corresponding to the suppression type. Do not emit for losers of arbitration — those are captured in `GoalCommittedPayload::rejected_alternatives` at commit time, not separately.

### 3. `GoalCommitted` emission in `agent_tick/planning.rs` (post-ranking commit path)

At the point where ranking selects a goal for commitment, construct `GoalCommittedPayload` from:

- `agent, goal_key, motive_score` — available in local scope.
- `rejected_alternatives` — take the ranked-but-not-committed list, sort by `(motive_score desc, goal_key asc)`, truncate to `agent.cognitive_profile.decision_history_alternatives as usize`, map each to `RejectedAlternativeSummary { goal_key, rejection_reason: GoalRejectionReason::LowerMotive, score_gap: committed_motive_score as i32 - alt_motive_score as i32 }`.

Emit with `tags: BTreeSet::from([EventTag::GoalCommitted])`.

### 4. `GoalSuspended` / `GoalAbandoned` emission in `goal_switching.rs`

At the goal-switch decision path in `goal_switching.rs`, differentiate suspension (awaiting a condition, reason captured via `SuspensionReason`) from abandonment (no resumption path). Emit `EventTag::GoalSuspended` + `GoalSuspendedPayload { agent, goal_key, reason: <existing SuspensionReason> }` for suspension, `EventTag::GoalAbandoned` + `GoalAbandonedPayload { agent, goal_key, reason: GoalRejectionReason::… }` for abandonment.

### 5. `PlanAdopted` emission in `agent_tick/planning.rs` (successful plan build)

When `search_plan` returns a plan and it is assigned to the agent, emit `EventTag::PlanAdopted` + `PlanAdoptedPayload { agent, goal_key, plan_step_count: plan.steps.len() as u16 }`.

### 6. `PlanInvalidated` emission in `plan_revalidation.rs`

Every invalidation return path in `plan_revalidation.rs` emits `EventTag::PlanInvalidated` + `PlanInvalidatedPayload { agent, goal_key, reason: PlanInvalidationReason::… }` with the reason variant matching the detected invalidation cause. A small match in-module maps the local invalidation code to the `PlanInvalidationReason` variant.

### 7. `ExpectationMismatch` emission in `plan_revalidation.rs` (pre-S114)

When an explicit `expected_materializations` mismatch is detected during revalidation, emit `EventTag::ExpectationMismatch` + `ExpectationMismatchPayload { agent, goal_key, step_index, expected_materializations: <the step's expected tag list, extracted as `Vec<MaterializationTag>`> }`. The step's `expected_materializations: Vec<ExpectedMaterialization>` field is mapped to `Vec<MaterializationTag>` by projecting `.tag` from each `ExpectedMaterialization`.

### 8. `RepairApplied` emission in `failure_handling.rs`

At each successful local repair path (alternate target, alternate route, alternate merchant, alternate recipe), emit `EventTag::RepairApplied` + `RepairAppliedPayload { agent, goal_key, step_index, repair_kind: RepairKind::… , substitute_target: Option<EntityId> }`. The `RepairKind` mapping is a small match at each repair site.

### 9. `BlockerRecorded` emission in `failure_handling.rs` (both memory classes)

At every memory-record call site — both `DiscrepancyMemory` and `BlockerMemory` — emit `EventTag::BlockerRecorded` + `BlockerRecordedPayload { agent, blocker_key, discrepancy: Option<Discrepancy>, blocking_fact: Option<BlockingFact>, expires_tick }`. Exactly one of `discrepancy` / `blocking_fact` is `Some`, matching the memory class being written.

### 10. `ReplanTriggered` emission in `handle_plan_failure`

At the end of `handle_plan_failure` (and any other replan trigger in `agent_tick/active_action.rs:292` or sibling call sites), emit `EventTag::ReplanTriggered` + `ReplanTriggeredPayload { agent, goal_key, reason: ReplanReason::… }`.

### 11. Per-call-site tests

For each of sections 1–10, add a focused unit test in the appropriate `#[cfg(test)]` block that constructs a minimal scenario triggering the emission and asserts `EventLog::events_by_tag(EventTag::…)` returns exactly one event with the expected payload.

### 12. Integration test on an existing golden

Add a new integration test in `crates/worldwake-ai/tests/` — `golden_decision_history_events.rs` — that runs `survival-baseline.ron` for a small tick window and asserts the event log contains expected `GoalCommitted`, `PlanAdopted`, `ActionStarted`, `ActionCommitted` sequences with correct same-tick ordering (item 5 in Verification Layers).

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — `GoalOffered` emission + `EmitterTag` mapping helper + `EvidenceSummary` construction)
- `crates/worldwake-ai/src/ranking.rs` (modify — `GoalSuppressed` emission at filter branches)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — `GoalCommitted` + `PlanAdopted` emission, truncation logic using `CognitiveProfile::decision_history_alternatives`)
- `crates/worldwake-ai/src/goal_switching.rs` (modify — `GoalSuspended` + `GoalAbandoned` emission)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — `PlanInvalidated` + `ExpectationMismatch` emission)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — `RepairApplied` + `BlockerRecorded` emission, both memory classes)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify — `ReplanTriggered` emission at `handle_plan_failure` call)
- `crates/worldwake-ai/tests/golden_decision_history_events.rs` (new — integration test)

## Out of Scope

- Widening `ExpectationMismatch` to cover S114's plan-step guards. Only the existing `expected_materializations` trigger is wired; S114 is a later spec.
- Observer rendering of the new events. Ticket 006 adds the "Decision History" section.
- Replay-invariance test. Ticket 005 adds the explicit decision-event replay check.
- Per-emitter gating (volume control). Every candidate offer emits `GoalOffered`; volume growth is accepted per the spec's D5 note. A future spec may add gating if observer workload demands.
- Changing any planner decision logic. Emission is a pure record-only side effect; ranking, commitment, invalidation, and repair logic are all unchanged.
- Adding a `decision_payload` populator for any other event kind (action lifecycle, world mutation, etc.). Those remain `None`.

## Acceptance Criteria

### Tests That Must Pass

1. Per-emission focused unit tests (11 total, one per variant) — each asserts the emission fires with the expected payload on the canonical trigger.
2. `rejected_alternatives` truncation unit test — 10 alternatives + `decision_history_alternatives = 3` produces exactly 3 entries sorted by `(motive_score desc, goal_key asc)`.
3. `BlockerRecordedPayload` memory-class routing unit tests (two tests): one for `DiscrepancyMemory` path, one for `BlockerMemory` path.
4. New integration test `golden_decision_history_events` — `survival-baseline.ron` emits expected `GoalCommitted` / `PlanAdopted` / `ActionStarted` / `ActionCommitted` sequence with correct same-tick ordering.
5. All existing goldens and AI-crate tests pass with no assertion change.
6. `cargo test --workspace` passes.
7. `cargo clippy --workspace --all-targets -- -D warnings` clean.

### Invariants

1. Every commit decision emits exactly one `GoalCommitted` event with `rejected_alternatives.len() <= decision_history_alternatives`.
2. `BlockerRecordedPayload` has exactly one of `discrepancy` / `blocking_fact` populated — never both, never neither.
3. No planner decision logic is changed by this ticket — emission is additive and record-only; golden E2E outcomes (committed goal sequences, action counts, death counts) are byte-identical to pre-ticket runs except for the new events in the log.
4. Same-tick pipeline-stage ordering is reflected in event-log order: `GoalOffered` for a candidate precedes `GoalCommitted` for a winner; `GoalCommitted` precedes `PlanAdopted` for the committed goal's plan.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (`#[cfg(test)]`) — `emit_candidate_with_trace_emits_goal_offered_event`.
2. `crates/worldwake-ai/src/ranking.rs` (`#[cfg(test)]`) — `rank_candidates_emits_goal_suppressed_on_filter`.
3. `crates/worldwake-ai/src/agent_tick/planning.rs` (`#[cfg(test)]`) — `commit_emits_goal_committed_with_truncated_alternatives`, `plan_adopted_fires_after_commit`.
4. `crates/worldwake-ai/src/goal_switching.rs` (`#[cfg(test)]`) — `goal_switch_emits_suspended_or_abandoned`.
5. `crates/worldwake-ai/src/plan_revalidation.rs` (`#[cfg(test)]`) — per-`PlanInvalidationReason`-variant tests and `expectation_mismatch_fires_on_materialization_drift`.
6. `crates/worldwake-ai/src/failure_handling.rs` (`#[cfg(test)]`) — `repair_applied_per_repair_kind`, `blocker_recorded_discrepancy_path`, `blocker_recorded_blocking_fact_path`.
7. `crates/worldwake-ai/src/agent_tick/active_action.rs` (`#[cfg(test)]`) — `handle_plan_failure_emits_replan_triggered`.
8. `crates/worldwake-ai/tests/golden_decision_history_events.rs` — new integration test exercising `survival-baseline.ron` for decision-event presence and same-tick ordering.

### Commands

1. `cargo test -p worldwake-ai decision_history` — runs the focused unit tests and the new integration test (all prefixed appropriately).
2. `cargo test -p worldwake-ai` — full AI-crate suite, confirms no golden regression.
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
