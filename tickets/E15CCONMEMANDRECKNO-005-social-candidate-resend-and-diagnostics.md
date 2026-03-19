# E15CCONMEMANDRECKNO-005: Social Candidate Resend And Diagnostics

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI social candidate generation and decision-trace omission diagnostics
**Deps**: `E15CCONMEMANDRECKNO-001`, `E15CCONMEMANDRECKNO-002`, `E15CCONMEMANDRECKNO-003`

## Problem

`crates/worldwake-ai/src/candidate_generation.rs::emit_social_candidates()` still suppresses `ShareBelief` by checking whether the speaker believes the subject is already at the current place. That shortcut is architecturally wrong and the current decision-trace model has no social omission reason analogous to political omissions, so developers cannot see whether a social candidate was skipped because it was already told, stale, or absent for some other reason.

## Assumption Reassessment (2026-03-19)

1. The current same-place shortcut is explicit at `crates/worldwake-ai/src/candidate_generation.rs::emit_social_candidates`, where `belief.last_known_place == Some(place)` suppresses sharing.
2. Existing focused coverage locks in that wrong behavior: `candidate_generation::tests::social_candidates_skip_subjects_already_known_to_be_colocated`.
3. `E15CCONMEMANDRECKNO-002` has already landed the actor-local told-memory and recipient-knowledge reads on live/runtime and planning surfaces, so this ticket should consume those surfaces rather than invent parallel lookup logic.
4. `E15CCONMEMANDRECKNO-003` is the shared resend-policy ticket for social helper logic and authoritative affordance parity. This ticket should explicitly reuse that helper so AI generation and tell affordances do not diverge.
5. `CandidateGenerationDiagnostics` currently contains only `omitted_political`, and `decision_trace.rs` only supports `GoalTraceStatus::OmittedPolitical(...)`; there is no social omission surface.
6. Candidate-generation work here is runtime `agent_tick` reasoning, but local focused tests in `candidate_generation.rs` are sufficient for the resend gate itself. Full action registries are not required for the core suppression logic.
7. The intended verification layer for omission explainability is the decision trace, not indirect absence of events or missing committed actions.
8. Mismatch and correction: the old same-place test must be removed or rewritten, not preserved beside the new resend model.

## Architecture Check

1. Reusing the shared resend helper from ticket 003 is cleaner than duplicating listener-aware filtering in AI with slightly different truncation semantics.
2. Adding explicit social omission diagnostics to decision traces is cleaner than forcing developers to infer resend suppression from missing candidates.
3. No backwards-compatibility flag should preserve the co-location shortcut.
4. This change is more beneficial than the current architecture because the present AI path suppresses social behavior using a world-state proxy instead of explicit remembered interaction state, which violates the intended locality and debuggability model.

## Verification Layers

1. `ShareBelief` candidate emission/suppression follows actor-local told memory -> focused unit tests in `candidate_generation.rs`
2. Social omission reasons appear in decision traces -> focused unit tests in `decision_trace.rs`
3. We do not use event-log or action-trace side effects as a proxy for candidate-generation reasoning because the contract here is reasoning-layer omission.

## What to Change

### 1. Replace same-place suppression

Remove the `last_known_place == Some(place)` shortcut and use retention-aware, listener-aware resend suppression based on current shareable belief content versus remembered told state.

### 2. Add social omission diagnostics

Extend `CandidateGenerationDiagnostics`, `decision_trace.rs`, and the `agent_tick` trace plumbing with social omission reasons and recipient-knowledge explanation surfaces.

### 3. Rewrite focused candidate tests

Replace the old co-location suppression test with resend-aware tests for unchanged repeats, changed content, expired memory, and `observed_tick`-only refreshes.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify)

## Out of Scope

- Authoritative Tell commit writes
- World/core schema changes
- Golden E2E social regressions
- `ShareBelief` ranking weights or suppression by danger/self-care

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_emit_without_same_place_shortcut_when_no_prior_tell_exists`
2. `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_suppress_unchanged_repeat_tells_via_told_memory`
3. `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_reemit_when_shared_content_changes`
4. `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_ignore_observed_tick_only_refreshes`
5. `cargo test -p worldwake-ai decision_trace::tests::goal_status_reports_social_omission_reason`
6. Existing suite: `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_require_tell_profile_and_respect_blocked_memory`

### Invariants

1. `ShareBelief` suppression depends on actor-local remembered told state, never on listener omniscience or same-place co-location.
2. A bookkeeping-only belief refresh must not re-enable `ShareBelief`.
3. Decision traces must be able to distinguish “already told current belief” from ordinary non-generation.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — replace co-location suppression coverage with resend-aware focused tests.
2. `crates/worldwake-ai/src/decision_trace.rs` — add social omission status coverage.
3. `crates/worldwake-ai/src/agent_tick.rs` — adjust trace plumbing tests if needed to carry the new omission records.

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::social_candidates_suppress_unchanged_repeat_tells_via_told_memory`
2. `cargo test -p worldwake-ai decision_trace::tests::goal_status_reports_social_omission_reason`
3. `cargo test -p worldwake-ai`
