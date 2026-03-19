# E15CCONMEMANDRECKNO-005: Social Candidate Resend And Diagnostics

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI social candidate generation and decision-trace omission diagnostics
**Deps**: `E15CCONMEMANDRECKNO-001`, `E15CCONMEMANDRECKNO-002`, `E15CCONMEMANDRECKNO-003`, `E15CCONMEMANDRECKNO-004`

## Problem

`crates/worldwake-ai/src/candidate_generation.rs::emit_social_candidates()` still suppresses `ShareBelief` by checking whether the speaker believes the subject is already at the current place. That shortcut is architecturally wrong and the current decision-trace model has no social omission reason analogous to political omissions, so developers cannot see whether a social candidate was skipped because it was already told, stale, or absent for some other reason.

## Assumption Reassessment (2026-03-19)

1. The current AI gap is still real and narrow: `crates/worldwake-ai/src/candidate_generation.rs::emit_social_candidates()` still suppresses `ShareBelief` through `belief.last_known_place == Some(place)` and still calls the raw `worldwake_sim::relayable_social_subjects(...)` helper instead of the listener-aware resend helper from ticket 003.
2. Existing focused coverage still locks in that obsolete behavior: `candidate_generation::tests::social_candidates_skip_subjects_already_known_to_be_colocated`.
3. `E15CCONMEMANDRECKNO-002` has already landed the actor-local conversation-memory read surfaces. Current runtime/planning boundaries already expose `told_belief_memory()` and `recipient_knowledge_status()` in `crates/worldwake-sim/src/per_agent_belief_view.rs`, `crates/worldwake-ai/src/planning_state.rs`, and `crates/worldwake-ai/src/planning_snapshot.rs`.
4. `E15CCONMEMANDRECKNO-003` is already completed and archived at `archive/tickets/completed/E15CCONMEMANDRECKNO-003-tell-affordance-resend-policy.md`. It introduced `worldwake_sim::listener_aware_relayable_subjects(...)` and switched authoritative tell-affordance enumeration to it. This ticket should reuse that helper rather than duplicate listener-aware resend filtering in AI.
5. `E15CCONMEMANDRECKNO-004` is also completed and archived at `archive/tickets/completed/E15CCONMEMANDRECKNO-004-tell-commit-participant-memory.md`. `crates/worldwake-systems/src/tell_actions.rs::commit_tell()` now writes speaker `told_beliefs` and listener `heard_beliefs`, so the AI resend gate has a live authoritative memory source to consume.
6. `crates/worldwake-core/src/belief.rs` already contains the share-equivalence and explanation substrate this ticket needs: `SharedBeliefSnapshot`, `share_equivalent(...)`, and `RecipientKnowledgeStatus::{UnknownToSpeaker, SpeakerHasAlreadyToldCurrentBelief, SpeakerHasOnlyToldStaleBelief, SpeakerPreviouslyToldButMemoryExpired}`.
7. `CandidateGenerationDiagnostics` currently contains only `omitted_political`, and `crates/worldwake-ai/src/decision_trace.rs` only supports `GoalTraceStatus::OmittedPolitical(...)`; there is still no social omission surface in decision traces.
8. Candidate-generation work here is runtime `agent_tick` reasoning, but the core resend gate remains a focused/unit concern. Local `candidate_generation.rs` tests are sufficient for the resend policy itself; a narrow `agent_tick` trace test is the right verification surface for trace plumbing. Full action registries are not required for either.
9. Existing acceptance criteria and test commands in this ticket are stale. `cargo test -p worldwake-ai -- --list` shows the real current focused tests include `candidate_generation::tests::social_candidates_emit_for_live_colocated_listeners_and_relayable_subjects`, `candidate_generation::tests::social_candidates_require_tell_profile_and_respect_blocked_memory`, and `decision_trace::tests::goal_status_distinguishes_omitted_suppressed_zero_motive_ranked_and_selected`; the ticket must name current or newly added real test names only.
10. Mismatch and correction: this ticket no longer needs to describe itself as landing new memory/query infrastructure. Its actual scope is narrower and cleaner: switch AI generation to the already-landed listener-aware resend substrate, add social omission diagnostics to traces, and rewrite the stale focused AI tests that still preserve the co-location shortcut.

## Architecture Check

1. Reusing the shared resend helper from ticket 003 is cleaner than duplicating listener-aware filtering in AI with slightly different truncation semantics.
2. Adding explicit social omission diagnostics to decision traces is cleaner than forcing developers to infer resend suppression from missing candidates.
3. No backwards-compatibility flag should preserve the co-location shortcut.
4. This change is more beneficial than the current architecture because the present AI path suppresses social behavior using a world-state proxy instead of explicit remembered interaction state, which violates locality, belief/truth separation, and debuggability.
5. The clean long-term architecture is one resend policy shared by authoritative tell affordances and AI candidate generation, with omission explainability carried in decision traces rather than rebuilt from scenario side effects.
6. Ticket 005 should assume tickets 003 and 004 have already landed rather than introducing fallback behavior for missing helper/query/write paths.

## Verification Layers

1. `ShareBelief` candidate emission/suppression follows actor-local told memory and listener-aware pre-truncation filtering -> focused unit tests in `candidate_generation.rs`
2. Social omission reasons appear in the trace model for omitted `ShareBelief` goals -> focused unit tests in `decision_trace.rs`
3. `agent_tick` preserves social omission diagnostics from candidate generation into the runtime decision trace -> focused runtime test in `agent_tick.rs`
4. We do not use event-log or action-trace side effects as a proxy for candidate-generation reasoning because the contract here is reasoning-layer omission.

## What to Change

### 1. Replace same-place suppression

Remove the `last_known_place == Some(place)` shortcut and switch AI social candidate generation to the already-landed listener-aware resend policy based on current shareable belief content versus remembered told state.

### 2. Add social omission diagnostics

Extend `CandidateGenerationDiagnostics`, `decision_trace.rs`, and the `agent_tick` trace plumbing with social omission reasons for resend suppression and explicit recipient-knowledge explanation surfaces.

### 3. Rewrite focused candidate tests

Replace the old co-location suppression test with resend-aware focused tests for no-prior-tell emission, unchanged repeats, changed content, expired memory, and `observed_tick`-only refreshes.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify)
- `crates/worldwake-sim/src/social_relay.rs` (read-only dependency; modify only if trace-friendly helper extraction is required)

## Out of Scope

- Authoritative Tell commit writes
- World/core schema changes
- Golden E2E social regressions
- `ShareBelief` ranking weights or suppression by danger/self-care

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_emit_for_live_colocated_listeners_and_relayable_subjects`
2. `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_suppress_unchanged_repeat_tells_via_told_memory`
3. `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_reemit_when_shared_content_changes`
4. `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_ignore_observed_tick_only_refreshes`
5. `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_reemit_when_tell_memory_has_expired`
6. `cargo test -p worldwake-ai decision_trace::tests::goal_status_reports_social_omission_reason`
7. `cargo test -p worldwake-ai agent_tick::tests::trace_social_resend_omission_reason`
8. Existing suite: `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_require_tell_profile_and_respect_blocked_memory`

### Invariants

1. `ShareBelief` suppression depends on actor-local remembered told state, never on listener omniscience or same-place co-location.
2. A bookkeeping-only belief refresh must not re-enable `ShareBelief`.
3. Decision traces must be able to distinguish “already told current belief” from ordinary non-generation.
4. Listener-aware resend filtering must happen before candidate truncation in AI just as it already does for tell-affordance enumeration.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — replace co-location suppression coverage with resend-aware focused tests proving no-prior-tell emission, unchanged-repeat suppression, stale/expired re-emission, and `observed_tick`-only refresh handling.
2. `crates/worldwake-ai/src/decision_trace.rs` — add social omission status coverage so `GoalTraceStatus` can explain resend suppression.
3. `crates/worldwake-ai/src/agent_tick.rs` — add focused trace-plumbing coverage proving social omissions survive candidate generation and appear in the recorded runtime decision trace.

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_suppress_unchanged_repeat_tells_via_told_memory`
2. `cargo test -p worldwake-ai decision_trace::tests::goal_status_reports_social_omission_reason`
3. `cargo test -p worldwake-ai agent_tick::tests::trace_social_resend_omission_reason`
4. `cargo test -p worldwake-ai`
5. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
6. `cargo test -p worldwake-ai --test golden_emergent golden_tell_propagates_political_knowledge`
7. `cargo test -p worldwake-ai --test golden_social`
8. `cargo clippy --workspace --all-targets -- -D warnings`
9. `cargo test --workspace`

## Outcome

Completed on 2026-03-19.

### What Changed

1. `emit_social_candidates()` now uses the shared listener-aware resend policy instead of the old same-place shortcut, so `ShareBelief` suppression is driven by actor-local tell memory and share-equivalent belief content.
2. Candidate-generation diagnostics, decision traces, and `agent_tick` trace plumbing now record social omission reasons via `RecipientKnowledgeStatus`, making resend suppression visible in traces instead of implicit.
3. Focused AI tests were rewritten around the actual resend contract: unchanged repeat suppression, changed-content re-emission, bookkeeping-only refresh suppression, retention expiry re-emission, and listener-aware pre-truncation filtering.

### What Changed Versus The Original Plan

1. The ticket was corrected before implementation because tickets 002, 003, and 004 had already landed. This work did not add new memory/query infrastructure or tell commit writes; it consumed the existing shared substrate.
2. Two existing goldens needed scenario-isolation fixes once the stale co-location heuristic was removed:
   - `golden_tell_propagates_political_knowledge` now waits for the office belief to be received instead of assuming the first tell commit must be the office tell.
   - `golden_social` now asserts on the actual committed tell action where needed and isolates the relay-chain test so it validates the intended chain-length contract rather than incidental same-place opportunities.

### Verification

1. Focused tests passed for candidate generation, decision-trace status mapping, and `agent_tick` trace plumbing.
2. `cargo test -p worldwake-ai` passed.
3. `cargo test -p worldwake-ai --test golden_emergent golden_tell_propagates_political_knowledge` passed.
4. `cargo test -p worldwake-ai --test golden_social` passed.
5. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` passed.
6. `cargo clippy --workspace --all-targets -- -D warnings` passed.
7. `cargo test --workspace` passed.
