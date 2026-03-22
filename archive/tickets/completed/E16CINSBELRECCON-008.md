# E16CINSBELRECCON-008: Tell Integration — Institutional Claims Piggyback on Entity Tell Memory

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — extend shared tell-memory payload in worldwake-core and tell commit in worldwake-systems
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-003, E16CINSBELRECCON-004

## Problem

When an agent Tells another about an entity that carries institutional meaning such as an office, the related institutional claims must flow through the existing E15c conversation-memory path before projecting into the listener's `institutional_beliefs`. The current code still treats Tell as an entity-subject transfer and its conversation memory only snapshots `BelievedEntityState`. Without extending that shared tell content, institutional propagation would either bypass E15c entirely or remain invisible to resend suppression when only the institutional claim changed.

## Assumption Reassessment (2026-03-22)

1. `crates/worldwake-systems/src/tell_actions.rs` is the authoritative Tell mutation point and already records `ToldBeliefMemory` / `HeardBeliefMemory`, but it only transfers `BelievedEntityState` keyed by `TellActionPayload.subject_entity`. There is no institutional projection in the current Tell commit path.
2. E15c conversation memory lives in `crates/worldwake-core/src/belief.rs`, but the current mismatch is not just `TellMemoryKey`. `ToldBeliefMemory.shared_state` and `HeardBeliefMemory.heard_state` are `SharedBeliefSnapshot`, which currently carries only entity snapshot data. Extending only the key would not let resend suppression notice institutional-claim changes for the same entity subject.
3. The live Tell/AI surface is still entity-subject based: `GoalKind::ShareBelief { listener, subject }` in `crates/worldwake-core/src/goal.rs`, `TellActionPayload.subject_entity` in `crates/worldwake-sim/src/action_payload.rs`, `emit_social_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs`, and `build_payload_override()` in `crates/worldwake-ai/src/goal_model.rs`. This ticket should preserve that surface and extend the shared tell content for an entity subject rather than introducing a second institutional-only Tell identity path.
4. Current political emergence coverage still reflects the pre-`-012/-014` migration seam. `crates/worldwake-ai/tests/golden_emergent.rs` scenarios `golden_tell_propagates_political_knowledge` and `golden_same_place_office_fact_still_requires_tell` currently seed and assert office entity beliefs, while `crates/worldwake-ai/src/candidate_generation.rs` still reads `office_holder()` / `support_declaration()` through the legacy runtime seam. This ticket must not silently absorb that broader migration work; it should only make Tell able to carry institutional claims alongside the existing entity subject path.
5. N/A — no ordering.
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. Mismatch + correction: the original ticket scoped the work as "extend `TellMemoryKey` and `tell_actions.rs`." The real clean change is to extend shared tell content in `belief.rs` so conversation memory stores the institutionally relevant payload for an entity subject, then update `tell_actions.rs` to project those remembered claims into `institutional_beliefs` on acceptance. `TellMemoryKey` can stay entity-subject keyed if the shared content becomes rich enough to distinguish changed institutional payload.
12. N/A.

## Architecture Check

1. Routing institutional Tell through the existing entity-subject conversation-memory path is cleaner than adding a second institutional-only Tell subject or a `TellMemoryKey` branch keyed by institutional belief identifiers. One social action should share one concrete payload for one entity subject, and resend suppression should compare the full shared content for that subject.
2. No backward-compatibility shims.

## Verification Layers

1. Tell about an office entity with attached institutional claim -> listener gains institutional belief with `Report` source -> focused tell runtime/unit test
2. Institutional relay increments report chain length through two Tell hops -> focused tell runtime/unit test
3. Changed institutional payload for the same entity subject re-enables Tell instead of being suppressed as "already told current belief" -> focused resend-suppression unit test
4. Heard/Told conversation memory stores the institutionally extended shared payload before listener projection -> focused tell runtime/unit test
5. Additional AI/golden migration is not applicable in this ticket because the live `ShareBelief` goal and political-candidate read path stay unchanged here; those are covered by `-012` / `-014`.

## What to Change

### 1. Extend shared tell-memory payload in `belief.rs`

Extend `SharedBeliefSnapshot` so a Tell about an entity subject can also carry the speaker's shareable institutional claims anchored to that subject entity. Update resend-suppression comparison to use the full shared payload, not just the entity snapshot.

This preserves the existing `TellMemoryKey { counterparty, subject }` shape while fixing the real bug: if the office entity belief is unchanged but the institutional claim changed, the social layer must see that as new shareable content.

### 2. Extend Tell commit in `tell_actions.rs`

When a Tell commits for an entity subject:
- write the institutionally extended shared payload into `ToldBeliefMemory` / `HeardBeliefMemory`
- project the speaker's shareable institutional claims for that subject into the listener's `institutional_beliefs` on acceptance
- degrade institutional provenance to `InstitutionalKnowledgeSource::Report { from: speaker, chain_len }`
- treat repeated identical claims from the same source as already held rather than appending duplicate institutional belief entries

### 3. Keep Tell's subject identity unchanged in this ticket

Do not introduce a new institutional-only `GoalKind`, `TellActionPayload`, or alias path here. The subject remains the existing entity subject. Institutional claims piggyback on that entity's shared tell payload. Broader AI subject-model changes belong in a separate ticket only if the current entity-anchored model later proves insufficient.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — extend shared tell-memory payload and resend-suppression comparison)
- `crates/worldwake-systems/src/tell_actions.rs` (modify — transfer and project institutional claims during Tell commit)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify only if assertions need to prove institutional-belief arrival rather than entity-belief arrival)

## Out of Scope

- AI deciding WHAT institutional claims to Tell beyond the current entity-subject `ShareBelief` surface
- ConsultRecord action (ticket -005)
- Perception projection for witnesses (ticket -006)
- Record mutation (ticket -007)
- Full political candidate-generation migration off the legacy runtime seam (`-012` / `-014`)
- Non-institutional Tell subjects beyond keeping existing E15c behavior intact

## Acceptance Criteria

### Tests That Must Pass

1. Agent A Tells Agent B about an office entity while holding an institutional office-holder belief for that office -> B gains `InstitutionalClaim::OfficeHolder` with `InstitutionalKnowledgeSource::Report { from: A, chain_len: 1 }`
2. Agent B relays the same office fact to Agent C -> C gains the claim with `InstitutionalKnowledgeSource::Report { from: B, chain_len: 2 }`
3. Resend suppression: once the full shared payload for an office entity has already been told to the same listener, unchanged content is suppressed; if the institutional claim changes while the entity snapshot stays the same, the Tell becomes shareable again
4. Conversation memory records the institutionally extended shared payload, proving the claim flowed through E15c state instead of a parallel direct projection path
5. Existing suite: `cargo test -p worldwake-systems tell_actions`
6. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Institutional social propagation remains a single Tell/conversation-memory channel for entity subjects; no institutional-only bypass path is introduced
2. Institutional report chain length monotonically increases through relays
3. Resend suppression compares the full shared payload for the entity subject, not just the entity snapshot
4. Provenance is always traceable by speaker and report chain length

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — add focused tests for institutional projection on Tell, two-hop institutional relay, and institutional-payload-sensitive resend suppression
2. `crates/worldwake-core/src/belief.rs` — add focused resend-suppression comparison coverage if needed for the extended shared tell payload
3. `crates/worldwake-ai/tests/golden_emergent.rs` — strengthen existing social-political golden assertions only if needed to prove institutional belief arrival rather than just office entity belief arrival

### Commands

1. `cargo test -p worldwake-systems tell_actions`
2. `cargo test -p worldwake-core belief`
3. `cargo test -p worldwake-ai golden_tell_propagates_political_knowledge -- --exact`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-22
- Actual changes:
  - Extended `SharedBeliefSnapshot` to carry institutionally shareable payload for an entity Tell subject.
  - Updated `AgentBeliefStore` resend suppression to compare the full entity-plus-institutional shared payload for that subject.
  - Updated `tell_actions.rs` so accepted Tells relay institutional claims into `institutional_beliefs`, degrade institutional provenance through `InstitutionalKnowledgeSource::Report`, and avoid duplicate identical claim insertion on repeat Tell.
  - Added focused regression coverage in `crates/worldwake-core/src/belief.rs` and `crates/worldwake-systems/src/tell_actions.rs`.
- Deviations from original plan:
  - Did not extend `TellMemoryKey`; the key remained `counterparty + entity subject`.
  - Did not add a new institutional-only Tell payload/goal surface; institutional claims piggyback on the existing entity-subject Tell path.
  - No golden test file changes were required because focused unit/runtime coverage captured the invariant and existing golden social-political coverage still passed unchanged.
- Verification results:
  - Passed `cargo test -p worldwake-core belief`
  - Passed `cargo test -p worldwake-systems tell_actions`
  - Passed `cargo test -p worldwake-ai golden_tell_propagates_political_knowledge -- --exact`
  - Passed `cargo clippy --workspace`
  - Passed `cargo test --workspace`
