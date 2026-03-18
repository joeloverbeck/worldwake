**Status**: PENDING

# E15c: Conversation Memory & Recipient Knowledge for Social Telling

## Epic Summary

Replace the current speaker-side same-place Tell suppression shortcut with explicit per-agent conversation memory that records what an agent has told or heard, and use that memory to suppress redundant social chatter.

This spec does **not** add omniscient second-order belief or a hidden anti-spam cache. It adds concrete local memory artifacts to `AgentBeliefStore` so a speaker can reason from:

- what they currently believe about a subject
- what they remember having already told a specific listener about that subject
- whether the current belief has materially changed since that tell

The result is a cleaner architecture than the current `emit_social_candidates()` heuristic in `crates/worldwake-ai/src/candidate_generation.rs`, which suppresses telling when the speaker merely believes the subject is already at the current place. That heuristic is only a proxy for recipient knowledge and is not grounded in explicit social memory.

## Phase

Phase 3: Information & Politics

## Crates

`worldwake-core`
- extend `AgentBeliefStore` with explicit conversation memory
- add social-memory query and retention types
- extend `TellProfile` with social-memory policy

`worldwake-sim`
- extend belief/runtime query traits and planning snapshot inputs for actor-local conversation memory

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
| `BelievedEntityState` | E14 belief system | reused as the remembered content of a tell |
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
    pub shared_state: BelievedEntityState,
    pub told_tick: Tick,
}
```

```rust
pub struct HeardBeliefMemory {
    pub counterparty: EntityId,
    pub subject: EntityId,
    pub heard_state: BelievedEntityState,
    pub heard_tick: Tick,
    pub disposition: HeardBeliefDisposition,
}
```

```rust
pub enum HeardBeliefDisposition {
    Accepted,
    Rejected,
    AlreadyHeldEqualOrNewer,
}
```

Rules:

1. `told_beliefs` stores what **this agent remembers telling** a specific counterparty about a specific subject.
2. `heard_beliefs` stores what **this agent remembers hearing** from a specific counterparty about a specific subject.
3. These are local memory artifacts, not shared truth.
4. Reuse `BelievedEntityState` directly rather than inventing a partial shadow schema. That keeps the remembered content concrete and extensible as E14 belief state evolves.

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
```

Enforcement rules:

1. Expire `told_beliefs` and `heard_beliefs` older than `conversation_memory_retention_ticks`.
2. If memory still exceeds `conversation_memory_capacity`, evict oldest entries deterministically.
3. Capacity/retention apply independently to conversation memory and must not silently piggyback on generic entity-belief eviction.

### 4. Tell Commit Records Participant Memory

`crates/worldwake-systems/src/tell_actions.rs` remains the authoritative mutation point.

On committed Tell:

1. Speaker memory records `ToldBeliefMemory` for `(listener, subject)` using the belief content that the speaker attempted to transmit.
2. Listener memory records `HeardBeliefMemory` for `(speaker, subject)` with one of:
   - `Accepted`
   - `Rejected`
   - `AlreadyHeldEqualOrNewer`
3. These writes happen whether or not the listener ultimately replaces `known_entities[subject]`.
4. Conversation-memory writes and belief-store writes happen atomically in the same `WorldTxn`.

This is important:

- the speaker should not read the listener's actual belief store later
- the speaker only remembers that they told them
- the listener remembers hearing the claim even if they rejected it

### 5. Replace Same-Place Tell Suppression With Explicit Resend Suppression

Replace the current shortcut in `emit_social_candidates()`:

- remove suppression based on `belief.last_known_place == Some(place)`
- do **not** infer that a listener knows a subject merely because the speaker thinks the subject is local

New resend rule:

For a candidate `ShareBelief { listener, subject }`, suppress if:

1. the actor has a current relayable belief about `subject`, and
2. actor `told_beliefs[(listener, subject)]` exists, and
3. the remembered `shared_state` is equal to the actor's current belief state for that subject

Re-emit if:

1. no tell memory exists for `(listener, subject)`, or
2. the actor's current belief about `subject` materially differs from `shared_state`

Material difference is defined by ordinary `BelievedEntityState` equality, not by a special social dedupe checksum.

This keeps the resend gate:

- concrete
- deterministic
- extensible to future richer belief content

### 6. Derived Recipient-Knowledge Read Model

Add a derived, local-only read model for AI and debugging:

```rust
pub enum RecipientKnowledgeStatus {
    UnknownToSpeaker,
    SpeakerHasAlreadyToldCurrentBelief,
    SpeakerHasOnlyToldStaleBelief,
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

### 7. Planning Snapshot / Runtime View Support

`worldwake-sim` and `worldwake-ai` must expose actor-local conversation memory through the existing belief/runtime boundary.

At minimum:

```rust
fn told_belief_memory(
    &self,
    actor: EntityId,
    counterparty: EntityId,
    subject: EntityId,
) -> Option<ToldBeliefMemory>;
```

Rules:

1. candidate generation only needs the actor's local `told_beliefs`
2. planning/runtime code must not query the counterparty's live `AgentBeliefStore`
3. snapshot/state plumbing should preserve deterministic ordering and storage semantics

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
4. do not preserve both paths behind a flag or compatibility shim

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
- extend `TellProfile`

### `worldwake-systems`

- update `commit_tell()` to write `ToldBeliefMemory` and `HeardBeliefMemory`
- apply conversation-memory retention/capacity after writes
- keep Tell as the only authoritative mutation point for this social-memory path

### `worldwake-sim`

- extend runtime/belief query traits to expose actor-local tell memory
- preserve tell-memory data in planning snapshot / read-model surface

### `worldwake-ai`

- update `emit_social_candidates()` to query actor-local tell memory
- remove same-place suppression
- add diagnostics for resend suppression via `RecipientKnowledgeStatus`
- keep ranking logic state-mediated; no direct read of listener truth

## Cross-System Interactions (Principle 24)

The interaction path must remain state-mediated:

1. perception and prior Tell actions create beliefs
2. Tell commit records conversation memory
3. candidate generation reads conversation memory plus current actor belief
4. ranking and planning react to those candidates
5. future systems may consume heard/told memory as social evidence

No system directly asks another system whether "the listener already knows this."

## FND-01 Section H

### H.1 Information-Path Analysis

| Information | Source | Path |
|-------------|--------|------|
| speaker's subject knowledge | E14/E15 belief acquisition | stored in `known_entities` |
| "I already told listener X about subject Y" | committed Tell participation | stored in `told_beliefs` on the speaker |
| "I heard speaker X tell me about subject Y" | committed Tell participation | stored in `heard_beliefs` on the listener |
| resend suppression decision | derived | compares current `BelievedEntityState` vs remembered `shared_state` |

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
- whether a subject belief is materially newer/different than what was previously told

## Invariants

1. resend suppression depends only on actor-local remembered conversation state, never on listener omniscience
2. same-place co-location alone does not count as recipient knowledge
3. lawful retelling remains possible after the actor's belief materially changes
4. conversation memory is explicit world state attached to agents, not an AI-only cache
5. no backward-compatibility path preserves the old same-place suppression once the new model lands

## Acceptance Criteria

1. `ShareBelief` resend suppression is explainable from `AgentBeliefStore.told_beliefs`
2. repeated telling of the same unchanged belief to the same listener is suppressed without relying on same-place subject location
3. if the speaker's belief about the subject changes, `ShareBelief` can reappear for that same listener
4. listener rejection or prior newer knowledge is representable in `heard_beliefs` without requiring the speaker to read listener truth
5. Tell participant memory obeys explicit retention/capacity policy from `TellProfile`
6. candidate-generation diagnostics can distinguish "never told" from "already told current belief"

## Tests

- [ ] focused unit: initial `ShareBelief` emits when no prior tell memory exists
- [ ] focused unit: unchanged repeat tell to same listener/subject is suppressed by `told_beliefs`
- [ ] focused unit: changed `BelievedEntityState` re-enables `ShareBelief`
- [ ] focused unit: same-place subject location alone does not suppress `ShareBelief`
- [ ] focused unit: `commit_tell()` records speaker `told_beliefs`
- [ ] focused unit: `commit_tell()` records listener `heard_beliefs` with `Accepted`
- [ ] focused unit: listener rejection records `heard_beliefs` with `Rejected`
- [ ] focused unit: listener newer belief records `heard_beliefs` with `AlreadyHeldEqualOrNewer`
- [ ] focused unit: conversation-memory retention and deterministic eviction obey `TellProfile`
- [ ] golden E2E: autonomous tell does not spam the same listener with the same unchanged subject over repeated ticks
- [ ] golden E2E: after a subject belief update, the speaker can lawfully retell the changed fact

