# S151TESRELROU-006: Observation-phase hook for testimony reliability + route preference updates

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes - AI-tick observation hook, learned-state runtime updates, and goal-commit decision-event context snapshots
**Deps**: archive/tickets/S151TESRELROU-001.md, archive/tickets/S151TESRELROU-002.md, archive/tickets/S151TESRELROU-003.md, archive/tickets/S151TESRELROU-005.md

## Problem

Before this ticket, S151 had per-agent `TestimonyReliability` and `RoutePreference` stores plus derived trust/preference views, but the AI tick did not update those stores from newly observed belief changes or traversal experience. `GoalCommittedPayload` also carried empty learned-context vectors even when a committed plan referenced a witness or crossed a segment with learned route history.

## Assumption Reassessment

Completed on 2026-05-17.

- Belief observation is available through per-agent `AgentBeliefStore` component deltas in the event log. The hook reads `ComponentDiff::BeliefStore` compact diffs directly and computes `BeliefStoreDiff` for full `Set` deltas.
- Route traversal outcome is already represented by authoritative per-agent `RouteExperience` component deltas. That live substrate is the canonical input for safe and hostile traversal counts; the draft's direct `TravelTo` action-payload scrape was superseded by this component-delta seam.
- Dangerous-route provenance was kept tick-scoped. The hook uses same-tick `Combat`, `Escalation`, or `WildernessRelief` events involving the agent when available, and otherwise falls back to the route-experience mutation event ID.
- `GoalSuppressedPayload.testimony_trust_context` was completed by the now-archived `archive/tickets/S151TESRELROU-007.md`; this ticket added reusable goal-commit snapshot helpers but did not wire the suppression site.

## Outcome

Completed on 2026-05-17.

- Added `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs`.
- Wired `record_learned_state_updates()` into `process_agent()` after overdue expectation processing and before read/planning consumes runtime learned state.
- Recorded testimony reliability updates for direct confirmation, direct refutation, stale report claims, and same-tick report/report contradictions.
- Mirrored `RouteExperience` safe-trip and hostile-encounter deltas into `AgentDecisionRuntime.route_preference`, including current-tick threat-event provenance when available.
- Populated `GoalCommittedPayload.testimony_trust_context` for `AskWitness` goals with existing witness/topic reliability entries.
- Populated `GoalCommittedPayload.route_preference_context` for committed travel plans that cross segments with existing route preference entries.
- Added focused unit coverage for learned-state observation and goal-commit learned contexts.
- Truth-synced the now-archived `archive/specs/S151-testimony-reliability-and-route-preferences.md` so downstream tickets use the landed `RouteExperience` route-update seam.

## Deviations

- The route hook does not inspect raw `ActionCommitted` travel payloads. Live code already preserves traversal outcome as `RouteExperience`, including safe and hostile counts, so reading that authoritative component delta is narrower and more stable than reconstructing traversal state from action history.
- No core `BeliefStoreDiff` accessor was added. The existing compact diff and `BeliefStoreDiff::compute()` paths were sufficient.
- Suppression payload context population was intentionally deferred to, and later completed by, the now-archived `archive/tickets/S151TESRELROU-007.md`.

## Acceptance Result

- Criteria 1-4 passed through `agent_tick::learned_state_observation::tests::records_testimony_confirmation_refutation_stale_and_contradiction`.
- Criteria 5-6 passed through `agent_tick::learned_state_observation::tests::records_safe_and_dangerous_route_preference_from_route_experience_delta`.
- Criteria 7-8 passed through `agent_tick::planning::tests::emit_plan_selection_events_records_learned_contexts_for_committed_goal`.
- Criteria 9 is covered by deterministic `BTreeMap` grouping in the hook and by the focused snapshot assertions above.
- Existing AI tick regression coverage passed through `cargo test -p worldwake-ai agent_tick`.
- Workspace all-target clippy passed through `cargo clippy --workspace --all-targets -- -D warnings`.

## Verification Result

- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-ai learned_state_observation`
- Passed `cargo test -p worldwake-ai emit_plan_selection_events_records_learned_contexts_for_committed_goal`
- Passed `cargo test -p worldwake-ai agent_tick`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
