# S62: Boundary Processes and Remote Shocks

## Summary

Implement explicit boundary processes so off-map dependencies enter the simulation through lawful channels with named origins, travel delay, capacities, failure modes, and observables. Currently the world is causally closed — nothing arrives from or departs to outside the simulated region. This spec adds source regions, boundary channels, scheduled inflows, and disruption mechanics so the settlement can experience upstream shortages, delayed convoys, refugee arrivals, and external decrees through Principle 13–compliant world processes.

## Phase

Phase 7: Consequence Carriers

## Status

Draft

## Crates

- `worldwake-core` (boundary types, source region, channel)
- `worldwake-sim` (boundary tick system)
- `worldwake-systems` (arrival actions, non-arrival detection)
- `worldwake-ai` (expectation-based detection goals)

## Dependencies

- E02 (world topology) — completed
- E09 (needs/metabolism) — completed
- E10 (production/transport) — completed
- S59 (expectation-obligation substrate) — provides the expectation/overdue mechanism that agents use to notice failed arrivals

## Design Goals

- Off-map sources are explicit entities with named regions, not hidden spawners
- Arrivals travel through boundary channels with declared delay, capacity, and failure modes
- Local agents detect non-arrival through expectation violation (S59), not global notification
- Disruptions are concrete world events (raid, storm, embargo) with causal origin, not drama dials
- The model starts with one concrete dependency class (staple food inflow) and generalizes

## Non-Goals

- Full off-map simulation (simulating the external world in detail) — source regions are lower-fidelity abstractions
- Dynamic off-map trade negotiation — inflows are scheduled or disrupted, not actively negotiated
- Weather systems or seasonal cycles — deferred unless needed as disruption sources
- Emigration or departure to off-map — deferred (S66 handles internal departure)

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P1 (Emergence) | Shortages emerge from actual non-arrival of expected inflows, not scripted scarcity events |
| P2 (No Ungrounded Triggers) | Disruptions have concrete causes (raid, storm, embargo) with named source regions and channels |
| P3 (Concrete State) | Inflow state is stored (manifest, expected tick, actual state), not derived from a scarcity score |
| P5 (Carriers of Consequence) | Failed inflows propagate downstream: stockout → substitution → theft → social pressure |
| P7 (Locality) | Agents learn about disruption through non-arrival observation and messenger reports, not global notification |
| P13 (Boundaries) | Directly satisfies — explicit boundary processes with named sources, routes, constraints, and observables |
| P14 (World ≠ Belief) | Agents expect arrivals based on schedule beliefs; actual arrival state is separate |
| P17 (Violated Expectation) | Non-arrival triggers expectation violation through S59 overdue detection |

## Deliverables

### 1. Source Region Entity

```rust
/// Represents a named off-map region that can send goods, people, or messages.
/// Registered on EntityKind::Place (uses the same entity kind — source regions
/// are places at the edge of the world graph).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceRegion {
    /// Human-readable name of the external region.
    pub region_name: String,
    /// What this region can supply.
    pub exports: Vec<BoundaryExport>,
    /// Current state of the region (stable, disrupted, collapsed).
    pub state: SourceRegionState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoundaryExport {
    pub commodity: CommodityKind,
    /// Maximum quantity per shipment.
    pub max_quantity: u32,
    /// How reliably this export arrives (under normal conditions).
    pub reliability: Permille,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SourceRegionState {
    /// Normal operations.
    Stable,
    /// Partially disrupted (reduced output, delays).
    Disrupted { cause: DisruptionCause, since_tick: Tick },
    /// Fully collapsed (no output until recovery).
    Collapsed { cause: DisruptionCause, since_tick: Tick },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DisruptionCause {
    Raid,
    Storm,
    Embargo,
    Plague,
    War,
    BridgeCollapse,
    Famine,
}
```

### 2. Boundary Channel

```rust
/// A route connecting a source region to a local settlement entry point.
/// Represented as a TravelEdge in the topology with additional metadata.
/// Component on the boundary Place entity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoundaryChannel {
    /// The source region this channel connects from.
    pub source_region: EntityId,
    /// The local entry place where arrivals appear.
    pub entry_place: EntityId,
    /// Transit delay in ticks (models travel time from off-map).
    pub transit_ticks: u32,
    /// Maximum cargo capacity per transit.
    pub capacity_units: u32,
    /// Current channel state.
    pub channel_state: ChannelState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ChannelState {
    Open,
    /// Reduced throughput.
    Degraded { capacity_fraction: Permille },
    /// Fully blocked.
    Blocked { since_tick: Tick },
}
```

### 3. Scheduled Inflow

```rust
/// A scheduled shipment expected through a boundary channel.
/// Component on the boundary channel's place entity.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledInflow {
    pub entries: Vec<InflowEntry>,
    next_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InflowEntry {
    pub id: InflowId,
    pub commodity: CommodityKind,
    pub quantity: u32,
    /// When the inflow is expected to arrive at the entry place.
    pub expected_arrival_tick: Tick,
    /// Who or what institution expects this shipment.
    pub expected_by: Option<EntityId>,
    pub state: InflowState,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct InflowId(pub u64);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum InflowState {
    /// In transit, on schedule.
    InTransit,
    /// Delayed but still coming.
    Delayed { new_expected_tick: Tick },
    /// Reduced quantity arriving.
    Reduced { actual_quantity: u32 },
    /// Canceled — will not arrive.
    Canceled { reason: DisruptionCause },
    /// Arrived successfully.
    Arrived { actual_tick: Tick },
}
```

### 4. Boundary Disruption Events

```rust
/// A disruption event that affects a source region or boundary channel.
/// Created by the boundary tick system based on scenario-defined disruption schedules
/// or by off-map event triggers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisruptionEvent {
    pub cause: DisruptionCause,
    pub target: DisruptionTarget,
    pub severity: Permille,
    pub start_tick: Tick,
    /// Duration in ticks. None = indefinite until recovery.
    pub duration_ticks: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DisruptionTarget {
    SourceRegion(EntityId),
    BoundaryChannel(EntityId),
}
```

### 5. Boundary Tick SystemFn

A new system function `tick_boundary_processes` that runs during the world maintenance phase:

1. **Check scheduled inflows**: For each `ScheduledInflow` with entries in `InTransit` state:
   - If `current_tick >= expected_arrival_tick`: materialize the arrival — spawn item lots at the entry place, transition state to `Arrived`.
   - If source region or channel is disrupted: transition to `Delayed`, `Reduced`, or `Canceled` based on severity.

2. **Apply disruption events**: For active `DisruptionEvent` entries:
   - Transition `SourceRegionState` to `Disrupted` or `Collapsed`.
   - Transition `ChannelState` to `Degraded` or `Blocked`.
   - Modify in-transit inflows accordingly.

3. **Recovery**: If a disruption's `duration_ticks` has elapsed:
   - Transition source/channel back to `Stable`/`Open`.
   - Schedule recovery inflows if applicable.

4. **Schedule next inflows**: For each `SourceRegion` in `Stable` state with active `BoundaryExport` entries:
   - If no in-transit inflow exists for this commodity, schedule the next one based on the channel's `transit_ticks` and export reliability.

### 6. Non-Arrival Detection

Local agents detect non-arrival through S59 expectations:
- When a scheduled inflow is created, the `expected_by` entity (e.g., a merchant, steward, or market office) gains an `ExpectationRecord` for the arrival.
- If the inflow is delayed or canceled, the expectation transitions to `Overdue`.
- The agent then responds through existing goal generation: investigate, report, substitute, ration.

No global non-arrival detector. The agent must be present at the expected delivery place and observe the absence.

### 7. Messenger/Report Arrival

For non-goods arrivals (messages, decrees, refugee reports):

```rust
/// A boundary message arriving from a source region.
/// Materialized as a social artifact or institutional claim at the entry place.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BoundaryMessage {
    /// Report of external event (war, famine, disaster).
    ExternalReport { content: String, urgency: Permille },
    /// Decree or order from external authority.
    ExternalDecree { content: String, issuer_region: EntityId },
    /// Refugee arrival announcement.
    RefugeeArrival { count: u16, origin_region: EntityId },
}
```

Messages materialize as records or social artifacts at the entry place. Local agents discover them through observation.

## FND-01 Section H — Causal Hooks Declaration

1. **Missing downstream consequence**: The world is causally closed. No goods, people, or information enter from outside, so the settlement cannot experience upstream supply disruption, refugee pressure, or external political events.

2. **New entities/relations/records**: `SourceRegion` (component on Place), `BoundaryChannel` (component on Place), `ScheduledInflow` (component on Place), `DisruptionEvent`, `BoundaryMessage`.

3. **Actions that mutate them**: `tick_boundary_processes` (system-driven: materializes arrivals, applies disruptions, schedules inflows). Agent actions interact indirectly — merchants buy arrived goods, stewards notice non-arrival.

4. **Information production and travel**: Arrivals are observable at the entry place. Non-arrival is detected through expectation violation (S59). External reports materialize as artifacts at the entry place and propagate through normal social channels.

5. **Conserved quantities**: Inflow goods are source/sink — they enter the simulation from outside (explicit source path through the boundary channel). Goods that enter are then fully conserved within the simulation. No goods appear without a declared source region and channel.

6. **Scarce capacities and contention**: Channel capacity limits throughput. Multiple inflows through the same channel compete for capacity. Entry place capacity limits how many goods/people can arrive simultaneously.

7. **Partial failures and aftermath**: Delayed inflow → goods arrive late, expectations violated, downstream behavior shifts. Reduced inflow → partial stockout, substitution pressure. Canceled → full non-arrival, shortage cascade. Channel blocked → all inflows through that channel halted.

8. **Positive feedback loops**: Shortage → theft → institutional response → more patrol → less trade labor → deeper shortage. Dampener: alternative channels, substitution, rationing (S64), demand reduction through population departure (S66).

9. **Physical dampeners**: Channel capacity limits, transit delay prevents instant recovery, source region recovery takes time, alternative routes may exist but are slower.

10. **Agent learning**: Merchants update supply reliability beliefs from repeated non-arrivals. Institutions may shift purchasing to alternative channels. Route preferences adapt to channel reliability.

11. **How agents can be wrong**: Expect arrival that was canceled off-screen — detection delayed until expectation deadline passes. Believe channel is blocked when it recovered — stale information. Attribute shortage to local cause when it's boundary-driven.

12. **Lifecycle states**: InflowEntry: InTransit → Arrived / Delayed / Reduced / Canceled. SourceRegionState: Stable → Disrupted → Collapsed → Stable. ChannelState: Open → Degraded → Blocked → Open.

13. **Temporal resolution**: Boundary tick runs once per world-tick during maintenance phase. Inflow arrival is deterministic based on scheduled ticks. Disruption timing is scenario-defined or event-driven.

14. **Boundary conditions**: This spec IS the boundary specification. Source regions are the outermost entities.

15. **Derived views**: None. All boundary state is authoritative.

16. **Causal records**: Inflow arrivals logged (commodity, quantity, channel, tick). Disruptions logged (cause, target, severity, tick). Non-arrivals logged through expectation violation events.

17. **Target patterns**: Off-map grain convoy delayed → local market continues under stale expectation → shortage emerges → ration/theft/substitution. Refugee party arrives with war report → patrol priorities change. External bridge collapse → reduced inflow until repaired.

18. **Save/load and replay**: All components are standard ECS. Disruption schedules are scenario-defined and deterministic. Inflow materialization is tick-based and deterministic.

## SystemFn Integration

`tick_boundary_processes` runs during Phase 1 (world maintenance), before perception. This ensures that arrivals materialized in this tick are observable in the perception pass.

Ordering: after resource regeneration, before perception sampling.

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `SourceRegion` | Place | Role-specific | `None` — only boundary places |
| `BoundaryChannel` | Place | Role-specific | `None` — only boundary edge places |
| `ScheduledInflow` | Place | Role-specific | `None` — only boundary places with active schedules |

All boundary components are scenario-defined. `SourceRegionDef`, `BoundaryChannelDef`, and `ScheduledInflowDef` wrapper types needed for scenario resolution (string names → EntityId).

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Expectations (S59) | Non-arrival triggers expectation overdue → search/report | State-mediated |
| Trade (E10) | Arrived goods enter local inventory → merchants buy/sell | State-mediated |
| Needs (E09) | Shortage of staple goods → hunger pressure escalation | State-mediated |
| Perception (E14) | Arrivals at entry place observable by co-located agents | State-mediated |
| Institutions (E16) | External decrees may affect local office behavior | State-mediated |
| Social artifacts (S45) | External reports materialize as artifacts at entry place | State-mediated |

## Profile-Driven Parameters

`BoundaryExport.reliability` is per-source-region per-commodity (scenario-configurable). Stable kingdoms have high reliability; war-torn regions have low.

`BoundaryChannel.transit_ticks` is per-channel (scenario-configurable). Nearby sources have short transit, distant ones long.

`DisruptionEvent.duration_ticks` is per-event. Storms are short, wars are long.

Disruption schedules are scenario-defined — the scenario author declares when disruptions occur, with what cause and severity. The simulation does not generate disruptions from dice rolls.
