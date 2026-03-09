# E18: Bandit Camp Dynamics

## Epic Summary
Implement bandit camps as facilities with members, supplies, morale, raid behavior, destruction consequences, survivor regrouping, and route danger updates.

## Phase
Phase 4: Group Adaptation, CLI & Verification

## Crate
`worldwake-systems`

## Dependencies
- E16 (faction system for bandit organization)

## Deliverables

### Bandit Camp Facility
- `BanditCamp` component on a facility entity:
  - `members: Vec<EntityId>` (via MemberOf relation to camp faction)
  - `supplies: EntityId` (container with camp supplies)
  - `morale: f32` (0.0 = broken, 1.0 = confident)
  - `preferred_raid_routes: Vec<(EntityId, EntityId)>` (travel edges to patrol)
- Camp is a Place with facility attributes

### Camp Destruction
- When camp facility destroyed (combat, fire, etc.):
  - Facility entity archived
  - Members become displaced (no camp affiliation)
  - Supplies scatter or are captured
  - Destruction event emitted

### Survivor Behavior
Per spec section 8 (no bandit respawn):
- Survivors do NOT despawn
- Each survivor independently:
  - Retains: injuries, morale, inventory, loyalties
  - Flees: moves to adjacent safe location
  - Surrenders: if morale critically low, may surrender to captors
  - Dies: if wounds are fatal
- No new bandits spawned to replace losses

### Regrouping
- Survivors with sufficient morale:
  - Seek each other (via knowledge of other members)
  - Travel to meeting point
  - If enough survivors gather: establish new camp
    - **EstablishCamp** action: find suitable location → create new facility
    - Duration: 100+ ticks
    - Requires: minimum member count, supplies, suitable location
- Regrouping uses normal travel and communication (no teleportation)

### Route Danger Updates
- `danger` values on travel edges reflect bandit presence:
  - Edges near active bandit camp: higher danger
  - Edges where bandits patrol: danger proportional to patrol frequency
  - After camp destruction: danger decreases on former patrol routes
  - After new camp established: danger increases on new patrol routes
- Danger values update based on actual bandit positions

### Raid Behavior (ActionDefs)
- **Raid**: bandits attack travelers on preferred routes
  - Precondition: bandit at route, traveler present, camp has raid plan
  - Effect: combat with travelers, loot if victorious
  - Visibility: witnesses present
- **Ambush**: stealth variant of raid
  - Lower visibility, higher success rate
  - Requires: suitable terrain (forest route)

## Invariants Enforced
- No bandit respawn (spec section 8)
- 9.10: No teleportation for regrouping
- 9.14: Dead bandits stay dead

## Tests
- [ ] T22: Bandit camp destruction chain:
  - Camp destroyed → survivors flee → some regroup → establish new camp
  - Route safety changes based on actual bandit locations
  - Merchants/travelers adapt to new danger map
- [ ] Survivors retain injuries, inventory, morale after camp destruction
- [ ] No respawn: destroyed camp doesn't regenerate members
- [ ] Regrouping requires physical travel to meeting point
- [ ] Danger values decrease on former patrol routes after destruction
- [ ] Danger values increase near new camp location
- [ ] Dead bandits do not participate in regrouping

## Acceptance Criteria
- Bandit camps as real facilities with real members
- Destruction has persistent consequences
- Survivors behave autonomously (flee, regroup, or die)
- Route danger reflects actual bandit positions
- No respawn, no despawn

## Spec References
- Section 1 (exemplar scenario 3: bandit camp destruction)
- Section 4.5 (bandit camp survival/migration)
- Section 7.1 (material propagation: damaged facilities)
- Section 8 (no bandit respawn)
- Section 9.10 (no teleportation)
