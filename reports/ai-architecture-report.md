# AI Architecture Reference — 2026-04-05

Self-contained structured reference of the Worldwake AI architecture, derived from golden E2E tests and source code tracing. Intended for external LLM evaluation against FOUNDATIONS.md (provided separately).

---

## 1. Architecture Overview

### Crate Structure

5-crate workspace:

| Crate | Role | Dependencies |
|-------|------|-------------|
| `worldwake-core` | IDs, types, ECS store, topology, items, relations | None |
| `worldwake-sim` | Event log, action framework, scheduler, replay | core |
| `worldwake-systems` | Needs/metabolism, production/crafting, trade, combat, travel, perception, politics, patrol, bandit camp | core, sim |
| `worldwake-ai` | Pressure-based GOAP planner, goal ranking, decision runtime | core, sim, systems |
| `worldwake-cli` | Human control interface | all |

System modules in `worldwake-systems` depend only on `worldwake-core` and `worldwake-sim`, never on each other. Cross-system interaction is mediated through shared state (components, event log), never through direct function calls.

### ECS Design

Custom ECS (no external crate). `BTreeMap`-based typed component storage for deterministic iteration order.

```rust
pub struct World {
    allocator: EntityAllocator,      // Generation-aware slot allocator
    components: ComponentTables,     // BTreeMap<EntityId, T> per component type
    relations: RelationTables,       // Social/spatial relationship graphs
    topology: Topology,              // Place graph with travel edges
}
```

`ComponentTables` is generated via macro. Each component type gets its own `BTreeMap<EntityId, T>` with standard CRUD methods (`insert`, `get`, `get_mut`, `remove`, `has`, `iter`, `entities_with`, `query`, `count`). There are 40+ registered component types.

### Determinism Guarantees

- `ChaCha8Rng` seeded — reproducible randomness
- `BTreeMap`/`BTreeSet` only in authoritative state — no `HashMap`/`HashSet`
- No floating-point arithmetic — all numeric values use `Permille(u16)` or integer types
- No wall-clock time — simulation uses logical `Tick(u64)`
- `StateHash` validates replay fidelity across save/load boundaries

### Key Foundational Types

```rust
pub struct EntityId { pub slot: u32, pub generation: u32 }  // Stable reference with stale detection
pub struct Tick(pub u64);                                     // Discrete logical time
pub struct Permille(u16);                                     // Fixed-point [0..=1000] per-mille
pub struct Quantity(pub u32);                                 // Conserved lot count
pub struct LoadUnits(pub u32);                                // Container capacity units

pub enum EntityKind {
    Agent, ItemLot, UniqueItem, Container, Facility,
    Place, Faction, Office, Record, SocialArtifact,
}
```

### Control Model

No `Player` type. Agents have `ControlSource`:
- `Human` — input comes from CLI
- `Ai` — autonomous decision pipeline runs each tick
- `None` — passive entity

---

## 2. Agent Decision Pipeline

The AI implements pressure-based GOAP (Goal-Oriented Action Planning). Each tick, every AI-controlled agent runs through a multi-stage pipeline that converts physiological/social pressures into committed actions.

### Pipeline Overview

```
Per-Tick Agent Decision Pipeline:

1. DEAD CHECK → if dead, skip
2. RECONCILE IN-FLIGHT STATE → apply replan signals, reconcile committed actions
3. PRE-PLANNING ASSUMPTION EVALUATION → check frame assumptions (except NoCriticalThreat)
4. CANDIDATE GENERATION → emit grounded goals from beliefs + drives
5. SUPPRESSION FILTERING → filter against BlockedIntentMemory
6. GOAL RANKING → score by motive, priority class, discounts; compute DecisionContext
7. DEFERRED NoCriticalThreat EVALUATION → now with ranked candidates available
8a. IF ACTIVE ACTION → INTERRUPT EVALUATION (compare vs top challenger)
8b. IF NO ACTIVE ACTION → PLANNING PATH (select best candidate, search for plan, enqueue)
9. PERSIST → update frame, active goal, facility intents, violation memory
```

### 2.1 Candidate Generation

**Entry point:**
```rust
pub fn generate_candidates_with_travel_horizon(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    blocked: &BlockedIntentMemory,
    violation_memory: &ViolationMemory,
    recipes: &RecipeRegistry,
    current_tick: Tick,
    travel_horizon: u8,       // default 6
    tracing_enabled: bool,
) -> CandidateGenerationResult
```

**Returns:**
```rust
pub struct CandidateGenerationResult {
    pub candidates: Vec<GroundedGoal>,
    pub diagnostics: CandidateGenerationDiagnostics,
    pub pending_violations: Vec<PendingViolationRecord>,
}
```

The pipeline calls 11 emission functions in sequence:

1. `emit_need_candidates()` — Sleep, Relieve, Wash, Consume
2. `emit_production_candidates()` — ProduceCommodity
3. `emit_enterprise_candidates()` — Restock, Sell, MoveCargo
4. `emit_bounty_candidates()` — FulfillBounty
5. `emit_combat_candidates()` — EngageHostile, ReduceDanger, RaidTarget, RegroupWithFaction, EstablishBanditCamp
6. `emit_crime_candidates()` — StealItem
7. `emit_social_candidates()` — ShareBelief
8. `emit_patrol_candidates()` — Patrol
9. `emit_political_candidates()` — ClaimOffice, SupportCandidateForOffice
10. `emit_recorded_violation_candidates()` — Accuse, PunishAccused, InvestigateViolation
11. `emit_expectation_violation_candidates()` — custom violation detection

Each function reads agent beliefs (never authoritative world state) to determine what goals are available.

### GoalKind Enum (all 27 variants)

```rust
pub enum GoalKind {
    // Basic Needs
    ConsumeOwnedCommodity { commodity: CommodityKind },
    Sleep,
    Relieve,
    Wash,
    
    // Acquisition & Enterprise
    AcquireCommodity { commodity: CommodityKind, purpose: CommodityPurpose },
    ProduceCommodity { recipe_id: RecipeId },
    SellCommodity { commodity: CommodityKind },
    RestockCommodity { commodity: CommodityKind },
    MoveCargo { commodity: CommodityKind, destination: EntityId },
    
    // Combat & Danger
    EngageHostile { target: EntityId },
    RaidTarget { target: EntityId },
    ReduceDanger,
    TreatWounds { patient: EntityId },
    
    // Faction
    RegroupWithFaction { faction: EntityId },
    EstablishBanditCamp { faction: EntityId },
    
    // Corpse
    LootCorpse { corpse: EntityId },
    BuryCorpse { corpse: EntityId, burial_site: EntityId },
    
    // Bounties
    FulfillBounty { bounty: EntityId },
    
    // Social
    ShareBelief { listener: EntityId, topic: TellTopic, communication_class: CommunicationClass },
    
    // Political
    ClaimOffice { office: EntityId },
    SupportCandidateForOffice { office: EntityId, candidate: EntityId },
    
    // Justice
    InvestigateViolation { violation_id: ViolationId, place: EntityId },
    Accuse { crime_register: EntityId, accused: EntityId, violation_id: ViolationId },
    PunishAccused { office: EntityId, accused: EntityId, accusation_entry: RecordEntryId, punishment: PunishmentKind },
    
    // Patrol
    Patrol { place: EntityId },
    
    // Crime
    StealItem { target_item: EntityId },
}
```

### 2.2 Suppression Filtering

Candidates are checked against `BlockedIntentMemory` — a time-based record of previously failed goals:

```rust
pub struct BlockedIntent {
    pub blocker_key: BlockerKey,
    pub blocking_fact: BlockingFact,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
}

pub struct BlockerKey {
    pub goal_key: GoalKey,
    pub place: Option<EntityId>,
    pub target: Option<EntityId>,
    pub action_def: Option<ActionDefId>,
}
```

A candidate is suppressed only if it matches all non-None fields of a blocker. Blockers expire after a configurable TTL.

**BlockingFact variants** (domain-specific failure reasons):
```rust
pub enum BlockingFact {
    Unknown, TargetGone, NoKnownPath, NoKnownSeller,
    SellerOutOfStock, TooExpensive, MissingInput(CommodityKind),
    CombatTooRisky, DangerTooHigh, LackingAffordance, ContentionLoss,
    // ... additional domain-specific facts
}
```

### 2.3 Goal Ranking

Goals are ranked using `GoalFamilyPolicy` + `DecisionContext`:

```rust
pub struct GoalFamilyPolicy {
    pub suppression: SuppressionRule,
    pub penalty_interrupt: PenaltyInterruptEligibility,
    pub free_interrupt: FreeInterruptRole,
}

pub enum SuppressionRule {
    Never,
    WhenStressedAtOrAbove(GoalPriorityClass),
}

pub struct DecisionContext {
    pub max_self_care_class: GoalPriorityClass,  // Highest homeostatic stress
    pub danger_class: GoalPriorityClass,         // Combat/threat pressure
}
```

Ranking integrates:
- **Motive score** — urgency from homeostatic state and utility profile weights
- **Priority class** — Critical, High, Medium, Low
- **Source reliability discount** — belief confidence decay from perception chain
- **Competition discount** — interference from rival candidates

### 2.4 Interrupt Evaluation

When an agent has an active action, the pipeline evaluates whether a challenger should preempt:

```rust
pub fn evaluate_interrupt(
    runtime: &AgentDecisionRuntime,
    active_goal: Option<GoalKey>,
    current_action_interruptibility: Interruptibility,
    ranked_candidates: &[RankedGoal],
    decision_context: &DecisionContext,
    // ... additional parameters
) -> InterruptDecision

pub enum InterruptDecision {
    NoInterrupt,
    InterruptForReplan { trigger: InterruptTrigger },
}

pub enum InterruptTrigger {
    CriticalSurvival,       // Health/hunger critical threshold crossed
    CriticalDanger,         // Immediate threat detected
    HigherPriorityGoal,     // Ranked goal in higher class
    SuperiorSameClassPlan,  // Better plan for same goal class
    PlanInvalid,            // Current plan no longer valid
    OpportunisticLoot,      // Quick valuable opportunity
}
```

**Interruptibility levels:**
- `NonInterruptible` — cannot stop mid-execution
- `InterruptibleWithPenalty` — only Critical priority challengers can interrupt
- `FreelyInterruptible` — stoppable without penalty

### 2.5 Plan Search (A* with Barrier Fallback)

```rust
pub fn search_plan(
    snapshot: &PlanningSnapshot,
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    budget: &ReasoningProfile,
    recipes: &RecipeRegistry,
    blocked: &BlockedIntentMemory,
    current_tick: Tick,
    // ... diagnostic outputs
) -> PlanSearchResult

pub enum PlanSearchResult {
    Found(Box<PlannedPlan>),
    Unsupported,
    BudgetExhausted { expansions_used: u16 },
    FrontierExhausted { expansions_used: u16 },
}
```

**Algorithm:** A* search with `f(node) = g(node) + h(node)` where `g` = steps taken, `h` = minimum perceived travel to goal. Frontier is a `BinaryHeap<FrontierEntry>`.

**Budget constraints (per-agent):**
```rust
pub struct ReasoningProfile {
    pub max_candidates_to_plan: u8,
    pub max_plan_depth: u8,
    pub snapshot_travel_horizon: u8,
    pub max_prerequisite_locations: u8,
    pub max_node_expansions: u16,       // Max nodes expanded before giving up
    pub beam_width: u8,                 // Max non-terminal successors per level
    pub switch_margin: Permille,        // Goal-switching utility threshold
    pub transient_block_ticks: u32,
    pub unknown_block_ticks: u32,
    pub structural_block_ticks: u32,
    pub initial_cooldown_ticks: u32,
    pub max_cooldown_ticks: u32,
}
```

**Terminal conditions:**
```rust
pub enum PlanTerminalKind {
    GoalSatisfied,       // Return immediately
    CombatCommitment,    // Return immediately
    ProgressBarrier,     // Keep searching, store as fallback
}
```

If frontier exhausts before finding `GoalSatisfied`, the search returns the best `ProgressBarrier` plan as a fallback (best-effort partial progress).

**PlannerOpKind (all 30 operation types):**
```rust
pub enum PlannerOpKind {
    Travel, Patrol, Consume, Sleep, Relieve, Wash,
    EstablishCamp, Trade, StaffMarket, QueueForFacilityUse,
    Harvest, Craft, MoveCargo, StockManagement,
    Heal, Loot, Bury, ClaimBounty,
    Tell, ConsultRecord,
    Attack, Defend, Bribe, Threaten,
    Accuse, Fine, Exile,
    DeclareSupport, PressForceClaim, YieldForceClaim,
    Investigate, AskWitness,
}
```

### 2.6 Plan Failure Handling

When a plan step fails during execution:

```rust
pub fn handle_plan_failure(
    context: &PlanFailureContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockedIntentMemory,
    reasoning: &ReasoningProfile,
)
```

Sequence:
1. Clear current plan and intention frame
2. Derive blocking fact from failure context (TargetGone, NoKnownPath, SellerOutOfStock, etc.)
3. Create `BlockedIntent` with TTL based on blocking fact severity
4. Set `REPLAN_SIGNAL` dirty flag to trigger replanning next tick

### 2.7 Plan Revalidation

Before executing the next step of an existing plan:

```rust
pub fn revalidate_next_step(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    step: &PlannedStep,
    bindings: &MaterializationBindings,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> bool
```

Checks whether the planned step's affordance still exists. Uses `requested_affordance_matches` which checks identity match first, then falls back to the handler's `payload_override_is_valid` callback for planner-synthesized payloads.

**Pursuit-specific revalidation:**
```rust
pub fn is_pursuit_plan_invalid(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    plan: &PlannedPlan,
    current_tick: Tick,
) -> Option<PursuitInvalidationReason>

pub enum PursuitInvalidationReason {
    NoProfile, NoBelief, TargetDead, PlaceUnknown,
    CoLocated, PlaceChanged, ConfidenceDecayed,
}
```

### 2.8 Decision Outcome

```rust
pub enum DecisionOutcome {
    Dead,
    ActiveAction {
        action_def_id: ActionDefId,
        action_name: String,
        interrupt: InterruptTrace,
        frame_transition: Option<FrameTransitionTrace>,
    },
    Planning(Box<PlanningPipelineTrace>),
}
```

### 2.9 Runtime State

```rust
pub struct AgentDecisionRuntime {
    pub current_plan: Option<PlannedPlan>,
    pub current_step_index: usize,
    pub step_in_flight: bool,
    pub materialization_bindings: MaterializationBindings,
    pub exhaustion_cache: BTreeMap<OpportunityKey, ExhaustionEntry>,
    pub dirty: DirtySet,
    pub last_frame_clear_reason: Option<FrameClearReason>,
    pub last_priority_class: Option<GoalPriorityClass>,
    pub last_effective_place: Option<EntityId>,
    pub last_facility_access_signature: BTreeSet<(EntityId, EntityId, EntityId)>,
    pub last_patrol_route: Option<PatrolRoute>,
}
```

`DirtySet` flags determine what triggers replanning:
- `STRUCTURAL_MASK` — dead agents, new frame, equipment changes → full replan
- `SNAPSHOT_MASK` — belief updates, new observations → revalidation
- `REPLAN_SIGNAL` — explicit replanning request
- `ASSUMPTION_FAILED` — frame assumption violation

---

## 3. Action Framework

### 3.1 Action Definition

```rust
// 14-field immutable template
pub struct ActionDef {
    pub id: ActionDefId,
    pub name: String,
    pub domain: ActionDomain,
    pub actor_constraints: Vec<ActorConstraint>,
    pub target_specs: Vec<TargetSpec>,
    pub preconditions: Vec<Precondition>,
    pub commit_conditions: Vec<CommitCondition>,
    pub duration_expr: DurationExpr,
    pub interruptibility: Interruptibility,
    pub default_payload: Option<ActionPayload>,
    pub contention_scope: Option<ContentionScope>,
    pub visibility: VisibilitySpec,
    pub tags: BTreeSet<EventTag>,
    pub handler_id: ActionHandlerId,
}
```

### 3.2 Action Domains (11 variants)

```rust
pub enum ActionDomain {
    Generic, Needs, Production, Trade, Social,
    Epistemic, Travel, Transport, Combat, Care, Corpse,
}
```

### 3.3 Handler Lifecycle

```rust
// Four core lifecycle functions per action:
trait ActionHandler {
    fn on_start(&self, ctx) -> Option<ActionState>;    // Initial setup
    fn on_tick(&self, ctx) -> ActionProgress;          // Repeating progress
    fn on_commit(&self, ctx) -> CommitOutcome;         // Finalization with materializations
    fn on_abort(&self, ctx, reason: AbortReason);      // Cleanup on interrupt/failure
}

pub enum ActionProgress {
    Continue,
    Complete,
}

pub enum AbortReason {
    CommitConditionFailed { condition },
    Interrupted { kind: InterruptReason, detail },
    ExternalAbort { kind: ExternalAbortReason, detail },
}
```

### 3.4 Action Payloads (23 variants)

```rust
pub enum ActionPayload {
    None, ConsultRecord, Tell, Bribe, Threaten, Accuse,
    Punish, EstablishCamp, DeclareSupport, PressForceClaim,
    YieldForceClaim, Transport, Harvest, Craft, Trade,
    Combat, Loot, Investigate, AskWitness,
    QueueForFacilityUse, StaffMarket, PostBounty, PostNotice,
}
```

### 3.5 Affordance Discovery

```rust
pub fn get_affordances(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> Vec<Affordance>

pub struct Affordance {
    pub def_id: ActionDefId,
    pub actor: EntityId,
    pub bound_targets: Vec<EntityId>,
    pub payload_override: Option<ActionPayload>,
    pub explanation: Option<String>,
    pub contention_status: ContentionStatus,
}
```

Pipeline per action def: check actor constraints → enumerate targets → check preconditions → expand payload variants → derive contention status.

### 3.6 Interruptibility

```rust
pub enum Interruptibility {
    NonInterruptible,
    InterruptibleWithPenalty,
    FreelyInterruptible,
}
```

---

## 4. System Interactions

### System Execution Order

Systems execute in a fixed canonical order within each tick. This order is load-bearing:

| Order | SystemId | Rationale |
|-------|----------|-----------|
| 1 | Needs | Deprivation visible before economic action |
| 2 | Production | New goods exist before trade |
| 3 | Trade | Economic resolution before combat |
| 4 | Combat | Deaths visible before camp abandonment |
| 5 | BanditCamp | Abandonment before contention |
| 6 | Contention | Completed exclusive actions free before politics |
| 7 | Politics | Institutional changes before perception |
| 8 | Perception | Witnessed crimes reshape routes before patrol |
| 9 | Patrol | Final system |

Pre-action: `ArtifactLifecycle` runs before input drain.

**System function signature:**
```rust
fn(SystemExecutionContext<'_>) -> Result<(), SystemError>

pub struct SystemExecutionContext<'a> {
    pub world: &'a mut World,
    pub event_log: &'a mut EventLog,
    pub rng: &'a mut DeterministicRng,
    pub active_actions: &'a BTreeMap<ActionInstanceId, ActionInstance>,
    pub action_defs: &'a ActionDefRegistry,
    pub tick: Tick,
    pub system_id: SystemId,
    // ... optional trace sinks
}
```

### 4.1 Needs / Metabolism

**What it does:** Advances homeostatic drives (hunger, thirst, fatigue, bladder, dirtiness) each tick based on per-agent metabolism profiles. Detects critical thresholds that trigger survival interrupts.

**Key types:**
```rust
pub struct HomeostaticNeeds {
    pub hunger: Permille,      // [0..=1000]
    pub thirst: Permille,
    pub fatigue: Permille,
    pub bladder: Permille,
    pub dirtiness: Permille,
}

pub struct MetabolismProfile {
    pub hunger_rate: Permille,
    pub thirst_rate: Permille,
    pub fatigue_rate: Permille,
    pub bladder_rate: Permille,
    pub dirtiness_rate: Permille,
    pub rest_efficiency: Permille,
    pub starvation_tolerance_ticks: NonZeroU32,
    pub dehydration_tolerance_ticks: NonZeroU32,
    pub exhaustion_collapse_ticks: NonZeroU32,
    pub bladder_accident_tolerance_ticks: NonZeroU32,
    pub toilet_ticks: NonZeroU32,
    pub wash_ticks: NonZeroU32,
    pub travel_fatigue_multiplier: Permille,
    pub travel_thirst_multiplier: Permille,
    pub travel_bladder_multiplier: Permille,
    pub wilderness_relief_dirtiness_penalty: Permille,
}
```

**State-mediated interactions:**
- Writes `HomeostaticNeeds` components → read by candidate generation for `ConsumeOwnedCommodity`, `Sleep`, `Relieve`, `Wash` goals
- Writes `DeprivationExposure` → read by combat system for wound recovery gates
- Travel multipliers applied when agent is in transit → creates bladder/fatigue escalation during travel

**Golden tests:** `golden_travel_physiology` (S58-S61), `golden_ai_decisions`, `golden_resilience`

### 4.2 Production / Crafting

**What it does:** Manages resource harvesting from workstations and crafting via recipes. Resources regenerate over time. Recipes consume input commodities and produce output commodities.

**Key types:**
```rust
pub struct ResourceSource {
    pub commodity: CommodityKind,
    pub available_quantity: Quantity,
    pub max_quantity: Quantity,
    pub regeneration_ticks_per_unit: Option<NonZeroU32>,
}

pub struct RecipeDefinition {
    pub inputs: Vec<(CommodityKind, Quantity)>,
    pub outputs: Vec<(CommodityKind, Quantity)>,
    pub workstation_tag: WorkstationTag,
    pub duration_ticks: NonZeroU32,
}
```

**State-mediated interactions:**
- Reads `WorkstationMarker` and `ResourceSource` from places → determines harvest availability
- Writes item lots to agent inventory → read by trade, needs systems
- `ProductionOutputOwner` policy (Actor vs Fixed location) determines where output goes

**Golden tests:** `golden_production`, `golden_supply_chain`, `golden_reasoning_diversity` (S97)

### 4.3 Trade

**What it does:** Manages buy/sell transactions between agents via negotiation state machines. Merchants have `MerchandiseProfile` defining what they sell. Demand memory tracks unfulfilled wants.

**Key types:**
```rust
pub struct MerchandiseProfile {
    pub sale_kinds: BTreeSet<CommodityKind>,
    pub home_facility: EntityId,
}

pub struct TradeDispositionProfile {
    pub negotiation_round_ticks: NonZeroU32,
    pub initial_offer_bias: Permille,
    pub concession_rate: Permille,
    pub rejection_escalation_rate: Permille,
}

pub struct DemandMemory {
    pub observations: Vec<DemandObservation>,
}
```

**State-mediated interactions:**
- Reads agent inventories and beliefs about seller stock → generates `SellCommodity`, `RestockCommodity` goals
- Writes commodity transfers between agent containers → conservation-tracked
- Demand observations feed back into candidate generation for enterprise goals

**Golden tests:** `golden_trade`, `golden_merchant_selling`, `golden_supply_chain`, `golden_commodity_opportunity`

### 4.4 Combat

**What it does:** Resolves attacks, inflicts wounds, tracks combat stances, manages death. Wounds have severity, bleed rate, and body part. Death occurs when wound severity exceeds capacity.

**Key types:**
```rust
pub struct CombatProfile {
    pub wound_capacity: Permille,
    pub incapacitation_threshold: Permille,
    pub attack_skill: Permille,
    pub guard_skill: Permille,
    pub defend_bonus: Permille,
    pub natural_clot_resistance: Permille,
    pub natural_recovery_rate: Permille,
    pub unarmed_wound_severity: Permille,
    pub unarmed_bleed_rate: Permille,
    pub unarmed_attack_ticks: NonZeroU32,
    pub defend_stance_ticks: NonZeroU32,
}

pub struct Wound {
    pub id: WoundId,
    pub body_part: BodyPart,
    pub cause: WoundCause,
    pub severity: Permille,
    pub inflicted_at: Tick,
    pub bleed_rate_per_tick: Permille,
}

pub struct DeadAt(pub Tick);

pub enum CombatStance { Defending }
```

**State-mediated interactions:**
- Writes `WoundList`, `DeadAt` → read by care system for `TreatWounds` goals
- Writes `DeadAt` → read by corpse management for `LootCorpse`, `BuryCorpse`
- Danger detection feeds `ReduceDanger` goal generation via `UtilityProfile.danger_weight`
- Wound accumulation above `flee_wound_threshold` suppresses `RaidTarget` emission in bandit faction logic

**Golden tests:** `golden_combat`, `golden_pursuit`, `golden_t22_bandit_camp_destruction`

### 4.5 Travel / Transport

**What it does:** Agents move between places via the place graph. Travel takes multiple ticks (edge-based). Agents in transit have elevated metabolism costs.

**Key types:**
```rust
pub struct Topology {
    places: BTreeMap<EntityId, Place>,
    edges: BTreeMap<TravelEdgeId, TravelEdge>,
    outgoing: BTreeMap<EntityId, Vec<TravelEdgeId>>,
    incoming: BTreeMap<EntityId, Vec<TravelEdgeId>>,
}

pub struct TravelEdge {
    id: TravelEdgeId,
    from: EntityId,
    to: EntityId,
    travel_time_ticks: NonZeroU32,
    capacity: Option<NonZeroU16>,
}

pub struct Place {
    pub name: String,
    pub capacity: Option<NonZeroU16>,
    pub tags: BTreeSet<PlaceTag>,
}

pub enum PlaceTag {
    Village, Farm, Store, Inn, Hall, Barracks, Latrine,
    Crossroads, Forest, Camp, Road, Trail, Field, Gate,
}
```

**Route computation:** Dijkstra on `travel_time_ticks`. Returns `Route { places, edges, total_travel_time }`.

**State-mediated interactions:**
- Travel actions write agent location → visible to perception system
- Departure observation by co-located agents → creates `BelievedActivity` with travel direction
- Travel body cost multipliers from `MetabolismProfile` → drives bladder/fatigue escalation
- `RouteExperience` records hostile encounters → influences future route selection via `PreferenceProfile.route_caution_weight`

**Golden tests:** `golden_travel_physiology` (S58-S61), `golden_experience_preferences` (S91-S93), `golden_pursuit`

### 4.6 Perception

**What it does:** Three-pass system that converts events and co-location into agent beliefs:

1. **Direct local observation** — agents at each place observe co-located entities with `observation_fidelity` probability
2. **Event-based witness resolution** — for each event, compute witness set via `VisibilitySpec`, roll observation, generate beliefs
3. **Mismatch detection** — when new observation contradicts prior belief, emit discovery events

**Key types:**
```rust
pub struct PerceptionProfile {
    pub memory_capacity: u32,
    pub memory_retention_ticks: u64,
    pub observation_fidelity: Permille,           // default 875‰
    pub confidence_policy: BeliefConfidencePolicy,
    pub institutional_memory_capacity: u32,
    pub consultation_speed_factor: Permille,
    pub contradiction_tolerance: Permille,
}

pub struct BeliefConfidencePolicy {
    pub direct_observation_base: Permille,     // 950‰
    pub report_base: Permille,                 // 780‰
    pub rumor_base: Permille,                  // 560‰
    pub inference_base: Permille,              // 420‰
    pub report_chain_penalty: Permille,        // -90‰ per hop
    pub rumor_chain_penalty: Permille,         // -110‰ per hop
    pub staleness_penalty_per_tick: Permille,  // -12‰ per tick
}

pub enum PerceptionSource {
    DirectObservation,
    Report { from: EntityId, chain_len: u8 },
    Rumor { chain_len: u8 },
    Inference,
}

pub enum VisibilitySpec {
    ParticipantsOnly,
    SamePlace,
    AdjacentPlaces { max_hops: u8 },
    PublicRecord,
    Hidden,
}
```

**State-mediated interactions:**
- Reads events from `EventLog` → writes `AgentBeliefStore` components
- Belief updates trigger dirty flags in AI runtime → candidate regeneration
- Social observations feed violation detection → candidate generation for justice goals
- Memory capacity enforcement evicts stale beliefs → bounds cognitive load

**Golden tests:** `golden_social`, `golden_pursuit` (S69 — stale belief honest failure), `golden_patrol`, `golden_t22_bandit_camp_destruction`

### 4.7 Social / Institutional

**What it does:** Manages belief sharing (tell actions), institutional records, office succession, faction membership, crime registers, and violation memory.

**Key types:**
```rust
pub struct TellProfile {
    pub max_tell_candidates: u8,
    pub max_relay_chain_len: u8,
    pub conversation_memory_capacity: u16,
    pub conversation_memory_retention_ticks: u64,
}

pub enum TellTopic {
    EntityBelief { subject: EntityId },
    SocialObservation { observation: SocialObservation },
    InstitutionalClaim { claim: InstitutionalClaim },
}

pub struct ToldBeliefMemory {
    pub shared_state: SharedTellState,
    pub told_tick: Tick,
}

pub enum HeardBeliefDisposition {
    Accepted, Rejected, AlreadyHeldEqualOrNewer, NotInternalized,
}

pub enum SuccessionLaw { Support, Force, Custom }
```

**State-mediated interactions:**
- Tell actions write `ToldBeliefMemory` / `HeardBeliefMemory` → suppresses re-telling unchanged beliefs
- Relay chain degradation: confidence decreases per hop via `report_chain_penalty`
- Office claims write `InstitutionalClaim` → read by perception for institutional belief propagation
- Violation memory written by perception → read by candidate generation for `InvestigateViolation`, `Accuse`

**Golden tests:** `golden_social`, `golden_offices`, `golden_patrol`, `golden_care` (belief sharing about wounds)

### 4.8 Bandit Camp System

**What it does:** Manages bandit faction lifecycle — camp establishment, abandonment detection, regroup behavior, and rally doctrine.

**Key types:**
```rust
pub struct BanditFactionPolicy {
    pub min_regroup_count: u8,
    pub establishment_duration_ticks: u32,
    pub abandonment_grace_ticks: u32,
    pub flee_wound_threshold: Permille,
    pub rally_place: EntityId,
}

pub struct BanditCamp {
    pub faction: EntityId,
    pub supplies: Container,
    pub empty_since_tick: Option<Tick>,
}
```

**State-mediated interactions:**
- Reads `WoundList` from faction members → triggers `RegroupWithFaction` when wounds exceed threshold
- Camp emptiness tracked via `empty_since_tick` → triggers abandonment after grace period
- `RaidTarget` generation reads co-located non-faction agents from beliefs → never authoritative state

**Golden tests:** `golden_t22_bandit_camp_destruction` (T22, S47-S49), `golden_pursuit`

### 4.9 Patrol System

**What it does:** Manages patrol route cycling for guard/authority agents. Patrol routes have assigned waypoints with dwell times. Violations discovered during patrol trigger investigation goals.

**Key types:**
```rust
pub struct PatrolProfile {
    pub base_dwell_ticks: u32,
    pub dwell_vigilance_scale_ticks: u32,
    pub vigilance: Permille,
    pub route_adaptation_sensitivity: Permille,
    pub patrol_motive_weight: Permille,
}

pub struct PatrolRoute {
    pub assigned_places: Vec<EntityId>,
    pub current_index: usize,
}

pub struct ViolationMemory {
    pub violations: BTreeMap<ViolationId, RecordedViolation>,
}
```

**State-mediated interactions:**
- Reads `ViolationMemory` → generates `InvestigateViolation` candidates
- Patrol dwell completion triggers advance to next waypoint → Travel action
- Hunger interrupts suspend patrol → resume from same waypoint after eating

**Golden tests:** `golden_patrol`

---

## 5. Cross-Cutting Infrastructure

### 5.1 Contention Queues

**What it is:** Exclusive access mechanism for shared facilities (workstations, care facilities, offices). Agents queue for access; only the granted agent can use the facility.

```rust
pub struct ContentionQueue {
    next_ordinal: u32,
    waiting: BTreeMap<u32, ContentionWaiter>,
    granted: Option<ContentionGrant>,
}

pub struct ContentionWaiter {
    pub actor: EntityId,
    pub intended_action: ActionDefId,
    pub queued_at: Tick,
}

pub struct ContentionGrant {
    pub actor: EntityId,
    pub intended_action: ActionDefId,
    pub granted_at: Tick,
    pub expires_at: Tick,
}

pub struct ContentionPolicy {
    pub grant_hold_ticks: NonZeroU32,
    pub auto_promote: bool,
    pub max_waiters: Option<u8>,
}

pub enum ContentionStatus {
    Unmanaged, Granted, Queued { position: u32 }, Available, Full,
}
```

**Operations:** `enqueue()` → `position_of()` → `promote_head()` → `clear_grant()`. Grant expires after `grant_hold_ticks`. Agent-side tracking via `ContentionIntents` and `ContentionDispositionProfile { queue_patience_ticks }`.

**Golden tests:** `golden_care` (medical resource contention), `golden_production`, `golden_resilience`

### 5.2 Force Control

**What it is:** Authority chain for entity manipulation. Determines whether an actor can exercise control over an entity (pick up, use, move, consume).

```rust
pub fn can_exercise_control(
    &self,
    actor: EntityId,
    entity: EntityId,
) -> Result<(), WorldError>
```

**Validation hierarchy (checked in order):**
1. Direct possession (`possessed_by[entity] == Some(actor)`)
2. Direct ownership (unpossessed) (`owned_by[entity] == Some(actor)`)
3. Faction delegation (actor in factions of entity owner)
4. Office delegation (actor holds office of entity owner)
5. Container traversal (recurse into direct container)

**Institutional projection:** Political events propagate force-control claims:
```rust
pub enum InstitutionalClaim {
    ForceControl { office, controller: Option<EntityId>, contested: bool, effective_tick },
    OfficeHolder { office, holder, effective_tick },
    FactionMembership { faction, member, active, effective_tick },
    SupportDeclaration { office, supporter, candidate, effective_tick },
}
```

**Golden tests:** `golden_offices`, `golden_t22_bandit_camp_destruction`

### 5.3 Event Log

**What it is:** Append-only causal record of everything that happens in the simulation. Source of truth for replay and perception.

```rust
pub struct EventLog {
    events: Vec<EventRecord>,
    next_id: EventId,
    by_tick: BTreeMap<Tick, Vec<EventId>>,
    by_actor: BTreeMap<EntityId, Vec<EventId>>,
    by_place: BTreeMap<EntityId, Vec<EventId>>,
    by_tag: BTreeMap<EventTag, Vec<EventId>>,
    by_cause: BTreeMap<EventId, Vec<EventId>>,
}

pub struct EventPayload {
    pub tick: Tick,
    pub cause: CauseRef,
    pub actor_id: Option<EntityId>,
    pub action_name: Option<String>,
    pub target_ids: Vec<EntityId>,
    pub evidence: Vec<EvidenceRef>,
    pub place_id: Option<EntityId>,
    pub state_deltas: Vec<StateDelta>,
    pub observed_entities: BTreeMap<EntityId, ObservedEntitySnapshot>,
    pub visibility: VisibilitySpec,
    pub witness_data: WitnessData,
    pub tags: BTreeSet<EventTag>,
}

pub enum CauseRef {
    Event(EventId),        // Caused by earlier event (must precede)
    SystemTick(Tick),      // Caused by system at tick N
    Bootstrap,             // Initial world creation
    ExternalInput(u64),    // From external input stream
}
```

**Causal invariants:**
- Every event has exactly one `CauseRef`
- Events form a DAG — backward causality only
- `emit()` validates cause exists and precedes new event
- `trace_event_cause()` walks backward to root

**Golden tests:** `golden_resilience` (T31 — causal link integrity per tick), `golden_determinism`

### 5.4 Perception Propagation

Information flows through the place graph via physical carriers:

1. **Direct observation** — co-location, `observation_fidelity` roll
2. **Event witnesses** — `VisibilitySpec` determines who can see
3. **Tell actions** — agent-to-agent belief sharing, relay chains with confidence degradation
4. **Institutional records** — `ConsultRecord` planner op for reading crime registers, office records
5. **Departure observation** — agents see travelers leave and project direction
6. **Mismatch detection** — contradictions between belief and observation generate events

**Confidence computation:**
```rust
pub fn belief_confidence(
    source: &PerceptionSource,
    staleness_ticks: u64,
    policy: &BeliefConfidencePolicy,
) -> Permille
// base(source) - chain_penalty(hops) - staleness_penalty(ticks)
```

Agents never query authoritative world state. All planning operates on `AgentBeliefStore`.

### 5.5 Belief Management

```rust
pub struct AgentBeliefStore {
    pub known_entities: BTreeMap<EntityId, BelievedEntityState>,
    pub social_observations: Vec<SocialObservation>,
    pub told_beliefs: BTreeMap<TellMemoryKey, ToldBeliefMemory>,
    pub heard_beliefs: BTreeMap<TellMemoryKey, HeardBeliefMemory>,
    pub asked_witnesses: BTreeMap<AskWitnessMemoryKey, AskWitnessMemory>,
    pub institutional_beliefs: BTreeMap<InstitutionalBeliefKey, Vec<BelievedInstitutionalClaim>>,
}

pub struct BelievedEntityState {
    pub last_known_place: Option<EntityId>,
    pub last_known_inventory: BTreeMap<CommodityKind, Quantity>,
    pub workstation_tag: Option<WorkstationTag>,
    pub resource_source: Option<ResourceSource>,
    pub alive: bool,
    pub wounds: Vec<Wound>,
    pub last_known_courage: Option<Permille>,
    pub believed_activity: Option<BelievedActivity>,
    pub believed_artifact: Option<BelievedArtifactState>,
    pub believed_contention: Option<BelievedContentionState>,
    pub observed_tick: Tick,
    pub source: PerceptionSource,
}
```

**Memory enforcement:**
- `enforce_capacity()` — evicts beliefs older than `memory_retention_ticks`, then LRU if over `memory_capacity`
- `enforce_conversation_memory()` — evicts old tell/heard memories
- Capacity bounds are per-agent via `PerceptionProfile` and `TellProfile`

### 5.6 Affordance Queries

**What it is:** Discovers what actions are available to an agent given their current beliefs and location.

```rust
pub fn get_affordances(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> Vec<Affordance>
```

Pipeline per action def:
1. Check actor constraints (all must pass)
2. Enumerate targets (target specs → entity bindings)
3. Check preconditions not covered by target specs
4. Expand payload variants (commodity selection, etc.)
5. Derive contention status (queue position if facility)

Affordances connect candidate generation to plan search — candidates specify goals, affordances specify available actions, and the planner bridges them.

---

## 6. Golden Test Coverage Map

| Test File | Scenarios | Goal Kinds Exercised | Action Domains | Systems Exercised |
|-----------|-----------|---------------------|----------------|-------------------|
| `golden_ai_decisions` | ~6 | ConsumeOwnedCommodity, AcquireCommodity, Wash, Relieve | Needs, Travel, Production, Transport | Needs, Production, AI Pipeline |
| `golden_production` | ~4 | AcquireCommodity, ProduceCommodity, Harvest, Craft | Production | Production, Needs |
| `golden_trade` | ~6 | SellCommodity, RestockCommodity, AcquireCommodity | Trade, Travel | Trade, Needs, Perception |
| `golden_merchant_selling` | ~4 | SellCommodity, RestockCommodity | Trade | Trade, Perception |
| `golden_supply_chain` | ~3 | RestockCommodity, MoveCargo, AcquireCommodity | Trade, Production, Travel | Trade, Production, Travel |
| `golden_commodity_opportunity` | ~3 | AcquireCommodity, RestockCommodity | Trade, Production | Trade, Production, Perception |
| `golden_emergent` | ~4 | Multiple (emergent multi-system) | Multiple | Full system stack |
| `golden_integration` | ~3 | Multiple (integration) | Multiple | Full system stack |
| `golden_combat` | ~9 | BuryCorpse, ReduceDanger, EngageHostile, LootCorpse, TreatWounds | Combat, Corpse, Care, Needs | Combat, Care, Corpse, Needs |
| `golden_patrol` | ~6 | Patrol, ConsumeOwnedCommodity, InvestigateViolation | Travel, Generic, Needs, Investigation | Patrol, Travel, Perception |
| `golden_pursuit` | 3 (S68-70) | RaidTarget, EngageHostile | Travel, Combat, Perception | Combat, Pursuit, Perception |
| `golden_care` | ~4 | TreatWounds, ShareBelief | Care, Social, Combat | Care, Combat, Social, Contention |
| `golden_offices` | ~12 | ClaimOffice, SupportCandidateForOffice | Generic, Social, Travel | Politics, Succession, Perception |
| `golden_social` | ~14 | ShareBelief, ConsumeOwnedCommodity, AcquireCommodity | Social, Needs, Travel, Production | Social, Tell, Perception, Needs |
| `golden_t22_bandit_camp_destruction` | 4 (T22,S47-49) | RaidTarget, RegroupWithFaction, EstablishBanditCamp, LootCorpse, ShareBelief | Combat, Travel, Social, Production, Corpse | BanditCamp, Combat, Perception, Travel |
| `golden_resilience` | 2 (T31,T32) | All goal kinds (20-agent stress test) | All domains | Full system stack + conservation |
| `golden_travel_physiology` | 4 (S58-61) | Relieve, AcquireCommodity | Needs, Travel | Needs, Travel, Metabolism |
| `golden_determinism` | 3 (S06,S02,S21) | ConsumeOwnedCommodity, AcquireCommodity | Multiple | Full stack + save/load |
| `golden_experience_preferences` | 3 (S91-93) | AcquireCommodity, Relieve | Travel, Production, Needs | Travel, Experience, Perception |
| `golden_reasoning_diversity` | 1 (S97) | AcquireCommodity, ConsumeOwnedCommodity | Production, Travel | Production, AI Planning |
| `golden_soak` | T30 (10,080 ticks) | All goal kinds (20 agents, 10 seeds) | All domains | Full stack + conservation invariants |
| `planner_conformance` | ~25 (S26) | Multiple per action family | All action families | Planner vs handler fidelity |

---

## 7. Architectural Observations

The following are patterns, asymmetries, and observations noted during analysis. These are descriptive, not prescriptive — they are flags for the external LLM to evaluate against FOUNDATIONS.md.

### 7.1 Belief-Only Planning Enforcement

The architecture rigorously separates authoritative state from agent beliefs. The `GoalBeliefView` and `RuntimeBeliefView` traits provide a narrow interface that agents plan against. Golden test S69 (pursuit with stale belief) explicitly validates that agents fail honestly when beliefs are outdated rather than omnisciently tracking targets. This is a strong enforcement of information locality.

### 7.2 Per-Agent Profile Diversity

Every behavioral parameter is per-agent via profile components (`MetabolismProfile`, `UtilityProfile`, `ReasoningProfile`, `PerceptionProfile`, `PreferenceProfile`, `CombatProfile`, `TellProfile`, `PatrolProfile`, `PursuitProfile`, `ContentionDispositionProfile`, `TradeDispositionProfile`). Golden test S93 validates that identical setups with different `route_caution_weight` produce different route choices. S97 validates that different `max_node_expansions` produces different plan depths. This creates behavioral diversity without authored behavioral scripts.

### 7.3 Suppression and Stress-Based Goal Filtering

The `GoalFamilyPolicy` system with `SuppressionRule::WhenStressedAtOrAbove` and `DecisionContext` creates a dynamic priority system where low-priority goals (gossip, looting, enterprise) are suppressed when agents are under survival or danger stress. This is an implicit prioritization mechanism — no explicit priority ranking is needed because stressed agents simply don't generate low-priority candidates.

### 7.4 Barrier Fallback in Plan Search

The A* search implements a barrier fallback strategy: if no `GoalSatisfied` terminal is found, the search returns the best `ProgressBarrier` plan. This means agents can make partial progress toward goals even when the full plan is unreachable. This is architecturally interesting because it creates "best-effort" behavior without explicit "fallback goal" design.

### 7.5 Fixed System Execution Order

The 9-system canonical execution order is explicitly documented as load-bearing. The rationale for each position is clear (deprivation before economics, deaths before camp abandonment, perception before patrol). The order creates implicit temporal dependencies between systems without direct coupling.

### 7.6 Conservation Verification

The `verify_authoritative_conservation` function checks commodity totals per tick during resilience tests. Golden test T31 runs this under random disruptions (agent death, item destruction, workstation removal, teleportation) to validate conservation even under chaos. This is a strong invariant enforcement mechanism.

### 7.7 Contention as Emergence Driver

The contention queue system creates emergent behavior from resource scarcity. When multiple agents want the same facility, only the granted agent can act — others must wait or choose alternative goals. This creates natural queuing behavior, spatial competition, and temporal coordination without authored scheduling. The `ContentionDispositionProfile.queue_patience_ticks` allows per-agent patience, so some agents abandon queues earlier than others.

### 7.8 Tell Memory and Gossip Dynamics

The `ToldBeliefMemory` and `HeardBeliefMemory` system prevents infinite gossip loops. An agent won't re-tell a belief to someone who already heard the same version. This creates natural information saturation — gossip propagates through the network and then stops. The `conversation_memory_retention_ticks` parameter means old conversations are eventually forgotten, enabling re-telling of updated information.

### 7.9 Route Learning as Emergent Preference

`RouteExperience` tracks hostile encounters per travel edge. Combined with `PreferenceProfile.route_caution_weight`, this creates personalized route preferences from experience — without any authored "safe route" or "dangerous route" labels. The architecture lets danger emerge from actual events and influence future decisions through the belief system.

### 7.10 Planner Conformance Testing

The `planner_conformance` tests validate that hypothetical transition semantics (`apply_hypothetical_transition`) directionally agree with real action handler outcomes. This is architecturally significant because the planner's model of the world must match the actual execution model — divergence would cause plans to fail systematically. Testing this explicitly prevents drift between planning and execution.

### 7.11 Institutional Knowledge Propagation

Institutional beliefs (`InstitutionalBeliefKey`, `BelievedInstitutionalClaim`) propagate through a different channel than entity beliefs. Institutional knowledge has its own `InstitutionalKnowledgeSource` variants (DirectObservation, WitnessedEvent, RecordConsultation, SelfDeclaration, Report) and its own memory capacity. This creates a two-track belief system: personal observations about entities, and institutional knowledge about offices, factions, and crimes.

### 7.12 DirtySet Flag System

The `DirtySet` differentiates between structural changes (requiring full replanning) and snapshot changes (allowing revalidation). This is an optimization that avoids unnecessary replanning when only beliefs changed — the existing plan can be revalidated instead. The granularity of the dirty flags determines how responsive agents are to environmental changes.

### 7.13 Observation Fidelity as Lever

`PerceptionProfile.observation_fidelity` (default 875‰ = 87.5%) means agents don't perceive everything at their location. This creates natural information asymmetry even among co-located agents. Combined with `memory_capacity` and `memory_retention_ticks`, it means agent beliefs are always partial and decaying — creating the conditions for mistakes, surprises, and emergent behavior from incomplete information.

### 7.14 Multi-Round Combat and Wound Accumulation

Combat is multi-round with wound accumulation. Wounds have severity, bleed rate, and body part — not abstract "hit points." The `flee_wound_threshold` in `BanditFactionPolicy` creates emergent retreat behavior when wounds accumulate beyond a per-faction tolerance. Golden test S49 validates that wound accumulation across multiple raids eventually suppresses further raid generation.

### 7.15 Tick Step Service Container

The `TickStepServices` struct aggregates all registries and optional trace sinks. The trace infrastructure (ActionTraceSink, PerceptionTraceSink, PoliticalTraceSink, InstitutionalKnowledgeTraceSink) is injection-optional — tests can capture detailed diagnostic traces without runtime overhead in production. This is a clean separation of concern between execution and observability.
