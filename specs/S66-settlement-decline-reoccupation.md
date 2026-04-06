# S66: Settlement Decline and Reoccupation

## Summary

Add mechanics for settlement decline as an emergent outcome of sustained pressure — household departure, facility closure, building vacancy, and eventual reoccupation by squatters or new settlers. Currently settlements are static: buildings never close, households never leave, and ruined towns cannot emerge from ordinary world processes. This spec makes decline and recovery emerge from population, assets, duties, inflow, and fear — not from a settlement health score.

## Phase

Phase 7: Consequence Carriers

## Status

Draft

## Crates

- `worldwake-core` (vacancy, departure, reoccupation types)
- `worldwake-systems` (departure, closure, reoccupation actions)
- `worldwake-ai` (departure decision goals, reoccupation candidate generation)

## Dependencies

- S60 (persistent site occupancy) — vacant buildings become occupyable sites
- S64 (scarcity response) — sustained shortage drives departure decisions
- S65 (social aftermath memory) — social bonds influence departure vs. staying decisions

## Design Goals

- Decline emerges from population loss, resource depletion, and institutional failure — not a settlement score
- Individual households and businesses make independent departure decisions based on their concrete circumstances
- Vacant buildings are occupyable by squatters, migrants, or returning settlers through the S60 site model
- Institutional degradation compounds: vacant offices → patrol gaps → more crime → more departures
- Recovery requires actual new inflow, production, or population movement — not a recovery timer

## Non-Goals

- Settlement health bar or prosperity index — explicitly forbidden (P3)
- Centralized settlement management AI — no settlement controller entity
- Procedural settlement generation — new settlements require explicit founding events
- Immigration system — new arrivals come through boundary processes (S62) or internal migration
- Building construction or repair — deferred to a construction spec

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P1 (Emergence) | Decline is an emergent pattern from individual agent decisions, not a scripted settlement event |
| P3 (Concrete State) | Vacancy is a concrete state on a building/facility entity, not a derived decline score |
| P4 (Persistent Identity) | Buildings, businesses, and their vacancy state persist — abandoned buildings don't vanish |
| P5 (Carriers of Consequence) | Each departure propagates: fewer customers for remaining merchants, fewer laborers for production, weaker patrol |
| P6 (World Runs Without Observers) | Decline continues without player presence |
| P7 (Locality) | Agents decide to leave based on local conditions they can observe — not global settlement metrics |
| P10 (Aftermath) | Departed households leave vacant buildings, reduced labor, fewer customers — aftermath compounds |

## Deliverables

### 1. Facility Vacancy Component

```rust
/// Marks a facility as vacant (closed shop, abandoned home, empty barracks).
/// Registered on EntityKind::Facility (or Place with facility tag).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FacilityVacancy {
    /// Why the facility is vacant.
    pub reason: VacancyReason,
    /// When it became vacant.
    pub since_tick: Tick,
    /// Who last operated it.
    pub last_operator: Option<EntityId>,
    /// Ownership status of the vacant facility.
    pub ownership: VacancyOwnership,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum VacancyReason {
    /// Operator left the settlement.
    OperatorDeparted,
    /// Operator died.
    OperatorDied,
    /// Business failed (no customers, no stock).
    BusinessFailure,
    /// Evicted or confiscated.
    Evicted,
    /// Abandoned due to danger.
    FledDanger,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum VacancyOwnership {
    /// Original owner still claims it (departed but may return).
    OwnerAbsent,
    /// No living owner — available for reoccupation.
    Unclaimed,
    /// Institution holds it (confiscated, office property).
    Institutional { office: EntityId },
}
```

### 2. Departure Intent

```rust
/// An agent's intention to leave the settlement permanently.
/// This is not a component — it is a GoalKind that produces departure behavior.
/// Included here for type documentation.

GoalKind::LeaveSettlement {
    /// Current home settlement.
    from: EntityId,
    /// Destination (if known). None = wander/flee.
    destination: Option<EntityId>,
    /// Why the agent is leaving.
    reason: DepartureReason,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DepartureReason {
    /// Sustained inability to meet basic needs.
    Starvation,
    /// Unacceptable danger level from personal experience.
    Danger,
    /// Social conflict (grudges, persecution, loss of kin).
    SocialConflict,
    /// Loss of livelihood (facility closed, no work).
    EconomicCollapse,
    /// Following a departed kin member.
    FollowingKin,
}
```

### 3. Departure Decision Logic

Agents do not have a "departure threshold" — the decision emerges from accumulated pressure signals:

**Pressure factors** (read from existing agent state, not new scores):
- `HomeostaticNeeds`: sustained critical hunger/thirst (N ticks at critical level)
- `DemandMemory`: repeated purchase failures with no substitution available
- `SocialMemory` (S65): unresolved grudges, loss of kin, persecution
- `ViolationMemory`: multiple overdue expectations (S59), repeated exposure to danger
- `RouteExperience`: dangerous routes making the settlement feel isolated

**Anchor factors** (reasons to stay):
- `SocialMemory` (S65): kin bonds, gratitude, patronage relationships at this settlement
- Active employment (facility operator, office holder)
- Stored goods and property at this settlement
- No known safer destination

Candidate generation emits `LeaveSettlement` when pressure factors exceed anchor factors over a sustained period. The exact evaluation is a weighted comparison using existing agent profile parameters — no new "departure willingness" score.

### 4. New Actions

#### `close_facility`
- **Preconditions**: Agent is the operator of a facility. Agent has decided to depart or cannot sustain the business (no stock, no customers for extended period).
- **Duration**: Medium (packing up, securing remaining goods).
- **Effect**: Creates `FacilityVacancy` on the facility entity. Agent takes portable goods. Facility's sale listings are removed. Production schedules are stopped.
- **Domain**: `ActionDomain::Trade`

#### `depart_settlement`
- **Preconditions**: Agent has `LeaveSettlement` goal. Agent has closed any facilities they operate. Agent has gathered portable belongings.
- **Duration**: Travel action to destination (or to a map edge if fleeing).
- **Effect**: Agent travels to destination. If destination is a boundary place, the agent effectively leaves the simulation. Home place is updated. Any institutional roles are vacated (triggering succession from E16).
- **Domain**: `ActionDomain::Travel`

#### `occupy_vacant_facility`
- **Preconditions**: Agent is at a place with a `FacilityVacancy` where `ownership: Unclaimed`. Agent has the skills/profile to operate the facility type. No institutional prohibition.
- **Duration**: Medium (moving in, establishing operations).
- **Effect**: Removes `FacilityVacancy`. Agent becomes the new operator. If the facility has a `SiteProfile` (S60), creates an `OccupancyClaim`.
- **Domain**: `ActionDomain::Generic`

#### `scavenge_vacant`
- **Preconditions**: Agent is at a vacant facility or abandoned building. Goods remain inside (containers with items).
- **Duration**: Medium (searching and collecting).
- **Effect**: Agent takes available items from the facility's containers. May create evidence (S52 container tampered). If the facility has an absent owner (`OwnerAbsent`), this is technically theft.
- **Domain**: `ActionDomain::Needs`

### 5. Institutional Degradation

When agents depart:
- If the departing agent holds an office → office becomes vacant → existing succession system (E16) activates
- If no successor is found → duties lapse → patrol gaps (regression F continues to worsen)
- If a merchant departs → one fewer supplier → remaining demand concentrated on fewer merchants → potential stockout cascade
- If production workers depart → less local production → greater dependency on boundary inflows (S62)

This is not a new system — it is the compound effect of existing systems losing their participants. The spec's contribution is giving agents the ability to leave, thereby creating the cascade.

### 6. Goal Kinds

```rust
GoalKind::LeaveSettlement {
    from: EntityId,
    destination: Option<EntityId>,
    reason: DepartureReason,
}
GoalKind::CloseFacility { facility: EntityId }
GoalKind::OccupyVacantFacility { facility: EntityId }
GoalKind::ScavengeVacant { place: EntityId }
```

**Candidate generation**:
- `LeaveSettlement`: generated when sustained pressure exceeds anchor factors (see section 3)
- `CloseFacility`: generated when `LeaveSettlement` is active and agent operates a facility
- `OccupyVacantFacility`: generated when a migrant/displaced agent arrives at a settlement with vacant facilities matching their skills
- `ScavengeVacant`: generated when an agent has unmet needs and is at a place with vacant facilities containing goods

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: Settlements currently cannot decline. No agent can leave. No business can close. No building can become vacant. This prevents the long-horizon world self-authorship the design note targets.

2. **New entities/relations/records**: `FacilityVacancy` (component on Place/Facility), `DepartureReason`, `VacancyReason`, `VacancyOwnership`.

3. **Actions that mutate them**: `close_facility` (creates vacancy), `depart_settlement` (removes agent from settlement), `occupy_vacant_facility` (removes vacancy), `scavenge_vacant` (removes goods from vacant facility).

4. **Information production and travel**: Vacancy is locally observable (empty building, closed shop). Departures are observable by co-located agents. No global population tracking. Agents in other settlements learn about decline through rumors and travelers.

5. **Conserved quantities**: Departing agents take their goods (item conservation maintained). Facility inventory remains at the facility until scavenged or decayed. No goods vanish from departure.

6. **Scarce capacities and contention**: Vacant facilities are a scarce resource — multiple agents may compete to occupy the same vacancy (resolved through standard contention: first actor to complete `occupy_vacant_facility` wins).

7. **Partial failures and aftermath**: Departure plan abandoned mid-trip → agent may return to find their facility already occupied. Reoccupation attempt fails (facility in bad condition) → agent must repair or find another. Scavenging a facility with absent owner → theft charges if owner returns.

8. **Positive feedback loops**: Departure → fewer customers → more departures → settlement collapse. Dampener: remaining agents may have strong anchor factors (kin, property), new arrivals through boundary (S62), reoccupation of vacancies by incoming agents, institutional adaptation (rationing S64, bounties for needed roles).

9. **Physical dampeners**: Travel time and danger deter casual departure. Portable goods limit how much an agent can take. Destination must be known and reachable. Kin bonds (S65) anchor agents. Some agents are too injured or old to travel.

10. **Agent learning**: Agents observe vacancy accumulation and update beliefs about settlement viability. Recent arrivals compare settlement state to expectations.

11. **How agents can be wrong**: Agent leaves based on temporary shortage that resolves after departure. Agent stays too long hoping for recovery that never comes. Agent believes a destination is better but it's worse.

12. **Lifecycle states**: FacilityVacancy: created → Unclaimed → occupied (removed) / decayed (structural). No explicit lifecycle transitions — vacancy is removed by reoccupation or persists indefinitely.

13. **Temporal resolution**: Departure decisions are agent-driven. No per-tick settlement health check. Vacancy is created by actions and persists until acted upon.

14. **Boundary conditions**: Agents departing to a boundary place exit the simulation. This is the inverse of S62 (arrivals through boundary). Departure to another in-simulation settlement is internal migration.

15. **Derived views**: None. Vacancy and departure state is authoritative.

16. **Causal records**: Departure events logged with agent, settlement, reason, destination. Facility closure logged. Reoccupation logged. All through existing event log.

17. **Target patterns**: Repeated convoy failure + patrol weakness → shop closure → household departure → abandoned building reused by squatters. Recovered route + new inflow → partial reoccupation of declining site.

18. **Save/load and replay**: All components are standard ECS. Departure and vacancy state persists through save/load.

## SystemFn Integration

No new system tick function. All settlement decline is action-driven:
- Agents decide to leave through goal generation
- Facility closure is an action
- Departure is a travel action
- Institutional degradation happens automatically through existing vacancy/succession systems

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `FacilityVacancy` | Place | Role-specific | `None` — only vacant facilities |

`FacilityVacancy` is runtime-generated state (created by `close_facility`), not scenario-configured. Exempt from `AgentDef` requirements.

Scenarios can pre-define vacant facilities by including `FacilityVacancy` in place definitions to create starting conditions with ruins or abandoned buildings.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Scarcity (S64) | Sustained shortage drives departure decisions | State-mediated |
| Social aftermath (S65) | Kin bonds anchor agents; social conflict drives departure | State-mediated |
| Site occupancy (S60) | Vacant buildings become occupyable sites | State-mediated |
| Boundary (S62) | Departing agents exit through boundary; arriving agents may reoccupy | State-mediated |
| Institutions (E16) | Departing office-holders trigger succession; vacant offices degrade services | State-mediated |
| Production (E10) | Closed facilities reduce local production capacity | State-mediated |
| Trade (E10) | Fewer merchants → reduced supply → scarcity pressure | State-mediated |
| Patrol (E19) | Fewer guards → patrol gaps → increased crime/danger → more departure | State-mediated |
| Crime (E17) | Scavenging vacant facilities with absent owners may constitute theft | State-mediated |

## Profile-Driven Parameters

No new per-agent profile component. Departure decisions use existing profiles:
- `HomeostaticNeeds` + `MetabolismProfile` for starvation pressure
- `DriveThresholds` for danger tolerance
- `SocialAftermathProfile` (S65) for kin loyalty and social conflict sensitivity
- `ScarcityResponseProfile` (S64) for economic distress tolerance
- `PreferenceProfile` for route caution and risk aversion

Agent diversity (P22) in these existing profiles ensures different agents depart at different pressure levels — some flee at the first sign of trouble, others stay until the end.
