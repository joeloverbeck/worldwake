# E17CRITHEJUS-018: Add artifact-level Tell traceability and belief-delta diagnostics

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — decision-trace and action-trace extensions across AI, sim, and tell runtime/tests
**Deps**: `archive/tickets/completed/E17CRITHEJUS-016.md`

## Problem

The current trace surfaces prove that Tell candidates and Tell actions exist, but they stop one layer short of the debugging questions that actually mattered during `E17CRITHEJUS-016`. They do not expose a structured reason for why a concrete Tell topic was relayable, suppressed, accepted, rejected as redundant, or reduced to a no-op at commit time. That forces debugging back onto ad-hoc state inspection and broad golden outcomes when the architecture should explain conversational causality directly.

For Worldwake this is not a cosmetic tooling issue. Tell is how concrete belief artifacts travel physically between agents. If the trace surface cannot explain why a social or institutional artifact did or did not move, the simulation becomes harder to audit against P7, P13, P16, and P27.

## Assumption Reassessment (2026-03-26)

1. AI decision traces already expose one narrow pre-emission social omission surface. `crates/worldwake-ai/src/decision_trace.rs` defines `SocialCandidateOmission { listener, topic, status }`, and focused coverage exists in `agent_tick::tests::trace_social_resend_omission_reason` and `decision_trace::tests::goal_status_reports_social_omission_reason`.
2. That live omission surface only covers resend suppression. In `crates/worldwake-ai/src/candidate_generation.rs`, `diagnostics.omitted_social` is recorded only when a topic is filtered by `RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief`. Direct observability, participant redundancy, relay-depth filtering, non-relayable social observations, and candidate-limit truncation are handled earlier or deeper in selection and are not represented as structured omission reasons today.
3. The shared selection boundary lives below AI. `crates/worldwake-sim/src/social_relay.rs` owns `listener_aware_relayable_tell_topics(...)`, which currently filters only relay depth, non-relayable social observations, resend suppression, sorting, and truncation, but returns only the selected `Vec<TellTopic>` and no typed rejection diagnostics.
4. `crates/worldwake-systems/src/tell_actions.rs` duplicates the same selection shape for authoritative affordance enumeration: it prefilters direct-observability and participant redundancy at the call site, then calls `listener_aware_relayable_tell_topics(...)`. The current architecture therefore has no shared typed explanation of why a candidate topic was skipped across AI and action affordance generation.
5. Action traces already record Tell payload identity at lifecycle boundaries. `crates/worldwake-sim/src/action_trace.rs` stores `ActionTraceDetail::Tell { listener, topic }`, while `ActionTraceKind::Committed` still exposes only the generic `CommitOutcome`.
6. The authoritative Tell handler in `crates/worldwake-systems/src/tell_actions.rs` computes richer outcomes than action traces expose. `commit_tell()` distinguishes `HeardBeliefDisposition::{Accepted, AlreadyHeldEqualOrNewer, NotInternalized}`, rechecks relay limits, degrades provenance, projects institutional claims, and decides whether any concrete listener artifact changed.
7. Existing focused Tell tests prove authoritative state changes but not their traceability surface. Verified live by `cargo test -p worldwake-systems tell -- --list`, including `tell_commit_records_listener_heard_belief_with_accepted_disposition`, `tell_commit_records_listener_heard_belief_with_already_held_equal_or_newer`, `tell_commit_records_listener_heard_belief_with_not_internalized`, and `tell_commit_rechecks_relay_limit_against_current_belief`.
8. Existing golden/AI coverage proves only the current coarse trace contract. Verified live by `cargo test -p worldwake-ai -- --list`, including `golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry`, `golden_agent_does_not_repeat_same_unchanged_tell_to_same_listener`, and `golden_tell_propagates_political_knowledge`.
9. The missing traceability is therefore a mixed architectural gap:
   - shared selection diagnostics gap in `worldwake-sim` / Tell affordance generation
   - focused AI trace-model gap for exposing typed omission reasons instead of raw resend status
   - focused authoritative runtime gap for exporting Tell commit semantics into action traces
   - no current golden contract requiring traces alone to explain Tell no-op outcomes
10. This aligns with `docs/FOUNDATIONS.md`: P7/P13 require explicit information-path reasoning, P16 requires memories/evidence/records to remain concrete state, P24 prefers a shared state-mediated seam over duplicated per-layer logic, and P27 requires causal chains to be inspectable without ad-hoc debugging.
11. Mismatch: the current trace system is not “missing entirely,” but the ticket also overstated what the live omission surface already tracks. The clean scope is to add a shared typed Tell-topic selection diagnostic boundary plus Tell commit-result trace data, rather than bolting more ad-hoc assertions onto AI-only or golden-only tests.

## Architecture Check

1. The clean fix is to extend existing structured traces and the shared selection seam, not add ad-hoc logging. Tell debugging should use the same typed, opt-in, zero-cost-when-disabled pattern as the rest of the trace system.
2. This is better than leaning harder on golden assertions or raw state dumps, because the missing provenance sits at two exact boundaries: topic selection before emission and authoritative commit semantics at completion.
3. The clean architecture is to move omission categorization into the shared Tell-topic selector in `worldwake-sim`, then let AI traces consume that typed result instead of re-deriving partial reasons in `worldwake-ai`.
4. Commit-time diagnostics should flow out of `commit_tell()` through a typed trace field on the authoritative commit result, not be recomputed later from mutated belief state. That keeps traces aligned with the real branch that executed.
5. The trace payload should remain artifact-first: topic identity, typed omission reason, heard disposition, and concrete belief-delta summary. Do not reduce it to vague “Tell succeeded/failed” strings.
6. No backwards-compatibility aliasing. Replace coarse generic Tell summaries where necessary instead of adding a parallel debug-only path with its own semantics.

## Verification Layers

1. Shared Tell-topic selection explains why a topic was selected or omitted -> focused `social_relay.rs`, `candidate_generation.rs`, and `tell_actions.rs` tests
2. Decision trace records the typed omission reason for skipped Tell goals -> focused `decision_trace.rs` / `agent_tick::tests`
3. Action lifecycle still records Tell start/commit/abort ordering -> existing action-trace tests plus focused Tell trace additions
4. Commit-time Tell result exposes accepted/redundant/not-internalized semantics and concrete artifact delta summary -> focused `tell_actions.rs` + `action_trace.rs` tests
5. Golden scenario can explain a Tell no-op or changed-knowledge result from traces alone without manual belief-store inspection -> targeted golden trace assertion

## What to Change

### 1. Add a shared typed Tell-topic selection diagnostic surface

Replace the current “selected topics only” seam in `crates/worldwake-sim/src/social_relay.rs` with a typed selection result that can report why each candidate topic was omitted.

Expected omission coverage includes:
- already told current content
- direct-observability suppression for entity beliefs
- participant redundancy for social observations
- non-relayable social observation kinds
- relay-depth exclusion
- max-candidate truncation after ranking/sorting

The exact enum names may change during implementation, but the omission reason must be explicit, typed, and reusable by both AI candidate generation and authoritative Tell affordance enumeration.

### 2. Add Tell commit-result trace detail to action traces

Extend the authoritative commit result / action-trace boundary so committed Tell events can carry Tell-specific diagnostics summarizing:
- listener
- topic
- heard disposition
- whether any listener belief artifact changed
- coarse artifact delta kind (entity belief updated, social observation internalized, institutional claim internalized, mixed update, no-op)

Do not stuff this into free-form strings only. The typed detail should be emitted by `commit_tell()` and consumed by `tick_step` when constructing the committed action trace.

### 3. Export authoritative Tell semantics into the trace layer

The trace contract should explain:
- why commit became a no-op
- which branch ran (`EntityBelief`, `SocialObservation`, later institutional topic once present)
- whether relay-limit revalidation prevented transfer
- whether acceptance fidelity blocked internalization

Refactor `commit_tell()` so the trace sink can observe the same lawful outcomes the handler already computes without recomputing them downstream.

### 4. Add focused and golden trace assertions

Strengthen tests so the trace surface itself becomes part of the debugging contract. The new tests should prove that the system can explain the specific classes of Tell failures that previously required manual state inspection.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-sim/src/social_relay.rs` (modify)
- `crates/worldwake-sim/src/action_handler.rs` (modify if commit diagnostics are carried on `CommitOutcome`)
- `crates/worldwake-sim/src/action_trace.rs` (modify)
- `crates/worldwake-sim/src/tick_step.rs` (modify if trace event construction needs richer Tell detail)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-ai/tests/golden_social.rs` (modify)

## Out of Scope

- First-class institutional Tell topics themselves (`E17CRITHEJUS-017`)
- Non-Tell traceability redesign across unrelated domains
- UI/debugger presentation work outside the existing trace sinks

## Acceptance Criteria

### Tests That Must Pass

1. Shared Tell-topic selection records explicit omission reasons for skipped Tell topics beyond raw `RecipientKnowledgeStatus`
2. Decision trace maps omitted Tell goals to those typed omission reasons
3. Action trace for committed Tell exposes structured commit-result detail, including heard disposition and whether a belief artifact changed
4. Tell commit no-op caused by equal/newer listener knowledge is visible in the trace without reading belief state manually
5. Tell commit non-internalization caused by acceptance fidelity is visible in the trace
6. At least one golden trace assertion proves a Tell behavior can now be explained from traces alone
7. Existing suite: `cargo test -p worldwake-systems tell -- --list`
8. Existing suite: `cargo test -p worldwake-ai -- --list`
9. Existing suite: `cargo test -p worldwake-systems tell`
10. Existing suite: `cargo test -p worldwake-ai`
11. Existing suite: `cargo build --workspace`
12. Existing suite: `cargo clippy --workspace`

### Invariants

1. Tell trace data remains a derived debug view over concrete conversational state transitions, never a source of truth (P25)
2. Traceability must expose how information traveled and why it stopped, without replacing local-causality rules with omniscient shortcuts (P7, P13)
3. The recorded artifact summaries must refer to concrete belief artifacts and dispositions, not abstract “confidence” or “interestingness” scores (P3, P16)
4. Shared topic-selection diagnostics must not fork Tell selection policy between AI and systems; both layers should consume the same typed omission categories from the same shared seam (P24)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/social_relay.rs` — typed Tell-topic selection diagnostics for direct-observability, participant redundancy, relay-depth, resend suppression, and truncation
2. `crates/worldwake-ai/src/candidate_generation.rs` — omission-reason coverage for direct-observability and resend suppression
3. `crates/worldwake-ai/src/decision_trace.rs` — formatting and storage tests for the richer Tell omission payloads
4. `crates/worldwake-sim/src/action_trace.rs` — typed Tell commit-result trace summary/serialization tests
5. `crates/worldwake-systems/src/tell_actions.rs` — commit-result trace assertions for accepted, redundant, not-internalized, and relay-limit Tell outcomes
6. `crates/worldwake-ai/tests/golden_social.rs` — one trace-driven regression proving the new artifact-level diagnostics are sufficient

### Commands

1. `cargo test -p worldwake-sim social_relay::tests::listener_aware_tell_topic_selection_reports_relay_filtering_reasons -- --exact`
2. `cargo test -p worldwake-ai decision_trace::tests::goal_status_reports_social_direct_observability_omission_reason -- --exact`
3. `cargo test -p worldwake-systems tell_commit_trace_reports_relay_limit_rejection -- --exact`
4. `cargo test -p worldwake-ai golden_skeptical_listener_rejects_told_belief -- --exact`
5. `cargo test -p worldwake-ai golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry -- --exact`
6. `cargo test -p worldwake-systems tell`
7. `cargo test -p worldwake-ai`
8. `cargo build --workspace`
9. `cargo clippy --workspace`

## Outcome

Completion date: 2026-03-26

What actually changed:

1. Added shared typed Tell-topic omission reasons in `worldwake-sim` and updated selection to return both selected topics and structured omissions for relay depth, resend suppression, non-relayable social observations, and truncation.
2. Updated AI social candidate generation and decision traces to record typed Tell omission reasons, including listener direct-observability and participant-redundancy prefilters.
3. Extended authoritative Tell commit results with structured Tell trace diagnostics carrying commit result kind, optional heard disposition, and concrete belief-delta kind.
4. Added focused runtime and trace coverage plus a golden skeptical-listener assertion proving the action trace alone explains a rejected Tell outcome.

Deviations from original plan:

1. The implementation kept direct-observability and participant-redundancy classification at the caller boundary, but standardized the omission-reason enum and the deeper selection diagnostics in the shared `worldwake-sim` seam instead of forcing all filtering into one monolithic helper.
2. Tell commit diagnostics were carried on `CommitOutcome` rather than a separate `ActionTraceKind` variant so `tick_step` could forward authoritative branch results without recomputation.

Verification results:

1. `cargo test -p worldwake-sim social_relay`
2. `cargo test -p worldwake-sim action_trace`
3. `cargo test -p worldwake-systems tell`
4. `cargo test -p worldwake-ai golden_skeptical_listener_rejects_told_belief -- --exact`
5. `cargo test -p worldwake-ai golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry -- --exact`
6. `cargo test -p worldwake-ai`
7. `cargo build --workspace`
8. `cargo clippy --workspace`
