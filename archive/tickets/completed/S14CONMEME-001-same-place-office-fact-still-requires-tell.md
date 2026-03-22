# S14CONMEME-001: Same-Place Office Fact Still Requires Tell

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` golden emergence coverage only unless implementation proves a reusable harness gap
**Deps**: `specs/S14-conversation-memory-emergence-golden-suites.md`, `specs/IMPLEMENTATION-ORDER.md`, E15c, E16d

## Problem

E15c removed the old same-place Tell suppression shortcut, but current golden coverage does not prove that removal matters in a live cross-system chain. The existing political emergence golden uses remote office facts, and the existing social Tell goldens do not prove that a same-place office subject still requires actual conversation to unlock downstream office behavior.

## Assumption Reassessment (2026-03-19)

1. Current cross-system office Tell coverage exists, but it is remote rather than same-place: `crates/worldwake-ai/tests/golden_emergent.rs` contains `golden_tell_propagates_political_knowledge` and `golden_tell_propagates_political_knowledge_replays_deterministically`, confirmed via `cargo test -p worldwake-ai -- --list`.
2. Current social E15c coverage proves unchanged-repeat suppression and lawful re-tell, but not this same-place political chain: `crates/worldwake-ai/tests/golden_social.rs` contains `golden_agent_does_not_repeat_same_unchanged_tell_to_same_listener`, `golden_agent_retells_after_subject_belief_changes`, `golden_agent_retells_after_conversation_memory_expiry`, and `golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry`.
3. Current focused coverage already proves the candidate-generation gate for office knowledge and the social omission diagnostics: `candidate_generation::tests::political_candidates_require_known_office_belief_for_generation`, `candidate_generation::tests::social_candidates_emit_for_live_colocated_listeners_and_relayable_subjects`, and `agent_tick::tests::trace_social_resend_omission_reason`.
4. `specs/S14-conversation-memory-emergence-golden-suites.md` explicitly calls for a same-place office scenario that proves co-location is not a lawful proxy for listener knowledge. `specs/IMPLEMENTATION-ORDER.md` places S14 after E15c and E16d and specifies S14-001 -> S14-002 -> S14-003.
5. Intended verification layer is golden E2E with explicit decision-trace, action-trace, and authoritative-state assertions. This is not a local needs-only harness case; it requires the live social plus political action registries already exercised by `golden_emergent.rs`.
6. Ordering contract is action lifecycle ordering, but reassessment during implementation revealed a same-place nuance: Tell can lawfully unlock listener `declare_support` in the same tick. The robust contract for this scenario is therefore append-only action-trace order, not "strictly later tick". The compared branches remain asymmetric because one branch lacks the office belief entirely before Tell.
7. Scenario isolation is required because the speaker could otherwise win the office through lawful political behavior. The compared branches are asymmetric by design: before Tell the listener lacks the office belief, while the speaker may lawfully know it and may also generate political goals. The clean isolation choice is to remove the speaker's competing office branch with concrete motive differences, not test-only suppression or manual mid-scenario belief injection.
8. Mismatch correction: the ticket file itself was misnamed `S14CONMEMEME-001`; it has been corrected to `S14CONMEME-001` so the ticket ID matches the spec naming. The spec allows `golden_social.rs` or `golden_emergent.rs`, but the current repo layout and downstream office focus make `crates/worldwake-ai/tests/golden_emergent.rs` the cleaner default home.
9. Mismatch correction: the current `golden_harness` already exposes the concrete helpers this scenario needs (`seed_office`, `seed_actor_beliefs`, `set_agent_perception_profile`, `set_agent_tell_profile`, action tracing, and decision tracing). Harness changes are not part of the intended scope unless implementation exposes a real generic gap.

## Architecture Check

1. Adding one targeted golden in `golden_emergent.rs` is cleaner than expanding focused unit coverage because the missing risk is specifically a cross-system causal chain: social belief transfer -> political candidate generation -> office action outcome.
2. Keeping this ticket test-first and test-only is architecturally preferable to changing production behavior. The current architecture already has the right substrate: belief-only planning, listener-aware social generation, and ordinary office succession. What is missing is end-to-end proof that same-place subject location does not alias into listener knowledge.
3. The ticket must not introduce new production mechanics, same-place shortcuts, or compatibility shims. If the scenario only passes by changing live behavior, the assumption behind the ticket must be reassessed before implementation continues.

## Verification Layers

1. Listener has no `ClaimOffice` candidate before hearing the office fact -> decision trace
2. Speaker still generates and commits Tell for the same-place office subject -> decision trace plus action trace
3. Tell appears earlier than the listener's `declare_support` commit in the append-only action trace -> action trace
4. Listener becomes office holder through ordinary office resolution -> authoritative world state
5. Co-location alone does not seed the political branch before Tell -> decision trace, not inferred indirectly from missing later office state

## What to Change

### 1. Add the same-place office emergence golden

Add `golden_same_place_office_fact_still_requires_tell` and a deterministic replay companion in `crates/worldwake-ai/tests/golden_emergent.rs`. Use existing live helpers. The implemented setup should keep speaker and listener co-located with the office subject from the start, disable passive office discovery for the listener through concrete perception state, delay the speaker's office belief seeding until after an initial co-location phase, and keep the listener as the lawful eventual claimant.

### 2. Use existing harness support; add helper code only if a genuine generic gap appears

Default expectation: no harness changes. If the scenario truly needs extra helper support, limit changes to reusable golden harness helpers in `crates/worldwake-ai/tests/golden_harness/mod.rs`. Helpers must remain generic for office seeding, belief seeding, or trace inspection and must not encode S14-specific shortcuts.

### 3. Record the docs follow-up requirement

When this golden lands, review `docs/golden-e2e-testing.md`, `docs/golden-e2e-coverage.md`, and `docs/golden-e2e-scenarios.md` for necessary updates. If documentation changes are deferred to the current follow-up ticket `tickets/S14CONMEMEME-003-golden-e2e-docs-catch-up.md`, note the required updates explicitly in the implementation handoff rather than silently skipping the review.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `docs/golden-e2e-testing.md` (review after implementation; update only if the scenario changes assertion-surface guidance)
- `docs/golden-e2e-coverage.md` (review after implementation; expected follow-up in `tickets/S14CONMEMEME-003-golden-e2e-docs-catch-up.md`)
- `docs/golden-e2e-scenarios.md` (review after implementation; expected follow-up in `tickets/S14CONMEMEME-003-golden-e2e-docs-catch-up.md`)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify only if implementation proves a real generic helper gap)

## Out of Scope

- Any production code change in `worldwake-core`, `worldwake-sim`, or `worldwake-systems`
- New conversation-memory rules, resend policies, or Tell mechanics
- Refactoring unrelated goldens or moving existing office/social suites between test binaries
- Manual mid-scenario belief injection after the scenario starts
- Documentation catch-up itself beyond the required review handoff to `tickets/S14CONMEMEME-003-golden-e2e-docs-catch-up.md`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell`
2. `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_emergent`
4. `cargo test -p worldwake-ai`

### Invariants

1. Belief-only planning remains intact: the listener must not gain a political office candidate from authoritative co-location alone.
2. Social-to-political propagation must remain state-mediated: the listener's office behavior appears only after a lawful Tell mutates listener belief state.
3. Same-place action ordering must be proven with append-only action-trace order rather than inferred from final office state or from a stricter later-tick assumption.
4. No production component registration, SystemFn integration, or live office/tell semantics change as part of this ticket.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs::golden_same_place_office_fact_still_requires_tell` — proves that co-location with the office does not generate `ClaimOffice`; only a later same-place Tell unlocks the political branch.
2. `crates/worldwake-ai/tests/golden_emergent.rs::golden_same_place_office_fact_still_requires_tell_replays_deterministically` — proves the same same-place social-to-political chain replays deterministically with the same seed.

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell`
2. `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_emergent`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-19
- What actually changed: corrected the ticket ID/path to `S14CONMEME-001`, added two new goldens in `crates/worldwake-ai/tests/golden_emergent.rs`, and kept the implementation test-only with no harness or production code changes.
- Deviations from original plan: no `golden_harness` changes were needed. The same-place scenario also showed that Tell and downstream `declare_support` can lawfully happen in the same tick, so the final ordering assertion uses append-only action-trace order rather than a stricter later-tick assumption.
- Documentation follow-up: reviewed the expected docs touchpoints and left the catch-up deferred to `tickets/S14CONMEMEME-003-golden-e2e-docs-catch-up.md`; no docs changed in this ticket.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_emergent golden_same_place_office_fact_still_requires_tell`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace`
