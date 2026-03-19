# E15CCONMEMANDRECKNO-004: Tell Commit Participant Memory

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — authoritative Tell commit writes speaker/listener conversation memory
**Deps**: `E15CCONMEMANDRECKNO-001`, `E15CCONMEMANDRECKNO-002`

## Problem

The current `crates/worldwake-systems/src/tell_actions.rs::commit_tell()` only updates the listener’s `known_entities` on successful acceptance. It never records what the speaker remembers having told, never records what the listener remembers hearing, and has no disposition surface for accepted versus ignored tells. Without that, E15c has no authoritative social-memory source of truth.

## Assumption Reassessment (2026-03-19)

1. `crates/worldwake-core/src/belief.rs` already contains the E15c conversation-memory substrate: `AgentBeliefStore::{told_beliefs, heard_beliefs}`, `TellMemoryKey`, `ToldBeliefMemory`, `HeardBeliefMemory`, `HeardBeliefDisposition`, `to_shared_belief_snapshot`, and deterministic `enforce_conversation_memory()`. The ticket’s gap is authoritative mutation in `crates/worldwake-systems/src/tell_actions.rs::commit_tell()`, not missing schema.
2. The current schema stores `counterparty` and `subject` in `TellMemoryKey`, not duplicated inside `ToldBeliefMemory` or `HeardBeliefMemory`. Ticket wording and planned assertions should target the actual key-plus-value shape.
3. `commit_tell()` is still pre-E15c: it degrades the source, may update `listener_beliefs.known_entities`, enforces only generic perception memory capacity, and never records speaker/listener conversation memory in any branch.
4. On failed acceptance fidelity, `commit_tell()` currently returns `CommitOutcome::empty()` before mutating either the speaker or the listener. That diverges from the spec, which requires `NotInternalized` heard memory plus speaker told memory for a committed social interaction.
5. Existing focused/system coverage is broader than the original ticket claims. `crates/worldwake-systems/src/tell_actions.rs` already contains affordance-level E15c tests such as `tell_affordances_skip_already_told_current_belief_for_listener`, `tell_affordances_reinclude_subject_when_prior_tell_is_stale_or_changed`, and `tell_affordances_listener_aware_filtering_happens_before_truncation`. There is also downstream golden coverage in `crates/worldwake-ai/tests/golden_social.rs::golden_skeptical_listener_rejects_told_belief`. None of these currently assert commit-side participant-memory writes or heard dispositions.
6. Existing commit-side focused coverage is limited to source degradation, relay-limit recheck, acceptance-fidelity skipping, listener-newer-belief retention, and generic perception-memory eviction. There is still no focused test that asserts `told_beliefs`, `heard_beliefs`, or `HeardBeliefDisposition` outcomes.
7. `E15CCONMEMANDRECKNO-002` and the current runtime/planning surfaces (`crates/worldwake-sim/src/per_agent_belief_view.rs`, `crates/worldwake-ai/src/planning_state.rs`) already read actor-local told memory. This ticket is therefore the missing authoritative write-half of an existing architecture, not an isolated social feature.
8. The spec requires conversation-memory writes and belief-store writes to happen in the same `WorldTxn`; the contract here is authoritative world-state ordering, not merely event-log ordering.
9. The current listener update path preserves only strictly newer local beliefs. For equal-tick existing beliefs, `AgentBeliefStore::update_entity()` will replace the existing state, which would degrade an equal-tick local belief to a rumor/report. That is weaker than the spec’s `AlreadyHeldEqualOrNewer` intent, so this ticket’s scope must include retaining equal-or-newer listener belief instead of downgrading it.
10. `Rejected` remains reserved for future explicit trust/contradiction logic. This ticket should not invent a new rejection substrate.

## Architecture Check

1. `commit_tell()` is the correct authoritative mutation point because Tell is already the concrete social action that transmits knowledge between colocated agents.
2. Writing speaker and listener conversation memory in that same transaction is cleaner than deriving it later from event-log replay or AI-side heuristics. Replay-derived memory would either duplicate live-authority semantics or force later layers to infer social state from proxies.
3. Retaining an equal-or-newer listener belief is architecturally stronger than the current equal-tick overwrite behavior. Local belief should not be silently downgraded to weaker provenance just because a social tell happened to arrive with the same timestamp.
4. This proposal is more beneficial than the current architecture because the current `commit_tell()` produces no authoritative participant-memory trail at all and still leaves an equal-tick downgrade hazard in place.

## Verification Layers

1. Speaker and listener conversation-memory writes happen in the same authoritative Tell commit -> focused `tell_actions.rs` commit test asserting both belief-store lanes after transaction commit
2. Accepted branch updates listener authoritative belief and records `HeardBeliefDisposition::Accepted` -> focused `tell_actions.rs` commit test
3. Equal-or-newer listener local belief is retained while heard memory records `AlreadyHeldEqualOrNewer` -> focused `tell_actions.rs` commit test
4. Acceptance-fidelity failure still records participant memory while leaving `known_entities` unchanged -> focused `tell_actions.rs` commit test
5. Existing affordance and golden resend/rejection coverage remains a regression surface, but does not by itself prove authoritative commit semantics -> existing `tell_actions.rs` affordance tests and `golden_social.rs::golden_skeptical_listener_rejects_told_belief`

## What to Change

### 1. Record speaker-side told memory

When Tell commits, record `ToldBeliefMemory` for `(listener, subject)` using the speaker’s shareable belief snapshot.

### 2. Record listener-side heard memory

Write `HeardBeliefMemory` for `(speaker, subject)` with `Accepted`, `AlreadyHeldEqualOrNewer`, or `NotInternalized` as dictated by the concrete branch.

### 3. Keep mutation ordering atomic

Apply conversation-memory writes, optional `known_entities` update, and conversation-memory retention maintenance inside the same authoritative transaction.

### 4. Preserve equal-or-newer local listener belief

Do not let an equal-tick tell overwrite a listener belief that is already equal-or-newer. Record the heard memory and disposition instead of degrading local provenance.

## Files to Touch

- `crates/worldwake-systems/src/tell_actions.rs` (modify)

## Out of Scope

- Listener-aware affordance filtering
- AI candidate generation
- Decision-trace omission diagnostics
- Adding trust-based explicit rejection logic without an existing substrate

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems tell_actions::tests::tell_commit_records_speaker_told_belief_memory`
2. `cargo test -p worldwake-systems tell_actions::tests::tell_commit_records_listener_heard_belief_with_accepted_disposition`
3. `cargo test -p worldwake-systems tell_actions::tests::tell_commit_records_listener_heard_belief_with_already_held_equal_or_newer`
4. `cargo test -p worldwake-systems tell_actions::tests::tell_commit_records_listener_heard_belief_with_not_internalized`
5. Existing focused regression: `cargo test -p worldwake-systems tell_actions::tests::tell_commit_keeps_listener_newer_belief`
6. Existing focused regression: `cargo test -p worldwake-systems tell_actions::tests::tell_commit_transfers_direct_observation_as_report_and_preserves_tick`

### Invariants

1. Speaker memory is derived from the speaker’s own belief content, not from later listener state.
2. Listener memory records hearing even when the tell is not internalized into `known_entities`.
3. No branch reads the listener’s live belief store on behalf of the speaker.
4. Equal-or-newer listener belief is not downgraded by an equal-tick tell.
5. Tell commit remains deterministic and same-place lawful.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — add focused commit-semantic tests for speaker `told_beliefs` and listener `heard_beliefs`.
2. `crates/worldwake-systems/src/tell_actions.rs` — strengthen acceptance-fidelity and retained-belief tests to assert participant-memory side effects, not only `known_entities`.
3. `None` — broader golden resend coverage already exists and additional goldens remain deferred to `E15CCONMEMANDRECKNO-006`.

### Commands

1. `cargo test -p worldwake-systems tell_actions::tests::tell_commit_records_speaker_told_belief_memory`
2. `cargo test -p worldwake-systems tell_actions::tests::tell_commit_records_listener_heard_belief_with_not_internalized`
3. `cargo test -p worldwake-systems tell_actions::tests::tell_commit_records_listener_heard_belief_with_already_held_equal_or_newer`
4. `cargo test -p worldwake-systems`

## Outcome

- Completion date: 2026-03-19
- What actually changed:
  - `crates/worldwake-systems/src/tell_actions.rs::commit_tell()` now records speaker `told_beliefs` and listener `heard_beliefs` in the authoritative Tell commit path.
  - Accepted tells now record `HeardBeliefDisposition::Accepted`; failed acceptance-fidelity commits record `HeardBeliefDisposition::NotInternalized`; equal-or-newer listener beliefs record `HeardBeliefDisposition::AlreadyHeldEqualOrNewer`.
  - Equal-tick tells no longer degrade an existing listener belief to weaker report/rumor provenance.
  - Focused tests were added for speaker-memory writes, accepted heard memory, equal-or-newer heard memory, and not-internalized heard memory. Existing acceptance-fidelity and newer-belief tests were strengthened to assert memory side effects.
- Deviations from original plan:
  - The ticket scope was corrected before implementation to match the real schema shape in `crates/worldwake-core/src/belief.rs` and the already-existing affordance/golden coverage.
  - The final implementation intentionally did not add any `Rejected` path because no explicit trust/contradiction substrate exists yet.
  - The implementation also fixed the equal-tick local-belief downgrade hazard because it was a direct architectural mismatch with the E15c `AlreadyHeldEqualOrNewer` contract.
- Verification results:
  - Passed focused tests:
    - `cargo test -p worldwake-systems tell_commit_records_`
    - `cargo test -p worldwake-systems tell_commit_keeps_listener_newer_belief`
    - `cargo test -p worldwake-systems tell_commit_respects_listener_acceptance_fidelity`
  - Passed crate checks:
    - `cargo test -p worldwake-systems`
    - `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
  - Passed workspace checks:
    - `cargo test --workspace`
    - `cargo clippy --workspace --all-targets -- -D warnings`
