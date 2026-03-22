# E15CCONMEMANDRECKNO-003: Tell Affordance Resend Policy

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — reusable listener-aware resend helper plus authoritative Tell payload enumeration
**Deps**: `E15CCONMEMANDRECKNO-001`, `E15CCONMEMANDRECKNO-002`

## Problem

E15c requires listener-aware resend filtering before truncation for Tell payload enumeration. Current authoritative affordance expansion in `crates/worldwake-systems/src/tell_actions.rs` still calls `relayable_social_subjects(...)`, which globally truncates by recency before considering listener-specific resend suppression. That is exactly the crowding failure the spec calls out.

## Assumption Reassessment (2026-03-19)

1. `crates/worldwake-core/src/belief.rs` already provides the needed resend substrate: `ToldBeliefMemory`, retention-aware `told_belief_memory()`, and `recipient_knowledge_status()`. This ticket should not describe itself as adding conversation memory.
2. `crates/worldwake-sim/src/social_relay.rs` currently exposes only the raw subject helper `relayable_social_subjects()`, which sorts all subjects by recency and truncates before any listener-specific resend check.
3. `crates/worldwake-systems/src/tell_actions.rs::enumerate_tell_payloads()` directly uses that raw helper and therefore still cannot honor per-listener resend suppression.
4. `E15CCONMEMANDRECKNO-002` has now landed the actor-local belief-view/planning plumbing needed for retention-aware told-memory reads. This ticket is the first consumer-switch on top of that plumbing.
5. Existing focused coverage names the current contract that must change: `tell_actions::tests::tell_affordances_filter_relay_depth_and_limit_subjects_by_recency`.
6. `tickets/E15CCONMEMANDRECKNO-005-social-candidate-resend-and-diagnostics.md` is the separate AI consumer-switch ticket. Ticket 003 should prepare that change by introducing a reusable helper in `worldwake-sim`, but it should not claim that full AI/tell parity is complete before 005 lands.
7. Existing focused coverage in `crates/worldwake-sim/src/social_relay.rs` only proves the raw helper contract today: `relayable_subjects_filter_sort_and_truncate` and `relayable_subjects_allow_zero_candidate_limit`. Existing focused coverage in `crates/worldwake-systems/src/tell_actions.rs` only proves raw relay-depth/recency enumeration: `tell_affordances_expand_live_colocated_listeners_across_relayable_subjects` and `tell_affordances_filter_relay_depth_and_limit_subjects_by_recency`.
8. Mismatch and correction: the original ticket wording overclaimed E15c parity by implying this ticket alone would satisfy the spec's shared resend policy across both AI and tell affordances. Its actual scope is narrower: add the reusable resend helper and switch tell affordance enumeration to it; ticket 005 must still consume the same helper before E15c is fully complete.

## Architecture Check

1. A reusable listener-aware helper in `worldwake-sim/src/social_relay.rs` is cleaner than embedding resend filtering directly in `tell_actions.rs`, and it gives ticket 005 one authoritative policy to reuse later in AI.
2. This helper should layer on top of the existing raw `relayable_social_subjects()` primitive rather than silently changing that primitive's contract while AI still depends on it.
3. The helper must accept actor-local current beliefs plus actor-local recipient-knowledge state, never listener truth.
4. This ticket is more beneficial than preserving the current recency-only truncation because the current architecture can permanently crowd out older untold subjects for a listener, which is exactly the hidden affordance artifact E15c is trying to eliminate.

## Verification Layers

1. Raw helper filtering/sorting remains unchanged for existing callers -> focused unit tests in `social_relay.rs`
2. Listener-aware resend filtering for tell affordance enumeration happens before truncation -> focused unit tests in `tell_actions.rs`
3. Tell affordance expansion matches the reusable resend helper output -> focused unit tests in `tell_actions.rs`
4. This ticket does not yet verify Tell commit writes or AI decision-trace behavior because those belong to tickets 004 and 005.

## What to Change

### 1. Introduce shared resend helper(s)

Add a deterministic helper in `crates/worldwake-sim/src/social_relay.rs` for listener-aware resend selection that:

- starts from current actor-local beliefs
- applies relay-depth filtering
- suppresses only subjects whose recipient-knowledge status is `SpeakerHasAlreadyToldCurrentBelief`
- truncates after that suppression step

Do not silently change the contract of `relayable_social_subjects()` while ticket 005 still depends on the old raw helper behavior.

### 2. Update Tell affordance enumeration

Change `enumerate_tell_payloads()` in `crates/worldwake-systems/src/tell_actions.rs` to use the reusable listener-aware resend helper instead of calling the raw `relayable_social_subjects()` primitive directly.

### 3. Replace stale recency-only test expectations

Update or replace the current affordance test that locks in pre-filter truncation behavior.

## Files to Touch

- `crates/worldwake-sim/src/social_relay.rs` (modify)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)

## Out of Scope

- Writing told/heard memory during `commit_tell()`
- AI candidate generation and decision-trace diagnostics
- Golden social tests
- Changing `ShareBelief` ranking
- Claiming full E15c AI/tell resend-policy parity before ticket 005 lands

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems tell_actions::tests::tell_affordances_skip_already_told_current_belief_for_listener`
2. `cargo test -p worldwake-systems tell_actions::tests::tell_affordances_reinclude_subject_when_prior_tell_is_stale_or_changed`
3. `cargo test -p worldwake-systems tell_actions::tests::tell_affordances_listener_aware_filtering_happens_before_truncation`
4. `cargo test -p worldwake-systems tell_actions::tests::tell_affordances_expand_live_colocated_listeners_across_relayable_subjects`
5. Existing suite: `cargo test -p worldwake-systems tell_actions::tests::tell_affordances_filter_relay_depth_and_limit_subjects_by_recency`

### Invariants

1. Affordance expansion reasons only from the actor’s current belief plus the actor’s remembered prior tells.
2. A previously told recent subject must not crowd out an older untold subject for the same listener.
3. Tell payload enumeration remains deterministic and same-place lawful.
4. Ticket 003 introduces the reusable resend helper but does not by itself complete the AI-side consumer switch from ticket 005.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/social_relay.rs` — add focused unit coverage for the reusable listener-aware resend helper while preserving the raw helper contract.
2. `crates/worldwake-systems/src/tell_actions.rs` — add listener-aware resend and truncation-order coverage.
3. `crates/worldwake-systems/src/tell_actions.rs` — strengthen the existing affordance tests so they prove current tell enumeration uses the new helper rather than raw recency-only truncation.

### Commands

1. `cargo test -p worldwake-systems tell_actions::tests::tell_affordances_listener_aware_filtering_happens_before_truncation`
2. `cargo test -p worldwake-systems tell_actions::tests::tell_affordances_skip_already_told_current_belief_for_listener`
3. `cargo test -p worldwake-sim social_relay::tests::listener_aware_relayable_subjects_filter_before_truncation`
4. `cargo test -p worldwake-systems`

## Outcome

- Completion date: 2026-03-19
- What actually changed:
  - added `listener_aware_relayable_subjects()` to `crates/worldwake-sim/src/social_relay.rs` as a reusable resend-aware layer on top of the existing raw relayable-subject helper
  - switched `crates/worldwake-systems/src/tell_actions.rs::enumerate_tell_payloads()` to that helper using actor-local `recipient_knowledge_status()` reads
  - added focused resend-policy tests in `worldwake-sim` and tell-affordance tests in `worldwake-systems`
- Deviations from original plan:
  - kept `relayable_social_subjects()` unchanged instead of redefining its contract, because ticket 005 still depends on the raw helper while removing the AI same-place heuristic
  - did not change AI candidate generation or decision-trace diagnostics here; that remains ticket 005
- Verification results:
  - `cargo test -p worldwake-sim social_relay::tests::listener_aware_relayable_subjects_filter_before_truncation` passed
  - `cargo test -p worldwake-systems tell_actions::tests::tell_affordances_skip_already_told_current_belief_for_listener` passed
  - `cargo test -p worldwake-systems tell_actions::tests::tell_affordances_reinclude_subject_when_prior_tell_is_stale_or_changed` passed
  - `cargo test -p worldwake-systems tell_actions::tests::tell_affordances_listener_aware_filtering_happens_before_truncation` passed
  - `cargo test -p worldwake-sim` passed
  - `cargo test -p worldwake-systems` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
