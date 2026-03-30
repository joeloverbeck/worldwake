# S43: Communication Type Differentiation

## Summary

Replace the single undifferentiated Tell action with typed communication classes — Alarm, Testimony, Gossip — each with distinct urgency, trust model, and suppression rules. Today all social communication routes through one `Tell` action that shares beliefs uniformly: same priority, same suppression under stress, same source degradation. This means a fleeing witness who shouts "dragon on the road!" is treated identically to idle chatter about commodity prices. FOUNDATIONS III.15 and III.18 demand that testimony, documents, records, and traces be distinct causal carriers with their own trust and urgency models.

The fix introduces a `CommunicationClass` enum on Tell payloads, per-class `CommunicationProfile` parameters on agents, and class-aware suppression in the goal policy. The existing Tell action infrastructure remains — this spec classifies the content, not the mechanism.

## Source

Derived from the ChatGPT architecture review (`brainstorming/improvements-to-ai-architecture.md`, Issue #2 and Improvement F) validated against the actual codebase. Confirmed: single `register_tell_action()` handles all social transmission. `GoalFamilyPolicy` suppresses `ShareBelief` uniformly under stress via `SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::Medium)`. No urgency differentiation exists.

## Phase

Phase 5: Architectural Substrates

## Crates

- `worldwake-core` (new `CommunicationClass` enum, `CommunicationProfile` component, Tell payload extension)
- `worldwake-ai` (class-aware social candidate generation, class-aware suppression, class-aware ranking)
- `worldwake-systems` (class-aware Tell handler: acceptance fidelity varies by class)

## Dependencies

- None. Can be scheduled in parallel with S42. Builds on existing Tell infrastructure (E15, E15b, E15c all completed).

## FOUNDATIONS Alignment

- **Principle 15, Knowledge Is Acquired Locally and Travels Physically**: "Witness testimony, posted notices, letters, ledgers, rumors, tracks, blood trails, empty shelves, missing items, and public speeches are not flavor. They are mechanisms of causal propagation." Different carriers demand different treatment — a panicked alarm is not a casual rumor.
- **Principle 18, Memory, Evidence, and Records Are World State**: Social transmissions carry different weight and urgency. A formal accusation should not be suppressed under the same stress threshold as idle gossip.
- **Principle 22, Agent Diversity Through Concrete Variation**: Per-agent `CommunicationProfile` means some agents are better alarm-responders, others are better gossip filters.
- **Principle 5, Simulate Carriers of Consequence**: Differentiated communication types create more downstream consequences — an alarm that gets through under stress creates different chains than gossip that gets suppressed.

## Design Goals

1. **Classify content, not mechanism**: The Tell action remains a single action type. The classification lives on the payload and affects urgency, suppression, and acceptance — not the action lifecycle.
2. **Three classes, not twelve**: Start with Alarm, Testimony, Gossip. These cover the meaningful behavioral distinctions. More can be added later (RecordConsultation already exists as a separate action; FormalAccusation is already the Accuse action).
3. **Class is derived from content, not chosen by the agent**: The classification is deterministic based on what is being communicated — agents don't decide "I'll gossip about this alarm." The mapping is authoritative.
4. **Per-class suppression**: Alarm is never suppressed by stress. Testimony is suppressed only under critical stress. Gossip is suppressed under medium stress (current behavior). This makes emergency information flow resilient.
5. **Per-class acceptance**: Listeners accept alarms more readily than gossip, governed by class-specific fidelity values on the listener's `CommunicationProfile`.

## Deliverables

### 1. `CommunicationClass` enum (`worldwake-core`)

```rust
/// Classification of social communication by urgency and trust model.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum CommunicationClass {
    /// Urgent safety information: nearby threats, active combat, fires,
    /// observed deaths. High urgency, high acceptance, never suppressed.
    Alarm,
    /// Direct observation reports and actionable intelligence: entity
    /// locations, resource availability, witnessed crimes, institutional
    /// facts. Medium urgency, moderate acceptance, suppressed only
    /// under critical stress.
    Testimony,
    /// Relayed second-hand information, stale beliefs, casual
    /// observations. Low urgency, baseline acceptance, suppressed
    /// under medium stress (current behavior).
    Gossip,
}
```

### 2. Classification rules

The communication class is derived deterministically from the Tell payload content:

| Content | Class | Rationale |
|---------|-------|-----------|
| Entity believed dead (alive status changed) | Alarm | Immediate safety relevance |
| Active combat observed (`WitnessedConflict`) | Alarm | Immediate safety relevance |
| `SuspectedTheft` with high-severity theft | Alarm | Immediate security relevance |
| Direct observation entity beliefs (`PerceptionSource::DirectObservation`) | Testimony | First-hand evidence |
| Institutional claims from `DirectObservation` or `WitnessedEvent` | Testimony | First-hand institutional knowledge |
| `WitnessedAbsence` (expected entity missing) | Testimony | Direct observation of anomaly |
| Report-sourced entity beliefs (`PerceptionSource::Report`) | Testimony | Second-hand but attributed evidence |
| Institutional claims from `Report` source | Testimony | Attributed institutional knowledge |
| Rumor-sourced entity beliefs (`PerceptionSource::Rumor`) | Gossip | Unattributed hearsay |
| Inference-sourced beliefs | Gossip | Speculation |
| `CoPresence` observations | Gossip | Casual social observation |
| `WitnessedCooperation`, `WitnessedObligation`, `WitnessedTelling` | Gossip | Social observation |
| Institutional claims from `RecordConsultation` or `SelfDeclaration` | Testimony | Attributed knowledge |

Implement as a pure function:

```rust
pub fn classify_communication(payload: &TellPayloadItem) -> CommunicationClass
```

This function lives in `worldwake-core` (or `worldwake-systems` near the Tell handler) and is the single authoritative classification point.

### 3. `CommunicationProfile` component (`worldwake-core`)

```rust
/// Per-agent parameters controlling communication behavior by class.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommunicationProfile {
    /// Acceptance fidelity for Alarm-class communications.
    /// Higher = more likely to accept alarms from others.
    pub alarm_acceptance: Permille,
    /// Acceptance fidelity for Testimony-class communications.
    pub testimony_acceptance: Permille,
    /// Acceptance fidelity for Gossip-class communications.
    pub gossip_acceptance: Permille,
    /// Whether this agent prioritizes sharing alarms even under stress.
    /// If true, this agent will attempt to share alarm-class information
    /// even when social goals would otherwise be suppressed.
    pub alarm_sharer: bool,
}

impl Component for CommunicationProfile {}
```

Default values:

| Field | Default | Rationale |
|-------|---------|-----------|
| `alarm_acceptance` | 950‰ | Near-certain acceptance of alarms |
| `testimony_acceptance` | 800‰ | Matches current `TellProfile.acceptance_fidelity` |
| `gossip_acceptance` | 600‰ | Lower trust for unattributed hearsay |
| `alarm_sharer` | true | Most agents share urgent news by default |

Register on `EntityKind::Agent` in component schema.

### 4. Class-aware suppression in goal policy (`worldwake-ai`)

Modify `GoalFamilyPolicy` for `ShareBelief` to return class-dependent suppression:

- **Alarm-class ShareBelief**: `SuppressionRule::Never` — agents share alarms even under survival stress.
- **Testimony-class ShareBelief**: `SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::Critical)` — suppressed only under critical danger.
- **Gossip-class ShareBelief**: `SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::Medium)` — current behavior preserved.

This requires the suppression evaluation to know the communication class. The `GoalKind::ShareBelief` already carries `listener` and `topic` — the class can be derived from the topic's underlying belief content at candidate-generation time and stored on the `GroundedGoal` for policy evaluation.

**Implementation approach**: Add an optional `CommunicationClass` field to the ShareBelief candidate's grounded goal metadata. `emit_social_candidates()` computes the class for each payload item and attaches it. The policy reads it. If absent (defensive fallback), treat as Gossip.

### 5. Class-aware acceptance in Tell handler (`worldwake-systems`)

Modify the Tell commit handler to use class-specific acceptance fidelity:

1. For each payload item, compute `classify_communication(item)`.
2. Look up the listener's `CommunicationProfile` (fall back to defaults if absent).
3. Use the class-appropriate acceptance fidelity for the RNG acceptance check.

This replaces the current uniform `TellProfile.acceptance_fidelity` check for Tell payload items. The `TellProfile.acceptance_fidelity` field becomes the legacy fallback and may be deprecated in a follow-up.

### 6. Class-aware ranking boost (`worldwake-ai`)

Alarm-class social goals should receive a ranking boost so they tend to be chosen over gossip-class goals when multiple ShareBelief candidates compete:

- Alarm: motive multiplier ×3 (or equivalent fixed bonus)
- Testimony: motive multiplier ×1 (no change)
- Gossip: motive multiplier ×1 (no change)

The multiplier is applied in `emit_social_candidates()` when computing the motive value. This means under equal `social_weight`, an alarm-class Tell outranks a gossip-class Tell.

### 7. Golden tests

**Scenario A: Stress-filtered communication**
- Agent with critical hunger has both an alarm (witnessed death) and gossip (commodity price) to share.
- The alarm-class ShareBelief is NOT suppressed; the gossip-class one IS.
- Prove the agent tells the alarm but not the gossip.

**Scenario B: Class-aware acceptance**
- Two listeners: one with default `CommunicationProfile`, one with `gossip_acceptance: Permille(100)`.
- Speaker tells gossip-class information to both.
- The skeptical listener rejects most gossip; the default listener accepts normally.

**Scenario C: Alarm propagation under stress**
- Three agents in a line of places. Agent A witnesses a death. Agent A is under survival stress. Agent A still tells Agent B (alarm not suppressed). Agent B relays to Agent C (now Testimony-class, since B's source is Report).
- Prove the alarm reaches C through lawful relay.

All scenarios with deterministic replay companions.

## Section H — FOUNDATIONS Analysis

### H.1 Information-path analysis

Communication class is derived locally from payload content. No new information paths are introduced — the existing Tell/perception/belief paths carry the classified content. The classification function reads only the Tell payload item's content type and source — no external state needed.

Alarm propagation uses the same Tell → belief update → re-Tell chain as current gossip. The difference is suppression and acceptance, not the propagation mechanism.

### H.2 Positive-feedback analysis

**Potential loop**: Alarm → agent reacts → creates new alarming situation → new alarm. Example: "dragon sighted" alarm → agents flee → stampede causes injuries → "injuries" alarm.

This is an intended emergent cascade, not a bug. The dampener is physical:
- Alarms require co-location to transmit (spatial limit)
- Each transmission takes 2 ticks (temporal limit)
- Relay chains degrade from Alarm → Testimony → Gossip (source degradation)
- Agents can only Tell one listener at a time (attention occupancy)

**No new runaway risk** beyond what the Tell system already has. The alarm class makes transmission *more likely under stress* but does not increase transmission *speed* or *range*.

### H.3 Concrete dampeners

| Loop | Dampener |
|------|----------|
| Alarm cascade (alarm → reaction → new alarm) | Co-location requirement, 2-tick Tell duration, source degradation on relay, one listener at a time |
| Over-sharing alarms (agent spams alarms) | Conversation memory (E15c) — told_beliefs tracking prevents repeat, per-listener capacity limit |
| Alarm acceptance saturation | `alarm_acceptance` permille provides tunable ceiling; not 1000‰ by default |

### H.4 Stored state vs. derived read-model list

| Item | Classification |
|------|---------------|
| `CommunicationProfile` component | **Stored authoritative state** — per-agent, persists in save/load |
| `CommunicationClass` on a Tell payload item | **Derived** — computed by `classify_communication()` at Tell time, not stored |
| Class attached to GroundedGoal metadata | **Derived** — computed at candidate generation, ephemeral within the decision pass |

## Cross-System Interactions (Principle 12)

- **Candidate generation** (worldwake-ai) reads Tell payload items → derives class → attaches to grounded goal.
- **Goal policy** (worldwake-ai) reads class from grounded goal → applies class-specific suppression.
- **Ranking** (worldwake-ai) reads class → applies motive multiplier.
- **Tell handler** (worldwake-systems) reads payload item → derives class → reads listener's `CommunicationProfile` → applies class-specific acceptance.

No system writes to another system's state. All interaction is through shared authoritative state reads.

## Migration Path

1. Add `CommunicationClass` enum and `classify_communication()` to `worldwake-core`.
2. Add `CommunicationProfile` component, register on `EntityKind::Agent`.
3. Extend `emit_social_candidates()` to compute and attach class.
4. Modify suppression evaluation to be class-aware for ShareBelief.
5. Modify Tell handler acceptance to use class-specific fidelity.
6. Add ranking boost for alarm-class.
7. Write golden tests.
8. Bump `SAVE_FORMAT_VERSION` if serialized format changes.

## Verification

- `cargo test --workspace` passes — gossip-class ShareBelief retains current suppression behavior.
- Golden test A proves alarm survives stress suppression.
- Golden test B proves class-aware acceptance differentiation.
- Golden test C proves alarm relay through stressed intermediary.
- Save/load round-trip preserves `CommunicationProfile`.
- `cargo clippy --workspace` clean.
