# E17CRITHEJUS-016: Relay social evidence through Tell with typed conversation topics

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — tell payload/topic refactor plus conversation-memory updates for relayable social evidence
**Deps**: E17CRITHEJUS-015

## Problem

The live `tell` action does not relay `SocialObservation` evidence. It only relays `known_entities` plus institutional beliefs keyed by `TellActionPayload { subject_entity }`. That means the current E17 witness-to-authority chain cannot exist as specified: a witness can observe a theft, but there is no lawful conversation path to transmit that social evidence to an owner or authority.

Without a relayable social-evidence topic, the remaining crime tickets would either:

1. overclaim that `SocialObservation(SuspectedTheft)` is already shareable when it is not, or
2. collapse crime testimony into unrelated entity-belief channels, which would violate P13 and P16 by losing the concrete evidence artifact.

## Assumption Reassessment (2026-03-25)

1. `crates/worldwake-systems/src/tell_actions.rs` currently enumerates tell payloads from `listener_aware_relayable_subjects(view.known_entity_beliefs(actor), ...)` and produces `TellActionPayload { listener, subject_entity }`.
2. `validate_tell_payload_authoritatively()` requires `AgentBeliefStore::get_entity(&payload.subject_entity)`; it has no path for relaying `SocialObservation`.
3. `commit_tell()` transfers one `BelievedEntityState` snapshot plus relayable institutional beliefs for the same entity subject. It does not copy `social_observations` or any typed evidence record.
4. Conversation memory keys (`TellMemoryKey`) and heard/told memory snapshots in `crates/worldwake-core/src/belief.rs` are entity-subject based today. That is sufficient for entity-state gossip, not for testimony about discrete social events or accusations.
5. `E17CRITHEJUS-007`, `E17CRITHEJUS-008`, `E17CRITHEJUS-011`, and `E17CRITHEJUS-013` all currently assume a shareable crime-evidence path that the live architecture does not provide.
6. This is a production contradiction, not merely a missing golden. The spec explicitly requires physical propagation of crime knowledge through witness testimony and Tell.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. Mismatch: the current ticket set treats Tell as if it can already relay social evidence. The live code cannot. Correct scope is to add typed relay topics and conversation-memory support before witness-driven accusation/punishment work proceeds.
12. N/A.

## Architecture Check

1. The clean architecture is to make Tell topics explicit and typed, so conversation can carry either entity-state beliefs or social evidence artifacts without smuggling one into the other.
2. This is better than special-casing theft inside the existing `subject_entity` payload because crime testimony is not an entity snapshot. It is a distinct social fact with its own provenance and fields.
3. No backwards-compatibility aliasing. Replace the entity-only topic model rather than layering a parallel ad hoc relay path beside it.

## Verification Layers

1. Tell affordance enumeration can surface relayable social-evidence topics in addition to entity topics -> focused runtime coverage in `crates/worldwake-systems/src/tell_actions.rs`
2. Authoritative tell validation accepts typed social-evidence topics only when the speaker actually holds that evidence -> focused authoritative runtime coverage
3. Tell commit transfers the expected social evidence into listener belief state with preserved provenance degradation -> focused authoritative runtime coverage
4. Conversation memory deduplicates entity topics and social-evidence topics independently -> focused unit/runtime coverage in `crates/worldwake-core/src/belief.rs` and `crates/worldwake-systems/src/tell_actions.rs`
5. Witness-driven accusation candidate/action chains can consume relayed social evidence -> follow-on verification in `E17CRITHEJUS-011` and `E17CRITHEJUS-013`; not fully proved by this ticket alone

## What to Change

### 1. Replace entity-only Tell payload topic with a typed conversation topic

Refactor Tell payloads away from raw `subject_entity`.

Recommended shape:

```rust
pub enum TellTopic {
    EntityBelief { subject: EntityId },
    SocialEvidence { detail: SocialObservationDetail },
}
```

Conversation-memory keys and heard/told memory payloads should key on `TellTopic`, not just an entity id.

### 2. Extend belief-store helpers for relayable social evidence

Add explicit belief-store queries/helpers to enumerate social evidence that is lawful to relay, with provenance and freshness rules consistent with existing tell-memory discipline.

At minimum, E17 needs relay for:

- witnessed theft evidence
- `SuspectedTheft` investigation aftermath

Design the API generally enough to support future relay of other social artifacts without another payload refactor.

### 3. Update tell enumeration, validation, and commit

Update `crates/worldwake-systems/src/tell_actions.rs` so:

1. affordance enumeration can emit relayable social-evidence topics
2. authoritative payload validation checks the speaker actually holds the specific evidence topic
3. commit transfers that evidence into the listener's `AgentBeliefStore`
4. heard/told memory and recipient-knowledge checks avoid repeated retelling of the same unchanged evidence topic

### 4. Keep provenance concrete

Relayed social evidence must preserve degraded provenance like existing entity-belief tell flow. Do not convert testimony into synthetic direct observation or world truth.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify if relayable-topic enumeration or knowledge-status helpers are consulted there)
- Existing focused/golden Tell tests (modify)

## Out of Scope

- Accuse action implementation itself
- Punishment actions
- Rewriting institutional-belief relay architecture
- Wrong-accusation or contradictory-rumor policy beyond what naturally follows from typed social-evidence relay

## Acceptance Criteria

### Tests That Must Pass

1. Tell affordance enumeration includes relayable social-evidence topics when speaker has them
2. Tell payload validation rejects social-evidence topics the speaker does not hold
3. Tell commit transfers a typed theft-related social evidence record into listener belief state
4. Conversation-memory dedup prevents retelling the same unchanged social evidence topic every tick
5. Existing entity-belief tell behavior remains intact
6. Existing suite: `cargo test -p worldwake-core`
7. Existing suite: `cargo test -p worldwake-systems`
8. Existing suite: `cargo test -p worldwake-ai`
9. Existing suite: `cargo build --workspace`
10. Existing suite: `cargo clippy --workspace`

### Invariants

1. Social evidence travels physically through conversation rather than global omniscience (P13)
2. Relayed testimony remains distinct from direct observation and preserves provenance degradation (P12, P14)
3. Conversation topics are typed and explicit; no new tuple or positional alias conventions are introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — relayable social-evidence enumeration, validation, transfer, and dedup coverage
2. `crates/worldwake-core/src/belief.rs` — conversation-memory keying/retention for typed tell topics
3. Existing golden Tell scenarios updated or extended only if needed to prove social-evidence relay without weakening current entity-belief coverage

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-systems`
3. `cargo test -p worldwake-ai`
4. `cargo build --workspace`
5. `cargo clippy --workspace`
