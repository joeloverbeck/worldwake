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

- E14 (perception & belief system) — completed (`archive/specs/E14-perception-beliefs.md`)
- E15 (rumor, witness & discovery) — completed (`archive/specs/E15-rumor-witness-discovery.md`)
- S28 (knowledge-path traces) — completed (`archive/specs/S28-knowledge-path-traces.md`)

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
| P27 (Derived Summaries Are Caches, Never Truth) | Core motivation — `BelievedEntityState` becomes a derived cache over concrete claims. Claims are the source of truth; the summary is always re-derivable. |
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

pub struct ClaimId(pub u64);  // Per-agent sequential (u64 for consistency with ViolationId, EvidenceEntryId)

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
    Evidence,           // Scene evidence at a place
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
    EvidenceState(Option<BelievedEvidenceState>),
}
```

The `EntityBeliefAspect` enum covers all 13 fields of `BelievedEntityState` (`belief.rs:738-755`): `last_known_place` → Location, `last_known_inventory` → Inventory, `alive` → Alive, `wounds` → Wounded, `believed_activity` → Activity, `workstation_tag` → WorkstationPresent, `resource_source` → ResourceAvailable, `believed_contention` → ContentionState, `believed_artifact` → ArtifactState, `last_known_courage` → Courage, `believed_evidence` → Evidence. The remaining fields (`observed_tick`, `source`) are per-summary metadata derived from the winning claim, not independent aspects.

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

For each `EntityBeliefAspect`, pick the claim with highest confidence (recomputed with staleness via `policy.staleness_penalty_per_tick`). The winning claim's value populates the corresponding `BelievedEntityState` field. The winning claim's source and tick become the summary's `source` and `observed_tick`.

### Perception Integration

Perception system modified to emit claims instead of directly writing `BelievedEntityState`:
- `observe_passive_local_entities` (`perception.rs:205`) → emits `EntityBeliefClaim` per observed aspect
- `process_witness_event` → emits claims from event state deltas
- Tell acceptance → emits claims from speaker's claims (source chain incremented)

After all claims are emitted, `derive_entity_summary` rebuilds `known_entities`.

### Memory Enforcement

`enforce_capacity` (`belief.rs:108`) now operates on claims:
- Evict claims older than `memory_retention_ticks`
- If claims per entity exceed capacity, evict lowest-confidence
- After eviction, re-derive `known_entities`
- Modeled after existing `enforce_institutional_capacity` (`belief.rs:489`) which already manages claim-like structures

## Cross-System Interactions (Principle 26)

- **Perception** writes claims → derives summaries
- **AI planner** reads `known_entities` (unchanged interface)
- **Tell system** shares claims (source chain incremented)
- **Decision traces** can now report "believed X because claim C from source S"
- **Investigation** can query claims to build evidence chains

All interaction through state. No cross-system direct calls.

## Profile-Driven Parameters

No new profiles. Existing `PerceptionProfile` (`memory_capacity`, `retention_ticks`, `confidence_policy`) governs claim lifecycle.

## Component Registration

No new components. `AgentBeliefStore` is extended with new fields.

## Section H — Causal Hooks

### H.1 Information path
Claims carry explicit source chain. Each claim traces to a perception event (DirectObservation), tell event (Report/Rumor with chain_len), or inference. The path: world state → perception → claim emission → confidence computation → summary derivation → planner access. Every step is local (P7).

### H.2 Positive feedback
More claims → more memory pressure → eviction → less knowledge. Self-dampening through memory capacity limits. No amplifying loops.

### H.3 Dampeners
| Loop | Dampener |
|------|----------|
| Claim accumulation | Memory capacity limits total claims per agent. Staleness penalty degrades old claims. Eviction removes lowest-confidence claims. |
| Tell propagation of claims | Chain length penalty reduces confidence with each relay. Agents with low rumor_base discount long chains. |

### H.4 Stored vs derived
| Item | Classification |
|------|---------------|
| `entity_claims` in AgentBeliefStore | **Stored authoritative state** (per-agent) |
| `known_entities` in AgentBeliefStore | **Derived cache** — rebuilt from claims via derive_entity_summary |
| `next_claim_id` | **Stored** — per-agent monotonic counter |
| `confidence` on each claim | **Stored at emission, recomputed with staleness for resolution** |

### H.5 Contention
Multiple claims about the same entity-aspect are not contention — they are the design goal (P16). Resolution picks the highest-confidence claim. No exclusive access or queuing needed.

### H.6 Partial failures
| Failure | Aftermath |
|---------|-----------|
| Claim emission interrupted (perception system abort) | No partial claims written — claims are emitted atomically per perception pass |
| All claims for an entity evicted | Entity removed from `known_entities` — agent has no belief about that entity (correct: ignorance is valid state per P16) |
| Confidence computation produces tie | Deterministic tie-break by `acquired_tick` (newer wins) then `claim_id` |

### H.7 Belief staleness and correction
Claims naturally age through staleness penalty. When an agent re-perceives an entity, a new DirectObservation claim is emitted with fresh confidence, which wins over stale Report/Rumor claims during summary derivation. Correction is automatic through the confidence policy — no explicit contradiction detection needed.

### H.8-H.12 (N/A)
- H.8 (temporal resolution): Claims emitted during perception pass (after action commits, before next AI tick). Summary derived immediately after claim emission. No ambiguity about when claims are visible to the planner.
- H.9-H.10: No derived views beyond known_entities. Correction covered in H.7.
- H.11: Standard tick resolution. No simultaneity concerns for claim emission.
- H.12: No boundary/off-map interfaces.

### H.13 Invariants and regression
- `known_entities` is always derivable from `entity_claims` — deleting and re-deriving must produce identical result
- Claim eviction + re-derivation never produces a `known_entities` entry with no backing claim
- `next_claim_id` is monotonically increasing per agent — never reused
- Existing golden tests must pass unchanged (planner reads `known_entities` which is derived from claims carrying the same information)

### H.14 Save/load
`entity_claims` and `next_claim_id` persist through save/load in the current format. `known_entities` can be re-derived from claims at load time (or persisted as a cache for convenience — either approach is valid since the derivation is deterministic). SAVE_FORMAT_VERSION bump required when the persisted shape changes, but older save versions are not migrated.

## Verification

### Migration test: known_entities equivalence

Verify that `derive_entity_summary` over freshly-emitted claims produces identical `BelievedEntityState` values to the current direct-write path. This ensures the migration is behavior-preserving.

### Golden test: contradictory claims coexist

Two agents tell a third agent contradictory facts about the same entity. The third agent holds both claims, and `known_entities` reflects the highest-confidence claim. When the third agent later perceives the entity directly, the DirectObservation claim wins over both heard claims.
