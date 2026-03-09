# Prototype Specification: Causality-First Emergent Micro-World

## 1. Objective
Build a small self-running world in which the human-controlled agent is only an agent with a different input source, and where material, social, and informational consequences propagate through explicit world state rather than scripts.

The prototype must demonstrate four unscripted causal chains:
1. Emptying a shop's apple stock creates real shortage, replanning, and resupply pressure.
2. Killing the ruler creates a real vacancy, contested succession, and changed public order.
3. Destroying a bandit camp causes survivors to flee, regroup, and alter route safety elsewhere.
4. Companion bodily needs create physical, social, and logistical consequences.

## 2. What Stays From the Report
- Causality and accounting matter more than cinematic content.
- The world must continue without the human.
- Agents need layered decision-making.
- Persistent simulation and replayability are mandatory.
- Prototype scope must stay small.

## 3. What Needs to Change

### 3.1 Shift from architecture-first to law-first
The report is still too high-level. The prototype needs explicit laws for:
- location
- ownership
- possession
- reservation
- production
- consumption
- injury/death
- office succession
- rumor/witness propagation

### 3.2 Remove the concept of "player" from simulation
There is no `Player` type in simulation code.
There is only:
- `Agent`
- `ControlSource = Human | AI | None`

Any agent can become the human-controlled agent at runtime.

### 3.3 Use a graph-world, not a full continuous world
For prototype v0, world space is a place graph with travel times.
This is not a compromise; it is the only sane way to prove causality before rendering complexity.

### 3.4 Simulate carriers of consequence, not generic realism
Only model things that can propagate downstream effects:
- goods
- facilities
- wounds
- waste
- offices
- loyalties
- debts/contracts
- witness knowledge
- rumors
- route danger

Do not model molecules, full weather, or broad crafting trees yet.

### 3.5 Separate world truth from agent belief
Agents plan from believed facts, not omniscient world state.
Beliefs come from:
- direct perception
- memory
- reports
- rumors
- ledgers/records

### 3.6 Replace per-item simulation with hybrid identity
Use:
- unique entities for people, weapons, offices, contracts, named artifacts
- stackable lots for apples, grain, wood, coin, waste

Lots must support split/merge with provenance.

### 3.7 Add explicit action semantics
Every action must define:
- actor constraints
- targets
- preconditions
- reservation requirements
- duration
- interruptibility
- commit conditions
- effects
- witnesses/visibility
- causal event emission

### 3.8 Defer multi-LOD world simulation
Prototype v0 uses one authoritative simulation for the whole micro-world.
Do not implement separate offline/online rules yet.
LOD can come after invariants are stable.

### 3.9 Add social institutions as first-class systems
Politics will not emerge from "reputation" alone.
The world needs:
- offices
- eligibility rules
- succession law
- support/loyalty relations
- coercion/bribery
- public-order consequences

### 3.10 Measure emergence directly
Success is not "the world looks busy."
Success is:
- causal chain depth
- cross-system propagation
- deterministic replay
- zero invariant violations
- no hidden scripts for the exemplar scenarios

## 4. Prototype Scope

### 4.1 World
- 1 village core
- 1 farm/orchard
- 1 general store
- 1 inn or communal house
- 1 ruler's hall
- 1 barracks/guard post
- 1 latrine or toilet facility
- 1 crossroads
- 1 forest route
- 1 bandit camp
- 12-20 place nodes total

### 4.2 Population
- 25-40 NPCs
- 1 human-controlled agent slot
- 0-2 companions
- 1 ruler
- 2-3 succession candidates
- 1 merchant
- 1-3 carriers/caravan actors
- 4-8 guards
- 4-8 bandits
- farmers/laborers/locals fill the rest

### 4.3 Goods
Minimum commodity set:
- apples
- grain
- bread
- water
- firewood
- medicine
- simple tools
- weapons
- coin
- waste

### 4.4 Needs
Each agent tracks at minimum:
- hunger
- thirst
- fatigue
- bladder/bowels
- hygiene
- pain
- fear
- social standing
- loyalty/commitment
- wealth pressure

### 4.5 Core Systems
- time and scheduler
- movement/travel
- containers and inventory transfer
- production/harvest
- trade and pricing
- consumption/metabolism
- rest/sleep
- toilet/hygiene/waste
- combat/injury/death
- healing
- crime and theft
- witness/rumor propagation
- office succession
- faction loyalty/support
- bandit camp survival/migration
- deterministic event log + replay

## 5. Authoritative Simulation Model

### 5.1 Time
- Fixed tick simulation.
- Suggested base tick: 1 minute.
- Actions consume integer ticks.
- No direct state mutation outside ticked systems.

### 5.2 Topology
The world is a directed graph of places and travel edges.
Each edge has:
- travel time
- capacity
- danger
- visibility

### 5.3 Entity Classes
- `Agent`
- `ItemLot`
- `UniqueItem`
- `Container`
- `Facility`
- `Place`
- `Faction`
- `Office`
- `Contract`
- `Rumor`
- `EventRecord`

### 5.4 Required Relations
Every relevant entity must support some subset of:
- `located_in`
- `contained_by`
- `possessed_by`
- `owned_by`
- `reserved_by`
- `member_of`
- `loyal_to`
- `holds_office`
- `hostile_to`
- `knows_fact`
- `believes_fact`

### 5.5 Ownership Semantics
These are different and must never be conflated:
- `location`: where the thing physically is
- `containment`: which container or carrier currently holds it
- `possession`: who has immediate control over it
- `ownership`: who is legally/socially recognized as owning it
- `reservation`: who has temporary exclusive claim to use it next

### 5.6 Event Model
Every persistent change emits an append-only event with:
- event id
- tick
- cause id
- actor id
- target ids
- place id
- state deltas
- visibility/witness data
- tags

State snapshots may be stored for performance, but the event stream remains the causal source of truth.

## 6. Decision Architecture

### 6.1 Rule
Do not build three competing AI brains.
Use one hierarchy:
1. homeostatic and social pressures generate scores
2. utility selects the current goal
3. planner builds a sequence of legal actions
4. reactive executor handles interrupts, danger, and animation-level detail

### 6.2 Goal Examples
- eat
- drink
- sleep
- relieve self
- wash
- trade
- restock shop
- escort cargo
- raid caravan
- flee danger
- claim office
- support claimant
- heal
- bury or move corpse
- establish new camp

### 6.3 Planning Rules
- planners operate on a compact abstract state, not the full simulation
- plans must be revalidated before each step commits
- broken preconditions trigger replanning
- the planner may only use believed facts, not global truth

### 6.4 Human Control
The human-controlled agent uses the exact same action query and execution pipeline as NPCs.
The UI only exposes actions whose preconditions are currently satisfiable from the agent's perceived context.

## 7. Propagation Channels
The prototype must support consequence propagation through at least five channels:

### 7.1 Material
goods, wounds, corpses, waste, damaged facilities

### 7.2 Economic
stock levels, scarcity, prices, debt, insolvency, labor availability

### 7.3 Informational
witnessing, rumor spread, record keeping, suspicion, discovery delays

### 7.4 Institutional
office vacancy, legitimacy, loyalty shifts, law enforcement, patrol intensity

### 7.5 Physiological/Social
hunger, fatigue, bladder, hygiene, shame, disgust, fear, morale

A "domino effect" only counts if it travels through explicit channels like these.

## 8. Hard Exclusions for Prototype v0
- no quest scripts resolving the exemplar scenarios
- no magical merchant restock
- no bandit respawn
- no leader replacement cutscene
- no special-case player discounts, protections, or permissions
- no global omniscience for NPCs
- no direct simulation mutations from UI code
- no second offline ruleset

## 9. Invariants That Must Always Hold

### 9.1 Simulation Authority
All persistent world state is owned by the simulation layer.
Render, UI, and dialogue can only read or request actions.

### 9.2 Determinism
Given identical initial state, RNG seed, and input events, the simulation must produce the same event sequence and final state hash.

### 9.3 Causal Completeness
Every persistent state mutation has exactly one direct cause event or system tick cause.
No orphan mutations.

### 9.4 Unique Physical Placement
Every physical entity or lot exists in exactly one place at a time:
- at a place
- in a container
- on an agent in transit

Never two at once. Never nowhere.

### 9.5 Conservation of Conserved Resources
For conserved goods, quantity changes only by:
- production
- consumption
- spoilage
- transformation
- destruction
- explicit spawn/sink defined by world rules

No silent creation or deletion.

### 9.6 No Negative Stocks
No container, wallet, need meter, or faction support pool may go below zero unless the variable explicitly supports signed debt or deficit.

### 9.7 Ownership/Possession Consistency
An item cannot be sold, gifted, or confiscated unless the action has a valid chain of possession/control and updates ownership rules correctly.

### 9.8 Reservation Exclusivity
A unique object or single-use facility slot cannot be concurrently reserved by multiple agents for the same timespan.

### 9.9 Legal Action Execution
No action may commit unless its commit conditions are true at commit time.
Interrupted or invalidated actions must abort or replan cleanly.

### 9.10 No Teleportation
Material transfer, travel, camp relocation, and succession support shifts must consume time and use valid routes or communication channels.

### 9.11 World/Belief Separation
Agents may react only to facts they perceived, inferred, remembered, or were told.
Global state may not directly leak into plans.

### 9.12 Player Symmetry
Simulation code may not branch on "is_player."
The only valid distinction is control source at input capture and presentation.

### 9.13 Office Uniqueness
Each office has at most one holder at a time.
Vacancy and transfer must be explicit events.

### 9.14 Death Finality
Dead agents do not plan, act, trade, vote, or consume.
Their bodies and possessions persist until acted upon by world rules.

### 9.15 Off-Camera Continuity
Camera movement or loss of visibility cannot create, erase, heal, restock, or relocate entities.

### 9.16 Need Continuity
Needs change only through time passage, consumption, rest, toileting, washing, injury, healing, or defined world effects.
They do not silently reset.

### 9.17 Traceable Discovery
Crimes, deaths, and shortages become socially real through discovery, witnesses, audits, or rumor propagation—not instant universal knowledge.

### 9.18 No Circular Containment
Containment graphs must remain acyclic.

### 9.19 Save/Load Integrity
Saving and loading must preserve all authoritative state, event provenance, and pending action timers.

### 9.20 Scriptlessness of Core Scenarios
The exemplar domino effects must arise from authored initial conditions plus general rules, not scenario-specific triggers after simulation start.

### 9.21 Controlled-Agent Mortality
Death or incapacitation of the currently human-controlled agent may change the control binding or presentation state, but it must not stop or rewind the world simulation.

## 10. Tests That Must Pass

### 10.1 Unit and Property Tests

#### T01_UniqueLocation
Randomized transfer sequences never produce a physical entity with multiple simultaneous locations.

#### T02_Conservation
For every conserved good, initial quantity + produced - consumed - spoiled - destroyed = current quantity across all containers and agents.

#### T03_NoNegativeInventory
Randomized trade, theft, and consumption never produce negative stock.

#### T04_ReservationLock
Two agents attempting to reserve the same bed, cart, toilet stall, or unique item cannot both succeed for the same window.

#### T05_ActionPreconditionGate
The affordance query never exposes an action whose start preconditions are false in the acting agent's perceived context.

#### T06_CommitValidation
An action whose target becomes unavailable before commit aborts cleanly and emits a replan reason.

#### T07_EventProvenance
Every persistent state delta in a tick is traceable to an event id and cause chain.

#### T08_ReplayDeterminism
Same initial state + same seed + same input log => identical event log hash and final state hash.

#### T09_SaveLoadRoundTrip
Save at arbitrary tick, load, continue with identical inputs => identical outcome to uninterrupted run.

#### T10_BeliefIsolation
An agent does not react to an unseen theft, unseen death, or unseen camp migration until information reaches them through a defined channel.

#### T11_OfficeUniqueness
Succession logic cannot produce two simultaneous rulers.

#### T12_NoPlayerBranching
A simulation test suite can attach human control to merchant, guard, bandit, claimant, or farmer without changing simulation rules or available action semantics.

#### T13_ContainmentAcyclic
Randomized container nesting never produces containment cycles.

#### T14_DeadAgentsInactive
Once dead, an agent generates no new plans or actions.

#### T15_NeedProgression
Without intervention, hunger/thirst/fatigue/bladder values evolve according to metabolism and time, not frame rate or camera position.

### 10.2 Scenario Integration Tests

#### T20_AppleStockoutChain
Setup:
- merchant has apples in shop
- farm/orchard can produce more
- carrier or merchant can restock
- bandits threaten at least one route

Action:
- any agent buys all apples

Expected:
- shop apple stock immediately reaches zero
- merchant cannot sell non-existent apples
- merchant or carrier generates a restock plan if economically rational
- at least one non-merchant consumer changes plan because of shortage
- scarcity affects either apple price, substitute demand, or both
- any restock occurs through physical movement of goods
- if the route is disrupted, shortage persists accordingly
- all downstream state changes are causally linked back to the initial purchase through the event graph

Pass threshold:
- within 2 in-world days, the event graph from the initial purchase reaches at least 3 subsystems: economy, logistics, and agent needs/behavior

#### T21_RulerDeathSuccessionChain
Setup:
- one ruler office
- at least two valid claimants
- guards with loyalties
- public-order metric or analogous state

Action:
- any agent kills the ruler

Expected:
- office becomes vacant immediately on death
- claimants begin support-seeking, coercion, or claim actions
- guards or elites may change loyalty based on beliefs and incentives
- patrol/public order changes during vacancy or after succession
- a successor can emerge without human intervention
- there is never more than one ruler at a time
- no cutscene or scripted fallback fills the role

Pass threshold:
- within 3 in-world days, the event graph from the killing reaches at least 3 subsystems: combat/death, politics, and security/economy

#### T22_BanditCampDestructionChain
Setup:
- bandit camp has members, supplies, morale, and preferred raid routes

Action:
- any group destroys or routs the camp

Expected:
- survivors flee, surrender, or die; they do not despawn
- surviving members retain injuries, morale, inventory, and loyalties
- the group may split, merge with another group, or establish a new camp
- route danger changes according to the group's actual location
- merchants, guards, or travelers adapt plans to the new danger map
- any renewed raids come from a real reconstituted group, not respawn logic

Pass threshold:
- within 5 in-world days, route safety and at least one downstream economic behavior must change because of the diaspora

#### T23_CompanionPhysiologyChain
Setup:
- companion has food/water needs and bladder/hygiene systems
- travel plan exists
- toilet or private place may or may not be available

Action:
- allow needs to escalate naturally during travel or waiting

Expected:
- companion reprioritizes based on need pressure
- if a toilet/facility exists and is reachable, companion uses it
- if blocked, companion chooses a fallback: ask to stop, seek privacy, use wilderness, accident, or breakdown
- the result produces material and social consequences: waste placement, hygiene change, relationship or witness reaction
- no silent reset of bladder/hunger/hygiene occurs

Pass threshold:
- at least one fallback behavior is observed when the ideal option is unavailable, and its consequences persist in world state

#### T24_PlayerReplacement
Setup:
- choose any living agent currently in world

Action:
- detach human control from current agent and attach it to a different agent at runtime

Expected:
- world simulation continues without reset
- newly controlled agent exposes only its own legal affordances
- former human-controlled agent continues under AI or no control with the same rules
- no simulation code path requires a designated hero entity

#### T25_UnseenCrimeDiscovery
Setup:
- theft can occur without direct witnesses
- guards and civilians can learn via witness, rumor, or inventory audit

Action:
- perform a hidden theft

Expected:
- no immediate global accusation
- suspicion appears only after a discovery pathway fires
- response intensity depends on who learned what and how reliable the information is

#### T26_CameraIndependence
Action:
- move camera or visibility focus repeatedly while simulation runs

Expected:
- no restock, healing, despawn, respawn, or need reset occurs because of visibility changes alone

#### T27_ControlledAgentDeath
Setup:
- human control is attached to a living agent

Action:
- kill or incapacitate that agent through normal world rules

Expected:
- the world simulation continues without rewind
- the dead or incapacitated agent remains in that state
- control can transfer to observer mode or another living agent without altering simulation laws
- no resurrection or failure screen is triggered unless it is itself a general world rule

### 10.3 Soak and Regression Tests

#### T30_SevenDayAutoplay
Run 100 seeded simulations for 7 in-world days with no human input.

Expected:
- zero invariant violations
- zero deadlocks where all agents are stuck in invalid plans
- zero disappearing agents or goods
- at least one unscripted shortage, one political tension, and one route-safety change occur across the run set

#### T31_StressWithFrequentInterruptions
Repeatedly invalidate plans by moving goods, killing leaders, blocking facilities, and attacking carriers during active execution.

Expected:
- agents replan or fail gracefully
- no corrupted state
- no duplicate items
- no orphan reservations

#### T32_LongReplayConsistency
Record a full 3-day run, replay it from seed and input log, and compare event hashes at regular intervals.

Expected:
- exact match at every checkpoint

## 11. Acceptance Criteria for "Emergence"
The prototype qualifies as successful only if all of the following are true:
- the world remains coherent with zero human input
- the four exemplar scenarios arise from general rules, not bespoke scripts
- the human-controlled agent can be reassigned to any other agent
- every major outcome can be traced through the event graph
- the same seed and input log reproduce exactly
- at least one triggered event in each exemplar scenario produces a causal chain of depth >= 4 across >= 3 subsystems

## 12. Implementation Order

### Phase 1: World legality
- topology
- entities/containers
- action schema
- transfer rules
- event log
- replay
- invariants

### Phase 2: Survival and logistics
- needs
- production
- trade
- transport
- theft
- scarcity response

### Phase 3: Information and politics
- witnesses
- rumors
- office
- succession
- loyalty/support

### Phase 4: Group adaptation
- bandit migration
- guard route adaptation
- companion fallback behaviors

Do not start Phase 3 or 4 until Phase 1 determinism and conservation tests are green.
