# S65: Social Aftermath Memory

## Summary

Add provenance-tracked social relationships (grudges, debts, kin-protection, gratitude) that accumulate from concrete events and influence future behavior. Currently `RelationTables` tracks `hostile_to`/`loyal_to` as flat `Permille` weights with no memory of cause. This spec adds relationship edges with event provenance, enabling revenge chains, selective cooperation, kin protection, and grudge-driven refusal — making agents feel human rather than merely rational.

## Phase

Phase 7: Consequence Carriers

## Status

Draft

## Crates

- `worldwake-core` (relationship types, grudge/obligation components)
- `worldwake-systems` (social aftermath actions)
- `worldwake-ai` (social goal generation, relationship-modulated candidate scoring)

## Dependencies

- S59 (expectation-obligation substrate) — provides the commitment tracking that creates social debts
- S63 (contested evidence/warrants) — wrongful accusation creates grudges; protection motives emerge
- E17 (crime/justice) — completed (theft/punishment creates grudge basis)
- E12 (combat) — completed (combat creates grudge basis)

## Design Goals

- Every social relationship edge tracks the concrete event that created or strengthened it
- Grudges decay but never fully disappear — they persist as aftermath (P10)
- Social bonds modulate existing goal priorities rather than creating a separate behavior system
- Kin and protector relations are concrete links, not implicit family trees
- Revenge, protection, and favoritism emerge from goal generation modulated by social memory

## Non-Goals

- Family tree or genealogy system — kin relations are declared social bonds, not biological modeling
- Universal "friendship score" or "social mood" — forbidden (P3)
- Emotion system — agents have social memory, not simulated feelings
- Forgiveness mechanics — grudge decay happens through time and compensating events, not a forgiveness action
- Faction-level grudges — this spec covers individual social aftermath; faction relations are separate

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P3 (Concrete State) | Every relationship edge has a source event, not an abstract score |
| P4 (Persistent Identity) | Relationship edges have stable identity and lifecycle |
| P5 (Carriers of Consequence) | Grudges drive revenge, debts drive repayment, kin bonds drive protection — all downstream consequences |
| P10 (Aftermath) | Social aftermath persists: a grudge from a theft affects behavior long after the theft is resolved |
| P18 (Records Are World State) | Grudge memory, obligation memory, and kin relations are inspectable world state |
| P22 (Agent Diversity) | Different agents hold grudges differently (grudge_persistence varies), protect kin at different costs |
| P22A (Learning) | Social memory is explicitly acquired, scoped, and decayable — meets the "accountable origin" standard |

## Deliverables

### 1. Social Memory Component

```rust
/// Per-agent store of social relationships with provenance.
/// Registered on EntityKind::Agent. Universal with defaults.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SocialMemory {
    /// Provenance-tracked relationship edges.
    pub edges: Vec<SocialEdge>,
    /// Maximum edges before oldest/weakest are evicted.
    pub capacity: u16,
}

impl Default for SocialMemory {
    fn default() -> Self {
        Self {
            edges: Vec::new(),
            capacity: 30,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SocialEdge {
    pub id: SocialEdgeId,
    /// The other entity in this relationship.
    pub target: EntityId,
    /// What kind of social link this is.
    pub kind: SocialEdgeKind,
    /// The concrete event that created this edge.
    pub provenance_event: EventId,
    /// Current strength of the edge (decays over time).
    pub strength: Permille,
    /// When this edge was created.
    pub created_tick: Tick,
    /// When this edge was last reinforced (by a new related event).
    pub last_reinforced_tick: Tick,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SocialEdgeId(pub u64);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SocialEdgeKind {
    /// Grudge from harm: theft, assault, wrongful accusation, property damage.
    Grudge { cause: GrudgeCause },
    /// Gratitude from aid: healing, rescue, lending, protection.
    Gratitude { cause: GratitudeCause },
    /// Kin or declared protector bond.
    KinBond { relation: KinRelation },
    /// Obligation: owes something to this entity (links to DebtRecord from S64).
    Obligation { debt_id: Option<DebtId> },
    /// Patron/protector relationship (non-kin).
    Patronage,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GrudgeCause {
    Theft,
    Assault,
    Murder,
    WrongfulAccusation,
    Betrayal,
    PropertyDamage,
    FailedObligation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum GratitudeCause {
    Healing,
    Rescue,
    Lending,
    Protection,
    Testimony,
    Aid,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum KinRelation {
    /// Declared family bond (spouse, sibling, parent, child).
    Family,
    /// Sworn bond (oath-brother, blood oath).
    Sworn,
    /// Mentor/apprentice.
    Mentor,
}
```

### 2. Social Aftermath Profile

```rust
/// Per-agent parameters governing social memory and response.
/// Registered on EntityKind::Agent. Universal with defaults.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SocialAftermathProfile {
    /// How slowly grudges decay (0 = forgets instantly, 1000 = never forgets).
    pub grudge_persistence: Permille,
    /// How strongly kin bonds influence behavior (0 = no kin loyalty, 1000 = absolute).
    pub kin_loyalty: Permille,
    /// Threshold at which a grudge triggers revenge-seeking behavior.
    pub revenge_threshold: Permille,
    /// Willingness to shelter fugitives who are kin or under patronage.
    pub shelter_willingness: Permille,
    /// How much gratitude edges boost cooperation willingness.
    pub gratitude_weight: Permille,
}

impl Default for SocialAftermathProfile {
    fn default() -> Self {
        Self {
            grudge_persistence: Permille::new(500),
            kin_loyalty: Permille::new(700),
            revenge_threshold: Permille::new(600),
            shelter_willingness: Permille::new(400),
            gratitude_weight: Permille::new(500),
        }
    }
}
```

### 3. Social Edge Creation Triggers

Social edges are created automatically when certain events are perceived by the affected agent:

| Event | Edge Created | On Whom |
|-------|-------------|---------|
| Agent observes theft of their property | `Grudge { Theft }` against thief | Victim |
| Agent wounded in combat by attacker | `Grudge { Assault }` against attacker | Victim |
| Agent's kin killed | `Grudge { Murder }` against killer | Kin of victim |
| Agent wrongfully accused (S63 exoneration) | `Grudge { WrongfulAccusation }` against accuser | Exonerated |
| Agent healed by healer | `Gratitude { Healing }` toward healer | Patient |
| Agent rescued (S59 search/rescue) | `Gratitude { Rescue }` toward rescuer | Rescued |
| Agent receives loan (S64 borrow) | `Gratitude { Lending }` + `Obligation` toward creditor | Borrower |
| Agent protected in combat | `Gratitude { Protection }` toward protector | Protected |
| Kin bond declared in scenario | `KinBond { Family/Sworn/Mentor }` | Both parties |

Edge creation happens in the perception/belief-update phase: when an agent perceives an event that affects them or their kin, a `SocialEdge` is added to their `SocialMemory`.

### 4. Social Edge Decay

During the world maintenance phase, social edges decay:

```
new_strength = strength - (1000 - grudge_persistence) / decay_rate_divisor
```

Where `decay_rate_divisor` is a per-edge-kind constant (grudges decay slower than gratitude). Edges below a minimum threshold are candidates for eviction when capacity is reached.

Reinforcement: if a new event of the same kind against the same target occurs, the edge's `strength` is restored and `last_reinforced_tick` updated.

### 5. Behavioral Modulation

Social memory modulates existing goal generation and candidate scoring rather than creating new behavior systems:

#### Revenge
When an agent has a `Grudge` edge with `strength > revenge_threshold`:
- `generate_candidates` emits `GoalKind::SeekRevenge { target }` — plans to find and harm the grudge target
- Revenge is not guaranteed — it competes with other goals (hunger, safety, duty)
- Revenge satisfaction creates a `Gratitude { sense of justice }` edge... just kidding. Successful revenge reduces grudge strength but creates new grudge on the target.

#### Kin Protection
When an agent perceives a kin entity in danger (combat, detention, pursuit):
- `generate_candidates` emits `GoalKind::ProtectKin { kin, threat }` with priority scaled by `kin_loyalty`
- Protection may mean: joining combat, providing alibi (S63), sheltering a fugitive, providing aid

#### Cooperation Modulation
Social edges modulate candidate scoring for existing goals:
- Agent asked to lend (S64 `borrow`) → creditor checks `SocialEdge::Grudge` against borrower → refuse if grudge is strong
- Agent asked about a person (S59 `ask_about_person`) → witness checks social edges → may refuse to help if grudge exists against the searcher or if kin bond exists with the subject
- Agent participating in rationing → office-holder may unconsciously prioritize agents with `Gratitude` edges (favoritism)
- Agent deciding whether to testify → checks social edges to decide cooperation

### 6. New Actions

#### `seek_revenge`
- **Preconditions**: Agent has `Grudge` edge with `strength > revenge_threshold`. Target is reachable. Agent has combat capability or other means of retaliation.
- **Duration**: Travel + confrontation.
- **Effect**: Agent travels to target and initiates combat, theft, accusation, or other retaliation. The form of revenge depends on available actions and agent profile.
- **Domain**: `ActionDomain::Combat` or `ActionDomain::Social` (depends on method)

#### `protect_kin`
- **Preconditions**: Agent has `KinBond` edge with a threatened entity. Agent perceives or believes kin is in danger.
- **Duration**: Travel + intervention.
- **Effect**: Agent moves to kin's location and acts to reduce threat: joins combat as ally, provides resources, provides alibi, or evacuates kin.
- **Domain**: `ActionDomain::Care`

#### `shelter_fugitive`
- **Preconditions**: Agent has `KinBond` or `Patronage` edge with a fugitive entity. Fugitive is co-located or can reach the agent. Agent's `shelter_willingness > threshold`.
- **Duration**: Ongoing (harboring).
- **Effect**: Agent provides a location for the fugitive to hide. The fugitive is at the agent's place but concealed from casual perception (uses `PlaceConcealment` from S56). Creates risk for the sheltering agent (harboring is a crime if discovered).
- **Domain**: `ActionDomain::Social`

#### `refuse_help`
- **Preconditions**: Agent is asked for assistance (lending, testimony, information). Agent has `Grudge` edge against the requester or social reason to refuse.
- **Duration**: Short.
- **Effect**: Rejects the request. Requester must seek help elsewhere. May create or reinforce a grudge in the requester.
- **Domain**: `ActionDomain::Social`

### 7. Goal Kinds

```rust
GoalKind::SeekRevenge { target: EntityId }
GoalKind::ProtectKin { kin: EntityId, threat: Option<EntityId> }
GoalKind::ShelterFugitive { fugitive: EntityId }
```

`refuse_help` is not a goal — it is a response modulation that affects whether existing social actions succeed or fail.

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: Currently harm, aid, and social events leave no lasting social memory. An agent who was wrongfully accused has no grudge. A healer who saved someone receives no future gratitude. Social interactions feel stateless.

2. **New entities/relations/records**: `SocialMemory` (component on Agent), `SocialEdge`, `SocialAftermathProfile` (component on Agent).

3. **Actions that mutate them**: `seek_revenge`, `protect_kin`, `shelter_fugitive`, `refuse_help`. Edge creation is automatic from event perception.

4. **Information production and travel**: Social edges are private to the agent — others cannot see someone's grudge list. The effects of social memory are visible through behavior (refusal, revenge, protection) which others observe.

5. **Conserved quantities**: None. Social memory is informational state.

6. **Scarce capacities and contention**: `SocialMemory.capacity` limits how many edges an agent tracks. Revenge-seeking occupies the agent and competes with other goals.

7. **Partial failures and aftermath**: Revenge attempt fails → combat casualties, new grudges on both sides. Shelter discovered → harboring agent faces charges. Protection fails → kin harmed anyway, grief.

8. **Positive feedback loops**: Grudge → revenge → counter-grudge → counter-revenge. Dampener: agents die (finite chain), revenge takes time and risk, competing needs pull agents away from revenge, forgiveness through decay.

9. **Physical dampeners**: Agent death terminates grudge chains. Travel time delays revenge. Combat risk deters revenge. Competing homeostatic needs. Memory capacity limits.

10. **Agent learning**: Social edges ARE the learning mechanism — they record what happened and influence future behavior. Edges decay with time, representing fading memory.

11. **How agents can be wrong**: Misattributed grudge (wrong person blamed for theft). Gratitude toward someone who helped for selfish reasons. Kin bond exploited by a manipulator. Stale grudge against someone who reformed.

12. **Lifecycle states**: SocialEdge: created → active (strength > 0) → decayed (strength at minimum) → evicted (capacity reached).

13. **Temporal resolution**: Edge creation during perception phase. Decay during world maintenance. Behavioral modulation during goal generation.

14. **Boundary conditions**: Social edges reference entity IDs. If a referenced entity leaves the simulation, the edge persists but the target is unreachable — the grudge cannot be acted on but the memory remains.

15. **Derived views**: None. SocialMemory is authoritative.

16. **Causal records**: Edge creation logged with event provenance. Revenge events logged. Protection events logged. All through existing event log.

17. **Target patterns**: Healer saves wounded traveler → later receives preferential aid. Thief punished but sibling retaliates. Wrongfully accused agent exonerated but not everyone forgives.

18. **Save/load and replay**: Standard ECS component. Deterministic edge decay.

## SystemFn Integration

Social edge creation runs during Phase 2 (perception/belief update) — after events are perceived, before goal generation. This ensures new social edges influence the next planning cycle.

Social edge decay runs during Phase 1 (world maintenance) alongside other decay processes.

No new phase needed — integrates into existing pipeline.

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `SocialMemory` | Agent | Universal | `Default` — all agents can form social memories |
| `SocialAftermathProfile` | Agent | Universal | `Default` — all agents have social aftermath parameters |

Both added to `AgentDef` with `unwrap_or_default()` in `spawn_agent()`.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Combat (E12) | Combat events create Grudge/Gratitude edges | State-mediated |
| Crime (E17) | Theft creates Grudge edges on victim | State-mediated |
| Justice (S63) | Wrongful accusation creates Grudge; alibi testimony creates Gratitude | State-mediated |
| Expectations (S59) | Rescue creates Gratitude; failed obligations create Grudge | State-mediated |
| Scarcity (S64) | Lending creates Gratitude + Obligation; debt default creates Grudge | State-mediated |
| Perception (E14) | Event perception triggers edge creation | State-mediated |
| Trade (S04, S10) | Grudge/Gratitude modulates willingness to trade with specific agents | State-mediated |
| RelationTables | Social edges complement but do not replace RelationTables hostile_to/loyal_to — those remain for faction-level and raw numeric relations | Complementary |

## Profile-Driven Parameters

`SocialAftermathProfile` is per-agent (scenario-configurable):
- `grudge_persistence`: varies from forgiving (200) to implacable (900)
- `kin_loyalty`: varies from detached (200) to absolute (1000)
- `revenge_threshold`: varies from volatile (300) to patient (800)
- `shelter_willingness`: varies from law-abiding (100) to loyal-above-law (800)
- `gratitude_weight`: varies from transactional (200) to deeply grateful (800)

Agent diversity (P22) demands significant variation across these parameters.
