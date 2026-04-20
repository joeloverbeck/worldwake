# S110DECHISEVE-004: Foundations-honest decision event emission slice

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `agent_tick` and event-log persistence paths emit the decision events whose causes are already concrete in the live runtime
**Deps**: archive/tickets/S110DECHISEVE-002.md (`EventTag` variants, `DecisionEventPayload`, and `EventPayload::decision_payload` field must exist), archive/tickets/S110DECHISEVE-003.md (`CognitiveProfile::decision_history_alternatives` must exist for `GoalCommittedPayload` truncation)

## Problem

Tickets 002 and 003 landed the S110 schema, but the original 004 draft assumed the AI runtime already exposed authoritative payload inputs for all eleven decision-event variants. Reassessment against the live code shows that is false: some decision reasons are still implicit inside helper layers or traces and cannot yet be emitted as authoritative world history without inventing new semantics. Per `docs/FOUNDATIONS.md` FND-3, FND-27, FND-29, FND-29A, and FND-30, this ticket must emit only the decision events the current runtime can prove from concrete state and live decision seams, and must defer the rest to explicit follow-up tickets instead of synthesizing guessed history.

## Assumption Reassessment (2026-04-20)

1. The shared abstraction boundary under audit is the AI crate's authoritative `EventLog` write path, specifically the `agent_tick` orchestration and the blocker/discrepancy persistence helpers in `crates/worldwake-ai/src/agent_tick/{mod,planning,execution}.rs`. Those layers already own `ctx.event_log` or the `event_log` persistence parameter.
2. The original draft's file ownership was materially wrong. `crates/worldwake-ai/src/candidate_generation.rs:4131` (`emit_candidate_with_trace`) and `crates/worldwake-ai/src/ranking.rs:108` (`rank_candidates_with_memories`) are pure helper layers today; they do not own `EventLog`. `crates/worldwake-ai/src/plan_revalidation.rs:14` (`revalidate_next_step`) is also a pure validator that returns `bool` and does not write history.
3. `GoalCommitted` and `PlanAdopted` have an honest live seam in `crates/worldwake-ai/src/agent_tick/planning.rs`, where selected plans are chosen and adopted. The commit path already has the selected ranked goal, the losing ranked alternatives, the selected plan, the tick, and the active goal transition in one place.
4. `BlockerRecorded` has an honest live seam in `crates/worldwake-ai/src/agent_tick/execution.rs::{persist_blocked_memory,persist_discrepancy_memory}`. Those functions already diff authoritative component state and commit the updated memory to the world. Emitting from the persistence seam preserves FND-26 and avoids duplicating record logic across callers such as `handle_plan_failure`, patience exhaustion, and assumption-failure recording.
5. `ExpectationMismatch` has an honest live seam in `crates/worldwake-ai/src/agent_tick/observation.rs`, where `apply_step_materialization_bindings(runtime, &step, &committed_action.outcome)` returns `Err(())` on explicit `expected_materializations` mismatch. That call site has the agent, tick, current goal, runtime step index, step payload, and `ctx.event_log`.
6. The original draft's `GoalSuppressed` assumption was wrong in two separate ways. Blocker/discrepancy suppression currently happens in `crates/worldwake-ai/src/candidate_generation.rs::{filter_suppressed_candidates,find_matching_suppression}`, not in ranking, while `ranking.rs` suppression is stress-policy suppression from `crates/worldwake-ai/src/goal_policy.rs::evaluate_suppression`. The core payload enum `GoalRejectionReason` does not currently contain a generic stress-policy suppression variant, so a foundations-honest `GoalSuppressed` implementation needs follow-up schema/runtime work.
7. The original draft's `PlanInvalidated`, `GoalSuspended`, `GoalAbandoned`, `RepairApplied`, and fully-typed `ReplanTriggered` assumptions were also overstated. The live runtime has partial local reasons (`PursuitInvalidationReason`, `AssumptionEvalResult`, repair-memory recording, `DirtySet::REPLAN_SIGNAL`), but those do not yet map one-to-one onto the core S110 payload enums at an authoritative write seam without adding new plumbing.
8. No failing golden motivates this ticket. This is proactive FND-29 / FND-29A instrumentation work. Because the corrected slice changes only record emission, not planner arithmetic or action legality, the strongest proof surfaces are focused unit/runtime coverage plus one golden integration check for same-tick event ordering.
9. This remains a runtime `agent_tick` ticket, and full action registries are required for the integration proof because expectation mismatch and blocker persistence are tied to active action execution, not just needs-only candidate generation.
10. Ordering under audit is event-log insertion order within a single tick. For the corrected slice, the relevant live ordering claim is `GoalCommitted` before `PlanAdopted` during a selection/adoption pass in `agent_tick/planning.rs`. The branches are asymmetric and share one call site, so the ordering proof is event-log order, not a derived observer assertion.
11. Adjacent contradictions from reassessment are future cleanup that must become their own tickets, not hidden expansion of this one: candidate-offer / suppression provenance, invalidation / suspension / abandonment reason transport, and repair / richer replan reason transport.
12. `rejected_alternatives` truncation remains valid on the corrected scope. At emission time in `agent_tick/planning.rs`, sort non-selected ranked alternatives by `(motive_score desc, goal_key asc)` and take the first `cognitive.decision_history_alternatives as usize`, preserving deterministic tie-breaking.

## Architecture Check

1. Emitting only from seams that already own authoritative runtime state is cleaner than retrofitting a speculative "decision bus" or synthesizing payloads from traces. It keeps the event log honest under FND-29A: every emitted payload corresponds to a decision the live runtime actually made at that write boundary.
2. Emitting `BlockerRecorded` from the memory-persistence seam is architecturally stronger than sprinkling emissions across every blocker/discrepancy call site. The authoritative fact is the committed memory delta, not the intermediate local helper call.
3. No backwards-compatibility shim or duplicate authority path is introduced. This ticket narrows scope rather than inventing an alias payload or fallback reason.

## Verification Layers

1. `GoalCommitted` payload correctness and alternative truncation -> focused `agent_tick/planning.rs` unit test inspecting emitted event payloads.
2. `PlanAdopted` same-tick ordering after commit -> focused `agent_tick/planning.rs` unit/runtime test asserting `EventLog::events_at_tick(t)` order.
3. `ExpectationMismatch` fires only on explicit materialization drift -> focused `agent_tick` runtime test around `apply_step_materialization_bindings` failure and emitted event payload.
4. `BlockerRecorded` reflects committed memory-class deltas, not helper-local guesses -> focused `agent_tick/execution.rs` tests for `persist_blocked_memory` and `persist_discrepancy_memory`.
5. Additive runtime instrumentation does not regress AI behavior -> targeted golden/integration coverage in `crates/worldwake-ai/tests/` plus full `cargo test -p worldwake-ai`.
6. Single-layer caveat: the deferred S110 variants are intentionally not proven here because the live runtime does not yet expose foundations-honest payload causes for them.

## What to Change

### 1. Emit `GoalCommitted` and `PlanAdopted` from `agent_tick/planning.rs`

At the selected-plan adoption seam:

- emit `EventTag::GoalCommitted` with `DecisionEventPayload::GoalCommitted(GoalCommittedPayload { agent, goal_key, motive_score, rejected_alternatives })`
- emit `EventTag::PlanAdopted` with `DecisionEventPayload::PlanAdopted(PlanAdoptedPayload { agent, goal_key, plan_step_count })`
- build `rejected_alternatives` from the ranked non-selected candidates using deterministic `(motive_score desc, goal_key asc)` ordering and profile-driven truncation
- use `CauseRef::SystemTick(tick)`, `actor_id: Some(agent)`, empty state-delta/evidence payloads, and tag-only classification for these record-only events

### 2. Emit `ExpectationMismatch` at the materialization-binding failure seam

At the `apply_step_materialization_bindings(...).is_err()` branch in `agent_tick/observation.rs`:

- emit `EventTag::ExpectationMismatch` with `ExpectationMismatchPayload { agent, goal_key, step_index, expected_materializations }`
- derive `step_index` from `runtime.current_step_index`
- derive `expected_materializations` by projecting `.tag` from `step.expected_materializations`
- keep this strictly pre-S114: only explicit materialization drift is in scope

### 3. Emit `BlockerRecorded` from committed memory deltas

Extend `persist_blocked_memory` and `persist_discrepancy_memory` in `agent_tick/execution.rs` so that when a new blocker/discrepancy entry is newly committed relative to the `before` snapshot, the function also emits:

- `EventTag::BlockerRecorded`
- `DecisionEventPayload::BlockerRecorded(BlockerRecordedPayload { agent, blocker_key, discrepancy, blocking_fact, expires_tick })`

Exactly one of `discrepancy` / `blocking_fact` must be `Some`, matching the committed memory class.

### 4. Add one integration proof over a real golden scenario

Add a focused integration test in `crates/worldwake-ai/tests/` that runs `survival-baseline.ron` for a small deterministic tick window and asserts:

- at least one `GoalCommitted` and `PlanAdopted` event exist
- at least one `BlockerRecorded` or `ExpectationMismatch` event exists if the scenario naturally reaches that branch during the chosen window; if not, keep those proofs at focused unit/runtime level instead of forcing the scenario
- same-tick `GoalCommitted` precedes `PlanAdopted`

### 5. Record the deferred remainder as explicit follow-up tickets

Create follow-up tickets for:

- `GoalOffered` / `GoalSuppressed` candidate provenance
- `PlanInvalidated` / `GoalSuspended` / `GoalAbandoned` / richer `ReplanTriggered` reason transport
- `RepairApplied`

These are not optional cleanup; they are the required next steps once the runtime exposes authoritative payload causes.

## Files to Touch

- `tickets/S110DECHISEVE-004.md` (modify — corrected scope and proof surface)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — `GoalCommitted` and `PlanAdopted` emission)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — `ExpectationMismatch` emission on materialization drift)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — `BlockerRecorded` emission from committed memory deltas)
- `crates/worldwake-ai/tests/golden_decision_history_events.rs` (new — focused integration proof)

## Out of Scope

- `GoalOffered` and `GoalSuppressed`. These need explicit candidate-emitter and suppression-reason transport that the live runtime does not yet expose authoritatively.
- `PlanInvalidated`, `GoalSuspended`, `GoalAbandoned`, and richer `ReplanTriggered` reasons. These need new runtime reason plumbing from invalidation and frame-transition seams.
- `RepairApplied`. The live runtime currently records successful alternate-path outcomes via repair memory, not a dedicated repair-application seam carrying `RepairKind`.
- S114 widening of `ExpectationMismatch`.
- Observer rendering and replay invariance; those remain with tickets 005 and 006 after the emission family is complete.

## Acceptance Criteria

### Tests That Must Pass

1. Focused planning test proves one `GoalCommitted` event and one same-tick `PlanAdopted` event with correct payloads and ordering.
2. Focused runtime test proves materialization drift emits exactly one `ExpectationMismatch` event with the expected tag list and step index.
3. Focused persistence tests prove `persist_blocked_memory` and `persist_discrepancy_memory` emit `BlockerRecorded` with correct memory-class routing.
4. New integration test proves real AI runtime emission of `GoalCommitted` / `PlanAdopted` in an existing deterministic scenario.
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo test --workspace`
7. Existing lint gate: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Every emitted payload in this ticket is backed by a concrete live runtime decision or committed memory delta, not inferred from optional trace output.
2. `GoalCommittedPayload::rejected_alternatives.len() <= cognitive.decision_history_alternatives as usize` and is deterministically ordered by `(motive_score desc, goal_key asc)`.
3. `BlockerRecordedPayload` always has exactly one of `discrepancy` / `blocking_fact` populated.
4. This ticket does not change planner choice, action legality, or authoritative world mutation semantics; it only appends new authoritative history records.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` — prove `GoalCommitted` payload contents, truncation, and same-tick `PlanAdopted` ordering.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` or another focused `agent_tick` runtime test surface — prove `ExpectationMismatch` emission on explicit materialization drift.
3. `crates/worldwake-ai/src/agent_tick/execution.rs` — prove `BlockerRecorded` emission for blocker-memory and discrepancy-memory persistence.
4. `crates/worldwake-ai/tests/golden_decision_history_events.rs` — prove real scenario-level `GoalCommitted` / `PlanAdopted` emission and ordering.

### Commands

1. `cargo test -p worldwake-ai agent_tick::planning`
2. `cargo test -p worldwake-ai golden_decision_history_events`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-20. The ticket landed the foundations-honest runtime slice only: `GoalCommitted` and `PlanAdopted` now emit from the selected-plan adoption seam in `agent_tick/planning.rs`, `ExpectationMismatch` emits from explicit materialization-binding failure in `agent_tick/observation.rs`, and `BlockerRecorded` emits from committed blocker/discrepancy memory deltas in `agent_tick/execution.rs`. The shared `emit_decision_event` helper in `agent_tick/mod.rs` keeps the record-only event shape consistent across these seams.

The integration proof landed in `crates/worldwake-ai/tests/golden_decision_history_events.rs` and confirms real scenario-level `GoalCommitted` / `PlanAdopted` emission plus same-tick ordering. Focused unit/runtime tests cover rejected-alternative truncation and ordering, explicit expectation-mismatch payloads, and blocker/discrepancy persistence emission. Per reassessment, the unsupported remainder was deferred into explicit follow-up tickets `tickets/S110DECHISEVE-007.md`, `tickets/S110DECHISEVE-008.md`, and `tickets/S110DECHISEVE-009.md` rather than emitted through inferred or lossy mappings.

## Verification Result

Passed on 2026-04-20:

1. `cargo test -p worldwake-ai agent_tick::planning`
2. `cargo test -p worldwake-ai golden_decision_history_events`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
