# Worldwake AI Architecture — Self-Sufficient Deep Analysis Report

**Date**: 2026-03-30
**Purpose**: External deep research review of the AI decision architecture
**Audience**: An LLM with no access to the codebase, tasked with identifying issues, suggesting improvements, recommending new features, and assessing alignment with foundational principles

---

## Table of Contents

0. [Purpose and Reading Guide](#section-0-purpose-and-reading-guide)
1. [Foundational Principles (Verbatim)](#section-1-foundational-principles)
2. [World Model and Simulation Substrate](#section-2-world-model-and-simulation-substrate)
3. [Belief System and Information Locality](#section-3-belief-system-and-information-locality)
4. [Per-Tick Agent Decision Pipeline](#section-4-per-tick-agent-decision-pipeline)
5. [GOAP Plan Search](#section-5-goap-plan-search)
6. [Pressure, Enterprise, and Specialized Subsystems](#section-6-pressure-enterprise-and-specialized-subsystems)
7. [Test Coverage Analysis](#section-7-test-coverage-analysis)
8. [Known Architectural Tensions and Open Questions](#section-8-known-architectural-tensions-and-open-questions)
9. [Appendices](#section-9-appendices)

---

## Section 0: Purpose and Reading Guide

This report is a self-sufficient technical description of the Worldwake AI architecture. It is intended for an external reviewer who has **no access to the codebase**. The reviewer should use this report to:

1. **Identify issues**: Bugs, design flaws, principle violations, or fragility in the AI architecture.
2. **Suggest improvements**: Ways to make the architecture more robust, more aligned with its principles, or more extensible.
3. **Recommend new features**: Capabilities that would be beneficial, architecturally coherent, and aligned with the foundational principles.
4. **Assess principle alignment**: Whether the implemented architecture fully reflects its stated foundational principles.

**Reading map**:
- Section 1 provides the full foundational principles — the standard against which everything is judged.
- Section 2 describes the world model the AI operates over (ECS, topology, actions, events, determinism).
- Section 3 describes the belief system — the most architecturally distinctive feature.
- Sections 4-6 detail the AI decision pipeline, plan search, and specialized subsystems with pseudocode.
- Section 7 maps test coverage to principles and explicitly identifies gaps.
- Section 8 pre-identifies architectural tensions for the reviewer to evaluate.

**Notation**: `Permille` is a fixed-point integer in [0, 1000] used in place of floating-point (the project forbids floats for determinism). `EntityId` is a generational slot-based identifier. `Tick` is a monotonic simulation clock.

---

## Section 1: Foundational Principles

The project defines 31 principles across 6 categories, plus 8 canonical regression scenarios. These are reproduced verbatim below — the reviewer needs the full text to assess alignment.

### Preamble

Every change to the simulation must be an **architecturally comprehensive** solution. Hacks, patches, shims, and workarounds are forbidden. The result must leave the architecture clean (no dead paths), robust (invariants hold under edge cases), and extensible (new systems compose without surgery).

### Category I: Causal Standard (Principles 1-4)

**1. Maximal Emergence Through Local Causality**: All events must arise from prior world state, agent belief, institutional rule, or natural process. Authored sequences or hidden quest logic are forbidden.

**2. No Ungrounded Triggers or Probabilities**: Probabilistic transitions allowed only when grounded in explicit world processes (perception noise, travel delay, disease exposure, etc.). Randomness must be seeded, attributable, replayable. No drama dials.

**3. Concrete State Over Abstract Scores**: Model the thing itself, not a number representing it. Danger = actual threats, not `danger_score`. Abstract summaries allowed only as derived views/caches, never as source of truth.

**4. Persistent Identity, Object Permanence, and Explicit Transfer**: Every meaningful entity has stable identity. Movement, transfer, destruction must be explicit world processes. For conserved quantities, every increase/decrease must have explicit source/sink path.

### Category II: World Dynamics (Principles 5-13)

**5. Simulate Carriers of Consequence, Not Decorative Realism**: Model only what propagates downstream effects. Skip subsystems whose only contribution is "feels more real."

**6. World Runs Without Observers**: Simulation continues meaningfully with no human present. No frozen towns, no suspended economics.

**7. Locality of Motion, Interaction, and Communication**: All physical interaction requires co-location or explicit range. All communication requires explicit carrier (witness, rumor chain, letter, notice, messenger). No global truth queries on behalf of characters.

**8. Every Action Has Preconditions, Duration, Cost, and Occupancy**: Nothing important is free or instantaneous. Long actions unfold over time and remain interruptible. Multiple actors attempting same scarce affordance requires explicit resolution (queue, race, grant, lock).

**9. Scheduling, Simultaneity, and Tie-Breaking Are Part of the World Model**: Authoritative clock, declared update regime, explicit tie-break rules. Tick order and container iteration order may not silently decide outcomes.

**10. Outcomes Are Granular and Leave Aftermath**: Actions are not binary success/fail. They create partial outcomes, side effects, leftovers. Failure is new state, not dead end.

**11. Every Positive Feedback Loop Needs a Physical Dampener**: When A increases B and B increases A, concrete limiting mechanism must exist (resource exhaustion, fatigue, competition, distance, social pushback). Never solve with invisible caps.

**12. Performance May Compress Computation, Never Causality**: Offscreen simulation may batch/summarize only if causally relevant outcomes remain equivalent. Save/load/replay boundaries may change encoding, never world meaning.

**13. Boundaries and External Inputs Are World Processes**: Map edges, imported goods, weather enter through explicit boundary processes with source, delay, capacity, observables, and failure modes.

### Category III: Knowledge, Belief, and Evidence (Principles 14-18)

**14. World State Is Not Belief State**: Ground truth and agent knowledge are SEPARATE LAYERS. Agents act on beliefs, not omniscient world data.

**15. Knowledge Is Acquired Locally and Travels Physically**: Knowledge enters through perception, memory, inference, testimony. Travels with delay, distortion, source attribution, possible loss. Beliefs carry provenance, confidence, freshness.

**16. Ignorance, Uncertainty, and Contradiction Are First-Class**: Agents can not-know, suspect, misremember, hold stale beliefs, believe false/conflicting reports. Unknown != false.

**17. Surprise Comes From Violated Expectation**: Agents notice anomalies relative to prior expectation, commitment, claim, count, reservation. No cheap omniscience about absences.

**18. Memory, Evidence, and Records Are World State**: Memories, accusations, warrants, contracts are state that can be created, transmitted, forgotten, destroyed. Physical aftermath (corpses, tracks, broken locks) is evidence.

### Category IV: Agents, Institutions, and Social Order (Principles 19-25)

**19. Agent Symmetry**: Engine makes no rule distinction between human-controlled and AI-controlled agents. `ControlSource` changes only who chooses action, never what reality allows.

**20. Resource-Bounded Practical Reasoning Over Scripts**: AI agents reason as limited actors using beliefs, priorities, skills, commitments. Goals name desired world conditions, not privileged solutions. Decisions must be explainable as "Agent X chose Y because believed Z and cared about Q."

**21. Intentions Are Revisable Commitments**: Commitments are stable intentions held under assumptions. Intent does not equal entitlement — plans reserve nothing unless world has explicit reservation/queue/contract. Agents must monitor assumptions and revise when evidence invalidates them.

**22. Agent Diversity Through Concrete Variation**: Agents in same role must differ in needs, skills, values, courage, greed, patience, memory reliability, perception fidelity. Differences from concrete per-agent parameters. Homogeneous populations collapse into herd behavior.

**23. Roles, Offices, and Institutions Are World State**: Authority is socially recognized role embedded in places, organizations, rules, records. Treasury can be empty. Office can be vacant. Jurisdiction stops at boundaries. Institutions act through agents and rules, never through omniscient manager code.

**24. Ownership, Custody, Access, Obligation, and Jurisdiction Are Distinct**: Possession != ownership != permission != capability. The model must separate who owns, who holds, who can access, who is owed, which institution adjudicates.

**25. Social Artifacts Are First-Class**: No quest system. Only world entities and records that people create, discover, believe, dispute, fulfill. Bounties, rumors, debts, contracts are world state with issuers, conditions, proof requirements, places, expiration.

### Category V: System Architecture (Principles 26-31)

**26. Systems Interact Through State, Not Through Each Other**: Systems read authoritative state and write new state. Influence travels through state mutation and event history, not hidden cross-system authority. Shared domain services allowed for generic computations (pathfinding, legality checks, pricing) but not for granting exceptions.

**27. Derived Summaries Are Caches, Never Truth**: Derived summaries may exist only as views over concrete source state. Must be invalidatable and replaceable. Distinguish from social artifacts (posted notices, reputation records ARE world state).

**28. No Backward Compatibility in Live Authority Paths**: Don't preserve dead abstractions. When design changes, live authority path changes. Compatibility only at boundaries (save migration, import/export).

**29. Debuggability Is a Product Feature**: Emergence without introspection is indistinguishable from bugs. Must answer: Why did agent do that? Who last held item? Who knows about event? Answers from state, beliefs, records, causal history.

**30. Every New System Spec Must Declare Its Causal Hooks**: 14 mandatory declaration items including: entities/relations introduced, actions that mutate them, information flow, conservation paths, contention rules, failure states, feedback loops, dampeners, derived views, error correction paths, temporal resolution, boundary conditions, validation checks, save/load survival.

**31. Validation and Falsification Are First-Class**: Every subsystem must declare patterns it reproduces, artifacts it must never produce, parameters that destabilize it, traces for detecting failure. Must support adversarial sampling, sensitivity sweeps, causal trace inspection.

### Canonical Regression Scenarios (A-H)

These are permanent acceptance tests — scenario classes the generic simulation must produce from general-purpose systems, not authored sequences:

**A. Beast Starvation -> Caravan Attack -> Report -> Bounty -> Hunt -> Reward**: Beast's food depletes -> beast attacks caravan en route -> survivors report to settlement -> office-holder issues bounty as real record -> adventurers pursue based on beliefs/needs -> hunt via actual travel/tracking/combat -> payment from actual treasury. Failure smell: hidden `post_beast_bounty()` trigger.

**B. Hungry Agent -> Market Trip -> Dragon Attack -> Interrupted Plan -> Retreat**: Agent has hunger -> travels to market (duration-bearing, exposes to events) -> dragon enters perception -> safety assumptions break -> agent re-evaluates and may flee/hide/continue based on beliefs/temperament. Failure smell: atomic "go to market", agent responds to unknowable dragon.

**C. Stored Gold -> Empty Stash -> Discovery -> Robbery Report**: Gold exists concretely -> another agent steals through actual state transition -> owner retains belief gold is present until inspection -> mismatch triggers search/accusation/reporting. Failure smell: gold disappears without transfer path.

**D. Rumor -> Travel -> Empty Source -> Discovery -> Belief Correction -> Replan**: Agent acquires stale belief from rumor -> plans based on belief -> world changes before arrival -> agent observes mismatch -> revises plan from new evidence -> others still act on stale report until new evidence reaches them. Failure smell: agent corrected by global truth before carrier arrives.

**E. Competing Claimants -> Queue or Race -> Expiry/Prune -> Next Actor Acts**: Multiple agents intend to use same scarce resource -> intentions don't silently reserve -> access resolved through explicit race/queue/grant -> losing agents wait/replan. Failure smell: selecting plan secretly guarantees access.

**F. Office Vacancy -> Succession Delay -> Patrol Gap -> Route Predation**: Office-holder dies -> succession rules apply -> duties delayed/dropped -> patrol gap noticed through local evidence -> opportunistic actors exploit. Failure smell: "the town" seamlessly continues without lawful successor.

**G. False Rumor -> Wrongful Accusation -> Contested Evidence -> Correction or Miscarriage**: False claim circulates through explicit carriers -> agents act on false claim -> institutions respond per their beliefs, not omniscient truth -> new evidence appears later -> different actors update at different times. Failure smell: institutions corrected instantly by ground truth.

**H. Remote Shock -> Delayed Arrival Failure -> Local Shortage -> Substitution or Exit**: Settlement depends on imports -> disruption occurs externally -> expected arrival fails -> agents act on prior expectations until local evidence updates -> inventories tighten through actual consumption -> buyers/suppliers react. Failure smell: off-map dependence only when script wants drama.

---

## Section 2: World Model and Simulation Substrate

### 2.1 Crate Architecture

The project is a 5-crate Rust workspace with strict dependency ordering:

```
worldwake-core    (no deps)      -> IDs, types, ECS store, topology, items, relations
worldwake-sim     (deps: core)   -> Event log, action framework, scheduler, replay
worldwake-systems (deps: core, sim) -> Needs, production, trade, combat, travel actions
worldwake-ai      (deps: core, sim, systems) -> GOAP planner, goal ranking, decision runtime
worldwake-cli     (deps: all)    -> Human control interface
```

Key constraint: `worldwake-systems` modules depend only on `core` and `sim`, never on each other (Principle 26). The AI crate depends on all lower crates but systems modules are decoupled.

### 2.2 Entity-Component System

Custom ECS (no external crate) with deterministic `BTreeMap`-based typed component storage:

- **EntityId**: Generational slot allocator — `EntityId(slot_index, generation)`. Reuse prevents stale references.
- **EntityKind**: Enum classifying entities — `Agent`, `Place`, `ItemLot`, `UniqueItem`, `Container`, `TravelEdge`, `Workstation`, `ResourceSource`, `Office`, `Faction`, `CrimeRegister`, etc.
- **ComponentTables**: Macro-generated typed storage. Each component type has its own `BTreeMap<EntityId, T>`.
- **RelationTables**: Placement, ownership, reservation, social relations (hostility, support, faction membership) as indexed relation tables.

All authoritative state uses `BTreeMap`/`BTreeSet` (never `HashMap`/`HashSet`) for deterministic iteration order.

### 2.3 Topology

The world is a **place graph** with travel times, not continuous space:

- **Place**: Named location with tags (Village, Farm, Latrine, Market, etc.)
- **TravelEdge**: Weighted directed edge between places with travel time in ticks
- **Route**: Ordered sequence of places from Dijkstra pathfinding
- **Topology**: The complete place graph, stored in `World`

All interactions require co-location. Travel is a duration-bearing action that exposes the agent to events en route. There is no teleportation.

### 2.4 Action Framework

Every action is declaratively defined via `ActionDef`:

```
ActionDef {
    id: ActionDefId,
    name: String,
    domain: ActionDomain,           // Generic, Needs, Production, Trade, Travel, Combat, etc.
    actor_constraints: [Constraint], // Who can do this?
    targets: [TargetSpec],          // What can it target?
    preconditions: [Precondition],  // What must be true to start?
    duration: DurationExpr,         // How long does it take?
    body_cost_per_tick: BodyCostPerTick, // Per-tick resource drain
    interruptibility: Interruptibility,  // NonInterruptible | InterruptibleWithPenalty | FreelyInterruptible
    commit_conditions: [Precondition],   // What's needed to complete?
    visibility: VisibilitySpec,     // Who sees it happen?
    causal_event_tags: Set<EventTag>,    // Evidence left behind
    payload: ActionPayload,         // Domain-specific data
    handler: ActionHandlerId,       // Which system executes it
}
```

Action lifecycle: `Requested -> Started -> Active (ticking) -> Committed | Aborted`

Actions are validated against the authoritative world state at start time. The planner's intent does not guarantee access (Principle 21).

### 2.5 Tick System and Execution Order

Each tick runs systems in this fixed order:

```
Needs -> Production -> Trade -> Combat -> FacilityQueue -> Politics -> Perception
```

The ordering is **load-bearing**. Key constraint: Politics runs before Perception so that institutional state changes are visible to co-located observers in the same tick.

Within a tick:
1. Produce inputs (player commands or AI decisions)
2. Abort dead actors' actions
3. Process inputs (start requested actions after validation)
4. Progress active actions (tick durations, complete if done)
5. Run system functions in the order above
6. Abort newly-dead actors
7. Emit end-of-tick marker in event log
8. Increment tick counter

### 2.6 Event Log and Causality

Append-only `EventLog` — the authoritative causal record:

- **EventRecord**: Contains `EventId`, `Tick`, `CauseRef` (links to prior causing event), `WitnessData` (who observed), `ComponentDelta`/`RelationDelta` (what changed), `EventTag` (classification).
- Events are never mutated or deleted.
- Causal chains are reconstructable from `CauseRef` links.
- Used for perception (agents only learn from witnessed events), debuggability, and replay.

### 2.7 Determinism Guarantees

- **ChaCha8Rng** seeded per simulation — same seed + same inputs = identical outcomes
- **No floats** — all ratios use `Permille` (integer 0-1000)
- **BTreeMap/BTreeSet only** in authoritative state (deterministic iteration)
- **Canonical hashing** via `blake3` — `hash_world()` and `hash_event_log()` produce deterministic state digests
- **Deterministic replay**: `replay_and_verify()` replays from initial state + seed + inputs, comparing per-tick hashes
- **Save/load round-trips** preserve all state including AI runtime, verified by golden tests

### 2.8 Conservation Invariants

Items cannot be created or destroyed except through explicit actions:

- `verify_live_lot_conservation(world, commodity, expected_qty)` — counts all live item lots
- `verify_authoritative_conservation(world, commodity, expected_qty)` — counts authoritative quantities
- Golden tests assert conservation after every scenario

### 2.9 Relation System

Relations are first-class indexed data, not just component flags:

- **Placement**: Entity-at-Place
- **Ownership**: Entity-owns-Entity (with distinction from custody/possession)
- **Reservation**: Entity-reserves-Entity (for facility queues, exclusive access)
- **Social**: Hostility, faction membership, support declarations, office holding
- **ArchiveDependency**: Tracks what depends on what for safe purging

---

## Section 3: Belief System and Information Locality

### 3.1 Architecture Overview

The belief system enforces **Principle 14** (World State Is Not Belief State) by interposing a belief layer between the AI and the world:

```
World (authoritative truth)
  |
  v
PerAgentBeliefView (filters through what agent has perceived)
  |
  v
GoalBeliefView / RuntimeBeliefView (trait interfaces for AI)
  |
  v
AI Decision Pipeline (reads ONLY through belief traits)
```

The AI never reads `&World` directly. All reads go through trait interfaces that return what the agent believes, which may be stale, incomplete, or wrong.

### 3.2 Belief Traits

Two trait interfaces exist:

**GoalBeliefView** — used for candidate generation and ranking:
```
trait GoalBeliefView {
    fn homeostatic_needs(agent) -> Option<HomeostaticNeeds>  // Self-authoritative
    fn drive_thresholds(agent) -> Option<DriveThresholds>    // Self-authoritative
    fn wounds(agent) -> Vec<Wound>                           // Self-authoritative
    fn visible_hostiles_for(agent) -> Vec<EntityId>          // Belief-filtered
    fn current_attackers_of(agent) -> Vec<EntityId>          // Belief-filtered
    fn merchandise_profile(agent) -> Option<MerchandiseProfile>  // Self-authoritative
    fn demand_memory(agent) -> Vec<DemandObservation>        // Belief memory
    fn commodity_quantity(holder, kind) -> (u32, ...)         // Belief about others
    fn agents_active_at(place, domain, filter) -> Vec<EntityId>  // Observed activity
    fn courage(agent) -> Option<Permille>                    // Self-authoritative
    fn bandit_factions_of(agent) -> Vec<EntityId>            // Self-authoritative
    // ... ~30 more methods
}
```

**RuntimeBeliefView** — broader surface for planning/search:
```
trait RuntimeBeliefView {
    fn is_alive(entity) -> bool
    fn effective_place(entity) -> Option<EntityId>
    fn entities_at(place) -> Vec<EntityId>
    fn direct_possessions(holder) -> Vec<EntityId>
    fn adjacent_places(place) -> Vec<EntityId>
    fn commodity_quantity(holder, kind) -> Quantity
    fn resource_source(entity) -> Option<ResourceSource>
    fn workstation_tag(entity) -> Option<WorkstationTag>
    fn can_control(actor, entity) -> bool
    fn is_dead(entity) -> bool
    fn has_wounds(entity) -> bool
    // ... ~25 more methods
}
```

Key distinction:
- **Self-authoritative reads** (agent's own needs, wounds, courage, skills): read directly from World because agents know their own state.
- **Subjective reads** (other entities' positions, inventories): filtered through what the agent has perceived — may be stale or wrong.

### 3.3 Belief Storage

Each agent has an `AgentBeliefStore` containing:
- **BelievedEntityState**: Per-entity beliefs (position, inventory, wounds, alive status) with perception source and observed tick
- **BelievedInstitutionalClaim**: Institutional knowledge (office holders, laws) with source provenance
- **PerceptionSource**: Enum — `DirectObservation`, `Testimony(source_agent, chain_length)`, `InferredFromEvidence`, `Report`
- **Conversation memory**: Who the agent told what, when (to avoid redundant tells)
- **Demand memory**: Remembered trade demand observations at market locations

### 3.4 Perception and Information Flow

Information enters an agent's belief store through:
1. **Direct observation**: Agent perceives entities at their current place (controlled by `PerceptionProfile`)
2. **Testimony (Tell action)**: Another co-located agent explicitly shares a belief
3. **Evidence observation**: Physical aftermath (corpses, tracks, empty containers) triggers inference
4. **Institutional records**: Reading posted notices, crime registers, office records

Each belief path carries provenance metadata:
- Source agent and chain length (for rumor degradation)
- Observation tick (for staleness detection)
- Confidence level (for contradiction resolution)
- Perception fidelity (per-agent `PerceptionProfile` parameter)

### 3.5 PerceptionProfile (Agent Diversity in Observation)

```
PerceptionProfile {
    memory_capacity: usize,           // Max beliefs agent can hold
    memory_retention_ticks: u64,      // TTL before beliefs decay
    observation_fidelity: Permille,   // 0-1000, accuracy of observation
    confidence_policy: BeliefConfidencePolicy,
    institutional_memory_capacity: usize,
    consultation_speed_factor: Permille,
    contradiction_tolerance: Permille,
}
```

Different agents have different perception profiles, creating agent diversity in what they notice, how long they remember, and how much they trust testimony (Principle 22).

### 3.6 Knowledge Path Tracing

During candidate generation, the system traces how the agent arrived at each goal candidate:

- `KnowledgePath`: Records the epistemic provenance of each opportunity
- `BeliefProvenance`: Tracks which belief, when acquired, from what source, led to this goal
- Each `GroundedGoal` carries evidence entities and places

This supports Principle 29 (Debuggability) — for any AI decision, you can trace back to the belief that motivated it.

### 3.7 Belief Staleness and Correction

Golden tests verify the full belief correction cycle:
1. Agent acquires belief (possibly stale) about remote resource
2. Agent travels based on that belief
3. Agent arrives and observes mismatch (resource depleted, entity moved, etc.)
4. Agent updates belief from direct observation
5. Agent replans based on corrected belief
6. Other agents may still hold the stale belief until they observe or are told

This is directly tested in `golden_stale_belief_travel_reobserve_replan` and `golden_rumor_leads_to_wasted_trip_then_discovery`.

---

## Section 4: Per-Tick Agent Decision Pipeline

### 4.1 Pipeline Overview

Every tick, for each AI-controlled agent, the `AgentTickDriver` runs the full decision pipeline:

```
process_agent(agent):
  1. Dead agent check          -> early return if dead
  2. Reconcile in-flight state -> handle completed/aborted actions
  3. Frame assumption evaluation -> check if current intention's assumptions still hold
  4. Read phase:
     a. Candidate generation   -> enumerate possible goals from beliefs
     b. Ranking                -> score and prioritize goals
  5. Deferred NoCriticalThreat evaluation
  6. Feasibility annotation    -> mark goals as Likely/Uncertain/Unlikely
  7. Decision branch:
     IF active action exists   -> evaluate interrupt (should we abandon current action?)
     ELSE                      -> plan search, plan selection, step execution
  8. Patience exhaustion check -> if frame stalled too long, mark exhausted
  9. Frame transition tracking -> record frame changes for tracing
  10. Finalization             -> persist frame, goal, intents, blocked memory to world
```

### 4.2 Observation and Reconciliation Phase

Before making decisions, the runtime reconciles what happened since last tick:

**reconcile_in_flight_state()**:
- If the agent had an action in flight and the scheduler reports it completed → advance to next plan step
- If the action was aborted (target died, precondition failed) → mark for replan
- If a `ReplanNeeded` signal exists → clear plan and dirty flags
- If a committed action matches the current plan step → apply materialization bindings (map hypothetical entities to now-real entities)

**refresh_runtime_for_read_phase()**:
- Compute `DirtySet` by comparing current world state to cached snapshots:
  - Position changed? → `STRUCTURAL_MASK`
  - Needs changed bands? → `SNAPSHOT_MASK`
  - Wounds changed? → `SNAPSHOT_MASK`
  - Commodity/unique item signature changed? → `SNAPSHOT_MASK`
  - Facility access changed? → `SNAPSHOT_MASK`
- If any dirty flags set → invalidate exhaustion cache entries that match

### 4.3 Frame System (Intention Frames)

The **IntentionFrame** implements Principle 21 (Intentions Are Revisable Commitments):

```
IntentionFrame {
    domain: IntentionDomain,           // What kind of activity (Travel, Harvest, Trade, etc.)
    destination: Option<EntityId>,     // Where we're heading (if travel-based)
    assumptions: Vec<FrameAssumption>, // What must remain true for this frame to persist
    state: FrameState,                 // Active | Suspended | Exhausted
    stalled_ticks: u16,                // How long without progress
    patience_limit: u16,               // Max stalled ticks before exhaustion
}
```

**FrameAssumption** types:
- `NoCriticalThreat` — no Critical-class danger exists
- `DestinationReachable(place)` — the destination is still believed reachable
- `GoalStillRelevant(goal_key)` — the goal's preconditions still hold
- `ResourceAvailable(entity)` — a specific resource is still believed available

**Assumption evaluation** runs before candidate generation:
- If any assumption fails → frame is cleared (with reason `AssumptionFailed`)
- If `NoCriticalThreat` fails → frame suspended (deferred until after ranking to check if a Critical challenger exists)
- If frame is cleared → agent starts fresh planning this tick

**Patience exhaustion**:
- Each tick the frame is Active but no progress occurs, `stalled_ticks` increments
- When `stalled_ticks >= patience_limit` → frame marked `Exhausted`
- Exhaustion records a blocked intent (see Section 4.11)
- Tested in `golden_facility_queue_patience_timeout`

### 4.4 Candidate Generation

**generate_candidates()** enumerates all possible goals from the agent's belief state:

**Homeostatic needs** (hunger, thirst, fatigue, bladder, dirtiness):
- For each need above its low threshold:
  - Generate `ConsumeOwnedCommodity` if agent has consumable matching the need
  - Generate `AcquireCommodity(SelfConsume)` if agent knows where to get consumable
  - Generate `Sleep` / `Relieve` / `Wash` for non-commodity needs

**Danger**:
- If visible hostiles or current attackers exist:
  - Generate `ReduceDanger` (defensive stance)
  - Generate `EngageHostile(target)` for each hostile

**Combat targets** (bandit faction logic):
- If agent is a bandit with raid disposition:
  - Generate `RaidTarget(target)` for vulnerable targets
  - Check wound deterrence threshold (courage-scaled)

**Care / wounds**:
- If agent has wounds:
  - Generate `TreatWounds(self)` (self-care)
- If agent observes wounded co-located agent and has care disposition:
  - Generate `TreatWounds(patient)` (altruistic care)
  - NOTE: Only direct observation triggers care — indirect reports do NOT (tested in `golden_indirect_report_does_not_trigger_care`)

**Enterprise / merchant**:
- If agent has `MerchandiseProfile`:
  - Compute `EnterpriseSignals` — restock gaps from demand memory vs. current stock
  - For each gap: generate `RestockCommodity(commodity)` or `ProduceCommodity(recipe)`
  - Generate `SellCommodity(commodity)` if stock exceeds demand

**Political goals**:
- If agent knows of vacant office within jurisdiction:
  - Generate `ClaimOffice(office)`
- If agent supports a candidate:
  - Generate `SupportCandidateForOffice(office, candidate)`

**Social goals**:
- For each co-located agent the agent has untold beliefs about:
  - Generate `ShareBelief(listener, topic)` based on tell profile and conversation memory
  - Suppressed if agent is under survival stress
  - Duplicate tells suppressed (conversation memory tracks what was told)

**Theft goals**:
- If agent has theft disposition and perceives accessible unguarded items:
  - Generate `StealItem(target_item)`
  - Suppressed by witness deterrence (observed bystanders reduce theft willingness)

**Justice goals**:
- If agent has justice authority and knows of recorded violations:
  - Generate `Accuse(crime_register, accused, violation)`
  - Generate `PunishAccused(office, accused, entry, punishment)`

**Miscellaneous**:
- `LootCorpse(corpse)` — if dead entities with lootable containers are visible
- `BuryCorpse(corpse, burial_site)` — if unburied corpses are visible
- `MoveCargo(commodity, destination)` — transport-related goals
- `InvestigateViolation(violation_id, place)` — if expectation violations detected

Each candidate carries:
- `GoalKey` (kind + optional target)
- `OpportunityAnchor` (the place or entity this opportunity is tied to)
- Evidence entities/places (for knowledge path tracing)

### 4.5 Goal Ranking

**rank_candidates()** scores and orders all candidates:

**Step 1: Suppression check**

`evaluate_suppression(goal_kind, decision_context)`:
- Compute `DecisionContext` = max priority class across all homeostatic needs + danger class
- Self-care goals (Consume, Sleep, Relieve, Wash, TreatWounds(self)): Never suppressed
- Danger goals (ReduceDanger, EngageHostile): Never suppressed
- Enterprise/social/political/theft/justice: Suppressed if agent is stressed above threshold
- Stress = `max(self_care_class, danger_class)`
- Suppressed goals are recorded but excluded from ranking

**Step 2: Priority class assignment**

Five priority classes (ascending):
```
Background < Low < Medium < High < Critical
```

For **danger goals**: priority class = `danger_class` from `DangerAssessment`

For **drive goals** (needs, enterprise, social, etc.):
- Map the relevant `Permille` value to a `ThresholdBand`:
  - `value >= critical_threshold` → `Critical`
  - `value >= high_threshold` → `High`
  - `value >= medium_threshold` → `Medium`
  - `value >= low_threshold` → `Low`
  - else → `Background`
- `ThresholdBand` is per-agent (from `DriveThresholds` component), enabling agent diversity

**Step 3: Motive score computation**

For **danger goals**: `utility.danger_weight * danger_pressure / 1000`

For **drive goals**: Maximum across all `RankedDriveMotiveInput` values:
- Each input has: `psi` (raw pressure/satiation Permille), `score` (weighted result)
- `score = utility_weight * psi / 1000`
- `UtilityProfile` is per-agent, containing weights for: hunger, thirst, fatigue, bladder, dirtiness, danger, enterprise, care, social, political

**Step 4: Competition discount**

When multiple agents are observed active at the same place in the same domain:
```
observed_count = min(competitors_at(place, domain), 3)
awareness = utility.activity_awareness_weight (per-agent, Permille)
factor = 1000 - (awareness * observed_count)
post_discount_motive = max(1, motive * factor / 1000)
```

An agent with `activity_awareness_weight = 0` ignores competition entirely.

**Step 5: Sort**

Goals sorted by: `(priority_class DESC, motive_score DESC, lexicographic tiebreak)`

### 4.6 Feasibility Pre-Check

Before planning, each ranked goal gets a `FeasibilityHint`:
- `Likely`: No blockers known, no exhaustion entry
- `Uncertain`: Default
- `Unlikely`: Exhaustion cache has entry for this goal, or blocked intent memory has unexpired entry

Goals are re-sorted incorporating feasibility (after the primary rank sort) to bias plan search toward likely-feasible goals first.

### 4.7 Active Action Branch (Interrupt Evaluation)

If the agent has an active action, the pipeline evaluates whether to interrupt:

**evaluate_interrupt()** checks:

1. **Interruptibility of current action**:
   - `NonInterruptible` → never interrupt (e.g., some combat actions)
   - `InterruptibleWithPenalty` → only Critical-class challengers can interrupt
   - `FreelyInterruptible` → full comparison logic

2. **Plan validity**: If current plan is no longer valid → `InterruptForReplan(PlanInvalid)`

3. **Best challenger identification**: Highest-ranked candidate that differs from current goal

4. **Penalty interrupt path** (for `InterruptibleWithPenalty`):
   - Only if challenger is `Critical` class
   - Consult `GoalFamilyPolicy.penalty_interrupt`:
     - `WhenCritical { trigger }` → interrupt with specified trigger
     - `Never` → no interrupt

5. **Free interrupt path** (for `FreelyInterruptible`):
   - **Opportunistic goals** (e.g., LootCorpse): Blocked if agent stressed at Medium+
   - **Normal goals**: Compare via `relation_aware_interrupt_candidate`:
     - `HigherPriorityGoal`: Challenger has strictly higher priority class
     - `SuperiorSameClassPlan`: Same class but motive exceeds current by switch margin
   - Frame awareness: different margins for frame-based vs. non-frame comparison
     - Default switch margin: 100 Permille (10%)
     - Frame switch margin: potentially higher (making it harder to abandon committed travel)

**InterruptTrigger** enum:
```
CriticalSurvival    — self-care at Critical priority
CriticalDanger      — danger at Critical priority
HigherPriorityGoal  — challenger has higher priority class
SuperiorSameClassPlan — challenger same class, higher motive by margin
PlanInvalid         — current plan no longer executable
OpportunisticLoot   — opportunistic goal available when unstressed
```

### 4.8 Planning Path

When no active action exists, the agent enters the planning path:

**Step 1: Build planning snapshot**

`PlanningSnapshot` — immutable read-only capture of the agent's belief state at planning time:
- Indexed entities with their believed states
- Place topology
- Institutional beliefs
- The agent's own state (needs, wounds, inventory, position)

**Step 2: Create planning state**

`PlanningState<'snapshot>` wraps the snapshot and supports hypothetical modifications:
- Entity place overrides (simulating movement)
- Commodity quantity overrides (simulating consumption/acquisition)
- Needs overrides (simulating eating/drinking/sleeping)
- Pain overrides (simulating healing)
- Unique item quantity overrides
- Facility queue/grant overrides
- Support declaration/office holder belief overrides
- Hypothetical entity registry (for items not yet crafted)

These overrides allow the GOAP search to simulate multi-step plans without mutating real state.

**Step 3: Search for plan** (see Section 5 for full details)

`search_plan()` returns:
- `Found(PlannedPlan)` — viable plan discovered
- `BudgetExhausted` — search ran out of node expansion budget
- `FrontierExhausted` — all search nodes explored, no plan exists
- `Unsupported` — goal kind not handled by planner

**Step 4: Budget for multiple goals**

The system searches plans for the top `max_candidates_to_plan` ranked goals (default: 2). This means the agent considers alternatives if the top goal can't be planned.

### 4.9 Plan Selection

**select_best_plan()** chooses which plan to adopt:

1. Collect all `SelectionCandidatePlan` (ranked goal + optional found plan)
2. Filter to only goals that have plans
3. Sort by `(priority_class DESC, motive_score DESC, estimated_ticks ASC)`

**If no current plan**: Accept best available plan

**If current plan exists**: Check goal switching:
- `RefreshesFrame` check: If challenger has same goal + same travel destination → switch (frame refresh)
- `compare_relation_aware_goal_switch()`:
  - `HigherPriorityGoal`: Always switch
  - `SameClassMargin`: Switch only if `challenger_motive >= current_motive + (current_motive * margin / 1000)`
- Respect `frame_switch_margin` (higher margin for traveling frames, making it harder to abandon mid-travel)

### 4.10 Step Execution

Once a plan is selected, the first step is executed:

1. **Resolve step targets**: Convert `PlanningEntityRef` to actual `EntityId`
   - `Authoritative(id)` → use directly
   - `Hypothetical(id)` → look up in `MaterializationBindings` (from prior committed actions)
   - If binding missing → failure path

2. **Revalidate step**: Check step still feasible against current affordances
   - Can detect: binding failures, target death, precondition failures

3. **Enqueue action**: Create `InputEvent::RequestAction` with:
   - Action def ID, targets, payload override
   - Mode: `BestEffort` (action framework validates independently)
   - Mark `step_in_flight = true`

4. **Handle failure**: If revalidation fails:
   - Try `handle_recoverable_travel_step_blockage()` for travel failures
   - Otherwise → `handle_plan_failure()` (Section 4.11)

### 4.11 Failure Handling

When a plan step fails (action start failure, revalidation failure, or runtime abort):

**handle_plan_failure()**:

1. **Clear plan and frame**: `current_plan = None`, `frame = None` (reason: `PlanFailed`)
2. **Clear materialization bindings**

3. **Derive blocking fact** from the failure context:
   - `TargetGone` — target entity is dead or missing
   - `NoKnownPath` — travel step but no known route
   - `SellerOutOfStock` — trade failure, seller has no inventory
   - `BuyerCannotAfford` — trade failure, buyer lacks payment
   - `InputMissing` — can't find input commodity for consumption/production
   - `CombatTooRisky` — combat situation too dangerous
   - `Unknown` — didn't match any specific pattern

4. **Compute TTL** based on blocking fact type:
   - Transient (20 ticks): `SellerOutOfStock`, `InputMissing`
   - Unknown (5 ticks): Generic `Unknown`
   - Structural (200 ticks): `NoKnownPath`, `TargetGone`

5. **Record blocked intent** in `BlockedIntentMemory`:
   - Key: `(goal_key, place, target, action_def)`
   - Blocking fact + TTL + diagnostic context
   - Prevents re-planning the same failed goal from the same place/target until TTL expires

6. **Exhaustion cache** (for search-level failures):
   - `FrontierExhausted`: Won't retry until invalidation condition triggers (position changed, commodity changed, wounds changed, etc.)
   - `BudgetExhausted`: Eligible after exponential cooldown (4 → 8 → 16 → 32 → 64 ticks)
   - Invalidation conditions include: `PositionChanged`, `CommodityChanged(kind)`, `WoundsChanged`, `FacilitiesChanged`, `BlockerExpired`, `HostilesChanged`, `NeedChangedBands`, `TargetDead(entity)`

---

## Section 5: GOAP Plan Search

### 5.1 Algorithm Overview

The plan search is an **A* graph search over hypothetical state space**, where:
- **Nodes** represent hypothetical world states after a sequence of actions
- **Edges** represent individual action steps
- **Goal test** checks if the desired world condition is satisfied in the hypothetical state
- **Cost** = cumulative estimated action ticks + A* heuristic

### 5.2 Search Node Structure

```
SearchNode {
    state: PlanningState,          // Hypothetical world state (snapshot + overrides)
    steps: SharedVec<PlannedStep>, // Actions taken so far (shared for memory efficiency)
    total_estimated_ticks: u32,    // Cumulative action durations
    search_cost: u32,              // Cost = total_estimated_ticks (Dijkstra component)
    heuristic_ticks: u32,          // A* heuristic: min travel cost to goal-relevant place
}
```

`SharedVec` uses reference-counted sharing to avoid deep-copying step lists when branching.

### 5.3 Search Loop

```
search_plan(snapshot, goal, semantics, registry, budget):
    frontier = priority_queue [root_node(snapshot, goal)]
    expansions = 0
    best_barrier = None

    while node = frontier.pop():
        if goal.is_satisfied(node.state):
            return Found(plan from node.steps)

        if node.steps.len() >= budget.max_plan_depth:  // default: 8
            continue  // skip, too deep

        if expansions >= budget.max_node_expansions:  // default: 224
            return best_barrier or BudgetExhausted

        expansions += 1

        candidates = search_candidates(goal, node, semantics, registry)
        prune_travel_away_from_goal(candidates, current_place, goal_places)

        for candidate in candidates:
            successor = build_successor(goal, node, candidate)
            if successor is terminal (materialization barrier):
                best_barrier = successor  // remember as fallback
            frontier.push(successor)

    return FrontierExhausted
```

### 5.4 Search Candidate Generation

For each search node, candidates are generated from two sources:

1. **Goal-model candidates**: The goal kind specifies which `PlannerOpKind` values are relevant
2. **Affordance-based candidates**: Actions available to the agent at their current (hypothetical) place

Each candidate is filtered by:
- Target binding (does the action match the goal's target requirement?)
- Precondition checking (are preconditions met in the hypothetical state?)
- Blocked intent memory (is this action blocked for this goal/place/target?)

### 5.5 Hypothetical State Transitions

When a candidate is selected, `build_successor()` creates a new search node:

1. Clone the `PlanningState` (cheap due to shared backing)
2. Apply hypothetical transition based on `PlannerTransitionKind`:
   - `GoalModelFallback`: Goal-specific state change (e.g., for Travel → update actor's hypothetical place)
   - `ConsumeMatchingTargetCommodity`: Reduce commodity quantity in hypothetical state
   - `PickUpGroundLot`: Move item lot to actor's hypothetical inventory
   - `StealGroundLot`: Same as pick-up but for unowned items
   - `PutDownGroundLot`: Move item lot from actor to ground

3. Apply action-specific overrides:
   - Travel → entity place override
   - Eat/Drink → needs override (reduce hunger/thirst)
   - Heal → pain override (reduce wound severity)
   - Harvest/Craft → these are **materialization barriers** (see Section 5.7)

### 5.6 A* Heuristic

The heuristic estimates minimum travel cost to the nearest goal-relevant place:

```
heuristic(node, goal):
    actor_place = node.state.effective_place(actor)
    goal_places = combined_relevant_places(goal, node.state, recipes)
    return min(perceived_travel_cost(actor_place, goal_place) for goal_place in goal_places)
```

Travel costs use the **agent's perceived travel cost model** (which may differ from reality if the agent has incomplete route knowledge).

**Travel pruning**: Candidates that move the agent away from all goal-relevant places are pruned from the frontier, reducing search waste.

### 5.7 Materialization Barriers

Some actions produce NEW entities (Harvest creates item lots, Craft transforms inputs to outputs, Trade exchanges goods, Loot extracts container contents). These are marked `is_materialization_barrier: true`.

The planner handles materialization barriers by:
1. When reaching a barrier step, recording the plan as `best_barrier`
2. Stopping further expansion past the barrier (the planner can't reliably predict what entities will materialize)
3. Returning the barrier plan if the budget is exhausted before finding a full plan

At execution time, when a materialization barrier action commits:
- The `CommittedAction` reports which entities were materialized
- The runtime creates `MaterializationBindings` mapping `HypotheticalEntityId` → actual `EntityId`
- Subsequent plan steps that referenced hypothetical entities are resolved through these bindings

This design ensures the planner doesn't assume speculative outcomes (Principle 14 — no omniscient planning).

### 5.8 Budget Parameters

```
PlanningBudget {
    max_candidates_to_plan: 2,        // Only search plans for top 2 ranked goals per tick
    max_plan_depth: 8,                // Plans capped at 8 steps
    snapshot_travel_horizon: 6,       // Consider places up to 6 travel-hops away
    max_prerequisite_locations: 3,    // Max prerequisite locations for multi-source plans
    max_node_expansions: 224,         // Abort search after 224 node expansions
    beam_width: 8,                    // Keep 8 best nodes per depth level
    switch_margin_permille: 100,      // 10% improvement required to switch goals same class
    transient_block_ticks: 20,        // Minor blocker TTL
    unknown_block_ticks: 5,           // Unknown failure TTL
    structural_block_ticks: 200,      // Major blocker TTL
    initial_cooldown_ticks: 4,        // Budget retry cooldown start
    max_cooldown_ticks: 64,           // Budget retry cooldown max
}
```

**Beam width**: At each depth level, only the 8 best nodes (by search cost + heuristic) are retained. This prevents exponential frontier growth but may miss optimal plans.

### 5.9 Planner Conformance Testing

21 conformance tests verify that hypothetical state transitions match real action outcomes:

For each action type (eat, drink, sleep, relieve, pick_up, put_down, harvest, craft, travel, trade, tell, investigate, accuse, loot, heal, attack, bury, declare_support, press_force_claim, queue_for_facility), the test:
1. Runs the hypothetical transition in `PlanningState`
2. Runs the real action through the action framework
3. Asserts that the direction of change matches (e.g., if hypothetical shows hunger decreasing, real action also shows hunger decreasing)

Note: Conformance tests check **direction agreement**, not exact magnitude match. This is because the planner uses simplified models — exact Permille values may differ between hypothetical and real execution.

---

## Section 6: Pressure, Enterprise, and Specialized Subsystems

### 6.1 Pressure System

**Danger assessment**:
```
assess_danger(view, agent):
    attackers = view.current_attackers_of(agent)
    hostiles = view.visible_hostiles_for(agent)
    has_wounds = view.has_wounds(agent)
    is_incapacitated = view.is_incapacitated(agent)

    if attackers >= 2 or (attackers >= 1 and (has_wounds or is_incapacitated)):
        pressure = thresholds.danger.critical()
    elif attackers >= 1 or (hostiles >= 1 and (has_wounds or is_incapacitated)):
        pressure = thresholds.danger.high()
    elif hostiles >= 1:
        pressure = thresholds.danger.medium()
    else:
        pressure = 0
```

**Pain pressure**:
```
derive_pain_pressure(view, agent):
    return sum(wound.severity for wound in view.wounds(agent))  // saturating at 1000
```

**Bandit raid deterrence**:
```
is_bandit_raid_deterred_by_wounds(view, agent):
    base_threshold = faction.bandit_flee_wound_threshold
    courage = agent.courage
    scaled_threshold = base_threshold * (1000 - courage) / 1000
    return pain_pressure >= scaled_threshold
```

Higher courage = higher pain tolerance before fleeing. This creates agent diversity in combat behavior (Principle 22).

**Priority class mapping**:
```
classify_band(value, band):
    if value >= band.critical: Critical
    elif value >= band.high:   High
    elif value >= band.medium: Medium
    elif value >= band.low:    Low
    else:                      Background
```

`ThresholdBand` values are per-agent (via `DriveThresholds` component), so different agents escalate at different urgency levels.

### 6.2 Enterprise / Merchant Logic

Merchants are ordinary agents with a `MerchandiseProfile`:
```
MerchandiseProfile {
    sale_kinds: Vec<CommodityKind>,   // What this merchant sells
    home_market: Option<EntityId>,     // Where they primarily sell
}
```

**Restock gap calculation**:
```
restock_gap_for_market(view, agent, market, commodity):
    observed_demand = sum(demand_memory observations at market for commodity)
    current_stock = agent's quantity of commodity
    if current_stock < observed_demand:
        return observed_demand - current_stock
    else:
        return None
```

**Opportunity signal** (Permille, 0-1000):
```
market_signal_for_place(view, agent, commodity, place):
    demand = relevant_demand_quantity at place
    stock = agent's current stock
    deficit = demand - stock  (saturating)
    delivered = min(stock, demand)
    dominant = max(deficit, delivered)
    return dominant * 1000 / demand
```

Key design: Merchants only restock when they have **observed demand** at their home market. They don't magically know what's needed — they remember demand from past market visits. This respects Principle 14 (belief-only planning) and Principle 7 (locality).

### 6.3 Goal Family Policy

Each goal kind belongs to a "goal family" with specific suppression and interrupt rules:

| Goal Family | Suppression | Penalty Interrupt | Free Interrupt |
|---|---|---|---|
| SelfCare (Consume, Sleep, Relieve, Wash, TreatWounds-self) | Never suppressed | CriticalSurvival | Reactive |
| Danger (ReduceDanger, EngageHostile) | Never suppressed | CriticalDanger | Reactive |
| Enterprise (Sell, Restock, Produce) | Suppressed when stressed | Never | Normal |
| Social (ShareBelief, Tell) | Suppressed when stressed | Never | Normal |
| Political (ClaimOffice, SupportCandidate) | Suppressed when stressed | Never | Normal |
| Opportunistic (LootCorpse, BuryCorpse) | Suppressed when stressed | Never | Opportunistic |
| Combat (RaidTarget) | Suppressed when stressed | Never | Normal |
| Justice (Accuse, Punish) | Suppressed when stressed | Never | Normal |
| Theft (StealItem) | Suppressed when stressed + deterred by witnesses | Never | Normal |

"Reactive" free interrupt means the goal can interrupt freely-interruptible actions with normal priority comparison. "Opportunistic" means it can only interrupt when the agent is NOT stressed.

### 6.4 Exhaustion Cache

The exhaustion cache prevents wasteful re-searching:

```
ExhaustionEntry {
    retry_state: FrontierExhausted | BudgetRetryPending,
    invalidation_conditions: Vec<ExhaustionInvalidationCondition>,
    baseline: ExhaustionBaseline,     // World state snapshot at exhaustion
    next_retry_tick: Option<Tick>,    // For budget retries
    consecutive_failures: u8,         // Tracks exponential cooldown
}
```

**Invalidation conditions** — the exhaustion entry is cleared (allowing re-search) when:
- `PositionChanged` — agent moved to a new place
- `CommodityChanged(kind)` — specified commodity quantity changed
- `UniqueItemChanged(kind)` — unique item count changed
- `WoundsChanged` — wound count changed
- `FacilitiesChanged` — facility availability changed
- `BlockerExpired` — blocking intent TTL expired
- `HostilesChanged` — hostile entity list changed
- `NeedChangedBands` — homeostatic need moved to a different urgency band
- `StealTargetStateChanged(entity)` — steal target ownership/access changed
- `TargetDead(entity)` — target entity died

**Budget retry cooldown**: Exponential backoff — 4 → 8 → 16 → 32 → 64 ticks between retries.

### 6.5 Theft and Justice

**Theft deterrence**:
```
assess_theft_deterrence(view, agent, target_item):
    witnesses = view.entities_at(target_place) that are alive, not self
    deterrence = witnesses.len() * agent.theft_deterrence_per_witness
    if deterrence >= agent.theft_deterrence_threshold:
        suppress theft candidate
```

Witness count directly suppresses theft goals. Tested in `golden_witness_deterrence_suppresses_theft_candidate`.

**Justice chain**: Theft → discovery (expectation violation when owner inspects) → investigation → accusation → punishment (fine or exile). Each step is a separate goal generated from belief state, not a scripted sequence.

### 6.6 Route Threat Perception

Agents maintain threat memory for routes. When planning travel, the planner uses `perceived_direct_travel_cost_from_memory` which inflates travel cost for routes the agent remembers as dangerous. This affects the A* heuristic and may cause agents to choose longer but safer paths.

---

## Section 7: Test Coverage Analysis

### 7.1 Golden Test Inventory

226 tests across 12 files (plus a 2,500-line test harness):

| File | Tests | Focus |
|---|---|---|
| `golden_ai_decisions` | 18 | Goal invalidation, frontier exhaustion, priority interrupts, blocked intents, deprivation cascades, multi-hop travel, goal switching mid-travel |
| `golden_care` | 18 | Healing, remote medicine acquisition, patient vs self-care, care invalidation, care goal suppression by indirect reports |
| `golden_combat` | 24 | Combat outcomes, death cascades, opportunistic looting, loot suppression under stress, danger avoidance, wound clotting, burial |
| `golden_determinism` | 9 | Replay fidelity, save/load round-trips, state preservation (including promoted commitments, suspended frames, frame assumptions) |
| `golden_emergent` | 50 | Cross-system interactions: care+metabolism, care+politics, care+observation, loot+self-care chains, combat→force succession, social knowledge propagation, force claims+hostility+belief, theft→discovery→accusation, witness deterrence, exile, dual discovery convergence |
| `golden_offices` | 17 | Political office claims (support and force law), succession resolution, knowledge locality, knowledge asymmetry races, survival pressure suppressing politics |
| `golden_production` | 27 | Resource contention, facility queues, queue patience timeout, dead agent pruning from queues, recipe chains, carry capacity, materialization barriers, faction ownership |
| `golden_social` | 14 | Tell mechanics, rumor chain degradation, stale belief correction via travel, skeptical listener rejection, bystander non-learning, duplicate tell suppression, retell after belief change/expiry, chain length filtering, agent diversity in social behavior |
| `golden_supply_chain` | 10 | Multi-role merchant restock cycles, prerequisite-aware crafting, stale prerequisite belief discovery+replan, witness queries for stale info |
| `golden_t22_bandit` | 8 | Bandit camp destruction, pressure-driven raid emergence, raid belief economic cascades, wound-dampened raid spirals |
| `golden_trade` | 8 | Buyer-driven trade acquisition, merchant restock after sale, trade failure → production fallback |
| `planner_conformance` | 21 | Hypothetical transition direction matches real action outcome for all 21 action types |

### 7.2 What Tests Prove (Mapped to Principles)

| Principle | Validated By |
|---|---|
| 1 (Emergence) | All golden_emergent tests (50) — cross-system behaviors emerge without orchestration |
| 2 (No ungrounded triggers) | golden_determinism — same seed reproduces same outcome |
| 3 (Concrete state) | golden_production — conservation checks, golden_combat — wound/bleed/clot mechanics |
| 4 (Persistent identity) | All conservation tests — items traced through transfers |
| 6 (World runs without observers) | golden_determinism.golden_world_runs_without_observers |
| 7 (Locality) | golden_social — tell mechanics, rumor degradation; golden_offices — political facts require tell; golden_care — indirect reports don't trigger care |
| 8 (Preconditions/duration/cost) | golden_production — facility queues, patience timeout; golden_combat — action lifecycle |
| 9 (Scheduling) | golden_determinism — tick-level determinism; golden_production — facility queue rotation |
| 10 (Granular outcomes) | golden_combat — partial outcomes (wound, bleed, clot, death); golden_emergent — cascading consequences |
| 11 (Dampeners) | golden_t22_bandit — wound-dampened raid spirals |
| 12 (Determinism) | Every `_replays_deterministically` variant (~80 tests) |
| 14 (Belief separation) | golden_social — stale belief correction; golden_supply_chain — stale prerequisite discovery |
| 15 (Knowledge travels physically) | golden_social — rumor chains; golden_offices — force control requires tell |
| 16 (Ignorance first-class) | golden_social — skeptical listener rejects; rumor chain degrades |
| 17 (Surprise from violated expectation) | golden_emergent — entity missing triggers investigation; theft discovery |
| 19 (Agent symmetry) | golden_determinism.golden_save_load_round_trip_under_ai — AI controller swap |
| 20 (Resource-bounded reasoning) | All planning tests — budget limits, beam width |
| 21 (Revisable intentions) | golden_ai_decisions — goal switching, interrupts; golden_care — care invalidation; golden_production — queue patience timeout |
| 22 (Agent diversity) | golden_social — agent_diversity_in_social_behavior; golden_combat — seed_sensitivity (different seeds → different combat outcomes) |
| 26 (Systems through state) | golden_emergent — systems interact only through state changes |
| 29 (Debuggability) | golden_ai_decisions — trace enabled scenario; golden_social — decision trace explains retell; golden_combat — action trace records loot lifecycle |

### 7.3 What Tests Do NOT Cover (Gaps)

**Untested Canonical Regression Scenarios**:
- **Scenario A** (Beast Starvation → Bounty chain): Not exercised. Requires beasts with territory/needs, caravan attack, survivor report, institutional bounty creation, hunter pursuit. Most subsystems exist individually but the full chain is untested.
- **Scenario B** (Market Trip → Dragon Interruption): Partially covered by `golden_priority_based_interrupt` and `golden_goal_switching_during_multi_leg_travel`, but no test with a third-party threat entering perception mid-travel.
- **Scenario F** (Office Vacancy → Patrol Gap → Predation): `golden_combat_death_triggers_force_succession` covers succession, but no test verifies the downstream effect of institutional duty gap → exploitation.
- **Scenario G** (False Rumor → Wrongful Accusation → Correction): `golden_witnessed_theft_accusation_chain` covers accusation, but no test verifies behavior when the accusation is based on false information that's later corrected.
- **Scenario H** (Remote Shock → Local Shortage): Not exercised. Requires boundary processes not yet implemented.

**Untested Architectural Properties**:
- **Large population scaling**: All golden tests use 2-6 agents. Behavior under 50+ agents is untested (competition discount, queue contention, belief propagation load).
- **Long-horizon plan stability**: Longest tested plans are ~8 steps. No test verifies that plans spanning 20+ ticks remain stable under world changes.
- **Belief memory pressure**: No test verifies behavior when `memory_capacity` is exceeded and beliefs must be evicted.
- **Multi-agent simultaneous contention for same resource**: `golden_resource_contention_with_conservation` tests 2 agents, but not 5+ agents all targeting the same scarce resource.
- **Belief contradiction resolution**: `golden_skeptical_listener_rejects_told_belief` tests rejection, but no test verifies behavior when an agent holds two contradictory beliefs from different sources.
- **Deep rumor chain distortion**: `golden_rumor_chain_degrades_through_three_agents` tests 3-hop, but no test verifies behavior under 5+ hops with compounding distortion.
- **Materialization binding edge cases**: No test verifies behavior when materialized entities are consumed/moved before the next plan step executes.

### 7.4 Trace Systems

Two complementary trace systems support Principle 29 (Debuggability):

**Decision Trace** (`DecisionTraceSink`):
- Per-agent per-tick: candidates generated, ranking outcome, plan search attempts, selection result, execution outcome, frame transitions
- `dump_agent(agent)`: Human-readable stderr dump
- `summary()`: One-line outcome string

**Action Trace** (`ActionTraceSink`):
- Per-agent per-tick: action started, committed, aborted events with reasons
- `events_for(agent)`, `events_at(tick)`, `events_for_at(agent, tick)`
- `last_committed(agent)`: Most recent completed action

**Cross-Layer Timeline** (`CrossLayerTimeline`):
- Unified view across Decision, Action, EventLog, and Politics traces
- Filterable by agent, office, tick window
- Used in golden tests for debugging complex multi-system scenarios

All trace systems are opt-in and zero-cost when disabled.

---

## Section 8: Known Architectural Tensions and Open Questions

### 8.1 PlanningBudget as Global Constants vs. Agent Diversity (Principle 22)

`PlanningBudget` is a single struct shared by all agents. All agents search with the same depth limit (8), the same expansion budget (224), the same beam width (8), and the same switch margin (100 Permille).

This means a cautious, methodical agent searches with the same parameters as an impulsive, reckless one. Principle 22 calls for "differences from concrete per-agent parameters" — but planning budget is currently uniform.

**Potential improvement**: Per-agent `PlanningBudget` derived from agent traits (e.g., patience affects max_plan_depth, intelligence affects max_node_expansions, impulsiveness affects switch_margin).

### 8.2 Danger Assessment Simplicity vs. Principle 3 (Concrete State)

The current danger assessment (`DangerAssessment`) maps directly to `ThresholdBand` priority classes using a simple rule table (2+ attackers → Critical, 1 attacker + wounds → High, etc.).

Principle 3 says "danger should come from actual threats on routes, not danger_score." The current implementation is closer to a score than a full threat model — it doesn't account for:
- Relative combat capability (a strong agent vs. weak hostile is less dangerous)
- Route-specific danger (danger is assessed at current location only, not along planned paths)
- Weapon/armor quality differentials
- Allied agents nearby who could help

The route threat perception system partially addresses path-based danger, but the core danger assessment is still a flat threshold mapping.

### 8.3 Beam Width and Search Completeness

The GOAP search uses beam width = 8, meaning only 8 nodes are retained per depth level. This is an approximation that may miss optimal plans when:
- Many equally-promising but divergent paths exist
- The optimal plan requires exploring an initially unpromising branch
- Goal-relevant places are distributed in a way that the heuristic doesn't capture

Golden tests caught at least one case where beam width truncation prevented plan discovery (documented in project memory). The current value of 8 was tuned to pass all golden tests, but this doesn't prove it's sufficient for all reasonable scenarios.

### 8.4 Frame System Complexity

The frame system (IntentionFrame + assumptions + patience + exhaustion + suspension + clearing) is intricate:
- 4+ assumption types, each with its own evaluation path
- Frame states (Active, Suspended, Exhausted) with transition rules
- Patience tracking with per-frame limits
- Frame-aware goal switching margins
- Frame plan relations (Refreshes, Suspends, Abandons)

**Risk**: This complexity may be overfit to the specific set of goal kinds currently implemented. Adding a new goal kind that doesn't fit the existing frame semantics could require reworking the frame system.

**Counterargument**: The frame system is implementing Principle 21 (revisable commitments under assumptions), which is fundamental. The complexity may be inherent.

### 8.5 GoalKind as Fixed Enum

`GoalKind` is a hardcoded enum with 22 variants. Adding a new goal requires:
1. Adding a variant to the enum
2. Adding candidate generation logic
3. Adding ranking/suppression rules
4. Adding planner support (search candidates, terminal detection, state transitions)
5. Adding goal family policy entries
6. Adding interrupt rules

This is not a plugin architecture — new goals require touching many files. The tradeoff is:
- **Pro**: Type safety, exhaustive match checking, no runtime dispatch overhead
- **Con**: Adding a new goal kind is a cross-cutting change

For a project with 22 goal kinds that change infrequently, this may be acceptable. But if the goal count grows to 50+, the maintenance burden could become significant.

### 8.6 Planner Conformance Gap

Conformance tests check **direction agreement** (if hypothetical shows hunger decreasing, real action also shows hunger decreasing) but not **magnitude agreement**. This means the planner could make decisions based on inaccurate magnitude estimates:
- Agent plans to eat, expecting hunger to drop to Low band, but real action only drops it to Medium
- Agent skips a second eat action based on hypothetical, but would actually still need it

The current tests don't verify that magnitude differences cause behavioral divergence.

### 8.7 Competition Discount Simplicity

The competition discount (`activity_awareness_weight * min(competitors, 3)`) is:
- Capped at 3 competitors (agents in very crowded areas see the same discount as areas with 3 agents)
- Based on observed activity at a place, not on the specific resource being competed for
- Applied uniformly regardless of how severe the competition actually is

This may lead to suboptimal behavior in high-contention scenarios where more granular competition awareness would help.

### 8.8 Candidate Generation Runs Every Tick

The full candidate generation pipeline runs every tick for every AI agent, including:
- Enumerating all homeostatic needs
- Scanning all visible entities for care/loot/bury/theft/social opportunities
- Analyzing enterprise signals
- Evaluating political opportunities

This is mitigated by the dirty flag system (skipping re-planning when nothing changed), but candidate generation itself doesn't benefit from caching between ticks when the agent's observations haven't changed.

---

## Section 9: Appendices

### Appendix A: GoalKind Enum (22 variants)

```
GoalKind {
    ConsumeOwnedCommodity { commodity }
    AcquireCommodity { commodity, purpose }   // purpose: SelfConsume | Restock | RecipeInput | TreatWounds
    Sleep
    Relieve
    Wash
    EngageHostile { target }
    RaidTarget { target }
    ReduceDanger
    RegroupWithFaction { faction }
    EstablishBanditCamp { faction }
    TreatWounds { patient }
    ProduceCommodity { recipe_id }
    SellCommodity { commodity }
    RestockCommodity { commodity }
    MoveCargo { commodity, destination }
    LootCorpse { corpse }
    BuryCorpse { corpse, burial_site }
    ShareBelief { listener, topic }
    ClaimOffice { office }
    SupportCandidateForOffice { office, candidate }
    InvestigateViolation { violation_id, place }
    StealItem { target_item }
    Accuse { crime_register, accused, violation_id }
    PunishAccused { office, accused, accusation_entry, punishment }
}
```

### Appendix B: PlannerOpKind Enum (27 variants)

```
PlannerOpKind {
    Travel, Consume, Sleep, Relieve, Wash, EstablishCamp,
    Trade, QueueForFacilityUse, Harvest, Craft, MoveCargo,
    Heal, Loot, Bury, Tell, ConsultRecord,
    Attack, Defend, Bribe, Threaten,
    Accuse, Fine, Exile,
    DeclareSupport, PressForceClaim, YieldForceClaim,
    Investigate, AskWitness
}
```

Each `PlannerOpKind` has associated `PlannerOpSemantics`:
```
PlannerOpSemantics {
    op_kind: PlannerOpKind,
    may_appear_mid_plan: bool,          // Can this step appear after the first step?
    is_materialization_barrier: bool,   // Does this create new entities?
    transition_kind: PlannerTransitionKind,  // How does this update hypothetical state?
}
```

### Appendix C: PlanningBudget Defaults

| Parameter | Default | Purpose |
|---|---|---|
| `max_candidates_to_plan` | 2 | Only search plans for top 2 ranked goals |
| `max_plan_depth` | 8 | Plans capped at 8 steps |
| `snapshot_travel_horizon` | 6 | Consider places up to 6 hops away |
| `max_prerequisite_locations` | 3 | Max prerequisite locations for multi-source plans |
| `max_node_expansions` | 224 | Abort search after 224 expansions |
| `beam_width` | 8 | Keep 8 best nodes per depth level |
| `switch_margin_permille` | 100 (10%) | Improvement required to switch same-class goals |
| `transient_block_ticks` | 20 | TTL for minor blockers (out of stock, input missing) |
| `unknown_block_ticks` | 5 | TTL for unclassified failures |
| `structural_block_ticks` | 200 | TTL for major blockers (no path, target gone) |
| `initial_cooldown_ticks` | 4 | Budget retry cooldown start |
| `max_cooldown_ticks` | 64 | Budget retry cooldown maximum |

### Appendix D: GoalPriorityClass Ordering

```
Background < Low < Medium < High < Critical
```

- `Background`: Agent has no pressing need (all needs below low threshold)
- `Low`: Minor need (above low threshold, below medium)
- `Medium`: Moderate need (above medium threshold, below high)
- `High`: Urgent need (above high threshold, below critical)
- `Critical`: Emergency (above critical threshold — starvation, active combat, etc.)

ThresholdBand values are per-agent via `DriveThresholds`, enabling different agents to classify the same Permille value differently.

### Appendix E: Key Type Relationships

```
GroundedGoal {
    key: GoalKey { kind: GoalKind, ... },
    anchor: OpportunityAnchor { Place(id) | Entity(id) | None },
    evidence_entities: Vec<EntityId>,
    evidence_places: Vec<EntityId>,
}

RankedGoal {
    grounded: GroundedGoal,
    priority_class: GoalPriorityClass,
    motive_score: u32,
    provenance: Option<RankedGoalProvenance>,
    competition_discount: Option<CompetitionDiscount>,
    feasibility: FeasibilityHint,
}

PlannedPlan {
    opportunity: OpportunityKey,
    goal_key: GoalKey,
    steps: Vec<PlannedStep>,
    terminal: PlanTerminalKind,    // GoalSatisfied | MaterializationBarrier
}

PlannedStep {
    def_id: ActionDefId,
    op_kind: PlannerOpKind,
    targets: Vec<PlanningEntityRef>,
    payload_override: Option<ActionPayload>,
    estimated_ticks: u32,
}

PlanningEntityRef {
    Authoritative(EntityId),    // Entity known to exist
    Hypothetical(HypotheticalEntityId),  // Entity expected to materialize
}
```

---

*End of report. Total coverage: 31 foundational principles, 8 canonical scenarios, full decision pipeline with pseudocode, GOAP search algorithm, 226 golden tests mapped to principles, explicit gap analysis, and 8 identified architectural tensions.*
