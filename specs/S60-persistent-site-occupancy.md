# S60: Persistent Site Occupancy

## Summary

Generalize place identity so sites can persist, be entered, be occupied, be cleared, and retain traces of prior use. Currently places are flat graph nodes with tags and capacity. This spec adds site profiles with medium-grain sublocations (child places), occupancy claims, and site-local traces that accumulate over time — creating persistent, reusable adventure sites without encounter tables or reset logic.

## Phase

Phase 7: Consequence Carriers

## Status

Draft

## Crates

- `worldwake-core` (site types, topology extension)
- `worldwake-systems` (site occupancy actions, trace materialization)
- `worldwake-ai` (site-related goal generation)

## Dependencies

- E02 (world topology) — completed
- S50 (rights lattice) — completed
- S52 (evidence aftermath) — completed

## Design Goals

- Places can have child sublocations (a ruin has an entrance, a hall, a cellar) connected by internal edges
- Occupancy is an explicit claim — a faction or agent declares control over a site, observable by visitors
- Sites accumulate traces (prior occupancy, combat aftermath, cached goods, abandoned equipment) that persist and decay
- The model is medium-grain: 2–8 sublocations per site, not tile-level interiors
- Existing `BanditCamp` pattern generalizes into the site occupancy model rather than remaining a special case

## Non-Goals

- Doors, locks, barriers, and access control mechanics — deferred to a future access-control spec
- Hidden stash spots and defendable chokepoints — deferred
- Procedural site generation — sites are authored in scenarios or created by world processes
- Tile-level or grid-based interiors — the world remains a place graph
- Dungeon content or reset logic — explicitly forbidden (P1)

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P1 (Emergence) | Sites change occupants through ordinary world processes, not encounter tables or refresh timers |
| P3 (Concrete State) | Occupancy is a stored claim with claimant, faction, and tick — not a derived score |
| P4 (Persistent Identity) | Sites and sublocations have stable identity; traces persist across occupant changes |
| P5 (Carriers of Consequence) | Site traces propagate downstream effects — a bloodstained hall tells a story, cached goods create theft/discovery opportunities |
| P7 (Locality) | Site reputation is belief state (agents who have visited or heard rumors), not global truth |
| P8 (Preconditions) | Occupying a site requires travel to it, clearing requires co-location and possibly combat |
| P10 (Aftermath) | Clearing a site leaves traces; abandoning a site leaves evidence of prior use |
| P18 (Records Are World State) | Site traces are inspectable world state, not log-only data |
| P28 (No Backward Compat) | `BanditCamp` component is replaced by the generalized `OccupancyClaim`, not wrapped |

## Deliverables

### 1. Site Profile Component

```rust
/// Marks a place as a structured site with sublocations.
/// Registered on EntityKind::Place.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SiteProfile {
    /// What kind of site this is (ruin, cave, watchtower, shrine, etc).
    pub site_kind: SiteKind,
    /// Child place entities that are sublocations of this site.
    /// These are regular Place entities connected to this place by internal edges.
    pub sublocations: Vec<EntityId>,
    /// Base concealment of the site (forests hide better than open ruins).
    pub concealment: Permille,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SiteKind {
    Ruin,
    Cave,
    Watchtower,
    Shrine,
    Crypt,
    Den,
    Hideout,
    Homestead,
}
```

### 2. Occupancy Claim Component

```rust
/// An explicit claim of control over a site by an agent or faction.
/// Registered on EntityKind::Place.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OccupancyClaim {
    /// Who controls the site.
    pub controller: OccupancyController,
    /// When the claim was established.
    pub since_tick: Tick,
    /// How many entities are present as part of this claim.
    pub garrison_count: u16,
    /// Whether the site is actively defended or merely claimed.
    pub posture: OccupancyPosture,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OccupancyController {
    /// A faction claims the site.
    Faction(EntityId),
    /// An individual claims the site (hermit, squatter).
    Individual(EntityId),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OccupancyPosture {
    /// Actively garrisoned and defended.
    Defended,
    /// Claimed but not actively defended (owners away, skeleton crew).
    Nominal,
    /// Recently abandoned but claim not formally relinquished.
    Abandoned { since_tick: Tick },
}
```

### 3. Site Trace Component

```rust
/// Accumulated traces of activity at a site, persisting across occupant changes.
/// Extends the SceneEvidence concept with site-specific long-term traces.
/// Registered on EntityKind::Place.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SiteTraces {
    pub entries: Vec<SiteTraceEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SiteTraceEntry {
    pub id: SiteTraceId,
    pub kind: SiteTraceKind,
    pub created_tick: Tick,
    /// Traces decay but more slowly than SceneEvidence.
    pub decay_ticks: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SiteTraceId(pub u64);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SiteTraceKind {
    /// Evidence of prior habitation (fire pits, bedding, refuse).
    PriorHabitation { occupant_kind: Option<SiteKind> },
    /// Combat aftermath at the site.
    CombatAftermath { severity: Permille },
    /// Cached or abandoned goods.
    AbandonedGoods { container: EntityId },
    /// Faction markings or territorial signs.
    TerritorialMarking { faction: Option<EntityId> },
    /// Structural damage or decay.
    StructuralDecay { severity: Permille },
}
```

### 4. PlaceTag Extensions

```rust
// Add to existing PlaceTag enum:
pub enum PlaceTag {
    // ... existing variants ...
    Ruin,
    Cave,
    Watchtower,
    Shrine,
    Den,
}
```

### 5. New Actions

#### `occupy_site`
- **Preconditions**: Actor (or actor's faction) is at the site. Site has no existing defended `OccupancyClaim`, or actor has cleared it. Actor has sufficient force (faction members present).
- **Duration**: Medium (establishing camp-like duration).
- **Effect**: Creates `OccupancyClaim` on the site's place entity. Adds `SiteTraceKind::TerritorialMarking`. If replacing existing claim, the old claim is removed (not superseded — occupancy is exclusive).
- **Domain**: `ActionDomain::Generic`

#### `search_site`
- **Preconditions**: Actor is at the site (or a sublocation).
- **Duration**: Medium (similar to investigate action).
- **Effect**: Reveals `SiteTraces` and `SceneEvidence` at the searched sublocation. Updates actor's beliefs about site contents and occupancy. May discover hidden entities or containers.
- **Domain**: `ActionDomain::Epistemic`

#### `clear_site`
- **Preconditions**: Actor is at the site. Site has a hostile `OccupancyClaim`. Actor has combat capability.
- **Duration**: Extended (combat + securing sublocations).
- **Effect**: Initiates combat with occupants. If successful, removes or changes `OccupancyClaim`. Adds `SiteTraceKind::CombatAftermath`. Existing traces persist.
- **Domain**: `ActionDomain::Combat`

#### `abandon_site`
- **Preconditions**: Actor controls the site (has `OccupancyClaim`).
- **Duration**: Short (declaration).
- **Effect**: Transitions `OccupancyPosture` to `Abandoned`. Adds `SiteTraceKind::PriorHabitation`. Does not remove the claim immediately — allows grace period for return.
- **Domain**: `ActionDomain::Generic`

#### `inspect_site`
- **Preconditions**: Actor is at the site.
- **Duration**: Short.
- **Effect**: Actor observes surface-level site state: `OccupancyClaim` (if visible), `SiteProfile`, obvious traces. Does not search sublocations.
- **Domain**: `ActionDomain::Epistemic`

### 6. Goal Kinds

```rust
GoalKind::OccupySite { site: EntityId }
GoalKind::ClearSite { site: EntityId }
GoalKind::SearchSite { site: EntityId }
GoalKind::AbandonSite { site: EntityId }
```

**Candidate generation**: Factions with `BanditFactionPolicy` (or similar doctrine) generate `OccupySite` goals when they need a base. Institutional agents generate `ClearSite` when they learn of hostile occupancy through patrol or report. `SearchSite` is generated when an agent has reason to believe something of interest is at a site (rumor, last-seen record from S59, investigation lead).

### 7. BanditCamp Migration

The existing `BanditCamp` component becomes a thin wrapper or is replaced by `OccupancyClaim` + `SiteProfile`:

- `BanditCamp.faction` → `OccupancyClaim.controller: Faction(id)`
- `BanditCamp.supplies_container` → Container placed at site sublocation
- `BanditCamp.empty_since_tick` → `OccupancyPosture::Abandoned { since_tick }`
- `BanditFactionPolicy.abandonment_grace_period` → read against `Abandoned.since_tick`

This is a P28-compliant replacement, not a backward-compatibility wrapper.

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: Places currently have no layered history or occupancy tracking. A cave is functionally identical whether it was occupied yesterday or never. This prevents emergent reuse of sites by different factions, fugitives, predators, or squatters.

2. **New entities/relations/records**: `SiteProfile` (component on Place), `OccupancyClaim` (component on Place), `SiteTraces` (component on Place), `SiteTraceEntry`, sublocation places as child entities.

3. **Actions that mutate them**: `occupy_site` (creates claim), `clear_site` (removes hostile claim, adds combat traces), `abandon_site` (transitions posture), `search_site`/`inspect_site` (reads state, updates beliefs).

4. **Information production and travel**: Site occupancy is observable by co-located agents through perception. Site reputation propagates as belief through tell/rumor. Traces are locally observable. No global site registry.

5. **Conserved quantities**: Goods cached at sites follow normal item conservation. Occupancy claims are exclusive per-site (one controller at a time).

6. **Scarce capacities and contention**: A site has one `OccupancyClaim` at a time. If two factions attempt to occupy the same site, the conflict resolves through combat (existing combat system). Site capacity limits how many entities can be present.

7. **Partial failures and aftermath**: Clearing fails → combat casualties on both sides, partial damage traces. Abandoned site → traces remain, goods may be left behind. Occupation attempt fails → retreat, trace evidence of attempt.

8. **Positive feedback loops**: Occupying sites → resources → stronger faction → more sites. Dampener: maintaining occupancy costs garrison presence (those agents can't do other things), sites have finite capacity, other factions or institutions respond to expansion.

9. **Physical dampeners**: Garrison occupies agents, travel time to distant sites, maintenance needs of garrison, competing homeostatic needs, institutional response to hostile occupancy.

10. **Agent learning**: Agents update beliefs about site occupancy from observation and rumor. Route preferences may shift to avoid occupied hostile sites.

11. **How agents can be wrong**: Stale rumor claims a site is occupied when it was abandoned. Agent believes site is safe when a new occupant arrived. Traces misinterpreted (old combat marks attributed to recent events).

12. **Lifecycle states**: OccupancyClaim: Defended → Nominal → Abandoned → removed. SiteTraces: created → active → decayed → removed. SiteProfile: static (created with the place).

13. **Temporal resolution**: Occupancy changes are action-driven (no per-tick decay). Trace decay is tick-based (checked during world maintenance phase). Abandonment grace period is tick-counted.

14. **Boundary conditions**: Sites at map edges follow the same model. Off-map sites are not modeled (boundary processes from S62 handle off-map entity flow).

15. **Derived views**: None. All site state is authoritative.

16. **Causal records**: Occupancy changes logged with actor, site, tick. Combat at sites logged through existing combat event system. Trace creation logged.

17. **Target patterns**: Abandoned watchtower → outlaw occupation after patrol gap → stolen goods cached → eventual discovery. Guards clear cave → traces remain → scavengers reuse later. Rumor of occupied ruin → traveler arrives → different occupants than rumor claimed.

18. **Save/load and replay**: All components are standard ECS — survive save/load. Sublocation places are regular Place entities in the topology.

## SystemFn Integration

No new system tick function needed. Site occupancy changes are action-driven. Trace decay can piggyback on the existing evidence decay system from S52 (same maintenance phase).

Abandonment grace period checking can run as part of the existing camp-abandonment check in the world maintenance phase, generalized from the current `BanditCamp`-specific logic.

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `SiteProfile` | Place | Role-specific | `None` — only structured sites, not every road segment |
| `OccupancyClaim` | Place | Role-specific | `None` — only occupied sites |
| `SiteTraces` | Place | Role-specific | `Default` (empty) — only sites that have been used |

`SiteProfile` is scenario-defined on Place entities. Added to place definition in scenario types.
`OccupancyClaim` is created by actions at runtime, not scenario-defined.
`SiteTraces` accumulates at runtime from actions.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Combat | `clear_site` triggers combat with occupants; combat aftermath creates traces | State-mediated |
| Evidence (S52) | `SiteTraces` complements `SceneEvidence` — site traces are long-term, evidence is short-term | State-mediated |
| Perception (E14) | Co-located agents perceive `OccupancyClaim` and surface traces | State-mediated |
| Patrol (E19) | Patrol discovery of hostile occupancy triggers institutional response | State-mediated |
| Bandit camps (E19) | `BanditCamp` migrated to `OccupancyClaim` | Replacement (P28) |
| Travel | Sublocations are places connected by edges — standard travel applies | State-mediated |
| Rights (S50) | `OccupancyClaim` interacts with `RightKind::FactionAuthority` for access decisions | State-mediated |

## Profile-Driven Parameters

`SiteProfile.concealment` is per-site (scenario-configurable). Caves have high concealment, watchtowers have low.

`SiteTraceEntry.decay_ticks` is per-trace-kind. Combat aftermath decays faster than structural markings.

Abandonment grace period is governed by the controlling faction's policy (existing `BanditFactionPolicy.abandonment_grace_period` pattern, generalized).
