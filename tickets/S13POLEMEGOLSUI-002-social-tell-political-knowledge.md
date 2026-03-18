# S13POLEMEGOLSUI-002: Social Tell Propagates Political Knowledge Into Office Claim

**Status**: PENDING
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
4. Current test helpers already cover the needed setup surface in `crates/worldwake-ai/tests/golden_harness/mod.rs`: `seed_office`, `set_agent_tell_profile`, `set_agent_perception_profile`, `seed_actor_local_beliefs`, `agent_belief_about`, plus decision/action tracing. No new engine mechanism should be required.

## Architecture Check

1. Keep this as a cross-system emergent scenario in `crates/worldwake-ai/tests/golden_emergent.rs` so the suite proves that political action availability can arise from lawful information transfer rather than setup injection.
2. The clean design is to assert the knowledge-path boundary explicitly: no `ClaimOffice` before Tell, then ordinary candidate generation and succession after Tell. That preserves Principles 7, 13, and 24 without adding a political special case inside the social system.

## What to Change

### 1. Add the social-to-political emergence golden scenario

Add `golden_tell_propagates_political_knowledge` and `golden_tell_propagates_political_knowledge_replays_deterministically` to `crates/worldwake-ai/tests/golden_emergent.rs`.

The scenario should:
- Create a support-law vacant office at `VillageSquare`.
- Give the informant direct office knowledge plus a Tell-capable profile.
- Start the ambitious listener without office knowledge.
- Use decision traces to prove the listener has no political candidate before the tell.
- Use the real Tell action lifecycle to transfer the office belief.
- Assert the listener later generates `ClaimOffice`, commits `declare_support`, and becomes office holder.
- Assert semantic ordering that Tell commits before political action/installation.

### 2. Update golden E2E documentation in the same ticket

Review and update the relevant `docs/golden-e2e*` docs after the scenario is implemented.

At minimum:
- `docs/golden-e2e-coverage.md`
- `docs/golden-e2e-scenarios.md`

Document the new cross-system knowledge path only after the test exists. Do not reserve future suite totals or scenario summaries.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Changing Tell payload semantics or planner semantics beyond what the test directly proves
- Refactoring office candidate generation or social ranking for unrelated cases
- Manually seeding the listener’s office belief as a substitute for the real Tell path
- Adding new political or social mechanics beyond the scenario itself

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_emergent golden_tell_propagates_political_knowledge`
2. `cargo test -p worldwake-ai --test golden_emergent golden_tell_propagates_political_knowledge_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts`
4. `cargo test -p worldwake-ai --test golden_social golden_agent_autonomously_tells_colocated_peer`
5. Existing suite: `cargo test -p worldwake-ai --test golden_emergent`

### Invariants

1. An agent must not generate `ClaimOffice` for an office it does not lawfully know about through its belief store.
2. Political knowledge transfer remains local and explicit: the listener’s office belief must arrive through a committed Tell action, not through omniscient setup or planner access to world truth.
3. The ordinary office-claim path remains unchanged after knowledge arrival: candidate generation, planning, action execution, and succession still use existing political systems.
4. Same-seed replay remains deterministic at both world-hash and event-log-hash level.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs` — add the Tell-to-political-emergence scenario and replay companion with decision/action trace assertions.
2. `docs/golden-e2e-coverage.md` — record the added cross-system interaction and updated suite totals.
3. `docs/golden-e2e-scenarios.md` — add the scenario catalog entry describing the knowledge path and trace assertions.

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_tell_propagates_political_knowledge`
2. `cargo test -p worldwake-ai --test golden_emergent golden_tell_propagates_political_knowledge_replays_deterministically`
3. `cargo test -p worldwake-ai --test golden_emergent`
4. `cargo test -p worldwake-ai --test golden_offices golden_information_locality_for_political_facts`
5. `cargo test -p worldwake-ai --test golden_social golden_agent_autonomously_tells_colocated_peer`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`
