# E17CRITHEJUS-016: Relay social evidence through Tell with typed tell topics

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — shared tell-topic/payload refactor across core/sim/systems/ai plus conversation-memory updates for relayable social evidence
**Deps**: E17CRITHEJUS-015

## Problem

The live `tell` action does not relay `SocialObservation` evidence. It only relays `known_entities` plus institutional beliefs keyed by `TellActionPayload { subject_entity }`. That means the current E17 witness-to-authority chain cannot exist as specified: a witness can observe a theft, but there is no lawful conversation path to transmit that social evidence to an owner or authority.

Without a relayable social-evidence topic, the remaining crime tickets would either:

1. overclaim that `SocialObservation(SuspectedTheft)` is already shareable when it is not, or
2. collapse crime testimony into unrelated entity-belief channels, which would violate P13 and P16 by losing the concrete evidence artifact.

## Assumption Reassessment (2026-03-25)

1. `crates/worldwake-core/src/belief.rs` already contains typed theft evidence. `SocialObservationDetail::SuspectedTheft { missing_entity, expected_place, suspect }` exists live, so this ticket must not claim that typed social-evidence detail still needs to be invented here.
2. `crates/worldwake-systems/src/tell_actions.rs` currently enumerates tell payloads from `listener_aware_relayable_subjects(view.known_entity_beliefs(actor), ...)` and produces `TellActionPayload { listener, subject_entity }`. Enumeration is entity-belief-only.
3. `validate_tell_payload_authoritatively()` currently requires `AgentBeliefStore::get_entity(&payload.subject_entity)` and enforces relay depth from that `BelievedEntityState`. It has no path for relaying `SocialObservation`.
4. `commit_tell()` currently transfers one `BelievedEntityState` snapshot plus relayable institutional beliefs for that same entity subject. It never copies `social_observations`, so witness testimony about theft cannot move through conversation today.
5. The entity-only shape is not confined to `tell_actions.rs`. The live shared boundary also hard-codes entity subjects in `crates/worldwake-sim/src/action_payload.rs` (`TellActionPayload`), `crates/worldwake-sim/src/belief_view.rs` (`RuntimeBeliefView::told_belief_memory`, `recipient_knowledge_status`), `crates/worldwake-sim/src/per_agent_belief_view.rs`, and `crates/worldwake-sim/src/social_relay.rs`.
6. Conversation memory is entity-subject-based today. `crates/worldwake-core/src/belief.rs` stores `TellMemoryKey { counterparty, subject }` and `ToldBeliefMemory` / `HeardBeliefMemory` snapshots for entity-belief sharing only. That is sufficient for entity-state gossip, not for discrete relayable evidence artifacts.
7. AI social candidate generation is coupled to the same entity-only surface. `crates/worldwake-ai/src/candidate_generation.rs` calls `listener_aware_relayable_subjects(ctx.view.known_entity_beliefs(...), ...)`, and planner payload construction in `crates/worldwake-ai/src/goal_model.rs` / `crates/worldwake-ai/src/planner_ops.rs` emits the same entity-only `TellActionPayload`.
8. Real current tell tests exist and should anchor the ticket instead of approximate claims. Verified live surfaces include `cargo test -p worldwake-systems tell -- --list` and `cargo test -p worldwake-ai social -- --list`, with focused tests such as `tell_actions::tests::tell_commit_records_speaker_told_belief_memory` and `candidate_generation::tests::social_candidates_emit_for_live_colocated_listeners_and_relayable_subjects`.
9. `E17CRITHEJUS-007`, `E17CRITHEJUS-008`, `E17CRITHEJUS-011`, and `E17CRITHEJUS-013` still depend on a shareable crime-evidence path that the live architecture does not provide.
10. This is a production contradiction, not merely a missing golden. `specs/E17-crime-theft-justice.md` explicitly requires physical propagation of crime knowledge through witness testimony and Tell.
11. Mismatch: the original scope under-described the refactor boundary and overstated missing typed evidence detail. Correct scope is to replace the entity-only tell topic model with a shared typed tell-topic boundary that can relay both entity beliefs and social observations before witness-driven accusation/punishment work proceeds.
12. The clean scope is not a theft-only fast path. The shared tell architecture should become topic-typed once, then reuse that boundary for theft evidence and future relayable social artifacts without another payload/memory rewrite.

## Architecture Check

1. The clean architecture is to make tell topics explicit and typed at the shared boundary, so conversation can carry either entity-belief snapshots or social-observation artifacts without smuggling one into the other.
2. This is better than adding a theft-specific branch to the existing `subject_entity` payload because crime testimony is not an entity snapshot. It is a distinct social artifact with its own provenance, tick, place, and typed detail.
3. This is also better than leaving `worldwake-sim` entity-shaped and only translating inside `tell_actions.rs`. That would fossilize an incorrect abstraction boundary and force every future social-artifact relay to special-case around sim/AI APIs.
4. No backwards-compatibility aliasing. Replace the entity-only tell topic model rather than layering a parallel ad hoc relay path beside it.

## Verification Layers

1. Tell affordance enumeration can surface relayable social-evidence topics in addition to entity topics -> focused candidate/runtime coverage in `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-systems/src/tell_actions.rs`
2. Authoritative tell validation accepts typed social-evidence topics only when the speaker actually holds that exact evidence artifact and relay depth is lawful -> focused authoritative runtime coverage in `crates/worldwake-systems/src/tell_actions.rs`
3. Tell commit transfers the expected social evidence into listener belief state with preserved provenance degradation and without converting testimony into direct observation -> focused authoritative runtime coverage in `crates/worldwake-systems/src/tell_actions.rs`
4. Conversation memory deduplicates entity topics and social-evidence topics independently -> focused unit/runtime coverage in `crates/worldwake-core/src/belief.rs`, `crates/worldwake-sim/src/per_agent_belief_view.rs`, and `crates/worldwake-systems/src/tell_actions.rs`
5. Planner payload construction and action binding still work after the typed tell payload refactor -> focused AI/planner coverage in `crates/worldwake-ai/src/goal_model.rs` / `crates/worldwake-ai/src/planner_ops.rs`
6. Witness-driven accusation candidate/action chains can consume relayed social evidence -> follow-on verification in `E17CRITHEJUS-011` and `E17CRITHEJUS-013`; not fully proved by this ticket alone

## What to Change

### 1. Replace entity-only tell payload topic with a shared typed tell topic

Refactor the tell payload/memory/view boundary away from raw `subject_entity`.

Recommended shape:

```rust
pub enum TellTopic {
    EntityBelief { subject: EntityId },
    SocialObservation { observation: SocialObservation },
}
```

`TellActionPayload`, `TellMemoryKey`, heard/told memory payloads, and the relevant `RuntimeBeliefView` methods should key on `TellTopic`, not just an entity id.

### 2. Extend belief-store and relay helpers for relayable social observations

Add explicit helpers to enumerate social observations that are lawful to relay, with provenance, freshness, and memory-dedup rules consistent with existing tell behavior.

At minimum, E17 needs relay for:

- witnessed theft evidence
- `SuspectedTheft` investigation aftermath

Design the API generally enough to support future relay of other social artifacts without another payload refactor.

### 3. Update tell enumeration, validation, commit, and planner payload construction

Update `crates/worldwake-systems/src/tell_actions.rs` so:

1. affordance enumeration can emit relayable social-evidence topics
2. authoritative payload validation checks the speaker actually holds the specific evidence topic
3. commit transfers that evidence into the listener's `AgentBeliefStore`
4. heard/told memory and recipient-knowledge checks avoid repeated retelling of the same unchanged evidence topic every tick

Update AI/runtime call sites so the typed tell topic flows cleanly through candidate generation, action payload construction, and runtime belief queries instead of translating ad hoc at one layer.

### 4. Keep provenance concrete

Relayed social evidence must preserve degraded provenance like existing entity-belief tell flow. Do not convert testimony into synthetic direct observation or world truth.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-sim/src/action_payload.rs` (modify)
- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-sim/src/social_relay.rs` (modify)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify if tell payload tests/builders depend on the old shape)
- Existing focused/integration/golden Tell tests (modify)

## Out of Scope

- Accuse action implementation itself
- Punishment actions
- Rewriting institutional-belief relay semantics beyond adapting them to the typed tell-topic boundary
- Wrong-accusation or contradictory-rumor policy beyond what naturally follows from typed social-evidence relay

## Acceptance Criteria

### Tests That Must Pass

1. Tell affordance enumeration includes relayable social-evidence topics when speaker has them
2. Tell payload validation rejects social-evidence topics the speaker does not hold
3. Tell commit transfers a typed theft-related social evidence record into listener belief state
4. Conversation-memory dedup prevents retelling the same unchanged social evidence topic every tick
5. Existing entity-belief tell behavior remains intact
6. Existing tell/social focused suites in `worldwake-systems`, `worldwake-ai`, and `worldwake-sim` remain green after the payload refactor
7. Existing suite: `cargo test -p worldwake-core`
8. Existing suite: `cargo test -p worldwake-sim`
9. Existing suite: `cargo test -p worldwake-systems`
10. Existing suite: `cargo test -p worldwake-ai`
11. Existing suite: `cargo build --workspace`
12. Existing suite: `cargo clippy --workspace`

### Invariants

1. Social evidence travels physically through conversation rather than global omniscience (P13)
2. Relayed testimony remains distinct from direct observation and preserves provenance degradation (P12, P14)
3. Conversation topics are typed and explicit at the shared boundary; no new tuple or positional alias conventions are introduced
4. The tell architecture stays general-purpose. Theft evidence uses the same lawful typed topic path as other future social observations rather than a crime-only exception

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — conversation-memory keying and recipient-knowledge status for typed tell topics
2. `crates/worldwake-sim/src/social_relay.rs` — topic enumeration/filtering helpers for entity and social relayables
3. `crates/worldwake-systems/src/tell_actions.rs` — relayable social-evidence enumeration, validation, transfer, provenance degradation, and dedup coverage
4. `crates/worldwake-ai/src/candidate_generation.rs` — social candidate emission over typed tell topics
5. `crates/worldwake-systems/tests/e15_information_integration.rs` or existing golden/integration tell coverage — extended only if needed to prove relayed social evidence without weakening current entity-belief coverage

### Commands

1. `cargo test -p worldwake-systems tell -- --list`
2. `cargo test -p worldwake-ai social -- --list`
3. `cargo test -p worldwake-core belief`
4. `cargo test -p worldwake-sim social_relay`
5. `cargo test -p worldwake-systems tell`
6. `cargo test -p worldwake-ai social`
7. `cargo test -p worldwake-core`
8. `cargo test -p worldwake-sim`
9. `cargo test -p worldwake-systems`
10. `cargo test -p worldwake-ai`
11. `cargo build --workspace`
12. `cargo clippy --workspace`

## Outcome

The original ticket assumption was corrected first: typed theft evidence already existed in `SocialObservationDetail::SuspectedTheft`, so the missing piece was not a new evidence type but an entity-only tell boundary that could not carry social observations.

Implemented outcome versus the original plan:

1. Replaced the shared tell boundary with typed topics (`TellTopic`) and typed shared tell state across core, sim, systems, and AI instead of adding a theft-only exception.
2. Added lawful relay of social observations through Tell, with concrete provenance degradation preserved and `WitnessedTelling` excluded from relayable/shared tell state to prevent gossip feedback loops.
3. Tightened tell dedup so repeated retells compare concrete shared content instead of treating provenance-only changes as novel content.
4. Extended direct-observability suppression to account for listener observation fidelity and same-place topic kinds, so colocated visible subjects are not redundantly told while blind listeners can still receive reports.
5. During verification, fixed an adjacent architecture gap in `declare_support`: self-declarations now immediately project into the speaker's own institutional belief state instead of waiting for external relay.
6. Corrected a stale combat golden that had been relying on incidental incumbent AI behavior rather than the intended force-succession invariant; the scenario now isolates the intended contract directly.

The result is better than the prior architecture because Tell now has a reusable typed topic boundary for concrete conversational artifacts instead of an entity-only API that would require new ad hoc branches for each future relayable social record.
