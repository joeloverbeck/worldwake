# GOAP Architecture Reference — 2026-04-11

## 1. Architecture Context

Worldwake is a causality-first emergent micro-world simulation in Rust. The codebase is a 5-crate workspace:

```
worldwake-core    → IDs, types, ECS store, topology, items, relations (no deps)
worldwake-sim     → Event log, action framework, scheduler, replay (deps: core)
worldwake-systems → Needs/metabolism, production/crafting, trade, combat, travel/transport actions (deps: core, sim)
worldwake-ai      → Pressure-based GOAP planner, goal ranking, decision runtime (deps: core, sim, systems)
worldwake-cli     → Human control interface (deps: all)
```

### ECS

Custom ECS (no external crate). All authoritative component storage uses `BTreeMap<EntityId, T>` for deterministic iteration order. No `HashMap`/`HashSet` in authoritative state. No floats anywhere — all fractional values use `Permille` (0–1000 integer range). Random number generation uses `ChaCha8Rng` with explicit seeds for full replay determinism.

### Key Foundational Types

```rust
/// Opaque typed entity handle. Wraps a u64.
pub struct EntityId(u64);

/// Integer permille (0–1000). Used for all fractional quantities.
pub struct Permille(u16);

/// Stack quantity for commodities.
pub struct Quantity(u32);

/// Discrete simulation tick. Monotonically increasing.
pub struct Tick(u64);
```

Entities have an `EntityKind` (Agent, Place, Item, etc.) and a `ControlSource` (Human, Ai, None). There is no `Player` type — only `ControlSource = Human | Ai | None`. The engine makes no rule distinction between human-controlled and AI-controlled agents.

### Belief Architecture

Agents plan from beliefs, never from world state directly. Two belief view traits provide the interface:

- **`RuntimeBeliefView`** — composite trait for action execution and affordance queries. Implements `ControlBeliefView`, `EntityBeliefView`, `ProfileBeliefView`, `SpatialBeliefView`, `TemporalBeliefView`, `InventoryBeliefView`, `CombatBeliefView`, `EconomicBeliefView`, `SocialBeliefView`, `PoliticalBeliefView`, `FacilityBeliefView`.
- **`GoalBeliefView`** — narrow surface for goal formation and ranking. Provides subjective reads (`effective_place`, `commodity_quantity`, `corpse_entities_at`, `seller_for_sale_lot`), self-authoritative reads (`homeostatic_needs`, `drive_thresholds`, `wounds`, `recipes`, `inventory`, `load`, `profiles`), and public structure reads (`entities_at`, `workstations`, `resource_sources`, `adjacent_places`).

Both views read from `AgentBeliefStore`, which is populated by the perception system. The belief store holds entity claims, commodity beliefs, spatial beliefs, social relationships, and institutional knowledge — all with provenance, confidence, and freshness metadata.

---

## 2. Goal Ranking

### How Needs Become Goals

The pipeline is: **homeostatic needs → candidate generation → ranking → planning**. Each agent has `HomeostaticNeeds` (hunger, thirst, fatigue, bladder, dirtiness) that accumulate over time. When a need crosses a drive threshold (configured per-agent via `DriveThresholds`), the candidate generation system emits goal candidates. For example, high hunger emits `AcquireCommodity { commodity: Food, purpose: SelfConsume }` and `ConsumeOwnedCommodity { commodity: Food }`.

### Pressure-Based Decision Context

Before ranking, a `DecisionContext` is built:

```rust
pub struct DecisionContext {
    pub max_self_care_class: GoalPriorityClass,  // Max urgency across all needs
    pub danger_class: GoalPriorityClass,          // Ambient danger level
}
```

`max_self_care_class` is the highest urgency band across hunger, thirst, fatigue, bladder, and dirtiness. `danger_class` derives from perceived threats via `derive_danger_pressure()`. Both use `GoalPriorityClass` bands (Background, Routine, Elevated, High, Critical).

### Ranking Algorithm

```rust
pub fn rank_candidates(
    candidates: &[GroundedGoal],
    view: &dyn GoalBeliefView,
    agent: EntityId,
    current_tick: Tick,
    utility: &UtilityProfile,
    decision_context: &DecisionContext,
) -> RankingOutcome
```

Algorithm:
1. For each candidate goal:
   a. **Suppression check**: `evaluate_suppression(&candidate.key.kind, &decision_context)`. If not `Available`, add to suppressed list and skip. Goals are suppressed when the agent's self-care or danger pressure exceeds the goal's allowed band (e.g., exploration goals only available during Background/Routine danger).
   b. **Provenance**: `goal_ranking_provenance()` derives whether goal is drive-driven (need pressure × utility weight) or danger-driven (danger pressure × danger weight).
   c. **Priority class**: `ranked_priority_class()` maps goal + context into a priority tier.
   d. **Motive score**: `ranked_motive_score()` computes the raw prioritization score (0–1000 permille range). Self-consume goals use pressure from current need levels. Production goals combine enterprise shortage signals. Expectation-response goals use time decay and basis weight.
   e. **Source reliability discount**: Reduces motive based on source entity's failure history. Formula: `discount_factor = trust_weight × failure_ratio / 1000`.
   f. **Competition discount**: Reduces motive for production/restock goals at crowded locations. Formula: `discount = 1 - (observed_competitors.min(3) × activity_awareness_weight) / 1000`.
   g. If motive == 0, add to `zero_motive` list and skip.
2. Sort ranked goals by `compare_ranked_goals` (primary key: motive score descending).
3. Return `RankingOutcome { ranked, suppressed, zero_motive }`.

### GoalKind Enum (31 variants)

```rust
pub enum GoalKind {
    ConsumeOwnedCommodity { commodity: CommodityKind },
    AcquireCommodity { commodity: CommodityKind, purpose: CommodityPurpose },
    Sleep,
    Relieve,
    Wash,
    FreeCarryCapacity,
    EngageHostile { target: EntityId },
    RaidTarget { target: EntityId },
    ReduceDanger,
    RegroupWithFaction { faction: EntityId },
    EstablishBanditCamp { faction: EntityId },
    TreatWounds { patient: EntityId },
    SearchForMissing { subject: EntityId, last_seen: Option<EntityId> },
    ReportMissing { subject: EntityId, to_office: Option<EntityId>, expectation_id: Option<ExpectationId> },
    ReportFound { subject: EntityId, expectation_id: ExpectationId },
    EscortToSafety { subject: EntityId, destination: EntityId },
    ProduceCommodity { recipe_id: RecipeId },
    SellCommodity { commodity: CommodityKind },
    RestockCommodity { commodity: CommodityKind },
    MoveCargo { commodity: CommodityKind, destination: EntityId },
    LootCorpse { corpse: EntityId },
    BuryCorpse { corpse: EntityId, burial_site: EntityId },
    FulfillBounty { bounty: EntityId },
    PostBounty { posting: ArtifactPostingContext, terms: BountyTerms },
    PostNotice { posting: ArtifactPostingContext, topic: NoticeTopic },
    ShareBelief { listener: EntityId, topic: TellTopic, communication_class: CommunicationClass },
    ClaimOffice { office: EntityId },
    SupportCandidateForOffice { office: EntityId, candidate: EntityId },
    InvestigateViolation { violation_id: ViolationId, place: EntityId },
    Patrol { place: EntityId },
    ExploreLocation { target_place: EntityId, motivating_need: HomeostaticNeedId },
    StealItem { target_item: EntityId },
    Accuse { crime_register: EntityId, accused: EntityId, violation_id: ViolationId },
    PunishAccused { office: EntityId, accused: EntityId, accusation_entry: RecordEntryId, punishment: PunishmentKind },
}
```

Supporting types:

```rust
pub enum CommodityPurpose { SelfConsume, Restock, RecipeInput(RecipeId) }

pub struct GoalKey {
    pub kind: GoalKind,
    pub commodity: Option<CommodityKind>,
    pub entity: Option<EntityId>,
    pub place: Option<EntityId>,
}

/// Distinguishes one opportunity from another for the same desire.
pub enum OpportunityAnchor {
    Place(EntityId),
    Entity(EntityId),
    None,
}
```

### Output Structure

```rust
pub struct RankedGoal {
    pub grounded: GroundedGoal,
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
    pub provenance: Option<RankedGoalProvenance>,
    pub source_reliability_discount: Option<SourceReliabilityDiscount>,
    pub competition_discount: Option<CompetitionDiscount>,
    pub feasibility: FeasibilityHint,
}

pub struct RankingOutcome {
    pub ranked: Vec<RankedGoal>,
    pub suppressed: Vec<GoalKey>,
    pub zero_motive: Vec<GoalKey>,
}
```

---

## 3. Candidate Generation

### Entry Point

```rust
pub fn generate_candidates(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    blocked: &BlockedIntentMemory,
    recipes: &RecipeRegistry,
    current_tick: Tick,
) -> Vec<GroundedGoal>
```

Delegates to the full version with `travel_horizon=6` and tracing disabled.

### Full Pipeline

```rust
pub fn generate_candidates_with_travel_horizon(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    blocked: &BlockedIntentMemory,
    violation_memory: &ViolationMemory,
    recipes: &RecipeRegistry,
    current_tick: Tick,
    travel_horizon: u8,
    tracing_enabled: bool,
) -> CandidateGenerationResult
```

Algorithm:
1. If agent is dead → return empty.
2. Build `GenerationContext` with agent's effective place, travel horizon, enterprise signals, blocked memory, recipes, current tick.
3. Emit candidates through 17 specialized gate functions (sequentially):
   - `emit_need_candidates()` — food, water, sleep, sanitation, medicine
   - `emit_production_candidates()` — crafting, harvesting
   - `emit_enterprise_candidates()` — economic opportunity-driven
   - `emit_disposal_candidates()` — drop unwanted items
   - `emit_bounty_candidates()` — respond to posted bounties
   - `emit_artifact_posting_candidates()` — post artifacts for rewards
   - `emit_combat_candidates()` — raid/defend based on threats
   - `emit_crime_candidates()` — theft/trespassing/violations
   - `emit_social_candidates()` — tell/gossip based on beliefs
   - `emit_patrol_candidates()` — guard duty, investigation
   - `emit_political_candidates()` — office claims, support declarations
   - `emit_recorded_violation_candidates()` — follow-up on violations
   - `emit_search_candidates()` — find missing persons/items
   - `emit_report_found_candidates()` — report found persons
   - `emit_escort_candidates()` — protect/escort travelers
   - `emit_exploration_candidates()` — explore unknown locations
   - `emit_expectation_violation_candidates()` — respond to violated expectations
4. **Filter blocked candidates** via `filter_blocked_candidates()`:
   - For each candidate, check if `find_matching_blocker()` returns an active blocker (not expired, matches goal key, passes `candidate_matches_blocker()` including opportunity anchor check).
   - Track fully blocked desires in diagnostics (all instances of a goal key blocked).
5. Return `CandidateGenerationResult { candidates, diagnostics, pending_violations }`.

### Emission Gates

Each emission function filters candidates through:
1. **Legality checks** — is the action legal under the current institutional regime?
2. **Feasibility gates** — does the agent have required commodities/capabilities?
3. **Belief evidence** — is there belief support (not pure speculation)?
4. **Travel cost** — is the opportunity within `travel_horizon` distance?

### Evidence Tracing

Each candidate maintains an `EvidenceTrace`:

```rust
pub struct EvidenceTrace {
    pub contributors: BTreeSet<CandidateEvidenceContributor>,
    pub exclusions: BTreeSet<CandidateEvidenceExclusion>,
    pub knowledge_path: KnowledgePath,
    pub legality: Option<CandidateLegalityTrace>,
    pub pursuit: Option<PursuitDiagnostic>,
}

pub struct CandidateEvidenceContributor {
    pub kind: CandidateEvidenceKind,
    pub place: EntityId,
    pub entity: EntityId,
}
```

### Locality Scoping

All candidate generation operates through `GoalBeliefView`, which returns only what the agent believes. `effective_place()` determines the agent's believed location. Entities, facilities, and merchants are visible only if they exist in the agent's belief store at known places within `travel_horizon` hops on the place graph. This enforces information locality (FND-7): agents cannot generate goals about things they don't know about.

### How Candidate Count Relates to Branching Factor

The number of candidates determines how many goals the planner considers. After ranking, only the top `max_candidates_to_plan` (default: 2) are sent to plan search. Each candidate may produce a different plan with different action sequences. In plan search itself, the branching factor is determined by the number of legal affordances at each search state, not by the candidate count.

---

## 4. Affordance Queries

### Core Function

```rust
pub fn get_affordances_for_defs(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    allowed_defs: &BTreeSet<ActionDefId>,
) -> Vec<Affordance>
```

Algorithm:
1. Filter registry to `allowed_defs`.
2. For each action def, skip if actor constraints fail.
3. Enumerate target bindings via `enumerate_targets()` for each `TargetSpec` in the def.
4. Filter by preconditions.
5. Expand payload variants via handler's `affordance_payloads()` callback.
6. Sort and deduplicate results.

### Target Scoping

`TargetSpec` defines what kinds of entities can fill each target slot:

- **EntityAtActorPlace** — entities co-located with the actor
- **AdjacentPlace** — neighboring places in the topology
- **Self** — the actor itself
- Other variants for specific entity types (offices, facilities, corpses, etc.)

Target enumeration calls the handler's `affordance_targets()` callback when `uses_dynamic_affordance_targets` is true, allowing handlers to provide custom target lists.

### ActionDef Structure

```rust
pub struct ActionDef {
    pub id: ActionDefId,
    pub name: String,
    pub domain: ActionDomain,
    pub actor_constraints: Vec<Constraint>,
    pub targets: Vec<TargetSpec>,
    pub preconditions: Vec<Precondition>,
    pub reservation_requirements: Vec<ReservationReq>,
    pub duration: DurationExpr,
    pub body_cost_per_tick: BodyCostPerTick,
    pub attention_cost: Permille,
    pub interruptibility: Interruptibility,
    pub commit_conditions: Vec<Precondition>,
    pub visibility: VisibilitySpec,
    pub causal_event_tags: BTreeSet<EventTag>,
    pub payload: ActionPayload,
    pub handler: ActionHandlerId,
}
```

### ActionHandler Structure

```rust
pub struct ActionHandler {
    pub on_start: ActionStartFn,
    pub on_tick: ActionTickFn,
    pub on_commit: ActionCommitFn,
    pub on_abort: ActionAbortFn,
    pub on_start_failure: ActionStartFailureFn,
    pub affordance_targets: AffordanceTargetsFn,
    pub uses_dynamic_affordance_targets: bool,
    pub affordance_payloads: AffordancePayloadFn,
    pub requires_explicit_payload_variants: bool,
    pub payload_override_is_valid: PayloadOverrideValidatorFn,
    pub authoritative_payload_is_valid: AuthoritativePayloadValidatorFn,
}
```

### relevant_ops Per Goal Kind

During plan search, `relevant_action_defs()` filters the action registry to only those defs whose `PlannerOpSemantics` can contribute to the current goal. For example:
- `ConsumeOwnedCommodity` → consume actions
- `AcquireCommodity` → trade, pickup, harvest actions
- `ProduceCommodity` → craft actions
- `TreatWounds` → heal actions
- `RaidTarget` → attack + travel actions
- `ExploreLocation` → explore + travel actions

This prevents the planner from considering irrelevant actions during search, reducing the effective branching factor.

---

## 5. Plan Search Pipeline

### Two-Phase Architecture: Strategic + Tactical

The planner uses a two-phase approach. Phase 1 (strategic) computes a high-level location-visit itinerary. Phase 2 (tactical) runs A* search at each location to find concrete action sequences.

### Phase 1: Strategic Planner

**File**: `crates/worldwake-ai/src/search/strategic.rs`

```rust
pub(crate) struct StrategicPlan {
    pub steps: Vec<StrategicStep>,
}

pub(crate) struct StrategicStep {
    pub destination: EntityId,          // Place to travel to
    pub sub_goal: TacticalSubGoal,      // What to do there
    pub estimated_travel_ticks: u32,    // Cost estimate
}

pub(crate) enum TacticalSubGoal {
    SatisfyGoal,                        // Complete the main goal here
    AcquirePrerequisite(CommodityKind), // Pick up a needed ingredient
    Explore,                            // Explore an unknown location
    SocialQuery(CommodityKind),         // Ask about commodity sources
}
```

```rust
pub(crate) fn plan(
    snapshot: &PlanningSnapshot,
    goal: &GroundedGoal,
    execution_budget: &ExecutionBudget,
    recipes: &RecipeRegistry,
) -> Option<StrategicPlan>
```

Algorithm (breadth-first search over prerequisite stages):
1. Check if goal already satisfied at current location → return empty plan.
2. Determine `goal_places`: all believed locations where the goal can be satisfied.
3. Determine `missing_commodities`: prerequisites not yet held by the agent.
4. Build stages as a multi-stage search:
   - Stage 0: satisfy current-place prerequisites (local commodities)
   - Stage 1..N: travel to acquire each missing commodity
   - Final stage: travel to a goal location
5. BFS over (stage, location, accumulated_cost) states:
   - Start at actor's current place, stage 0.
   - For each location in current stage's candidate places, calculate travel cost.
   - Prune: if (stage, place) seen with lower cost, skip.
   - On reaching final stage at a goal place, return plan with steps.
6. `max_prerequisite_locations` (default: 3) limits how many locations per stage are explored.

**Purpose**: Decompose multi-location objectives (e.g., "go to mill, acquire flour, return to bakery, bake bread") into a high-level itinerary that guides tactical search.

### Phase 2: Tactical A* Search

**File**: `crates/worldwake-ai/src/search/mod.rs`

```rust
pub fn search_plan(
    snapshot: &PlanningSnapshot,
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    cognitive: &CognitiveProfile,
    execution_budget: &ExecutionBudget,
    recipes: &RecipeRegistry,
    blocked: &BlockedIntentMemory,
    current_tick: Tick,
    binding_rejections: Option<&mut Vec<BindingRejection>>,
    expansion_summaries: Option<&mut Vec<SearchExpansionSummary>>,
) -> PlanSearchResult
```

```rust
pub enum PlanSearchResult {
    Found(Box<PlannedPlan>),
    Unsupported,
    BudgetExhausted { expansions_used: u16 },
    FrontierExhausted { expansions_used: u16 },
}
```

**Main A* Loop**:
1. Pre-compute relevant action defs: `relevant_action_defs(goal, semantics_table)`.
2. Build strategic plan (if applicable) to guide search.
3. Initialize `DualFrontier` with root node.
4. Initialize empty `LandmarkSet`.
5. Track `best_barrier: Option<PlannedPlan>` (best progress barrier found so far).
6. **While frontier not empty**:
   a. `node = frontier.pop()` — best node via A* ordering.
   b. If `goal.is_satisfied(node.state)` → return `Found(node.steps)`.
   c. If `node.steps.len() >= max_plan_depth` → skip (depth limit).
   d. If `expansions >= max_node_expansions` → return `best_barrier` or `BudgetExhausted`.
   e. `expansions += 1`.
   f. Generate candidates via `search_candidates()`:
      - Get all legal affordances for current state.
      - Filter by goal-relevance using `PlannerOpSemantics`.
      - Apply tactical goal filter (if two-phase search active).
      - Prune travel moves away from goal-relevant places.
   g. Build successors via `build_successor_detailed()`:
      - Compute payload override via `goal.build_payload_override()`.
      - Estimate action duration.
      - Apply hypothetical state transition (belief update).
      - Check if action is terminal (goal satisfied, combat commitment, progress barrier).
      - Compute A* costs:
        * `search_cost`: accumulated cost (travel: perceived travel cost; non-travel: duration)
        * `heuristic_ticks`: `max(spatial_heuristic, landmark_heuristic)`
        * `total_estimated_ticks`: sum of all action durations
   h. For each successor:
      - If terminal goal satisfied → sort by cost, return immediately.
      - If progress barrier → save as `best_barrier` if better than current.
      - Otherwise → add to frontier (preferred or regular queue based on landmark guidance).
7. Return appropriate result.

### Search State Representation

```rust
struct SearchNode<'snapshot> {
    state: PlanningState<'snapshot>,     // Simulated belief state
    steps: SharedVec<PlannedStep>,       // Plan built so far
    total_estimated_ticks: u32,          // Sum of all step durations
    search_cost: u32,                    // A* g(n)
    heuristic_ticks: u32,               // A* h(n)
}

struct PlannedStep {
    def_id: ActionDefId,
    targets: Vec<PlanningEntityRef>,
    payload_override: Option<ActionPayload>,
    op_kind: PlannerOpKind,
    estimated_ticks: u32,
    is_materialization_barrier: bool,
    expected_materializations: Vec<ExpectedMaterialization>,
}
```

### Landmark Extraction (Delete-Relaxation)

**File**: `crates/worldwake-ai/src/search/landmarks.rs`

```rust
pub(super) enum PlanningFact {
    AtPlace(EntityId),
    HasCommodity(CommodityKind),
    HasEntity(EntityId),
    FacilityAvailable(EntityId),
    EntityPresent(EntityId),
    NeedSatisfied(HomeostaticNeedId),
}

pub(super) struct LandmarkSet {
    pub landmarks: BTreeSet<PlanningFact>,
    pub orderings: Vec<(PlanningFact, PlanningFact)>,
}
```

```rust
pub(super) fn extract_landmarks(
    initial_facts: &BTreeSet<PlanningFact>,
    goal_facts: &BTreeSet<PlanningFact>,
    operators: &[PlanningOperator],
    max_depth: u8,
) -> LandmarkSet
```

Algorithm (backward chaining from goal facts):
1. Initialize `landmarks = goal_facts`.
2. Queue each goal fact with depth=0.
3. While queue not empty:
   a. Pop `(fact, depth)`.
   b. If `depth >= max_depth` or `fact ∈ initial_facts` → skip.
   c. Find all operators that ADD this fact (achievers).
   d. If no achievers → fact is unachievable landmark (no ordering added).
   e. Otherwise, compute **shared preconditions** = intersection of preconditions across ALL achievers.
   f. For each shared precondition:
      - Add ordering: `(predecessor_fact, current_fact)`.
      - Mark predecessor as landmark.
      - Enqueue `(predecessor, depth+1)` if new.
4. Return `LandmarkSet { landmarks, orderings }`.

**Key insight**: Landmarks are facts that EVERY valid plan must achieve. By finding shared preconditions of ALL achievers, the algorithm identifies mandatory intermediate states. `max_depth` (default: 4, from `CognitiveProfile.landmark_extraction_depth`) controls how far back the chain extends.

### Preferred Operators

```rust
pub(super) fn preferred_operators(
    landmarks: &LandmarkSet,
    current_facts: &BTreeSet<PlanningFact>,
    candidates: &[SearchCandidate],
    operators: &[PlanningOperator],
) -> BTreeSet<usize>
```

Returns indices of candidates whose operators achieve **actionable landmarks** — landmarks that are (a) not yet achieved and (b) whose prerequisites are satisfied in the current state. These candidates get priority in the dual frontier.

### Dual Frontier

**File**: `crates/worldwake-ai/src/search/frontier.rs`

```rust
pub(super) struct DualFrontier<'snapshot> {
    regular: BinaryHeap<FrontierEntry<'snapshot>>,
    preferred: BinaryHeap<FrontierEntry<'snapshot>>,
    boost_remaining: u8,
    preferred_operator_boost: u8,
    use_preferred_next: bool,
}
```

The `pop()` method uses an alternating strategy:
1. If `boost_remaining > 0` OR `use_preferred_next`:
   - Try `preferred.pop()` first.
   - Fall back to `regular.pop()` if preferred is empty.
   - Decrement `boost_remaining` if boost was active.
2. Else:
   - Try `regular.pop()` first.
   - Fall back to `preferred.pop()` if regular is empty.
3. Toggle `use_preferred_next` each pop.

`trigger_boost()` sets `boost_remaining = preferred_operator_boost` (default: 2, from `ExecutionBudget`). Called when a landmark-achieving action is expanded.

**Effect**: Prefers landmark-achieving actions but allows fallback to standard A* to maintain exploration breadth.

### Heuristics

**Spatial heuristic** (`heuristic.rs`):
```rust
pub fn compute_heuristic(
    snapshot: &PlanningSnapshot,
    state: &PlanningState<'_>,
    goal_relevant_places: &[EntityId],
) -> u32
```
Returns minimum perceived travel cost from actor's current simulated position to nearest goal-relevant place (0 if already at one). Uses the snapshot's pre-computed distance matrix.

**Landmark heuristic**:
```rust
pub fn compute_landmark_heuristic(
    landmarks: &LandmarkSet,
    current_facts: &BTreeSet<PlanningFact>,
) -> u32
```
Returns count of actionable landmarks (not yet achieved, prerequisites met).

**Combined**: `h(n) = max(spatial_heuristic, landmark_heuristic)`.

### Node Ordering (A* f-value comparison)

```rust
pub fn compare_search_nodes(left: &SearchNode<'_>, right: &SearchNode<'_>) -> Ordering
```

Cascade:
1. `f = search_cost + heuristic_ticks` — lower is better (standard A*).
2. Tie-break: lower `search_cost` (prefer shallower paths).
3. Tie-break: lower `total_estimated_ticks` (prefer faster-executing plans).
4. Tie-break: fewer steps (prefer shorter plans).
5. Tie-break: lexicographic comparison of step vectors.

### Beam Truncation

The search uses `beam_width` (default: 8, from `ExecutionBudget`) for frontier-level truncation. When the frontier at a given depth level exceeds `beam_width`, only the top nodes (by f-value) are retained. Combined with `max_node_expansions` (default: 224, from `CognitiveProfile`), this bounds total search effort.

### PlanningSnapshot as Belief Surface

```rust
pub struct PlanningSnapshot {
    pub(crate) actor: EntityId,
    pub(crate) current_tick: Tick,
    pub(crate) actor_belief_store: AgentBeliefStore,
    pub(crate) entities: BTreeMap<EntityId, SnapshotEntity>,
    pub(crate) places: BTreeMap<EntityId, SnapshotPlace>,
    // + internal distance matrix, spatial index
}
```

Built by `build_planning_snapshot()` from the agent's belief view. Contains:
- `SnapshotEntity`: comprehensive entity state (core, spatial, inventory, combat, social, economic, political, temporal, profiles, facility, control).
- `SnapshotPlace`: place metadata (entities present, tags, adjacencies, bandit camps).
- Pre-computed distance matrix for fast spatial heuristic queries.

The snapshot is the planner's belief surface — it cannot see anything the agent doesn't believe. `max_snapshot_entities_per_place` (default: 50) limits entity inclusion per place.

---

## 6. Plan Revalidation & Execution

### Plan Revalidation

**File**: `crates/worldwake-ai/src/plan_revalidation.rs`

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

Algorithm:
1. Resolve action def and handler from registries.
2. Resolve planning targets via materialization bindings (hypothetical → actual entity mapping).
3. Call `get_affordances_for_defs()` to get all legal affordances for actor/targets.
4. Check `requested_affordance_matches()` — does any affordance match the planned step?
5. Fallback paths if no exact match:
   a. `revalidate_best_effort_payload_override_step()` — step has payload override; check actor constraints, preconditions, and handler's `payload_override_is_valid()` callback.
   b. `revalidate_exact_target_step()` — action def has specific entity targets; synthesize affordance from step targets and check via `requested_affordance_matches()`.

Returns `true` if step can execute, `false` if revalidation failed.

### requested_affordance_matches

**File**: `crates/worldwake-sim/src/affordance_query.rs`

```rust
pub fn requested_affordance_matches(
    affordance: &Affordance,
    def: &ActionDef,
    handler: &ActionHandler,
    actor: EntityId,
    targets: &[EntityId],
    payload_override: Option<&ActionPayload>,
    view: &dyn RuntimeBeliefView,
) -> bool
```

Checks: affordance has correct `def_id`, all preconditions hold, constraints satisfied, targets available/accessible, payload override valid (if present). For actions with planner-synthesized payloads (not affordance-derived), the handler's `payload_override_is_valid` callback is critical — without it, revalidation silently fails.

### Pursuit Plan Revalidation

```rust
pub fn is_pursuit_plan_invalid(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    plan: &PlannedPlan,
    current_tick: Tick,
) -> Option<PursuitInvalidationReason>
```

Pursuit plans (RaidTarget, EngageHostile with travel + attack steps) are additionally checked:
- Target believed dead → invalidate.
- Target's believed place unknown → invalidate.
- Target's believed place ≠ plan destination (moved) → invalidate.
- Confidence in location below `min_location_confidence` (decays with time since last observation) → invalidate.

### Action Dispatch (BestEffort)

**File**: `crates/worldwake-ai/src/agent_tick/execution.rs`

After revalidation passes:
1. Resolve `PlanningEntityRef` targets via `resolve_step_targets()`:
   - Authoritative refs pass through.
   - Hypothetical refs look up in `MaterializationBindings` (mapping planning-time hypothetical entities to actual created entities).
2. Create `InputKind::RequestAction` and enqueue on scheduler's input queue:
   - `actor`, `def_id`, resolved `targets`, `payload_override`
   - `mode: ActionRequestMode::BestEffort` — system attempts action; if preconditions fail, action aborts
   - `provenance: RequestProvenance::AiPlan`
3. Set `runtime.step_in_flight = true`.

If revalidation fails:
1. Attempt `handle_recoverable_travel_step_blockage()` — if this is a travel step blocked by path, agent waits one tick and retries.
2. Otherwise: trigger full plan failure handling.

### Materialization Binding

When an action commits and creates new entities (e.g., crafting produces items):

```rust
pub fn apply_step_materialization_bindings(
    runtime: &mut AgentDecisionRuntime,
    step: &PlannedStep,
    outcome: &CommitOutcome,
) -> Result<(), ()>
```

Maps hypothetical entity IDs (from planning) to actual entity IDs (from execution). Future plan steps can then reference these newly created entities by looking up their hypothetical IDs in the bindings.

---

## 7. Replanning

### handle_plan_failure

**File**: `crates/worldwake-ai/src/failure_handling.rs`

```rust
pub fn handle_plan_failure(
    context: &PlanFailureContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    jc: &mut Option<IntentionFrame>,
    blocked_memory: &mut BlockedIntentMemory,
    facility_intents: &mut ContentionIntents,
    cognitive: &CognitiveProfile,
)

pub struct PlanFailureContext<'a> {
    pub view: &'a dyn RuntimeBeliefView,
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub failed_step: &'a PlannedStep,
    pub execution_failure: Option<ExecutionFailure<'a>>,
    pub current_tick: Tick,
}
```

Algorithm:
1. **Clear state**: Clear current plan, intention frame, materialization bindings, facility intents.
2. **Classify failure** via `derive_blocking_fact()`:
   - `TargetGone` — target entity died/despawned
   - `NoKnownPath` — cannot reach location
   - `NoAvailableSeller` — no merchant has the commodity
   - `SellerOutOfStock` — seller lacks commodity
   - `SellerTooExpensive` — cannot afford seller's price
   - `ResourceExhausted` — production resource depleted
   - `ToolMissing` — required crafting tool absent
   - `InputMissing` — consumption input not held
   - `CombatTooRisky` — enemy threat exceeds tolerance
   - `DangerTooHigh` — ambient danger too severe
   - `Unknown` — execution failed for unknown reason
3. **Compute TTL** via `blocking_fact_ttl()`:
   - `transient_block_ticks` (default: 20) for: SellerOutOfStock, InputMissing, CombatTooRisky
   - `unknown_block_ticks` (default: 5) for: Unknown
   - `structural_block_ticks` (default: 200) for: TargetGone, NoKnownPath, ResourceExhausted
4. **Derive clearing condition and baseline**:
   ```rust
   pub enum ClearingCondition {
       OnPositionChange,
       OnCommodityChange(CommodityKind),
       OnNeedsChange,
       OnHealthChange,
       OnWoundsHealed,
       Manual,
   }
   ```
   Baseline captures agent state at failure (position, needs, commodity quantities, wound count).
5. **Record BlockedIntent** in memory:
   ```rust
   pub struct BlockedIntent {
       pub blocker_key: BlockerKey,
       pub blocking_fact: BlockingFact,
       pub observed_tick: Tick,
       pub expires_tick: Tick,
       pub clearing_condition: ClearingCondition,
       pub baseline_snapshot: ClearingBaseline,
       pub diagnostic_context: Option<BlockerDiagnostic>,
   }
   ```
6. **Signal replan**: `runtime.dirty.insert(DirtySet::REPLAN_SIGNAL)`.

### Replan Triggers

- **Action failure**: execution handler returns failure → `handle_plan_failure()`.
- **Revalidation failure**: next planned step no longer legal (target moved, preconditions changed).
- **Pursuit invalidation**: target moved/died/confidence decayed.
- **Budget exhaustion**: planner couldn't find a plan within expansion limits.
- **Belief contradiction**: perception reveals the world has changed (blocker clearing condition met).

### How the Agent Re-enters the Pipeline

When a blocker expires (TTL elapsed) or clears (clearing condition met — e.g., agent moved, commodity acquired, needs changed):
1. `clear_resolved_blockers()` removes the expired/cleared `BlockedIntent` from memory.
2. On next decision tick, `generate_candidates()` re-emits previously blocked goals.
3. `rank_candidates()` re-ranks all goals (priorities may have shifted due to elapsed time).
4. Top candidates enter `search_plan()` from scratch with updated belief snapshot.

The agent's `initial_cooldown_ticks` (default: 4) and `max_cooldown_ticks` (default: 64) provide exponential backoff for repeated planning failures on the same goal.

---

## 8. Cognitive Parameters

### CognitiveProfile (per-agent planning parameters)

| Field | Type | Default | Effect |
|-------|------|---------|--------|
| `max_candidates_to_plan` | `u8` | 2 | Maximum ranked goals sent to plan search per decision tick |
| `max_plan_depth` | `u8` | 8 | Maximum steps in a single plan |
| `snapshot_travel_horizon` | `u8` | 6 | Maximum place-graph hops for candidate generation |
| `max_node_expansions` | `u16` | 224 | Total A* node expansion budget per search |
| `switch_margin` | `Permille` | 100 | Hysteresis for switching active goals (motive difference threshold) |
| `planning_switch_margin` | `Permille` | 150 | Hysteresis for switching planning targets |
| `transient_block_ticks` | `u32` | 20 | TTL for transient blockers (out of stock, input missing) |
| `unknown_block_ticks` | `u32` | 5 | TTL for unknown-cause blockers |
| `structural_block_ticks` | `u32` | 200 | TTL for structural blockers (target gone, no path) |
| `initial_cooldown_ticks` | `u32` | 4 | Initial backoff on repeated plan failure |
| `max_cooldown_ticks` | `u32` | 64 | Maximum backoff cap |
| `max_snapshot_entities_per_place` | `u16` | 50 | Entity count limit per place in planning snapshot |
| `speculative_acquisition` | `bool` | false | Whether to consider known places without current positive evidence |
| `landmark_extraction_depth` | `u8` | 4 | Depth of backward-chaining landmark extraction (0 = disabled) |

### ExecutionBudget (per-agent search bounds)

| Field | Type | Default | Effect |
|-------|------|---------|--------|
| `beam_width` | `u8` | 8 | Maximum nodes retained per frontier level |
| `max_prerequisite_locations` | `u8` | 3 | Locations explored per stage in strategic planner |
| `preferred_operator_boost` | `u8` | 2 | Consecutive preferred-queue expansions before alternating |

### Agent Diversity (FND-22)

These profiles are per-agent ECS components registered on `EntityKind::Agent`. Scenarios define them via `AgentDef` + `spawn_agent()`. Different agents can have dramatically different planning behavior:

- **Search depth divergence**: Agent with `max_node_expansions: 2` finds only shallow plans, while default (224) agent finds multi-step chains. Tested in `golden_reasoning_diversity::search_depth_divergence()`.
- **Landmark depth divergence**: Agent with `landmark_extraction_depth: 0` loses landmark guidance and degenerates to null heuristic, expanding more nodes for the same plan quality. Tested in `golden_reasoning_diversity::landmark_depth_divergence()`.
- **Utility profile diversity**: Different `UtilityProfile` weights cause agents with identical beliefs to rank goals differently (e.g., one prioritizes eating, another drinking). Tested in `golden_reasoning_diversity::golden_utility_profile_diversity()`.
- **Speculative acquisition**: Agents with `speculative_acquisition: true` consider known-but-unconfirmed resource locations, producing more candidates but also more plan failures.
- **Memory and perception**: `PerceptionProfile` controls observation fidelity, memory capacity, retention, and contradiction tolerance — all feeding into what the belief store contains, which determines what the planner can see.

---

## 9. FOUNDATIONS Alignment

### FND-1: Maximal Emergence Through Local Causality

> Worldwake exists to produce emergent behavior through interacting systems and agents, never through authored sequences, hidden quest logic, or one-off story triggers. An event is valid only if it arose from prior world state, agent belief, institutional rule, or natural process already present in the simulation.
>
> Authoring beasts, hunger, roads, caravans, towns, offices, and bounty procedures is correct. Authoring "a beast attack happens so adventurers have content" is forbidden.
>
> **Test**: If the only honest explanation for an event is "the game decided something interesting should happen now," the design violates this principle.

**Architecture alignment**: Strong. The GOAP pipeline produces behavior entirely from agent beliefs, needs, and utility weights. No goal kind exists to "make something interesting happen." Every goal traces to concrete state: hunger drives food acquisition, wounds drive care-seeking, threats drive combat or flight. The 31 GoalKind variants are all grounded in world conditions, not authored sequences.

**Potential tension**: The `relevant_ops` per goal kind mapping is a form of encoded decomposition knowledge — "to satisfy hunger, consider trade/harvest/pickup actions." This is legal under FND-20 ("the implementation may evolve — GOAP, utility systems, BDI, HTN, or hybrids are all acceptable") but the mapping is a design-time authoring choice, not an emergent discovery.

### FND-3: Concrete State Over Abstract Scores

> Prefer modeling the thing itself over a score that represents it. Danger should come from actual threats on routes, not `danger_score`. Scarcity should come from inventories, queues, failed purchases, and unmet needs, not `scarcity_score`.
>
> Abstract summaries are allowed only as derived views or caches. They may never become the source of truth.
>
> **Test**: If a system relies on a number that cannot be traced back to concrete entities, relations, or events, the design violates this principle.

**Architecture alignment**: Strong. Motive scores in ranking are derived views: hunger pressure comes from actual `HomeostaticNeeds` levels, competition discount from observed competitors at specific places, source reliability from remembered failure history. The `PlanningSnapshot` is a derived view of the belief store (itself a derived view of perceived world state).

**Potential tension**: The `GoalPriorityClass` bands (Background, Routine, Elevated, High, Critical) are abstract classifications of concrete state. They compress continuous need levels into discrete bands, which is a permissible derived summary under FND-3 but means the ranking logic loses some granularity at band boundaries.

### FND-7: Locality of Motion, Interaction, and Communication

> All physical interaction requires co-location or explicit range. All communication requires co-location or a physical carrier moving through the place graph.
>
> Agents, institutions, and planners may not query global truth on behalf of a character.
>
> **Test**: For any belief, report, or institutional action, trace the path by which the relevant information arrived. If no path exists, the design violates locality.

**Architecture alignment**: Strong. The entire pipeline operates through belief views, never world state. `GoalBeliefView` returns only what the agent has perceived, been told, or inferred. Candidate generation scopes to `travel_horizon` on the place graph. `PlanningSnapshot` captures only believed entities. The affordance query in revalidation uses `RuntimeBeliefView`.

**No identified gaps**: The planner genuinely cannot see what the agent doesn't believe.

### FND-12: Performance May Compress Computation, Never Causality

> Optimization is allowed. Causal cheating is not. Offscreen simulation may batch, summarize, sleep, or approximate only if causally relevant outcomes remain equivalent to the explicit model.
>
> **Test**: If an optimization or boundary causes an agent to observe a state that could not have arisen from any legal sequence of world events, the optimization is invalid.

**Architecture alignment**: Strong. The planner's `max_node_expansions`, `beam_width`, and `max_plan_depth` compress computation (bounded search). They never alter world state or create phantom observations. A budget-exhausted search simply fails to find a plan — the agent idles rather than taking an impossible action.

**No identified gaps**: Search bounds affect plan quality (agent may miss optimal plans) but never cause illegal state transitions.

### FND-14: World State Is Not Belief State

> Ground truth and agent knowledge are separate layers. Agents act on what they believe, remember, infer, suspect, and are told — not on what the simulation knows to be true.
>
> A planner may consult only the agent's accessible belief state, memory, and known plans. No AI may silently use omniscient world data to make "smarter" choices.
>
> **Test**: If an agent can plan around a fact it has never perceived, inferred, remembered, or been told, the design violates this principle.

**Architecture alignment**: Strong. This is the central invariant of the architecture. `PlanningSnapshot` is built from `AgentBeliefStore`. `GoalBeliefView` provides subjective reads. All candidate generation, ranking, search, and revalidation use belief views. Golden tests enforce this — agents with `observation_fidelity: pm(0)` (blind) cannot perceive and therefore cannot plan around nearby entities.

**No identified gaps**: The belief/world separation is enforced at the type level through trait boundaries.

### FND-16: Ignorance, Uncertainty, and Contradiction Are First-Class

> Agents must be able to not know, to suspect, to misremember, to hold stale beliefs, and to believe false or conflicting reports. Unknown is not false. Unobserved is not empty.
>
> **Test**: If the architecture forces every proposition into a clean true/false value for each agent at all times, it is too crude for the target simulation.

**Architecture alignment**: Strong. The planner handles ignorance through snapshot incompleteness — entities not in the belief store simply don't exist for planning purposes. Stale beliefs cause plan failures that are handled gracefully via `handle_plan_failure()` and blocker recording. Pursuit plan revalidation includes confidence decay over time since last observation.

**Potential tension**: The `PlanningFact` enum used by landmarks is binary (fact present or absent). This means the landmark heuristic doesn't reason about uncertainty in intermediate planning facts — a landmark like `HasCommodity(Wheat)` is either achieved or not, with no partial confidence.

### FND-20: Resource-Bounded Practical Reasoning Over Scripts

> AI agents must reason as limited actors in a dynamic world, using beliefs, priorities, habits, skills, and commitments to choose actions. Plans exist to make reasoning tractable under limited time and limited knowledge, not to hard-script a performance.
>
> Goals name desired world conditions, not privileged one-step solutions. Reaching them may require enabling subchains — travel, acquisition, queueing, bargaining, pickup, treatment, proof, or retreat — through the same lawful affordances everyone else uses.
>
> Any planner formalism may encode only reusable lawful affordances, decomposition knowledge, or search control. It may not encode plot progression, scene-specific rails, target-specific success paths, or hidden exception logic.
>
> **Test**: For any decision, you must be able to explain it as "Agent X chose Y because they believed Z and cared about Q."

**Architecture alignment**: Strong. The GOAP planner searches through lawful affordances (action defs registered in the shared registry). No goal kind has privileged access to actions. `PlannerOpSemantics` encodes reusable decomposition knowledge ("consume actions can satisfy hunger goals"), not scene-specific logic. Cognitive bounds (`max_node_expansions`, `max_plan_depth`, `beam_width`) make reasoning explicitly resource-limited.

**No identified gaps**: Every decision is traceable through the ranking → search → revalidation pipeline with concrete belief and priority evidence.

### FND-22: Agent Diversity Through Concrete Variation

> Agents in the same role must differ in needs, skills, values, loyalties, courage, greed, patience, memory reliability, perception fidelity, and tolerance for risk or ambiguity. These differences come from concrete per-agent parameters, histories, injuries, relationships, and learned experience.
>
> **Test**: Two agents with the same role and similar beliefs should still sometimes choose differently because they are not the same person.

**Architecture alignment**: Strong. `CognitiveProfile` (14 fields) and `ExecutionBudget` (3 fields) are per-agent components that directly affect planning behavior. `UtilityProfile` weights cause different goal rankings. `PerceptionProfile` controls what enters the belief store. `DriveThresholds` affect when needs become goals. `PursuitProfile` controls pursuit aggression. Golden tests explicitly verify diversity: `search_depth_divergence`, `landmark_depth_divergence`, `golden_utility_profile_diversity`.

**No identified gaps**: The diversity mechanism is comprehensive and well-tested.

### FND-28: No Backward Compatibility in Live Authority Paths

> Do not preserve dead abstractions, alias paths, compatibility layers, deprecated shims, or legacy systems inside the live authoritative simulation simply because old code once depended on them.
>
> **Test**: If you are adding a wrapper so an obsolete abstraction can continue to mutate live world meaning beside the new one, stop and pay the migration cost now.

**Architecture alignment**: Strong. The planning pipeline has a clean authority path: ranking → search → revalidation → dispatch. No legacy planners or deprecated planning modes coexist. `PlanSearchResult` has no "legacy fallback" variants.

### FND-29: Debuggability Is a Product Feature

> Emergence without introspection is indistinguishable from bugs. The simulation must support questions such as: Why did this agent do that? Why did this caravan take this road? Why is this stash empty?
>
> **Test**: For any nontrivial event chain, you must be able to inspect both the causal path and the knowledge path separately.

**Architecture alignment**: Strong. The pipeline produces rich diagnostic data:
- `CandidateGenerationDiagnostics` — which goals were generated, which were blocked, evidence traces.
- `RankingOutcome` — which goals were suppressed, which had zero motive, motive scores and provenance for ranked goals.
- `SearchExpansionSummary` (optional) — per-expansion details during plan search.
- `BindingRejection` (optional) — why specific target bindings were rejected.
- `BlockedIntent` with `blocking_fact` and `diagnostic_context` — why replanning occurred.
- `PursuitInvalidationReason` — why pursuit plans were abandoned.
- `EvidenceTrace` on each candidate — knowledge path for goal formation.

---

## 10. Live Diagnostics

### Available Trace Infrastructure

The codebase includes diagnostic trace types that capture planning metrics:

- **`SearchExpansionSummary`**: Per-expansion details during plan search. Captures candidate counts, which candidates were preferred, successor generation results. Available via optional `expansion_summaries` parameter on `search_plan()`.
- **`BindingRejection`**: Records why specific target bindings were rejected during search. Available via optional `binding_rejections` parameter.
- **`CandidateGenerationDiagnostics`**: Records emitted candidate counts per gate, blocked desires, and fully blocked goal keys.
- **`PlanAttemptTrace`**: Records search outcomes per goal candidate (found, budget exhausted, frontier exhausted, unsupported) with expansion counts.

### Metrics NOT Currently Captured

The following metrics would be valuable for scaling analysis but are not currently captured as structured data:

1. **Landmark extraction cost** — time or operations spent in `extract_landmarks()` per search. Would help evaluate whether `landmark_extraction_depth` values are cost-effective.
2. **Per-expansion candidate counts** — how many legal affordances exist at each search node. Would reveal whether branching factor grows with world complexity.
3. **Frontier size over time** — peak frontier size during search. Would indicate whether `beam_width` is effectively limiting memory usage.
4. **Plan success rate by goal kind** — ratio of `Found` to `BudgetExhausted`/`FrontierExhausted` per GoalKind. Would identify which goal types need more search budget.
5. **Strategic plan hit rate** — how often strategic plans successfully guide tactical search to a plan. Would evaluate strategic planner effectiveness.
6. **Preferred operator hit rate** — fraction of expansions from preferred queue that lead to the final plan. Would evaluate landmark quality.

---

## 11. Architectural Observations

### Patterns

1. **Strict belief separation**: The type-level enforcement of belief views (no `&World` access in AI code) is unusually rigorous. This is a significant architectural strength that prevents accidental omniscience.

2. **Two-phase planning reduces combinatorial explosion**: Strategic planning over the place graph (cheap BFS) guides tactical A* search (expensive), avoiding brute-force search over multi-location plans. This is a standard hierarchical planning pattern applied effectively.

3. **Delete-relaxation landmarks are well-suited**: The `PlanningFact` enum has a small variant count (6 kinds), keeping the landmark extraction tractable. Backward chaining with shared-precondition intersection is a principled approach from classical planning.

4. **Blocked intent memory provides learning without global state**: Failed plans create agent-local blockers with clearing conditions, implementing a form of negative experience learning. This satisfies FND-22A (learning through concrete state).

### Asymmetries

5. **Candidate generation is sequential, not goal-aware**: The 17 `emit_*` functions run sequentially and independently. There's no cross-gate awareness — e.g., `emit_need_candidates` doesn't know that `emit_production_candidates` already found a crafting path to food. This is architecturally clean (no coupling) but may produce redundant candidates.

6. **Strategic planner uses BFS; tactical uses A***: The strategic planner's BFS doesn't benefit from landmarks or heuristics. For complex multi-prerequisite goals with many possible locations, strategic planning quality depends heavily on `max_prerequisite_locations` (default: 3) — a low value may miss better routing.

7. **Landmark heuristic is count-based, not cost-based**: `compute_landmark_heuristic` returns a count of actionable landmarks, not an estimate of the cost to achieve them. This provides admissible but potentially weak heuristic guidance — achieving 3 cheap landmarks and 3 expensive landmarks look identical.

### Scaling Concerns

8. **Expansion budget is fixed per search, not per tick**: `max_node_expansions` (default: 224) applies per `search_plan()` call. With `max_candidates_to_plan: 2`, an agent may run up to 2 × 224 = 448 expansions per decision tick. As the number of agents grows, per-tick compute scales linearly with agent count.

9. **Snapshot construction cost**: `build_planning_snapshot()` copies belief state into `BTreeMap<EntityId, SnapshotEntity>`. With `max_snapshot_entities_per_place: 50` across many known places, snapshot size could grow substantially. The pre-computed distance matrix adds O(P²) cost for P places.

10. **Affordance enumeration in revalidation**: `revalidate_next_step()` calls `get_affordances_for_defs()` which enumerates all legal affordances for the actor. For action defs with many target types and high entity counts per place, this is a per-tick cost for every agent with an active plan.

11. **Evidence tracing overhead**: Every candidate carries an `EvidenceTrace` with `BTreeSet` fields. In scenarios with many candidates (20+ per agent per tick), the allocation and comparison costs of these sets may become material.

### Oddities

12. **CognitiveProfile has `max_plan_depth` and `max_node_expansions` while ExecutionBudget has `beam_width`**: Planning bounds are split across two components without an obvious separation principle. Both are per-agent ECS components, both affect search behavior. The split appears to be historical (CognitiveProfile predates ExecutionBudget).

13. **Default `preferred_operator_boost` is 2**: This is quite conservative — only 2 consecutive preferred expansions before alternating. With typical landmark counts of 2–4, the boost may not substantially redirect search before regular queue takes over. Higher values (4–8) might more aggressively exploit landmark guidance.

14. **Default `max_candidates_to_plan` is 2**: Only the top 2 ranked goals are sent to plan search. If both fail (budget exhausted), the agent idles until blockers clear. Goals ranked 3rd and below are never attempted, even if they would have succeeded. This is a deliberate tractability bound but creates a cliff effect between "plannable" and "ignored" goals.

15. **Clearing condition granularity**: `ClearingCondition::OnCommodityChange` triggers on any change to the specified commodity. This means acquiring an unrelated batch of the same commodity type could prematurely clear a blocker, causing the agent to retry a still-failing plan. The condition checks baseline snapshot for change, which mitigates this partially.
