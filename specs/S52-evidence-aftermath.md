# S52: Evidence Artifacts and Aftermath Materialization

## Summary

Materialize action aftermath into inspectable world state that agents can discover and reason about. Currently actions emit rich event log entries but don't leave persistent evidence in the world. This spec adds scene evidence components (tamper state, disturbance markers, movement traces) that agents can perceive, investigate, and use as the basis for accusations or belief updates.

## Phase

Phase 6: Architectural Substrates II

## Status

Draft

## Crates

- `worldwake-core` (evidence types, components)
- `worldwake-sim` (action aftermath emission)
- `worldwake-systems` (perception of evidence, evidence decay)
- `worldwake-ai` (investigation affordances, evidence-driven candidate generation)

## Dependencies

- E17 (crime/justice) — completed
- S27 (expectation-violation goals) — completed
- S45 (social artifacts) — completed

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
| P10 (Aftermath) | Actions leave granular physical residue, not just event log entries |
| P17 (Violated Expectation) | Missing items detected through container evidence, not omniscience |
| P18 (Evidence Is World State) | Directly satisfies this principle |

## Deliverables

### New Types

```rust
pub enum EvidenceKind {
    ContainerTampered {
        container: EntityId,
        tampered_at: Tick,
        items_missing: Vec<(CommodityKind, Quantity)>,
    },
    BloodTrail {
        from_place: EntityId,
        severity: Permille,
        caused_by: Option<EntityId>,  // If witnessed
    },
    DisturbanceMarker {
        place: EntityId,
        kind: DisturbanceKind,
        created_at: Tick,
    },
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

### New Component

```rust
pub struct SceneEvidence {
    pub evidence: Vec<EvidenceEntry>,
}

pub struct EvidenceEntry {
    pub id: EvidenceEntryId,
    pub kind: EvidenceKind,
    pub created_at: Tick,
    pub decay_ticks: u32,          // Evidence fades after this many ticks
    pub discovered_by: BTreeSet<EntityId>,  // Who has already seen it
}
```

### Evidence Emission

Action handlers modified to emit `SceneEvidence` on commit:
- **Theft** (`steal` action): `ContainerTampered` on target container
- **Combat** (`attack` action): `BloodTrail` at combat location if wounds inflicted
- **Forced pickup** (ground items): `DisturbanceMarker::ForcedEntry` if not owned
- **Death**: `DisturbanceMarker::CombatAftermath` at death location

### Evidence Decay System

New `SystemId::EvidenceDecay` (runs after Perception):
- Each tick, decrement remaining decay time on all evidence entries
- Remove entries where `current_tick - created_at >= decay_ticks`
- Decay rate is physical (weather, traffic would eventually affect it — for now, fixed per kind)

### Perception Integration

- `SceneEvidence` components are observable through existing passive perception
- `BelievedEvidenceState` added to `BelievedEntityState` for places
- Agents who perceive evidence gain belief entries about what happened

### Investigation Affordance

- Existing `InvestigateViolation` goal now checks for `SceneEvidence` at the violation place
- Investigation action reads `SceneEvidence` and converts entries into structured beliefs
- Evidence discovered through investigation has higher confidence than passive perception

## Cross-System Interactions

- **Crime system** emits `ContainerTampered` evidence on theft → perception system makes it observable → victim or guard perceives it → violation detection triggers investigation
- **Combat system** emits `BloodTrail`/`CombatAftermath` → perception → danger awareness
- **Justice system** reads `discovered_by` to determine if evidence supports accusation
- **Needs system** is unaffected (evidence is passive state)

## Profile-Driven Parameters

Evidence decay rates are per-`EvidenceKind`, not per-agent. No new agent profiles needed.

Default decay rates:
- `ContainerTampered`: 200 ticks
- `BloodTrail`: 100 ticks
- `DisturbanceMarker`: 50 ticks
- `MovementTrace`: 30 ticks

## Component Registration

- `SceneEvidence` on `EntityKind::Place`

## Section H — Causal Hooks

1. **Information path**: Evidence created at action location → perceived by co-located agents → belief update → investigation goal → accusation. Every step is local.
2. **Positive feedback**: Evidence → investigation → accusation → punishment. No amplifying loop — each step consumes the motivation for the next.
3. **Dampeners**: Evidence decays over time. Investigation requires travel and action time.
4. **Stored vs derived**: `SceneEvidence` is stored authoritative state. `BelievedEvidenceState` is per-agent belief (derived from perception).
