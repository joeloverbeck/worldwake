# S151TESRELROU-006: Observation-phase hook for testimony reliability + route preference updates

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new AI-tick observation phase reading belief overwrites and travel-action commits, writing to runtime stores, and populating decision-event contexts
**Deps**: archive/tickets/S151TESRELROU-001.md, S151TESRELROU-002, S151TESRELROU-003, S151TESRELROU-005

## Problem

S151 needs to fire `TestimonyReliability` updates from belief-overwrite events (AskWitness confirmation/refutation, stale-claim observation, contradiction detection) and `RoutePreference` updates from `TravelTo` action commits (safe traversal) and threat-class events during travel (dangerous traversal). The spec explicitly takes a no-new-event-tag stance — all updates flow through existing event-log emissions and existing belief-store overwrite sites.

## Assumption Reassessment (2026-05-17)

1. Belief overwrite sites: `refute_entity_claims()` at `crates/worldwake-core/src/belief.rs:129-150` clears stale claims; `import_entity_snapshot()` at `crates/worldwake-core/src/belief.rs:163-193` overwrites claims with fresh observations. `EntityBeliefClaim` at `crates/worldwake-core/src/entity_belief_claim.rs:53-64` carries `source: PerceptionSource` and `acquired_tick: Tick`; `PerceptionSource::Report { from: EntityId, chain_len: u8 }` at `belief.rs:2481-2486` is the witness-provenance carrier the hook reads.
2. Travel events: there is **no** `EventTag::TravelCompleted` variant (per Step 2 spot-check (a)). Travel completions appear as `EventTag::ActionCommitted` entries whose `ActionState::Travel { edge_id, origin, destination, departure_tick, arrival_tick }` payload identifies the segment. Threat-class events during the action's tick window: `EventTag::Combat`, `EventTag::Escalation`, `EventTag::WildernessRelief`, and any wound-applied tag.
3. Existing AI-tick observation infrastructure lives in `crates/worldwake-ai/src/agent_tick/` (the perception-import phase is the natural sibling to the new hook). The new observation phase reads:
   - Per-agent belief diff (which claims were refuted/replaced this tick, with their prior `PerceptionSource::Report { from }` provenance) to drive testimony updates.
   - Per-agent tick-scoped event-log entries (TravelTo commits + threat-class events with the traveler as actor) to drive route-preference updates.
4. The hook must run AFTER the belief-store overwrites land in the current tick (so the diff is complete) and AFTER the action-commit events are appended, but BEFORE downstream readers (ranking in tickets 007/008) consume the updated stores. The exact phase placement within `agent_tick/mod.rs` is determined during implementation — likely immediately after the perception-import phase.
5. Goal-commit integration: when planner commits a goal whose plan references witness sources or traverses tracked segments, the commit handler populates `GoalCommittedPayload.testimony_trust_context` and `route_preference_context` (from ticket 005) by snapshotting current `TestimonyReliability`/`RoutePreference` derived views for the referenced entities. This lives in the planner's commit path (`agent_tick/planning.rs:1190` constructs the payload), not in the observation hook itself.

## Architecture Check

1. Per FND-15: knowledge is acquired locally; the hook reads only the agent's own belief diff and tick-scoped events, never global truth.
2. Per FND-22A: updates are concrete state changes with `EventId` provenance — every counter increment carries the originating event reference in `provenance_events`.
3. Per FND-26: state-mediated; the hook reads belief-store and event-log state, writes to per-agent runtime stores. No cross-system command channels.
4. No new event tag (spec Non-Goal) — existing `EventTag::ActionCommitted` + `ActionState::Travel` payload + threat-class event tags carry all required information.
5. Hook placement order: AFTER perception import (so belief diff is complete), AFTER action commits land (event-log entries appended), BEFORE planner reads `TestimonyReliability`/`RoutePreference` (so consumer reads see the updated state).

## Verification Layers

1. Belief-overwrite hook correctness → focused unit test: construct an `AgentDecisionRuntime` with a witness-sourced belief, run the hook with a direct-observation overwrite event, assert `direct_confirmations` or `direct_refutations` incremented appropriately.
2. Travel-event hook correctness → focused unit test: simulate a `TravelTo` commit without threat events → `safe_traversals` increments; simulate with `EventTag::Combat` in the tick window → `dangerous_traversals` increments.
3. Goal-commit context population → integration test: a goal commit referencing a witness with non-empty `TestimonyReliability` produces a `GoalCommittedPayload` with the matching `TestimonyTrustSummary` in `testimony_trust_context`.
4. Determinism → repeated runs with the same observation sequence produce identical store snapshots (BTreeMap ordering preserved).
5. Hook ordering → action trace + decision trace show the hook fires after perception import and before ranking; verified via trace assertions in a focused integration test.

## What to Change

### 1. New observation-phase module in `crates/worldwake-ai/src/agent_tick/`

Add a new file (likely `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs`) with the per-agent hook:

```rust
pub fn record_learned_state_updates(
    runtime: &mut AgentDecisionRuntime,
    agent: EntityId,
    belief_diff: &BeliefStoreDiff,             // from perception-import phase
    tick_events: &[EventLogEntry],             // current-tick events with `actor == agent`
    profile_view: &dyn GoalBeliefView,         // for topic-scope and profile reads
    current_tick: Tick,
) {
    // Phase A: Testimony reliability updates
    for refuted_claim in belief_diff.refuted_claims_for(agent) {
        if let PerceptionSource::Report { from: witness, .. } = refuted_claim.source {
            let topic = entity_aspect_to_topic_scope(&refuted_claim.aspect);
            let key = TestimonyReliabilityKey { source: witness, topic };
            // Distinguish refutation vs. stale per the diff classification:
            //   - direct observation contradicts → record_refutation
            //   - claim aged out without contradiction → record_stale
            //   - direct observation confirms (claim re-acquired with same value but new source) → record_confirmation
            //   - simultaneous-claim conflict → record_contradiction
            runtime.testimony_reliability.record_<variant>(key, refuted_claim.refuting_event_id, current_tick);
        }
    }

    // Phase B: Route preference updates
    for travel_commit in tick_events.iter().filter(|e| matches!(e.tag, EventTag::ActionCommitted)
                                                     && matches!(e.payload, ActionPayload::Travel { .. })) {
        let segment = RouteSegment::new(travel_commit.origin, travel_commit.destination);
        let threat_observed = tick_events.iter().any(|e| matches!(e.tag,
            EventTag::Combat | EventTag::Escalation | EventTag::WildernessRelief)
            && e.actor_or_target_includes(agent)
            && tick_window_overlaps(travel_commit, e));
        if threat_observed {
            runtime.route_preference.record_dangerous(segment, threat_event_id, current_tick);
        } else {
            runtime.route_preference.record_safe(segment, current_tick);
        }
    }
}
```

The precise pattern-matching shapes (`BeliefStoreDiff`, `ActionPayload::Travel`, threat-event filtering) are determined during implementation by reading the actual types — the structure above is the contract, not the literal code.

### 2. Wire the hook into the agent-tick observation phase

Edit `crates/worldwake-ai/src/agent_tick/mod.rs` (and any related orchestration file) to invoke `record_learned_state_updates` at the appropriate phase boundary — after perception-import, before planner ranking. Likely site: the per-agent loop in the existing `run_agent_tick` or equivalent function.

### 3. Populate decision-history contexts in the planner commit path

Edit `crates/worldwake-ai/src/agent_tick/planning.rs` around the `GoalCommittedPayload` construction at line 1190:

- For witness-sourced commits, iterate the goal's belief dependencies and snapshot `TestimonyTrustSummary` for each unique source-topic pair.
- For commits whose plan crosses tracked `RouteSegment`s, snapshot `RoutePreferenceSummary` for each segment with a non-default entry.

Both snapshots use the derived `trust()` and `preference()` views from ticket 003.

### 4. Populate `GoalSuppressedPayload.testimony_trust_context`

This is owned primarily by ticket 007 (the suppression-decision site), but the snapshot helper introduced here (a small function returning `Vec<TestimonyTrustSummary>` for a given witness set) is reused by both. Co-locate the helper near the planner commit path so both consumers reference the same code.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs` (new)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — hook invocation)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — commit-time context population at line 1190 and 4023)
- Likely: `crates/worldwake-core/src/belief.rs` — if `BeliefStoreDiff` doesn't already expose refuted-claim provenance, add the necessary accessor (grep `BeliefStoreDiff` to confirm)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — new tests for the hook)

## Out of Scope

- Consumer reads (ranking damping in ticket 007, travel cost in ticket 008)
- Diagnostics aggregator (ticket 009 reads the populated decision-event contexts emitted here)
- `SAVE_FORMAT_VERSION` bump (ticket 010)
- Profile parameters and derived-view formulas (ticket 003)

## Acceptance Criteria

### Tests That Must Pass

1. Direct-observation refutation of a witness-sourced belief increments `TestimonyReliability.entries[(witness, topic)].direct_refutations` for the mapped `TopicScope`.
2. Direct-observation confirmation of a witness-sourced belief increments `direct_confirmations`.
3. Stale-claim eviction via `refute_entity_claims` increments `stale_claims`.
4. Two simultaneous Reports about the same `(subject, aspect)` with the agent picking one increments `contradicted_claims` for the loser's witness.
5. A successful `TravelTo` commit with no threat events in the tick window increments `safe_traversals` and updates `last_safe_tick`.
6. A `TravelTo` commit with `EventTag::Combat` or another threat-class event during the action's tick window increments `dangerous_traversals` and stores the event ID in `last_traversal_event`.
7. A goal commit referencing a witness with a non-empty `TestimonyReliability` entry produces a `GoalCommittedPayload` with `testimony_trust_context` populated.
8. A goal commit whose plan crosses a tracked segment produces a `GoalCommittedPayload` with `route_preference_context` populated.
9. Determinism: same observation sequence on a fresh runtime produces identical store snapshots and identical payload contexts.
10. Existing suite: `cargo test --workspace`.

### Invariants

1. Hook reads only per-agent belief diffs and tick-scoped events with the agent as actor or target — no global belief or event scan.
2. No new event tag emitted — all hook inputs come from existing event-log substrate.
3. Per-agent `EventId` provenance attached to every counter increment.
4. Hook ordering: perception-import → learned-state observation → ranking. Asserted by trace ordering test.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/learned_state_observation.rs#[cfg(test)]` — per-variant hook tests (confirmation, refutation, stale, contradiction, safe traversal, dangerous traversal).
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — integration test asserting hook ordering and goal-commit context population.

### Commands

1. `cargo test -p worldwake-ai learned_state_observation`
2. `cargo test -p worldwake-ai agent_tick`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `./scripts/verify.sh`
