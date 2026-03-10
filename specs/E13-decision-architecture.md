# E13: Grounded Agent Decision Architecture

## Epic Summary
Implement the unified AI pipeline for Phase 2: grounded candidate-goal generation, utility ranking, GOAP-style planning over **parameterized** action steps, plan revalidation, reactive execution, and belief-only decision making. All AI code must operate through `&dyn BeliefView`.

## Phase
Phase 2: Survival & Logistics (final epic before Phase 2 gate)

## Crate
`worldwake-ai`

## Dependencies
- E09 (physiology: `HomeostaticNeeds`, `DriveThresholds`)
- E10 (recipes, workstations, transport / route occupancy)
- E11 (trade affordances, demand memory, merchandise profiles)
- E12 (wounds, combat state, corpses)

## Foundations Alignment Changes
This revision fixes the largest architecture problems in the original spec:

1. **Future-epic goals are removed from Phase 2.** `ClaimOffice`, `SupportClaimant`, `EstablishCamp`, and similar goals were not grounded by current systems.
2. **Planning stores parameterized action steps, not bare `ActionDefId`s.** A plan step must know *which* seller, *which* item, *which* destination, and *which* workstation.
3. **Danger is derived from beliefs about threats, not a stored fear score.**
4. **Goal selection becomes grounded candidate generation + ranking**, not picking from a static prose wish list.
5. **Replan loops gain a concrete dampener** via failure memory instead of hoping needs eventually change enough.

## Deliverables

### UtilityProfile Component
Per-agent utility / temperament weights enabling Principle 11.

- `hunger_weight: Permille`
- `thirst_weight: Permille`
- `fatigue_weight: Permille`
- `bladder_weight: Permille`
- `dirtiness_weight: Permille`
- `pain_weight: Permille`
- `danger_weight: Permille`
- `enterprise_weight: Permille` — importance of maintaining saleable stock / pursuing profitable work when the agent has concrete commerce or production affordances

Removed from the Phase 2 profile:
- `fear_weight` — replaced by `danger_weight`, because danger is derived from believed threats
- `greed_weight` as “coin deficit” — too abstract / underspecified
- `sociability_weight` — no grounded social system exists yet

### Shared DriveThresholds Usage
E13 consumes the shared `DriveThresholds` component introduced as Phase 2 schema. E13 does **not** own it.

Important correction:
- thresholds must be **per drive**
- AI interrupts and urgency calculations use the relevant drive’s threshold band, not one generic number for every pressure

### Grounded Candidate Goal Generation
Goal selection is a two-stage process:

1. **Generate grounded candidate goals** from current beliefs and concrete evidence
2. **Rank those candidates** with utility scoring

Candidate goals must arise from actual state, not from a static universal wishlist.

Examples of grounded evidence:
- hunger above threshold + owned food -> `ConsumeOwnedFood`
- hunger above threshold + no owned food + visible sellers -> `BuyFood`
- fatigue high + visible bed / sleep spot -> `Sleep`
- merchant has `MerchandiseProfile` for bread + no bread in stock + recent demand memory -> `RestockCommodity(bread)`
- known recipe + free workstation + available source stock -> `ProduceCommodity(recipe_id)`
- nearby corpse with useful items -> `LootCorpse(corpse_id)`
- self or co-located ally wounded + medicine available -> `Heal(target)`

### Phase 2 Goal Kinds
The allowed goal kinds in Phase 2 are:

- `ConsumeOwnedCommodity { commodity }`
- `AcquireCommodity { commodity, purpose }`
- `Sleep`
- `Relieve`
- `Wash`
- `ReduceDanger`
- `Heal { target }`
- `ProduceCommodity { recipe_id }`
- `SellCommodity { commodity }`
- `RestockCommodity { commodity }`
- `MoveCargo { lot, destination }`
- `LootCorpse { corpse }`
- `BuryCorpse { corpse, burial_site }` (optional if burial affordances exist)

Explicitly **deferred out of Phase 2**:
- `ClaimOffice`
- `SupportClaimant`
- `Escort`
- `Raid`
- `EstablishCamp`

Those require later epics to provide lawful grounding.

### Utility Scoring
Utility scoring ranks candidate goals; it does not invent them.

A compliant scoring pipeline must:
- read `HomeostaticNeeds`
- read `DriveThresholds`
- derive pain from `WoundList`
- derive danger from believed hostile presence / current attackers / recent local violence
- read commerce / production affordances
- read `DemandMemory` and `MerchandiseProfile` when evaluating restock / sell goals

Example interface:

```rust
fn score_candidate(
    agent: EntityId,
    candidate: &GroundedGoal,
    view: &dyn BeliefView,
    profile: &UtilityProfile,
) -> Permille
```

### OmniscientBeliefView as Temporary P10 Violation
`OmniscientBeliefView` remains an explicit temporary violation of the final intelligent-agency standard. It is permitted only as the adapter used at the call site while E14 is pending.

All E13 logic must still operate on `&dyn BeliefView`, never on `&World` or concrete world access.

### Required BeliefView Trait Extensions
The trait must support the data needed to generate and rank grounded goals.

Required extensions include:

- `homeostatic_needs(agent: EntityId) -> Option<HomeostaticNeeds>`
- `drive_thresholds(agent: EntityId) -> Option<DriveThresholds>`
- `wounds(agent: EntityId) -> Vec<Wound>`
- `visible_hostiles_for(agent: EntityId) -> Vec<EntityId>`
- `agents_selling_at(place: EntityId, commodity: CommodityKind) -> Vec<EntityId>`
- `known_recipes(agent: EntityId) -> Vec<RecipeId>`
- `matching_workstations_at(place: EntityId, tag: WorkstationTag) -> Vec<EntityId>`
- `resource_sources_at(place: EntityId, commodity: CommodityKind) -> Vec<EntityId>`
- `demand_memory(agent: EntityId) -> Vec<DemandObservation>`
- `merchandise_profile(agent: EntityId) -> Option<MerchandiseProfile>`
- `corpse_entities_at(place: EntityId) -> Vec<EntityId>`
- `in_transit_state(entity: EntityId) -> Option<InTransitOnEdge>`

These are belief queries, not world queries.

### Parameterized Planning
The planner output must be parameterized.

```rust
struct PlannedAction {
    action_id: ActionDefId,
    target_entity: Option<EntityId>,
    target_place: Option<EntityId>,
    target_item: Option<EntityId>,
    quantity: Option<Quantity>,
    reservation_target: Option<EntityId>,
}
```

Storing only `ActionDefId::Negotiate` or `ActionDefId::Travel` is insufficient.  
A valid plan must remember *who* to negotiate with and *where* to travel.

### GOAP-Style Planner
- Operates on compact `PlanningState` derived from `BeliefView`
- Forward state-space search with action effects
- Only actions whose physical preconditions are satisfiable in planning state may be expanded
- Multi-step goals such as restock and procurement are legal only when their intermediate steps are grounded

### Plan Revalidation
Before each plan step:
- re-check physical preconditions against current beliefs
- if false, emit replan reason event
- discard or revise invalid tail of plan

### Failure Memory / BlockedIntent
Repeated immediate replanning into the same impossible action is forbidden.

Store concrete failure memory on the agent:

```rust
struct BlockedIntent {
    goal_kind: GoalKind,
    related_entity: Option<EntityId>,
    blocking_fact: BlockingFact,
    observed_tick: u64,
}
```

Candidate generation and scoring must consult this memory. The agent should avoid retrying the same blocked target until:
- the relevant believed state changes, or
- the memory naturally expires

This is the concrete dampener for replan loops.

### Reactive Executor
Interrupt evaluation each tick uses:
- derived danger pressure vs `DriveThresholds.danger`
- physiology pressures vs relevant per-drive thresholds
- current wound state
- interruptibility of the active action

Examples:
- hostile appears / current attacker persists -> interrupt to `ReduceDanger`
- critical thirst while hauling cargo -> interrupt to seek water if physically possible
- acute wound / bleeding -> interrupt for `Heal` or escape

### Agent Tick Integration
Each tick, for each `ControlSource::Ai` agent:

1. derive current candidate goals from beliefs
2. rank candidates
3. if no current plan or plan invalid -> plan
4. if a reactive interrupt outranks current action -> interrupt
5. execute / progress current step

Human-controlled agents still use the same action legality and effect pipeline. `ControlSource` only changes the source of chosen inputs.

## Component Registration
New components to register in `component_schema.rs`:

- `UtilityProfile` — on `EntityKind::Agent`
- `BlockedIntentMemory` / stored blocked intents — on `EntityKind::Agent`
- parameterized current plan / current goal state — on `EntityKind::Agent` if not already present in scheduler-owned state

`DriveThresholds` is **not** introduced here; it is shared Phase 2 schema.

## Cross-System Interactions (Principle 12)
- **E09 → E13**: reads physiology and per-drive thresholds
- **E10 → E13**: reads recipes, workstations, sources, cargo, and transit state
- **E11 → E13**: reads demand memory, visible sellers, and merchandise profiles
- **E12 → E13**: reads wounds, corpses, and local hostile evidence
- **E13 → all**: requests actions through the scheduler / action framework; it does not call other systems directly

## FND-01 Section H

### Information-Path Analysis
- all decision inputs come through `&dyn BeliefView`
- danger is derived from believed threats, not from omniscient global danger scores
- grounded goals arise from local or remembered evidence
- future E14 replaces the omniscient adapter with actual per-agent beliefs without changing E13 logic

### Positive-Feedback Analysis
- **Success at procurement / trade → more opportunities → more enterprise behavior**
- **Plan failure → repeated replanning → same failure**

### Concrete Dampeners
- **Enterprise amplification dampeners**:
  - physiology pressures
  - carry capacity
  - travel time
  - finite source stock
- **Replan-loop dampener**:
  - concrete `BlockedIntent` memory tied to a fact and related entity
  - the memory fades only with time or observed world change

### Stored State vs. Derived Read-Model
**Stored (authoritative)**:
- `UtilityProfile`
- `BlockedIntent` memory
- current goal
- current plan as parameterized steps

**Derived (transient read-model)**:
- candidate goal set
- utility scores
- pain pressure
- danger pressure
- affordances available from current beliefs
- `PlanningState`

## Invariants Enforced
- 9.11: World / belief separation — planner uses beliefs only
- 9.12: No player branching — same legality / execution pipeline regardless of `ControlSource`
- 9.14: Dead agents skipped entirely
- Principle 3: no stored utility scores, danger scores, or urgency scores
- Principle 11: agent diversity through `UtilityProfile`, `DriveThresholds`, `KnownRecipes`, `MerchandiseProfile`, and concrete memory differences

## Tests
- [ ] T12: No player branching — switching `ControlSource` does not change available actions or effects
- [ ] All planning / scoring uses `&dyn BeliefView` only
- [ ] Goal generation creates only goals grounded in current believed evidence
- [ ] Different `UtilityProfile` values produce different rankings for the same candidate set
- [ ] Different per-drive thresholds produce different interrupt behavior
- [ ] Danger interrupts derive from believed hostile presence, not a stored fear scalar
- [ ] Parameterized planning stores target entities / places / quantities, not only `ActionDefId`
- [ ] Replan triggers when a physical precondition fails
- [ ] `BlockedIntent` memory suppresses immediate repeated retries against the same blocked target
- [ ] Dead agents generate no goals or plans
- [ ] Planner finds valid simple plans (eat, buy food, sleep, produce commodity)
- [ ] Plan revalidation catches stale plans
- [ ] Same agent type behaves lawfully under either `ControlSource`
- [ ] No `sociability_weight`, no stored fear, and no “coin deficit” utility shortcut remain in Phase 2
- [ ] `PlanningState` is never stored as authoritative world state

## Phase 2 Gate
After E13, verify:
- [ ] Agents autonomously eat, drink, sleep, wash, and relieve themselves
- [ ] Agents can buy needed goods when available
- [ ] Merchants restock through physical procurement paths
- [ ] Basic survival loop runs for 24+ in-world hours without deadlock
- [ ] No agent starves when food is available, reachable, and believed reachable
- [ ] No agent loops forever on the same blocked target without either world change or memory expiry

## Acceptance Criteria
- Unified pipeline: beliefs -> grounded candidate goals -> utility ranking -> parameterized plan -> execution
- All AI reads go through `&dyn BeliefView`
- Plans are parameterized, not just action IDs
- Future-epic goals are not smuggled into Phase 2
- Reactive interrupts use derived danger and per-drive thresholds
- Replan loops are damped by concrete failure memory
- Same legal action / effect pipeline applies to human and AI agents
- `OmniscientBeliefView` remains documented as temporary scaffolding only

## Spec References
- Section 6.1 (one hierarchy, not three competing brains)
- Section 6.2 (goal examples)
- Section 6.3 (planning rules: compact state, revalidation, beliefs only)
- Section 6.4 (human control uses same pipeline)
- Section 9.11 (world / belief separation)
- Section 9.12 (player symmetry)
- `docs/FOUNDATIONS.md` Principles 2, 3, 7, 9, 10, 11, 12