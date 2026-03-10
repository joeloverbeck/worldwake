# E10: Production, Transport & Route Occupancy

**Status**: COMPLETED

## Epic Summary
Implement production from concrete resource sources, work-in-progress jobs, carry-capacity transport, and physical travel with explicit in-transit occupancy. Harvesting must transfer goods out of a real source stock; it may not conjure infinite goods from a place tag.

## Phase
Phase 2: Survival & Logistics

## Crate
`worldwake-systems`

## Dependencies
- E08 (scheduler drives long-running actions and travel time)

## Foundations Alignment Changes
This revision fixes four major alignment failures:

1. **Harvesting can no longer create goods ex nihilo.** Farms, orchards, forests, and similar sites need concrete source stock or yield buffers.
2. **Facility “slot counts” are replaced by concrete workstations.** Concurrency comes from reservable entities, not an abstract capacity integer.
3. **Interrupted work cannot silently destroy progress or inputs.** Work-in-progress must persist as concrete state.
4. **Travel uses explicit in-transit occupancy.** An entity traveling for ten ticks is physically on the route for ten ticks, not frozen at the source and then teleported at arrival.

## Deliverables

### RecipeDefinition Data Struct
Data-driven production definitions analogous to `ActionDef`, but grounded in explicit material accounting.

- `inputs: Vec<(CommodityKind, Quantity)>`
- `outputs: Vec<(CommodityKind, Quantity)>`
- `work_ticks: NonZeroU32`
- `required_workstation_tag: Option<WorkstationTag>`
- `required_tool_kinds: Vec<UniqueItemKind>` — possessed unique tools, not consumed
- `body_cost_per_tick: BodyCostPerTick`

There is no hidden loss and no hidden creation. If a recipe should produce scrap, ash, chaff, or other leftovers, they must appear in `outputs`.

### RecipeRegistry
Registry of all available recipes.

- `register(recipe_id, RecipeDefinition)`
- `get(recipe_id) -> Option<&RecipeDefinition>`
- `recipes_for_workstation(tag: WorkstationTag) -> Vec<RecipeId>`

### KnownRecipes Component
Per-agent concrete production knowledge / capability.

Possible representations:
- `KnownRecipes(Vec<RecipeId>)`
- or a capability/tag-based equivalent if recipe families are grouped

This is required so “anyone at a forge can craft anything” does not flatten agent diversity and collapse role differentiation.

### Workstation Entities
Reservable concrete work sites that enable concurrency.

Examples:
- orchard row
- field plot
- forge hearth
- chopping block
- millstone
- wash basin
- latrine stall

A place may contain multiple workstations. “Open production slots” are a derived read-model equal to the number of unreserved matching workstations.

### ResourceSource / YieldBuffer
Concrete, depletable production stock attached to a place or workstation.

```rust
struct ResourceSource {
    commodity: CommodityKind,
    available_quantity: Quantity,
    max_quantity: Quantity,
    regeneration_ticks_per_unit: Option<NonZeroU32>,
}
```

Examples:
- orchard trees accumulate apples into a harvestable buffer
- farm plots accumulate grain into a harvestable buffer
- forest stands hold available firewood / standing timber

Harvest transfers material out of `available_quantity`. If the source is empty, harvest fails.

### ProductionJob / WorkInProgress
Concrete persistent state for in-flight production.

- `recipe_id: RecipeId`
- `worker: EntityId`
- `workstation: EntityId`
- `staged_inputs_container: EntityId`
- `progress_ticks: u32`

On job start:
- reserve workstation
- move required inputs into staged container / WIP state
- begin accumulating `progress_ticks`

On interruption:
- the job persists unless explicitly abandoned
- staged inputs remain staged
- partial work does not disappear

This removes “partial progress lost” hand-waving and replaces it with traceable state.

### Production Actions

#### Harvest
- Precondition:
  - actor knows the recipe / capability
  - actor is co-located with a matching workstation or resource source
  - source has sufficient `available_quantity`
  - required tool(s) are possessed as concrete unique items
- Effect:
  - reduce `ResourceSource.available_quantity`
  - create / increase output lot at location or in staged container
- Duration: `RecipeDefinition.work_ticks`
- Labor reservation: reserve concrete workstation if required

#### Craft
- Precondition:
  - actor knows recipe
  - actor is co-located with matching workstation
  - required inputs accessible
  - required tools possessed as concrete unique items
- Effect:
  - move inputs into staged WIP container
  - after work completes, create outputs defined by recipe
- Duration: `RecipeDefinition.work_ticks`

### CarryCapacity Component
- `CarryCapacity(LoadUnits)` — uses existing `LoadUnits` infrastructure
- Pick-up / put-down remain physical actions
- Current capacity is derived from carried load, not duplicated in a shadow score
- Goods move only because the carrier moves

### TransitOccupancy / InTransitOnEdge
Travel must be represented as concrete occupancy on a route edge.

```rust
struct InTransitOnEdge {
    edge_id: EntityId,
    origin: EntityId,
    destination: EntityId,
    departure_tick: u64,
    arrival_tick: u64,
}
```

On travel start:
- actor leaves origin place
- actor gains `InTransitOnEdge`
- carried items remain parented to actor and are therefore also in transit through containment

On travel completion:
- remove `InTransitOnEdge`
- set actor `LocatedIn = destination`

This satisfies the route-presence requirement for later ambush, escort, witness, and interception logic without introducing abstract danger scores.

### Route-Based Transport
- Goods move only through:
  - carried containment under a moving agent / vehicle
  - later explicit vehicle/container systems
- No teleportation
- No “merchant restock” side channel
- Caravans and carriers are just agents moving carried loads through the place graph

### Carrier Behavior (ActionDefs only; AI in E13)
- Pick up goods
- Travel while occupying the edge
- Deliver goods
- Repeat for multi-stop routes

No contracts are required in Phase 2.

## Component Registration
New components to register in `component_schema.rs`:

- `CarryCapacity(LoadUnits)` — on `EntityKind::Agent`
- `KnownRecipes` — on `EntityKind::Agent`
- `InTransitOnEdge` — on any traveling entity that leaves a place for an edge
- `ResourceSource` — on resource-bearing place / workstation entities
- `WorkstationTag` / workstation marker — on reservable workstation entities
- `ProductionJob` / WIP state — on job entity or persistent action state container

New registries:
- `RecipeRegistry` — stored in `SimulationState`

## SystemFn Integration
E10 owns the production and transport state changes required by this epic.

Reads:
- `RecipeRegistry`
- `KnownRecipes`
- workstation reservation state
- `ResourceSource`
- inventories / containment
- travel edges

Writes:
- `ProductionJob` / WIP state
- `ResourceSource.available_quantity`
- item lot creation / transfer
- `InTransitOnEdge`
- arrival / departure events

## Cross-System Interactions (Principle 12)
- **E10 → E09**: recipes and travel expose `BodyCostPerTick`; physiology reads active action state
- **E10 → E11**: produced goods and route occupancy make physical procurement / restock possible
- **E10 → E12**: route occupancy creates future combat encounter space without storing route danger
- **E10 → E13**: AI reads known recipes, workstations, source stocks, cargo, and transit state through `BeliefView`

## FND-01 Section H

### Information-Path Analysis
- Production knowledge comes from the worker’s `KnownRecipes`
- Workstation availability is local: the actor must be co-located to observe and reserve it
- Resource availability is local to the source entity
- Transit occupancy is concrete: travelers on the same edge at overlapping times are physically co-present there

### Positive-Feedback Analysis
- **Production success → more inventory → more production opportunity**
- **Transport success → wider trade radius → more procurement opportunity**

### Concrete Dampeners
- **Production loop dampeners**:
  - finite source stock
  - finite workstation count
  - body costs (fatigue / thirst)
  - concrete tool requirements via possessed unique items
- **Transport loop dampeners**:
  - route travel time
  - carry capacity
  - co-presence risk on the route once combat / bandit systems exist

### Stored State vs. Derived Read-Model
**Stored (authoritative)**:
- `RecipeDefinition`
- `KnownRecipes`
- `ResourceSource`
- workstation entities / reservations
- `ProductionJob` / WIP state
- `CarryCapacity`
- `InTransitOnEdge`
- item lots

**Derived (transient read-model)**:
- available workstations at a place
- can the actor carry more
- which recipes the actor can perform right now
- which travelers can currently encounter each other on an edge

## Invariants Enforced
- 9.5: Conservation — all production outputs come from explicit recipe accounting or explicit source stock
- 9.10: No teleportation — goods move only through containment and travel
- Principle 3: no abstract facility slot count
- Principle 7: route presence is concrete occupancy, not a score

## Outcome

- Completion date: 2026-03-10
- What actually changed:
  - implemented the shared E10 schema in `worldwake-core` and `worldwake-sim`, including `KnownRecipes`, `ResourceSource`, `ProductionJob`, `InTransitOnEdge`, `CarryCapacity`, and `RecipeRegistry`
  - implemented `harvest`, `craft`, `pick_up`, `put_down`, and generic adjacent-place `travel` actions in `worldwake-systems`
  - implemented resource regeneration, workstation reservation behavior, explicit WIP staging, carried-load enforcement, and route occupancy on the traveling actor
  - added focused action-level tests plus scheduler-driven integration coverage for multi-step transport, depletion/regeneration gating, and craft accounting
  - refined material accounting so the codebase now distinguishes live-lot conservation from authoritative commodity accounting that includes `ResourceSource` stock
- Deviations from original plan:
  - “deliver” is not a standalone action; transport remains the cleaner `pick_up -> travel -> put_down` composition
  - carried goods do not each receive their own `InTransitOnEdge`; the actor carries route occupancy while possessions/contained items follow through placement and containment
  - the conservation surface was clarified after implementation into separate lot-only and authoritative helpers rather than one ambiguous helper
- Verification results:
  - `cargo test -p worldwake-core conservation` passed on 2026-03-10
  - `cargo test -p worldwake-systems` passed on 2026-03-10
  - `cargo clippy --workspace --all-targets -- -D warnings` passed on 2026-03-10
  - `cargo test --workspace` passed on 2026-03-10

## Tests
- [ ] Harvest reduces `ResourceSource.available_quantity`
- [ ] Harvest fails when the source is empty
- [ ] Resource sources regenerate only through their explicit regeneration rule
- [ ] Craft stages inputs into WIP and produces only recipe-defined outputs
- [ ] Interrupted craft leaves WIP / staged inputs in the world
- [ ] Workstation concurrency is enforced by reservation of concrete workstation entities
- [ ] Known recipe gating works; agents cannot perform recipes they do not know
- [ ] Carry capacity is enforced via `LoadUnits`
- [ ] Travel creates `InTransitOnEdge` for the full route duration
- [ ] Arrival removes `InTransitOnEdge` and updates `LocatedIn`
- [ ] Carried items remain with the carrier during transit
- [ ] No production path creates infinite goods from a tag alone
- [ ] No teleportation path moves goods without a carrier or explicit container chain

## Acceptance Criteria
- Harvesting transfers material out of a concrete source stock
- Concurrency is derived from workstations, not abstract slot counts
- Interrupted work remains traceable through WIP state
- Transport uses explicit in-transit occupancy
- Produced goods, carried goods, and travelers all move physically through the graph
- All work and travel expose explicit body costs where applicable

## Spec References
- Section 4.5 (production / harvest, movement / travel)
- Section 7.1 (material propagation)
- Section 8 (no magical merchant restock)
- Section 9.5 (conservation)
- Section 9.10 (no teleportation)
- `docs/FOUNDATIONS.md` Principles 2, 3, 6, 7, 11, 12
