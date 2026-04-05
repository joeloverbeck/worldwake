# S52: Evidence Artifacts and Aftermath Materialization

## Summary

Materialize action aftermath into inspectable world state that agents can discover and reason about. Currently actions emit rich event log entries but don't leave persistent evidence in the world. This spec adds scene evidence components (tamper state, disturbance markers, movement traces) that agents can perceive, investigate, and use as the basis for accusations or belief updates.

## Phase

Phase 6: Architectural Substrates II

## Status

COMPLETED

## Crates

- `worldwake-core` (evidence types, components)
- `worldwake-sim` (action aftermath emission)
- `worldwake-systems` (perception of evidence, evidence decay)
- `worldwake-ai` (investigation affordances, evidence-driven candidate generation)

## Dependencies

- E17 (crime/justice) — completed (`archive/specs/E17-crime-theft-justice.md`)
- S27 (expectation-violation goals) — completed (`archive/specs/S27-expectation-violation-goals.md`)
- S45 (social artifacts) — completed (`archive/specs/S45-unified-social-artifact-model.md`)

## Design Goals

- Actions that modify containers, transfer items, inflict wounds, or change location leave inspectable physical evidence as world state
- Evidence is perceivable by co-located agents through the existing perception system
- Evidence decays over time (physical processes, not magic expiry)
- Evidence feeds into the justice system as proof for accusations
- Investigation affordances let agents actively examine locations for evidence

## Non-Goals

- Forensic analysis system (matching evidence to suspects through deduction) — deferred
- Evidence forging or planting — deferred
- Chain of custody for evidence items — deferred

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P3 (Concrete State) | Evidence is stored state, not derived from event log queries |
| P4 (Persistent Identity) | Evidence has stable identity and lifecycle |
| P5 (Carriers of Consequence) | Evidence propagates downstream effects (accusations, belief changes) |
| P7 (Locality) | Evidence must be at the scene; agents discover it locally |
| P10 (Outcomes Are Granular and Leave Aftermath) | Actions leave granular physical residue, not just event log entries |
| P14 (World State ≠ Belief State) | Evidence is authoritative world state; agent knowledge of evidence is belief state. Perception determines what agents know, not tracking flags on the evidence itself. |
| P17 (Violated Expectation) | Missing items detected through container evidence combined with belief-vs-reality comparison, not omniscience |
| P18 (Evidence Is World State) | Directly satisfies this principle |

## Deliverables

### 1. New Types

```rust
/// Unique identifier for an evidence entry within a SceneEvidence component.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct EvidenceEntryId(pub u64);

pub enum EvidenceKind {
    /// Container was tampered with (e.g., during theft).
    ContainerTampered {
        container: EntityId,
        tampered_at: Tick,
    },
    /// Blood trail from a wounded entity.
    BloodTrail {
        from_place: EntityId,
        severity: Permille,
        caused_by: Option<EntityId>,  // If witnessed during combat
    },
    /// Generic disturbance at a place.
    DisturbanceMarker {
        place: EntityId,
        kind: DisturbanceKind,
        created_at: Tick,
    },
    /// Movement trace left at a departure place.
    MovementTrace {
        entity: EntityId,
        departed_from: EntityId,
        direction: EntityId,  // Next place on route
        observed_at: Tick,
    },
}

pub enum DisturbanceKind {
    CombatAftermath,       // Weapons, broken items, blood
    ForcedEntry,           // Container tamper evidence
    AbandonedGoods,        // Items on ground from interrupted action
    WildernessRelief,      // Existing EventTag, now materialized
}
```

### 2. New Component

```rust
pub struct SceneEvidence {
    pub evidence: Vec<EvidenceEntry>,
    pub next_entry_id: u64,
}

pub struct EvidenceEntry {
    pub id: EvidenceEntryId,
    pub kind: EvidenceKind,
    pub created_at: Tick,
    pub decay_ticks: u32,          // Evidence fades after this many ticks
}
```

Note: No `discovered_by` tracking on evidence entries. Evidence perception is purely co-location-based per Principle 7. Whether an agent has previously seen evidence is tracked in the agent's belief store (`BelievedEvidenceState`), not on the evidence itself. This preserves the P14 separation — authoritative evidence state does not track agent knowledge.

### 3. Evidence Emission

Action handlers modified to emit `SceneEvidence` on commit:
- **Theft** (`steal` action in `crates/worldwake-systems/src/transport_actions.rs:589`): `ContainerTampered { container, tampered_at }` on the place containing the stolen item's container. The specific items missing are NOT stored on the evidence — agents discover what's missing through belief-vs-reality comparison (P17).
- **Combat** (`attack` action in `crates/worldwake-systems/src/combat.rs:1739`): `BloodTrail` at combat location if wounds inflicted. `DisturbanceMarker::CombatAftermath` if death occurs.
- **Travel** (`travel` action commit): `MovementTrace { entity, departed_from, direction, observed_at }` at the departure place. Agents arriving later can see that someone recently left and which direction they went.
- **Forced pickup** (ground items): `DisturbanceMarker::ForcedEntry` if item taken from a container not owned by the actor.
- **Wilderness relief** (needs system): `DisturbanceMarker::WildernessRelief` at the relief location, materializing the existing `EventTag::WildernessRelief`.

**Authoritative-to-AI Impact Rule note**: These changes add post-commit world state (evidence emission) but do NOT modify action preconditions, affordance generation, candidate generation, or planning operators. Existing affordances, candidates, and plan shapes are unaffected. No payload revalidation changes. The only downstream AI impact is that future perception of evidence may feed investigation candidates — this is new information entering the belief layer, not a change to the decision pipeline.

### 4. Evidence Decay System

New `SystemId::EvidenceDecay` (runs after Perception, before the next tick's action systems):
- Each tick, check all `SceneEvidence` components.
- Remove entries where `current_tick - created_at >= decay_ticks`.
- If all entries removed, remove the `SceneEvidence` component from the place.

Default decay rates (per `EvidenceKind`, as concrete world-process constants):

| Kind | Decay Ticks | Rationale |
|------|-------------|-----------|
| `ContainerTampered` | 200 | Physical marks on container persist longer |
| `BloodTrail` | 100 | Blood dries and fades |
| `DisturbanceMarker` | 50 | Scene disturbance settles |
| `MovementTrace` | 30 | Tracks fade quickly |

### 5. Perception Integration

- `SceneEvidence` components on Place entities are observable through existing passive perception (the perception system already iterates all co-located entities via `world.entities_effectively_at(place)`)
- New `BelievedEvidenceState` added to `BelievedEntityState` for perceived places:

```rust
pub struct BelievedEvidenceState {
    pub entries: Vec<BelievedEvidenceEntry>,
    pub observed_tick: Tick,
}

pub struct BelievedEvidenceEntry {
    pub kind: EvidenceKind,
    pub freshness: Tick,  // When the evidence was believed to have been created
}
```

- Agents who perceive evidence gain belief entries about what happened at the scene. Standard confidence and staleness policies apply.

### 6. Investigation Integration

- Existing `InvestigateViolation` goal (`goal.rs:88-91`) and investigation action handler (`crates/worldwake-systems/src/investigate_actions.rs`) extended to check `SceneEvidence` at the violation place.
- Investigation action reads `SceneEvidence` and converts entries into structured beliefs with higher confidence than passive perception.
- Evidence discovered through investigation strengthens accusation confidence in the justice system.

## Cross-System Interactions (Principle 26)

- **Crime system** emits `ContainerTampered` evidence on theft → perception system makes it observable → victim or guard perceives it → violation detection triggers investigation
- **Combat system** emits `BloodTrail`/`CombatAftermath` → perception → danger awareness
- **Travel system** emits `MovementTrace` → perception → tracking awareness
- **Justice system** reads agent beliefs about evidence to support accusation strength
- **Needs system** is unaffected (evidence is passive state)

All interaction through state. No cross-system direct calls.

## Profile-Driven Parameters

Evidence decay rates are per-`EvidenceKind`, not per-agent. No new agent profiles needed. Decay rates are concrete world-process constants (P2 compliant — they represent physical processes like blood drying and tracks fading, not drama levers).

## Component Registration

- `SceneEvidence` on `EntityKind::Place`

## Section H — Causal Hooks

### H.1 Information path
Evidence created at action location → perceived by co-located agents through standard perception → belief update (BelievedEvidenceState) → investigation goal if violation detected → accusation. Every step is local (P7). No global evidence registry.

### H.2 Positive feedback
Evidence → investigation → accusation → punishment. No amplifying loop — each step consumes the motivation for the next. A resolved crime produces no further evidence. Punishment resolves the investigation motive.

### H.3 Dampeners
| Loop | Dampener |
|------|----------|
| Evidence → investigation spiral | Evidence decays over time. Investigation requires travel + action duration. Only agents with ViolationDispositionProfile investigate. |
| Combat evidence → fear → avoidance | Evidence decays. Agent diversity in courage/risk tolerance. Fresh observation contradicts stale evidence. |

### H.4 Stored vs derived
| Item | Classification |
|------|---------------|
| `SceneEvidence` on Place entity | **Stored authoritative state** |
| `EvidenceEntry.decay_ticks` | **Stored** — per-entry countdown |
| `BelievedEvidenceState` in agent belief store | **Derived belief** — perceived snapshot, may be stale |
| "What's missing from container" | **Derived** — agent compares believed inventory vs observed inventory (P17) |

### H.5 Contention
Multiple agents can perceive the same evidence simultaneously — this is not contention (evidence is read-only world state). Investigation actions may contend for access to the place (via standard contention substrate if applicable), but evidence perception itself is non-exclusive.

### H.6 Partial failures
| Failure | Aftermath |
|---------|-----------|
| Action aborts before commit | No evidence created — evidence is emitted only on commit |
| Evidence decays before anyone perceives it | Evidence silently removed. Agents who arrive late find no evidence — correct behavior (P10, aftermath is not permanent) |
| Investigation interrupted | Agent retains partial beliefs from perception. Can re-investigate later. |

### H.7 Belief staleness
Agents can hold stale evidence beliefs after evidence decays. An agent who saw blood at the market yesterday returns today and the blood is gone — their belief is updated on the next perception pass. If the agent never returns, the stale belief remains until memory retention expires. This is correct behavior (P14, P16).

### H.8 Temporal resolution
Evidence is created at action commit time (same tick as the action effect). Evidence decay runs after Perception each tick, so evidence created at tick T is first perceivable at tick T (same tick, since Perception runs after action commits) and begins decaying at tick T+1.

### H.9-H.12 (N/A)
- H.9: No derived views or optimizations.
- H.10: (covered in H.7 — belief staleness and correction through re-perception)
- H.11: Standard tick resolution. Evidence system runs after Perception — no simultaneity concerns.
- H.12: No boundary/off-map interfaces.

### H.13 Invariants and regression
- Evidence is only created on action commit — never during planning or precondition checks
- Evidence decays monotonically — `decay_ticks` never increases
- Removing decayed evidence does not affect agent beliefs — stale beliefs expire through normal retention
- `SceneEvidence` component removed from place when all entries decay — no empty component accumulation
- Conservation: evidence creation/decay does not affect item quantities or agent state

### H.14 Save/load
`SceneEvidence` components persist through save/load as standard ECS components. `EvidenceEntryId` counter persists. Evidence decay continues correctly after load because decay is tick-based (`current_tick - created_at >= decay_ticks`), not wall-clock-based.

## Verification

### Golden test: Theft evidence discovery

**Setup**: One place with container. One thief. One guard with PerceptionProfile and ViolationDispositionProfile.

**Execution**: Thief steals from container. Guard arrives at place. Ticks until guard perceives evidence and starts investigation.

**Assertions**:
- `ContainerTampered` evidence created on place after theft commit (authoritative world state)
- Guard perceives evidence (belief store — `believed_evidence` populated)
- Guard generates `InvestigateViolation` candidate (decision trace)
- Evidence decays after `decay_ticks` (authoritative world state — evidence removed)

## Outcome

- Completed: 2026-04-05
- Landed the S52 ticket chain:
  - `S52EVIDAFT-001` added the core evidence substrate (`EvidenceEntryId`, `DisturbanceKind`, `EvidenceKind`, `EvidenceEntry`, `SceneEvidence`) and registered `SceneEvidence` on places.
  - `S52EVIDAFT-002` added authoritative aftermath emission for contained/displayed theft, fatal combat, travel departure, and wilderness relief.
  - `S52EVIDAFT-003` added canonical evidence decay via `SystemId::EvidenceDecay`.
  - `S52EVIDAFT-004` added `BelievedEvidenceState`, explicit current-place evidence perception, and investigation-time place-belief refresh.
  - `S52EVIDAFT-005` added Scenario 114 in `crates/worldwake-ai/tests/golden_integration.rs`, proving theft evidence emission, local perception, mismatch-driven investigation selection, decay of theft residue, and deterministic replay.
- Generated golden docs were refreshed and now record Scenario 114 in the coverage matrix, scenario map, and inventory.
- Deviation from original plan: the spec's broader prose about “evidence-driven candidate generation,” accusation-strength integration, generic same-place place iteration, and direct downstream uses for `BloodTrail`/`MovementTrace` did not all land in S52. The implemented boundary is narrower and honest: evidence is stored, perceived, refreshed through investigation, and proven in the theft-evidence golden, while `InvestigateViolation` generation remains mismatch-driven in the live AI.
- Verification:
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-systems`
  - `cargo test -p worldwake-ai golden_s52_theft_evidence_discovery -- --nocapture`
  - `cargo test -p worldwake-ai --test golden_integration`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
