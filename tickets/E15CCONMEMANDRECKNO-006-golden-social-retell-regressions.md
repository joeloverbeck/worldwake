# E15CCONMEMANDRECKNO-006: Golden Social Retell Regressions

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — verification coverage for completed E15c behavior
**Deps**: `E15CCONMEMANDRECKNO-003`, `E15CCONMEMANDRECKNO-004`, `E15CCONMEMANDRECKNO-005`

## Problem

E15c changes a socially sensitive loop that already regressed when the old same-place shortcut was weakened. Focused tests are necessary but not sufficient: the repo needs golden coverage proving that autonomous telling no longer spams unchanged facts, still permits lawful retelling after belief change or expiry, and leaves those state transitions legible in decision traces.

## Assumption Reassessment (2026-03-19)

1. Current social goldens in `crates/worldwake-ai/tests/golden_social.rs` cover autonomous Tell, rumor relay, diversity, and suppression by needs, but none cover resend suppression or retell-after-change/expiry.
2. Existing relevant goldens are `golden_agent_autonomously_tells_colocated_peer`, `golden_bystander_sees_telling_but_gets_no_belief`, and `golden_survival_needs_suppress_social_goals`.
3. The E15c spec explicitly calls for goldens proving: no spam over repeated ticks, lawful retell after belief-content change, co-location alone not suppressing initial tell, memory-expiry retell, and trace visibility for reappearance.
4. These are mixed-layer scenarios. Candidate-generation assertions should rely on decision traces for reasoning-layer behavior, while authoritative delivery should still be checked by resulting belief/memory state where necessary.
5. Mismatch and correction: this ticket should not backfill core or systems semantics. It assumes the lower-layer tickets are already landed.

## Architecture Check

1. Keeping the final regressions in `golden_social.rs` is cleaner than scattering E15c end-to-end behavior across unrelated emergent or office goldens.
2. Golden assertions should isolate the social branch intentionally and document any removed competing lawful affordances.

## Verification Layers

1. Candidate reappearance/suppression over time -> decision trace assertions in golden tests
2. Tell execution and knowledge transfer -> authoritative belief/conversation-memory state checks in golden tests
3. We do not rely only on absence of tell events when the actual contract is decision-layer resend suppression.

## What to Change

### 1. Add resend-regression goldens

Add golden scenarios for unchanged-repeat suppression, belief-change retell, and retention-expiry retell.

### 2. Add trace-based explanation assertions

Enable decision tracing in the new goldens and assert that the social candidate is omitted or re-enabled for the expected resend reason.

### 3. Keep scenario isolation explicit

Configure the harness so unrelated high-priority goals do not mask the social branch being proven.

## Files to Touch

- `crates/worldwake-ai/tests/golden_social.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)

## Out of Scope

- Changing core conversation-memory schema
- Changing Tell commit semantics
- Changing candidate-generation filtering logic
- Adding non-social integration scenarios outside `golden_social.rs`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_agent_does_not_repeat_same_unchanged_tell_to_same_listener`
2. `cargo test -p worldwake-ai golden_agent_retells_after_subject_belief_changes`
3. `cargo test -p worldwake-ai golden_same_place_without_prior_tell_memory_still_allows_initial_tell`
4. `cargo test -p worldwake-ai golden_agent_retells_after_conversation_memory_expiry`
5. `cargo test -p worldwake-ai golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry`
6. Existing suite: `cargo test -p worldwake-ai golden_agent_autonomously_tells_colocated_peer`

### Invariants

1. Repeated unchanged tells to the same listener are suppressed over repeated ticks without reviving the same-place shortcut.
2. Material belief change or memory expiry lawfully re-enables retelling.
3. Golden assertions for reasoning use decision traces, not only downstream event absence.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` — add the five E15c resend/retell goldens.
2. `crates/worldwake-ai/tests/golden_harness/mod.rs` — add only the minimal helpers required to seed belief changes, expiry windows, and trace inspection.
3. `None — no production code changes are expected in this ticket.`

### Commands

1. `cargo test -p worldwake-ai golden_agent_does_not_repeat_same_unchanged_tell_to_same_listener`
2. `cargo test -p worldwake-ai golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry`
3. `cargo test -p worldwake-ai --test golden_social`
