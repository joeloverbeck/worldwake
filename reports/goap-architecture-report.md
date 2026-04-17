# GOAP Architecture Reference — 2026-04-17

This document is a self-contained technical reference for the Worldwake GOAP
planning pipeline. It is written for an external evaluator (e.g., a
reasoning model) that has **no access to the repository**. Every type
definition, function signature, and algorithm description that matters for
evaluating the decision cycle is embedded inline.

Scope: the agent decision cycle — goal ranking, candidate generation,
affordance queries, plan search, revalidation, dispatch, and replanning.
Domain-specific system mechanics (metabolism curves, recipe math, trade
protocol details, combat resolution) are excluded except where they cross
the planner's interface.

---

## 1. Architecture Context

Worldwake is a causality-first emergent micro-world simulation in Rust.
The workspace is split into five crates:

```
worldwake-core    — IDs, types, ECS store, topology, items, relations
worldwake-sim     — Event log, action framework, scheduler, replay
worldwake-systems — Needs/metabolism, production, trade, combat, travel
worldwake-ai      — GOAP planner, goal ranking, decision runtime
worldwake-cli     — Human control interface
```

Dependencies flow strictly downward: `systems` modules depend only on
`core` and `sim`, never on each other (FND-26 enforcement). The planner
crate (`ai`) sits above `systems` and consumes action definitions via
the registry in `sim`.

### Key Foundational Types

```rust
// Unique identity for every entity in the world.
pub struct EntityId(pub NonZeroU32);

// Simulation clock. Integer ticks, monotonically increasing.
pub struct Tick(pub u64);

// Fixed-point 0..=1000. Replaces f32/f64 in authoritative state
// (no floats permitted; determinism invariant).
pub struct Permille(u16);

// Conserved item quantity. Integer.
pub struct Quantity(pub u32);

// Action / recipe / topic identifiers.
pub struct ActionDefId(pub NonZeroU32);
pub struct RecipeId(pub NonZeroU32);
```

### ECS and Determinism Guarantees

- Custom ECS, no external crate. Typed component storage is
  `BTreeMap<EntityId, Component>`. Iteration order is the key's `Ord`.
- Random streams use `ChaCha8Rng`, always explicitly seeded.
- `BTreeMap` / `BTreeSet` only in authoritative state; `HashMap` /
  `HashSet` are forbidden there.
- No floats, no wall-clock time, no system-clock RNG.
- Append-only event log is the causal source of truth and is never
  mutated.
- Conservation: items cannot be created or destroyed except through
  explicit actions; enforced by `verify_conservation`.
- Unique location: every entity exists at exactly one place.
- `ControlSource = Human | Ai | None` — there is no `Player` type;
  humans and AI drive the same bodies through identical rules
  (FND-19 Agent Symmetry).

### Belief vs. World (FND-14)

`WorldState` (ground truth) and `AgentBeliefStore` (per-agent) are
separate layers. The planner consumes a `RuntimeBeliefView` / the
immutable `PlanningSnapshot` for the acting agent — never the raw
world. Systems that want agents to react to a fact must propagate it
through perception, reports, witnesses, or written artifacts, never
through direct query of world state (FND-7).

---

## 2. Goal Ranking

Goals are selected per decision cycle from a set of candidates produced
by `generate_candidates`. Ranking converts `GroundedGoal` candidates
into a prioritised `Vec<RankedGoal>` via pressure-based classification
plus motive scoring.

### 2.1 GoalKind — All Variants

```rust
pub enum GoalKind {
    // Self-care (homeostatic needs)
    ConsumeOwnedCommodity { commodity: CommodityKind },
    AcquireCommodity      { commodity: CommodityKind,
                            purpose:   CommodityPurpose },
    Sleep,
    Relieve,
    Wash,
    FreeCarryCapacity,

    // Combat & danger
    EngageHostile { target: EntityId },
    RaidTarget    { target: EntityId },
    ReduceDanger,
    TreatWounds   { patient: EntityId },

    // Faction & banditry
    RegroupWithFaction    { faction: EntityId },
    EstablishBanditCamp   { faction: EntityId },

    // People tracking
    SearchForMissing { subject: EntityId,
                       last_seen: Option<EntityId> },
    ReportMissing    { subject: EntityId,
                       to_office: Option<EntityId>,
                       expectation_id: Option<ExpectationId> },
    ReportFound      { subject: EntityId,
                       expectation_id: ExpectationId },
    EscortToSafety   { subject: EntityId,
                       destination: EntityId },

    // Production & commerce
    ProduceCommodity  { recipe_id: RecipeId },
    SellCommodity     { commodity: CommodityKind },
    RestockCommodity  { commodity: CommodityKind },
    MoveCargo         { commodity: CommodityKind,
                        destination: EntityId },

    // Corpse handling
    LootCorpse   { corpse: EntityId },
    BuryCorpse   { corpse: EntityId, burial_site: EntityId },

    // Social & bounty
    FulfillBounty { bounty: EntityId },
    PostBounty    { posting: ArtifactPostingContext,
                    terms:   BountyTerms },
    PostNotice    { posting: ArtifactPostingContext,
                    topic:   NoticeTopic },
    ShareBelief   { listener: EntityId,
                    topic: TellTopic,
                    communication_class: CommunicationClass },

    // Politics & justice
    ClaimOffice               { office: EntityId },
    SupportCandidateForOffice { office: EntityId,
                                candidate: EntityId },
    InvestigateViolation      { violation_id: ViolationId,
                                place: EntityId },
    Patrol                    { place: EntityId },
    Accuse                    { crime_register: EntityId,
                                accused: EntityId,
                                violation_id: ViolationId },
    PunishAccused             { office: EntityId,
                                accused: EntityId,
                                accusation_entry: RecordEntryId,
                                punishment: PunishmentKind },

    // Exploration & theft
    ExploreLocation { target_place: EntityId,
                      motivating_need: ExplorationMotivation },
    StealItem       { target_item: EntityId },
}

pub enum CommodityPurpose {
    SelfConsume,
    Restock,
    RecipeInput(RecipeId),
}

pub enum ExplorationMotivation {
    NeedDriven(HomeostaticNeedId),
    Proactive,
}

pub enum GoalPriorityClass {
    Critical,
    High,
    Medium,
    Low,
    Background,
}
```

### 2.2 Pressure Derivation

```rust
pub fn build_decision_context(
    view:  &dyn GoalBeliefView,
    agent: EntityId,
) -> DecisionContext;

pub struct DecisionContext {
    pub max_self_care_class: GoalPriorityClass,
    pub danger_class:        GoalPriorityClass,
}

pub fn classify_band(value: Permille,
                     band:  &ThresholdBand)
    -> GoalPriorityClass;
```

`classify_band` maps a continuous pressure in `Permille` to one of five
classes by comparing against monotonic thresholds on a per-agent
`ThresholdBand` (thresholds are authored per-profile, satisfying the
"no magic numbers" constraint of FND-22).

Danger pressure is computed from concrete threats — not a
`danger_score` (FND-3):

```rust
pub struct DangerAssessment {
    pub pressure:           Permille,
    pub thresholds_present: bool,
    pub current_attackers:  Vec<EntityId>,
    pub visible_hostiles:   Vec<EntityId>,
    pub hostile_targets:    Vec<EntityId>,
    pub has_wounds:         bool,
    pub is_incapacitated:   bool,
}

pub fn derive_danger_pressure(view: &dyn GoalBeliefView,
                              agent: EntityId) -> Permille;
pub fn derive_pain_pressure  (view: &dyn GoalBeliefView,
                              agent: EntityId) -> Permille;
```

Rough mapping (exact thresholds profile-driven):

| Condition                                              | Class    |
|--------------------------------------------------------|----------|
| ≥2 attackers OR 1 attacker + wounds/incapacitated      | Critical |
| 1 attacker OR visible hostile + wounded                | High     |
| Visible hostile only                                   | Medium   |
| None of the above                                      | 0        |

### 2.3 Ranking

```rust
pub fn rank_candidates(
    candidates:        &[GroundedGoal],
    view:              &dyn GoalBeliefView,
    agent:             EntityId,
    current_tick:      Tick,
    utility:           &UtilityProfile,
    decision_context:  &DecisionContext,
) -> RankingOutcome;

pub struct RankingOutcome {
    pub ranked:      Vec<RankedGoal>,
    pub suppressed:  Vec<GoalKey>,
    pub zero_motive: Vec<GoalKey>,
}

pub struct RankedGoal {
    pub grounded:                    GroundedGoal,
    pub priority_class:              GoalPriorityClass,
    pub motive_score:                u32,
    pub provenance:                  Option<RankedGoalProvenance>,
    pub source_reliability_discount: Option<SourceReliabilityDiscount>,
    pub competition_discount:        Option<CompetitionDiscount>,
    pub feasibility:                 FeasibilityHint,
}

pub enum RankedGoalProvenance {
    Danger(DangerAssessment),
    Drive (RankedDriveGoalProvenance),
}

pub struct RankedDriveGoalProvenance {
    pub base_priority_class:  GoalPriorityClass,
    pub final_priority_class: GoalPriorityClass,
    pub adjustment:           Option<RankedPriorityAdjustment>,
    pub motive_inputs:        Vec<RankedDriveMotiveInput>,
}

pub struct RankedDriveMotiveInput {
    pub need:     HomeostaticNeedId,
    pub pressure: Permille,
    pub score:    u32,
}
```

Algorithm pseudocode:

```
ranked, suppressed, zero_motive := [], [], []

for cand in candidates:
    if evaluate_suppression(cand, decision_context):
        suppressed.push(cand.key); continue

    prov       := derive_ranking_provenance(cand)
    priority   := prov.priority  OR policy_based_priority(cand)
    motive     := prov.score     OR goal_motive_score(cand)

    motive     := apply_source_reliability_discount(cand, motive)
    motive     := apply_competition_discount      (cand, motive)

    if motive == 0:
        zero_motive.push(cand.key)
    else:
        ranked.push(RankedGoal{ cand, priority, motive, prov, ... })

stable_sort(ranked, compare_ranked_goals)
return RankingOutcome{ ranked, suppressed, zero_motive }
```

`compare_ranked_goals` orders by (descending): `priority_class`,
`motive_score`, opportunity strength, then `GoalKind` lexicographic
tiebreaker for determinism.

### 2.4 Suppression

`evaluate_suppression` implements FND-16-aware gating:

- **Danger = Critical** suppresses every non-combat / non-danger-reduction
  goal.
- **Max self-care = Critical** suppresses every goal that does not
  address self-care.
- **Bandit raid deterrence**: if wounds exceed a courage-scaled threshold,
  raid/engage goals are suppressed even when targets are visible.

Suppression is *per-decision*, not a state flag — the suppressed goals
are re-offered next tick if context changes.

---

## 3. Candidate Generation

### 3.1 Entry Points

```rust
pub fn generate_candidates(
    view:         &dyn GoalBeliefView,
    agent:        EntityId,
    blocked:      &BlockedIntentMemory,
    recipes:      &RecipeRegistry,
    current_tick: Tick,
) -> Vec<GroundedGoal>;

pub fn generate_candidates_with_travel_horizon(
    view:              &dyn GoalBeliefView,
    agent:             EntityId,
    blocked:           &BlockedIntentMemory,
    violation_memory:  &ViolationMemory,
    recipes:           &RecipeRegistry,
    current_tick:      Tick,
    travel_horizon:    u8,
    tracing_enabled:   bool,
) -> CandidateGenerationResult;

pub struct CandidateGenerationResult {
    pub candidates:   Vec<GroundedGoal>,
    pub diagnostics:  CandidateGenerationDiagnostics,
    pub pending_violations: Vec<PendingViolationRecord>,
    pub pending_acquisition_exhaustion_resets:
        BTreeSet<HomeostaticNeedId>,
}

pub struct CandidateGenerationDiagnostics {
    pub omitted_political:           Vec<PoliticalCandidateOmission>,
    pub omitted_bandit:              Vec<BanditCandidateOmission>,
    pub omitted_social:              Vec<SocialCandidateOmission>,
    pub omitted_violation_detection: Vec<ViolationDetectionOmission>,
    pub evidence:
        BTreeMap<OpportunityKey, CandidateEvidenceTrace>,
    pub fully_blocked_desires:       Vec<DesireFullyBlocked>,
    pub places_reachable:            u32,
    pub places_after_belief_filter:  u32,
}
```

### 3.2 GroundedGoal

```rust
pub struct GroundedGoal {
    pub key:              GoalKey,
    pub anchor:           OpportunityAnchor,
    pub evidence_entities: BTreeSet<EntityId>,
    pub evidence_places:   BTreeSet<EntityId>,
}

pub struct GoalKey {
    pub kind:      GoalKind,
    pub commodity: Option<CommodityKind>,
    pub entity:    Option<EntityId>,
    pub place:     Option<EntityId>,
}

pub enum OpportunityAnchor {
    Place(EntityId),
    Entity(EntityId),
    None,
}
```

### 3.3 Emission Gates

Candidates are emitted by a fixed sequence of `emit_*` functions. Each
gate has a preflight check (needs threshold, visible hostiles, owned
stock, merchant signals, etc.) that decides whether to push goals.

| # | Gate                                   | Produces                                                  |
|---|----------------------------------------|-----------------------------------------------------------|
| 1 | `emit_need_candidates`                 | `ConsumeOwnedCommodity`, `AcquireCommodity(SelfConsume)`, `Sleep`, `Relieve`, `Wash`, `FreeCarryCapacity` |
| 2 | `emit_production_candidates`           | `ProduceCommodity` per available recipe                  |
| 3 | `emit_enterprise_candidates`           | `SellCommodity`, `RestockCommodity`                       |
| 4 | `emit_disposal_candidates`             | `MoveCargo` to unload excess                              |
| 5 | `emit_bounty_candidates`               | `FulfillBounty`                                           |
| 6 | `emit_artifact_posting_candidates`     | `PostBounty`, `PostNotice`                                |
| 7 | `emit_combat_candidates`               | `EngageHostile`, `RaidTarget`                             |
| 8 | `emit_crime_candidates`                | `StealItem`, `LootCorpse`                                 |
| 9 | `emit_social_candidates`               | `ShareBelief` on novel/stale topics                       |
| 10| `emit_patrol_candidates`               | `Patrol`                                                  |
| 11| `emit_political_candidates`            | `ClaimOffice`, `SupportCandidateForOffice`                |
| 12| `emit_recorded_violation_candidates`   | `InvestigateViolation`, `Accuse`                          |
| 13| `emit_search_candidates`               | `SearchForMissing`                                        |
| 14| `emit_report_found_candidates`         | `ReportFound`                                             |
| 15| `emit_escort_candidates`               | `EscortToSafety`                                          |
| 16| `emit_exploration_candidates`          | `ExploreLocation(NeedDriven)`                             |
| 17| `emit_proactive_exploration_candidates`| `ExploreLocation(Proactive)`                              |
| + | `emit_expectation_violation_candidates`| Pending violation records for the next tick              |

Every gate queries the agent's beliefs via `GoalBeliefView`, not world
state. Places and entities considered are gated by the agent's
travel-horizon snapshot (Section 5.8).

### 3.4 Blocked-Intent Filtering

```rust
pub struct BlockedIntentMemory {
    pub failures:         BTreeMap<OpportunityKey, BlockingFact>,
    pub transient_blocks: BTreeMap<OpportunityKey, Tick>,
    pub last_attempted:   BTreeMap<OpportunityKey, Tick>,
}
```

Filter rule inside each gate:

```
if cand.anchor_key in failures         → drop
if cand.anchor_key in transient_blocks
   and current_tick < retry_tick       → drop
if recently_attempted(cooldown_active) → drop
```

### 3.5 Branching-Factor Shape

Per-agent cognitive caps (Section 8) limit how many generated candidates
reach the search:

- `max_candidates_to_plan`  (default 2) — number of top-ranked goals the
  search attempts per cycle.
- `max_candidates_per_expansion` (default 200) — successors per search
  node.

Typical decision cycles produce 50–500 `GroundedGoal` entries before
ranking. After `rank_candidates` and the top-N truncation, only 1–2
goals enter `search_plan` per tick.

---

## 4. Affordance Queries

### 4.1 ActionDef

```rust
pub struct ActionDef {
    pub id:     ActionDefId,
    pub name:   String,
    pub domain: ActionDomain,

    pub actor_constraints: Vec<Constraint>,
    pub targets:           Vec<TargetSpec>,
    pub preconditions:     Vec<Precondition>,

    pub reservation_requirements: Vec<ReservationReq>,
    pub duration:                 DurationExpr,

    pub body_cost_per_tick: BodyCostPerTick,
    pub attention_cost:     Permille,

    pub interruptibility:   Interruptibility,
    pub commit_conditions:  Vec<Precondition>,

    pub visibility:         VisibilitySpec,
    pub causal_event_tags:  BTreeSet<EventTag>,

    pub payload: ActionPayload,
    pub handler: ActionHandlerId,
}

pub enum Interruptibility {
    FreelyInterruptible,
    InterruptibleWithPenalty,
    NonInterruptible,
}

pub enum TargetSpec {
    SpecificEntity(EntityId),
    ActorPlace,
    EntityAtActorPlace       { kind: EntityKind },
    EntityAtActorPlace_Dead  { kind: EntityKind },
    EntityInInventory        { kind: EntityKind },
    EntityInContainer        { kind: EntityKind },
    // ... additional contextual variants
}
```

### 4.2 PlannerOpKind

~54 operator kinds classify affordances for goal routing:

```rust
pub enum PlannerOpKind {
    // Movement & positioning
    Travel, Patrol,
    // Needs
    Consume, Sleep, Relieve, Wash, EstablishCamp,
    // Economic
    Trade, StaffMarket, QueueForFacilityUse,
    Harvest, Craft, MoveCargo, DropItem, StockManagement,
    // Combat & survival
    Heal, Attack, Defend, Loot, Bury,
    // Social & bounty
    Tell, Accuse, PostBounty, ClaimBounty, PostNotice,
    ReportMissing, ReportFound, EscortToSafety,
    // Investigation & administration
    ConsultRecord, Investigate, AskWitness, SearchPlace,
    AskAboutPerson, Fine, Bribe, Threaten, Exile,
    DeclareSupport, PressForceClaim, YieldForceClaim,
}
```

Each `GoalKind` declares `relevant_op_kinds() -> &'static [PlannerOpKind]`
via a trait, e.g.:

| Goal                       | Relevant ops                                                        |
|----------------------------|---------------------------------------------------------------------|
| `AcquireCommodity`         | Trade, Harvest, Craft, QueueForFacilityUse, Consume                 |
| `EngageHostile`            | Travel, Attack, Defend                                              |
| `ProduceCommodity`         | Travel, QueueForFacilityUse, Craft, Consume                         |
| `PostBounty`               | Travel, PostBounty                                                  |
| `ClaimOffice`              | Travel, PressForceClaim, Investigate, ConsultRecord                 |

### 4.3 Affordance Enumeration

```rust
pub fn get_affordances_for_defs(
    view:          &dyn RuntimeBeliefView,
    actor:         EntityId,
    defs:          &[&ActionDef],
    handlers:      &ActionHandlerRegistry,
) -> Vec<Affordance>;

pub fn enumerate_targets(
    view:  &dyn RuntimeBeliefView,
    actor: EntityId,
    spec:  &TargetSpec,
) -> Vec<EntityId>;
```

Algorithm (high-level):

```
for each ActionDef def in defs:
    if not actor_constraints_met(actor, def.actor_constraints): continue
    candidate_target_sets := []
    for tgt_spec in def.targets:
        candidate_target_sets.push(enumerate_targets(view, actor, tgt_spec))
    for combination in cross_product(candidate_target_sets):
        if preconditions_met(view, actor, def.preconditions, combination):
            push Affordance { def_id, targets: combination, payload }
```

Target enumeration is **locality-scoped** (FND-7): `EntityAtActorPlace`
pulls entities via `view.entities_at(view.effective_place(actor))`;
`EntityInInventory` pulls via `view.direct_possessions(actor)`. The
planner can never request a target the agent has no belief-grounded
reason to consider.

### 4.4 RuntimeBeliefView Surface

```rust
// Spatial
fn effective_place(&self, entity: EntityId) -> Option<EntityId>;
fn entities_at    (&self, place:  EntityId) -> Vec<EntityId>;
fn is_in_transit  (&self, entity: EntityId) -> bool;
fn adjacent_places(&self, place:  EntityId) -> Vec<EntityId>;

// Inventory
fn direct_possessions (&self, holder: EntityId) -> Vec<EntityId>;
fn commodity_quantity (&self, holder: EntityId,
                       kind: CommodityKind) -> Quantity;
fn direct_container   (&self, entity: EntityId) -> Option<EntityId>;

// Facility
fn matching_workstations_at(&self, place: EntityId,
                            tag: WorkstationTag) -> Vec<EntityId>;
fn resource_sources_at     (&self, place: EntityId,
                            commodity: CommodityKind) -> Vec<EntityId>;

// Temporal
fn reservation_conflicts(&self, entity: EntityId,
                         range: TickRange) -> bool;
fn estimate_duration    (...) -> Option<ActionDuration>;
```

All methods operate on **believed** state. The implementation used by
the AI layer wraps `AgentBeliefStore`; the simulation layer uses a view
over authoritative state for non-AI purposes (e.g., witness observation
resolution). The planner receives only the belief-backed view.

---

## 5. Plan Search Pipeline

Planning is two-layered: a **strategic planner** produces a
location-visit itinerary from the agent's beliefs, then a **tactical
planner** performs A* with dual-frontier beam search at each strategic
step.

### 5.1 Strategic Planner

```rust
pub(crate) struct StrategicPlan {
    pub steps: Vec<StrategicStep>,
}

pub(crate) struct StrategicStep {
    pub destination:            EntityId,
    pub sub_goal:               TacticalSubGoal,
    pub estimated_travel_ticks: u32,
}

pub(crate) enum TacticalSubGoal {
    SatisfyGoal,
    AcquirePrerequisite(CommodityKind),
    ExploreWithBarrier,
    ExploreFallback,
    SocialQuery(CommodityKind),
}

pub(crate) fn plan(
    snapshot:         &PlanningSnapshot,
    goal:             &GroundedGoal,
    execution_budget: &ExecutionBudget,
    recipes:          &RecipeRegistry,
) -> Option<StrategicPlan>;
```

Algorithm (Dijkstra over belief graph):

```
1. If goal already satisfied: return empty plan.
2. Determine actor's effective place from beliefs.
3. goal_places       := goal_places(goal, snapshot)
   missing_commodities := missing_commodities(goal, snapshot)
4. For each missing commodity c:
     stage_c := acquisition places ranked by perceived travel cost,
                truncated to execution_budget.max_prerequisite_locations
     stages.push(stage_c)
   stages.push(Goal stage -> goal_places)
5. If actor already satisfies head stages locally, consume them.
6. Frontier search:
     best[(0, actor_place)] := 0
     heap := [(actor_place, stage_idx=0, cost=0)]
     while heap not empty and cost <= 2*max_prerequisite_locations:
         (place, idx, cost) := heap.pop_min()
         if idx >= len(stages): return materialize steps from
                                        predecessor chain
         for dest in stages[idx]:
             c' := cost + perceived_travel_cost(place, dest)
             if c' < best[(idx+1, dest)]:
                 update best; push (dest, idx+1, c')
7. If no plan within budget: return None.
```

The strategic plan feeds the tactical planner one `StrategicStep` at a
time. Tactical search terminates as soon as the step's `sub_goal` is
satisfied (e.g., prerequisite acquired), then advances to the next
strategic step.

### 5.2 Tactical Search — Entry Point

```rust
pub fn search_plan(
    snapshot:           &PlanningSnapshot,
    goal:               &GroundedGoal,
    semantics_table:    &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry:           &ActionDefRegistry,
    handlers:           &ActionHandlerRegistry,
    cognitive:          &CognitiveProfile,
    execution_budget:   &ExecutionBudget,
    recipes:            &RecipeRegistry,
    blocked:            &BlockedIntentMemory,
    current_tick:       Tick,
    binding_rejections: Option<&mut Vec<BindingRejection>>,
    expansion_summaries:
        Option<&mut Vec<SearchExpansionSummary>>,
) -> PlanSearchResult;

pub enum PlanSearchResult {
    Found(Box<PlannedPlan>),
    Unsupported,
    BudgetExhausted   { expansions_used: u16 },
    FrontierExhausted { expansions_used: u16 },
}

pub struct PlannedPlan {
    pub opportunity:   OpportunityKey,
    pub goal:          GoalKey,
    pub steps:         Vec<PlannedStep>,
    pub terminal_kind: PlanTerminalKind,
}

pub enum PlanTerminalKind {
    GoalSatisfied,
    CombatCommitment,
    ProgressBarrier,
}

pub struct PlannedStep {
    pub def_id:                   ActionDefId,
    pub targets:                  Vec<PlanningEntityRef>,
    pub payload_override:         Option<ActionPayload>,
    pub op_kind:                  PlannerOpKind,
    pub estimated_ticks:          u32,
    pub is_materialization_barrier: bool,
    pub expected_materializations: Vec<EntityId>,
}
```

### 5.3 Search Node

```rust
pub(crate) struct SearchNode<'snapshot> {
    pub state:                    PlanningState<'snapshot>,
    pub steps:                    SharedVec<PlannedStep>,
    pub total_estimated_ticks:    u32,
    pub search_cost:              u32,
    pub tactical_barrier_reached: bool,
    pub heuristic_ticks:          u32,
}
```

`PlanningState` is a copy-on-write overlay over `PlanningSnapshot`;
mutations applied during simulated transitions (inventory changes,
place changes, reservation shadows) are stored in `SharedMap` /
`SharedSet` structures and cloned only on divergence.

### 5.4 Main Loop Algorithm

```
result        := Unsupported
landmarks     := LandmarkSet::empty()
frontier      := DualFrontier::new()
frontier.push(root_node)
expansions    := 0
best_barrier  := None

while frontier not empty and expansions < cognitive.max_node_expansions:
    node := frontier.pop()

    if goal.is_satisfied(&node.state):
        return Found(plan_from(node))

    if node.steps.len() >= cognitive.max_plan_depth:
        continue

    if expansions >= cognitive.max_node_expansions:
        break

    expansions += 1

    candidates := affordances(node.state)
                 ∪ synthesized_candidates(goal, node.state)
                 ∪ planner_only_operators(goal.kind)

    candidates := filter_by(
        commodity_relevance(goal),
        tactical_goal_constraint(strategic_step),
        travel_direction_toward(goal_places),
        per_expansion_cap(cognitive.max_candidates_per_expansion),
    )

    successors := []
    for cand in candidates:
        succ := build_successor_detailed(node, cand, registry, handlers)
        if succ.terminal_kind == CombatCommitment or barrier_hit:
            best_barrier = best(best_barrier, succ)
        else:
            spatial_h := compute_heuristic(snapshot,
                                           &succ.state, goal_places)
            if cognitive.use_ff_heuristic:
                ff_result := run_ff_heuristic(...)
                succ.heuristic_ticks := max(spatial_h, ff_result.h_ff)
                mark_preferred_if(cand in ff_result.helpful_actions)
            else:
                lm_h := compute_landmark_heuristic(
                            &landmarks, current_facts(&succ.state))
                succ.heuristic_ticks := max(spatial_h, lm_h)
                mark_preferred_if(cand achieves actionable landmark)
            successors.push(succ)

    if expansions == 1 and cognitive.landmark_extraction_depth > 0:
        landmarks := extract_landmarks(
            current_facts(&node.state),
            goal_facts(goal),
            operators_from_semantics_table,
            cognitive.landmark_extraction_depth)

    sort(successors by f = search_cost + heuristic_ticks asc,
                      then search_cost asc,
                      then total_estimated_ticks asc,
                      then steps.len asc,
                      then steps lexicographic)
    truncate(successors, execution_budget.beam_width)

    if any(successor.preferred):
        frontier.trigger_boost()
    for s in successors:
        frontier.push(s)

if best_barrier:
    return Found(plan_from(best_barrier))    # progress-barrier fallback
else if expansions >= cognitive.max_node_expansions:
    return BudgetExhausted { expansions_used: expansions }
else:
    return FrontierExhausted { expansions_used: expansions }
```

Termination kinds:

| Terminal               | Condition                                          |
|------------------------|----------------------------------------------------|
| `GoalSatisfied`        | `goal.key.kind.is_satisfied(&node.state)`          |
| `ProgressBarrier`      | Tactical sub-goal met (e.g., prerequisite obtained)|
| `CombatCommitment`     | Attack/Defend produced — execution takes priority  |
| `BudgetExhausted`      | `expansions >= max_node_expansions`                |
| `FrontierExhausted`    | Frontier empty before any terminal found           |

### 5.5 Landmark Extraction

```rust
pub(super) enum PlanningFact {
    AtPlace            (EntityId),
    HasCommodity       (CommodityKind),
    HasEntity          (EntityId),
    FacilityAvailable  (EntityId),
    EntityPresent      (EntityId),
    NeedSatisfied      (HomeostaticNeedId),
}

pub(super) struct LandmarkSet {
    pub landmarks: BTreeSet<PlanningFact>,
    pub orderings: Vec<(PlanningFact, PlanningFact)>,
}

pub(super) fn extract_landmarks(
    initial_facts: &BTreeSet<PlanningFact>,
    goal_facts:    &BTreeSet<PlanningFact>,
    operators:     &[PlanningOperator],
    max_depth:     u8,
) -> LandmarkSet;
```

Delete-relaxation algorithm:

```
landmarks := goal_facts
orderings := []
queue     := [(f, 0) for f in goal_facts]

while queue not empty:
    (fact, depth) := queue.pop()
    if depth >= max_depth or fact in initial_facts: continue

    achievers := [op for op in operators if fact in op.add_effects]
    if achievers empty: continue       # unachievable → no ordering

    shared_preconds := intersection(ach.preconditions
                                    for ach in achievers)

    for pred in shared_preconds:
        if pred != fact:
            orderings.push((pred, fact))
            if pred not in landmarks:
                landmarks.insert(pred)
                queue.push((pred, depth+1))

return LandmarkSet { landmarks, orderings }
```

Intuition: a fact `F` is a landmark if every operator that achieves it
shares precondition `P`; then `P` must hold before any plan reaches `F`.

`landmark_extraction_depth = 0` disables landmark extraction and falls
back to the pure spatial heuristic.

### 5.6 Dual Frontier

```rust
pub(super) struct DualFrontier<'s> {
    regular:   BinaryHeap<FrontierEntry<'s>>,
    preferred: BinaryHeap<FrontierEntry<'s>>,
    boost_remaining:          u8,
    preferred_operator_boost: u8,
    use_preferred_next:       bool,
}
```

Semantics:

- Two priority queues, both ordered by the same comparator
  (`f`, then `g`, then ticks, then depth, then steps lexicographic).
- Default alternation: pop toggles `use_preferred_next` each call.
- **Boost**: when an expansion produces any preferred successor,
  `boost_remaining := preferred_operator_boost`. While boosted, pops
  prefer the preferred queue even when alternation would pick regular.
  Each boosted pop decrements `boost_remaining`.
- Preferred sources:
  - FF helpful actions (layer-0 operators in the relaxed plan), or
  - Operators that achieve an actionable landmark
    (unachieved landmark whose ordering-predecessors all hold).

Beam truncation is **per expansion**: successors are sorted, then
`successors = successors[..execution_budget.beam_width]` before
enqueue. This is a hard cap; pruned nodes are not retained anywhere.

### 5.7 Heuristics

```rust
pub(super) fn compute_heuristic(
    snapshot: &PlanningSnapshot,
    state:    &PlanningState<'_>,
    goal_relevant_places: &[EntityId],
) -> u32;
// Min perceived travel cost from current place to any goal place.

pub(super) fn compute_landmark_heuristic(
    landmarks:     &LandmarkSet,
    current_facts: &BTreeSet<PlanningFact>,
) -> u32;
// Count of actionable (unachieved, ordering-ready) landmarks.
```

Combination:

```
successor.heuristic_ticks = max(spatial_h, ff_or_landmark_h)
```

Admissibility:

| Heuristic          | Admissible? | Notes                                        |
|--------------------|-------------|----------------------------------------------|
| Spatial            | Yes         | Perceived travel cost ≤ actual travel cost   |
| Landmark-count     | Yes         | Each remaining landmark → at least one step  |
| FF (relaxed plan)  | No          | Ignores delete effects                        |
| max(spatial, LM)   | Yes         | Max of admissibles is admissible             |
| max(spatial, FF)   | No          | FF component breaks admissibility            |

Because `use_ff_heuristic = true` by default, the combined heuristic is
**inadmissible** in the default profile — trading optimality guarantees
for goal-focus on typical instances.

Tiebreakers (stable): `g`, `total_estimated_ticks`, `steps.len`, steps
lexicographic.

### 5.8 PlanningSnapshot — the Belief Surface

```rust
pub struct PlanningSnapshot {
    pub(crate) actor:        EntityId,
    pub(crate) current_tick: Tick,

    pub(crate) actor_belief_store: AgentBeliefStore,

    // Subset of world within snapshot_travel_horizon hops
    pub(crate) entities: BTreeMap<EntityId, SnapshotEntity>,
    pub(crate) places:   BTreeMap<EntityId, SnapshotPlace>,

    pub(crate) blocked_facility_uses:
        BTreeSet<(EntityId, ActionDefId)>,

    pub(crate) actor_known_entity_beliefs:
        BTreeMap<EntityId, BelievedEntityState>,
    pub(crate) actor_known_social_observations:
        Vec<SocialObservation>,
    pub(crate) actor_known_institutional_beliefs:
        Vec<BelievedInstitutionalClaim>,
    pub(crate) actor_told_beliefs:
        BTreeMap<TellMemoryKey, ToldBeliefMemory>,

    pub(crate) actor_bandit_factions: Vec<EntityId>,
    pub(crate) actor_active_violation_records:
        Vec<RecordedViolation>,
    pub(crate) actor_contested_offices: Vec<EntityId>,
    pub(crate) actor_loyalties: BTreeMap<EntityId, Permille>,
    pub(crate) actor_office_holder_beliefs:
        BTreeMap<EntityId, SupportBeliefRead>,
    pub(crate) actor_force_controller_beliefs:
        BTreeMap<EntityId, ForceControllerBeliefRead>,
    pub(crate) office_certain_support_declarations:
        BTreeMap<EntityId, Vec<(EntityId, EntityId)>>,
    pub(crate) office_support_declaration_beliefs:
        BTreeMap<EntityId, OfficeSupportBeliefReads>,

    pub(crate) actor_confidence_policy:    BeliefConfidencePolicy,
    pub(crate) actor_tell_profile:         Option<TellProfile>,
    pub(crate) actor_epistemic_profile:
        Option<EpistemicDispositionProfile>,
    pub(crate) actor_consultation_speed_factor:
        Option<Permille>,
    pub(crate) actor_expectation_store:    Option<ExpectationStore>,
    pub(crate) actor_last_seen_memory:     Option<LastSeenMemory>,

    pub(crate) actor_bandit_flee_thresholds:
        BTreeMap<EntityId, Permille>,
    pub(crate) actor_bandit_establishment_ticks:
        BTreeMap<EntityId, NonZeroU32>,

    // Floyd-Warshall-precomputed, agent-scoped distance matrices
    shortest_travel_ticks:    DistanceMatrix,
    perceived_travel_costs:   DistanceMatrix,
}
```

`PlanningSnapshot` is immutable across an entire search. `PlanningState`
wraps it with CoW override maps:

```rust
pub struct PlanningState<'snapshot> {
    snapshot: &'snapshot PlanningSnapshot,

    entity_place_overrides:         SharedMap<PlanningEntityRef, Option<EntityId>>,
    direct_container_overrides:     SharedMap<PlanningEntityRef, Option<PlanningEntityRef>>,
    direct_possessor_overrides:     SharedMap<PlanningEntityRef, Option<PlanningEntityRef>>,
    resource_quantity_overrides:    SharedMap<EntityId, Quantity>,
    commodity_quantity_overrides:   SharedMap<(PlanningEntityRef, CommodityKind), Quantity>,

    reservation_shadows:            SharedMap<EntityId, Vec<TickRange>>,
    removed_entities:               SharedSet<PlanningEntityRef>,
    sale_listing_overrides:         SharedMap<PlanningEntityRef, bool>,
    needs_overrides:                SharedMap<EntityId, HomeostaticNeeds>,
    pain_overrides:                 SharedMap<EntityId, Permille>,

    support_declaration_overrides:
        SharedMap<(EntityId, EntityId), Option<EntityId>>,
    office_holder_belief_overrides:
        SharedMap<EntityId, InstitutionalBeliefRead<Option<EntityId>>>,

    hypothetical_registry: SharedMap<HypotheticalEntityId, HypotheticalEntityMeta>,
    next_hypothetical_id:  u32,

    entities_at_cache:       Rc<RefCell<BTreeMap<EntityId, Vec<EntityId>>>>,
    effective_place_cache:   Rc<RefCell<BTreeMap<PlanningEntityRef, Option<EntityId>>>>,
}
```

This is how simulated transitions are realised without touching the
snapshot — each `SearchNode` owns a `PlanningState` with independent
override maps (`SharedMap` is `Rc<RefCell<BTreeMap<...>>>`).

### 5.9 What Counts as an Expansion

A "node expansion" consists of:

1. Popping a node from the dual frontier.
2. Passing depth / terminal guards.
3. Incrementing `expansions`.
4. Running affordance + synthesized-candidate generation, filtering, and
   successor construction.
5. Applying heuristic evaluation and beam truncation.

Each expansion enqueues at most `beam_width` nodes (default 8).

---

## 6. Plan Revalidation & Execution

### 6.1 Revalidation

```rust
pub fn revalidate_next_step(
    view:     &dyn RuntimeBeliefView,
    actor:    EntityId,
    step:     &PlannedStep,
    bindings: &MaterializationBindings,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> bool;
```

Algorithm:

```
1. Resolve action def & handler; resolve planning targets from bindings.
2. affordances := get_affordances_for_defs(view, actor, &[def], handlers)
3. if any af in affordances satisfies requested_affordance_matches(step, af):
       return true
4. if def.payload == ActionPayload::None:
       // BestEffort payload-override path
       if revalidate_best_effort_payload_override_step(...):
           return true
5. if revalidate_exact_target_step(view, actor, step, ...):
       // Synthesise an affordance with exact targets & retry match
       return true
6. return false
```

```rust
fn revalidate_best_effort_payload_override_step(
    view:   &dyn RuntimeBeliefView,
    actor:  EntityId,
    step:   &PlannedStep,
    targets: &[EntityId],
    def:     &ActionDef,
    handler: &ActionHandler,
) -> bool;
```

Steps 4–5 matter because the planner may **synthesise payload overrides**
that no affordance directly exposes (e.g., a custom trade counterparty
chosen by the planner). For such actions the handler must be registered
with `.with_payload_override_validator(closure)`; without the validator,
`revalidate_best_effort_payload_override_step` returns false and the
step silently fails revalidation. This is a known contract between the
planner and handler authors — see the "Authoritative-to-AI Impact Rule"
in `CLAUDE.md`.

### 6.2 Pursuit Plan Invalidation

```rust
pub fn is_pursuit_plan_invalid(
    view:         &dyn RuntimeBeliefView,
    actor:        EntityId,
    plan:         &PlannedPlan,
    current_tick: Tick,
) -> Option<PursuitInvalidationReason>;

pub enum PursuitInvalidationReason {
    TargetDead,
    PlaceUnknown,
    CoLocated,
    PlaceChanged,
    ConfidenceDecayed,
}
```

Used for remote pursuit goals (`RaidTarget`, `EngageHostile`). Returns
`None` if the plan is still valid. `ConfidenceDecayed` uses the agent's
`BeliefConfidencePolicy` to fold `current_tick - last_observed_tick`
into a confidence measure; plans drop when confidence falls below the
per-agent `min_location_confidence` threshold.

### 6.3 agent_tick Module — Dispatch Pipeline

Driver:

```rust
pub struct AgentTickDriver {
    runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>,
    semantics_cache:  Option<(usize, BTreeMap<ActionDefId, PlannerOpSemantics>)>,
    trace_sink:       Option<DecisionTraceSink>,
}
```

Implements `AutonomousController::produce_agent_input()`. Per-agent
flow:

```
1. Fast-path: if agent is dead and cleanup done, skip.
2. observation.rs :: update_runtime_observation_snapshot()
   - Cache effective place, commodities, carry, hostile list.
   - Compute dirty flags by comparing to previous tick's snapshot.
3. candidates.rs :: abandon_expired_facility_queues()
   - Drop stale facility-queue intents; record BlockingFact::
     ExclusiveFacilityUnavailable.
4. active_action.rs :: handle_active_action_phase()
   - If a step is in flight, decide whether to interrupt.
   - For FreelyInterruptible actions: build challenger plans
     and call evaluate_interrupt().
   - Returns InterruptDecision (NoInterrupt | InterruptForReplan).
5. planning.rs :: plan_and_validate_next_step_traced()
   - Build PlanningSnapshot for top-ranked goal(s).
   - Call search_plan() for each (up to max_candidates_to_plan).
   - On Found: revalidate first step.
6. execution.rs :: enqueue_valid_step_or_handle_failure()
   - If step valid: enqueue RequestAction with
     ActionRequestMode::BestEffort and RequestProvenance::AiPlan.
     Set step_in_flight := true.
   - If step invalid: try recoverable travel-step blockage fixup;
     else handle_current_step_failure() → Section 7.
7. frame.rs :: update_frame_for_adopted_plan()
   - Update IntentionFrame (active goal, assumptions, plan).
8. Finalise tick: persist blocked_memory & violation_memory, update
   observation snapshot.
```

Action dispatch uses `ActionRequestMode::BestEffort` so the scheduler
can substitute targets if exact materialisation bindings drift slightly
between plan-time and dispatch-time. The handler's precondition check
remains authoritative.

---

## 7. Replanning

### 7.1 Failure Handling

```rust
pub fn handle_plan_failure(
    context:         &PlanFailureContext<'_>,
    runtime:         &mut AgentDecisionRuntime,
    jc:              &mut Option<IntentionFrame>,
    blocked_memory:  &mut BlockedIntentMemory,
    facility_intents:&mut ContentionIntents,
    cognitive:       &CognitiveProfile,
);
```

Algorithm:

```
1. Clear plan, step index, materialization bindings, facility intents.
2. Set IntentionFrame clear reason := PlanFailed.
3. Derive BlockingFact via derive_blocking_fact(context):
     TargetGone | NoKnownPath | CombatTooRisky
     TraderUnwilling | InsufficientStock | MissingRecipe
     ExclusiveFacilityUnavailable | Unknown | ...
4. Derive TTL from blocking_fact_ttl(fact_kind, cognitive):
     - transient          → cognitive.transient_block_ticks  (20)
     - unknown            → cognitive.unknown_block_ticks    (5)
     - structural         → cognitive.structural_block_ticks (200)
5. BlockedIntent { opportunity, fact, expires_at, clearing_condition }
   → blocked_memory.record(intent)
6. runtime.dirty |= DirtySet::REPLAN_SIGNAL
```

### 7.2 Replan Triggers

| Trigger                                       | Source                                |
|-----------------------------------------------|---------------------------------------|
| Step revalidation failed                      | `execution.rs`                        |
| Action execution error from sim layer         | `execution.rs`                        |
| `PlanSearchResult::BudgetExhausted`           | `planning.rs`                         |
| `PlanSearchResult::FrontierExhausted`         | `planning.rs`                         |
| `PursuitInvalidationReason`                   | `plan_revalidation.rs`                |
| `InterruptTrigger::PlanInvalid`               | `active_action.rs` / `interrupts.rs`  |
| Belief contradiction (observation vs. snapshot)| `observation.rs` dirty-set           |
| `AssumptionEvalResult::Violated`              | `frame.rs`                            |

### 7.3 Interrupt Model

```rust
pub enum InterruptDecision {
    NoInterrupt,
    InterruptForReplan { trigger: InterruptTrigger },
}

pub enum InterruptTrigger {
    CriticalSurvival,       // Self-care pressure hit Critical
    CriticalDanger,         // Danger pressure hit Critical
    HigherPriorityGoal,     // New candidate outranks current
    SuperiorSameClassPlan,  // Better plan for same goal class
    PlanInvalid,            // Pursuit invalidation / step failure
    OpportunisticLoot,      // Visible high-value loot
}

pub fn evaluate_interrupt(
    active_plan:       &PlannedPlan,
    challengers:       &[RankedGoal],
    candidate_plans:   Option<&[CandidatePlanSearch]>,
    cognitive:         &CognitiveProfile,
) -> InterruptDecision;
```

Intention stability is controlled by two cognitive margins:

- `switch_margin` (default 100 ‰): how much better a challenger must
  be to *interrupt the active action*.
- `planning_switch_margin` (default 150 ‰): how much better a
  challenger must be to *replace a held plan* during planning.

These margins operationalise FND-21 ("Intentions Are Revisable
Commitments"): they are thresholds for revision, not locks.

### 7.4 Exhaustion and Retry

```rust
pub struct ExhaustionEntry {
    opportunity:          OpportunityKey,
    consecutive_failures: u8,
    last_failure_tick:    Tick,
    retry_eligible:       bool,
    next_retry_tick:      Option<Tick>,
    retry_state:          ExhaustionRetryState,
}

pub fn derive_invalidation_conditions(
    goal:            &GoalKind,
    agent:           EntityId,
    view:            &dyn GoalBeliefView,
    recipe_registry: &RecipeRegistry,
) -> (Vec<ExhaustionInvalidationCondition>, ExhaustionBaseline);

pub enum ExhaustionInvalidationCondition {
    PositionChanged,
    CommodityChanged(CommodityKind),
    WoundsChanged,
    TargetDead,
    HostilesChanged,
    // ... goal-specific variants
}

pub struct ExhaustionBaseline {
    position:              Option<EntityId>,
    needs:                 HomeostaticNeeds,
    commodity_quantities:  BTreeMap<CommodityKind, Quantity>,
    unique_item_counts:    BTreeMap<CommodityKind, u32>,
    wound_count:           u32,
    hostile_count:         u32,
}
```

Exponential backoff:
`cooldown := min(initial_cooldown_ticks * 2^failures, max_cooldown_ticks)`.

Retry policy: an exhausted goal is only re-offered if (a) invalidation
conditions hold compared to baseline AND (b) cooldown elapsed. This
ensures the agent does not thrash on structurally blocked goals.

### 7.5 BlockedIntentMemory Clearing

```rust
impl BlockedIntentMemory {
    pub fn record(&mut self, intent: BlockedIntent);
    pub fn expire(&mut self, tick: Tick);
    pub fn sweep_cleared<F: Fn(&BlockedIntent) -> bool>(&mut self, p: F);
}

pub enum BlockClearingCondition {
    TtlOnly,
    TargetAlive,
    CommodityAvailable,
    CustomCheck,
}
```

Clearing is explicit world-state-driven: e.g., a "no stock" block is
cleared when the relevant commodity appears in the snapshot. This keeps
the agent from being permanently stuck after conditions change
(FND-16 interacts cleanly here — the agent's memory is fallible state,
not global truth).

---

## 8. Cognitive Parameters

### 8.1 CognitiveProfile

```rust
pub struct CognitiveProfile {
    pub max_candidates_to_plan:             u8,
    pub max_candidates_per_expansion:       u16,
    pub max_plan_depth:                     u8,
    pub max_travel_candidates_per_expansion: Option<u16>,
    pub snapshot_travel_horizon:            u8,
    pub max_node_expansions:                u16,
    pub switch_margin:                      Permille,
    pub planning_switch_margin:             Permille,
    pub transient_block_ticks:              u32,
    pub unknown_block_ticks:                u32,
    pub structural_block_ticks:             u32,
    pub initial_cooldown_ticks:             u32,
    pub max_cooldown_ticks:                 u32,
    pub max_snapshot_entities_per_place:    u16,
    pub landmark_extraction_depth:          u8,
    pub use_ff_heuristic:                   bool,
}
```

Defaults and effects:

| Field                              | Default | Effect                                                         |
|------------------------------------|---------|----------------------------------------------------------------|
| `max_candidates_to_plan`           | 2       | Top-N ranked goals attempted per decision cycle                |
| `max_candidates_per_expansion`     | 200     | Successor cap inside `search_plan`                             |
| `max_plan_depth`                   | 8       | Max sequential actions per plan                                |
| `max_travel_candidates_per_expansion` | None | Optional cap on travel-only successors                         |
| `snapshot_travel_horizon`          | 6       | Reachability hops when building `PlanningSnapshot`             |
| `max_node_expansions`              | 224     | Hard cap on total frontier pops per search                     |
| `switch_margin`                    | 100‰    | Execution-time interrupt threshold (FND-21 stability)          |
| `planning_switch_margin`           | 150‰    | Plan-replacement threshold                                     |
| `transient_block_ticks`            | 20      | TTL for transient blocks (queued, temporary)                   |
| `unknown_block_ticks`              | 5       | TTL for unclassified failures                                  |
| `structural_block_ticks`           | 200     | TTL for structural blocks (no plan exists)                     |
| `initial_cooldown_ticks`           | 4       | Base retry cooldown                                            |
| `max_cooldown_ticks`               | 64      | Exponential backoff cap                                        |
| `max_snapshot_entities_per_place`  | 50      | Cap on entities per place in snapshot                          |
| `landmark_extraction_depth`        | 4       | Max depth for delete-relaxation landmark extraction            |
| `use_ff_heuristic`                 | true    | Enable FF relaxed-plan heuristic (inadmissible, more informed) |

### 8.2 ExecutionBudget

```rust
pub struct ExecutionBudget {
    beam_width:                 u8,  // default 8
    max_prerequisite_locations: u8,  // default 3
    preferred_operator_boost:   u8,  // default 2
}
```

| Field                        | Effect                                                                 |
|------------------------------|------------------------------------------------------------------------|
| `beam_width`                 | Top-K successors retained per expansion                                |
| `max_prerequisite_locations` | Max acquisition places chained in the strategic plan per commodity     |
| `preferred_operator_boost`   | Consecutive preferred-queue pops after a preferred successor is found  |

### 8.3 Agent Diversity (FND-22) via Profiles

Profiles are per-agent data (registered via `AgentDef` +
`spawn_agent()` per the scenario-profile-completeness invariant), so
two agents in the same role can differ in:

- Search breadth vs. committedness (`beam_width`, `switch_margin`,
  `planning_switch_margin`).
- Planning horizon (`snapshot_travel_horizon`, `max_plan_depth`).
- Retry tempo (`initial_cooldown_ticks`, `max_cooldown_ticks`,
  `transient_block_ticks`, `structural_block_ticks`).
- Heuristic quality (`landmark_extraction_depth`, `use_ff_heuristic`).

Diversity therefore emerges from scenario authoring, not from random
noise injected at decision time.

---

## 9. FOUNDATIONS Alignment

Principle text embedded inline. Each section records how the current
planner satisfies the principle and where tension points remain.

### FND-1. Maximal Emergence Through Local Causality

> "Worldwake exists to produce emergent behavior through interacting
> systems and agents, never through authored sequences, hidden quest
> logic, or one-off story triggers. An event is valid only if it arose
> from prior world state, agent belief, institutional rule, or natural
> process already present in the simulation."
>
> **Test**: If the only honest explanation for an event is "the game
> decided something interesting should happen now," the design violates
> this principle.

**Alignment.** The planner consumes `GroundedGoal` candidates produced
from the agent's beliefs and selects actions from the same
`ActionDefRegistry` available to everyone. No goal kind encodes a
one-off story beat; every `GoalKind` corresponds to a class of agent
desire (self-care, acquisition, danger, institution, social, economic).
The 17 emission gates are lawful affordance enumerators, not scripted
triggers.

**Tension.** `emit_proactive_exploration_candidates` is a soft driver
that can look like a "diversity garnish." It is FND-1-compatible only
if its motivation is an actual agent property (e.g., curiosity,
epistemic profile) rather than a designer lever — worth inspecting.

### FND-3. Concrete State Over Abstract Scores

> "Prefer modeling the thing itself over a score that represents it.
> Danger should come from actual threats on routes, not `danger_score`.
> Scarcity should come from inventories, queues, failed purchases, and
> unmet needs, not `scarcity_score`."

**Alignment.** Danger is computed from `DangerAssessment`
(attackers, hostiles, wounds, incapacitation). Pressure uses `Permille`,
but the `Permille` value is a **derived view** of concrete state, not
the authoritative source (FND-27). Suppression gates reference the
concrete class, not a raw number.

**Tension.** `motive_score: u32` is the dominant second-level sort key
and is populated by `goal_motive_score(cand)`. If that function ever
becomes a naked designer dial rather than a transparent function over
beliefs and needs, the ranking layer silently drifts to FND-2 violation
territory. Worth inspecting the `UtilityProfile` shape for opaque
dials.

### FND-7. Locality of Motion, Interaction, and Communication

> "All physical interaction requires co-location or explicit range. All
> communication requires co-location or a physical carrier moving
> through the place graph... Agents, institutions, and planners may not
> query global truth on behalf of a character."

**Alignment.** `RuntimeBeliefView` is the only surface the planner sees
during affordance enumeration. Target enumeration scoping
(`EntityAtActorPlace`, `EntityInInventory`, `EntityInContainer`) is
intrinsically local. `PlanningSnapshot` is constructed from beliefs
with a reachability horizon of `snapshot_travel_horizon`, never from
global world state.

**Tension.** `PlanningSnapshot` stores **Floyd-Warshall precomputed**
distance matrices (`shortest_travel_ticks`, `perceived_travel_costs`).
If `shortest_travel_ticks` is authoritative ground-truth distance
(not perceived), it grants the planner knowledge the agent may not
have. The presence of both fields suggests a deliberate split —
`perceived_travel_costs` for planning, `shortest_travel_ticks` for
perhaps heuristic bounds — but an external evaluator should confirm
that the planner in fact only consults the perceived matrix.

### FND-12. Performance May Compress Computation, Never Causality

> "Optimization is allowed. Causal cheating is not. Offscreen
> simulation may batch, summarize, sleep, or approximate only if
> causally relevant outcomes remain equivalent to the explicit model."

**Alignment.** The planner operates on immutable snapshots; successor
states are `SharedMap` CoW overlays that never mutate the world.
Beam truncation, budget caps, and landmark depth limits are **search
approximations**, not world approximations.

**Tension.** When `search_plan` returns `BudgetExhausted` and
`handle_plan_failure` fires, the agent records a `BlockingFact::Unknown`
with a short TTL. The "unknown" label can hide legitimate structural
blockers (e.g., FND-7 actually prevents the plan because the agent
cannot know enough to complete it). The architecture should ensure
that short TTLs on "unknown" blockers do not cause the planner to
repeatedly burn the search budget on structurally-impossible goals —
which would be a performance compromise, not a causal one, but still
worth tracing.

### FND-14. World State Is Not Belief State

> "Ground truth and agent knowledge are separate layers. Agents act on
> what they believe, remember, infer, suspect, and are told — not on
> what the simulation knows to be true. A planner may consult only the
> agent's accessible belief state."
>
> **Test**: If an agent can plan around a fact it has never perceived,
> inferred, remembered, or been told, the design violates this
> principle.

**Alignment.** `PlanningSnapshot::actor_belief_store` is the only
first-class source of truth the planner sees. Entity subsets in
`snapshot.entities` and `snapshot.places` are derived from the agent's
beliefs. `is_pursuit_plan_invalid` uses a believed-last-place check,
not ground truth.

**Tension.** See FND-7 — the `shortest_travel_ticks` matrix on the
snapshot is an omniscient-adjacent signal if ever used by the search
heuristic. External evaluation should confirm `compute_heuristic` and
`is_pursuit_plan_invalid` reference only the perceived/belief variants.

### FND-16. Ignorance, Uncertainty, and Contradiction Are First-Class

> "Agents must be able to not know, to suspect, to misremember, to
> hold stale beliefs, and to believe false or conflicting reports...
> The simulation must support cases where one witness says the beast
> fled east, another says west, and the town reacts imperfectly."

**Alignment.** `BeliefConfidencePolicy` decays information over time;
`is_pursuit_plan_invalid` rejects plans whose underlying belief falls
below `min_location_confidence`. `BlockedIntentMemory` treats past
failures as **state**, not ground truth — the agent can be wrong about
whether a blocker still holds, and correcting predicates
(`TargetAlive`, `CommodityAvailable`) clear blockers based on *new*
observations, not omniscience.

**Tension.** `BlockingFact::Unknown` with a short TTL is a sensible
default for unclassified failures, but it collapses three different
epistemic states (actually unknown cause; structurally impossible;
transient race) into one bucket. That may be the right pragmatic
choice, but it is also the kind of collapse FND-16 warns about.

### FND-20. Resource-Bounded Practical Reasoning Over Scripts

> "AI agents must reason as limited actors in a dynamic world, using
> beliefs, priorities, habits, skills, and commitments to choose
> actions. Plans exist to make reasoning tractable under limited time
> and limited knowledge, not to hard-script a performance."
>
> "Any planner formalism may encode only reusable lawful affordances,
> decomposition knowledge, or search control. It may not encode plot
> progression, scene-specific rails, target-specific success paths, or
> hidden exception logic."
>
> **Test**: For any decision, you must be able to explain it as
> "Agent X chose Y because they believed Z and cared about Q."

**Alignment.** `CognitiveProfile.max_node_expansions`,
`max_plan_depth`, and `beam_width` enforce resource bounds. Strategic
decomposition (`TacticalSubGoal::AcquirePrerequisite`) is lawful —
acquisition chains the same affordances everyone uses. The tactical
search operates only on declared `ActionDef`s and `PlannerOpKind`s.

**Tension.** The FF heuristic is computed from a relaxed plan over the
operator set. That is standard, but external evaluators should confirm
FF helpful-action marking doesn't bypass locality: FF works on facts
and operators, not on the agent's perceived reachability. If an FF
relaxation can reach a goal fact via an operator the agent believes
unreachable, the "preferred" label might briefly inject omniscient
guidance — though the regular frontier would still have to surface the
concrete successor through the belief-bounded CoW state.

### FND-21. Intentions Are Revisable Commitments

> "Agents need commitments so they do not thrash between options every
> tick. But commitments are never rails. They are stable intentions
> held under assumptions... Intent is not entitlement. A plan reserves
> nothing unless the world contains an explicit reservation, queue
> position, contract, assignment, or other claim artifact."

**Alignment.** `switch_margin` and `planning_switch_margin` give
commitments inertia without locking them. `evaluate_interrupt` considers
challenger plans. `abandon_expired_facility_queues` explicitly clears
stale queue intents. The `IntentionFrame` is revisable on observation.

**Tension.** None structurally — but the *default* margins (100‰ and
150‰) are engine-wide. FND-22 expects per-agent diversity here;
confirm that scenarios author differing margins per agent rather than
using defaults universally.

### FND-22. Agent Diversity Through Concrete Variation

> "Agents in the same role must differ in needs, skills, values,
> loyalties, courage, greed, patience, memory reliability, perception
> fidelity, and tolerance for risk or ambiguity... Homogeneous
> populations collapse into herd behavior and single-path outcomes."

**Alignment.** `CognitiveProfile` and `ExecutionBudget` are per-agent.
`UtilityProfile`, `BeliefConfidencePolicy`, `TellProfile`,
`EpistemicDispositionProfile`, `ThresholdBand` are per-agent and
attach via scenario authoring. The scenario-profile-completeness
invariant (see `docs/spec-drafting-rules.md` §5) ensures every profile
is scenario-definable.

**Tension.** The report's defaults table (Section 8.1/8.2) shows single
default values. Diversity depends on scenarios authoring non-default
profiles; if scenarios rely heavily on defaults, the population will
be homogeneous in planner behavior, violating FND-22 in practice even
though the architecture permits variation.

### FND-28. No Backward Compatibility in Live Authority Paths

> "Do not preserve dead abstractions, alias paths, compatibility layers,
> deprecated shims, or legacy systems inside the live authoritative
> simulation simply because old code once depended on them."

**Alignment.** Only one live path exists for goal generation (the 17
gates), one for ranking (`rank_candidates`), one for search
(`search_plan`). No compatibility aliases observed in the planner
surface.

**Tension.** The payload-override revalidation path
(`revalidate_best_effort_payload_override_step`) is a *second* route
to validate a plan step beside the primary
`requested_affordance_matches` path. This is by design for
planner-synthesized payloads — but it creates two authoritative
validation paths. If a handler forgets to register
`with_payload_override_validator`, the planner can produce plans that
bypass the handler's intended checks. That's not FND-28 as literally
written, but it is the same *spirit*: dual authority paths are fragile.

### FND-29. Debuggability Is a Product Feature

> "Emergence without introspection is indistinguishable from bugs."
>
> The simulation must support questions such as: Why did this agent do
> that? Why did this caravan take this road? Why is this stash empty?
>
> **Test**: For any nontrivial event chain, you must be able to inspect
> both the causal path and the knowledge path separately.

**Alignment.** `DecisionTraceSink`, `PlanAttemptTrace`,
`SearchExpansionSummary`, `CandidateTrace`, `BindingRejection`,
`ExhaustionTraceEntry`, and `UnknownBlockerTrace` capture the decision
cycle at each layer (candidates, ranking, search expansions, binding
resolution, exhaustion, blockers). `goal_explanation.rs` presumably
surfaces agent-readable explanations.

**Tension.** Traces are opt-in (`trace_sink: Option<...>`). When
disabled, post-hoc reconstruction depends on the event log alone.
Whether the event log captures enough to answer "why did the planner
pick this and not that?" without the trace sink is worth confirming.

### FND-26. Systems Interact Through State, Not Through Each Other (bonus, relevant)

> "Systems do not imperatively command each other to force outcomes.
> They read authoritative state, local beliefs, and prior records;
> they write new state, effects, and records."

**Alignment.** The planner writes only `RequestAction` inputs to the
scheduler; it never invokes system mutators directly. Needs, combat,
production, trade systems communicate with the planner exclusively via
the event log and agent beliefs.

**Tension.** The handler's `payload_override_is_valid` closure is
called from `plan_revalidation.rs` (planner side) but is registered
from handler-side code (sim side). Conceptually this is a generic
service boundary (FND-26 allows legality checks as generic domain
services), but it's a privileged callback — external review should
confirm these closures are pure checks and don't mutate state.

---

## 10. Live Diagnostics

The trace infrastructure for planning metrics is in place but not
driven to produce aggregate statistics inside this report.

### Trace Types

```rust
pub struct PlanAttemptTrace {
    pub opportunity:  OpportunityKey,
    pub ranked_goal:  RankedGoalSummary,
    pub outcome:      PlanSearchOutcome,
    pub plan_trace:   Option<PlanSearchTrace>,
}

pub struct SearchExpansionSummary {
    pub expansion_index:        u16,
    pub frontier_size:          u16,
    pub expansions_so_far:      u16,
    pub remaining_travel_ticks: u32,
    pub travel_pruning:         Option<TravelPruning>,
}

pub struct CandidateTrace {
    pub ranked:                Vec<RankedGoalSummary>,
    pub top_ranked_comparison: Option<RankedGoalComparison>,
    pub excluded:              Vec<ExclusionReason>,
}

pub struct BindingRejection {
    // records which affordance bindings were rejected and why
}

pub struct UnknownBlockerTrace {
    // active BlockingFact::Unknown intents at trace time
}

pub struct ExhaustionTraceEntry {
    // snapshot of exhausted goal's retry state
}
```

### What They Capture

| Signal                               | Source                                        |
|--------------------------------------|-----------------------------------------------|
| Candidates ranked + excluded         | `CandidateTrace`                              |
| Goal attempt outcome                 | `PlanAttemptTrace.outcome`                    |
| Per-expansion frontier size          | `SearchExpansionSummary`                      |
| Expansions used                      | `PlanSearchResult::{Budget,Frontier}Exhausted`|
| Helpful/preferred markings           | Not directly captured                         |
| Landmark count / coverage            | Not directly captured                         |
| Cooldown / exhaustion state          | `ExhaustionTraceEntry`                        |
| Blocked intents                      | `UnknownBlockerTrace`                         |

### Gaps

These metrics would be valuable but are not currently captured in
structured form:

- **Branching factor per expansion** (successor count before/after beam
  truncation). Useful for evaluating whether `beam_width = 8` is
  appropriate.
- **Landmark set size** per search (how many landmarks extracted, how
  many orderings, how many actionable at search start). Useful for
  evaluating landmark extraction ROI.
- **FF relaxed-plan length** at root. A proxy for heuristic
  informedness.
- **Time spent in strategic vs. tactical planner**. Useful for
  profile-level optimisation.
- **Goal-churn rate**: fraction of ticks where the active goal changed.
  Useful for evaluating `switch_margin` appropriateness per agent.
- **Replan rate**: plan failures per completed plan.

Live diagnostics would need to be extracted by running representative
scenarios with tracing enabled and summarising trace output. This
report does not run those scenarios; that is a separate task.

---

## 11. Architectural Observations

Flagged for external evaluation only. No recommendations.

1. **Dual validation paths for payload overrides.** Plan revalidation
   consults two paths: `requested_affordance_matches` (primary) and
   `revalidate_best_effort_payload_override_step` (for planner-synthesised
   payloads). The latter depends on handler-side registration of
   `with_payload_override_validator`. Forgetting to register the
   validator silently causes step revalidation to fail — a contract
   that is easy to miss in new handlers. See the Authoritative-to-AI
   Impact Rule checklist item #6 in `CLAUDE.md`.

2. **Default-inadmissible heuristic.** `use_ff_heuristic = true` is the
   default, combining spatial (admissible) with FF (inadmissible) via
   max. This trades optimality guarantees for goal-focus on typical
   instances. Agents using the default profile get an inadmissible
   heuristic; only profiles with `use_ff_heuristic = false` revert to
   max(spatial, landmark-count), which is admissible. Plan optimality
   depends on agent profile.

3. **Floyd-Warshall matrices on the snapshot.** `PlanningSnapshot`
   stores two distance matrices — `shortest_travel_ticks` and
   `perceived_travel_costs`. If both are used during search, the
   planner has access to ground-truth distance in addition to
   perceived cost. Whether `shortest_travel_ticks` is ever read by
   heuristics or pursuit logic should be verified against FND-7 /
   FND-14.

4. **17 emission gates in a fixed order.** The sequence is implicitly
   authoritative: if two gates produce goals that reference the same
   opportunity, only one may survive (dedup logic not covered here).
   Gate ordering can affect which goal provenance wins — a subtle form
   of authoring influence on ranking.

5. **`BlockingFact::Unknown` with short TTL.** Unclassified planner
   failures get a 5-tick retry window by default. This masks three
   distinct epistemic states: actually unknown cause, structurally
   impossible, and transient race. May cause the planner to repeatedly
   burn `max_node_expansions` on structurally-impossible goals until
   the structural classification path catches them.

6. **Asymmetry between `max_candidates_per_expansion` (200) and
   `beam_width` (8).** An expansion may generate up to 200 successors
   but only enqueue 8. The 192 discarded successors are scored,
   heuristically evaluated, and then thrown away. If successor
   generation is expensive, this is a hot loop. If it's cheap, the
   asymmetry provides robust pruning.

7. **Landmark extraction runs once per search (first expansion).**
   This is efficient but means landmark freshness is bounded by search
   duration. On long searches with `max_node_expansions = 224` and
   `max_plan_depth = 8`, landmarks can become stale relative to the
   current node's hypothetical state. Whether landmark-count heuristic
   values account for already-achieved landmarks along the search path
   (via `actionable_landmarks(&landmarks, &current_facts)`) is the
   mechanism that compensates.

8. **`ExecutionBudget` has only 3 knobs.** `beam_width`,
   `max_prerequisite_locations`, and `preferred_operator_boost`. Agent
   diversity in search behavior is dominated by `CognitiveProfile`
   rather than `ExecutionBudget`. If authoring relies solely on
   `ExecutionBudget` defaults, per-agent search shape is effectively
   homogeneous.

9. **Strategic planner's `2 * max_prerequisite_locations` search
   budget.** The Dijkstra frontier cost cap scales linearly with a
   single integer knob. For agents with long prerequisite chains (e.g.,
   sophisticated crafting), this can silently cap plan feasibility
   even when tactical search would succeed.

10. **Interrupts can trigger expensive replans.** In
    `handle_active_action_phase`, challengers get full GOAP searches
    for the interrupt decision. With `max_node_expansions = 224` and
    multiple challengers, the cost of interrupt evaluation can dwarf
    the cost of the committed plan's next step. Worth profiling in
    crowded scenarios.

11. **Belief-vs-snapshot drift during long actions.** `PlanningSnapshot`
    is built at planning time. While the plan executes (multi-tick
    actions), the agent's real beliefs may diverge from the snapshot's
    cached belief store. Revalidation runs against a fresh
    `RuntimeBeliefView`, but other plan steps are not continuously
    revalidated — only the next one. Stale mid-plan assumptions rely on
    `AssumptionEvalResult::Violated` in `frame.rs`.

12. **`max_candidates_to_plan = 2` is small.** Only the top 2 ranked
    goals are fed to `search_plan`. If the ranking is noisy or if the
    top 1–2 goals are infeasible but the 3rd is trivially doable, the
    agent waits a tick (after exhaustion records the failure) before
    discovering the 3rd goal. For scenarios with many competing modest
    goals, this can feel laggy compared to a broader search.
