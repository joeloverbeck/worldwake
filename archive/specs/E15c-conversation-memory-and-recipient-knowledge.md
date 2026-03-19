**Status**: COMPLETED

# E15c: Conversation Memory & Recipient Knowledge for Social Telling

## Epic Summary

Replace the current speaker-side same-place Tell suppression shortcut with explicit per-agent conversation memory that records what an agent has told or heard, and use that memory to suppress redundant social chatter.

This spec does **not** add omniscient second-order belief or a hidden anti-spam cache. It adds concrete local memory artifacts to `AgentBeliefStore` so a speaker can reason from:

- what they currently believe about a subject
- what they remember having already told a specific listener about that subject
- whether the current belief has materially changed since that tell

The result is a cleaner architecture than the current `emit_social_candidates()` heuristic in `crates/worldwake-ai/src/candidate_generation.rs`, which suppresses telling when the speaker merely believes the subject is already at the current place. That heuristic is only a proxy for recipient knowledge and is not grounded in explicit social memory.

This reassessment tightens the original proposal in four places the draft left underspecified:

- resend suppression must be listener-aware before per-tick subject truncation, otherwise already-told recent subjects can crowd out untold older subjects
- conversation-memory retention must be enforced on reads as well as writes, otherwise stale tell memory can suppress forever until some later maintenance pass happens to run
- lawful retelling must compare a speaker's current belief against remembered shared content, not raw `BelievedEntityState` equality over bookkeeping fields such as `observed_tick`
- social omission and resend reasons should be visible in decision traces, not only inferred from the absence of a generated goal

## Phase

Phase 3: Information & Politics

## Crates

`worldwake-core`
- extend `AgentBeliefStore` with explicit conversation memory
- add social-memory query and retention types
- extend `TellProfile` with social-memory policy

`worldwake-sim`
- extend belief/runtime query traits for actor-local conversation memory; planning snapshot persistence remains optional unless later stages truly need it

`worldwake-systems`
- update Tell commit semantics to record outbound and inbound conversation memory

`worldwake-ai`
- replace same-place Tell suppression with explicit resend suppression based on conversation memory
- add focused and golden verification coverage for resend suppression and lawful retelling after belief change

## Dependencies

- E14 (belief stores, provenance, retention, local perception)
- E15 (Tell action, rumor propagation, social observations)
- E15b (social AI goals, `ShareBelief`, social ranking/candidate generation)

## Why This Exists

Current social AI has one architectural seam that is too weak:

1. `emit_social_candidates()` suppresses some `ShareBelief` candidates by checking whether the **speaker's** belief says the subject is already at the current place.
2. That is not the same thing as the **listener** knowing the subject.
3. The heuristic prevents some redundant chatter, but it does so through an implicit shortcut rather than explicit social memory.
4. When we tried weakening or removing that shortcut without replacement, we exposed repeated local-gossip loops and goldens regressed.

The current behavior is therefore pragmatically useful but architecturally wrong.

This spec replaces it with a lawful, extensible model:

- conversations leave memory artifacts
- speakers remember what they have already told specific listeners
- listeners remember what they heard and whether they accepted it
- resend suppression becomes a query over stored local memory, not a proxy over world truth

## Foundational Alignment

This spec exists to satisfy the following non-negotiable principles in [FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md):

- Principle 3: concrete state over abstract scores
- Principle 7: locality of interaction and communication
- Principle 12: world state is not belief state
- Principle 13: knowledge is acquired locally and travels physically
- Principle 14: ignorance, uncertainty, and contradiction are first-class
- Principle 16: memory and evidence are world state
- Principle 18: decisions must be explainable from local belief and priorities
- Principle 20: agent diversity through concrete variation
- Principle 23: social artifacts are first-class
- Principle 24: systems interact through state, not through each other
- Principle 26: no backward compatibility in live authority paths
- Principle 27: debuggability is a product feature

## Design Goals

1. Redundant telling must be suppressed through explicit local memory, not through a speaker-side world-truth shortcut.
2. The architecture must support lawful retelling when the speaker's belief about a subject has materially changed.
3. The design must remain first-order and bounded. It must not become a general nested-belief or theory-of-mind system.
4. Tell participation memory must be stored in ordinary agent belief state so it can later support trust, annoyance, contradiction resolution, and richer dialogue mechanics.
5. The same architecture should extend naturally to future institutional, legal, and market information exchange without creating a second social-memory subsystem.
6. Per-listener resend suppression must not accidentally reduce the set of distinct tellable subjects available in the same tick.
7. Retention expiry must be deterministic and must not depend on unrelated future tell actions happening to trigger cleanup.

## Non-Goals

1. Full arbitrary higher-order belief (`A believes that B believes that C believes ...`)
2. Modeling attention or guaranteed listener awareness from mere co-location
3. A dialogue tree, conversation scheduler, or social reputation rewrite
4. Solving all future social coordination problems in one spec
5. Preserving both the current same-place heuristic and the new conversation-memory architecture in parallel

## Existing Infrastructure (Leveraged, Not Reimplemented)

| Infrastructure | Location | Usage in E15c |
|----------------|----------|---------------|
| `AgentBeliefStore` | `crates/worldwake-core/src/belief.rs` | gains first-class conversation memory |
| `BelievedEntityState` | E14 belief system | source for remembered shared-content snapshots |
| `PerceptionSource` | E14/E15 | preserves provenance on remembered shared/heard beliefs |
| `TellProfile` | E15 | extended with conversation-memory policy |
| `social_observations` | E14/E15 | remains the place for witnessed social acts; conversation memory is participant-local |
| `GoalKind::ShareBelief` | E15b | candidate generation will query conversation memory |
| `tell` action commit | E15 | becomes the authoritative mutation point for tell-memory writes |
| decision trace + golden harness | `worldwake-ai/tests` | verify resend suppression and lawful retelling |

## Deliverables

### 1. First-Class Conversation Memory in `AgentBeliefStore`

Extend `AgentBeliefStore` with participant-local tell memory:

```rust
pub struct AgentBeliefStore {
    pub known_entities: BTreeMap<EntityId, BelievedEntityState>,
    pub social_observations: Vec<SocialObservation>,
    pub told_beliefs: BTreeMap<TellMemoryKey, ToldBeliefMemory>,
    pub heard_beliefs: BTreeMap<TellMemoryKey, HeardBeliefMemory>,
}
```

```rust
pub struct TellMemoryKey {
    pub counterparty: EntityId,
    pub subject: EntityId,
}
```

```rust
pub struct ToldBeliefMemory {
    pub counterparty: EntityId,
    pub subject: EntityId,
    pub shared_state: SharedBeliefSnapshot,
    pub told_tick: Tick,
}
```

```rust
pub struct HeardBeliefMemory {
    pub counterparty: EntityId,
    pub subject: EntityId,
    pub heard_state: SharedBeliefSnapshot,
    pub heard_tick: Tick,
    pub disposition: HeardBeliefDisposition,
}
```

```rust
pub enum HeardBeliefDisposition {
    Accepted,
    Rejected,
    AlreadyHeldEqualOrNewer,
    NotInternalized,
}
```

```rust
pub struct SharedBeliefSnapshot {
    pub last_known_place: Option<EntityId>,
    pub last_known_inventory: BTreeMap<CommodityKind, Quantity>,
    pub workstation_tag: Option<WorkstationTag>,
    pub resource_source: Option<ResourceSource>,
    pub alive: bool,
    pub wounds: Vec<Wound>,
    pub last_known_courage: Option<Permille>,
    pub source: PerceptionSource,
}
```

Rules:

1. `told_beliefs` stores what **this agent remembers telling** a specific counterparty about a specific subject.
2. `heard_beliefs` stores what **this agent remembers hearing** from a specific counterparty about a specific subject.
3. These are local memory artifacts, not shared truth.
4. `shared_state` and `heard_state` are content snapshots, not full listener-truth state. They deliberately exclude `observed_tick` so mere local refresh of the same underlying belief content does not force a resend.
5. The speaker-side remembered content is derived from the speaker's current belief content, not from the degraded listener-side transfer timestamp.
6. `SharedBeliefSnapshot` should stay structurally close to `BelievedEntityState` and be maintained beside it, not become an independent social-only schema with divergent meaning.

### 2. Conversation Memory Policy on `TellProfile`

Extend `TellProfile` with explicit conversation-memory policy:

```rust
pub struct TellProfile {
    pub max_tell_candidates: u8,
    pub max_relay_chain_len: u8,
    pub acceptance_fidelity: Permille,
    pub conversation_memory_capacity: u16,
    pub conversation_memory_retention_ticks: u64,
}
```

Purpose:

- `conversation_memory_capacity` bounds stored `told_beliefs` and `heard_beliefs`
- `conversation_memory_retention_ticks` bounds how long agents remember what was shared

This avoids a hidden hardcoded cooldown table and keeps the dampener agent-specific and explicit.

Capacity applies independently to `told_beliefs` and `heard_beliefs`. A speaker with many heard records must not silently lose all told-memory suppression just because a different memory lane filled first.

### 3. Conversation Memory Retention and Capacity Enforcement

Add explicit maintenance helpers on `AgentBeliefStore`:

```rust
pub fn record_told_belief(&mut self, memory: ToldBeliefMemory);
pub fn record_heard_belief(&mut self, memory: HeardBeliefMemory);
pub fn enforce_conversation_memory(
    &mut self,
    profile: &TellProfile,
    current_tick: Tick,
);
pub fn told_belief_memory(
    &self,
    key: &TellMemoryKey,
    current_tick: Tick,
    profile: &TellProfile,
) -> Option<&ToldBeliefMemory>;
pub fn heard_belief_memory(
    &self,
    key: &TellMemoryKey,
    current_tick: Tick,
    profile: &TellProfile,
) -> Option<&HeardBeliefMemory>;
```

Enforcement rules:

1. Expire `told_beliefs` and `heard_beliefs` older than `conversation_memory_retention_ticks`.
2. If memory still exceeds `conversation_memory_capacity`, evict oldest entries deterministically.
3. Capacity/retention apply independently to conversation memory and must not silently piggyback on generic entity-belief eviction.
4. Read surfaces used by AI/runtime code must behave as though expired entries do not exist even if a write-time maintenance pass has not yet run.
5. Eviction order must be deterministic by `(tick, counterparty, subject)` or equivalent stable ordering, never by hash order.

### 4. Tell Commit Records Participant Memory

`crates/worldwake-systems/src/tell_actions.rs` remains the authoritative mutation point.

On committed Tell:

1. Speaker memory records `ToldBeliefMemory` for `(listener, subject)` using the belief content that the speaker attempted to transmit.
2. Listener memory records `HeardBeliefMemory` for `(speaker, subject)` with one of:
   - `Accepted`
   - `Rejected`
   - `AlreadyHeldEqualOrNewer`
   - `NotInternalized`
3. These writes happen whether or not the listener ultimately replaces `known_entities[subject]`.
4. Conversation-memory writes and belief-store writes happen atomically in the same `WorldTxn`.

This is important:

- the speaker should not read the listener's actual belief store later
- the speaker only remembers that they told them
- the listener remembers hearing the claim even if they rejected it

Disposition rules:

- `Accepted`: the listener internalized the tell and `known_entities[subject]` was updated or refreshed from the tell
- `AlreadyHeldEqualOrNewer`: the listener heard the tell but retained an equal-or-newer existing belief
- `NotInternalized`: the tell reached the listener as a social interaction but failed the acceptance-fidelity gate before becoming belief content
- `Rejected`: reserved for future explicit contradiction/trust refusal paths; E15c may write it if implementation already has a concrete basis, but must not invent an omniscient rejection path merely to satisfy the enum

### 5. Replace Same-Place Tell Suppression With Explicit Resend Suppression

Replace the current shortcut in `emit_social_candidates()`:

- remove suppression based on `belief.last_known_place == Some(place)`
- do **not** infer that a listener knows a subject merely because the speaker thinks the subject is local

New resend rule:

For a candidate `ShareBelief { listener, subject }`, suppress if:

1. the actor has a current relayable belief about `subject`, and
2. actor `told_beliefs[(listener, subject)]` exists, and
3. the remembered `shared_state` is share-equivalent to the actor's current belief content for that subject

Re-emit if:

1. no tell memory exists for `(listener, subject)`, or
2. the actor's current belief about `subject` materially differs from `shared_state`, or
3. the remembered tell has expired under `conversation_memory_retention_ticks`

Material difference is defined by `SharedBeliefSnapshot` equality over shareable content, not by raw `BelievedEntityState` equality and not by a hidden hash/checksum cache.

This keeps the resend gate:

- concrete
- deterministic
- extensible to future richer belief content

### 5a. Listener-Aware Candidate Selection Before Truncation

Current tell infrastructure first chooses a globally truncated subject list by recency, then expands listeners. That is insufficient once resend suppression becomes listener-specific.

E15c therefore changes selection order:

1. enumerate live lawful listeners
2. compute relayable subjects from current beliefs
3. filter each `(listener, subject)` pair through actor-local resend suppression
4. only then apply deterministic truncation/capping

At minimum, the implementation must avoid the failure mode:

- recent subject A was already told to listener X
- older subject B has never been told to listener X
- `max_tell_candidates == 1`
- naive pre-filter truncation chooses A and drops B forever

The same listener-aware filtering rule applies to:

- AI `emit_social_candidates()`
- Tell affordance payload enumeration used by planning/runtime search

No split brain is allowed where candidate generation knows about resend suppression but authoritative affordance expansion still offers only stale duplicate subjects.

### 6. Derived Recipient-Knowledge Read Model

Add a derived, local-only read model for AI and debugging:

```rust
pub enum RecipientKnowledgeStatus {
    UnknownToSpeaker,
    SpeakerHasAlreadyToldCurrentBelief,
    SpeakerHasOnlyToldStaleBelief,
    SpeakerPreviouslyToldButMemoryExpired,
}
```

Suggested pure helper:

```rust
pub fn recipient_knowledge_status(
    current_belief: &BelievedEntityState,
    prior_tell: Option<&ToldBeliefMemory>,
) -> RecipientKnowledgeStatus;
```

Important:

- this is **not** actual listener truth
- it is the speaker's remembered interaction state
- it exists to explain and debug candidate generation decisions
- it should consume retention-aware prior tell memory, not raw stale storage

### 7. Planning Snapshot / Runtime View Support

`worldwake-sim` and `worldwake-ai` must expose actor-local conversation memory through the existing belief/runtime boundary.

Rules:

1. candidate generation only needs the actor's local `told_beliefs`
2. planning/runtime code must not query the counterparty's live `AgentBeliefStore`
3. snapshot/state plumbing should preserve deterministic ordering and storage semantics
4. runtime and planning read surfaces should expose retention-aware tell memory lookups, not a requirement that all callers manually remember to purge first

Recommended trait shape:

```rust
fn told_belief_memory(
    &self,
    actor: EntityId,
    counterparty: EntityId,
    subject: EntityId,
) -> Option<ToldBeliefMemory>;
```

and, when useful for explanation:

```rust
fn recipient_knowledge_status(
    &self,
    actor: EntityId,
    counterparty: EntityId,
    subject: EntityId,
) -> Option<RecipientKnowledgeStatus>;
```

`PlanningSnapshot` does **not** need to persist full conversation-memory maps unless a later planning stage actually consumes them. Expose the actor-local read through the belief/runtime boundary first; snapshot caching is optional and should only be added if it buys clear search/runtime value.

### 8. No Co-Location-as-Knowledge Shortcut

This spec explicitly rejects the old assumption:

"If speaker believes subject is local, listener probably already knows, so suppress Tell."

That is not lawful recipient knowledge because:

- passive observation can fail (`observation_fidelity`)
- co-location does not prove awareness
- the speaker may not know what the listener noticed

If future work wants stronger local-awareness inference, it must model that through explicit observation or conversational state, not through a hidden candidate-generation shortcut.

### 9. Migration Requirement

When this spec is implemented:

1. remove the current same-place Tell suppression rule
2. replace the current unit test that locks it in
3. add focused tests for explicit resend suppression and lawful retelling after belief change
4. update tell affordance enumeration to use the same resend policy
5. do not preserve both paths behind a flag or compatibility shim

## Component Registration

Update component/data registration for:

- `TellProfile` gains:
  - `conversation_memory_capacity: u16`
  - `conversation_memory_retention_ticks: u64`
- `AgentBeliefStore` gains:
  - `told_beliefs`
  - `heard_beliefs`

No new singleton social cache, manager entity, or AI-only runtime table is permitted.

## SystemFn Integration

### `worldwake-core`

- extend `AgentBeliefStore`
- add tell-memory structs/enums
- add deterministic retention/eviction helpers
- add `SharedBeliefSnapshot` and comparison helpers such as `to_shared_belief_snapshot()` / `share_equivalent()`
- extend `TellProfile`

### `worldwake-systems`

- update `commit_tell()` to write `ToldBeliefMemory` and `HeardBeliefMemory`
- apply conversation-memory retention/capacity after writes
- keep listener-aware resend filtering aligned with affordance payload enumeration
- keep Tell as the only authoritative mutation point for this social-memory path

### `worldwake-sim`

- extend runtime/belief query traits to expose actor-local tell memory
- preserve retention-aware tell-memory reads in the belief/runtime surface; snapshot storage is optional unless later planning stages truly need it

### `worldwake-ai`

- update `emit_social_candidates()` to query actor-local tell memory
- remove same-place suppression
- add diagnostics for resend suppression via `RecipientKnowledgeStatus`
- apply resend filtering before per-listener candidate truncation
- keep ranking logic state-mediated; no direct read of listener truth

## Cross-System Interactions (Principle 24)

The interaction path must remain state-mediated:

1. perception and prior Tell actions create beliefs
2. Tell commit records conversation memory
3. belief/runtime views expose retention-aware actor-local conversation memory
4. candidate generation and tell affordance enumeration read conversation memory plus current actor belief
5. ranking and planning react to those candidates
6. future systems may consume heard/told memory as social evidence

No system directly asks another system whether "the listener already knows this."

## FND-01 Section H

### H.1 Information-Path Analysis

| Information | Source | Path |
|-------------|--------|------|
| speaker's subject knowledge | E14/E15 belief acquisition | stored in `known_entities` |
| "I already told listener X about subject Y" | committed Tell participation | stored in `told_beliefs` on the speaker |
| "I heard speaker X tell me about subject Y" | committed Tell participation | stored in `heard_beliefs` on the listener |
| resend suppression decision | derived | compares current shareable belief content vs remembered `shared_state` through retention-aware lookup |

No remote agent knowledge is read directly. The only lawful input is the actor's own memory.

### H.2 Positive-Feedback Analysis

**Loop 1: repeated gossip spam**
- without dampening, an agent can repeatedly tell the same listener the same fact every decision cycle

**Loop 2: heard fact -> new plan -> more contact -> more telling**
- social propagation can still amplify information flow across the world

### H.3 Concrete Dampeners

**Loop 1 dampeners**
- Tell action duration and co-location requirement
- action-slot occupancy
- `conversation_memory_capacity`
- `conversation_memory_retention_ticks`
- listener-aware resend suppression
- need/danger suppression already present in social ranking
- blocked-intent memory for temporarily impossible tells

**Loop 2 dampeners**
- travel time
- Tell relay depth limits
- acceptance fidelity
- ordinary belief staleness and retention
- future contradictions can still suppress action without deleting memory

No invisible spam cooldown table or hardcoded retry clamp is permitted.

### H.4 Stored State vs Derived

**Stored**
- `AgentBeliefStore.known_entities`
- `AgentBeliefStore.told_beliefs`
- `AgentBeliefStore.heard_beliefs`
- `TellProfile`
- Tell action events and social observations

**Derived**
- `RecipientKnowledgeStatus`
- resend suppression decision
- whether a subject belief is share-equivalent to what was previously told
- social candidate omission diagnostics for resend suppression / stale-vs-current tell state

## Invariants

1. resend suppression depends only on actor-local remembered conversation state, never on listener omniscience
2. same-place co-location alone does not count as recipient knowledge
3. lawful retelling remains possible after the actor's belief materially changes
4. conversation memory is explicit world state attached to agents, not an AI-only cache
5. no backward-compatibility path preserves the old same-place suppression once the new model lands
6. expired tell memory must not continue suppressing `ShareBelief`
7. duplicate recent tells must not crowd out distinct untold subjects due to pre-filter truncation order
8. candidate generation and tell affordance enumeration must use the same resend policy

## Acceptance Criteria

1. `ShareBelief` resend suppression is explainable from `AgentBeliefStore.told_beliefs`
2. repeated telling of the same unchanged belief to the same listener is suppressed without relying on same-place subject location
3. if the speaker's belief about the subject changes, `ShareBelief` can reappear for that same listener
4. listener rejection or prior newer knowledge is representable in `heard_beliefs` without requiring the speaker to read listener truth
5. Tell participant memory obeys explicit retention/capacity policy from `TellProfile`
6. candidate-generation diagnostics can distinguish "never told" from "already told current belief"
7. expired tell memory no longer suppresses candidate generation even if no intervening tell action has occurred
8. listener-aware resend suppression is applied before candidate truncation in both AI generation and tell affordance expansion
9. same-content belief refreshes that only change bookkeeping fields such as observed tick do not force redundant retelling
10. decision traces can explain why a social tell candidate was omitted or re-enabled

## Tests

- [ ] focused unit: initial `ShareBelief` emits when no prior tell memory exists
- [ ] focused unit: unchanged repeat tell to same listener/subject is suppressed by `told_beliefs`
- [ ] focused unit: changed shareable belief content re-enables `ShareBelief`
- [ ] focused unit: same-place subject location alone does not suppress `ShareBelief`
- [ ] focused unit: same-content refresh with newer `observed_tick` does not re-enable `ShareBelief`
- [ ] focused unit: improved shareable content does re-enable `ShareBelief`
- [ ] focused unit: `commit_tell()` records speaker `told_beliefs`
- [ ] focused unit: `commit_tell()` records listener `heard_beliefs` with `Accepted`
- [ ] focused unit: listener rejection records `heard_beliefs` with `Rejected`
- [ ] focused unit: listener newer belief records `heard_beliefs` with `AlreadyHeldEqualOrNewer`
- [ ] focused unit: failed acceptance-fidelity records `heard_beliefs` with `NotInternalized`
- [ ] focused unit: conversation-memory retention and deterministic eviction obey `TellProfile`
- [ ] focused unit: expired tell memory is ignored by retention-aware read helpers before any cleanup write occurs
- [ ] focused unit: listener-aware truncation still emits an untold older subject when a newer subject is suppressed as already told
- [ ] focused unit: tell affordance payload enumeration matches AI resend suppression behavior
- [ ] focused unit: candidate diagnostics report social resend omission reason
- [ ] golden E2E: autonomous tell does not spam the same listener with the same unchanged subject over repeated ticks
- [ ] golden E2E: after a subject belief update, the speaker can lawfully retell the changed fact
- [ ] golden E2E: repeated local co-location without new tell memory does not itself suppress lawful telling
- [ ] golden E2E: after tell-memory expiry, the speaker can lawfully retell even without a belief-content change
- [ ] golden E2E: decision trace shows social candidate reappearing only after belief-content change or memory expiry

## Outcome

- Completion date: 2026-03-19
- What actually changed:
  - `worldwake-core` now stores first-class conversation memory in `AgentBeliefStore` via `told_beliefs`, `heard_beliefs`, retention-aware read helpers, deterministic eviction, `SharedBeliefSnapshot`, and recipient-knowledge derivation that ignores bookkeeping-only belief refreshes.
  - `worldwake-sim` and `worldwake-ai` now expose actor-local conversation-memory reads consistently across live runtime views, planning snapshots, and planning state.
  - `worldwake-systems` Tell affordance expansion now applies listener-aware resend filtering before truncation, and Tell commit now records speaker/listener participant memory with concrete heard dispositions.
  - `worldwake-ai` social candidate generation now uses conversation memory instead of the old same-place shortcut, and decision traces now report social omission reasons through `RecipientKnowledgeStatus`.
  - Golden coverage now proves unchanged-repeat suppression, lawful re-tell after belief-content change, lawful re-tell after conversation-memory expiry, and trace visibility for re-enabled social candidates.
- Deviations from original plan:
  - the final core record shape keeps `(counterparty, subject)` in `TellMemoryKey` rather than duplicating identity fields inside `ToldBeliefMemory` and `HeardBeliefMemory`.
  - the spec's placeholder `Rejected` disposition remains reserved; implementation landed `Accepted`, `AlreadyHeldEqualOrNewer`, and `NotInternalized` without inventing a trust/contradiction path.
  - the old raw `relayable_social_subjects()` helper was preserved and the resend-aware policy was added as a separate reusable layer so authoritative tell enumeration and AI candidate generation could converge on one explicit policy without silently changing unrelated callers.
- Verification results:
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-systems`
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-ai --test golden_social`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
