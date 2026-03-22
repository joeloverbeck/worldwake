# S13POLEMEGOLSUI-002: Social Tell Propagates Political Knowledge Into Office Claim

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: `specs/S13-political-emergence-golden-suites.md`, existing `golden_social.rs` and `golden_offices.rs` locality coverage

## Problem

The current suite proves that political claims require office knowledge and that social Tell works autonomously, but it does not yet prove the combined emergent chain where office knowledge arrives through the real Tell system and only then unlocks political candidate generation, planning, and installation.

## Assumption Reassessment (2026-03-18)

1. Existing political locality coverage is `golden_information_locality_for_political_facts` and `golden_information_locality_for_political_facts_replays_deterministically` in `crates/worldwake-ai/tests/golden_offices.rs`. Those tests manually seed the office belief update and therefore do not exercise the autonomous social transfer path.
2. Existing social golden coverage in `crates/worldwake-ai/tests/golden_social.rs` includes `golden_agent_autonomously_tells_colocated_peer`, `golden_rumor_chain_degrades_through_three_agents`, and `golden_rumor_leads_to_wasted_trip_then_discovery`, which prove Tell, propagation, and locality generally, but not office-claim emergence from a told political fact.
3. Candidate-generation unit coverage already asserts the belief gate in `candidate_generation::tests::political_candidates_require_known_office_belief_for_generation` and social candidate emission in `candidate_generation::tests::social_candidates_emit_for_live_colocated_listeners_and_relayable_subjects`. The missing layer is golden E2E across social and political domains with full action registries.
4. Current test helpers already cover the needed setup surface in `crates/worldwake-ai/tests/golden_harness/mod.rs`: `seed_office`, `set_agent_tell_profile`, `set_agent_perception_profile`, `seed_actor_local_beliefs`, `agent_belief_about`, plus decision/action tracing. `crates/worldwake-systems/src/tell_actions.rs` `commit_tell` clones arbitrary `BelievedEntityState` entries and degrades provenance, so office beliefs are transferable through the live Tell path without a political special case once a Tell goal is emitted.
5. Mismatch discovered during reassessment: `crates/worldwake-ai/src/candidate_generation.rs` `emit_social_candidates` intentionally skips tell subjects whose believed `last_known_place` matches the speaker’s current place, and `social_candidates_skip_subjects_already_known_to_be_colocated` explicitly locks that behavior in. Attempting to weaken that rule caused repeat-gossip loops and regressed existing office coalition goldens because the current architecture has no recipient-knowledge model to suppress redundant local chatter. The clean correction is to keep same-place Tell suppression and scope this ticket to a remote-office Tell chain instead of changing engine behavior.
6. `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` are currently stale for this ticket’s scope: they already describe Scenario 21 from S13-001, but do not yet catalog Scenario 22. The ticket scope must therefore include documentation correction, not just optional doc follow-up.

## Architecture Check

1. Keep the end-to-end proof in `crates/worldwake-ai/tests/golden_emergent.rs` so the suite proves that political action availability can arise from lawful information transfer rather than setup injection. That is cleaner than adding another office-locality variant in `golden_offices.rs`, because the behavior under test is the cross-system handoff from social knowledge transfer into political planning.
2. Keep the existing same-place Tell suppression. Without explicit world-state support for recipient knowledge or recent-share memory, weakening that rule produces redundant local gossip loops and destabilizes unrelated coalition behavior. The more robust architecture is to prove the social-to-political handoff with a remote office fact, where Tell is already the lawful information carrier.
3. The clean design is to assert the knowledge-path boundary explicitly: no `ClaimOffice` before Tell, then ordinary candidate generation, travel, political action, and succession after Tell. That preserves Principles 7, 13, and 24 without adding a political special case inside the social system or a social special case inside political candidate generation.
4. No backwards-compatibility aliasing or shim behavior should be introduced. If the scenario needs a knowledge-path proof, add it as a golden test over the current lawful Tell architecture rather than weakening the social layer.

## Verification Layers

1. Listener has no office-specific political candidate before learning the office fact -> decision trace assertions on generated candidates in `AgentTickDriver` output.
2. Office belief arrives through the real social channel rather than setup injection -> authoritative belief-store inspection plus `tell` action trace commit ordering.
3. Listener later executes the lawful political path -> decision trace proves `ClaimOffice` appears after Tell; authoritative location and action traces prove travel to jurisdiction and `declare_support` after Tell.
4. Office installation occurs through ordinary authoritative politics -> authoritative office-holder state and/or event-log political mutation assertions.
5. Replay determinism for the mixed social+political chain -> world-hash and event-log-hash equality across identical-seed reruns.

## What to Change

### 1. Add the social-to-political emergence golden scenario

Add `golden_tell_propagates_political_knowledge` and `golden_tell_propagates_political_knowledge_replays_deterministically` to `crates/worldwake-ai/tests/golden_emergent.rs`.

The scenario should:
- Create a support-law vacant office at `VillageSquare`.
- Start the informant and ambitious listener colocated away from the office jurisdiction (for example `BanditCamp`) so the office fact is remote to both at tick 0.
- Give the informant direct office knowledge plus a Tell-capable profile.
- Start the ambitious listener without office knowledge.
- Use decision traces to prove the listener has no political candidate before the tell.
- Use the real Tell action lifecycle to transfer the office belief.
- Assert the listener later generates `ClaimOffice`, travels to the jurisdiction, commits `declare_support`, and becomes office holder.
- Assert semantic ordering that Tell commits before political action/installation.

### 2. Update golden E2E documentation in the same ticket

Review and update the relevant `docs/golden-e2e*` docs after the scenario is implemented.

At minimum:
- `docs/golden-e2e-coverage.md`
- `docs/golden-e2e-scenarios.md`

Document the new cross-system knowledge path only after the test exists. Correct any stale totals, backlog summaries, or scenario inventory that become inaccurate once Scenario 22 lands.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Changing Tell payload semantics or planner semantics beyond what the test directly proves
- Refactoring office candidate generation or social ranking for unrelated cases
- Manually seeding the listener’s office belief as a substitute for the real Tell path
- Adding new political or social mechanics beyond the scenario itself
- Weakening the current same-place Tell suppression rule without a dedicated recipient-knowledge architecture

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent golden_tell_propagates_political_knowledge`
2. `cargo test -p worldwake-ai --test golden_emergent golden_tell_propagates_political_knowledge_replays_deterministically`
3. `cargo test -p worldwake-ai social_candidates_emit_for_live_colocated_listeners_and_relayable_subjects`
4. `cargo test -p worldwake-ai social_candidates_skip_subjects_already_known_to_be_colocated`
5. `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts`
6. `cargo test -p worldwake-ai --test golden_social golden_agent_autonomously_tells_colocated_peer`
7. Existing suite: `cargo test -p worldwake-ai --test golden_emergent`

### Invariants

1. An agent must not generate `ClaimOffice` for an office it does not lawfully know about through its belief store.
2. Political knowledge transfer remains local and explicit: the listener’s office belief must arrive through a committed Tell action, not through omniscient setup or planner access to world truth.
3. Remote office knowledge received through Tell must unlock the same ordinary office-claim path as a manually seeded report: candidate generation, travel, action execution, and succession still use existing political systems.
4. Same-seed replay remains deterministic at both world-hash and event-log-hash level.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs` — add the remote-office Tell-to-political-emergence scenario and replay companion with decision/action trace assertions.
2. `docs/golden-e2e-coverage.md` — record the added cross-system interaction and correct summary counts once the scenario exists.
3. `docs/golden-e2e-scenarios.md` — add the scenario catalog entry describing the remote knowledge path and trace assertions.

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_tell_propagates_political_knowledge`
2. `cargo test -p worldwake-ai --test golden_emergent golden_tell_propagates_political_knowledge_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_emergent`
4. `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts`
5. `cargo test -p worldwake-ai --test golden_social golden_agent_autonomously_tells_colocated_peer`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-18
- What actually changed:
  - Added `golden_tell_propagates_political_knowledge` and `golden_tell_propagates_political_knowledge_replays_deterministically` to `crates/worldwake-ai/tests/golden_emergent.rs`.
  - Updated `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` to record Scenario 22 and the current `golden_*` inventory totals.
- Deviations from original plan:
  - Kept `Engine Changes: None`, but only after reassessment corrected the scenario scope.
  - Did not weaken same-place Tell suppression in `crates/worldwake-ai/src/candidate_generation.rs`. Reassessment showed that approach introduced redundant local-gossip loops and regressed existing office coalition goldens.
  - Implemented the cleaner remote-office knowledge path instead: autonomous Tell transfers a remote office belief, then the listener follows the ordinary `ClaimOffice` -> travel -> `declare_support` -> succession path.
- Verification results:
  - Focused checks passed:
    - `cargo test -p worldwake-ai --test golden_emergent golden_tell_propagates_political_knowledge -- --exact`
    - `cargo test -p worldwake-ai --test golden_emergent golden_tell_propagates_political_knowledge_replays_deterministically -- --exact`
    - `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts -- --exact`
    - `cargo test -p worldwake-ai --test golden_social golden_agent_autonomously_tells_colocated_peer -- --exact`
  - Broader validation passed:
    - `cargo test -p worldwake-ai --test golden_emergent`
    - `cargo test -p worldwake-ai`
    - `cargo test --workspace`
    - `cargo clippy --workspace --all-targets -- -D warnings`
