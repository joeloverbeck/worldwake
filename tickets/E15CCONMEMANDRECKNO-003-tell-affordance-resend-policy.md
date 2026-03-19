# E15CCONMEMANDRECKNO-003: Tell Affordance Resend Policy

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — shared resend helper plus authoritative Tell payload enumeration
**Deps**: `E15CCONMEMANDRECKNO-001`, `E15CCONMEMANDRECKNO-002`

## Problem

E15c requires listener-aware resend filtering before truncation for Tell payload enumeration. Current authoritative affordance expansion in `crates/worldwake-systems/src/tell_actions.rs` still calls `relayable_social_subjects(...)`, which globally truncates by recency before considering listener-specific resend suppression. That is exactly the crowding failure the spec calls out.

## Assumption Reassessment (2026-03-19)

1. `crates/worldwake-core/src/belief.rs` already provides the needed resend substrate: `ToldBeliefMemory`, retention-aware `told_belief_memory()`, and `recipient_knowledge_status()`. This ticket should not describe itself as adding conversation memory.
2. `crates/worldwake-sim/src/social_relay.rs` currently exposes only `relayable_social_subjects()`, which sorts all subjects by recency and truncates before listener expansion.
3. `crates/worldwake-systems/src/tell_actions.rs::enumerate_tell_payloads()` directly uses that helper and therefore still cannot honor per-listener resend suppression.
4. `E15CCONMEMANDRECKNO-002` has now landed the actor-local belief-view/planning plumbing needed for retention-aware told-memory reads. This ticket is the first consumer-switch on top of that plumbing.
5. Existing focused coverage names the current contract that must change: `tell_actions::tests::tell_affordances_filter_relay_depth_and_limit_subjects_by_recency`.
6. The E15c spec explicitly requires AI candidate generation and Tell affordance payload enumeration to use the same resend policy. The cleanest way to do that is a shared helper rather than duplicated filters in AI and systems.
7. This ticket is about payload enumeration, not candidate traces or Tell commit writes.

## Architecture Check

1. A shared resend-policy helper in `worldwake-sim/src/social_relay.rs` is cleaner than duplicating listener-aware filtering logic in `candidate_generation.rs` and `tell_actions.rs`.
2. The helper must accept actor-local current beliefs and actor-local told-memory state, never listener truth.
3. This ticket is more beneficial than preserving the current recency-only truncation because the current architecture can permanently crowd out older untold subjects for a listener, which is exactly the kind of hidden planner/runtime artifact E15c is trying to eliminate.

## Verification Layers

1. Listener-aware resend filtering happens before truncation -> focused unit tests in `tell_actions.rs`
2. Affordance expansion matches the shared resend helper output -> focused unit tests in `tell_actions.rs`
3. This ticket does not yet verify action commit or AI decision traces.

## What to Change

### 1. Introduce shared resend helper(s)

Add a deterministic helper in `crates/worldwake-sim/src/social_relay.rs` that expands `(listener, subject)` pairs, filters them through actor-local resend memory, and only then truncates/caps.

### 2. Update Tell affordance enumeration

Change `enumerate_tell_payloads()` in `crates/worldwake-systems/src/tell_actions.rs` to use the shared listener-aware resend helper instead of `relayable_social_subjects()`.

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

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — add listener-aware resend and truncation-order coverage.
2. `crates/worldwake-sim/src/social_relay.rs` — add unit coverage for pair expansion and truncation ordering when needed.
3. `crates/worldwake-systems/src/tell_actions.rs` — replace the old recency-only affordance expectation with resend-aware behavior.

### Commands

1. `cargo test -p worldwake-systems tell_actions::tests::tell_affordances_listener_aware_filtering_happens_before_truncation`
2. `cargo test -p worldwake-systems tell_actions::tests::tell_affordances_skip_already_told_current_belief_for_listener`
3. `cargo test -p worldwake-systems`
