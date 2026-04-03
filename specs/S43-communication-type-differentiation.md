# S43: Communication Type Differentiation

## Summary

Replace the single undifferentiated Tell action with typed communication classes — Alarm, Testimony, Gossip — each with distinct urgency, trust model, and suppression rules. Today all social communication routes through one `Tell` action that shares beliefs uniformly: same priority, same suppression under stress, same source degradation. This means a fleeing witness who shouts "dragon on the road!" is treated identically to idle chatter about commodity prices. FOUNDATIONS III.15 and III.18 demand that testimony, documents, records, and traces be distinct causal carriers with their own trust and urgency models.

The fix introduces a `CommunicationClass` enum on Tell payloads, per-class `CommunicationProfile` parameters on agents, and class-aware suppression in the goal policy. The existing Tell action infrastructure remains — this spec classifies the content, not the mechanism.

## Source

Derived from the ChatGPT architecture review (`brainstorming/improvements-to-ai-architecture.md`, Issue #2 and Improvement F) validated against the actual codebase. Confirmed: single `register_tell_action()` handles all social transmission. `GoalFamilyPolicy` suppresses `ShareBelief` uniformly under stress via `SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High)`. No urgency differentiation exists.

## Phase

Phase 5: Architectural Substrates

## Crates

- `worldwake-core` (new `CommunicationClass` enum, `CommunicationProfile` component, `GoalKind::ShareBelief` extension, classification function)
- `worldwake-ai` (class-aware social candidate generation, class-aware suppression, class-aware ranking)
- `worldwake-systems` (class-aware Tell handler: acceptance fidelity varies by class)

## Dependencies

- S42 (completed, archived at `archive/specs/S42-per-agent-reasoning-style.md`).
- Builds on existing Tell infrastructure (E15, E15b, E15c — all completed, archived at `archive/specs/`).

## FOUNDATIONS Alignment

- **Principle 15, Knowledge Is Acquired Locally and Travels Physically**: "Witness testimony, posted notices, letters, ledgers, rumors, tracks, blood trails, empty shelves, missing items, and public speeches are not flavor. They are mechanisms of causal propagation." Different carriers demand different treatment — a panicked alarm is not a casual rumor.
- **Principle 18, Memory, Evidence, and Records Are World State**: Social transmissions carry different weight and urgency. A formal accusation should not be suppressed under the same stress threshold as idle gossip.
- **Principle 22, Agent Diversity Through Concrete Variation**: Per-agent `CommunicationProfile` means some agents are better gossip filters, others accept alarms more readily.
- **Principle 5, Simulate Carriers of Consequence**: Differentiated communication types create more downstream consequences — an alarm that gets through under stress creates different chains than gossip that gets suppressed.
- **Principle 28, No Backward Compatibility**: `TellProfile.acceptance_fidelity` is fully replaced by `CommunicationProfile` class-specific acceptance fields. No legacy fallback.

## Design Goals

1. **Classify content, not mechanism**: The Tell action remains a single action type. The classification lives on the payload and affects urgency, suppression, and acceptance — not the action lifecycle.
2. **Three classes, not twelve**: Start with Alarm, Testimony, Gossip. These cover the meaningful behavioral distinctions. More can be added later (RecordConsultation already exists as a separate action; FormalAccusation is already the Accuse action).
3. **Class is derived from content, not chosen by the agent**: The classification is deterministic based on what is being communicated — agents don't decide "I'll gossip about this alarm." The mapping is authoritative.
4. **Per-class suppression**: Alarm is never suppressed by stress. Testimony is suppressed only under critical stress. Gossip is suppressed under high stress (preserving current behavior). This makes emergency information flow resilient.
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
    /// under high stress (current behavior).
    Gossip,
}
```

### 2. Classification rules

The communication class is derived deterministically from the `TellTopic` and its inner content. The function requires the speaker's belief store to inspect `EntityBelief` topics.

**By TellTopic variant:**

| TellTopic variant | Inner content | Class | Rationale |
|-------------------|---------------|-------|-----------|
| `SocialObservation` | `WitnessedConflict` | Alarm | Immediate safety relevance |
| `SocialObservation` | `SuspectedTheft` | Testimony | Security observation, not immediate danger |
| `SocialObservation` | `WitnessedAbsence` | Testimony | Direct observation of anomaly |
| `SocialObservation` | `CoPresence` | Gossip | Casual social observation |
| `SocialObservation` | `WitnessedCooperation` | Gossip | Social observation |
| `SocialObservation` | `WitnessedObligation` | Gossip | Social observation |
| `SocialObservation` | `WitnessedTelling` | Gossip | Social observation |
| `EntityBelief` | Subject believed dead (alive=false) | Alarm | Immediate safety relevance |
| `EntityBelief` | Source is `DirectObservation` | Testimony | First-hand evidence |
| `EntityBelief` | Source is `Report` | Testimony | Second-hand but attributed evidence |
| `EntityBelief` | Source is `Rumor` | Gossip | Unattributed hearsay |
| `EntityBelief` | Source is `Inference` | Gossip | Speculation |
| `InstitutionalClaim` | Source is `DirectObservation` or `WitnessedEvent` | Testimony | First-hand institutional knowledge |
| `InstitutionalClaim` | Source is `RecordConsultation` or `SelfDeclaration` | Testimony | Attributed institutional knowledge |
| `InstitutionalClaim` | Source is `Report` | Testimony | Attributed institutional knowledge |
| `InstitutionalClaim` | Source is `Rumor` | Gossip | Unattributed institutional hearsay |

Implement as a function in `worldwake-core`:

```rust
/// Classify a Tell topic by urgency and trust model.
/// Requires the speaker's belief store to inspect EntityBelief topics.
pub fn classify_communication(
    topic: &TellTopic,
    speaker_beliefs: &AgentBeliefStore,
) -> CommunicationClass
```

This is the single authoritative classification point, called from both `worldwake-ai` (candidate generation) and `worldwake-systems` (Tell handler).

### 3. `CommunicationProfile` component (`worldwake-core`)

```rust
/// Per-agent parameters controlling communication acceptance by class.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommunicationProfile {
    /// Acceptance fidelity for Alarm-class communications.
    /// Higher = more likely to accept alarms from others.
    pub alarm_acceptance: Permille,
    /// Acceptance fidelity for Testimony-class communications.
    pub testimony_acceptance: Permille,
    /// Acceptance fidelity for Gossip-class communications.
    pub gossip_acceptance: Permille,
}

impl Component for CommunicationProfile {}
```

Default values:

| Field | Default | Rationale |
|-------|---------|-----------|
| `alarm_acceptance` | 950‰ | Near-certain acceptance of alarms |
| `testimony_acceptance` | 800‰ | Matches current `TellProfile.acceptance_fidelity` default |
| `gossip_acceptance` | 600‰ | Lower trust for unattributed hearsay |

Register on `EntityKind::Agent` in component schema.

### 4. Extend `GoalKind::ShareBelief` with communication class (`worldwake-core`)

Add a `communication_class` field to the `ShareBelief` variant:

```rust
GoalKind::ShareBelief {
    listener: EntityId,
    topic: TellTopic,
    communication_class: CommunicationClass,
}
```

The class is computed at candidate generation time (in `emit_social_candidates()`) when the speaker's full belief context is available. It travels with the goal through ranking, policy evaluation, and plan synthesis without requiring re-derivation.

This is preferred over adding a field to `GroundedGoal` (which would pollute a shared struct with an `Option` meaningful to only one goal kind) and over deriving at policy time (the policy function only sees `GoalKind` and lacks belief context for `EntityBelief` classification).

### 5. Class-aware suppression in goal policy (`worldwake-ai`)

Modify `GoalFamilyPolicy` for `ShareBelief` to return class-dependent suppression by reading the `communication_class` field on the `GoalKind` variant:

- **Alarm-class ShareBelief**: `SuppressionRule::Never` — agents share alarms even under survival stress.
- **Testimony-class ShareBelief**: `SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::Critical)` — suppressed only under critical danger.
- **Gossip-class ShareBelief**: `SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::High)` — preserves current behavior.

The `GoalFamilyPolicy` match arm for `ShareBelief` changes from a group match to an individual match that inspects `communication_class`.

### 6. Class-aware acceptance in Tell handler (`worldwake-systems`)

Modify the Tell commit handler to use class-specific acceptance fidelity:

1. For the Tell payload's topic, compute `classify_communication(topic, speaker_beliefs)`.
2. Look up the listener's `CommunicationProfile` (fall back to defaults if absent).
3. Use the class-appropriate acceptance fidelity (`alarm_acceptance`, `testimony_acceptance`, or `gossip_acceptance`) for the RNG acceptance check.

This fully replaces the current `TellProfile.acceptance_fidelity` check. Remove the `acceptance_fidelity` field from `TellProfile` and update all test setups that reference it. Per Principle 28, no legacy fallback.

### 7. Class-aware ranking boost (`worldwake-ai`)

Alarm-class social goals receive a ranking boost so they tend to be chosen over gossip-class goals when multiple ShareBelief candidates compete:

- Alarm: result of `social_pressure_for_topic()` is multiplied by 3 (saturating at `Permille(1000)`) before `score_product` with `social_weight`
- Testimony: no change (×1)
- Gossip: no change (×1)

The multiplier is applied in the `ShareBelief` arm of `compute_raw_motive()` in `ranking.rs`, reading the `communication_class` field from the `GoalKind`.

### 8. Golden tests

**Scenario A: Stress-filtered communication**
- Agent with critical hunger has both an alarm (witnessed conflict as `SocialObservation`) and gossip (entity belief from Rumor source) to share.
- The alarm-class ShareBelief is NOT suppressed; the gossip-class one IS.
- Prove the agent tells the alarm but not the gossip.

**Scenario B: Class-aware acceptance**
- Two listeners: one with default `CommunicationProfile`, one with `gossip_acceptance: Permille(100)`.
- Speaker tells gossip-class information to both.
- The skeptical listener rejects most gossip; the default listener accepts normally.

**Scenario C: Alarm propagation under stress**
- Three agents in a line of places. Agent A witnesses a conflict (WitnessedConflict). Agent A is under survival stress. Agent A still tells Agent B (alarm not suppressed). Agent B relays to Agent C (now Testimony-class, since B's source is `Report`).
- Prove the alarm reaches C through lawful relay.

All scenarios with deterministic replay companions.

## Section H — FOUNDATIONS Analysis

### H.1 Information-path analysis

Communication class is derived locally from payload content and speaker belief state. No new information paths are introduced — the existing Tell/perception/belief paths carry the classified content. The classification function reads only the Tell topic's content type, source metadata, and (for `EntityBelief`) the speaker's belief about the subject — no external state needed.

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
| `CommunicationClass` on a `TellTopic` | **Derived** — computed by `classify_communication()` at Tell time, not stored |
| `communication_class` on `GoalKind::ShareBelief` | **Derived** — computed at candidate generation, ephemeral within the decision pass, not persisted |

## Cross-System Interactions (Principle 12)

- **Candidate generation** (worldwake-ai) reads speaker beliefs and `TellTopic` → calls `classify_communication()` → stores class on `GoalKind::ShareBelief` variant.
- **Goal policy** (worldwake-ai) reads `communication_class` from `GoalKind::ShareBelief` → applies class-specific suppression.
- **Ranking** (worldwake-ai) reads `communication_class` → applies alarm motive multiplier.
- **Tell handler** (worldwake-systems) reads `TellTopic` and speaker beliefs → calls `classify_communication()` → reads listener's `CommunicationProfile` → applies class-specific acceptance.

No system writes to another system's state. All interaction is through shared authoritative state reads.

## Migration Path

1. Add `CommunicationClass` enum and `classify_communication()` to `worldwake-core`.
2. Add `CommunicationProfile` component, register on `EntityKind::Agent`.
3. Extend `GoalKind::ShareBelief` with `communication_class` field. Update all match arms and constructors.
4. Update `emit_social_candidates()` to compute and attach class at candidate generation.
5. Modify suppression evaluation to be class-aware for ShareBelief.
6. Remove `acceptance_fidelity` from `TellProfile`. Modify Tell handler acceptance to use class-specific fidelity from `CommunicationProfile`.
7. Add ranking boost for alarm-class in `compute_raw_motive()`.
8. Write golden tests.
9. Bump `SAVE_FORMAT_VERSION` (serialized `GoalKind` changes, `TellProfile` field removed, new `CommunicationProfile` component).

## Verification

- `cargo test --workspace` passes — gossip-class ShareBelief retains current suppression behavior (`High`).
- Golden test A proves alarm survives stress suppression.
- Golden test B proves class-aware acceptance differentiation.
- Golden test C proves alarm relay through stressed intermediary.
- Save/load round-trip preserves `CommunicationProfile`.
- `cargo clippy --workspace` clean.
