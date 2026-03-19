# E15CCONMEMANDRECKNO-006: Golden Social Retell Regressions

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — golden/runtime verification coverage for already-landed E15c behavior
**Deps**: `E15CCONMEMANDRECKNO-003`, `E15CCONMEMANDRECKNO-004`, `E15CCONMEMANDRECKNO-005`

## Problem

E15c changes a socially sensitive loop that already regressed when the old same-place shortcut was weakened. Focused tests are necessary but not sufficient: the repo needs golden coverage proving that autonomous telling no longer spams unchanged facts, still permits lawful retelling after belief change or expiry, and leaves those state transitions legible in decision traces.

## Assumption Reassessment (2026-03-19)

1. Current social goldens in `crates/worldwake-ai/tests/golden_social.rs` already cover autonomous Tell, rumor relay, stale-belief reobserve/replan, listener rejection, bystander locality, survival-pressure suppression, chain-length filtering, diversity, and wasted-trip discovery. `cargo test -p worldwake-ai --test golden_social -- --list` confirms there is still no golden covering unchanged resend suppression, retell-after-belief-change, or retell-after-conversation-memory-expiry.
2. Existing lower-layer coverage is broader than this ticket originally claimed:
   - candidate-generation focused coverage in `crates/worldwake-ai/src/candidate_generation.rs`: `social_candidates_suppress_unchanged_repeat_tells_via_told_memory`, `social_candidates_reemit_when_shared_content_changes`, `social_candidates_ignore_observed_tick_only_refreshes`, `social_candidates_reemit_when_tell_memory_has_expired`, and `social_candidates_listener_aware_filtering_happens_before_truncation`
   - runtime decision-trace coverage in `crates/worldwake-ai/src/agent_tick.rs`: `trace_social_resend_omission_reason`
   - authoritative/storage coverage in `crates/worldwake-core/src/belief.rs`, `crates/worldwake-systems/src/tell_actions.rs`, and `crates/worldwake-sim/src/per_agent_belief_view.rs`
3. `worldwake-core` conversation-memory storage, `worldwake-sim` actor-local conversation-memory views, `worldwake-systems` tell commit writes, and `worldwake-ai` recipient-knowledge checks already exist in production code. This ticket is therefore verification-first, not substrate-delivery work.
4. The E15c spec explicitly calls for goldens proving: no spam over repeated ticks, lawful retell after belief-content change, memory-expiry retell, and trace visibility for reappearance. The “co-location alone must not suppress the initial tell” case is already covered by `golden_agent_autonomously_tells_colocated_peer`, so this ticket should reference that existing golden instead of adding a duplicate scenario.
5. This is a mixed-layer golden gap. Reasoning-layer resend suppression / reappearance should be asserted through decision traces; authoritative tell delivery should be asserted through listener belief state and speaker conversation-memory state.
6. Scenario isolation must be explicit: the resend/retell goldens should remove unrelated need-driven or production affordances so the contract under test is the social branch, not incidental competition from survival or work goals.
7. Mismatch and correction: this ticket should not claim “no relevant coverage exists” or propose harness work by default. The corrected scope is to add missing golden E2E scenarios in `crates/worldwake-ai/tests/golden_social.rs`, reusing existing harness tracing support unless implementation proves a tiny helper is genuinely necessary.

## Architecture Check

1. Keeping the final resend/retell regressions in `golden_social.rs` remains cleaner than scattering E15c end-to-end behavior across unrelated goldens; the subject is specifically social information transfer.
2. Reusing the existing conversation-memory architecture is better than inventing another anti-spam layer. The remaining need is proof that the current design works end to end under autonomous scheduling, not another alias/cache/shim.
3. Golden assertions should isolate the social branch intentionally and document removed competing lawful affordances so failures remain architectural, not setup-noise artifacts.

## Verification Layers

1. Candidate suppression for unchanged re-tells -> decision trace `omitted_social` / goal-status assertions
2. Candidate reappearance after belief change or expiry -> decision trace assertions plus successful tell execution
3. Tell execution and knowledge transfer -> authoritative listener belief state and/or action trace where same-tick completion matters
4. Conversation-memory refresh after lawful retell -> authoritative speaker `told_beliefs` state

## What to Change

### 1. Add resend-regression goldens

Add golden scenarios for unchanged-repeat suppression, belief-change retell, and retention-expiry retell.

### 2. Add trace-based explanation assertions

Enable decision tracing in the new goldens and assert that the social candidate is omitted or re-enabled for the expected resend reason.

### 3. Reuse existing golden harness support unless a tiny helper is unavoidable

Prefer `GoldenHarness::driver.enable_tracing()` and current belief/profile seeding helpers. Do not broaden this ticket into general harness refactors.

### 4. Keep scenario isolation explicit

Configure each scenario so unrelated high-priority goals do not mask the social branch being proven.

## Files to Touch

- `crates/worldwake-ai/tests/golden_social.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify only if a truly minimal helper is required after implementation)

## Out of Scope

- Changing core conversation-memory schema
- Changing Tell commit semantics
- Changing candidate-generation filtering logic
- Duplicating already-covered initial same-place Tell behavior with a second golden
- Adding non-social integration scenarios outside `golden_social.rs`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_agent_does_not_repeat_same_unchanged_tell_to_same_listener`
2. `cargo test -p worldwake-ai golden_agent_retells_after_subject_belief_changes`
3. `cargo test -p worldwake-ai golden_agent_retells_after_conversation_memory_expiry`
4. `cargo test -p worldwake-ai golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry`
5. Existing suite: `cargo test -p worldwake-ai golden_agent_autonomously_tells_colocated_peer`
6. Existing suite: `cargo test -p worldwake-ai --test golden_social`

### Invariants

1. Repeated unchanged tells to the same listener are suppressed over repeated ticks without reviving the same-place shortcut.
2. Material belief change or conversation-memory expiry lawfully re-enables retelling.
3. Golden assertions for reasoning use decision traces, not only downstream event absence.
4. Existing initial-tell behavior remains covered by `golden_agent_autonomously_tells_colocated_peer`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` — `golden_agent_does_not_repeat_same_unchanged_tell_to_same_listener`; proves end-to-end autonomous resend suppression keeps the original `(listener, subject)` told-memory record stable.
2. `crates/worldwake-ai/tests/golden_social.rs` — `golden_agent_retells_after_subject_belief_changes`; proves a material subject-belief change lawfully re-enables `ShareBelief`, refreshes told memory, and updates the listener.
3. `crates/worldwake-ai/tests/golden_social.rs` — `golden_agent_retells_after_conversation_memory_expiry`; proves retention expiry re-enables autonomous telling without reintroducing the old same-place heuristic.
4. `crates/worldwake-ai/tests/golden_social.rs` — `golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry`; proves the decision trace shows omission before re-send and reappearance after lawful re-enable conditions.
5. `None — no production code changes were needed for this ticket.`

### Commands

1. `cargo test -p worldwake-ai golden_agent_does_not_repeat_same_unchanged_tell_to_same_listener`
2. `cargo test -p worldwake-ai golden_agent_retells_after_subject_belief_changes`
3. `cargo test -p worldwake-ai --test golden_social`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-19
- What actually changed:
  - corrected the ticket scope to reflect existing focused/runtime coverage and the actual remaining golden gap
  - added four golden tests in `crates/worldwake-ai/tests/golden_social.rs` covering unchanged suppression, belief-change re-tell, expiry re-tell, and decision-trace re-enable visibility
- Deviations from original plan:
  - no `golden_harness` changes were required
  - no duplicate “initial same-place tell still works” golden was added because `golden_agent_autonomously_tells_colocated_peer` already covered that contract
  - no production code changes were required; the current conversation-memory architecture proved sufficient once the golden gap was filled
- Verification results:
  - `cargo test -p worldwake-ai --test golden_social` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
