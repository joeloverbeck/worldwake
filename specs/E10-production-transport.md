# E10: Production & Transport

## Epic Summary
Implement production actions (harvest, craft) via data-driven recipes, cargo carrying using `LoadUnits`, and route-based physical transport of goods.

## Phase
Phase 2: Survival & Logistics

## Crate
`worldwake-systems`

## Dependencies
- E08 (scheduler drives production ticks)

## Deliverables

### RecipeDefinition Data Struct
Data-driven recipe definitions (analogous to `ActionDef` in the action framework):
- `inputs: Vec<(CommodityKind, Quantity)>` — required input goods
- `outputs: Vec<(CommodityKind, Quantity)>` — produced output goods
- `base_duration_ticks: NonZeroU32` — base production time (never zero, per FND-01)
- `required_facility_tag: Option<PlaceTag>` — facility type required (e.g., Farm, Forge, Orchard)
- `tool_requirement: Option<CommodityKind>` — optional tool that must be possessed (not consumed)

No hardcoded production durations — all durations come from `RecipeDefinition.base_duration_ticks`.

### RecipeRegistry
Registry of all available recipes (like `ActionDefRegistry`):
- `register(recipe_id, RecipeDefinition)`
- `get(recipe_id) -> Option<&RecipeDefinition>`
- `recipes_at(place_tag) -> Vec<RecipeId>` — which recipes are available at a facility type
- Loaded at initialization with prototype recipes

### Production Actions

- **Harvest**: agent at farm/orchard → produce goods over time
  - Precondition: agent at production place with matching `required_facility_tag`, has required tool (if any)
  - Effect: create new ItemLot of produced good type at location
  - Duration: `RecipeDefinition.base_duration_ticks` for the selected recipe
  - Labor requirement: 1 agent per production slot
  - Produces: apples (orchard), grain (farm), firewood (forest) — defined by recipes

- **Craft**: agent has input goods → transform into output goods
  - Precondition: agent possesses required inputs (per recipe), at appropriate facility
  - Effect: consume input lots, create output lot
  - Duration: `RecipeDefinition.base_duration_ticks` for the selected recipe
  - Conservation: input quantity consumed = output quantity produced (by weight via `LoadUnits` mapping)

### Production Duration & Labor
- Production takes real ticks (no instant creation)
- Each production facility has limited slots (capacity)
- Labor reservation: agent must reserve production slot for duration
- Interrupted production: partial progress lost or produces partial output

### CarryCapacity Component
- `CarryCapacity(LoadUnits)` — uses existing `LoadUnits` from `crates/worldwake-core/src/numerics.rs`
- Integrates with existing load accounting in `crates/worldwake-core/src/load.rs`
- Pick up action: agent takes item from ground/container → carried by agent
- Put down action: agent places item at current location
- Carried items travel with agent (update location as agent moves)
- Carry capacity enforced via `load.rs` accounting: cannot pick up beyond limit

### Route-Based Transport
- Travel action: agent moves from place A to place B along travel edge
  - Precondition: travel edge exists from current place to destination
  - Duration: `travel_edge.travel_time_ticks`
  - Effect: agent's LocatedIn changes to destination
  - Carried items move with agent
- Goods move only through physical transport (no teleportation)
- Carrier/caravan actors: agents specialized in transport between places

### Carrier Behavior (ActionDefs only, AI in E13)
- Pick up goods → travel → deliver goods
- Multiple stops possible
- No "transport contracts" (contracts deferred — not in Phase 2 scope)

## Component Registration
New components to register in `component_schema.rs`:
- `CarryCapacity(LoadUnits)` — on `EntityKind::Agent`

New registries:
- `RecipeRegistry` — stored in `SimulationState` (like `ActionDefRegistry`)

## SystemFn Integration
- Implements the `SystemId::Production` handler registered in `SystemDispatch`
- Runs once per tick for all active production actions
- Reads: `RecipeRegistry`, facility slot availability, agent inventory
- Writes: ItemLot creation/consumption, production progress

## Cross-System Interactions (Principle 12)
- **E10 → E09**: Production actions consume agent time/ticks, increasing fatigue (E09 reads activity state)
- **E10 → E11**: Produced goods become available for trade at the production location
- **E10 → E13**: Decision architecture queries available recipes/facilities via `BeliefView` to plan production goals

## FND-01 Section H

### Information-Path Analysis
- Recipe knowledge: agents know which recipes exist (global knowledge for prototype; in E14, this could be belief-gated)
- Facility availability: agent must be co-located with facility to use it (Principle 7)
- Production slot availability: observable only by agents at the facility location
- Goods availability: agents observe goods at their current location

### Positive-Feedback Analysis
- **Production → surplus → no demand → abandoned production**: overproduction could lead to wasted effort and agent starvation (time spent producing unsellable goods instead of meeting needs)
- No amplifying loops identified in the production system itself — production is a linear input→output transformation

### Concrete Dampeners
- **Overproduction**: agents' homeostatic needs (E09) compete with production time. An agent who spends too long crafting will get hungry/tired and must stop to address needs. Biological needs are the physical dampener against infinite production.
- **Resource exhaustion**: input goods are finite and must be sourced (harvested or traded). Production rate is limited by input availability — a physical constraint, not a numerical cap.

### Stored State vs. Derived Read-Model
**Stored (authoritative)**:
- `RecipeDefinition` data (inputs, outputs, duration, requirements)
- `CarryCapacity(LoadUnits)` component
- ItemLot entities (produced goods)
- Production slot reservations

**Derived (transient read-model)**:
- Available production slots at a facility (count of unreserved slots)
- Whether an agent can carry more (current load vs. capacity — computed via `load.rs`)
- Which recipes an agent can perform (intersection of possessed inputs, co-located facility, tool ownership)

## Invariants Enforced
- 9.5: Conservation through production/transport (inputs consumed = outputs produced)
- 9.10: No teleportation — goods move only through physical travel along valid routes

## Tests
- [ ] Harvest creates new goods at production location using recipe definition
- [ ] Craft consumes inputs and creates outputs with conservation
- [ ] Goods only appear via production (no magical creation)
- [ ] No teleportation: goods move only via agent carrying along edges
- [ ] Carry capacity enforced via `LoadUnits` accounting
- [ ] Travel duration matches edge `travel_time_ticks`
- [ ] Agent location updates correctly after travel
- [ ] Carried items' location tracks agent's location
- [ ] Production facility slots limit concurrent producers
- [ ] Interrupted production handles cleanup
- [ ] Production duration comes from `RecipeDefinition.base_duration_ticks`, not hardcoded
- [ ] `RecipeRegistry` correctly indexes recipes by facility tag

## Acceptance Criteria
- Production creates goods through data-driven `RecipeDefinition` entries
- Transport is physical: agent carries goods along graph edges
- Conservation maintained through all production and transport
- No instant creation or teleportation of goods
- All durations from `RecipeDefinition`, not hardcoded constants
- Carry capacity uses existing `LoadUnits` infrastructure

## FND-01 Section D — Route Presence Gate

**GATE**: Any route-based encounter, risk assessment, interception, or witness-along-route logic in this epic MUST NOT proceed until a concrete route presence model exists in the codebase. This model must support:

- Determining which entities are physically on a route or route segment
- Determining which travelers can physically encounter each other
- Determining which agents can witness route events locally

It is **forbidden** to introduce stored route danger or visibility scores to compensate for missing route presence. All route risk/danger must be derived from concrete entity presence, never from stored abstract scores (Principle 3, `docs/FOUNDATIONS.md`).

See `specs/FND-01-phase1-foundations-alignment.md` Section D for full context.

## Spec References
- Section 4.5 (production/harvest, movement/travel)
- Section 7.1 (material propagation channel)
- Section 8 (no magical merchant restock)
- Section 9.5 (conservation)
- Section 9.10 (no teleportation)
- `docs/FOUNDATIONS.md` Principles 2, 3, 6, 11, 12
- `specs/FND-01-phase1-foundations-alignment.md` Section D (route presence gate)
