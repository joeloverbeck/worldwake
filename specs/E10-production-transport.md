# E10: Production & Transport

## Epic Summary
Implement production actions (harvest, craft), cargo carrying, and route-based physical transport of goods.

## Phase
Phase 2: Survival & Logistics

## Crate
`worldwake-systems`

## Dependencies
- E08 (scheduler drives production ticks)

## Deliverables

### Production Actions

- **Harvest**: agent at farm/orchard → produce goods over time
  - Precondition: agent at production place, has tools (optional efficiency bonus)
  - Effect: create new ItemLot of produced good type at location
  - Duration: 30-60 ticks per batch
  - Labor requirement: 1 agent per production slot
  - Produces: apples (orchard), grain (farm), firewood (forest)

- **Craft**: agent has input goods → transform into output goods
  - Precondition: agent possesses required inputs, at appropriate facility
  - Effect: consume input lots, create output lot
  - Recipes: grain → bread, wood + tools → simple tools, etc.
  - Duration: 20-40 ticks per craft
  - Conservation: input quantity consumed = output quantity produced (by weight/value mapping)

### Production Duration & Labor
- Production takes real ticks (no instant creation)
- Each production facility has limited slots (capacity)
- Labor reservation: agent must reserve production slot for duration
- Interrupted production: partial progress lost or produces partial output

### Cargo Carrying
- Agents have a carry capacity (weight/quantity limit)
- Pick up action: agent takes item from ground/container → carried by agent
- Put down action: agent places item at current location
- Carried items travel with agent (update location as agent moves)
- Carry capacity enforced: cannot pick up beyond limit

### Route-Based Transport
- Travel action: agent moves from place A to place B along travel edge
  - Precondition: travel edge exists from current place to destination
  - Duration: `travel_edge.travel_time_ticks`
  - Effect: agent's LocatedIn changes to destination
  - Carried items move with agent
- Goods move only through physical transport (no teleportation)
- Carrier/caravan actors: agents specialized in transport between places

### Carrier Behavior (ActionDefs only, AI in E13)
- Transport contract: deliver goods from source to destination
- Pick up goods → travel → deliver goods
- Multiple stops possible

## Invariants Enforced
- 9.5: Conservation through production/transport (inputs consumed = outputs produced)
- 9.10: No teleportation - goods move only through physical travel along valid routes

## Tests
- [ ] Harvest creates new goods at production location
- [ ] Craft consumes inputs and creates outputs with conservation
- [ ] Goods only appear via production (no magical creation)
- [ ] No teleportation: goods move only via agent carrying along edges
- [ ] Carry capacity enforced
- [ ] Travel duration matches edge travel_time_ticks
- [ ] Agent location updates correctly after travel
- [ ] Carried items' location tracks agent's location
- [ ] Production facility slots limit concurrent producers
- [ ] Interrupted production handles cleanup

## Acceptance Criteria
- Production creates goods through defined recipes
- Transport is physical: agent carries goods along graph edges
- Conservation maintained through all production and transport
- No instant creation or teleportation of goods

## Spec References
- Section 4.5 (production/harvest, movement/travel)
- Section 7.1 (material propagation channel)
- Section 8 (no magical merchant restock)
- Section 9.5 (conservation)
- Section 9.10 (no teleportation)
