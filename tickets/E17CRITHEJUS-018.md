# E17CRITHEJUS-018: Add artifact-level Tell traceability and belief-delta diagnostics

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — decision-trace and action-trace extensions across AI, sim, and tell runtime/tests
**Deps**: E17CRITHEJUS-016

## Problem

The current trace surfaces prove that Tell candidates and Tell actions exist, but they stop one layer short of the debugging questions that actually mattered during `E17CRITHEJUS-016`. They do not expose a structured reason for why a concrete Tell topic was relayable, suppressed, accepted, rejected as redundant, or reduced to a no-op at commit time. That forces debugging back onto ad-hoc state inspection and broad golden outcomes when the architecture should explain conversational causality directly.

For Worldwake this is not a cosmetic tooling issue. Tell is how concrete belief artifacts travel physically between agents. If the trace surface cannot explain why a social or institutional artifact did or did not move, the simulation becomes harder to audit against P7, P13, P16, and P27.

## Assumption Reassessment (2026-03-26)

1. AI decision traces already expose one pre-emission social omission surface. `crates/worldwake-ai/src/decision_trace.rs` defines `SocialCandidateOmission { listener, topic, status }`, and existing focused coverage includes `agent_tick::tests::trace_social_resend_omission_reason` plus `decision_trace::tests::goal_status_reports_social_omission_reason`.
2. That current omission trace is too coarse for the recent failure mode. It records only `RecipientKnowledgeStatus`, not why the topic was omitted because of direct observability, participant redundancy, relay-depth filtering, candidate-limit truncation, or later acceptance/no-op outcomes.
3. Action traces already record Tell payload identity at lifecycle boundaries. `crates/worldwake-sim/src/action_trace.rs` stores `ActionTraceDetail::Tell { listener, topic }`, and `ActionTraceKind::Committed` only carries the generic `CommitOutcome`.
4. The authoritative Tell handler in `crates/worldwake-systems/src/tell_actions.rs` computes richer results than traces expose: it distinguishes `HeardBeliefDisposition::{Accepted, AlreadyHeldEqualOrNewer, NotInternalized}`, rechecks relay limits, degrades provenance, and decides whether any concrete artifact actually changed in listener state.
5. Existing focused Tell tests prove the authoritative state changes but not their traceability surface. Verified by `cargo test -p worldwake-systems tell -- --list`, which includes `tell_commit_records_listener_heard_belief_with_accepted_disposition`, `tell_commit_records_listener_heard_belief_with_already_held_equal_or_newer`, and `tell_commit_records_listener_heard_belief_with_not_internalized`.
6. Existing golden/AI coverage verifies that traces can explain some Tell behavior, but still indirectly. Verified by `cargo test -p worldwake-ai -- --list`, which includes `golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry`, `golden_tell_propagates_political_knowledge`, and `golden_agent_does_not_repeat_same_unchanged_tell_to_same_listener`.
7. The missing traceability is therefore a mixed gap:
   - focused trace-model gap in AI/sim
   - focused authoritative runtime gap for exporting Tell commit semantics into traces
   - no golden/E2E requirement yet for artifact-level Tell delta summaries
8. This aligns with `docs/FOUNDATIONS.md`: P7/P13 require explicit information-path reasoning, P16 requires memories/evidence/records to remain concrete state, and P27 requires causal chains to be inspectable without ad-hoc debugging.
9. Mismatch: the current trace system is not “missing entirely,” but it is incomplete for Tell debugging. The clean scope is to extend the trace surfaces to artifact-level conversational causality rather than compensating with broader downstream assertions.

## Architecture Check

1. The clean fix is to extend existing structured traces, not add ad-hoc logging. Tell debugging should use the same opt-in, typed, zero-cost-when-disabled pattern as the rest of the trace system.
2. This is better than leaning harder on golden assertions or raw state dumps, because the missing provenance is at the candidate/commit boundary itself, not only in downstream world state.
3. The trace payload should remain artifact-first: topic identity, omission/suppression reason, acceptance result, and belief delta summary. Do not reduce it to vague “Tell succeeded/failed” strings.
4. No backwards-compatibility aliasing. Replace coarse generic Tell summaries where necessary instead of adding an unstructured secondary debug path.

## Verification Layers

1. Candidate omission reason for each skipped Tell topic -> focused decision-trace tests in `candidate_generation.rs` / `agent_tick::tests`
2. Action lifecycle still records Tell start/commit/abort ordering -> existing action-trace tests plus focused Tell trace additions
3. Commit-time Tell result exposes accepted/redundant/not-internalized semantics and artifact delta summary -> focused `tell_actions.rs` + `action_trace.rs` tests
4. Golden scenarios can explain why a Tell did or did not change listener knowledge without ad-hoc inspection -> targeted golden trace assertions
5. Additional broader layer mapping is applicable here because the current problem specifically spans candidate-generation reasoning and authoritative commit mutation

## What to Change

### 1. Extend social omission tracing to structured Tell omission reasons

Replace or extend the current `SocialCandidateOmission` payload so Tell omissions record the actual reason category, not just `RecipientKnowledgeStatus`.

Expected coverage includes:
- already told current content
- stale tell memory vs changed content re-enabled
- direct-observability suppression
- participant redundancy for social observations
- relay-depth exclusion
- max-candidate truncation if a topic lost before emission

The exact enum names may change during implementation, but the omission reason must be explicit and typed.

### 2. Add Tell commit-result trace detail to action traces

Extend `ActionTraceDetail` or `ActionTraceKind::Committed` with Tell-specific result diagnostics that summarize:
- listener
- topic
- heard disposition
- whether any listener belief artifact changed
- coarse artifact delta kind (entity belief updated, social observation internalized, institutional claim internalized, no-op)

Do not stuff this into free-form strings only.

### 3. Export authoritative Tell semantics into the trace layer

Refactor `commit_tell()` so the trace sink can observe the same lawful outcomes the handler already computes without recomputing them downstream.

The trace contract should explain:
- why commit became a no-op
- which branch ran (`EntityBelief`, `SocialObservation`, later institutional topic once present)
- whether relay-limit revalidation prevented transfer
- whether acceptance fidelity blocked internalization

### 4. Add focused and golden trace assertions

Strengthen tests so the trace surface itself becomes part of the debugging contract. The new tests should prove that the system can explain the specific classes of Tell failures that previously required manual state inspection.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-sim/src/action_trace.rs` (modify)
- `crates/worldwake-sim/src/tick_step.rs` (modify if trace event construction needs richer Tell detail)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-ai/tests/golden_social.rs` or `crates/worldwake-ai/tests/golden_emergent.rs` (modify)

## Out of Scope

- First-class institutional Tell topics themselves (`E17CRITHEJUS-017`)
- Non-Tell traceability redesign across unrelated domains
- UI/debugger presentation work outside the existing trace sinks

## Acceptance Criteria

### Tests That Must Pass

1. Decision trace records explicit omission reasons for skipped Tell topics beyond raw `RecipientKnowledgeStatus`
2. Action trace for committed Tell exposes structured commit-result detail, including heard disposition and whether a belief artifact changed
3. Tell commit no-op caused by equal/newer listener knowledge is visible in the trace without reading belief state manually
4. Tell commit non-internalization caused by acceptance fidelity is visible in the trace
5. At least one golden trace assertion proves a Tell behavior can now be explained from traces alone
6. Existing suite: `cargo test -p worldwake-systems tell -- --list`
7. Existing suite: `cargo test -p worldwake-ai -- --list`
8. Existing suite: `cargo test -p worldwake-systems tell`
9. Existing suite: `cargo test -p worldwake-ai`
10. Existing suite: `cargo build --workspace`
11. Existing suite: `cargo clippy --workspace`

### Invariants

1. Tell trace data remains a derived debug view over concrete conversational state transitions, never a source of truth (P25)
2. Traceability must expose how information traveled and why it stopped, without replacing local-causality rules with omniscient shortcuts (P7, P13)
3. The recorded artifact summaries must refer to concrete belief artifacts and dispositions, not abstract “confidence” or “interestingness” scores (P3, P16)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — omission-reason coverage for direct-observability, resend suppression, and topic re-enable paths
2. `crates/worldwake-ai/src/decision_trace.rs` — formatting and storage tests for the richer Tell omission/result trace payloads
3. `crates/worldwake-sim/src/action_trace.rs` — typed Tell commit-result trace serialization/summary tests
4. `crates/worldwake-systems/src/tell_actions.rs` — commit-result trace assertions for accepted, redundant, and not-internalized Tell outcomes
5. `crates/worldwake-ai/tests/golden_social.rs` or `crates/worldwake-ai/tests/golden_emergent.rs` — one trace-driven regression proving the new artifact-level diagnostics are sufficient

### Commands

1. `cargo test -p worldwake-ai agent_tick::tests::trace_social_resend_omission_reason -- --exact`
2. `cargo test -p worldwake-ai decision_trace::tests::goal_status_reports_social_omission_reason -- --exact`
3. `cargo test -p worldwake-systems tell_commit_records_listener_heard_belief_with_accepted_disposition -- --exact`
4. `cargo test -p worldwake-ai golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry -- --exact`
5. `cargo test -p worldwake-systems tell`
6. `cargo test -p worldwake-ai`
7. `cargo build --workspace`
8. `cargo clippy --workspace`
