# S61: Predator Ecology and Dens

## Summary

Add predator agents with territory-driven roaming, prey hunting, den habitation, and ecological pressure that displaces them into caravan routes and settlement peripheries. Currently all danger is human-originated (bandits, combat). This spec adds nonhuman threats that emerge from hunger pressure and habitat competition — creating beast starvation → caravan attack → report → bounty → hunt chains from general-purpose systems.

## Phase

Phase 7: Consequence Carriers

## Status

Draft

## Crates

- `worldwake-core` (predator profile, den component, evidence extensions)
- `worldwake-systems` (ecology actions, carcass/track evidence)
- `worldwake-ai` (predator goal generation, hunt-prey candidate generation)

## Dependencies

- S60 (persistent site occupancy) — den habitation uses the `OccupancyClaim` model
- E12 (combat) — completed
- E14 (perception) — completed
- S52 (evidence aftermath) — completed

## Design Goals

- Predators are regular `Agent` entities with beast-specific profiles — not a new `EntityKind`
- Predator behavior emerges from hunger, territory, and prey availability — not encounter spawning
- Dens are persistent sites (S60 `SiteProfile` with `SiteKind::Den`) with occupancy claims
- Carcasses and tracks are physical evidence that propagates through existing perception/belief channels
- Institutional and private hunt responses use existing notice/bounty infrastructure (S45)

## Non-Goals

- Predator breeding or population dynamics — deferred
- Pack coordination or social hierarchy among predators — deferred
- Domestication or taming — deferred
- Multiple predator species with complex food webs — start with one family, extend later
- Encounter spawning, threat directors, or global danger meters — explicitly forbidden (P1, P2)

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P1 (Emergence) | Beast attacks emerge from hunger + territory + co-location, not encounter scripts |
| P2 (No Ungrounded Triggers) | No `chanceOfEncounter` or roaming threat director. Predator movement follows hunger and territory logic |
| P3 (Concrete State) | Hunger is `HomeostaticNeeds.hunger`, territory is a set of places, prey availability is concrete entity counts |
| P5 (Carriers of Consequence) | Predator attacks create carcasses, tracks, survivor reports, bounties, route fear — all downstream carriers |
| P6 (World Runs Without Observers) | Predators roam, hunt, and den with or without human presence |
| P7 (Locality) | Route fear from predator sightings spreads through witness reports, not global danger score |
| P10 (Aftermath) | Attacks leave carcasses, tracks, blood trails, scattered cargo, wounded survivors |
| P19 (Agent Symmetry) | Predators use the same combat, needs, perception, and belief systems as human agents |
| P22 (Agent Diversity) | Individual predators differ in aggression, hunger tolerance, territory size, retreat threshold |

## Deliverables

### 1. Predator Profile Component

```rust
/// Behavioral profile for predator agents.
/// Registered on EntityKind::Agent. Role-specific.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PredatorProfile {
    /// Places the predator considers its territory (roaming range).
    pub territory: Vec<EntityId>,
    /// How far the predator will range when hungry (in additional hops beyond territory).
    pub range_expansion_hops: u8,
    /// Hunger threshold (Permille) at which the predator begins hunting.
    pub hunt_threshold: Permille,
    /// Hunger threshold at which the predator expands beyond normal territory.
    pub desperation_threshold: Permille,
    /// Aggression: how likely to attack large or defended targets.
    pub aggression: Permille,
    /// Retreat threshold: at what wound severity the predator flees.
    pub retreat_wound_threshold: Permille,
    /// Preferred prey category.
    pub prey_preference: PreyPreference,
    /// Den site, if the predator has one.
    pub den: Option<EntityId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PreyPreference {
    /// Prefers wild game or livestock.
    Herbivores,
    /// Prefers smaller/weaker targets.
    Opportunistic,
    /// Attacks anything when desperate.
    Indiscriminate,
}
```

### 2. Evidence Extensions

```rust
// Add to existing EvidenceKind enum:
pub enum EvidenceKind {
    // ... existing variants ...

    /// Animal tracks at a place.
    AnimalTracks {
        direction: Option<EntityId>,  // Place the tracks lead toward
        freshness: Permille,
        created_at: Tick,
    },
    /// Carcass of a killed animal or prey.
    Carcass {
        entity: EntityId,  // The dead entity
        killed_at: Tick,
        partially_consumed: bool,
    },
    /// Signs of predator presence (scat, claw marks, nesting materials).
    PredatorSign {
        created_at: Tick,
    },
}
```

### 3. New Actions

#### `roam`
- **Preconditions**: Agent has `PredatorProfile`. Not currently in combat or fleeing.
- **Duration**: Travel action (uses existing travel infrastructure). Destination is a random place within territory, or expanded range if desperate.
- **Effect**: Predator moves to a new place within territory. If co-located with prey and hungry, may trigger hunt decision. Leaves `AnimalTracks` evidence at origin.
- **Domain**: `ActionDomain::Travel`

#### `hunt_prey`
- **Preconditions**: Agent has `PredatorProfile`. Agent is hungry (above `hunt_threshold`). Prey entity is co-located or at an adjacent place. Prey matches `prey_preference` or agent is desperate.
- **Duration**: Combat-like (attack action against the prey entity).
- **Effect**: Initiates combat with the prey using existing combat system. On kill: creates `Carcass` evidence, predator feeds (reduces hunger). On failure: predator may be wounded, prey may flee.
- **Domain**: `ActionDomain::Combat`

#### `feed_on_carcass`
- **Preconditions**: Agent has `PredatorProfile`. A `Carcass` evidence entry exists at current place with `partially_consumed: false` or with remaining food value.
- **Duration**: Medium.
- **Effect**: Reduces predator hunger. Marks carcass as `partially_consumed`. Leaves `PredatorSign` evidence.
- **Domain**: `ActionDomain::Needs`

#### `retreat_to_den`
- **Preconditions**: Agent has `PredatorProfile` with `den: Some(place)`. Agent is wounded above `retreat_wound_threshold` or recently fled combat.
- **Duration**: Travel action to den place.
- **Effect**: Predator travels to den. On arrival, enters rest/recovery state (existing rest action). Leaves `AnimalTracks` from current location toward den.
- **Domain**: `ActionDomain::Travel`

#### `claim_den`
- **Preconditions**: Agent has `PredatorProfile`. Agent is at a place with `SiteProfile` and no hostile `OccupancyClaim` (or agent can displace current occupant). Site kind is suitable (Cave, Den, Ruin).
- **Duration**: Medium.
- **Effect**: Creates `OccupancyClaim` on the site with `OccupancyController::Individual(predator_id)`. Sets `PredatorProfile.den` to this site. Adds `SiteTraceKind::TerritorialMarking` and `PredatorSign` evidence.
- **Domain**: `ActionDomain::Generic`

### 4. Goal Kinds and Candidate Generation

```rust
GoalKind::HuntPrey { prey: EntityId }
GoalKind::Roam { destination: EntityId }
GoalKind::RetreatToDen
GoalKind::ClaimDen { site: EntityId }
GoalKind::FeedOnCarcass { carcass_place: EntityId }
```

**Candidate generation for predators**: When an agent has a `PredatorProfile`:
- If hunger > `hunt_threshold` and prey observed nearby → `HuntPrey`
- If hunger > `hunt_threshold` and no prey nearby → `Roam` to a new territory location
- If hunger > `desperation_threshold` → `Roam` with expanded range
- If wounded above `retreat_wound_threshold` and has den → `RetreatToDen`
- If no den and at suitable site → `ClaimDen`
- If carcass at current place → `FeedOnCarcass`

Predators still have `HomeostaticNeeds` for thirst, fatigue, etc. — hunger is the primary driver but other needs compete for attention through existing need-pressure system.

### 5. Route Fear Propagation

No new fear component. Route danger perception works through existing mechanisms:
- Survivors of predator attacks carry `RouteExperience` with hostile encounter data
- Witnesses observe predator presence and propagate via Tell/SocialObservation
- `AnimalTracks` and `PredatorSign` evidence at places updates agent beliefs about route safety
- `PreferenceProfile.route_caution_weight` (S38) influences route selection based on accumulated danger beliefs

### 6. Institutional Hunt Response

Uses existing infrastructure:
- Survivor reports predator attack to office → office creates bounty (S45 `SocialArtifact::Bounty`) targeting the predator
- Bounty is posted at notice board → agents observe it
- Agents with sufficient combat capability and matching goal generation respond
- Proof of kill (carcass, trophy) satisfies bounty conditions

No new institutional types needed. The predator is just another hostile entity that the existing bounty system handles.

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: All world danger is currently human-originated. Without nonhuman threats, the world lacks ecological pressure, route danger from wildlife, and the beast-bounty-hunt cycle that canonical regression A demands.

2. **New entities/relations/records**: `PredatorProfile` (component on Agent), `EvidenceKind::AnimalTracks/Carcass/PredatorSign` (extends existing enum), predator agents as regular Agent entities.

3. **Actions that mutate them**: `roam` (moves predator, leaves tracks), `hunt_prey` (combat, creates carcass), `feed_on_carcass` (consumes prey, marks carcass), `retreat_to_den` (moves predator), `claim_den` (creates occupancy claim).

4. **Information production and travel**: Predator sightings propagate through witness observation → Tell → belief update. Tracks and signs are locally observable evidence. Bounties propagate through notice system. No global threat awareness.

5. **Conserved quantities**: Predator feeding reduces hunger (homeostatic need — not a conserved good). Carcasses are evidence entries, not items. Prey death follows existing combat death mechanics.

6. **Scarce capacities and contention**: Den occupancy is exclusive (one occupant per site via `OccupancyClaim`). Territory is not exclusive — predators may have overlapping territories leading to competition. Hunt attempts occupy the predator (combat duration).

7. **Partial failures and aftermath**: Failed hunt → predator wounded, prey flees, evidence left. Partial feeding → half-consumed carcass attracts scavengers. Den claimed by another → competition or displacement.

8. **Positive feedback loops**: Prey depletion → range expansion → more attacks → more bounties → more hunting → predator death. This is self-dampening: the predator eventually dies if prey is gone and hunters respond.

9. **Physical dampeners**: Predator fatigue and wounds limit hunting frequency. Prey scarcity in expanded range. Institutional hunt response. Predator death permanently removes the threat (no respawn). Den limitations (finite suitable sites).

10. **Agent learning**: Human agents update route danger beliefs from predator sightings and attack reports. Predators do not learn in this spec (simple threshold-based behavior). Future specs may add predator learning.

11. **How agents can be wrong**: Stale predator sighting — predator has moved. Tracks attributed to a predator that left the area. Rumor of dangerous route when predator is dead. Bounty remains posted after predator killed elsewhere.

12. **Lifecycle states**: PredatorProfile: static configuration. Carcass evidence: created → partially_consumed → decayed. OccupancyClaim for dens: same lifecycle as S60.

13. **Temporal resolution**: Predator decision-making uses the same agent-tick as all other agents. Roaming and hunting are standard duration-bearing actions.

14. **Boundary conditions**: Predators may roam to map-edge places. Predators do not cross simulation boundaries. Future boundary specs (S62) may introduce migrating herds as boundary events.

15. **Derived views**: None. All predator state is authoritative (profile, needs, location, evidence).

16. **Causal records**: Hunt events logged (predator, prey, place, tick, outcome). Carcass creation logged. Den claim logged. All through existing event log infrastructure.

17. **Target patterns**: Prey shortage → range expansion → caravan attack → survivors report → bounty posted → hunter tracks and kills → reward claimed. Cleared den → route safer → until another predator claims it.

18. **Save/load and replay**: `PredatorProfile` is a standard ECS component. Evidence entries are standard. All deterministic (ChaCha8Rng for combat outcomes, BTreeMap for territory iteration).

## SystemFn Integration

No new system tick function. Predator behavior is driven entirely by goal generation and action execution through the existing agent decision pipeline. Predators are agents — they use the same tick schedule as all other agents.

Evidence decay for `AnimalTracks`, `Carcass`, and `PredatorSign` uses the existing evidence decay system from S52.

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `PredatorProfile` | Agent | Role-specific | `None` — only predator agents |

`PredatorProfile` added to `AgentDef` as `Option<PredatorProfile>` with conditional application in `spawn_agent()`. A `PredatorProfileDef` wrapper type is needed because `territory` and `den` reference `EntityId` values that must be resolved from scenario string names (following `PatrolRouteDef` pattern).

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Combat (E12) | `hunt_prey` uses existing combat system for attack resolution | State-mediated |
| Needs (E09) | Predator hunger drives hunt behavior; feeding reduces hunger | State-mediated |
| Evidence (S52) | Carcass, tracks, and predator signs are evidence entries | State-mediated |
| Perception (E14) | Predator and prey perceive each other through standard perception | State-mediated |
| Bounty (S45) | Survivor reports trigger bounty creation through existing institutional flow | State-mediated |
| Site occupancy (S60) | Dens use `OccupancyClaim` and `SiteProfile` | State-mediated |
| Route preferences (S38) | Predator encounters update route danger beliefs via `RouteExperience` | State-mediated |
| Travel | Roaming and retreat use existing travel actions | State-mediated |

## Profile-Driven Parameters

All behavioral thresholds are per-predator via `PredatorProfile`:
- `hunt_threshold`, `desperation_threshold`: when to hunt and when to range-expand
- `aggression`: target selection (avoids defended caravans at low aggression)
- `retreat_wound_threshold`: when to flee
- `range_expansion_hops`: how far to roam when desperate
- `prey_preference`: what to attack

`MetabolismProfile` on predator agents controls hunger/thirst/fatigue rates. Higher hunger rate = more frequent hunting need.

`CombatProfile` on predator agents controls attack/defense capability. Different predator species (in future) will have different combat profiles.
