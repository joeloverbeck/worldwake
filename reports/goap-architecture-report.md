# GOAP Architecture Reference — 2026-05-13

This document is a self-contained technical reference for Worldwake's goal-oriented action planning (GOAP) decision pipeline. It is intended for external evaluation (e.g. ChatGPT Pro) without repository access, so it embeds the relevant type definitions, function signatures, algorithm pseudocode, and foundational principle texts inline. All paths in section headings are repository-relative.

The pipeline being documented is the per-agent decision cycle that runs each simulation tick: belief view → goal ranking → candidate generation → affordance enumeration → two-tier plan search (strategic itinerary + tactical action sequence) → plan revalidation → action dispatch → on-failure replanning.

---

## 1. Architecture Context

### 1.1 Workspace shape

Worldwake is a Rust workspace with five crates (`crates/`):

```
worldwake-core    → IDs, types, ECS store, topology, items, relations (no deps)
worldwake-sim     → Event log, action framework, scheduler, replay (deps: core)
worldwake-systems → Needs/metabolism, production/crafting, trade, combat,
                    travel/transport actions (deps: core, sim)
worldwake-ai      → Pressure-based GOAP planner, goal ranking, decision runtime
                    (deps: core, sim, systems)
worldwake-cli     → Human control interface (deps: all)
```

The planner described in this report lives entirely in `worldwake-ai`. It consults shapes (`ActionDef`, `RuntimeBeliefView`, `BlockerMemory`, event log) defined in `worldwake-core` and `worldwake-sim`.

### 1.2 ECS basics

- Custom ECS, no external crate. Typed components are stored in deterministic `BTreeMap`s keyed by entity ID.
- The world is a place graph (`worldwake-core::Topology`) — not continuous space. Travel times are integer ticks on edges between place entities.
- `EntityId` is a 32-bit handle; all positions, items, agents, places, records, and offices share the same handle space.

### 1.3 Determinism contract

- Randomness: `ChaCha8Rng`, seeded.
- No floats anywhere in authoritative state.
- No `HashMap`/`HashSet` in authoritative state — only `BTreeMap`/`BTreeSet` so iteration order is stable.
- No wall-clock time. The only clock is `Tick`, a monotone `u32`.
- Append-only causal event log is the source of truth; world state is a derived view of events.

### 1.4 Foundational scalar types used throughout the planner

- `Tick(u32)` — simulation time.
- `EntityId(u32)` — handle for any persistent thing.
- `Permille` — integer in [0, 1000] representing a [0, 1] proportion. Replaces `f32` everywhere the planner stores ratios (confidence, switch margins, salience).
- `Quantity(u32)` — integer count of commodity units, item-lot sizes, etc.
- `Permille::new_unchecked(n)` is the `const fn` constructor used in defaults.

### 1.5 Belief-state separation (FND-14)

The planner never reads world state directly on behalf of an agent. It consults a `RuntimeBeliefView` (a compound trait described in §4) that exposes only what the agent has perceived, remembered, been told, or inferred. The single exception is FND-14A: same-tick observation of co-located physical properties (e.g. an item lot at the actor's place, a workstation tag, a container's contents) may be read from world state because a correct perception pipeline would deliver those facts on the same tick. Social/relational facts (ownership, jurisdiction, effective rights) always require an explicit belief entry, even when the subject is co-located.

This split is implemented in `crates/worldwake-sim/src/per_agent_belief_view.rs` and is the bedrock that everything downstream (candidate generation, affordance queries, planning snapshot construction) is built on.

---

## 2. Goal Ranking

### 2.1 What this stage produces

Given the agent's belief view, current pressures (needs, danger, commitments), blocked-intent memory, and prior agenda, this stage emits a totally-ordered list of `AgendaEntry` (ranked candidates) for downstream planning. Only the top `CognitiveProfile.max_candidates_to_plan` entries are passed to plan search.

### 2.2 The `GoalKind` enum (full variant set)

`GoalKind` is the central enumeration of what an agent can want. It has 32 variants across seven families:

```rust
pub enum GoalKind {
    // --- Self-care / homeostasis ---
    ConsumeOwnedCommodity { commodity: CommodityKind },
    AcquireCommodity {
        commodity: CommodityKind,
        purpose: CommodityPurpose,
        quantity: AcquisitionQuantity,
    },
    Sleep,
    Relieve,
    Wash,
    FreeCarryCapacity,

    // --- Danger reduction & combat ---
    EngageHostile { target: EntityId },
    RaidTarget { target: EntityId },
    ReduceDanger,
    RegroupWithFaction { faction: EntityId },
    EstablishBanditCamp { faction: EntityId },

    // --- Healing ---
    TreatWounds { patient: EntityId },

    // --- Missing-person handling ---
    SearchForMissing { subject: EntityId, last_seen: Option<EntityId> },
    ReportMissing {
        subject: EntityId,
        to_office: Option<EntityId>,
        expectation_id: Option<ExpectationId>,
    },
    ReportFound { subject: EntityId, expectation_id: ExpectationId },
    EscortToSafety { subject: EntityId, destination: EntityId },

    // --- Enterprise / economic ---
    ProduceCommodity { recipe_id: RecipeId },
    SellCommodity { commodity: CommodityKind },
    RestockCommodity { commodity: CommodityKind },
    MoveCargo { commodity: CommodityKind, destination: EntityId },

    // --- Corpse / artifact / bounty ---
    LootCorpse { corpse: EntityId },
    BuryCorpse { corpse: EntityId, burial_site: EntityId },
    FulfillBounty { bounty: EntityId },
    PostBounty { posting: ArtifactPostingContext, terms: BountyTerms },
    PostNotice { posting: ArtifactPostingContext, topic: NoticeTopic },

    // --- Social / epistemic ---
    ShareBelief {
        listener: EntityId,
        topic: TellTopic,
        communication_class: CommunicationClass,
    },
    AskWitness { witness: EntityId, topic: TellTopic },

    // --- Political / institutional ---
    ClaimOffice { office: EntityId },
    SupportCandidateForOffice { office: EntityId, candidate: EntityId },

    // --- Justice / patrol ---
    InvestigateViolation { violation_id: ViolationId, place: EntityId },
    Patrol { place: EntityId },
    Accuse {
        crime_register: EntityId,
        accused: EntityId,
        violation_id: ViolationId,
    },
    PunishAccused {
        office: EntityId,
        accused: EntityId,
        accusation_entry: RecordEntryId,
        punishment: PunishmentKind,
    },

    // --- Exploration / theft ---
    ExploreLocation {
        target_place: EntityId,
        motivating_need: ExplorationMotivation,
        hypothesis: HypothesisKind,
    },
    StealItem { target_item: EntityId },
}
```

Each variant declares the world condition the agent wants to bring about — never the one-step solution. The planner is responsible for synthesizing a lawful sequence to satisfy the variant (FND-20).

### 2.3 `GoalOffer` (what candidate generation emits)

```rust
pub struct GoalOffer {
    pub key: GoalKey,                       // (kind discriminant + arguments)
    pub anchor: OpportunityAnchor,          // place/entity anchor for ranking
    pub evidence_entities: BTreeSet<EntityId>,
    pub evidence_places: BTreeSet<EntityId>,
    pub obligation_source: Option<EntityId>,
    pub commitment_impact_if_ignored: Permille,
    pub required_information_gaps: Vec<BeliefClaimKey>,
    pub invalidators: Vec<Invalidator>,
    pub learned_expectation_refs: Vec<ExpectationId>,
    pub motive_sources: Vec<MotiveSourceRef>,
    pub acquisition_quantity: Option<AcquisitionQuantity>,
}
```

`GoalOffer` carries enough provenance to (a) rank under multiple motive sources, (b) attach guards/invalidators to the resulting plan steps, and (c) discount a goal when a learned-source has previously failed.

### 2.4 `rank_candidates` — entry point

```rust
pub fn rank_candidates(
    candidates: &[GoalOffer],
    view: &dyn GoalBeliefView,
    agent: EntityId,
    current_tick: Tick,
    utility: &UtilityProfile,
    decision_context: &DecisionContext,
) -> RankingOutcome
```

`DecisionContext` carries two pressure summaries derived from the belief view:

- `max_self_care_class` — highest of hunger, thirst, fatigue, bladder, dirtiness classes (`Background / Low / Medium / High / Critical`).
- `danger_class` — derived from current attackers, visible hostiles, hostile targets, wounds, incapacitation.

### 2.5 Ranking algorithm

For each emitted `GoalOffer`:

1. **Suppression filter** (`evaluate_suppression(goal_kind, decision_context)`):
   - **Never suppressed**: all self-care, danger reduction, healing, combat/raid, enterprise, theft.
   - **Suppressed when max_self_care_class ≥ High OR danger_class ≥ High**: corpse handling, social/political goals, investigation.
   - **Suppressed when max_self_care_class ≥ Critical OR danger_class ≥ Critical**: `AskWitness` (epistemic sensing).
2. **Priority class** (`ranked_priority_class`): maps goal kind + motive source + pressure into one of `Background`, `Low`, `Medium`, `High`, `Critical`.
3. **Motive score** (`ranked_motive_score`): integer in [0, ~10_000] composed from each motive source's contribution.
4. **Source-reliability discount**: subtract a fraction if a contributing motive source has a recent failure record.
5. **Competition discount**: when multiple candidates share the same `GoalKey`, lower-ranked competitors are reduced so a single representative carries through.
6. **Total-order sort** (`compare_ranked_goals`): `(priority_class, motive_score, deterministic tiebreaks)`.

### 2.6 Suppression sources beyond pressure

Three additional filters can drop a candidate:

- `BlockerMemory.intents` — keyed by `(goal_key, place?, target?, action_def?)`. If any matching blocker has not yet expired and its `BlockingFact` is generation-blocking (everything except `ExclusiveFacilityUnavailable` and `SourceDepleted`), the candidate is suppressed.
- `DiscrepancyMemory` — records of mismatches between expected and observed state with their own TTL (per-discrepancy entries from `CognitiveProfile`'s `*_backoff_ticks` family).
- `ViolationMemory` — known violations the agent has witnessed/recorded; gates Accuse / Investigate emissions.

### 2.7 `BlockerKey`, `Blocker`, and `BlockingFact`

```rust
pub struct BlockerKey {
    pub goal_key: GoalKey,
    pub place: Option<EntityId>,
    pub target: Option<EntityId>,
    pub action_def: Option<ActionDefId>,
}

pub struct Blocker {
    pub blocker_key: BlockerKey,
    pub blocking_fact: BlockingFact,
    pub diagnostic_context: Option<BlockerDiagnostic>,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
    pub clearing_condition: BlockerClearingCondition,
    pub baseline_snapshot: Option<ClearingBaseline>,
}
```

`BlockingFact` variants observed in candidate-generation code paths include `NoKnownPath`, `NoKnownSeller`, `SellerOutOfStock`, `TooExpensive`, `SourceDepleted`, `WorkstationBusy`, `ReservationConflict`, `TargetGone`, `DangerTooHigh`, `ExclusiveFacilityUnavailable`. The `clearing_condition` records what world change would let the blocker expire early (e.g., `CommodityAvailabilityChanged`, `PathDiscovered`, `EntityReappeared`).

### 2.8 `RankingOutcome` (what is passed forward)

```rust
pub struct RankingOutcome {
    pub ranked: Vec<AgendaEntry>,
    pub suppressed: Vec<CandidateSuppressionDiagnostic>,
    pub damped: Vec<CandidateDampingEntry>,
    pub zero_motive: Vec<GoalKey>,
}

pub struct AgendaEntry {
    pub key: AgendaEntryKey,
    pub offer: GoalOffer,
    pub phase: AgendaPhase,
    pub origin: AgendaOrigin,
    pub introduced_tick: Tick,
    pub last_reconsidered_tick: Tick,
    pub revival_trigger: Option<RevivalTrigger>,
    pub kill_condition: KillCondition,
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
    pub motive_source_contributions: Vec<(MotiveSourceRef, u32)>,
    pub provenance: Option<RankedGoalProvenance>,
    pub source_reliability_discount: Option<SourceReliabilityDiscount>,
    pub competition_discount: Option<CompetitionDiscount>,
    pub source_composite: Option<SourceCompositeRank>,
    pub feasibility: FeasibilityHint,
}
```

Only the top `max_candidates_to_plan` (default 2) entries proceed to plan search.

---

## 3. Candidate Generation

### 3.1 Entry points

```rust
pub fn generate_candidates(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    blocked: &BlockerMemory,
    recipes: &RecipeRegistry,
    current_tick: Tick,
) -> Vec<GoalOffer>;

pub fn generate_candidates_with_travel_horizon(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    blocked: &BlockerMemory,
    violation_memory: &ViolationMemory,
    recipes: &RecipeRegistry,
    current_tick: Tick,
    travel_horizon: u8,
    tracing_enabled: bool,
) -> CandidateGenerationResult;
```

`travel_horizon` defaults to `CognitiveProfile.snapshot_travel_horizon` (default 6). It scopes how many travel hops out from the actor candidates can reference.

### 3.2 The emission gates (the ~17 `emit_*` functions)

Candidate generation is a fan-out of independent gates, each of which inspects the belief view for evidence of a particular kind of opportunity and emits zero or more `GoalOffer`s:

| Gate function | Emits |
|---|---|
| `emit_opportunity_compiler_candidates` | Re-emits `AcquireCommodity` from a learned opportunity compiler. |
| `emit_need_candidates` | Homeostatic drives: `ConsumeOwnedCommodity`, `AcquireCommodity` (self-serve), `Sleep`, `Relieve`, `Wash`, dirtiness-driven acquisition. |
| `emit_production_candidates` | `ProduceCommodity` with a recipe known to the agent. |
| `emit_enterprise_candidates` | `RestockCommodity`, `SellCommodity`, `MoveCargo` for merchant-role agents. |
| `emit_bounty_candidates` | `FulfillBounty` parsed from believed artifact records (`EliminateEntity`, `DeliverCommodity`). |
| `emit_artifact_posting_candidates` | Generic artifact (bounty/notice) discovery scanning for unactionable constraints. |
| `emit_bounty_posting_candidates` | `PostBounty` from tactical threat assessments. |
| `emit_notice_posting_candidates` | `PostNotice` (`ThreatWarning`, `GeneralNotice`) scoped to jurisdiction. |
| `emit_combat_candidates` | `EngageHostile`, `RaidTarget` from hostility detection. |
| `emit_crime_candidates` | `InvestigateViolation`, `Patrol` from violation memory. |
| `emit_justice_candidates` | Justice meta-goals (accusation prerequisites, punishment precedent). |
| `emit_accusation_candidates` | `Accuse` grounded in violation records + witness evidence. |
| `emit_punishment_candidates` | `PunishAccused` from institutional authority + case records. |
| `emit_social_candidates` | `ShareBelief`, `AskWitness` from social disposition + knowledge gaps. Cap of 3 `AskWitness` per topic (`ASK_WITNESS_EMISSION_CAP_PER_TOPIC`). |
| `emit_regroup_with_faction_goals` | `RegroupWithFaction` from faction membership + danger pressure. |
| `emit_political_candidates` | `ClaimOffice`, `SupportCandidateForOffice` from succession law + institutional slots. |
| `emit_claim_office_candidate` / `emit_support_candidate_goals` | Per-instance expansions of the political family. |

Every gate is read-only against the belief view and may consult `blocked`, `violation_memory`, and `recipes`.

### 3.3 Blocked-intent filtering

After each gate emits, `filter_suppressed_candidates` runs:

```rust
fn find_matching_suppression(
    candidate: &GoalOffer,
    blocked: &BlockerMemory,
    discrepancies: &DiscrepancyMemory,
    current_tick: Tick,
) -> Option<SuppressionMatch>;
```

- Matches against `BlockerMemory.intents` and `DiscrepancyMemory.entries`.
- A blocker matches if `goal_key` matches *and* (place/target/action_def scoping is satisfied by the candidate's evidence + anchor).
- A blocker that has expired (`expires_tick <= current_tick`) does not match.
- For `BlockingFact::TargetGone` the match also requires that the candidate's target equals the blocker's target.

When all emissions for a `goal_key` are suppressed, that fact is recorded in `CandidateGenerationResult` diagnostics so ranking can surface "fully blocked" desires.

### 3.4 How candidate count relates to branching factor

There is no hard global cap on the number of candidates emitted; the upper bound is set by belief content (item lots, perceived sellers, perceived offices, known violations). Practical bounds:

- `AskWitness` caps at 3 per topic.
- Opportunity-compiler de-duplicates on `goal_key`.
- The travel horizon bounds how many place-scoped candidates can appear (e.g., `Patrol(place)` only for places within `snapshot_travel_horizon` hops).
- Downstream ranking truncates to `max_candidates_to_plan` (default 2).

So the planner does not pay full search cost per emitted candidate — only for the top 2.

### 3.5 `relevant_op_kinds` (link to affordance pruning)

Each `GoalKind` carries a static list of `PlannerOpKind` discriminants describing which action families are even worth enumerating for it:

```rust
pub trait GoalKindPlannerExt {
    fn relevant_op_kinds(&self) -> &'static [PlannerOpKind];
    // ... goal_relevant_places(), is_satisfied(state), goal_facts(...), etc.
}
```

For example `AcquireCommodity` lists `Trade`, `Harvest`, `Production`; it does not list `Attack` or `DeclareSupport`. Affordance enumeration uses this list to restrict to candidate-relevant `ActionDef`s.

---

## 4. Affordance Queries

### 4.1 Entry points

```rust
pub fn get_affordances(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> Vec<Affordance>;

pub fn get_affordances_for_defs(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    allowed_defs: &BTreeSet<ActionDefId>,
) -> Vec<Affordance>;
```

The `_for_defs` variant is used during plan search — only `ActionDef`s mapped to the candidate's `relevant_op_kinds` are enumerated.

### 4.2 `ActionDef` (parameterized action schema)

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
    pub binding_strictness: BindingStrictness,
    pub guard_template: Option<GuardTemplateSpec>,
    pub expectation_template: Vec<ExpectationTemplateSpec>,
    pub effect_schema: EffectSchema,
}
```

### 4.3 `TargetSpec` (locality-scoped target enumeration)

```rust
pub enum TargetSpec {
    SpecificEntity(EntityId),
    ActorPlace,
    EntityAtActorPlace { kind: EntityKind },
    EntityAtActorPlaceAnyOf { kinds: [EntityKind; 2] },
    EntityDirectlyPossessedByActor { kind: EntityKind },
    EntityDirectlyPossessedByActorAnyOf { kinds: [EntityKind; 2] },
    AdjacentPlace,
}
```

Note that *every* target spec is local to the actor (its place, possessions, or an adjacent place). There is no `EntityAnywhere`. This is the locality invariant (FND-7, FND-15) enforced in the action registry.

### 4.4 `enumerate_targets`

```rust
fn enumerate_targets(
    spec: &TargetSpec,
    actor: EntityId,
    view: &dyn RuntimeBeliefView,
    actor_place_entity_cache: &mut BTreeMap<EntityId, Vec<EntityId>>,
) -> Vec<EntityId>;
```

Algorithm:

- `SpecificEntity(id)` — return `[id]` if alive.
- `ActorPlace` — return `[view.effective_place(actor)]`.
- `EntityAtActorPlace { kind }` — list entities at the actor's place filtered by `kind`; cache the per-place vector to amortize over the affordance pass.
- `EntityAtActorPlaceAnyOf { kinds }` — same, two kinds.
- `EntityDirectlyPossessedByActor*` — filter the actor's direct possessions by kind.
- `AdjacentPlace` — return the set of place entities reachable via one topology edge.

All results are sorted and deduplicated for deterministic ordering.

### 4.5 `Precondition` (post-enumeration filtering)

```rust
pub enum Precondition {
    ActorAlive,
    ActorCanControlTarget(u8),
    TargetExists(u8),
    TargetAlive(u8),
    TargetDead(u8),
    TargetIsAgent(u8),
    TargetAtActorPlace(u8),
    TargetAdjacentToActor(u8),
    TargetKind { target_index: u8, kind: EntityKind },
    TargetCommodity { target_index: u8, kind: CommodityKind },
    TargetHasWorkstationTag { target_index: u8, tag: WorkstationTag },
    TargetHasResourceSource {
        target_index: u8,
        commodity: CommodityKind,
        min_available: Quantity,
    },
    TargetHasWashBasinClean { target_index: u8, min: u16 },
    TargetNotInContainer(u8),
    TargetUnpossessed(u8),
    TargetDirectlyPossessedByActor(u8),
    TargetLacksProductionJob(u8),
    TargetHasConsumableEffect { target_index: u8, effect: ConsumableEffect },
    TargetHasWounds(u8),
    TargetUnownedOrActorControls(u8),
}
```

Preconditions not implicit in a target spec are evaluated after enumeration; the bound target tuple is retained only if every precondition holds against the belief view.

### 4.6 `Affordance` (what plan search consumes)

```rust
pub struct Affordance {
    pub def_id: ActionDefId,
    pub actor: EntityId,
    pub bound_targets: Vec<EntityId>,
    pub payload_override: Option<ActionPayload>,
    pub explanation: Option<String>,
    pub contention_status: ContentionStatus,
}
```

`contention_status` (`Available`, `Queued`, `Granted`, etc.) is computed from facility reservation state in the snapshot and lets the planner reason about scarce exclusive affordances without silently reserving them.

### 4.7 `RuntimeBeliefView` (the belief surface for affordance + planning)

```rust
pub trait RuntimeBeliefView:
    ControlBeliefView
    + EntityBeliefView
    + ProfileBeliefView
    + SpatialBeliefView
    + TemporalBeliefView
    + InventoryBeliefView
    + CombatBeliefView
    + EconomicBeliefView
    + SocialBeliefView
    + PoliticalBeliefView
    + FacilityBeliefView
{ }
```

Selected methods (one per sub-trait, illustrative):

- `effective_place(entity) -> Option<EntityId>` — current location, or transit-step destination.
- `entities_at(place) -> &[EntityId]` — used by `EntityAtActorPlace` enumeration.
- `entity_kind(entity) -> Option<EntityKind>`.
- `is_alive(entity)`, `is_dead(entity)`, `is_incapacitated(entity)`.
- `can_control(actor, entity) -> bool` — ownership / rights check; the only entrance for `TargetUnownedOrActorControls`.
- `direct_possessions(entity) -> &[EntityId]`.
- `commodity_quantity(entity, kind) -> Quantity`.
- `workstation_tag(entity) -> Option<WorkstationTag>`.
- `resource_source(entity, commodity) -> Option<ResourceSourceState>`.
- `homeostatic_needs(agent) -> &HomeostaticNeeds`.

All accessors are belief-view methods. The implementation routes co-located physical reads to authoritative state (FND-14A) and routes everything else through the agent's belief store.

---

## 5. Plan Search Pipeline

Plan search is a two-tier hierarchy: a **strategic itinerary planner** lays out the sequence of places the actor needs to visit (acquire prerequisites, then satisfy the goal); a **tactical action-sequence planner** runs best-first search at each leg using FF (Fast-Forward) delete-relaxation, landmark counting, and a dual-frontier preferred-operator scheme.

### 5.1 Top-level entry point

```rust
pub fn search_plan(
    snapshot: &PlanningSnapshot,
    goal: &GoalOffer,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    cognitive: &CognitiveProfile,
    execution_budget: &ExecutionBudget,
    recipes: &RecipeRegistry,
    blocked: &BlockerMemory,
    current_tick: Tick,
    binding_rejections: Option<&mut Vec<BindingRejection>>,
    expansion_summaries: Option<&mut Vec<SearchExpansionSummary>>,
) -> PlanSearchResult;
```

The trace-metadata variants (`search_plan_with_trace_metadata`, `search_plan_with_trace_metadata_and_opportunities`) add optional channels for richer instrumentation; they share the same algorithm.

```rust
pub enum PlanSearchResult {
    Found(Box<PlannedPlan>),
    Unsupported,
    BudgetExhausted { expansions_used: u16 },
    FrontierExhausted { expansions_used: u16 },
}
```

### 5.2 Strategic planner — location-visit itinerary

File: `crates/worldwake-ai/src/search/strategic.rs`.

```rust
pub(crate) struct StrategicPlan {
    pub steps: Vec<StrategicStep>,
}

pub(crate) struct StrategicStep {
    pub destination: EntityId,
    pub sub_goal: TacticalSubGoal,
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
    snapshot: &PlanningSnapshot,
    goal: &GoalOffer,
    execution_budget: &ExecutionBudget,
    recipes: &RecipeRegistry,
) -> Option<StrategicPlan>;
```

Pseudocode:

```
1. Early exit:
     if goal already satisfied at current state or actor is at a goal place with
        no prerequisites missing: return Some(empty plan).

2. Decompose into stages, in order:
       a. For each missing prerequisite commodity, one stage whose `places` field
          is the up-to-`max_prerequisite_locations` acquisition places, ranked
          by distance from actor.
       b. One final stage whose `places` are the goal_relevant_places().

3. Greedy local consumption: if the first-stage prerequisite is satisfiable at
   the actor's current location, consume it locally and drop that stage.

4. Frontier search over stages (Dijkstra style):
       frontier   = BinaryHeap<SearchNode { stage_idx, place, cost, steps }>
       best_cost  = BTreeMap<(stage_idx, place), min_cost>
       budget     = max(1, 2 * max_prerequisite_locations)

       while frontier nonempty and expansions < budget:
           node = frontier.pop()  // lowest cost
           if node.stage_idx == stages.len():
               return Some(StrategicPlan { steps: node.steps })
           for destination in stages[node.stage_idx].places:
               if min_travel_cost(node.place, destination) is known:
                   successor = node + StrategicStep { destination, sub_goal, ticks }
                   if successor.cost < best_cost[(stage_idx+1, destination)]:
                       frontier.push(successor)

5. If no plan and goal kind warrants exploration, emit a single
   `ExploreWithBarrier` step toward an unexplored adjacent place.
   For commodity goals with no known sources, emit a `SocialQuery(commodity)`
   step toward a known information source.

6. If still nothing: return None.
```

Supporting subroutines: `goal_places(goal, state, recipes)`, `missing_commodities(goal, state, recipes, actor_place, goal_places)`, `acquisition_places_for_commodity(state, snapshot, actor, actor_place, commodity, per_stage_limit)`. All consult the snapshot (§5.10), not authoritative world state.

### 5.3 Tactical planner — action sequence search

The tactical planner operates on `SearchNode<'snapshot>`:

```rust
struct SearchNode<'snapshot> {
    state: PlanningState<'snapshot>,
    steps: SharedVec<PlannedStep>,
    total_estimated_ticks: u32,
    search_cost: u32,                  // g(n) — accumulated action cost
    tactical_barrier_reached: bool,    // turns off tactical filter once a barrier
                                       // (e.g. prerequisite collected) is satisfied
    heuristic_ticks: u32,              // h(n) — max(spatial, ff_h, landmark_h)
}
```

Node ordering (tiebreaking cascade):

```rust
fn compare_search_nodes(left: &SearchNode<'_>, right: &SearchNode<'_>) -> Ordering {
    let lf = left.search_cost.saturating_add(left.heuristic_ticks);
    let rf = right.search_cost.saturating_add(right.heuristic_ticks);
    lf.cmp(&rf)                                             // 1. f = g+h
        .then_with(|| left.search_cost.cmp(&right.search_cost))           // 2. g
        .then_with(|| left.total_estimated_ticks.cmp(&right.total_estimated_ticks))
        .then_with(|| left.steps.len().cmp(&right.steps.len()))           // 4. depth
        .then_with(|| left.steps.as_slice().cmp(right.steps.as_slice()))  // 5. lex
}
```

Main loop pseudocode:

```
relevant_defs = candidates::relevant_action_defs(goal, semantics_table)
strategic     = strategic::plan(snapshot, goal, execution_budget, recipes)
tactical_goal = TacticalGoal::from_strategic_step(goal, strategic.first(), snapshot)

if strategic.first() is ExploreWithBarrier and tactical_goal is None:
    return FrontierExhausted { expansions_used: 0 }

frontier        = DualFrontier::new(execution_budget.preferred_operator_boost())
frontier.push_regular(root_node)
landmark_set    = LandmarkSet::empty()
expansions      = 0
effective_budget = cognitive.max_node_expansions
best_barrier_plan = None

while frontier nonempty:
    node = frontier.pop()
    if goal.kind.is_satisfied(&node.state):
        return Found(PlannedPlan { steps: node.steps, terminal_kind: GoalSatisfied })

    if node.steps.len() >= cognitive.max_plan_depth: continue
    if expansions >= effective_budget:
        if best_barrier_plan.is_some(): return Found(best_barrier_plan)
        return BudgetExhausted { expansions_used }

    expansions += 1

    // 1. Compute active tactical goal (None if barrier reached or already satisfied)
    active_tactical_goal = if node.tactical_barrier_reached { None }
                          else { tactical_goal.filter(barrier_not_satisfied) }

    // 2. Candidate generation
    candidates = search_candidates_with_expansion_trace(... node, relevant_defs ...)
    if tactical_goal == SocialQuery { destination, commodity } and at destination:
        candidates += AskWitness affordances

    // 3. Filtering: commodity relevance, tactical filter, travel-away pruning
    apply_commodity_relevance_filter(...)
    apply_tactical_candidate_filter(...)
    if at current_place: prune_travel_away_from_goal(...)
    cap_travel_candidates(... cognitive.max_travel_candidates_per_expansion)

    // 4. Successor construction (mark terminal_kind for goal-satisfying / barrier steps)
    terminal_successors = []
    nonterminal         = []
    for cand in candidates:
        let (terminal_kind, succ, _) = build_successor_detailed(... cand ...)
        if terminal_kind.is_some(): terminal_successors.push((terminal_kind, succ))
        else: nonterminal.push((None, succ, /*preferred*/ false))

    // 5. Process terminal successors first
    for (k, succ) in sort_by_f_score(terminal_successors):
        if k == GoalSatisfied:
            return Found(PlannedPlan { steps: succ.steps, terminal_kind: GoalSatisfied })
        if k == ProgressBarrier and best_barrier_plan is None:
            best_barrier_plan = PlannedPlan { steps: succ.steps, terminal_kind: ProgressBarrier }

    // 6. Landmark extraction on first qualifying expansion
    if landmark_set is empty and active_tactical_goal is Some and cognitive.landmark_extraction_depth > 0:
        landmark_set = extract_landmarks(
            initial_facts = planning_facts_from_state(node.state),
            goal_facts    = active_tactical_goal.goal_facts(goal, node.state, recipes),
            operators     = planning_operator_from_transition(node.state, succ.state) for each successor,
            max_depth     = cognitive.landmark_extraction_depth,
        )

    // 7. FF heuristic application
    ff_result = apply_ff_heuristic_to_successors(... nonterminal ...)
                                          (skipped when cognitive.use_ff_heuristic is false)

    // 8. Landmark-based preferred-operator marking if no FF result
    if ff_result is None and landmark_set nonempty:
        preferred_indices = preferred_operators(landmark_set, current_facts,
                                                successor_candidates, successor_operators)
        for i in preferred_indices: nonterminal[i].preferred = true

    // 9. Sort + beam truncation
    sort_by_f_score(nonterminal)
    nonterminal.truncate(execution_budget.beam_width())

    // 10. Push to dual frontier; trigger boost if any preferred
    if nonterminal.any(|(_, _, preferred)| preferred): frontier.trigger_boost()
    for (_, succ, preferred) in nonterminal:
        if preferred: frontier.push_preferred(succ) else: frontier.push_regular(succ)

if best_barrier_plan is Some: return Found(best_barrier_plan)
return FrontierExhausted { expansions_used }
```

### 5.4 Landmark extraction (delete-relaxation)

File: `crates/worldwake-ai/src/search/landmarks.rs`.

```rust
pub(super) enum PlanningFact {
    AtPlace(EntityId),
    HasCommodity(CommodityKind),
    HasEntity(EntityId),
    FacilityAvailable(EntityId),
    EntityPresent(EntityId),
    NeedSatisfied(HomeostaticNeedId),
}

pub(super) struct PlanningOperator {
    pub(super) preconditions: BTreeSet<PlanningFact>,
    pub(super) add_effects:   BTreeSet<PlanningFact>,
    pub(super) del_effects:   BTreeSet<PlanningFact>,
}

pub(super) struct LandmarkSet {
    pub(super) landmarks: BTreeSet<PlanningFact>,
    pub(super) orderings: Vec<(PlanningFact, PlanningFact)>,
}

pub(super) fn extract_landmarks(
    initial_facts: &BTreeSet<PlanningFact>,
    goal_facts:    &BTreeSet<PlanningFact>,
    operators:     &[PlanningOperator],
    max_depth:     u8,
) -> LandmarkSet;
```

Algorithm:

```
landmarks = goal_facts
orderings = {}
queue     = VecDeque<(fact, depth=0)> for each goal_fact

while queue nonempty:
    (fact, depth) = queue.pop_front()
    if depth >= max_depth or fact in initial_facts: continue
    achievers = [op for op in operators if fact in op.add_effects]
    if achievers empty: continue
    shared_precs = ∩ over achievers of (op.preconditions)
    for prec in shared_precs where prec != fact:
        orderings.insert((prec, fact))
        if landmarks.insert(prec): queue.push_back((prec, depth+1))

return LandmarkSet { landmarks, orderings }
```

The key insight: if all operators that add a fact share a common precondition, that precondition is a necessary landmark. Depth cap prevents infinite chains on cyclic operator sets (`max_depth` defaults to `cognitive.landmark_extraction_depth`, default 4).

### 5.5 FF (Fast-Forward) relaxed-plan heuristic

```rust
pub(super) struct RelaxedPlanResult {
    pub(super) h_ff: u32,
    pub(super) helpful_action_indices: BTreeSet<usize>,
}

pub(super) fn compute_ff_heuristic(
    initial_facts: &BTreeSet<PlanningFact>,
    goal_facts:    &BTreeSet<PlanningFact>,
    operators:     &[PlanningOperator],
) -> Option<RelaxedPlanResult>;
```

Algorithm:

```
reachable = initial_facts
first_achiever : Map<Fact, (layer, op_index)>

for layer in 0..operators.len():
    new = {}
    for (op_idx, op) in operators:
        if op.preconditions ⊆ reachable:
            for f in op.add_effects:
                if f not in reachable and f not in new:
                    new.insert(f)
                    first_achiever[f] = (layer, op_idx)
    if new is empty: return None
    reachable.extend(new)
    if goal_facts ⊆ reachable: break

if goal_facts ⊄ reachable: return None

selected = {}
helpful  = {}
for f in goal_facts:
    select_fact(f, ...)  // recursive: select first achiever and its preconditions
                          // mark layer-0 operators as helpful

return Some(RelaxedPlanResult {
    h_ff = selected.len(),
    helpful_action_indices = helpful,
})
```

Delete effects are ignored throughout — that's the relaxation. Helpful actions (layer-0 operators in the extracted relaxed plan) are used to mark successors as preferred and trigger the dual-frontier boost.

### 5.6 Heuristic combination

```
spatial_h  = snapshot.min_perceived_travel_cost_to_any(actor_place, goal_places)
ff_h       = ff_result.h_ff if Some else 0
landmark_h = unsatisfied landmarks whose predecessor landmarks are achieved
node.heuristic_ticks = max(spatial_h, ff_h, landmark_h)
```

Using max keeps the combined h admissible relative to any single source while ensuring no information is lost when one estimator is uninformative.

### 5.7 Dual frontier with preferred-operator boost

File: `crates/worldwake-ai/src/search/frontier.rs`.

```rust
pub(super) struct DualFrontier<'snapshot> {
    regular:                BinaryHeap<FrontierEntry<'snapshot>>,
    preferred:              BinaryHeap<FrontierEntry<'snapshot>>,
    boost_remaining:        u8,
    preferred_operator_boost: u8,   // from ExecutionBudget (default 2)
    use_preferred_next:     bool,   // alternation flag
}

pub(super) fn pop(&mut self) -> Option<SearchNode<'snapshot>>;
pub(super) fn trigger_boost(&mut self);
```

Pop semantics:

```
prefer_preferred = boost_remaining > 0 OR use_preferred_next
popped = if prefer_preferred: preferred.pop().or_else(regular.pop)
         else:                regular.pop().or_else(preferred.pop)
if popped is Some:
    if prefer_preferred and boost_remaining > 0: boost_remaining -= 1
    use_preferred_next = !use_preferred_next
```

`trigger_boost()` is called whenever any successor is marked preferred and resets `boost_remaining = preferred_operator_boost`. The effect is that the preferred queue gets disproportionate pop priority for the next `preferred_operator_boost` ticks, biasing search toward FF-helpful / landmark-supporting operators while still maintaining completeness (preferred always falls through to regular if empty).

### 5.8 Beam truncation and expansion budget

```rust
let retained_len = successors.len().min(usize::from(execution_budget.beam_width()));
successors.truncate(retained_len);
```

Each expansion's surviving non-terminal successors are truncated to `beam_width` (default 8). Plan depth is bounded by `cognitive.max_plan_depth` (default 8). The global expansion budget is `cognitive.max_node_expansions` (default 224). On budget exhaustion, the planner returns the best `ProgressBarrier` plan (a plan that lands on an explicit tactical barrier even if it does not satisfy the goal) when available; otherwise `BudgetExhausted`.

### 5.9 Per-expansion candidate filters

Applied in order:

1. `apply_commodity_relevance_filter` — drop candidates whose effects consume or produce a commodity unrelated to the tactical goal.
2. `apply_tactical_candidate_filter` — when a `TacticalSubGoal::AcquirePrerequisite(commodity)` is active, exclude non-acquisition actions until the barrier is crossed; conversely exclude travel-away when at the destination.
3. `prune_travel_away_from_goal` — drop travel candidates whose destination increases the perceived travel cost to the goal places, unless detour is opportunity-justified within `cognitive.detour_budget_permille` (default 150 = 0.15).
4. `cap_travel_candidates` — optional cap from `cognitive.max_travel_candidates_per_expansion` (default `None` = uncapped).

### 5.10 `PlanningSnapshot` (frozen belief surface)

File: `crates/worldwake-ai/src/planning_snapshot.rs`.

```rust
pub struct PlanningSnapshot {
    pub(crate) actor: EntityId,
    pub(crate) current_tick: Tick,
    pub(crate) actor_belief_store: AgentBeliefStore,
    pub(crate) entities: BTreeMap<EntityId, SnapshotEntity>,
    pub(crate) places:   BTreeMap<EntityId, SnapshotPlace>,
    pub(crate) blocked_facility_uses: BTreeSet<(EntityId, ActionDefId)>,
    pub(crate) actor_known_entity_beliefs:       BTreeMap<EntityId, BelievedEntityState>,
    pub(crate) actor_known_social_observations:  Vec<SocialObservation>,
    pub(crate) actor_known_institutional_beliefs: Vec<BelievedInstitutionalClaim>,
    pub(crate) actor_told_beliefs:               BTreeMap<TellMemoryKey, ToldBeliefMemory>,
    pub(crate) actor_bandit_factions:            Vec<EntityId>,
    pub(crate) actor_active_violation_records:   Vec<RecordedViolation>,
    pub(crate) actor_contested_offices:          Vec<EntityId>,
    pub(crate) actor_loyalties:                  BTreeMap<EntityId, Permille>,
    pub(crate) actor_office_holder_beliefs:      BTreeMap<EntityId, SupportBeliefRead>,
    pub(crate) actor_force_controller_beliefs:   BTreeMap<EntityId, ForceControllerBeliefRead>,
    pub(crate) office_certain_support_declarations: BTreeMap<EntityId, Vec<(EntityId, EntityId)>>,
    pub(crate) office_support_declaration_beliefs:  BTreeMap<EntityId, OfficeSupportBeliefReads>,
    pub(crate) actor_confidence_policy:          BeliefConfidencePolicy,
    pub(crate) actor_claim_confidence_threshold: Permille,
    pub(crate) actor_tell_profile:               Option<TellProfile>,
    pub(crate) actor_epistemic_profile:          Option<EpistemicDispositionProfile>,
    pub(crate) actor_consultation_speed_factor:  Option<Permille>,
    pub(crate) actor_expectation_store:          Option<ExpectationStore>,
    pub(crate) actor_last_seen_memory:           Option<LastSeenMemory>,
    pub(crate) actor_bandit_flee_thresholds:     BTreeMap<EntityId, Permille>,
    pub(crate) actor_bandit_establishment_ticks: BTreeMap<EntityId, NonZeroU32>,
    shortest_travel_ticks:  DistanceMatrix,   // all-pairs, Floyd-Warshall
    perceived_travel_costs: DistanceMatrix,   // adjusted for threat / confidence
}
```

The snapshot is constructed once per planning cycle from the agent's belief store. After that point, plan search never touches the live world. `PlanningState` (below) wraps the snapshot with cheap copy-on-write override maps so each search node can simulate its own hypothetical state without mutating the snapshot or any other node.

### 5.11 `PlanningState` (mutable simulation layer)

File: `crates/worldwake-ai/src/planning_state.rs`.

```rust
pub struct PlanningState<'snapshot> {
    snapshot: &'snapshot PlanningSnapshot,
    entity_place_overrides:        SharedMap<PlanningEntityRef, Option<EntityId>>,
    bandit_camp_faction_overrides: SharedMap<EntityId, Option<EntityId>>,
    direct_container_overrides:    SharedMap<PlanningEntityRef, Option<PlanningEntityRef>>,
    direct_possessor_overrides:    SharedMap<PlanningEntityRef, Option<PlanningEntityRef>>,
    resource_quantity_overrides:   SharedMap<EntityId, Quantity>,
    commodity_quantity_overrides:  SharedMap<(PlanningEntityRef, CommodityKind), Quantity>,
    reservation_shadows:           SharedMap<EntityId, Vec<TickRange>>,
    removed_entities:              SharedSet<PlanningEntityRef>,
    sale_listing_overrides:        SharedMap<PlanningEntityRef, bool>,
    sale_seller_overrides:         SharedMap<PlanningEntityRef, Option<EntityId>>,
    needs_overrides:               SharedMap<EntityId, HomeostaticNeeds>,
    pain_overrides:                SharedMap<EntityId, Permille>,
    support_declaration_overrides: SharedMap<(EntityId, EntityId), Option<EntityId>>,
    office_holder_belief_overrides: SharedMap<EntityId, InstitutionalBeliefRead<Option<EntityId>>>,
    force_controller_belief_overrides: SharedMap<EntityId, InstitutionalBeliefRead<(Option<EntityId>, bool)>>,
    support_declaration_belief_overrides: SharedMap<(EntityId, EntityId), InstitutionalBeliefRead<Option<EntityId>>>,
    facility_queue_membership_overrides: SharedMap<EntityId, Option<HypotheticalQueueJoin>>,
    facility_grant_overrides:      SharedMap<EntityId, Option<ContentionGrant>>,
    hypothetical_registry:         SharedMap<HypotheticalEntityId, HypotheticalEntityMeta>,
    entities_at_cache:    Rc<RefCell<BTreeMap<EntityId, Vec<EntityId>>>>,
    effective_place_cache: Rc<RefCell<BTreeMap<PlanningEntityRef, Option<EntityId>>>>,
    next_hypothetical_id: u32,
}
```

`SharedMap`/`SharedSet` are persistent (`Rc<...>`-wrapped) maps with structural sharing, so cloning a `PlanningState` (which happens at every successor) is O(1) plus a per-mutation copy-on-write. The state can simulate moves, inventory changes, queue joins/grants, and even instantiate hypothetical entities (e.g., the bar of bread that a `Craft` action would produce) without ever touching live world data.

---

## 6. Plan Revalidation & Execution

### 6.1 What this stage does

Between "plan found" and "action executes" the planner re-checks every step against the up-to-date belief view. Beliefs can drift between the snapshot and dispatch: another agent might have taken the target item, a route may have been newly blocked by a hostile, a perceived seller may have closed shop. Revalidation is the gate that catches stale plans before they reach the scheduler.

### 6.2 `classify_revalidation`

File: `crates/worldwake-ai/src/plan_revalidation.rs`.

```rust
pub enum RevalidationOutcome {
    Valid,
    Invalidated {
        reason: PlanInvalidationReason,
        expectation_kind: Option<ExpectationKindTag>,
        mismatch_detail: Option<MismatchDetail>,
    },
}

pub fn classify_revalidation(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    step_index: u16,
    step: &PlannedStep,
    bindings: &MaterializationBindings,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> RevalidationOutcome;
```

Four sequential sub-checks:

1. **Guard validation.** Each `PlannedStep` may carry a `PlanGuard` with `required_facts` (e.g., target presence, route knowledge, resource availability) and `invalidators` (e.g., `BeliefStatusChange { claim }`). The effective minimum confidence is `min(step_guard.min_confidence, cognitive.guard_min_confidence_ceiling)` (default ceiling = 1000 = 1.0). Stale-plan detection lives here:
   ```rust
   fn belief_status_changed(
       status: BeliefStatus,
       confidence: Permille,
       effective_min_confidence: Permille,
   ) -> bool {
       matches!(status, BeliefStatus::Stale | BeliefStatus::Contradicted)
           || confidence < effective_min_confidence
   }
   ```
2. **Affordance matching** (exact-target steps): rebuild the affordance from current belief state and confirm slot-for-slot identity:
   ```rust
   pub fn requested_affordance_matches(
       affordance: &Affordance,
       def:        &ActionDef,
       handler:    &ActionHandler,
       actor:      EntityId,
       targets:    &[EntityId],
       payload_override: Option<&ActionPayload>,
       view:       &dyn RuntimeBeliefView,
   ) -> bool;
   ```
   When a `payload_override` is present and both `def.payload` and the live affordance payload are `None`, this delegates to the handler's `payload_override_is_valid` closure — that is the mechanism by which planner-synthesized payloads survive revalidation.
3. **Best-effort payload override revalidation.** When `step.payload_override` is set, `def.payload == ActionPayload::None`, all actor constraints and preconditions pass, and the handler has no built-in affordance payloads, the planner calls `handler.payload_override_is_valid(def, actor, targets, payload_override, view)` to confirm the synthesized payload is still legal under current beliefs.
4. **Exact-target synthetic affordance.** For `ActionDef`s whose `targets` are all `TargetSpec::SpecificEntity`, the planner constructs a synthetic `Affordance` with the step's materialized bindings and feeds it through `requested_affordance_matches`.

Any failure produces `Invalidated { reason, expectation_kind, mismatch_detail }` (`reason` is typically `PlanInvalidationReason::ExpectationMismatch { step_index }` from a guard failure, or `PlanInvalidationReason::TargetGone` from an affordance failure).

### 6.3 Dispatch path

File: `crates/worldwake-ai/src/agent_tick/execution.rs`.

```rust
pub(super) fn enqueue_valid_step_or_handle_failure(
    ctx:                            &mut AgentTickContext<'_>,
    runtime:                        &mut AgentDecisionRuntime,
    active_goal:                    Option<GoalKey>,
    jc:                             &mut Option<IntentionFrame>,
    blocked_memory:                 &mut BlockerMemory,
    discrepancy_memory:             &mut DiscrepancyMemory,
    facility_intents:               &mut ContentionIntents,
    agent:                          EntityId,
    tick:                           Tick,
    original_blocked:               &BlockerMemory,
    original_discrepancy_memory:    &DiscrepancyMemory,
    original_violation_memory:      &ViolationMemory,
    violation_memory:               &ViolationMemory,
    original_repair_memory:         &RepairMemory,
    repair_memory:                  &mut RepairMemory,
    memory_capacity:                MemoryCapacityProfile,
    original_learned_opportunity_memory: &LearnedOpportunityMemory,
    learned_opportunity_memory:     &LearnedOpportunityMemory,
    step:                           &PlannedStep,
    valid:                          bool,
    mut repair_attempt_traces:      Option<&mut Vec<RepairAttemptTrace>>,
) -> Result<(), TickInputError>;
```

When `valid == true`, the step is enqueued to the scheduler with its bindings and `payload_override`. The step's `PlanGuard` (if present) is attached as `plan_step_expectations` so mid-tick expectation violation detection can fire. When `valid == false`, the function first attempts a recoverable-travel-step handler (e.g., dynamic routing around a newly blocked corridor), then attempts localized plan repair via `attempt_local_repair_for_invalidated_step` (budget = `cognitive.repair_budget_fraction * cognitive.max_node_expansions`, default 0.25 × 224 ≈ 56 nodes). If repair succeeds, the repaired suffix is applied and execution proceeds without a full replan. If everything fails, the function falls through into Stage 7 failure handling.

### 6.4 Interrupt evaluation

Before the next step is enqueued the agent_tick code checks whether the currently active action should be interrupted:

```rust
pub(super) fn handle_active_action_phase(
    ctx:                  &mut AgentTickContext<'_>,
    runtime:              &mut AgentDecisionRuntime,
    active_goal:          &mut Option<AgendaEntry>,
    jc:                   &mut Option<IntentionFrame>,
    facility_intents:     &mut ContentionIntents,
    blocked_memory:       &mut BlockerMemory,
    discrepancy_memory:   &mut DiscrepancyMemory,
    agent:                EntityId,
    ranked_candidates:    &OrderedRanked<'_>,
    active_action:        &ActionInstance,
    default_switch_margin: Permille,
    frame_switch_margin:  Permille,
    tick:                 Tick,
    action_defs:          &ActionDefRegistry,
    action_handlers:      &ActionHandlerRegistry,
    decision_context:     DecisionContext,
) -> Result<InterruptDecision, TickInputError>;
```

The check consults `ActionDef.interruptibility`:

- `NonInterruptible` — bypass.
- `FreelyInterruptible` / `InterruptibleWithPenalty` — compare candidate utilities against the active action's residual utility. A rival must beat the current goal by at least `switch_margin` (default 100‰) and a rival plan must beat the current plan by `planning_switch_margin` (default 150‰) before `InterruptDecision::InterruptForReplan` fires, at which point the scheduler issues `interrupt_active_action(InterruptReason::Reprioritized)` and the planning phase restarts.

### 6.5 Frame relation classification

```rust
pub(super) fn update_frame_for_adopted_plan(
    frame:         Option<&IntentionFrame>,
    selected_plan: &PlannedPlan,
    tick:          Tick,
    runtime:       &mut AgentDecisionRuntime,
) -> Option<IntentionFrame>;
```

Classifies the relationship between an active frame (committed intention with assumptions) and a new plan as `SuspendsFrame`, `ContinuesFrame`, or `ClearsFrame`. Frame state is the unit that survives plan-step revalidation: a continued frame carries assumptions, guards, and expectations forward; a suspended frame is shelved for possible resumption; a cleared frame is dropped and the `runtime.last_frame_clear_reason` is recorded.

---

## 7. Replanning (Failure Handling)

### 7.1 Entry point

File: `crates/worldwake-ai/src/failure_handling.rs`.

```rust
pub struct PlanFailureContext<'a> {
    pub view:               &'a dyn RuntimeBeliefView,
    pub agent:              EntityId,
    pub goal_key:           GoalKey,
    pub failed_step:        &'a PlannedStep,
    pub execution_failure:  Option<ExecutionFailure<'a>>,
    pub belief_discrepancy: Option<Discrepancy>,
    pub current_tick:       Tick,
}

pub enum FailureClassification {
    Blocker(BlockingFact),
    Discrepancy(Discrepancy),
}

pub fn handle_plan_failure(
    context:           &PlanFailureContext<'_>,
    runtime:           &mut AgentDecisionRuntime,
    jc:                &mut Option<IntentionFrame>,
    blocked_memory:    &mut BlockerMemory,
    discrepancy_memory: &mut DiscrepancyMemory,
    facility_intents:  &mut ContentionIntents,
    cognitive:         &CognitiveProfile,
) -> FailureClassification;
```

### 7.2 Classification cascade

```rust
pub(crate) fn classify_discrepancy(
    view:               &dyn RuntimeBeliefView,
    agent:              EntityId,
    goal_key:           &GoalKey,
    step:               &PlannedStep,
    execution_failure:  Option<ExecutionFailure<'_>>,
    belief_discrepancy: Option<Discrepancy>,
) -> FailureClassification;
```

In order:

1. Explicit `belief_discrepancy` → return it.
2. Step's primary target no longer believed-to-exist → `BlockingFact::TargetGone`.
3. Action-domain dispatch:
   - `Travel` → `no_known_path` check (returns `BlockingFact::NoKnownPath` if true).
   - `Trade` / `StaffMarket` / `StockManagement` → `classify_trade_failure` inspects payload, counterparty stock, actor budget, competing sellers.
   - `Harvest` / `Craft` → `classify_production_failure` inspects input availability and workstation occupation.
   - `Consume` / `Heal` → `classify_input_failure` inspects commodity availability.
   - `Attack` / `Defend` → `combat_too_risky` (danger assessment).
4. Ambient danger check (`danger_too_high(view, agent)`) → `BlockingFact::DangerTooHigh`.
5. Goal-target commodity belief contradicted locally → `Discrepancy::BeliefContradicted`.
6. Map scheduler-level `ExecutionFailure` reasons to specific `BlockingFact` / `Discrepancy` via `map_execution_failure`.
7. Fallback → `Discrepancy::ImproperPlanningState`.

### 7.3 Recording the failure

```rust
pub(crate) fn record_failure_classification(
    context:           &PlanFailureContext<'_>,
    classification:    FailureClassification,
    runtime:           &mut AgentDecisionRuntime,
    blocked_memory:    &mut BlockerMemory,
    discrepancy_memory: &mut DiscrepancyMemory,
    cognitive:         &CognitiveProfile,
) -> FailureClassification;
```

For a `Blocker(blocking_fact)`:

- Build `BlockerKey { goal_key, place, target, action_def }` from the failed step.
- `expires_tick = current_tick + blocking_fact_ttl(blocking_fact, cognitive)`. The TTL is one of:
  - `transient_block_ticks` (20) for transient ones.
  - `structural_block_ticks` (200) for structural ones.
- Compute `BlockerClearingCondition` (e.g., `CommodityAvailabilityChanged`, `EntityReappeared`, `PathDiscovered`) and a `baseline_snapshot` so the blocker can be cleared early when its baseline observation flips.
- Insert into `blocked_memory.intents`.

For a `Discrepancy(kind)`:

- Same `BlockerKey`, TTL chosen from the `*_backoff_ticks` family on `CognitiveProfile` (e.g., `contradicted_belief_backoff_ticks = 60`, `counterparty_refusal_backoff_ticks = 40`, `route_unknown_backoff_ticks = 200`).
- Compute `DiscrepancyClearing` (belief-status flip, expected observation).
- Insert into `discrepancy_memory.entries`.

Both paths set `runtime.dirty.insert(DirtySet::REPLAN_SIGNAL)` and clear the runtime's current plan and frame, forcing re-entry into the full pipeline on the next tick.

### 7.4 Belief updates on failure

Belief updates happen indirectly. The failed action itself emits events (target observed absent, seller refused, route blocked) that flow through perception into the agent's belief store. The discrepancy/blocker record gates *re-emission* until either:

- the TTL expires, or
- the clearing condition's baseline observation flips (e.g., the agent witnesses the path being clear again).

No replan-limit counters exist — backoff is the only throttle. This is intentional: an agent who keeps trying because they have no other moves is allowed to keep trying after the timer.

---

## 8. Cognitive Parameters

All planner tuning lives on two per-agent components: `CognitiveProfile` (decision-cycle behavior, search bounds, backoff TTLs, memory budgets) and `ExecutionBudget` (search-frontier shape). Both are inserted on every agent at creation with `Default::default()` and may be overridden per-agent via scenarios (or live mutation).

### 8.1 `CognitiveProfile` defaults

File: `crates/worldwake-core/src/cognitive_profile.rs`. Defaults are quoted verbatim from `impl Default` and the `default_*` `const fn`s.

| Field | Type | Default | Effect |
|---|---|---|---|
| `max_candidates_to_plan` | `u8` | `2` | Number of top-ranked goals that proceed to plan search per tick. |
| `max_candidates_per_expansion` | `u16` | `200` | Max action successors expanded at a single search node. |
| `max_plan_depth` | `u8` | `8` | Hard cap on a plan's step count. |
| `max_travel_candidates_per_expansion` | `Option<u16>` | `None` | Optional cap on travel-action successors per expansion. |
| `snapshot_travel_horizon` | `u8` | `6` | Place-graph hops included in the planning snapshot. |
| `max_node_expansions` | `u16` | `224` | Total node expansions allowed in one plan search. |
| `switch_margin` | `Permille` | `100` | Goal-switch utility margin during execution. |
| `planning_switch_margin` | `Permille` | `150` | Plan-switch utility margin. |
| `transient_block_ticks` | `u32` | `20` | TTL on transient blockers (e.g., temporary contention). |
| `structural_block_ticks` | `u32` | `200` | TTL on structural blockers (e.g., no known seller). |
| `stale_belief_backoff_ticks` | `u32` | `30` | TTL for stale-belief discrepancies. |
| `contradicted_belief_backoff_ticks` | `u32` | `60` | TTL for contradicted-belief discrepancies. |
| `improper_state_backoff_ticks` | `u32` | `2` | TTL for planner-state discrepancies. |
| `missing_observation_backoff_ticks` | `u32` | `20` | TTL for missing-observation discrepancies. |
| `no_legal_binding_backoff_ticks` | `u32` | `120` | TTL for no-legal-binding discrepancies. |
| `counterparty_refusal_backoff_ticks` | `u32` | `40` | TTL for counterparty-refusal discrepancies. |
| `route_unknown_backoff_ticks` | `u32` | `200` | TTL for route-unknown discrepancies. |
| `search_exhaustion_backoff_ticks` | `u32` | `100` | TTL for search-budget-exhaustion discrepancies. |
| `partial_drift_backoff_ticks` | `u32` | `4` | TTL for partial-execution-drift discrepancies. |
| `expectation_tolerance_ticks` | `u32` | `2` | Grace window before plan-step expectations are overdue. |
| `guard_min_confidence_ceiling` | `Permille` | `1000` | Per-agent cap on guard `min_confidence`. |
| `repair_memory_ticks` | `u32` | `120` | TTL on successful-repair memory. |
| `learned_opportunity_memory_ticks` | `u32` | `60` | TTL on learned-opportunity memory. |
| `survey_memory_capacity` | `usize` | `24` | Max survey records retained. |
| `survey_memory_retention_ticks` | `u64` | `300` | TTL on a survey record. |
| `initial_cooldown_ticks` | `u32` | `4` | Base post-failure cooldown. |
| `max_cooldown_ticks` | `u32` | `64` | Cap on exponential post-failure cooldown. |
| `landmark_extraction_depth` | `u8` | `4` | Backtrack depth of landmark extraction (0 disables landmarks). |
| `use_ff_heuristic` | `bool` | `true` | Whether to compute the FF relaxed-plan heuristic per expansion. |
| `decision_history_alternatives` | `u8` | `5` | Max rejected alternatives logged in decision-history events. |
| `detour_budget_permille` | `Permille` | `150` | Salience budget allowing opportunity-aware travel detours (0.15). |
| `compile_opportunity_cap` | `u16` | `16` | Soft cap on compiled opportunities per cycle. |
| `slot_weights` | `PortfolioSlotWeights` | `survival=1000, commitment=900, economic=700` | Portfolio slot weighting. |
| `repair_budget_fraction` | `Permille` | `250` | Fraction of `max_node_expansions` granted to localized plan repair (0.25 ≈ 56 nodes). |
| `causal_links_per_step_cap` | `u8` | `8` | Cap on causal links retained per guarded plan step. |

### 8.2 `ExecutionBudget` defaults

File: `crates/worldwake-core/src/execution_budget.rs`.

```rust
pub struct ExecutionBudget {
    beam_width:               u8,
    max_prerequisite_locations: u8,
    preferred_operator_boost: u8,
}

impl Default for ExecutionBudget {
    fn default() -> Self { Self::new(8, 3, 2) }
}
```

| Field | Type | Default | Effect |
|---|---|---|---|
| `beam_width` | `u8` | `8` | Max non-terminal successors retained per expansion (>=1, hard checked at construction). |
| `max_prerequisite_locations` | `u8` | `3` | Per-stage cap on acquisition places included in strategic search (>=1). |
| `preferred_operator_boost` | `u8` | `2` | Consecutive preferred-queue pops after each `trigger_boost`. `0` = alternate 1:1 forever. |

Construction is validated by `try_new`, which rejects `beam_width == 0` and `max_prerequisite_locations == 0`. Deserialization runs through `try_new`, so on-disk scenarios cannot bypass those invariants.

### 8.3 How agent diversity (FND-22) shows up

Both components are per-agent. Scenarios can override any field on any agent. Some examples of how this propagates into observable behavior:

- An agent with `landmark_extraction_depth = 0` and `use_ff_heuristic = false` searches purely on spatial heuristic — cheap but myopic, biases toward "go where the goal place is."
- An agent with `max_plan_depth = 12` can string together longer crafting chains than the default-8 agent at the same `max_node_expansions`.
- An agent with `beam_width = 12` explores wider per expansion but pays more per node; `max_prerequisite_locations = 5` widens strategic options at the cost of a larger frontier search.
- `guard_min_confidence_ceiling = 700` will let an agent dispatch on weaker beliefs than the default-1000 agent — modeling a brash or low-vigilance personality.
- TTL fields (`*_backoff_ticks`) let a "stubborn" agent retry faster after refusal than a "cautious" agent.
- `detour_budget_permille` controls how much the agent will detour for opportunistic harvesting / trading on the way to its goal.

These knobs are designed to be set per role / per personality at scenario authoring time; the engine treats them as opaque tuning surfaces (no special-case code depends on default values).

---

## 9. FOUNDATIONS Alignment

Each subsection embeds the principle text verbatim, then describes alignment.

### FND-1 — Maximal Emergence Through Local Causality

> Worldwake exists to produce emergent behavior through interacting systems and agents, never through authored sequences, hidden quest logic, or one-off story triggers. An event is valid only if it arose from prior world state, agent belief, institutional rule, or natural process already present in the simulation. Authoring beasts, hunger, roads, caravans, towns, offices, and bounty procedures is correct. Authoring "a beast attack happens so adventurers have content" is forbidden. **Test**: If the only honest explanation for an event is "the game decided something interesting should happen now," the design violates this principle.

**Alignment.** The planner does not "decide outcomes." It synthesizes a plan from `ActionDef`s, every step of which is a lawful affordance available to *any* agent. There is no quest pipeline, no story trigger, and no `GoalKind` variant whose satisfaction bypasses normal world causality. Even `PostBounty` requires the issuer to have lawful authority, a reward source, and visibility constraints — emitted by the same `emit_*` family that emits `Sleep`.

**Tension.** None at the planner level. The variety of `emit_*` gates is bounded by ~17 functions; the system relies on combinatorics over a small kind set to produce emergent chains.

### FND-3 — Concrete State Over Abstract Scores

> Prefer modeling the thing itself over a score that represents it. Danger should come from actual threats on routes, not `danger_score`. Scarcity should come from inventories, queues, failed purchases, and unmet needs, not `scarcity_score`. A price spike should emerge from actual stock, seller beliefs, buyer pressure, and substitute availability, not from `if stock < 50% then price *= 1.5`. Abstract summaries are allowed only as derived views or caches. They may never become the source of truth. **Test**: If a system relies on a number that cannot be traced back to concrete entities, relations, or events, the design violates this principle.

**Alignment.** Goal kinds carry concrete arguments (`commodity: CommodityKind`, `target: EntityId`, `office: EntityId`). Suppression filters consult concrete pressure classes derived from concrete needs (`HomeostaticNeeds`), wounds, and visible hostiles — not a `danger_score`. Plan-search heuristics (`h_ff`, landmark count, perceived travel cost) are derived numeric *summaries* but are computed each time from the snapshot's concrete state and never written back as truth.

**Tension.** The `motive_score: u32` and `Permille` scalars (switch margins, detour budget) are aggregate numbers. They are legal as long as the *inputs* are concrete (they are — `motive_score` aggregates per-source contributions, each of which traces back to a believed entity / record). A future audit risk: if a new `MotiveSourceRef` is added that bottoms out in a constant rather than a believed fact, that would violate FND-3.

### FND-7 — Locality of Motion, Interaction, and Communication

> All physical interaction requires co-location or explicit range. All communication requires co-location or a physical carrier moving through the place graph: a witness, rumor chain, letter, notice, messenger, ledger, smoke plume, tracks, corpse, or other evidence carrier. Agents, institutions, and planners may not query global truth on behalf of a character. A magistrate cannot know a caravan was attacked until some information carrier reaches them. A merchant cannot know a road is unsafe until they perceive evidence or receive a report. A bounty board cannot update itself from global state. **Test**: For any belief, report, or institutional action, trace the path by which the relevant information arrived. If no path exists, the design violates locality.

**Alignment.** All `TargetSpec` variants are local: `ActorPlace`, `EntityAtActorPlace`, `EntityDirectlyPossessedByActor`, `AdjacentPlace`. There is no `EntityAnywhere`. `RuntimeBeliefView` does not expose globally-keyed accessors; every read is routed through the agent's belief store or the FND-14A same-tick co-located shortcut. `PlanningSnapshot` is built from the agent's belief store, not the world.

**Tension.** The snapshot's `shortest_travel_ticks` and `perceived_travel_costs` are *all-pairs* matrices, which sounds globally-keyed — but they are computed from the agent's *believed* place graph, so they are agent-local summaries of locally-acquired knowledge. The fact that the matrices are all-pairs is a performance/cache concern (FND-12, FND-27), not a locality violation, as long as their inputs are entirely from the agent's beliefs.

### FND-12 — Performance May Compress Computation, Never Causality

> Optimization is allowed. Causal cheating is not. Offscreen simulation may batch, summarize, sleep, or approximate only if causally relevant outcomes remain equivalent to the explicit model. You may compress the math. You may not compress away travel time, information delay, inventory depletion, injury recovery, or other state that agents could later observe and react to. The same rule applies to save/load, replay, migration, and any other representation boundary. Boundaries may change encoding, batching, or scheduling strategy, never world meaning. The rule is simple: performance may change how the machine computes a result, never what the world means.

**Alignment.** The planner's many caches (`shortest_travel_ticks`, `perceived_travel_costs`, `entities_at_cache`, `effective_place_cache`, FF / landmark / preferred markings) are read-only derived data. They do not feed back into world state. Beam truncation, expansion budget, plan-depth caps, and TTLs are computation budgets — they bound search effort without lying about what is true.

**Tension.** `BudgetExhausted` can return a plan when the optimal plan was not reachable within the budget. This is a *decision compression* (the agent decides now rather than after a more thorough search), not a *causal compression* — the agent then executes that compromise plan as a lawful sequence of actions, and the world reacts as normal. This is consistent with FND-20 (resource-bounded practical reasoning).

### FND-14 — World State Is Not Belief State

> Ground truth and agent knowledge are separate layers. Agents act on what they believe, remember, infer, suspect, and are told — not on what the simulation knows to be true. A planner may consult only the agent's accessible belief state, memory, and known plans. No AI may silently use omniscient world data to make "smarter" choices. **Test**: If an agent can plan around a fact it has never perceived, inferred, remembered, or been told, the design violates this principle.

**Alignment.** The trait wall is `RuntimeBeliefView`. Every planner read (candidate generation, affordance enumeration, snapshot construction, revalidation, failure classification) goes through that trait. The trait is implemented by the per-agent belief view (`crates/worldwake-sim/src/per_agent_belief_view.rs`). A planning bug that "knew something the agent didn't" would by construction require a violation of the trait wall.

**Tension.** FND-14A (the same-tick co-located exception) is documented in the codebase; the per-agent belief view explicitly routes physical co-located reads to authoritative state. The split is currently enforced by code-review and the integration tests below; there is no static mechanism (e.g., separate trait for omniscient reads) that prevents accidentally widening the FND-14A scope. That is the single biggest discipline risk in this stack.

### FND-16 — Ignorance, Uncertainty, and Contradiction Are First-Class

> Agents must be able to not know, to suspect, to misremember, to hold stale beliefs, and to believe false or conflicting reports. Unknown is not false. Unobserved is not empty. Contradiction is not a system error. Retention is not perfect or free. Beliefs may decay, be overwritten, or be evicted when time passes, memory is weak, or stronger evidence arrives. The simulation must support cases where one witness says the beast fled east, another says west, and the town reacts imperfectly. It must support an owner believing their gold is home while the gold is already gone. **Test**: If the architecture forces every proposition into a clean true/false value for each agent at all times, it is too crude for the target simulation.

**Alignment.** `BeliefStatus { Fresh, Stale, Contradicted }`, `Permille` confidence, `guard_min_confidence_ceiling`, the multi-TTL `*_backoff_ticks` family on `CognitiveProfile`, and the `Discrepancy` machinery all exist precisely so the planner can produce plans on imperfect, contestable beliefs and revise them when reality bites back. `BlockerMemory` and `DiscrepancyMemory` are stale-belief and contradiction stores.

**Tension.** None visible at the planner layer. Possible future tension: `LandmarkSet` and `RelaxedPlanResult` reduce belief state to `PlanningFact` propositions during search — that is appropriate (search needs abstraction) as long as the *gate* into search (`PlanningSnapshot`) preserves uncertainty, which it does.

### FND-20 — Resource-Bounded Practical Reasoning Over Scripts

> AI agents must reason as limited actors in a dynamic world, using beliefs, priorities, habits, skills, and commitments to choose actions. Plans exist to make reasoning tractable under limited time and limited knowledge, not to hard-script a performance. Goals name desired world conditions, not privileged one-step solutions. Reaching them may require enabling subchains — travel, acquisition, queueing, bargaining, pickup, treatment, proof, or retreat — through the same lawful affordances everyone else uses. The implementation may evolve — GOAP, utility systems, BDI, HTN, or hybrids are all acceptable — but the standard does not change: decisions must be explainable as what this agent, with this belief state and these priorities, would try to do.

**Alignment.** This is the architecture's load-bearing principle. The planner is GOAP with hierarchical decomposition (strategic + tactical), bounded expansion budget, beam-truncated frontier, FF heuristic with delete-relaxation, landmark counting, and preferred-operator boosting — all standard tractability tools. Goal kinds name desired world conditions (`Sleep`, `AcquireCommodity`, `PunishAccused`), not specific action recipes. The strategic planner decomposes prerequisites into travel + acquisition + satisfy; the tactical planner generalizes over all lawful `ActionDef`s.

**Tension.** None at the design level. Implementation risk: a future `emit_*` gate that emits a `GoalKind` whose `relevant_op_kinds` returns a single `ActionDef` would collapse goal-with-plan into goal-equals-action, which would tilt toward scripting. The "the planner emits goal X iff condition Y" pattern is the closest the architecture comes to scripting — but as long as `relevant_op_kinds` lists a *family* of lawful actions, the planner remains a planner.

### FND-22 — Agent Diversity Through Concrete Variation

> Agents in the same role must differ in needs, skills, values, loyalties, courage, greed, patience, memory reliability, perception fidelity, and tolerance for risk or ambiguity. These differences come from concrete per-agent parameters, histories, injuries, relationships, and learned experience. Homogeneous populations collapse into herd behavior and single-path outcomes. Diversity is not garnish. It is one of the engines of emergence. **Test**: Two agents with the same role and similar beliefs should still sometimes choose differently because they are not the same person.

**Alignment.** `CognitiveProfile` and `ExecutionBudget` are per-agent. Every field is independently tunable. Combined with per-agent `UtilityProfile`, `TellProfile`, `EpistemicDispositionProfile`, `BeliefConfidencePolicy`, and learned memory (repair, learned-opportunity, survey, blocker, discrepancy, violation), two agents in the same role with the same role definition can rank candidates differently, search differently, switch goals at different margins, and recover from failure at different rates.

**Tension.** All defaults are identical (`Default::default()` is shared). Achieving FND-22's "homogeneous populations collapse" property requires scenarios that actively diversify these profiles — the engine permits diversity but does not generate it.

### FND-26 — Systems Interact Through State, Not Through Each Other

> Systems do not imperatively command each other to force outcomes. They read authoritative state, local beliefs, and prior records; they write new state, effects, and records. Influence travels through state mutation and event history, not through hidden cross-system authority. Shared domain services are allowed when they are generic computations over authoritative state — pathfinding, line-of-sight, legality checks, reservation arbitration, pricing calculations, ballistics, planner search, or similar solvers. Such services compute lawful consequences; they do not grant exceptions or bypass the world model.

**Alignment.** The planner is precisely the "planner search" solver from the principle text. It reads (a) the agent's belief store, (b) the `ActionDefRegistry`, (c) the `RecipeRegistry`, (d) per-agent memory components. It writes (a) chosen actions to the scheduler, (b) blockers/discrepancies/repair entries to the agent's memory. It does not call combat/trade/production systems directly; it dispatches actions and lets those systems read the action queue and react.

**Tension.** None. The `worldwake-systems` crate's modules depend only on `worldwake-core` and `worldwake-sim` (CLAUDE.md invariant: "system modules in `worldwake-systems` depend only on `worldwake-core` and `worldwake-sim`, never on each other"). The planner is the only consumer that depends on all of them.

### FND-28 — No Backward Compatibility in Live Authority Paths

> Do not preserve dead abstractions, alias paths, compatibility layers, deprecated shims, or legacy systems inside the live authoritative simulation simply because old code once depended on them. When the design changes, the live authority path changes with it. Broken callers get updated or removed. Compatibility may exist at boundaries — save migration, import/export, tooling, replay decoding — only if it normalizes into the current model before the world advances.

**Alignment.** The planner has only one entry point per stage. There are no `legacy_search_plan` aliases, no `v1_candidate_generation`, no deprecated wrappers. The `*_with_trace_metadata` variants of `search_plan` are not compatibility shims; they are tracing-augmented forms that share the same body.

**Tension.** None visible. Possible future tension when adding new heuristics or operators — the temptation to keep an "old heuristic path" around for comparison must be resisted.

### FND-29 — Debuggability Is a Product Feature

> Emergence without introspection is indistinguishable from bugs. The simulation must support questions such as: Why did this agent do that? Why did this caravan take this road? Why is this stash empty? Why was this bounty posted? Why was this bounty not posted? Why was the reward unpaid? Who last held this item? Who knows about this event? The answers must be reconstructable from state, beliefs, records, and causal history — not guessed by developers.

**Alignment.** `AgentDecisionTrace`, `RankingOutcome` (which keeps `suppressed`, `damped`, `zero_motive` lists), `PlanSearchTrace`, `SearchExpansionSummary`, `RepairAttemptTrace`, and `CausalLinkCapHit` together let the user reconstruct why any choice was made on any tick. The append-only event log makes "why is this stash empty?" answerable from `EventTag`-tagged events. Decision traces capture rejected alternatives up to `decision_history_alternatives` (default 5).

**Tension.** Trace coverage is paid for by memory and tick time. The user can disable tracing for production-grade scenarios; the system currently provides per-call optional trace arguments rather than a global on/off, which is mildly awkward for production tooling but does not violate the principle.

### FND-29A — Causal History Is Authoritative, Append-Only, and Queryable

> Meaningful world changes must leave stable historical records. Events may be summarized, indexed, or compacted for storage, but the authoritative history must behave as append-only: later evidence may supersede an earlier claim, refute a belief, or close a case, yet it does not erase that the earlier event, claim, belief, or judgment occurred. History must preserve enough structure to answer provenance questions over entities, activities, and responsible agents.

**Alignment.** All plan-relevant events (action commit, blocker recorded, expectation breached, plan adopted, plan abandoned) go through `worldwake-sim`'s append-only event log and emit `EventTag`-tagged records. `Blocker` carries `observed_tick` so its history is preserved even after the blocker clears. `RepairAttemptTrace` and `CausalLinkCapHit` preserve repair attempts that did not succeed.

**Tension.** The planner's internal traces (e.g., `SearchExpansionSummary`) are optional and not part of the authoritative log. That is correct (search internals are not world facts), but the decision *outcome* of each tick is recorded in the agent's history and the event log, which satisfies the principle for everything that matters to other agents.

---

## 10. Live Diagnostics

The codebase ships rich planner-side trace infrastructure but does not capture aggregate metrics by default. Field definitions verbatim (the same shapes that would be filled in if you ran a scenario):

```rust
pub struct AgentDecisionTrace {
    pub agent: EntityId,
    pub tick:  Tick,
    pub outcome: DecisionOutcome,
    pub compiled_opportunities:    Vec<Opportunity>,
    pub opportunity_compiler_load: Option<OpportunityCompilerLoad>,
    pub repair_attempts:           Vec<RepairAttemptTrace>,
    pub causal_link_cap_hits:      Vec<CausalLinkCapHit>,
}

pub struct PlanSearchTrace {
    pub attempts:        Vec<PlanAttemptTrace>,
    pub same_goal_trace: Option<SameGoalPlanningTrace>,
}

pub struct SearchExpansionSummary {
    pub depth:                     u8,
    pub remaining_travel_ticks:    u32,
    pub combined_places_count:     u16,
    pub prerequisite_places_count: u16,
    pub candidates_generated:      u16,
    pub candidates_skipped:        u16,
    pub terminal_successors:       u16,
    pub non_terminal_before_beam:  u16,
    pub non_terminal_after_beam:   u16,
    pub found_goal_satisfied:      bool,
    pub preferred_candidates:      u16,
    pub landmark_heuristic:        u32,
    pub ff_heuristic:              Option<u32>,
    pub helpful_action_count:      u16,
    pub travel_pruning:            Option<TravelPruningTrace>,
    pub prerequisite_guidance:     Option<PrerequisiteGuidanceTrace>,
    pub expansion_candidates:      Vec<ExpansionCandidateTrace>,
    pub root_candidates:           Vec<RootCandidateTrace>,
    pub root_omissions:            Vec<RootOperatorOmissionTrace>,
}

pub struct RepairAttemptTrace {
    pub breach:          BreachSignature,
    pub chosen_kind:     Option<RepairKind>,
    pub rejected:        Vec<(RepairKind, RepairFailure)>,
    pub budget_consumed: u16,
    pub budget_total:    u16,
}

pub struct CausalLinkCapHit {
    pub plan_step_index: u16,
    pub truncated_count: u8,
    pub cap:             u8,
}
```

Notable golden test inventory (a sample exercising different parts of the pipeline — there are ~55 `golden_*.rs` tests in total):

| Test file | Goal kinds | Candidate sources | What it asserts |
|---|---|---|---|
| `golden_survival_baseline.rs` | `AcquireCommodity{SelfConsume}`, `Sleep`, `Relieve`, `ExploreLocation` | Affordance-derived + synthesized exploration fallback | Agent survives 1440 ticks without critical-need breach. |
| `golden_survival_trade.rs` | `AcquireCommodity` via Trade, `ConsumeOwnedCommodity` | Trade affordances + dynamic seller discovery | First trade tick, post-trade inventory, coin flow, stuck-idle bounds. |
| `golden_survival_justice.rs` | `RaidTarget`, `PostBounty`, `Accuse` | Affordance-derived investigation + synthesized bounty posting | Theft detected, accusation completed, bounty terms correct. |
| `golden_exploration.rs` | `ExploreLocation`, fallback `AcquireCommodity` | Synthesized exploration + affordances | Frontier reached; explore demoted on exhaustion; fallback succeeds. |
| `golden_quantity_aware_acquisition.rs` | `AcquireCommodity{quantity: 3}`, `Harvest` | Affordance-derived harvest with contention status | Queueing at extraction slot, grant arrival, partial-completion snapshot. |
| `golden_perception_omission.rs` | Generic production goals | Affordances from truncated snapshots | Omissions logged; revalidation marks omitted affordances unavailable. |
| `golden_survival_combat.rs` | `EngageHostile`, `Defend`, fallback `AcquireCommodity` | Synthesized combat + opportunity compilation | Combat fires only on imminent danger; post-combat switch is clean. |
| `golden_plan_repair.rs` | Multi-step acquisition with dynamic obstacles | Repaired plans + original affordances | Guard breach detected; repair succeeds; `RepairAttemptTrace` populated. |
| `golden_ai_decisions.rs` | Mixed (survival/trade/exploration/justice) | All sources | `DecisionOutcome` correctly attributed; frame lifecycle auditable. |
| `golden_opportunity_compiler.rs` | Production in high-variety scenarios | Compiled opportunities + fresh affordances | Compile cache expires per TTL; cache-hit rate non-zero over time. |

No "live metric capture" infrastructure exists that aggregates counts across many scenarios into a single dashboard. Per-tick traces are sufficient for spot inspection. Adding aggregate metrics (mean candidates per expansion, search-budget hit rate, beam-truncation ratio, average plan depth, FF/landmark-heuristic agreement rate, blocker churn rate) would require:

- A trace consumer that aggregates `SearchExpansionSummary` across a scenario run.
- A persistence path (probably `reports/` write rather than authoritative log).
- A scenario-runner subcommand to dump aggregates.

The infrastructure to *collect* the data exists; only the aggregator is missing.

---

## 11. Architectural Observations

These are patterns and asymmetries noticed while assembling this report. They are flags, not recommendations.

1. **Top-2 candidates only.** `max_candidates_to_plan = 2` is small. Combined with `max_node_expansions = 224` and `beam_width = 8`, the agent gets aggressive budget per candidate but very little breadth across goals. If the top goal cannot find a plan, the agent has exactly one fallback before falling back on blocker recording. Worth measuring how often goal #2 yields the executed plan.

2. **Strategic budget is `2 × max_prerequisite_locations` (default 6).** The strategic planner has six expansions to find a place-itinerary. For acquisition chains with multiple intermediate stops, this might force premature failure into "no strategic plan" → tactical-only mode, which can lead to thrashing. The `golden_quantity_aware_acquisition` and `golden_survival_scattered` tests exercise this; an adversarial scenario with 4+ prerequisites is worth probing.

3. **Heuristic `max(spatial, ff_h, landmark_h)`.** Using max is admissible if each estimate is individually admissible. The landmark count is admissible (every landmark is necessary). The FF heuristic with delete-relaxation is admissible. The spatial heuristic is the minimum perceived travel cost to any goal place, which is admissible when "any goal place" really is sufficient. Combining via max sacrifices some information vs. e.g. sum-with-correction; that is a deliberate trade for safety.

4. **Two-tier search but no full hierarchical decomposition.** Strategic gives a place itinerary; tactical takes the first leg only. Re-planning happens implicitly because each subsequent expansion's tactical goal is recomputed from the new state, but there is no explicit "strategic step done, advance to next strategic step" handoff — the tactical search runs all the way to goal satisfaction in one pass. This is simpler than HTN-style decomposition but means a 5-place itinerary is one tactical search across all 5 legs, which is what the depth/budget bounds limit.

5. **Preferred-operator boost is the only non-A* element.** Pop semantics are pure A* (sorted by `f = g + h`) except for the boost, which gives the preferred queue priority for `preferred_operator_boost` consecutive pops after each successor pulses preferred. This is a small, well-isolated departure from strict A*; its impact depends on FF helpful-action accuracy. Default `2` is conservative.

6. **TTL family on `CognitiveProfile` has 10 separate constants.** The fine grain (`stale`, `contradicted`, `improper_state`, `missing_observation`, `no_legal_binding`, `counterparty_refusal`, `route_unknown`, `search_exhaustion`, `partial_drift`, `expectation_tolerance`) gives precise tuning but is a wide tuning surface for non-experts. Worth a memo on which knob to turn for which failure mode.

7. **Blocker scoping is hierarchical but matching is exact.** `BlockerKey { goal_key, place?, target?, action_def? }` allows narrow blockers, but the candidate-side matcher requires field-by-field equality. A "any acquisition at this place fails" blocker still has to be keyed to a specific `goal_key.kind = AcquireCommodity`. Cross-goal blockers (e.g., "this place is dangerous for *any* goal") would need a separate mechanism.

8. **Revalidation is per-step, not per-plan.** Each plan step is revalidated immediately before dispatch, not the whole plan at once. This is correct under FND-21 (revisable commitments) but means a multi-step plan can succeed at step 1 and fail at step 3 due to changes that were already true at step 1 but not noticed until step 3 is revalidated. This is intentional — beliefs may change between step 1's dispatch and step 3's revalidation — but worth understanding when reading traces.

9. **Repair budget is 25% of expansion budget.** `repair_budget_fraction = 250 / 1000` × `max_node_expansions = 224` ≈ 56 nodes. That is tight for any non-trivial repair. The system's stance is "if repair takes a lot, just replan from scratch," which is defensible but means small belief shifts that ought to be repairable in 60–80 nodes will trip the full pipeline.

10. **Per-tick decision uses fresh snapshot, not incremental.** `PlanningSnapshot` is rebuilt each planning cycle from the belief store. With `Floyd-Warshall` O(n³) distance precomputation and full entity census, the per-tick cost grows with the size of the agent's believed world. The `snapshot_travel_horizon = 6` cap limits this, but for agents who travel widely the believed-place count can still grow large. No incremental snapshot is currently maintained.

11. **`max_plan_depth = 8` is small for crafting chains.** With travel + acquisition + setup + craft + delivery, even a simple production goal can hit 8 steps. The `golden_survival_production` test exercises this. A depth-bound is correct in spirit but the default may force production goals to terminate at `ProgressBarrier` rather than `GoalSatisfied` more often than expected — instrumenting `terminal_kind` distribution would expose this.

12. **`use_ff_heuristic` is a single per-agent boolean.** When `false`, the planner falls back to spatial + landmark heuristics only. There is no middle ground (e.g., "use FF on every Nth expansion"). This is a clean abstraction but precludes per-goal FF policy.

13. **No explicit FND-22 default differentiation.** All agents start with identical `CognitiveProfile::default()`. Scenarios are the only injection point for diversity; engine-side baseline diversity (e.g., random ±10% on each agent at spawn) is not provided. Whether to do so is a scenario-author concern.

14. **`AskWitness` cap is 3 per topic; no analogous caps on emit gates.** Most `emit_*` functions are uncapped (other than what blockers and travel horizon already constrain). Worth measuring whether other gates (e.g., `emit_artifact_posting_candidates`) ever produce pathological counts in dense scenarios.

15. **Decision history retains 5 alternatives by default.** `decision_history_alternatives = 5` is short for forensics ("why didn't the agent pick X?" where X is candidate 6 or beyond). Bumping it for golden-test diagnostics is a per-agent change.

16. **Plan repair lives between revalidation and full replan.** A breach is first attempted as a recoverable-travel-step fix-up, then as localized repair, only then as full replan via `handle_plan_failure`. Each tier has its own budget. Worth tracing what fraction of breaches survive each tier in production scenarios.

17. **`PlanningState` uses persistent (structurally-shared) maps.** Cloning per-successor is fast (O(1) plus per-mutation copy-on-write). But the `entities_at_cache` and `effective_place_cache` are `Rc<RefCell<...>>` — shared mutable state across siblings of a search node. This is correct under the assumption that the cache is purely a memoization (idempotent), but any future cache field that becomes order-dependent would silently break determinism. Static documentation of this invariant would be valuable.

---

*End of report. Generated 2026-05-13 from `crates/worldwake-ai/src/` and `crates/worldwake-core/src/`. No live test execution was performed; all numbers cited as "defaults" are read verbatim from `impl Default` blocks and `default_*` `const fn`s in `cognitive_profile.rs` and `execution_budget.rs`.*
