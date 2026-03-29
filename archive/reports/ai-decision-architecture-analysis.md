**Status**: COMPLETED

# Worldwake AI Decision Architecture: Technical Analysis for Deep Research

## 0. Preamble and Instructions for the Researcher

### Purpose

This report provides a complete technical analysis of the AI decision architecture in the **worldwake** project — a causality-first emergent micro-world simulation written in Rust. The AI system is a pressure-based GOAP (Goal-Oriented Action Planning) planner that drives autonomous agent behavior.

The system has grown organically across 13 implementation epics and is suspected of having accumulated substandard patterns, coupling issues, and missed optimization opportunities.

### What You Should Produce

After reading this report, you should be able to:

1. **Identify issues**: Structural problems, coupling hotspots, performance bottlenecks, correctness gaps
2. **Propose improvements**: Refactoring opportunities, architectural cleanups, performance optimizations
3. **Suggest new features**: Capabilities that would make the AI more emergent, robust, or interesting

### Critical Constraint

**All suggestions must respect the foundational principles in Section 1.** These are non-negotiable design rules. Any proposal that violates them — even if it would improve performance or simplify code — is invalid. Read Section 1 carefully before forming opinions about the architecture.

### Self-Contained

This report contains all information you need. No source code access is assumed. All type signatures, data flows, and architecture details are included inline.

---

## 1. Foundational Constraints (Non-Negotiable)

The project is governed by 28 foundational principles organized in 5 categories. Every system, feature, and optimization must be judged against these principles. They are reproduced here in full because they constrain every valid suggestion.

### Overarching Mandate

> Every change to the simulation — new system, revised spec, implementation plan, or bugfix — must be an architecturally comprehensive solution. Hacks, patches, shims, and workarounds that avoid the root design concern are not acceptable, even when they are faster. The result must leave the architecture clean, robust, and extensible.

**Test**: If the most accurate description of a proposed change is "a workaround," "a patch for now," or "a localized fix that avoids the real problem," it violates this mandate.

### I. Causal Standard

**Principle 1 — Maximal Emergence Through Local Causality**: Events are valid only if they arose from prior world state, agent belief, institutional rule, or natural process. No authored sequences, hidden quest logic, or one-off story triggers.

**Principle 2 — No Ungrounded Triggers or Probabilities**: No outcome may bottom out at a naked designer dial (`chanceOfEncounter`, `spawnRate`). Randomness is allowed only when it stands in for hidden local microstate (perception noise, execution uncertainty). Utility weights, need rates, and skill parameters may exist as concrete agent properties.

**Principle 3 — Concrete State Over Abstract Scores**: Danger comes from actual threats, not `danger_score`. Scarcity comes from inventories and unmet needs, not `scarcity_score`. Abstract summaries are allowed only as derived views/caches, never as source of truth.

**Principle 4 — Persistent Identity, Object Permanence, and Explicit Transfer**: Every meaningful thing has stable identity. Movement, splitting, consumption, creation, and destruction must be explicit. For conserved quantities, every change must have an explicit source/sink path.

### II. World Dynamics

**Principle 5 — Simulate Carriers of Consequence, Not Decorative Realism**: Model only what propagates downstream effects. Fidelity comes from consequence density, not from subsystem count.

**Principle 6 — World Runs Without Observers**: No Schrodinger's NPCs. No frozen towns. The simulation continues meaningfully without human intervention.

**Principle 7 — Locality of Motion, Interaction, and Communication**: All interaction requires co-location or explicit range. All communication requires a physical carrier (witness, letter, messenger, tracks). Agents may not query global truth on behalf of a character.

**Principle 8 — Every Action Has Preconditions, Duration, Cost, and Occupancy**: Nothing important is free or instantaneous. Long actions unfold over time and are interruptible. Contested affordances require explicit resolution (queue, grant, race).

**Principle 9 — Outcomes Are Granular and Leave Aftermath**: Actions are not binary success/fail. They create partial outcomes, side effects, and future hooks. Failure is new state.

**Principle 10 — Every Positive Feedback Loop Needs a Physical Dampener**: Never solve runaway loops with invisible caps. The dampener must be a world mechanism (resource exhaustion, competition, fatigue).

**Principle 11 — Performance May Compress Computation, Never Causality**: Optimization may change how the machine computes, never what the world means.

### III. Knowledge, Belief, and Evidence

**Principle 12 — World State Is Not Belief State**: Agents act on beliefs, not truth. A planner may consult only the agent's accessible belief state. No AI may silently use omniscient world data. *This is the most AI-relevant principle.*

**Principle 13 — Knowledge Is Acquired Locally and Travels Physically**: Knowledge enters through perception, testimony, documents, traces. It travels with delay, distortion, and possible loss. Beliefs carry provenance, acquisition time, and confidence.

**Principle 14 — Ignorance, Uncertainty, and Contradiction Are First-Class**: Agents can not-know, suspect, misremember, hold stale beliefs, and believe contradictory reports.

**Principle 15 — Surprise Comes From Violated Expectation**: Agents discover mismatch between belief and observation, not "missing things" globally.

**Principle 16 — Memory, Evidence, and Records Are World State**: Memories, contracts, debts, warrants are entities that can be created, destroyed, forged, or contested.

### IV. Agents, Institutions, and Social Order

**Principle 17 — Agent Symmetry**: No rule distinction between human and AI agents. `ControlSource` changes only who chooses, never what reality allows.

**Principle 18 — Resource-Bounded Practical Reasoning Over Scripts**: Decisions must be explainable as "Agent X chose Y because they believed Z and cared about Q." Goals name desired world conditions, not one-step solutions.

**Principle 19 — Intentions Are Revisable Commitments**: Plans reserve nothing unless the world contains an explicit reservation. Agents monitor assumptions and abandon intentions when evidence invalidates them.

**Principle 20 — Agent Diversity Through Concrete Variation**: Agents differ in needs, skills, values, loyalties, courage, patience, perception fidelity. These come from concrete per-agent parameters.

**Principle 21 — Roles, Offices, and Institutions Are World State**: Authority is a socially recognized role with jurisdiction, duties, limits, succession rules. Institutions act through agents and rules, not omniscient manager code.

**Principle 22 — Ownership, Custody, Access, Obligation, and Jurisdiction Are Distinct**: Possession ≠ ownership ≠ permission ≠ capability. These distinctions apply to organizations and places.

**Principle 23 — Social Artifacts Are First-Class**: Bounties, contracts, debts, accusations, rumors, warrants are world entities, not UI abstractions.

### V. System Architecture

**Principle 24 — Systems Interact Through State, Not Through Each Other**: Systems read world state and write new state. Influence travels through state mutation and event history, not cross-system calls.

**Principle 25 — Derived Summaries Are Caches, Never Truth**: Threat maps, route advisories, inventory summaries must be derived from source state and invalidated when it changes.

**Principle 26 — No Backward Compatibility in Live Authority Paths**: Do not preserve dead abstractions. When design changes, the live path changes with it.

**Principle 27 — Debuggability Is a Product Feature**: The simulation must support "Why did this agent do that?" from state, beliefs, records, and causal history.

**Principle 28 — Every New System Spec Must Declare Its Causal Hooks**: Entities, relations, actions, information flow, conservation, contention, failures, feedback loops, dampeners, derived views, and save/load requirements.

### Canonical Regression Scenarios

The architecture must support these emergent chains from general-purpose systems (not special-case code):

- **A**: Beast starvation → caravan attack → report → bounty → hunt → reward
- **B**: Hungry agent → market trip → dragon attack → interrupted plan → retreat
- **C**: Stored gold → empty stash → discovery → robbery report
- **D**: Rumor → travel → empty source → discovery → belief correction → replan
- **E**: Competing claimants → queue/race → expiry/prune → next actor acts

### Key Determinism Requirements

- `ChaCha8Rng` seeded randomness only
- `BTreeMap`/`BTreeSet` only in authoritative state (no `HashMap`/`HashSet`)
- No floats — `Permille` (0..=1000) for all [0,1] ranges
- No wall-clock time
- Identical seed + identical inputs → identical state hashes

---

## 2. Project Context and Simulation Architecture

### Crate Workspace

5 crates with strict dependency direction:

```
worldwake-core    → IDs, types, ECS store, topology, items, relations (no deps)
worldwake-sim     → Event log, action framework, scheduler, replay (deps: core)
worldwake-systems → Needs, production, trade, combat, travel actions (deps: core, sim)
worldwake-ai      → GOAP planner, goal ranking, decision runtime (deps: core, sim, systems)
worldwake-cli     → Human control interface (deps: all)
```

### Custom ECS

- Deterministic `BTreeMap`-based typed component storage (no external ECS crate)
- Generational slot allocator for `EntityId` (slot + generation)
- `ComponentTables` with macro-generated per-type storage
- `RelationTables` for placement, ownership, reservation, social relations

### World Topology

The world is a **place graph** with named locations and travel edges (not continuous space). Agents move between discrete places via travel actions with duration proportional to edge weight. Dijkstra pathfinding provides shortest-path queries.

Key places in the prototype world: VillageSquare, OrchardFarm, RulersHall, PublicLatrine, GrainField, Marketplace, ForgeWorkshop, GravePlot, RiverBend.

### Action Framework

Every action in the simulation is defined by an `ActionDef` with:

- **Preconditions**: What must be true for the action to start
- **Duration**: How many ticks the action takes
- **Interruptibility**: `NonInterruptible`, `InterruptibleWithPenalty`, `FreelyInterruptible`
- **Targets**: Entities the action operates on
- **Payload**: Domain-specific data (trade terms, recipe inputs, combat parameters)
- **Reservations**: Exclusive access claims on resources

Actions progress through a lifecycle: Start → Active (tick-by-tick) → Committed or Aborted. The scheduler manages active actions and delivers completion/failure signals.

### Belief Boundary

The AI reads agent beliefs through two trait interfaces:

- `GoalBeliefView`: Provides believed entity states, homeostatic needs, drive thresholds, utility profiles, combat profiles, known recipes, institutional beliefs, social profiles, wound lists, facility data
- `RuntimeBeliefView`: Provides runtime-level queries (replan signals, active actions, committed actions)

Currently, an `OmniscientBeliefView` delegates everything to `&World` — agents are functionally omniscient until Epic 14 (Perception) implements per-agent belief stores. The architectural boundary exists but the information restriction does not yet.

### Tick System Execution Order

Systems run each tick in this order:

```
Needs → Production → Trade → Combat → FacilityQueue → Politics → Perception
```

The AI runs **before** these systems, during the `produce_tick_inputs()` phase. The AI generates action requests, which the scheduler processes before passive systems run.

### Append-Only Event Log

The event log is the causal source of truth. Every state mutation produces an `EventRecord` with:
- `EventId` and `Tick` (when)
- `CauseRef` (links to prior events)
- `ComponentDelta` and `RelationDelta` (what changed)
- `WitnessData` (who observed it)
- `EventTag` (classification)

Events are never mutated after creation.

---

## 3. AI Decision Architecture — Overview

### Pipeline Flow

Each tick, for each AI-controlled agent, `process_agent()` runs the following pipeline:

```
ENTRY: process_agent(agent, replan_signals, committed_actions)
│
├─ DEAD CHECK: If agent is dead → clear frame, return
│
├─ RECONCILIATION: Process completed/failed actions
│  ├─ Advance completed step index
│  ├─ Record step failures as blockers
│  └─ Process replan signals
│
├─ FACILITY QUEUE: Check patience exhaustion on queue positions
│
├─ ASSUMPTION EVALUATION (pre-planning):
│  └─ Evaluate intention frame assumptions (except NoCriticalThreat)
│
├─ READ PHASE: Refresh runtime for planning
│  ├─ CANDIDATE GENERATION: scan beliefs → GroundedGoal[]
│  ├─ RANKING: priority class + motive score → sorted RankedGoal[]
│  ├─ BLOCKER CLEANUP: Clear expired BlockedIntents
│  └─ SNAPSHOT DIRTINESS: Detect if beliefs changed
│
├─ DEFERRED ASSUMPTION EVAL: NoCriticalThreat (needs ranked candidates)
│
├─ FEASIBILITY ANNOTATION: Reorder by feasibility hints
│
├─ BRANCH: Active Action vs Planning
│  │
│  ├─ ACTIVE ACTION PATH (action is running):
│  │  ├─ Evaluate interrupt triggers
│  │  ├─ Optionally build candidate plans (if FreelyInterruptible)
│  │  └─ Decision: NoInterrupt | InterruptForReplan
│  │
│  └─ PLANNING PATH (no active action):
│     ├─ Build candidate plans (GOAP search for top-N goals)
│     ├─ Select best plan (priority/margin/cost)
│     ├─ Revalidate next step against current affordances
│     ├─ Resolve materialization bindings
│     └─ Enqueue step → scheduler.input_queue
│
├─ PATIENCE CHECK: Increment stalled ticks, check exhaustion
│
├─ FRAME TRANSITIONS: Record Created/Progressed/Suspended/Exhausted/Cleared
│
├─ PERSISTENCE: Write intention frame, active goal, blocked memory, queue intents
│
└─ OUTPUT: AgentDecisionTrace { agent, tick, outcome }
```

### Module Inventory (Source Lines)

| Module | Lines | Purpose |
|--------|------:|---------|
| `candidate_generation.rs` | 9,735 | Goal evidence collection from beliefs |
| `goal_model.rs` | 6,190 | GoalKindPlannerExt trait + 21 implementations |
| `agent_tick/tests.rs` | 5,030 | Unit tests for agent tick driver |
| `search/tests.rs` | 5,954 | Unit tests for plan search |
| `planning_state.rs` | 3,907 | Copy-on-write immutable planning simulation |
| `ranking.rs` | 3,145 | Priority classification + motive scoring |
| `decision_trace.rs` | 2,675 | Structured decision tracing |
| `planner_ops.rs` | 1,990 | Action semantics for planner |
| `planning_snapshot.rs` | 1,699 | Read-only belief state snapshot |
| `failure_handling.rs` | 1,685 | Plan failure recovery |
| `exhaustion.rs` | 1,505 | Exhausted goal caching |
| `agent_tick/frame.rs` | 1,151 | Intention frame lifecycle |
| `agent_tick/planning.rs` | 1,097 | Plan building orchestration |
| `interrupts.rs` | 1,013 | Interrupt evaluation |
| `feasibility.rs` | 917 | Cheap pre-search feasibility hints |
| `agent_tick/mod.rs` | 897 | Per-tick orchestrator |
| `plan_revalidation.rs` | 879 | Step revalidation |
| `plan_selection.rs` | 720 | Best plan selection |
| `decision_runtime.rs` | 709 | Per-agent persistent state |
| `agent_tick/observation.rs` | 510 | Snapshot dirtiness detection |
| `enterprise.rs` | 498 | Merchant restock signals |
| `pressure.rs` | 493 | Danger/pain pressure derivation |
| `search/candidates.rs` | 527 | Search candidate generation |
| `agent_tick/execution.rs` | 413 | Step enqueueing |
| `search/mod.rs` | 301 | A* search core |
| `agent_tick/active_action.rs` | 286 | Active action handling |
| `dirty_set.rs` | 296 | Change tracking bitmask |
| `shared_collections.rs` | 247 | Copy-on-write containers |
| `search/transition.rs` | 169 | Hypothetical state transitions |
| `search/heuristic.rs` | 159 | A* heuristic (travel distance) |
| `goal_switching.rs` | 137 | Margin-based switching logic |
| `planner_duration_contract.rs` | 109 | Duration dependency modeling |
| `goal_policy.rs` | ~200 | Suppression rules per goal family |
| `goal_explanation.rs` | ~150 | Human-readable goal explanations |
| `frame_switch_policy.rs` | 105 | Frame-aware switching margins |
| `knowledge_path.rs` | 96 | Belief provenance tracing |
| `theft.rs` | 85 | Theft deterrence assessment |
| `agent_tick/candidates.rs` | 73 | Queue intent abandonment |
| `budget.rs` | 65 | Planning budget defaults |
| `search/frontier.rs` | 50 | Binary heap frontier |
| `institutional_queries.rs` | 41 | Office record queries |
| **Total source** | **~56,941** | (including tests) |

---

## 4. Deep-Dive Per Module

### 4.1 Goal Identity: `GoalKind`, `GoalKey`, `GoalKindTag`

**Location**: `worldwake-core/src/goal.rs` (authoritative), `worldwake-ai/src/goal_model.rs` (planner extensions)

The goal system has three layers of identity:

**`GoalKind`** — the full goal enum with 21 variants:

```rust
pub enum GoalKind {
    ConsumeOwnedCommodity { commodity: CommodityKind },
    AcquireCommodity { commodity: CommodityKind, purpose: CommodityPurpose },
    Sleep,
    Relieve,
    Wash,
    EngageHostile { target: EntityId },
    ReduceDanger,
    TreatWounds { patient: EntityId },
    ProduceCommodity { recipe_id: RecipeId },
    SellCommodity { commodity: CommodityKind },
    RestockCommodity { commodity: CommodityKind },
    MoveCargo { commodity: CommodityKind, destination: EntityId },
    LootCorpse { corpse: EntityId },
    BuryCorpse { corpse: EntityId, burial_site: EntityId },
    ShareBelief { listener: EntityId, topic: TellTopic },
    ClaimOffice { office: EntityId },
    SupportCandidateForOffice { office: EntityId, candidate: EntityId },
    InvestigateViolation { violation_id: ViolationId, place: EntityId },
    StealItem { target_item: EntityId },
    Accuse { crime_register: EntityId, accused: EntityId, violation_id: ViolationId },
    PunishAccused { office: EntityId, accused: EntityId, accusation_entry: RecordEntryId, punishment: PunishmentKind },
}
```

**`GoalKey`** — canonical identity for deduplication:

```rust
pub struct GoalKey {
    pub kind: GoalKind,
    pub commodity: Option<CommodityKind>,
    pub entity: Option<EntityId>,
    pub place: Option<EntityId>,
}
```

`GoalKey` extracts canonical fields from `GoalKind` for identity comparison. Two `AcquireCommodity(Apple, SelfConsume)` goals with different source locations share the same `GoalKey`.

**`GoalKindTag`** — simplified enum for matching (no fields):

```rust
pub enum GoalKindTag {
    ConsumeOwnedCommodity, AcquireCommodity, Sleep, Relieve, Wash,
    EngageHostile, ReduceDanger, TreatWounds, ProduceCommodity,
    SellCommodity, RestockCommodity, MoveCargo, LootCorpse, BuryCorpse,
    ShareBelief, ClaimOffice, SupportCandidateForOffice,
    InvestigateViolation, StealItem, Accuse, PunishAccused,
}
```

**Coupling note**: Adding a new `GoalKind` variant requires updating all three types plus at least 5 modules (candidate_generation, goal_model, planner_ops, exhaustion, failure_handling). This is the primary extensibility bottleneck.

### 4.2 Candidate Generation (9,735 lines)

**Purpose**: Scans agent beliefs to produce goal candidates with evidence trails.

**Input**: `GoalBeliefView` (agent beliefs), agent ID, utility profile, known recipes, blocked memory, conversation memory, social profile

**Output**: `Vec<GroundedGoal>` where:

```rust
pub struct GroundedGoal {
    pub key: GoalKey,
    pub evidence_entities: BTreeSet<EntityId>,
    pub evidence_places: BTreeSet<EntityId>,
}
```

**Process**: For each of the 21 `GoalKind` variants, the module contains a dedicated code path that:
1. Queries beliefs for relevant entities (inventory, hostiles, wounded agents, trade partners, offices, etc.)
2. Filters by evidence requirements (e.g., care goal requires direct observation of wounds, not rumor)
3. Filters by blocked intent memory (skip recently blocked goals)
4. Creates `GroundedGoal` with discovered evidence

**Examples**:
- `ConsumeOwnedCommodity`: Scans inventory for edible/drinkable commodities, emits one goal per commodity kind
- `AcquireCommodity`: Scans for commodity shortages (need threshold exceeded), finds sources (markets, ground lots, production chains)
- `ShareBelief`: Scans for co-located agents who haven't been told recently, filters by tell profile and chain length
- `ClaimOffice`: Scans believed office vacancies where agent meets eligibility rules
- `InvestigateViolation`: Scans for unresolved violation records the agent has authority to investigate

**Complexity note**: This single file has ~163 occurrences of `GoalKind::` match patterns. The per-goal-kind code paths range from 20 lines (Sleep) to 200+ lines (ClaimOffice, AcquireCommodity).

### 4.3 Ranking (3,145 lines)

**Purpose**: Sort goal candidates by priority class and motive score.

**Priority classes** (highest to lowest):

```rust
pub enum GoalPriorityClass {
    Critical,    // Immediate threats: attacker present, incapacitated, starving
    High,        // Serious unmet needs: hungry, thirsty, wounded
    Medium,      // Moderate drive pressure
    Low,         // Weak interest
    Background,  // Purely opportunistic (gossip, theft if safe)
}
```

**Process**:

1. **Build `DecisionContext`**: Derives danger pressure (from attackers/wounds) and self-care pressure (from homeostatic needs) using the agent's `DriveThresholds` bands.

2. **Suppression check**: `GoalFamilyPolicy` defines which goals are suppressed under stress. Non-critical goals suppressed when self-care or danger is High+.

3. **Priority class assignment**: Each drive (hunger, thirst, fatigue, bladder, dirtiness) maps to a priority class via `ThresholdBand` comparison against `DriveThresholds`. Enterprise goals use `enterprise_weight * signal` as motive.

4. **Motive score**: 0-1000 Permille value. For drives: `need_level * utility_weight`. For enterprise: `gap_signal * enterprise_weight`. For combat: `danger_pressure`. For social: `social_weight`.

5. **Sort**: Primary key = priority class (descending). Secondary key = motive score (descending).

**Output**:

```rust
pub struct RankingOutcome {
    pub ranked: Vec<RankedGoal>,
    pub suppressed: Vec<GoalKey>,
    pub zero_motive: Vec<GoalKey>,
}
```

Where `RankedGoal` contains:

```rust
pub struct RankedGoal {
    pub grounded: GroundedGoal,
    pub priority_class: GoalPriorityClass,
    pub motive_score: Permille,  // 0-1000
    pub provenance: Option<RankedGoalProvenance>,
    pub feasibility: FeasibilityHint,
}
```

### 4.4 GoalKindPlannerExt Trait (6,190 lines)

**Purpose**: Defines planning semantics for each `GoalKind`.

**Trait signature**:

```rust
pub trait GoalKindPlannerExt {
    fn goal_kind_tag(&self) -> GoalKindTag;
    fn relevant_op_kinds(&self) -> &'static [PlannerOpKind];
    fn relevant_observed_commodities(&self, recipes: &RecipeRegistry) -> Option<BTreeSet<CommodityKind>>;
    fn build_payload_override(
        &self, affordance_payload: Option<&ActionPayload>,
        state: &PlanningState, targets: &[EntityId],
        def: &ActionDef, semantics: &PlannerOpSemantics,
    ) -> Result<Option<ActionPayload>, GoalPayloadOverrideError>;
    fn apply_planner_step<'snapshot>(
        &self, state: PlanningState<'snapshot>,
        op_kind: PlannerOpKind, targets: &[EntityId],
        payload_override: Option<&ActionPayload>,
    ) -> PlanningState<'snapshot>;
    fn is_progress_barrier(&self, step: &PlannedStep) -> bool;
    fn is_satisfied(&self, state: &PlanningState) -> bool;
    fn goal_relevant_places(&self, state: &PlanningState, recipes: &RecipeRegistry) -> Vec<EntityId>;
    fn prerequisite_places(&self, state: &PlanningState, recipes: &RecipeRegistry, budget: &PlanningBudget) -> Vec<EntityId>;
    fn matches_binding(&self, authoritative_targets: &[EntityId], op_kind: PlannerOpKind) -> bool;
}
```

Each of the 21 `GoalKind` variants implements this trait. Examples:

- `ConsumeOwnedCommodity`: relevant ops = [Consume, Travel, MoveCargo]. Satisfied when agent has consumed the commodity (inventory updated in planning state). Goal-relevant places = agent's current location.
- `AcquireCommodity`: relevant ops = [Travel, Trade, QueueForFacilityUse, Harvest, Craft, MoveCargo]. Satisfied when inventory contains commodity. Goal-relevant places = markets, resource sources.
- `TreatWounds`: relevant ops = [Travel, Heal, Harvest, Trade, MoveCargo, QueueForFacilityUse, Craft]. Satisfied when patient's wound load decreases. Prerequisite places = medicine sources.

**Op-kind to goal-kind mappings**: Static arrays define which op kinds serve which goal kinds:

```rust
const CONSUME_OPS: &[PlannerOpKind] = &[Consume, Travel, MoveCargo];
const ACQUIRE_OPS: &[PlannerOpKind] = &[Travel, Trade, QueueForFacilityUse, Harvest, Craft, MoveCargo];
const SLEEP_OPS: &[PlannerOpKind] = &[Sleep, Travel];
const ENGAGE_HOSTILE_OPS: &[PlannerOpKind] = &[Attack];
// ... 21 total
```

### 4.5 Planner Operations (1,990 lines)

**Purpose**: Maps action definitions to planning semantics.

**`PlannerOpKind`** — 25 operation types:

```rust
pub enum PlannerOpKind {
    Travel, Consume, Sleep, Relieve, Wash, Trade,
    QueueForFacilityUse, Harvest, Craft, MoveCargo,
    Heal, Loot, Bury, Tell, ConsultRecord,
    Attack, Defend, Bribe, Threaten,
    Accuse, Fine, Exile, DeclareSupport,
    PressForceClaim, YieldForceClaim, Investigate,
}
```

**`PlannerOpSemantics`** — per-action-def planning metadata:

```rust
pub struct PlannerOpSemantics {
    pub op_kind: PlannerOpKind,
    pub may_appear_mid_plan: bool,         // Can this op appear after depth 0?
    pub is_materialization_barrier: bool,   // Does this op create new entities?
    pub transition_kind: PlannerTransitionKind,
    pub relevant_goal_kinds: &'static [GoalKindTag],  // Which goals can use this op?
}
```

`build_semantics_table()` maps every `ActionDefId` to its `PlannerOpSemantics`, creating the lookup table used by the search. The semantics cache is invalidated when action definitions change.

`apply_hypothetical_transition()` applies a planned step to a `PlanningState`, mutating entity locations, commodity quantities, facility states, beliefs, etc.

### 4.6 Planning Snapshot (1,699 lines)

**Purpose**: Immutable read-only snapshot of world state for planning.

`PlanningSnapshot` is constructed once per tick per agent and captures:
- All entity states relevant to the agent (positions, inventory, needs, wounds, combat stance)
- Believed entity states for subjects the agent knows about
- Topology (travel edges, distances)
- Facility states (workstations, queue positions, grants)
- Institutional beliefs (office holders, support declarations, force controllers)

**`SnapshotEntity`** has ~40+ fields per entity. The snapshot is built by `build_planning_snapshot()` which copies all entities within the `snapshot_travel_horizon` (default: 6 hops).

**Concern**: Full entity state is copied regardless of goal kind. A Sleep goal only needs the agent's location and fatigue level, but receives 40+ fields for every visible entity.

### 4.7 Planning State (3,907 lines)

**Purpose**: Copy-on-write mutable state for GOAP search simulation.

`PlanningState<'snapshot>` wraps an immutable `&PlanningSnapshot` and accumulates overrides:

```rust
pub struct PlanningState<'snapshot> {
    snapshot: &'snapshot PlanningSnapshot,
    entity_place_overrides: SharedMap<PlanningEntityRef, Option<EntityId>>,
    container_overrides: SharedMap<PlanningEntityRef, Option<EntityId>>,
    possessor_overrides: SharedMap<PlanningEntityRef, Option<EntityId>>,
    commodity_quantity_overrides: SharedMap<(PlanningEntityRef, CommodityKind), Quantity>,
    needs_overrides: SharedMap<PlanningEntityRef, HomeostaticNeeds>,
    pain_pressure_overrides: SharedMap<PlanningEntityRef, Permille>,
    facility_queue_membership_overrides: SharedMap<EntityId, Option<HypotheticalQueueJoin>>,
    hypothetical_registry: SharedMap<HypotheticalEntityId, HypotheticalEntityMeta>,
    institutional_belief_overrides: SharedMap<(PlanningEntityRef, EntityId), InstitutionalOverrideValue>,
    consumed_lot_overrides: SharedSet<PlanningEntityRef>,
    wound_count_overrides: SharedMap<PlanningEntityRef, usize>,
    social_overrides: SharedMap<(PlanningEntityRef, PlanningEntityRef), SocialOverrideValue>,
    // + more override maps
}
```

**Copy-on-write semantics**: `SharedMap<K,V>` is backed by `Rc<BTreeMap<K,V>>`. When a search node needs to modify a map, it clones the Rc (cheap if shared) or mutates in place (if uniquely owned). This avoids full state copying on every A* expansion.

**Hypothetical entities**: When a plan creates items (harvesting apples, crafting bread), the planner tracks them as `HypotheticalEntityId`. Only after the action commits do `MaterializationBindings` map hypothetical IDs to real `EntityId`s.

**`PlanningEntityRef`**: Either `Authoritative(EntityId)` for real entities or `Hypothetical(HypotheticalEntityId)` for planned entities. All state queries check overrides first, then fall back to the snapshot.

### 4.8 Search — A* GOAP (search/ ~1,200 lines)

**Purpose**: Best-first search over planning states to find multi-step action plans.

**Core type**:

```rust
struct SearchNode<'snapshot> {
    state: PlanningState<'snapshot>,
    steps: SharedVec<PlannedStep>,
    total_estimated_ticks: u32,
    heuristic_ticks: u32,  // A* heuristic: min travel ticks to goal-relevant place
}
```

**Budget defaults** (from `PlanningBudget`):

```rust
PlanningBudget {
    max_candidates_to_plan: 2,      // Plan top-2 ranked goals per tick
    max_plan_depth: 8,              // Max steps in a plan
    snapshot_travel_horizon: 6,     // How many hops from agent to include
    max_prerequisite_locations: 3,  // Max prerequisite locations to consider
    max_node_expansions: 224,       // A* node expansion budget
    beam_width: 8,                  // Max successors kept per expansion
    switch_margin_permille: 100,    // 10% motive margin for goal switching
    transient_block_ticks: 20,      // TTL for transient blockers
    unknown_block_ticks: 5,         // TTL for unknown blockers
    structural_block_ticks: 200,    // TTL for structural blockers
}
```

**Search algorithm**:

1. Create root node from planning snapshot
2. Push root onto binary heap (priority = estimated_ticks + heuristic_ticks)
3. Pop lowest-cost node
4. If `goal.is_satisfied(node.state)` → return `Found(plan)`
5. If depth >= `max_plan_depth` → skip node
6. If expansions >= `max_node_expansions` → return barrier plan or `BudgetExhausted`
7. Generate search candidates (valid actions for this state)
8. Prune travel away from goal-relevant places
9. Build successors via `apply_hypothetical_transition()`
10. Separate terminal successors (GoalSatisfied, CombatCommitment, ProgressBarrier)
11. Sort non-terminal successors by cost, truncate to `beam_width`
12. Push to frontier
13. Repeat until frontier empty → `FrontierExhausted`

**Result**:

```rust
pub enum PlanSearchResult {
    Found(PlannedPlan),
    Unsupported,
    BudgetExhausted { expansions_used: u16 },
    FrontierExhausted { expansions_used: u16 },
}
```

**Terminal kinds**: `GoalSatisfied` (plan fully satisfies goal), `ProgressBarrier` (plan reaches materialization barrier — e.g., Trade creates items that don't exist yet), `CombatCommitment` (combat commitment reached).

**Materialization barriers**: Some actions create new entities (Harvest creates apple lots, Trade creates sold goods). The planner cannot simulate past these because the real entities don't exist yet. The search returns a `ProgressBarrier` plan and execution handles the rest step by step.

### 4.9 Intention Frames (agent_tick/frame.rs, 1,151 lines)

**Purpose**: Per-agent commitment tracking with monitored assumptions.

```rust
// From worldwake-core:
pub struct IntentionFrame {
    pub state: FrameState,          // Active, Suspended, Exhausted
    pub goal: GoalKey,
    pub domain: IntentionDomain,    // Travel, Care, Errand, Generic
    pub destination: Option<EntityId>,
    pub established_at: Tick,
    pub last_progress_tick: Option<Tick>,
    pub stalled_ticks: u32,
    pub patience_limit: u32,
    pub assumptions: Vec<FrameAssumption>,
}
```

**Frame states**:
- `Active`: Pursuing the goal with a valid plan
- `Suspended`: Goal still desired but current plan blocked; waiting for conditions to change
- `Exhausted`: Patience limit reached; goal will be blocked for some TTL

**Assumptions**: Each frame carries assumptions that, if violated, trigger replanning:
- `NoCriticalThreat`: No critical danger detected
- `TargetAlive(EntityId)`: Target entity still alive
- `StillOwned(CommodityKind)`: Agent still possesses the commodity
- `DestinationReachable(EntityId)`: Destination exists and is reachable

**Patience exhaustion**: If `stalled_ticks >= patience_limit`, the frame is marked Exhausted and a `BlockedIntent` is recorded. Different domains have different patience limits (Travel frames are more patient than Generic frames).

**Frame-plan relation**: When a new plan is selected, it is classified as:
- `RefreshesFrame`: Same goal, continue frame
- `SuspendsFrame`: Different goal, suspend frame for later
- `AbandonsFrame`: Incompatible goal, clear frame

### 4.10 Interrupts (1,013 lines)

**Purpose**: Determine when a running action should be interrupted for replanning.

**Interrupt levels** (from action definition):
- `NonInterruptible`: Never interrupt (some combat actions)
- `InterruptibleWithPenalty`: Only interrupt for Critical goals
- `FreelyInterruptible`: Can be interrupted by any superior goal

**Interrupt triggers**:

```rust
pub enum InterruptTrigger {
    CriticalSurvival,       // Incapacitated, starving
    CriticalDanger,         // Active attacker
    HigherPriorityGoal,     // Higher priority class with plan
    SuperiorSameClassPlan,  // Same class but better motive
    PlanInvalid,            // Current plan can't continue
    OpportunisticLoot,      // Corpse available, no stress
}
```

**Decision logic**: For `FreelyInterruptible` actions, the system evaluates ranked candidates and their plans. The `GoalFamilyPolicy` determines each goal kind's interrupt eligibility and role (Reactive vs Opportunistic). Margin-based switching prevents thrashing: challenger must exceed current motive by `switch_margin_permille` (default 10%).

### 4.11 Failure Handling (1,685 lines)

**Purpose**: Recover from plan step failures.

When a planned step fails (action can't start, precondition violated, target unavailable), `handle_plan_failure()`:

1. Derives a `BlockingFact` from the failure context:
   - `MissingPrerequisite`: Required commodity/item not available
   - `TargetUnavailable`: Target entity not at expected location
   - `Unknown`: Unclassified failure

2. Records a `BlockedIntent` with TTL:
   - Transient: 20 ticks (retryable conditions like target moved)
   - Unknown: 5 ticks (unclear what went wrong)
   - Structural: 200 ticks (persistent barriers like missing facility)

3. Clears the current plan and marks the runtime for replanning

**`BlockedIntentMemory`**: Per-agent `BTreeMap<BlockerKey, BlockedIntent>` where `BlockerKey = (GoalKey, Option<EntityId>)`. Candidate generation checks this memory to skip recently blocked goals.

### 4.12 Exhaustion (1,505 lines)

**Purpose**: Cache goals that exhausted the search budget to avoid repeating expensive searches.

**`ExhaustionEntry`**:

```rust
pub struct ExhaustionEntry {
    pub retry_state: ExhaustionRetryState,  // FrontierExhausted | BudgetRetryPending
    pub invalidation_conditions: Vec<ExhaustionInvalidationCondition>,
    pub baseline: ExhaustionBaseline,
    pub consecutive_budget_exhaustions: u8,
}
```

**Invalidation conditions** (per goal kind):

```rust
pub enum ExhaustionInvalidationCondition {
    PositionChanged,
    CommodityChanged(CommodityKind),
    UniqueItemChanged(UniqueItemKind),
    WoundsChanged,
    FacilitiesChanged,
    BlockerExpired,
    HostilesChanged,
    NeedChangedBands { need: HomeostaticNeedId, band: ThresholdBand },
    TargetDead(EntityId),
}
```

Each `GoalKind` variant hand-declares which conditions would make a re-search worthwhile. Example: `AcquireCommodity(Apple)` invalidates on `PositionChanged` or `CommodityChanged(Apple)`.

**Exponential backoff**: `effective_max_expansions = base >> consecutive_budget_exhaustions` (floor 16). So after 1 exhaustion: 112 nodes; after 2: 56; after 3: 28; after 4+: 16. Reset to full budget when a plan is found or an invalidation condition fires.

### 4.13 Decision Runtime (709 lines)

**Purpose**: Per-agent persistent state across ticks.

```rust
pub struct AgentDecisionRuntime {
    pub current_plan: Option<PlannedPlan>,
    pub current_step_index: usize,
    pub step_in_flight: bool,
    pub materialization_bindings: MaterializationBindings,
    pub exhaustion_cache: BTreeMap<GoalKey, ExhaustionEntry>,
    pub dirty: DirtySet,
    // + observation snapshots for dirtiness detection
}
```

**`DirtySet`**: Bitmask flags tracking what changed:
- `SNAPSHOT_MASK`: World beliefs changed (commodities, hostiles, institutions)
- `FRAME_MASK`: Frame structure changed
- `REPLAN_SIGNAL`: External replan request
- `NO_PLAN`: No current plan
- `PLAN_FINISHED`: Current plan's goal satisfied
- `ASSUMPTION_FAILED`: Frame assumption failed

**`MaterializationBindings`**: Maps hypothetical entity IDs from planning to real entity IDs after action execution. When a Harvest action creates an apple lot, the binding resolves `Hypothetical(0)` → `EntityId(42, 0)`.

### 4.14 Pressure (493 lines)

**Purpose**: Derive danger and pain pressure from beliefs.

**`DangerAssessment`**:
- Counts active attackers, visible hostiles
- Computes wound-based incapacitation risk
- Produces `danger_pressure: Permille` (0-1000)
- Classifies into threshold bands using agent's `DriveThresholds`

**Pain pressure**: Derived from total wound severity. Used for priority class promotion (wound_load > threshold → Critical self-care).

### 4.15 Goal Switching (137 lines)

**Purpose**: Margin-based goal switching to prevent thrashing.

```rust
pub fn compare_goal_switch(
    current: &RankedGoal,
    challenger: &RankedGoal,
    margin: Permille,
) -> Option<GoalSwitchKind>
```

Rules:
1. If challenger's priority class > current's → always switch (`HigherPriorityClass`)
2. If same class and challenger.motive >= current.motive + margin → switch (`SuperiorSameClass`)
3. Otherwise → no switch

The margin prevents oscillation between goals with similar motives.

### 4.16 Feasibility (917 lines)

**Purpose**: Cheap pre-search annotation to reorder candidates.

**`FeasibilityHint`**: `Likely`, `Uncertain`, `Unlikely`

Checks (without running full GOAP search):
- Is this goal in blocked memory? → Unlikely
- Is there an exhaustion cache entry? → Unlikely
- Is the goal kind spatially reachable? → Uncertain if far, Likely if near
- Goal-specific checks (e.g., medicine available for TreatWounds)

Feasibility never excludes goals. It only reorders within the same priority class, so the most feasible goals are planned first (reducing wasted budget).

### 4.17 Decision Trace (2,675 lines)

**Purpose**: Full structured tracing of the decision pipeline for debugging.

```rust
pub struct AgentDecisionTrace {
    pub agent: EntityId,
    pub tick: Tick,
    pub outcome: DecisionOutcome,
}

pub enum DecisionOutcome {
    Dead,
    ActiveAction { action_def_id: ActionDefId, interrupt: InterruptTrace, frame_transition: ... },
    Planning(PlanningPipelineTrace),
}

pub struct PlanningPipelineTrace {
    pub candidates: CandidateTrace,
    pub planning: PlanSearchTrace,
    pub selection: SelectionTrace,
    pub execution: ExecutionTrace,
    pub action_start_failures: Vec<ActionStartFailureSummary>,
}
```

Tracing is opt-in via `driver.enable_tracing()` and zero-cost when disabled. Golden tests use traces extensively to verify that the correct goals were generated, ranked, planned, and selected.

### 4.18 Enterprise (498 lines)

**Purpose**: Derives merchant restock signals for trading goals.

Computes the "restock gap" for each commodity a merchant sells: `gap = max_stock - current_stock`. Produces `RestockCommodity` goal candidates when gap > 0. Also computes market opportunity signals for `SellCommodity` goals based on observed demand at the home market.

---

## 5. Cross-Cutting Data Flow: Hungry Agent Acquires Food

This traces a complete decision cycle for a hungry agent with no food in inventory.

**Setup**: Agent at VillageSquare, hunger at 800/1000 (Critical threshold at 750), Apple lot at OrchardFarm, DriveThresholds with hunger Critical at 750.

**Tick N — Candidate Generation**:
1. `generate_candidates()` scans beliefs
2. Finds `HomeostaticNeeds.hunger = 800` > drive threshold → emit `ConsumeOwnedCommodity(Apple)` — but agent has no apples
3. Agent has no apples in inventory → emit `AcquireCommodity(Apple, SelfConsume)` with evidence_places = {OrchardFarm}
4. Other goals also emitted (Sleep if tired, ShareBelief if co-located agents, etc.)

**Tick N — Ranking**:
1. `build_decision_context()`: hunger 800 with critical threshold 750 → `max_self_care_class = Critical`
2. `AcquireCommodity(Apple)` ranked: priority = Critical (hunger > critical threshold), motive = `hunger_pressure * utility.hunger_weight`
3. Other goals suppressed by Critical self-care pressure (ShareBelief, ClaimOffice suppressed)
4. Result: `AcquireCommodity(Apple)` is top-ranked

**Tick N — Plan Search**:
1. Root node: agent at VillageSquare, no apples
2. Goal-relevant places: OrchardFarm (has ResourceSource for apples)
3. Heuristic: travel ticks from VillageSquare to OrchardFarm = 3
4. Expansion 1: Try Travel(OrchardFarm) → agent moves to OrchardFarm, heuristic = 0
5. Expansion 2: Try Harvest(apple_source) → `ProgressBarrier` (creates hypothetical apple lot)
6. Result: `Found(plan = [Travel(OrchardFarm), Harvest(apple_source)])` with `PlanTerminalKind::ProgressBarrier`

**Tick N — Plan Selection**:
1. Single planned candidate → selected directly
2. Frame created: `IntentionFrame { state: Active, goal: AcquireCommodity(Apple), domain: Errand }`

**Tick N — Step Execution**:
1. Step 0: Travel(OrchardFarm) → emit `InputKind::RequestAction(Travel, [OrchardFarm])`
2. Scheduler processes input → Travel action starts
3. Travel action has duration = 3 ticks

**Ticks N+1, N+2**: Travel action progresses. Each tick:
- Read phase runs (candidates re-generated, re-ranked)
- Active action path: interrupt evaluation
- AcquireCommodity still top-ranked → no interrupt
- Travel is FreelyInterruptible but no superior challenger → `NoInterrupt`

**Tick N+3**: Travel completes. `CommittedAction` delivered.
- Reconciliation: advance step index to 1
- Step 1: Harvest(apple_source)
- Revalidation: affordance check passes (agent at OrchardFarm, source has apples)
- Emit `InputKind::RequestAction(Harvest, [apple_source])`
- Harvest action starts (duration = 2 ticks)

**Tick N+5**: Harvest completes. Apple lot created (materialization).
- `MaterializationBindings.bind(Hypothetical(0), EntityId(new_apple_lot))`
- Plan finished (ProgressBarrier reached, goal partially satisfied)
- Next tick: `ConsumeOwnedCommodity(Apple)` now feasible (apples in inventory)
- New plan: `[Consume(apple_lot)]`
- Consume action starts and completes in 1 tick
- Agent hunger decreases

**If Harvest Fails** (another agent harvested first):
- `handle_plan_failure()` → `BlockingFact::MissingPrerequisite`
- `BlockedIntent` recorded with TTL = 20 ticks
- Next tick: `AcquireCommodity(Apple)` re-evaluated
- If OrchardFarm source depleted, search finds alternative (Marketplace Trade, or another farm)
- If no alternative found, `BudgetExhausted` → `ExhaustionEntry` cached

---

## 6. Golden Test Coverage Analysis

### Test Suite Summary

| File | Tests | Focus |
|------|------:|-------|
| `golden_emergent.rs` | 47 | Cross-system chains (care+combat, politics+tell, force claims) |
| `golden_combat.rs` | 25 | Combat, loot, death cascades, wound mechanics |
| `golden_offices.rs` | 22 | Political succession, force claims, eligibility |
| `golden_production.rs` | 21 | Resource contention, facility queues, conservation |
| `golden_care.rs` | 18 | Healing, medicine acquisition, care invalidation |
| `golden_ai_decisions.rs` | 16 | Goal ranking, invalidation, interrupts, spatial planning |
| `golden_social.rs` | 14 | Belief propagation, tell system, rumor chains |
| `golden_supply_chain.rs` | 11 | Multi-step production, stale belief discovery |
| `golden_determinism.rs` | 10 | Replay, save/load, runtime serialization |
| `golden_trade.rs` | 7 | Trade negotiations, restocking, fallback |
| **Total** | **191** | |

### Coverage Matrix by AI Subsystem

| Subsystem | Decisions | Care | Combat | Prod | Trade | Social | Offices | Emergent | Determ |
|-----------|:---------:|:----:|:------:|:----:|:-----:|:------:|:-------:|:--------:|:------:|
| Candidate gen | X | X | X | X | X | X | X | X | |
| Ranking | X | X | X | X | X | X | X | X | X |
| Plan search | X | X | X | X | X | | X | | |
| Plan execution | X | X | X | X | X | X | X | X | |
| Blocked intents | X | X | | | | X | X | X | |
| Exhaustion | X | | | | | | | | |
| Belief system | | X | | | | X | X | X | |
| Interrupts | X | | X | | | | | | |
| Intention frames | | | | | | | | | X |
| Serialization | | | | | | | | | X |

### Common Test Patterns

1. **Determinism pairs**: ~60 tests use the "run twice with same seed, assert identical world + event_log hashes" pattern
2. **Conservation checks**: Production tests verify `total_live_lot_quantity()` never increases
3. **Trace verification**: Tests enable decision tracing and assert on `DecisionOutcome::Planning` fields
4. **Save/load roundtrips**: Snapshot → serialize → deserialize → resume → hash equality
5. **Mid-simulation mutations**: Custom `TickInputProducer` implementations mutate world between planning and execution

### Coverage Gaps

1. **Exhaustion backoff behavior**: No golden test specifically verifies the exponential backoff (halving budget on consecutive exhaustions). Only unit tests in `exhaustion.rs`.
2. **Beam width effects**: No golden test exercises the beam_width=8 truncation and its impact on plan quality.
3. **Multi-agent contention at scale**: Most tests have 2-3 agents. No test with 10+ agents competing for the same resource.
4. **Feasibility reordering impact**: No test that would fail if feasibility hints were removed.
5. **Frame patience diversity**: Tests use default patience limits. No test verifying that agents with different patience profiles abandon frames at different rates.
6. **Exhaustion invalidation completeness**: No test verifying that a new `GoalKind` without invalidation conditions degrades correctly.

---

## 7. Known Issues and Organic Growth Patterns

### 7.1 Monolithic Files

**candidate_generation.rs** (9,735 lines) and **goal_model.rs** (6,190 lines) are the two largest files. They grow linearly with each new `GoalKind` variant. Both contain a single massive match/dispatch over all 21 variants.

**Impact**: Merge conflicts when multiple contributors touch different goal kinds. Cognitive load when navigating the file. IDE performance on large files. No compilation parallelism benefit since both are single compilation units.

### 7.2 Parallel Dispatch on GoalKind

Adding one new `GoalKind` requires updating at minimum:

| File | What to add |
|------|-------------|
| `worldwake-core/src/goal.rs` | Variant to `GoalKind`, `GoalKey::from()` mapping |
| `worldwake-ai/src/goal_model.rs` | `GoalKindTag` variant + `GoalKindPlannerExt` implementation (~50-200 lines) |
| `worldwake-ai/src/candidate_generation.rs` | Evidence collection logic (~20-200 lines) |
| `worldwake-ai/src/planner_ops.rs` | Relevant op-kind arrays, transition semantics |
| `worldwake-ai/src/exhaustion.rs` | `derive_invalidation_conditions()` match arm |
| `worldwake-ai/src/ranking.rs` | Priority class + motive scoring logic |
| `worldwake-ai/src/goal_policy.rs` | `GoalFamilyPolicy` configuration |
| `worldwake-ai/src/feasibility.rs` | Feasibility hint logic |

That is **8 files** minimum, with match-arm additions in most. There is no compile-time enforcement that all files are updated — a missing match arm in `exhaustion.rs` would silently skip invalidation conditions for the new goal.

### 7.3 `process_agent` Complexity

`process_agent()` in `agent_tick/mod.rs` orchestrates the full pipeline. It modifies 7+ mutable state variables across its ~897-line body:
- `blocked_memory`
- `current_frame` (IntentionFrame)
- `current_active_goal`
- `current_facility_intents`
- `runtime` (AgentDecisionRuntime)
- `frame_transitions` (trace collector)
- Various dirty flags

The function is annotated `#[allow(clippy::too_many_lines)]` and has multiple early-return paths with cleanup obligations. It has been decomposed into submodules (observation, planning, execution, frame, active_action) but the orchestration logic remains in a single long function.

### 7.4 Uniform Search Budget

All 21 goal kinds share the same `max_node_expansions: 224`. This means:

- **Simple goals** (Sleep, Relieve, Wash): Typically find plans in 1-3 expansions but allocate budget for 224. The search terminates quickly so this is not a runtime concern, but it masks the cost profile.
- **Complex goals** (multi-hop supply chains, remote care): Can exhaust 224 expansions. The exponential backoff reduces retries to 16 expansions floor, which may be too few for legitimately complex scenarios.
- **No per-goal tuning**: A `Sleep` goal and a `RestockCommodity` goal (which may need Travel → Harvest → Craft → Travel → Sell) get the same budget.

### 7.5 Snapshot Copying

`build_planning_snapshot()` copies ~40 fields per entity for all entities within `snapshot_travel_horizon` (6 hops). In a world with 50 entities:
- 50 * 40 fields = 2000 field copies per snapshot
- 2 snapshots per tick per agent (one per planned candidate)
- N agents per tick

For a world with 20 agents and 100 entities, this is 20 * 2 * ~4000 = ~160K field copies per tick. Most fields are unused for any given goal.

### 7.6 Ad-hoc Exhaustion Invalidation

Each `GoalKind` manually declares its invalidation conditions in `derive_invalidation_conditions()`. There is no compile-time enforcement that:
1. A new `GoalKind` registers conditions (the match has a catch-all)
2. Conditions are correct (a goal that should invalidate on `FacilitiesChanged` but doesn't will silently ignore facility state changes)
3. Conditions are complete (a goal might need a new condition type that doesn't exist yet)

### 7.7 No Learning or Adaptation

Agents have fixed utility profiles that never change. There is no mechanism for:
- Learning from past plan outcomes ("this route was dangerous")
- Adjusting preferences based on experience ("selling apples is more profitable than grain")
- Building route preferences from travel history
- Developing social preferences from interaction outcomes

Per Principle 20 (Agent Diversity), agents should develop different preferences over time through experience. Currently, diversity comes only from initial configuration.

### 7.8 No Cooperative Planning

Agents plan independently with no awareness of other agents' intentions. This leads to:
- N agents simultaneously traveling to the same scarce resource
- Multiple agents trying to heal the same patient
- Redundant tell attempts (two agents telling the same fact to the same listener)

Per Principle 19, plan intent does not reserve resources — only explicit world state does. But agents could observe others' visible actions and adjust plans accordingly (per Principle 7, locality).

### 7.9 OmniscientBeliefView Still Active

Although the architectural boundary for belief-based planning exists (`GoalBeliefView` trait), the current implementation uses `OmniscientBeliefView` which delegates all queries to `&World`. This means agents are functionally omniscient — they can see every entity, every inventory, every wound in the world.

This is a known architectural debt (planned for Epic 14: Perception). However, it means that all current golden test results will change when true perception is implemented, and some emergent behaviors tested today are artifacts of omniscience.

### 7.10 Planning State Override Map Proliferation

`PlanningState` has 12+ override maps, each requiring:
- A query method (check override → fallback to snapshot)
- A mutation method (clone-on-write the SharedMap)
- Consistency maintenance (if you move an entity, update both position and container)

Adding a new aspect of state to planning (e.g., tracking fatigue during multi-step plans) requires adding a new override map, new query/mutation methods, and updating `apply_hypothetical_transition()`.

---

## 8. Improvement Proposals

### 8.1 Goal Kind Registration Pattern

**Problem**: Adding a `GoalKind` requires editing 8+ files with no compile-time completeness check.

**Proposal**: Introduce a `GoalKindSpec` trait that bundles all per-goal-kind logic:

```rust
trait GoalKindSpec {
    fn tag() -> GoalKindTag;
    fn generate_candidates(view: &dyn GoalBeliefView, agent: EntityId, ...) -> Vec<GroundedGoal>;
    fn planner_ext() -> &'static dyn GoalKindPlannerExt;
    fn exhaustion_conditions(goal: &GoalKind, ...) -> Vec<ExhaustionInvalidationCondition>;
    fn ranking_policy() -> GoalFamilyPolicy;
    fn feasibility_hint(...) -> FeasibilityHint;
}
```

A registration macro could generate the dispatch tables and provide a compile-time check that all required methods are implemented. This consolidates per-goal-kind logic into one file per goal kind.

**Affected modules**: candidate_generation, goal_model, planner_ops, exhaustion, ranking, goal_policy, feasibility
**Complexity**: Large — requires restructuring the core dispatch mechanism
**Principle alignment**: Principle 26 (no backward compat) supports restructuring; Principle 28 (declare causal hooks) is easier with bundled specs

### 8.2 Split candidate_generation.rs by Domain

**Problem**: 9,735-line monolith.

**Proposal**: Split into domain files:

```
candidate_generation/
    mod.rs         — orchestration, shared helpers
    survival.rs    — ConsumeOwnedCommodity, AcquireCommodity, Sleep, Relieve, Wash
    combat.rs      — EngageHostile, ReduceDanger, LootCorpse, BuryCorpse
    care.rs        — TreatWounds
    production.rs  — ProduceCommodity, SellCommodity, RestockCommodity, MoveCargo
    social.rs      — ShareBelief
    political.rs   — ClaimOffice, SupportCandidateForOffice
    justice.rs     — InvestigateViolation, Accuse, PunishAccused, StealItem
```

**Affected modules**: candidate_generation.rs only (internal refactor)
**Complexity**: Medium — mostly file splitting with shared helper extraction
**Principle alignment**: No behavioral change; improves Principle 27 (debuggability)

### 8.3 Goal-Scoped Snapshot Construction

**Problem**: Full entity state copied for every snapshot, regardless of goal kind.

**Proposal**: The planning snapshot could accept a `GoalKindTag` parameter and skip copying fields irrelevant to that goal. A Sleep goal needs: agent position, fatigue level, bed locations. It does not need: trade profiles, combat stats, political beliefs.

Approach: Define `SnapshotScope` per goal kind specifying which entity fields to copy. The snapshot builder uses this scope as a filter.

**Affected modules**: planning_snapshot.rs, goal_model.rs (scope declaration)
**Complexity**: Medium
**Principle alignment**: Principle 11 (compress computation, not causality) — this is a pure performance optimization that doesn't change search behavior

### 8.4 Per-Goal-Kind Budget Tuning

**Problem**: All goals share `max_node_expansions: 224`.

**Proposal**: `GoalKindSpec` (or `GoalFamilyPolicy`) declares a budget modifier:

```rust
fn budget_hint() -> BudgetHint {
    BudgetHint::Low    // Sleep: max 32 expansions
    BudgetHint::Medium // AcquireCommodity: max 128 expansions
    BudgetHint::High   // RestockCommodity: max 224 expansions
    BudgetHint::Full   // supply chains: max 224+ expansions
}
```

This would allow simple goals to terminate faster and give complex goals more room.

**Affected modules**: budget.rs, search/mod.rs, goal_policy.rs
**Complexity**: Small
**Principle alignment**: Principle 11 (performance optimization without causality change)

### 8.5 Compile-Time Exhaustion Condition Enforcement

**Problem**: No compile-time check that all `GoalKind` variants register invalidation conditions.

**Proposal**: The match in `derive_invalidation_conditions()` should not have a catch-all arm. Instead, require an explicit arm for every variant. Rust's exhaustive match will then catch missing variants at compile time.

If a `GoalKind` genuinely has no conditions, use an explicit empty arm: `GoalKind::Sleep => { /* always invalidate on position change */ }`.

**Affected modules**: exhaustion.rs
**Complexity**: Small
**Principle alignment**: Direct code quality improvement

### 8.6 `process_agent` State Machine Extraction

**Problem**: 897-line orchestration function with 7+ mutable state variables.

**Proposal**: Extract the pipeline into a state machine with explicit phases:

```rust
enum AgentTickPhase {
    Reconciliation,
    AssumptionEval,
    ReadPhase,
    DeferredAssumptionEval,
    FeasibilityAnnotation,
    ActiveActionEval,
    PlanningEval,
    PatienceCheck,
    Finalization,
}
```

Each phase is a pure function: `phase(inputs) -> (outputs, next_phase)`. This makes the data flow explicit and testable per-phase.

**Affected modules**: agent_tick/mod.rs and submodules
**Complexity**: Large — significant refactor
**Principle alignment**: Principle 27 (debuggability), Principle 24 (state-mediated, not call-chain)

### 8.7 Expand Decision Trace with Cost Metrics

**Problem**: Decision traces record what was decided but not the computational cost.

**Proposal**: Add timing and expansion metrics to traces:

```rust
pub struct PlanAttemptTrace {
    pub goal: GoalKey,
    pub outcome: PlanSearchOutcome,
    pub expansions_used: u16,
    pub frontier_max_size: u16,
    pub planning_state_copies: u16,  // How many CoW copies
    pub total_estimated_ticks: u32,
    // ... existing fields
}
```

This enables performance analysis of search efficiency without instrumentation.

**Affected modules**: decision_trace.rs, search/mod.rs
**Complexity**: Small
**Principle alignment**: Principle 27 (debuggability)

---

## 9. Potential New Features

### 9.1 Learned Route Preferences (from Plan Outcomes → Belief State)

**Description**: When an agent travels a route and encounters danger (combat, wound), record a negative route preference in belief state. Future travel planning would check route preferences and prefer safer alternatives.

**Principle fit**: Principle 13 (knowledge acquired locally), Principle 20 (diversity through learned experience), Principle 3 (concrete state — route preference is a belief about a specific edge, not `danger_score`)

**Module impact**: `worldwake-core` (new belief component: RoutePreference), `planning_snapshot` (include route beliefs), `search/heuristic` (adjust travel cost by route preference), `worldwake-systems/perception` (record route experiences)

**Dependencies**: Requires Epic 14 (Perception) for per-agent belief stores

### 9.2 Visible Intention Signals (Without Violating Locality)

**Description**: Agents performing visible actions (traveling, harvesting, trading) create observable signals that co-located agents can perceive. An agent who sees another agent heading to the orchard might choose a different food source.

**Principle fit**: Principle 7 (locality — only co-located agents observe), Principle 19 (intentions are revisable — observing competition triggers replan), Principle 8 (actions have occupancy — visible actions are perception events)

**Module impact**: `worldwake-systems/perception` (emit action-observation events), `candidate_generation` (discount goals when competition observed), `ranking` (adjust motive based on observed competition)

**Dependencies**: Requires Epic 14 (Perception)

### 9.3 Goal-Kind-Specific Search Strategies

**Description**: Replace the single A* search with strategy selection per goal kind. Simple goals use greedy search. Multi-step supply chains use hierarchical decomposition (first find recipe chain, then plan individual steps).

**Principle fit**: Principle 11 (compress computation, not causality), Principle 18 (resource-bounded reasoning — different goals have different planning complexity)

**Module impact**: `search/mod.rs` (strategy selection), `goal_model.rs` (declare preferred strategy), new module `search/hierarchical.rs`

**Dependencies**: None — can be implemented independently

### 9.4 Temporal Planning: Scheduled Actions

**Description**: Allow agents to plan future actions ("I will sell apples at the market when the merchant arrives"). Creates a pending intention that activates when conditions are met.

**Principle fit**: Principle 19 (intentions as commitments with assumptions), Principle 8 (actions unfold over time)

**Module impact**: New component `ScheduledIntention`, `candidate_generation` (emit from known schedules), `agent_tick/frame` (manage scheduled frames), `interrupts` (evaluate scheduled vs active)

**Dependencies**: Requires institutional schedules or routine patterns

### 9.5 Partial Plan Reuse

**Description**: When a plan partially succeeds (3 of 5 steps completed) and replanning is needed, reuse the remaining steps as a starting point rather than searching from scratch.

**Principle fit**: Principle 11 (compress computation), Principle 18 (resource-bounded reasoning)

**Module impact**: `search/mod.rs` (accept initial plan as hint), `agent_tick/planning` (pass remaining steps)

**Dependencies**: None

### 9.6 Meta-Reasoning: Plan Value Assessment

**Description**: Before committing search budget to a goal, estimate the expected value of planning. If a goal has exhausted 3 times and conditions haven't changed much, skip it entirely rather than burning 16 expansions on the backoff.

**Principle fit**: Principle 18 (resource-bounded reasoning — agents shouldn't waste cognitive effort)

**Module impact**: `exhaustion.rs` (value assessment), `agent_tick/planning` (skip planning for low-value goals)

**Dependencies**: None

### 9.7 Social Observation Integration

**Description**: Agents observe other agents' actions and adjust their own goals. Seeing a merchant selling bread at a good price → increase trade motivation. Seeing a guard patrolling → decrease theft motivation.

**Principle fit**: Principle 7 (locality — co-location required), Principle 13 (knowledge via observation), Principle 1 (emergence from interacting systems)

**Module impact**: `worldwake-systems/perception` (action observation events), `candidate_generation` (social evidence), `ranking` (social motive adjustment)

**Dependencies**: Requires Epic 14 (Perception)

---

## 10. Metrics Summary

### Module Size Distribution

```
Files >5K lines:  2  (candidate_generation, goal_model)
Files 1-5K lines: 14 (planning_state, ranking, decision_trace, planner_ops, planning_snapshot,
                       failure_handling, exhaustion, frame, planning, interrupts, feasibility,
                       agent_tick/mod, plan_revalidation, plan_selection)
Files <1K lines:  20+ (pressure, enterprise, goal_switching, budget, etc.)
```

### GoalKind Match Site Count (Approximate)

| File | Match sites |
|------|----------:|
| candidate_generation.rs | ~163 |
| goal_model.rs | ~367 |
| planner_ops.rs | ~68 |
| exhaustion.rs | ~54 |
| ranking.rs | ~40 |
| goal_policy.rs | ~21 |
| feasibility.rs | ~21 |
| **Total** | **~734** |

### Key Ratios

| Metric | Value |
|--------|------:|
| Goal kinds | 21 |
| Planner op kinds | 25 |
| Files touched per new goal kind | 8+ |
| Match arms per new goal kind | ~35 |
| Search budget (max expansions) | 224 |
| Search depth limit | 8 steps |
| Beam width | 8 successors |
| Default switch margin | 10% (100 permille) |
| Block TTLs | 5 / 20 / 200 ticks |
| Exhaustion backoff floor | 16 expansions |
| Override maps in PlanningState | 12+ |
| Golden tests | 191 |
| Test infrastructure | ~2,430 lines |

### Crate Dependency Chain for AI Decisions

```
Agent perception → BeliefView → candidate_generation → ranking → search → plan_selection
                                                                     ↓
                                                              planning_state
                                                              planning_snapshot
                                                              planner_ops
                                                              goal_model
                                                                     ↓
                                                              interrupts ← goal_switching
                                                                     ↓
                                                              failure_handling → blocked_intent_memory
                                                              exhaustion → invalidation_conditions
                                                                     ↓
                                                              decision_runtime (persistent state)
                                                                     ↓
                                                              agent_tick (orchestration)
                                                                     ↓
                                                              scheduler.input_queue (output)
```

---

## Appendix A: Golden Test Harness Infrastructure

The test harness (`GoldenHarness`) provides:

**World setup**: `seed_agent()`, `give_commodity()`, `place_workstation_with_source()`, `seed_office()`, `seed_faction()`, `add_hostility()`

**Belief seeding**: `seed_actor_beliefs()`, `seed_belief_from_world()`, `seed_office_holder_belief()`, `seed_force_controller_belief()`, `seed_told_belief_memory()`

**Profile factories**: `default_combat_profile()`, `keen_perception_profile()`, `accepting_tell_profile()`, `enterprise_weighted_utility()`

**Execution**: `step_once()` → runs one full tick (AI decisions → action processing → system execution)

**State queries**: `agent_hunger()`, `agent_wound_load()`, `agent_is_dead()`, `agent_has_active_action()`, `agent_active_action_name()`, `agent_commodity_qty()`

**Trace access**: `driver.enable_tracing()` + `driver.trace_sink()`, `enable_action_tracing()` + `action_trace_sink()`

**Determinism**: `hash_world()`, `hash_event_log()`, `snapshot_state()`, `save_load_roundtrip()`

**Timeline**: `CrossLayerTimelineBuilder` merges decision, action, political, and event log layers into chronological view filtered by agent or office.

---

## Appendix B: Canonical Regression Scenario Alignment

How the current AI architecture maps to the canonical regression scenarios:

| Scenario | Current Support | Gaps |
|----------|----------------|------|
| **A** (Beast → Bounty → Hunt) | Partial: combat, loot, death work. No beast AI, no bounty system, no jurisdiction | Epic 14+ (perception, institutions) |
| **B** (Market trip → Dragon → Retreat) | Strong: travel interruptible, priority-based interrupts, flee goals | Needs real perception (E14) for dragon detection |
| **C** (Gold → Theft → Discovery) | Partial: ownership tracking works, theft goal exists. Discovery needs expectation-mismatch detection | Epic 14+ (stale belief + observation) |
| **D** (Rumor → Travel → Discovery → Replan) | Strong: golden_social tests this pattern. Stale beliefs, reobservation, belief correction, replanning all work | Active with OmniscientBeliefView caveat |
| **E** (Competing claimants → Queue → Prune) | Strong: facility queues, patience exhaustion, dead agent pruning, contention all tested | Works end-to-end in golden_production tests |

## Outcome

- Completion date: 2026-03-29
- What actually changed: Archived this report as historical analysis for a superseded AI decision architecture version and moved it to `archive/reports/`.
- Deviations from original plan: None. The report content was preserved; only archival metadata and repository references were updated.
- Verification results: Confirmed the archived file exists under `archive/reports/`, updated the in-repo spec reference to the archived path, and verified the original path no longer exists under `reports/`.
