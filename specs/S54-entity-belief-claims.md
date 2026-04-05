# S54: Entity Belief Claims

## Summary

Introduce a claim-based substrate beneath `BelievedEntityState` for entity-level beliefs. Currently `known_entities` stores one summary per entity with a single source and observation tick. This spec adds `EntityBeliefClaim` entries that record individual propositions with provenance, so `BelievedEntityState` becomes a derived working-memory cache rather than the root representation. This enables contradictory reports to coexist, provenance queries ("why do I believe this?"), and uneven correction propagation.

## Phase

Phase 6: Architectural Substrates II

## Status

Draft

## Crates

- `worldwake-core` (claim types, belief store extension)
- `worldwake-systems` (perception emits claims, working-memory derivation)
- `worldwake-ai` (planner reads derived summaries as before)

## Dependencies

- E14 (perception & belief system) — completed
- E15 (rumor, witness & discovery) — completed
- S28 (knowledge-path traces) — completed

## Design Goals

- `BelievedEntityState` becomes a derived cache over claims, not the source of truth for entity beliefs
- Multiple claims about the same entity can coexist (e.g., "A told me X is at Market" vs "I saw X at Farm")
- Each claim carries full provenance (source, chain, tick, confidence)
- The planner continues to read `BelievedEntityState` (working memory) — no planner changes
- Claim resolution (which claim wins for the summary) uses existing confidence policy
- Memory capacity enforcement evicts claims, which updates the derived summary

## Non-Goals

- Claim status lifecycle (disputed/retracted/superseded) — deferred
- Propositional belief engine beyond entity-level facts — deferred
- Explicit contradiction detection and resolution logic — deferred (implicit via confidence comparison)
- Changes to institutional beliefs (already claim-based via `Vec<BelievedInstitutionalClaim>`)

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P3 (Concrete State) | Claims are concrete state, not abstract confidence scores |
| P15 (Knowledge Travels Physically) | Each claim carries source, chain length, acquisition tick |
| P16 (Ignorance and Contradiction) | Multiple conflicting claims can coexist without being a bug |
| P17 (Violated Expectation) | Claims enable "I believed X because A told me" → "but I now see Y" |
| P27 (Provenance) | Claims carry full provenance chain — who, when, how |
| P29 (Debuggability) | Claims make belief formation inspectable: why does agent believe X? |

## Deliverables

### New Types

```rust
pub struct EntityBeliefClaim {
    pub claim_id: ClaimId,
    pub subject: EntityId,
    pub aspect: EntityBeliefAspect,
    pub value: ClaimValue,
    pub source: PerceptionSource,
    pub acquired_tick: Tick,
    pub claimed_event_tick: Option<Tick>,  // When the source says it happened
    pub confidence: Permille,              // Computed from source + staleness
}

pub struct ClaimId(pub u32);  // Per-agent sequential

pub enum EntityBeliefAspect {
    Location,           // Where the entity is
    Inventory(CommodityKind),  // How much of a commodity they have
    Alive,              // Whether alive
    Wounded,            // Wound state
    Activity,           // What they're doing
    WorkstationPresent, // Workstation at place
    ResourceAvailable(CommodityKind),  // Resource source state
    ContentionState,    // Queue/grant status
    ArtifactState,      // Social artifact status
    Courage,            // Perceived courage level
}

pub enum ClaimValue {
    Place(Option<EntityId>),
    Quantity(Quantity),
    Bool(bool),
    Activity(Option<BelievedActivity>),
    WorkstationTag(Option<WorkstationTag>),
    ResourceSource(Option<ResourceSource>),
    ContentionState(Option<BelievedContentionState>),
    ArtifactState(Option<BelievedArtifactState>),
    Courage(Option<Permille>),
    WoundSnapshot(Vec<Wound>),
}
```

### AgentBeliefStore Extension

```rust
pub struct AgentBeliefStore {
    // NEW: claim-based substrate
    pub entity_claims: BTreeMap<EntityId, Vec<EntityBeliefClaim>>,
    pub next_claim_id: ClaimId,
    
    // DERIVED: working-memory cache (rebuilt from claims)
    pub known_entities: BTreeMap<EntityId, BelievedEntityState>,
    
    // UNCHANGED
    pub social_observations: Vec<SocialObservation>,
    pub told_beliefs: BTreeMap<TellMemoryKey, ToldBeliefMemory>,
    pub heard_beliefs: BTreeMap<TellMemoryKey, HeardBeliefMemory>,
    pub asked_witnesses: BTreeMap<AskWitnessMemoryKey, AskWitnessMemory>,
    pub institutional_beliefs: BTreeMap<InstitutionalBeliefKey, Vec<BelievedInstitutionalClaim>>,
}
```

### Working-Memory Derivation

```rust
fn derive_entity_summary(
    claims: &[EntityBeliefClaim],
    current_tick: Tick,
    policy: &BeliefConfidencePolicy,
) -> BelievedEntityState
```

For each `EntityBeliefAspect`, pick the claim with highest confidence (recomputed with staleness). The winning claim's value populates the corresponding `BelievedEntityState` field. The winning claim's source and tick become the summary's `source` and `observed_tick`.

### Perception Integration

Perception system modified to emit claims instead of directly writing `BelievedEntityState`:
- `observe_passive_local_entities` → emits `EntityBeliefClaim` per observed aspect
- `process_witness_event` → emits claims from event state deltas
- Tell acceptance → emits claims from speaker's claims

After all claims are emitted, `derive_entity_summary` rebuilds `known_entities`.

### Memory Enforcement

`enforce_capacity` now operates on claims:
- Evict claims older than `memory_retention_ticks`
- If claims per entity exceed capacity, evict lowest-confidence
- After eviction, re-derive `known_entities`

## Cross-System Interactions

- **Perception** writes claims → derives summaries
- **AI planner** reads `known_entities` (unchanged interface)
- **Tell system** shares claims (source chain incremented)
- **Decision traces** can now report "believed X because claim C from source S"
- **Investigation** can query claims to build evidence chains

## Profile-Driven Parameters

No new profiles. Existing `PerceptionProfile` (memory_capacity, retention_ticks, confidence_policy) governs claim lifecycle.

## Component Registration

No new components. `AgentBeliefStore` is extended with new fields.

## Section H — Causal Hooks

1. **Information path**: Claims carry explicit source chain. Each claim traces to a perception event, tell event, or inference.
2. **Positive feedback**: More claims → more memory pressure → eviction → less knowledge. Self-dampening.
3. **Dampeners**: Memory capacity limits total claims. Staleness penalty degrades old claims.
4. **Stored vs derived**: `entity_claims` is stored. `known_entities` is derived from claims via confidence resolution.
