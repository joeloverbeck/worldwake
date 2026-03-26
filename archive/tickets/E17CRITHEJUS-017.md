# E17CRITHEJUS-017: Promote institutional knowledge to first-class Tell topics

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — Tell topic/state refactor across core, sim, systems, AI, and political/crime golden coverage
**Deps**: E17CRITHEJUS-016

## Problem

Institutional knowledge still rides through Tell as sidecar data on `TellTopic::EntityBelief { subject: office_or_record }`. That means office-holder facts, force-control facts, support declarations, and future record-backed crime facts are not modeled as first-class conversational artifacts. Instead, they are smuggled inside an entity snapshot, which couples claim propagation to the physical observability of offices/records and forces direct-observability suppression to special-case `Office` and `Record` entity kinds.

That boundary is architecturally wrong for Worldwake. Institutional claims and record knowledge are social artifacts and records, not properties of “seeing an office entity.” Leaving them attached to entity tell topics weakens locality semantics, makes future CrimeRegister propagation brittle, and pushes more special cases into the shared Tell layer.

## Assumption Reassessment (2026-03-26)

1. `crates/worldwake-core/src/belief.rs` already defines `TellTopic::InstitutionalClaim { claim }`, `SharedTellState::InstitutionalClaim`, and typed memory-lane comparison helpers. The codebase is in a mixed state: the new topic exists, but the old entity-sidecar path still survives.
2. `AgentBeliefStore::shared_belief_snapshot_for_subject()` in `crates/worldwake-core/src/belief.rs` still injects `relayable_institutional_beliefs_for_subject(subject, ...)` into `SharedBeliefSnapshot.institutional_claims`. That means entity-topic sharing still aliases institutional content through the entity subject instead of the claim artifact.
3. `commit_tell()` in `crates/worldwake-systems/src/tell_actions.rs` already has a first-class `TellTopic::InstitutionalClaim { claim }` branch, but the `TellTopic::EntityBelief { subject }` branch still relays institutional claims as side effects. The architecture is therefore duplicated rather than replaced.
4. `tell_subject_is_directly_observable_by_listener()` in `crates/worldwake-core/src/belief.rs` no longer special-cases `EntityKind::Office` or `EntityKind::Record`; direct observability is already limited to agent/item/container-like entities. The remaining architectural mismatch is not direct-observability logic but the legacy entity-sidecar Tell path.
5. The exact shared abstraction boundary under audit is the `GoalKind::ShareBelief { listener, topic }` data path: `TellTopic` -> `SharedTellState` -> `TellMemoryKey` / heard-told memory -> relay enumeration in `crates/worldwake-sim/src/social_relay.rs` -> authoritative validation/commit in `crates/worldwake-systems/src/tell_actions.rs` -> AI candidate, ranking, planning-state, and trace surfaces in `crates/worldwake-ai/src/*.rs`.
6. The live goal family under test is `GoalKind::ShareBelief`. The current operator surface is `enumerate_tell_payloads()` / `validate_tell_payload_authoritatively()` / `commit_tell()` in `crates/worldwake-systems/src/tell_actions.rs`, with AI emission in `emit_social_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs` and recipient-memory reconstruction in `crates/worldwake-ai/src/planning_state.rs` and `crates/worldwake-sim/src/per_agent_belief_view.rs`.
7. Focused runtime coverage already encodes the mixed architecture. `cargo test -p worldwake-systems tell -- --list` shows `tell_actions::tests::tell_commit_projects_institutional_claims_and_records_them_in_heard_memory`, `tell_actions::tests::tell_commit_relays_force_control_claims`, `tell_actions::tests::tell_affordances_allow_same_place_office_entity_topics_without_claim_sidecars`, and `tell_actions::tests::tell_affordances_include_local_institutional_claim_topics_even_when_office_is_visible`.
8. Golden coverage also depends on office facts traveling as entity tell topics. Verified by `cargo test -p worldwake-ai --test golden_emergent -- --list` and `cargo test -p worldwake-ai --test golden_offices -- --list`, including `golden_tell_propagates_political_knowledge`, `golden_same_place_office_fact_still_requires_tell`, `golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact`, and `golden_force_control_locality_requires_tell`.
9. Additional live coupling exists outside the original file list: `crates/worldwake-ai/src/ranking.rs` and `crates/worldwake-sim/src/social_relay.rs` both encode tell-topic ordering, and `crates/worldwake-ai/src/planning_state.rs` reconstructs recipient knowledge from told-memory lanes. Those surfaces must be reassessed together so the refactor does not leave political action starved by newly promoted tell chatter.
10. Remaining active crime tickets (`E17CRITHEJUS-008`, `E17CRITHEJUS-011`, `E17CRITHEJUS-013`) consume institutional knowledge or record state, but none of them owns the Tell-boundary refactor. If this cleanup is not explicitly ticketed, those tickets risk adding more office/record entity-topic coupling around CrimeRegister flows.
11. This refactor aligns with `docs/FOUNDATIONS.md`: P3/P16/P23 require concrete records and social artifacts; P7 requires lawful communication paths; P21 requires offices/institutions to act through records and roles rather than omniscient shortcuts; P24 forbids papering over the boundary with cross-system special cases.
12. Mismatch + correction: after `E17CRITHEJUS-016`, Tell is typed for social observations but still not typed for institutional artifacts. The current architecture is an intermediate improvement, not the final clean boundary, and the ticket scope must include all `ShareBelief` consumers that currently hard-code the two-topic model.

## Architecture Check

1. The clean architecture is to make institutional knowledge a first-class Tell topic/state, parallel to entity beliefs and social observations. A support declaration, office-holder claim, force-control claim, or crime-register claim should travel through conversation as the claim artifact itself.
2. This is better than keeping institutional claims as attachments on entity snapshots because claim relay and claim dedup should be keyed by the claim content, not by whether an office or record entity happened to be the subject of the conversation.
3. This is also better than adding more office-specific or record-specific exceptions in affordance filtering. Offices and records remain concrete entities in world state, but their institutional contents should be shared through institutional topics or direct record consultation, not through “you can see the office, therefore you know the claim.”
4. No backwards-compatibility aliasing. Replace the entity-sidecar institutional Tell path rather than carrying both architectures indefinitely.

## Verification Layers

1. Institutional Tell affordance enumeration emits first-class institutional topics independently of entity topics -> focused `social_relay.rs`, `tell_actions.rs`, and `candidate_generation.rs` tests
2. Authoritative payload validation accepts an institutional topic only when the speaker actually holds that claim and its relay depth is lawful -> focused `tell_actions.rs` runtime tests
3. Tell commit transfers institutional knowledge without mutating or depending on an entity snapshot branch -> authoritative `AgentBeliefStore` state assertions in `tell_actions.rs`
4. Conversation-memory dedup and recipient-knowledge reconstruction compare institutional topic content directly and no longer rely on office/record entity topics -> focused `belief.rs`, `per_agent_belief_view.rs`, and `planning_state.rs` tests
5. `GoalKind::ShareBelief` ranking/trace surfaces remain coherent after the new topic variant is introduced -> focused `ranking.rs` and `decision_trace.rs` tests
6. Same-place suppression for institutional topics depends on actual listener knowledge / consultation state, not `EntityKind::Office` or `EntityKind::Record` “direct observability” -> focused affordance tests
7. Existing political locality scenarios still pass after the refactor -> `golden_emergent` / `golden_offices` goldens

## What to Change

### 1. Add first-class institutional Tell topic/state

Extend the shared Tell boundary in core with an institutional topic/state variant. The exact type shape may be refined during implementation, but the boundary must be claim-first, not entity-first. A concrete direction is:

```rust
pub enum TellTopic {
    EntityBelief { subject: EntityId },
    SocialObservation { observation: SocialObservation },
    InstitutionalClaim { claim: InstitutionalClaim },
}
```

`SharedTellState`, `TellMemoryKey`, heard/told memory comparisons, and runtime/planning view helpers should all support the institutional variant directly.

### 2. Move institutional relay enumeration off entity subjects

Refactor `AgentBeliefStore` and sim relay helpers so relayable institutional topics are enumerated from institutional belief memory itself rather than via `relayable_institutional_beliefs_for_subject(subject, ...)` embedded in an entity snapshot path.

Derived helper views that answer “which claims mention this office / record / supporter / accused?” are fine, but they must become filters over institutional topics, not the authoritative Tell boundary.

### 3. Refactor Tell validation, commit, and suppression rules

Update systems/runtime/AI call sites so:

1. affordance enumeration emits institutional topics directly
2. authoritative validation checks speaker possession of the exact institutional claim topic
3. commit transfers the claim with degraded institutional provenance and records typed heard/told memory for that institutional topic
4. direct-observability suppression no longer treats `Office` or `Record` entities as a proxy for institutional knowledge; those cases should be handled by explicit listener knowledge checks or record-consultation semantics

### 4. Rebase political and crime tests on the clean boundary

Update focused tests and goldens that currently use office/record entity tell topics when what they are really sharing is institutional knowledge. The goal is not to remove locality requirements, but to make the shared artifact explicit.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify)
- `crates/worldwake-core/src/goal.rs` (modify if `GoalKind::ShareBelief` topic types need updates)
- `crates/worldwake-sim/src/action_payload.rs` (modify)
- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-sim/src/social_relay.rs` (modify)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify if payload constructors require it)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify if tell payload binding depends on old topic shape)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify if told-memory snapshot serialization assumes two Tell variants)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify)

## Out of Scope

- New institutional claim types
- Crime accusation/fine/exile behavior changes beyond adapting to the new Tell boundary
- Record-consult action redesign
- Global “institutional reputation” abstractions or summary scores

## Acceptance Criteria

### Tests That Must Pass

1. Tell affordance enumeration can emit institutional claim topics without wrapping them in an office/record entity topic
2. Tell payload validation rejects institutional topics the speaker does not hold
3. Tell commit transfers institutional claim knowledge with degraded provenance through the institutional topic branch
4. Conversation-memory dedup for institutional topics is keyed by institutional claim content rather than entity subject identity
5. `tell_subject_is_directly_observable_by_listener()` no longer needs `Office` or `Record` special cases to preserve political/crime locality behavior
6. Existing political/locality goldens still pass after the refactor
7. Existing suite: `cargo test -p worldwake-systems tell -- --list`
8. Existing suite: `cargo test -p worldwake-ai --test golden_offices -- --list`
9. Existing suite: `cargo test -p worldwake-ai --test golden_emergent -- --list`
10. Existing suite: `cargo test -p worldwake-systems tell`
11. Existing suite: `cargo test -p worldwake-ai`
12. Existing suite: `cargo build --workspace`
13. Existing suite: `cargo clippy --workspace`

### Invariants

1. Institutional knowledge remains concrete world-state-derived social information, not a hidden side effect of office/entity observation (P3, P16, P23)
2. Communication locality is preserved: claims still travel through conversation or consultation, never through global truth (P7, P13)
3. Offices and records remain world entities, but their institutional contents are shared through explicit institutional topics or record consultation, not entity-topic aliasing (P21, P24)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — remove entity-sidecar institutional tell content and keep conversation-memory equality claim-first. Rationale: prove the old alias path is gone rather than merely de-emphasized.
2. `crates/worldwake-sim/src/social_relay.rs` — preserve freshness-first relay truncation while supporting institutional topics. Rationale: adding a third topic kind must not silently change the existing “freshest lawful tell first” contract.
3. `crates/worldwake-systems/src/tell_actions.rs` — validate and commit institutional topics directly, plus assert that plain entity tells no longer piggyback institutional claims. Rationale: authoritative mutation must match the cleaned boundary.
4. `crates/worldwake-ai/src/candidate_generation.rs` — emit institutional topics directly and avoid office/record entity-topic duplication when the claim-first topic exists. Rationale: the planner should see the real conversational artifact, not two aliases for the same fact.
5. `crates/worldwake-ai/src/planning_state.rs` or `crates/worldwake-sim/src/per_agent_belief_view.rs` — reconstruct stale/current tell memory by institutional memory lane instead of exact entity-topic identity. Rationale: replanning and dedup must remain stable after the boundary change.
6. `crates/worldwake-ai/src/ranking.rs` — keep actionable political goals above tell chatter after institutional topics become first-class. Rationale: the refactor must not cause agents to gossip forever instead of finishing office succession.
7. `crates/worldwake-ai/tests/golden_emergent.rs` — keep political-knowledge locality scenarios passing on the cleaned Tell boundary. Rationale: the refactor must preserve lawful information travel.
8. `crates/worldwake-ai/tests/golden_offices.rs` — keep office/force-control and bribe-coalition scenarios passing. Rationale: the cleaned boundary must still support real political follow-through, not just unit-level topic plumbing.

### Commands

1. `cargo test -p worldwake-core belief`
2. `cargo test -p worldwake-sim social_relay`
3. `cargo test -p worldwake-systems tell`
4. `cargo test -p worldwake-ai ranking::tests::support_candidate_uses_social_weight_times_loyalty -- --exact`
5. `cargo test -p worldwake-ai --test golden_emergent golden_tell_propagates_political_knowledge -- --exact`
6. `cargo test -p worldwake-ai --test golden_offices golden_force_control_locality_requires_tell -- --exact`
7. `cargo test -p worldwake-ai --test golden_offices golden_bribe_support_coalition -- --exact`
8. `cargo test -p worldwake-ai`
9. `cargo build --workspace`
10. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-26
- What changed:
  - Removed institutional claim sidecars from `SharedBeliefSnapshot` and deleted the legacy entity-subject institutional relay path.
  - Kept institutional knowledge on the first-class `TellTopic::InstitutionalClaim { claim }` path for affordance enumeration, validation, commit, planning-state reconstruction, and heard/told memory comparison.
  - Preserved freshness-first tell selection in `social_relay.rs` and added regression coverage so the new topic kind does not reorder older entity topics ahead of fresher ones.
  - Added authoritative coverage proving plain entity tells no longer piggyback institutional claims.
  - Promoted `SupportCandidateForOffice` from `Low` to `Medium` priority so first-class institutional tell chatter does not starve actual political follow-through such as the bribe coalition office-installation path.
- Deviations from original plan:
  - `tell_subject_is_directly_observable_by_listener()` had already been cleaned up before this completion work; no new direct-observability special-case removal was needed.
  - Plain office entity beliefs remain tellable as plain entity topics when they carry actual entity-state information; only the institutional-content alias path was removed.
  - `decision_trace.rs` and `planning_snapshot.rs` did not require net-new source edits after reassessment because the live variant support there was already sufficient once the boundary cleanup landed in core/runtime/ranking.
- Verification:
  - `cargo test -p worldwake-core belief`
  - `cargo test -p worldwake-sim social_relay`
  - `cargo test -p worldwake-systems tell`
  - `cargo test -p worldwake-ai ranking::tests::support_candidate_uses_social_weight_times_loyalty -- --exact`
  - `cargo test -p worldwake-ai --test golden_emergent golden_tell_propagates_political_knowledge -- --exact`
  - `cargo test -p worldwake-ai --test golden_offices golden_force_control_locality_requires_tell -- --exact`
  - `cargo test -p worldwake-ai --test golden_offices golden_bribe_support_coalition -- --exact`
  - `cargo test -p worldwake-ai`
  - `cargo build --workspace`
  - `cargo clippy --workspace`
