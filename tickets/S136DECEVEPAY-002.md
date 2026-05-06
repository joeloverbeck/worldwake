# S136DECEVEPAY-002: Reorder emit_plan_selection_events + populate assumptions at all 5 emission sites

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai::agent_tick::planning::emit_plan_selection_events` flow reordering; assumptions wire-up at failure-path emission sites in `execution`, `observation`, and `mod`
**Deps**: archive/tickets/S136DECEVEPAY-001.md

## Problem

Spec D5: `emit_plan_selection_events` (`crates/worldwake-ai/src/agent_tick/planning.rs:1003`) is called at `planning.rs:~1690` and `~2083`, BEFORE `update_frame_for_adopted_plan` and `populate_assumptions` run (`planning.rs:~1701-1707`). Today the prepared frame's assumption list is empty at the moment `GoalCommitted` and `PlanAdopted` events fire, so populating the new `assumptions` field added by ticket 001 from the active frame would always emit empty data on the success path.

This ticket lands the reorder so `update_frame_for_adopted_plan` and `populate_assumptions` run first, then wires the populated `frame.assumptions` into both `GoalCommittedPayload` and `PlanAdoptedPayload`. It also wires the failure-path payloads (`BlockerRecordedPayload`, `ReplanTriggeredPayload`, `ExpectationMismatchPayload`) where the active frame already exists at emission time. `SourceExpectationFailurePayload` does NOT gain `assumptions` per spec D4 (no active-plan frame at source-expectation failure time).

## Assumption Reassessment (2026-05-06)

1. `emit_plan_selection_events` at `planning.rs:1003` is called from two sites (`planning.rs:1692` — contested-commit path; `planning.rs:2083` — re-verify which secondary path this is at implementation time). After ticket 001, both sites construct `GoalCommittedPayload` / `PlanAdoptedPayload` with `assumptions: Vec::new()` placeholder; this ticket replaces both with real data sourced from `frame.assumptions`.
2. `populate_assumptions` (called from `planning.rs:~1707`) populates `frame.assumptions: Vec<FrameAssumption>` from the active belief view (`refreshed_view`) at `tick`. The reorder must keep `populate_assumptions` reading the same `refreshed_view` snapshot and tick — no new belief queries (preserves spec design goal #2).
3. `IntentionFrame.assumptions` lives at `crates/worldwake-core/src/intention_frame.rs:145` as `pub assumptions: Vec<FrameAssumption>`. Failure-path emission sites have access to the active frame via the runtime's intention-frame reference. The converter from `Vec<FrameAssumption>` to `Vec<PlanAssumptionRef>` adds `introduced_at_step`. **Verify at implementation time**: whether `populate_assumptions` already records the introducing step (e.g., via the plan's step index when the assumption was synthesized) or whether step provenance must be added in this ticket. If the source data does not currently carry step provenance, populate `introduced_at_step: 0` for all entries in this ticket and open a follow-up traceability ticket to thread step provenance through `populate_assumptions`. Cite the follow-up ticket ID in the converter's doc-comment.
4. Existing test `emit_plan_selection_events_records_commit_then_adoption_with_truncation` (`planning.rs:3464`) covers the truncation behavior of `rejected_alternatives`. This ticket extends that test (or adds a sibling) to assert `assumptions` is populated on the produced `GoalCommittedPayload` and `PlanAdoptedPayload` when the agent has an active intention frame.
5. Boundary under audit: the success-path emission ordering. Compared branches: pre-reorder (frame empty at emission) vs. post-reorder (frame populated). Divergence is purely in payload content — no behavioral change in plan adoption, plan selection, or downstream commit logic.

## Architecture Check

1. The reorder is a structural-only change to `emit_plan_selection_events`'s caller flow; no behavior change at the agent or simulation level. The frame's `assumptions` field is still populated by the same `populate_assumptions` call against the same `refreshed_view` and tick — only the call order changes.
2. Wiring `frame.assumptions` into the payload preserves FND-29A: the always-on event log now carries the load-bearing assumption list at the moment of plan commit/adoption, recoverable on replay without `enable_tracing()`.
3. No new `FrameAssumption` variants. No new computation. No new SystemFn. No new authoritative state.
4. Failure-path wire-up reuses the active frame's assumptions at sites where the frame already exists — no ordering change needed there, just the field read at emission.

## Verification Layers

1. Reorder coherence → extended `emit_plan_selection_events_records_commit_then_adoption_with_truncation:3464` asserts `assumptions` populated on success-path payloads.
2. Failure-path `assumptions` populated → focused unit per failure tag (`BlockerRecorded`, `ReplanTriggered`, `ExpectationMismatch`); ticket 006 adds golden coverage.
3. No behavioral regression → existing planning suite passes (`cargo test -p worldwake-ai planning::`).
4. Same belief-view input → `populate_assumptions`'s arguments unchanged (verified by reading the call site post-reorder).

## What to Change

### 1. Reorder in `emit_plan_selection_events`'s callers

In `crates/worldwake-ai/src/agent_tick/planning.rs` around the contested-commit caller (`~1690`) and the secondary caller (`~2083`), restructure the call sequence:

```rust
// Build prepared frame and populate assumptions FIRST.
let mut prepared_frame = update_frame_for_adopted_plan(jc.as_ref(), &selected_plan, tick, runtime);
if let Some(frame) = prepared_frame.as_mut() {
    let completion_tick = plan_completion_tick_for_adoption(&selected_plan, tick);
    frame.assumptions = populate_assumptions(frame, agent, &refreshed_view, tick, completion_tick);
}

// Then emit, passing the prepared frame so payloads carry assumptions.
emit_plan_selection_events(
    event_log,
    tick,
    agent,
    ranked_candidates,
    &plans.portfolio,
    active_goal_key,
    &selected_plan,
    cognitive.decision_history_alternatives,
    prepared_frame.as_ref(),  // NEW: frame ref for assumptions
);

// Caller continues with prepared_frame as before — no other state mutation moved.
```

Update `emit_plan_selection_events`'s signature to accept `Option<&IntentionFrame>`; thread `frame.map(|f| &f.assumptions[..]).unwrap_or(&[])` through to the converter for both `GoalCommittedPayload.assumptions` and `PlanAdoptedPayload.assumptions` constructions.

### 2. `FrameAssumption` → `PlanAssumptionRef` converter

Define a helper `assumptions_to_refs(&[FrameAssumption], cap: usize) -> Vec<PlanAssumptionRef>` in `crates/worldwake-ai/src/agent_tick/planning.rs` (or a sibling submodule if it grows). Cap the output at `cognitive.decision_history_alternatives` (matching the existing `rejected_alternatives` cap discipline at `planning.rs:991`).

`introduced_at_step` source: per Assumption Reassessment item 3, if `populate_assumptions` does not currently carry step provenance, populate `introduced_at_step: 0` and document the limitation in a doc-comment naming the follow-up traceability ticket.

### 3. Failure-path emission site wire-up

Replace each `assumptions: Vec::new()` placeholder from ticket 001 with `assumptions: assumptions_to_refs(&frame.assumptions, cap)` at:

- `crates/worldwake-ai/src/agent_tick/execution.rs:140, 222` — `ReplanTriggered` emissions.
- `crates/worldwake-ai/src/agent_tick/execution.rs:448, 503` — `BlockerRecorded` emissions.
- `crates/worldwake-ai/src/agent_tick/observation.rs:123` — `ExpectationMismatch` emission.
- `crates/worldwake-ai/src/agent_tick/mod.rs:497` — `ReplanTriggered` emission.

Each site has the active frame in scope via the runtime's intention-frame reference; verify the exact accessor at implementation time.

`SourceExpectationFailurePayload` (`mod.rs:621`) has no `assumptions` field per spec D4 — leave that emission's payload construction unchanged.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — reorder, signature change, success-path wiring, converter helper)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — `BlockerRecorded` and `ReplanTriggered` `assumptions` wire-up)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — `ExpectationMismatch` `assumptions` wire-up)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — `ReplanTriggered:497` `assumptions` wire-up)

## Out of Scope

- Populating `decisive_*` fields (ticket 004 owns these at the same emission sites).
- Populating `rejection_dimension` (ticket 003).
- Threading step provenance through `populate_assumptions` — deferred follow-up if `introduced_at_step: 0` is insufficient long-term.
- `SourceExpectationFailurePayload` does NOT gain `assumptions` (spec D4).
- Observer Section 3 rendering (ticket 005).
- Golden coverage (ticket 006).

## Acceptance Criteria

### Tests That Must Pass

1. Existing test `emit_plan_selection_events_records_commit_then_adoption_with_truncation:3464` extended to assert `GoalCommittedPayload.assumptions` and `PlanAdoptedPayload.assumptions` are non-empty when the agent has an active intention frame.
2. New focused tests per failure tag (one each for `BlockerRecorded`, `ReplanTriggered`, `ExpectationMismatch`) asserting `assumptions` carries the active frame's assumption set.
3. Existing planning suite passes: `cargo test -p worldwake-ai planning::`.
4. Existing agent_tick suite passes: `cargo test -p worldwake-ai agent_tick::`.

### Invariants

1. The reorder is structural-only — no observable behavior change in plan adoption, plan selection, or downstream commit logic.
2. `assumptions` field on emitted payloads exactly mirrors the active frame's `assumptions` at emission time, post-cap.
3. `populate_assumptions` reads from the same `refreshed_view` and tick as before the reorder.
4. Vec is bounded by `cognitive.decision_history_alternatives`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs::tests::emit_plan_selection_events_records_commit_then_adoption_with_truncation` — extend with `assumptions` assertion.
2. `crates/worldwake-ai/src/agent_tick/execution.rs::tests` — new focused units for `BlockerRecorded` and `ReplanTriggered` `assumptions` wire-up.
3. `crates/worldwake-ai/src/agent_tick/observation.rs::tests` — new focused unit for `ExpectationMismatch` `assumptions` wire-up.

### Commands

1. `cargo test -p worldwake-ai planning::tests::emit_plan_selection_events_records_commit_then_adoption_with_truncation`
2. `cargo test -p worldwake-ai agent_tick::`
3. `cargo test -p worldwake-ai`
4. `./scripts/verify.sh`
