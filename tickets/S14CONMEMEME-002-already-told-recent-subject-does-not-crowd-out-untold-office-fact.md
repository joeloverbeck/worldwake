# S14CONMEMEME-002: Already-Told Recent Subject Does Not Crowd Out Untold Office Fact

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` golden emergence coverage and possible generic golden harness helpers only
**Deps**: `S14CONMEMEME-001`, `specs/S14-conversation-memory-emergence-golden-suites.md`, E15c, E16d

## Problem

E15c added listener-aware resend suppression before candidate truncation, and focused tests prove that rule on helper surfaces. Current goldens do not prove the same ordering in the live AI/action path with a real downstream consequence. Without that proof, a regression could silently let an already-told recent subject crowd out an older untold office fact and still leave focused tests green.

## Assumption Reassessment (2026-03-19)

1. Focused coverage for the specific ordering invariant already exists: `candidate_generation::tests::social_candidates_listener_aware_filtering_happens_before_truncation` proves the helper-layer contract, and `agent_tick::tests::trace_social_resend_omission_reason` proves the omission reason is visible in runtime traces.
2. Current golden social coverage proves resend suppression and lawful re-tell, but not crowd-out avoidance with downstream office behavior: `golden_agent_does_not_repeat_same_unchanged_tell_to_same_listener`, `golden_agent_retells_after_subject_belief_changes`, and `golden_agent_retells_after_conversation_memory_expiry` all stop at the social layer.
3. Current political emergence coverage proves Tell can unlock office behavior, but not under pre-truncation pressure from a stale recent subject: `golden_tell_propagates_political_knowledge` in `crates/worldwake-ai/tests/golden_emergent.rs`.
4. `specs/S14-conversation-memory-emergence-golden-suites.md` requires a live scenario with `TellProfile { max_tell_candidates: 1, .. }`, a lawful already-told recent subject A, and an older untold office fact B whose downstream office consequence proves pre-truncation filtering.
5. Intended verification layer is golden E2E with explicit decision traces for omission reason, action traces for Tell ordering, and authoritative state for the office outcome. This again requires the full live action stack in `golden_emergent.rs`, not a reduced harness.
6. Ordering contract is mixed but explicit: resend suppression must happen before candidate truncation in the speaker's social selection path, and the resulting Tell for subject B must commit before any office action unlocked by B. Subject A and B are not symmetric because only B is untold and politically enabling.
7. Scenario isolation must preserve A as lawful and shareable. Making subject A invalid, unreachable, or otherwise non-shareable would fail to test the actual truncation-order risk.
8. Mismatch correction: this ticket should stay narrowly scoped to one golden plus replay companion even if implementation reveals reusable helper gaps. Broader golden refactors belong elsewhere.

## Architecture Check

1. A dedicated live golden is cleaner than extending focused unit tests because the unresolved risk is the interaction between conversation memory, candidate truncation, ordinary Tell execution, and downstream political planning.
2. The ticket must not weaken resend suppression, increase candidate limits globally, or add test-only bypasses. No backward-compatibility aliasing or hidden knowledge shortcuts are allowed.

## Verification Layers

1. Subject A is omitted with `SpeakerHasAlreadyToldCurrentBelief` before truncation -> decision trace
2. The speaker commits Tell for subject B without repeating committed Tell for subject A first -> action trace
3. Subject B produces ordinary downstream office behavior only after Tell for B -> decision trace plus authoritative world state
4. If remote office travel is used, Tell for B must precede travel or `declare_support` ordering claims -> action trace
5. The scenario keeps A lawful and shareable, so success is not explained by invalidating A -> focused setup review plus decision trace omission reason

## What to Change

### 1. Add the crowd-out prevention emergence golden

Add `golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact` and its replay companion in `crates/worldwake-ai/tests/golden_emergent.rs`. The scenario should force `max_tell_candidates: 1`, establish A as more recent and already told to the current listener, and establish B as older, untold, and sufficient to unlock a real office-claim chain.

### 2. Keep helper additions generic and minimal

If setup is awkward, extend `crates/worldwake-ai/tests/golden_harness/mod.rs` only with generic helpers for social-memory seeding, trace inspection, or office setup. Do not encode the subject-A/subject-B ordering logic inside the harness.

### 3. Record the docs follow-up requirement

After the golden lands, review `docs/golden-e2e-testing.md`, `docs/golden-e2e-coverage.md`, and `docs/golden-e2e-scenarios.md` to confirm the new assertion surfaces and scenario coverage are documented. If the updates are deferred to `S14CONMEMEME-003`, capture that explicitly during implementation.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, only if a generic helper is necessary)
- `docs/golden-e2e-testing.md` (review after implementation; update only if assertion-surface guidance changes)
- `docs/golden-e2e-coverage.md` (review after implementation; expected follow-up in `S14CONMEMEME-003`)
- `docs/golden-e2e-scenarios.md` (review after implementation; expected follow-up in `S14CONMEMEME-003`)

## Out of Scope

- Any production change to conversation-memory retention, resend suppression, or political planning behavior
- Making subject A invalid, unreachable, or unshareable just to simplify the test
- Broad cleanups of social or political golden suites unrelated to this truncation-order invariant
- Documentation catch-up itself beyond the required review handoff to `S14CONMEMEME-003`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact`
2. `cargo test -p worldwake-ai --test golden_emergent golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_emergent`
4. `cargo test -p worldwake-ai`

### Invariants

1. Listener-aware resend suppression must remain pre-truncation in the live AI/action path, not only in focused helper coverage.
2. Subject A must remain a lawful shareable candidate whose omission is explained by conversation memory, not by invalid setup.
3. The downstream office consequence must arise through ordinary belief transfer, planning, and office actions, not through manual belief injection or direct office mutation.
4. No production system behavior changes are allowed unless a genuine architecture mismatch is found and ticket scope is reassessed first.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs` — add the crowd-out prevention golden and its replay companion.
2. `crates/worldwake-ai/tests/golden_harness/mod.rs` — only if needed for generic setup or trace helpers.

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact`
2. `cargo test -p worldwake-ai --test golden_emergent golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_emergent`
4. `cargo test -p worldwake-ai`
