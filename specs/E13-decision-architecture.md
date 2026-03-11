# E13: Grounded Agent Decision Architecture

## Epic Summary
Implement the first production decision architecture for Phase 2: grounded candidate generation, deterministic priority ranking, bounded plan search over planner-visible action semantics, affordance-keyed execution, plan revalidation, reactive interrupts, and belief-only decision making.

All AI reads must go through `&dyn BeliefView`.
All AI outputs must be `InputKind::RequestAction`.

Critical correction:
E13 is **not** “generic GOAP over arbitrary action handlers.”
The current codebase does not expose declarative action effects, and `DurationExpr::resolve_for()` still reads `&World`.
Therefore E13 must use:

1. `get_affordances()` as the sole legality / successor source
2. planner-local semantics for a bounded set of Phase 2 action families
3. belief-side duration estimation
4. exact affordance keys as plan steps
5. full replanning from current beliefs on invalidation
6. goal persistence across short plan prefixes when a step materializes unknown future targets

This still yields grounded, emergent behavior, but it matches what the codebase can actually support.

## Phase
Phase 2: Survival & Logistics

## Crate
`worldwake-ai`

## Dependencies
- E09 (`HomeostaticNeeds`, `DriveThresholds`)
- E10 (`KnownRecipes`, `ResourceSource`, `InTransitOnEdge`, workstations, cargo movement)
- E11 (`DemandMemory`, `MerchandiseProfile`, trade affordances)
- E12 (`WoundList`, corpses, combat action affordances)

### Cargo.toml Fix
`worldwake-ai/Cargo.toml` must add the missing `worldwake-sim` dependency.
E13 depends on `BeliefView`, `Affordance`, `ActionDefRegistry`, `ActionDefId`, `ActionPayload`, `InputEvent`, `ReplanNeeded`, and `OmniscientBeliefView`.

## Hard Scope Correction

### In Scope for E13
Single-agent Phase 2 goals grounded by current systems:
- survival / bodily upkeep
- local procurement
- local trade
- production and restock
- local cargo movement
- corpse looting / burial if affordances exist
- immediate danger reduction
- simple heal behavior
- bounded combat commitment as a final step, never as a full tactical planner

### Explicitly Out of Scope for E13
Do not smuggle these into Phase 2:
- law / policing / denouncement
- contracts / bounty boards
- office / succession behavior
- rumor propagation
- shared party plans
- expedition leadership
- social persuasion
- equipment adequacy evaluation for future combat
- crime investigation
- escort / raid / camp systems

Those require later world systems and later belief machinery.

### Forward-Compatible Requirement
Even though those later behaviors are out of scope now, E13 must be structured so they can be added by introducing:
- new `GoalKind` entries
- new `PlannerOpKind` entries
- new belief queries
- new goal semantics
without replacing the core loop.

## Core Architecture Corrections

1. Replace the current vague `PlannedAction` sketch with an exact affordance-keyed step.
   The plan must store the exact `def_id`, exact ordered `targets`, and exact `payload_override` needed to emit `InputKind::RequestAction`.

2. Do not promise generic action-effect planning.
   `ActionDef` has no declarative effect list.
   E13 must introduce a planner-local `PlannerOpKind` + semantics table for only the Phase 2 action families the planner understands.

3. Define `PlanningSnapshot` and `PlanningState` explicitly.
   Search nodes must not clone the world or clone heavy vectors per node.

4. Revalidate steps by affordance identity, not by re-implementing precondition logic.
   If the same affordance is no longer present, the step is invalid.

5. Replan from scratch on failure.
   Phase 2 does not attempt tail surgery, partial splice repair, or alternative-step patching inside an existing plan.

6. Planning is event-driven and budgeted.
   E13 must not run full planning for every agent on every tick if nothing relevant changed.

7. Introduce materialization barriers.
   If a step creates or transfers future target identities that cannot be bound stably in advance, the plan ends there, preserves the top-level goal, and replans from the new belief state next tick.

8. Enterprise never overrides critical survival.
   Commerce and production are real ambitions, but they are capped beneath critical bodily danger.

## Deliverables

### 1) UtilityProfile Component
Per-agent utility / temperament weights enabling Principle 11.

    pub struct UtilityProfile {
        pub hunger_weight: Permille,
        pub thirst_weight: Permille,
        pub fatigue_weight: Permille,
        pub bladder_weight: Permille,
        pub dirtiness_weight: Permille,
        pub pain_weight: Permille,
        pub danger_weight: Permille,
        pub enterprise_weight: Permille,
    }

Corrections:
- No `fear_weight`
- No `greed_weight` as abstract coin deficit
- No `sociability_weight`
- No stored “urgency score” or “danger score”

`enterprise_weight` does **not** map to a stored enterprise pressure.
It multiplies a candidate-specific `opportunity_signal` derived from concrete demand, stock deficit, reachable sellers/sources/workstations, and actually available sale paths.

### 2) BlockedIntentMemory Component
Failure memory is authoritative agent memory and must be stored on the agent.

    pub struct BlockedIntentMemory {
        pub intents: Vec<BlockedIntent>,
    }

    pub struct BlockedIntent {
        pub goal_key: GoalKey,
        pub blocking_fact: BlockingFact,
        pub related_entity: Option<EntityId>,
        pub related_place: Option<EntityId>,
        pub observed_tick: Tick,
        pub expires_tick: Tick,
    }

    pub enum BlockingFact {
        NoKnownPath,
        NoKnownSeller,
        SellerOutOfStock,
        TooExpensive,
        SourceDepleted,
        WorkstationBusy,
        ReservationConflict,
        MissingTool(UniqueItemKind),
        MissingInput(CommodityKind),
        TargetGone,
        DangerTooHigh,
        CombatTooRisky,
        Unknown,
    }

Expiry policy:
- transient blockers use a short TTL
  - out of stock
  - workstation busy
  - reservation conflict
  - temporary target absence
- structural blockers use a long TTL
  - no known seller
  - missing tool
  - no path
  - combat too risky
- any blocker clears early if the believed blocker is no longer true
- `Unknown` uses the short TTL

These TTLs are AI config constants, not world-tuning variables.

### 3) Runtime-Only Decision State
Current plans and search frontiers are **not** authoritative world state.
They live in scheduler / AI runtime state.

    pub struct AgentDecisionRuntime {
        pub current_goal: Option<GoalKey>,
        pub current_plan: Option<PlannedPlan>,
        pub dirty: bool,
        pub last_priority_class: Option<GoalPriorityClass>,
    }

This state must not be registered in `component_schema.rs`.

## Goal Model

### Goal Kinds Allowed in Phase 2

    pub enum CommodityPurpose {
        SelfConsume,
        Restock,
        RecipeInput(RecipeId),
        Treatment,
    }

    pub enum GoalKind {
        ConsumeOwnedCommodity { commodity: CommodityKind },
        AcquireCommodity { commodity: CommodityKind, purpose: CommodityPurpose },
        Sleep,
        Relieve,
        Wash,
        ReduceDanger,
        Heal { target: EntityId },
        ProduceCommodity { recipe_id: RecipeId },
        SellCommodity { commodity: CommodityKind },
        RestockCommodity { commodity: CommodityKind },
        MoveCargo { lot: EntityId, destination: EntityId },
        LootCorpse { corpse: EntityId },
        BuryCorpse { corpse: EntityId, burial_site: EntityId },
    }

Normalize goal identity for planning, switching, and failure memory:

    pub struct GoalKey {
        pub kind: GoalKind,
        pub commodity: Option<CommodityKind>,
        pub entity: Option<EntityId>,
        pub place: Option<EntityId>,
    }

    pub enum GoalPriorityClass {
        Critical,
        High,
        Medium,
        Low,
        Background,
    }

    pub struct GroundedGoal {
        pub key: GoalKey,
        pub evidence_entities: BTreeSet<EntityId>,
        pub evidence_places: BTreeSet<EntityId>,
    }

    pub struct RankedGoal {
        pub grounded: GroundedGoal,
        pub priority_class: GoalPriorityClass,
        pub motive_score: u32,
    }

`GroundedGoal` must be built from concrete current beliefs only.
No static universal wish list.
No role-script fallback tree.

## Required BeliefView Extensions

Correction:
`BeliefView` currently has 23 methods.
E13 adds these methods alongside the existing ones.

Required additions:

- `homeostatic_needs(agent: EntityId) -> Option<HomeostaticNeeds>`
- `drive_thresholds(agent: EntityId) -> Option<DriveThresholds>`
- `wounds(agent: EntityId) -> Vec<Wound>`
- `visible_hostiles_for(agent: EntityId) -> Vec<EntityId>`
- `current_attackers_of(agent: EntityId) -> Vec<EntityId>`
- `agents_selling_at(place: EntityId, commodity: CommodityKind) -> Vec<EntityId>`
- `known_recipes(agent: EntityId) -> Vec<RecipeId>`
- `matching_workstations_at(place: EntityId, tag: WorkstationTag) -> Vec<EntityId>`
- `resource_sources_at(place: EntityId, commodity: CommodityKind) -> Vec<EntityId>`
- `demand_memory(agent: EntityId) -> Vec<DemandObservation>`
- `merchandise_profile(agent: EntityId) -> Option<MerchandiseProfile>`
- `corpse_entities_at(place: EntityId) -> Vec<EntityId>`
- `in_transit_state(entity: EntityId) -> Option<InTransitOnEdge>`
- `adjacent_places_with_travel_ticks(place: EntityId) -> Vec<(EntityId, NonZeroU32)>`
- `estimate_duration(
      actor: EntityId,
      duration: DurationExpr,
      targets: &[EntityId],
      payload: &ActionPayload,
  ) -> Option<ActionDuration>`

Semantics requirements:
- `visible_hostiles_for()` must be local in meaning.
  In Phase 2 it must only return believed threats at the agent’s effective place or sharing the same transit edge.
  It must never become a hidden global “everyone dangerous everywhere” query.
- `current_attackers_of()` returns believed direct combat opponents currently attacking the agent.
- `estimate_duration()` must succeed for all planner-visible Phase 2 action families except leaf-only indefinite combat actions.

`OmniscientBeliefView` must implement all of the above as temporary scaffolding until E14 replaces it with actual per-agent beliefs.

## Derived Pressures

### Pain
Pain remains transient and derived each tick.

    derive_pain_pressure(view, agent):
        sum all `Wound.severity`
        cap at `Permille(1000)`

### Danger
Danger remains transient and derived each tick.

Danger derivation must be monotone and local:
- no hostiles and no attackers -> `Permille(0)`
- hostile presence without active attack -> at least the agent’s danger medium band
- active attacker present -> at least the agent’s danger high band
- multiple attackers, or any attacker while wounded / incapacitated -> at least the agent’s danger critical band

Reference implementation rule:

    derive_danger_pressure(view, agent):
        let thresholds = view.drive_thresholds(agent)?;
        let attackers = view.current_attackers_of(agent);
        let hostiles = view.visible_hostiles_for(agent);

        if attackers.is_empty() && hostiles.is_empty() {
            Permille(0)
        } else if attackers.len() >= 2
            || (!attackers.is_empty() && (view.has_wounds(agent) || view.is_incapacitated(agent)))
        {
            thresholds.danger.critical()
        } else if !attackers.is_empty() {
            thresholds.danger.high()
        } else {
            thresholds.danger.medium()
        }

This keeps danger grounded in concrete threat state without inventing a stored fear scalar.

## Candidate Goal Generation

Candidate generation is a pure read-model pass over current beliefs.

A candidate may only be emitted when concrete believed evidence exists that the goal is relevant **and** there is at least one concrete path class that could pursue it.

### Candidate Rules

- `ConsumeOwnedCommodity { commodity }`
  Emit when:
  - the relevant self-care drive is at or above its low band, and
  - the agent already controls a matching consumable commodity

- `AcquireCommodity { commodity, SelfConsume }`
  Emit when:
  - the relevant self-care drive is at or above its low band,
  - the agent does not control enough matching commodity,
  - and at least one concrete acquisition path exists:
    - controlled off-site stock
    - reachable seller
    - reachable source
    - reachable recipe path
    - reachable corpse with useful loot

- `Sleep`, `Relieve`, `Wash`
  Emit when:
  - the corresponding drive is at or above its low band, and
  - a reachable affordance / place path exists

- `ReduceDanger`
  Emit when:
  - derived danger is above zero, and
  - a concrete danger-reduction path exists:
    - move to a safer adjacent place
    - heal immediate wounds if healing affordance exists
    - attack a current attacker if combat affordance exists

- `Heal { target }`
  Emit when:
  - target is alive and wounded, and
  - a healing / treatment path exists

- `ProduceCommodity { recipe_id }`
  Emit when:
  - recipe is known,
  - its outputs serve a concrete current purpose:
    - self-consume
    - recipe input for another grounded path
    - merchant restock
    - merchant sale
  - and source / input / workstation paths exist

- `SellCommodity { commodity }`
  Emit when:
  - the agent controls matching commodity,
  - and a concrete reachable sell path exists

- `RestockCommodity { commodity }`
  Emit when:
  - the agent has a `MerchandiseProfile` that includes the commodity,
  - current sale stock is absent or insufficient,
  - and a concrete replenishment path exists:
    - fetch own stock
    - buy
    - harvest
    - craft

- `MoveCargo { lot, destination }`
  Emit when:
  - the agent can control the lot,
  - and the destination is concrete and different from the lot’s current place / holder

- `LootCorpse { corpse }`
  Emit when:
  - corpse is reachable,
  - and it is believed to still have useful possessions

- `BuryCorpse { corpse, burial_site }`
  Emit only when:
  - burial affordances actually exist,
  - and corpse + burial site are both concrete

### Acquisition Preference Rule
For any acquisition / restock / production chain, the planner must prefer already-controlled resources before creating new ones.

Preference order:
1. use on-hand stock
2. fetch controlled off-site stock
3. move controlled cargo
4. buy
5. harvest
6. craft
7. loot

This is not a script.
It is a causally grounded preference for “use what you already control before creating more work.”

## Utility and Priority Ranking

### Priority Class
Ranked candidates are ordered first by `GoalPriorityClass`.

Mapping:
- self-care goals (`ConsumeOwnedCommodity`, `AcquireCommodity(SelfConsume)`, `Sleep`, `Relieve`, `Wash`) use the corresponding drive band
- `ReduceDanger` uses danger band
- `Heal` uses pain band, but is promoted by one class if danger is also high or critical
- `RestockCommodity`, `ProduceCommodity`, `SellCommodity`, `MoveCargo` are capped at `Medium`
- `LootCorpse` and `BuryCorpse` are capped at `Low`

Hard cap rule:
- if any self-care drive or danger is `Critical`, enterprise / loot / burial goals may not outrank it
- if danger is `High`, loot and burial candidates are suppressed entirely
- if any self-care drive is `High`, loot and burial are suppressed entirely

### Motive Score
Inside a priority class, rank by `motive_score: u32`.

Formulas:

- self-care:
  `motive_score = relevant_weight.raw() * relevant_pressure.raw()`

- `ReduceDanger`:
  `motive_score = danger_weight.raw() * danger_pressure.raw()`

- `Heal`:
  `motive_score = pain_weight.raw() * pain_pressure.raw()`
  plus a danger contribution if danger is non-zero

- enterprise goals:
  `motive_score = enterprise_weight.raw() * opportunity_signal.raw()`

`opportunity_signal` is candidate-specific and derived from concrete facts only:
- current stock deficit at sale point
- retained `DemandMemory` for that commodity
- known reachable replenishment path
- whether the agent already controls needed inputs or lots

There is no global stored enterprise score.

### Tie-Breaking
Deterministic ordering only:
1. `GoalPriorityClass`
2. `motive_score`
3. cheapest valid plan ticks
4. `GoalKind` discriminant
5. commodity / entity / place ids in lexicographic order

No `HashMap`
No `HashSet`
No non-deterministic priority queue behavior

## Planner-Visible Action Semantics

Because `ActionDef` has no declarative effect model, E13 must define a planner-local semantics table.

    pub enum PlannerOpKind {
        Travel,
        Consume,
        Sleep,
        Relieve,
        Wash,
        TradeAcquire,
        TradeSell,
        Harvest,
        Craft,
        MoveCargo,
        Heal,
        Loot,
        Bury,
        Attack,    // leaf-only in Phase 2
    }

For each planner-visible action family, the semantics table must provide:
- which `ActionDefId` maps to which `PlannerOpKind`
- whether it may appear mid-plan or only as a leaf
- whether it is a materialization barrier
- how it updates `PlanningState`
- which goal families it is relevant to

### Materialization Barrier Rule
A step is a materialization barrier if its successful result produces item / entity identities that cannot be bound stably before execution.

Examples in Phase 2:
- trade acquisition
- harvest
- craft
- loot
- any future action that creates new entities or transfers unknown target identities

When a barrier step is selected:
- the plan may end there even if the top-level goal is not yet fully satisfied
- the top-level goal remains active
- E13 replans from the new belief state next tick

This is the lawful replacement for pretending future unknown item ids already exist.

### Exact Plan Step Type
Replace the current ambiguous `PlannedAction` sketch with:

    pub struct PlannedStep {
        pub def_id: ActionDefId,
        pub targets: Vec<EntityId>,               // exact `Affordance.bound_targets` order
        pub payload_override: Option<ActionPayload>,
        pub op_kind: PlannerOpKind,
        pub estimated_ticks: u32,
        pub is_materialization_barrier: bool,
    }

    pub enum PlanTerminalKind {
        GoalSatisfied,
        ProgressBarrier,
    }

    pub struct PlannedPlan {
        pub goal: GoalKey,
        pub steps: Vec<PlannedStep>,
        pub total_estimated_ticks: u32,
        pub terminal_kind: PlanTerminalKind,
    }

This fixes the `PlannedAction -> InputKind::RequestAction` mismatch completely.
The plan stores the exact flat `targets` vector that the simulation already expects.

## PlanningSnapshot and PlanningState

### Snapshot Boundary
Planning is candidate-specific, not world-global.

For each candidate considered for planning, build a `PlanningSnapshot` that includes only:
- the actor
- the actor’s current place or transit state
- the actor’s body state:
  - `HomeostaticNeeds`
  - `DriveThresholds`
  - wounds summary
  - alive / incapacitated state
- reachable places within the plan’s travel horizon
- places and entities explicitly referenced by the candidate evidence
- holders / containers / direct possessions of those entities
- local sellers, corpses, workstations, sources, and controllable cargo at included places
- local reservation information for included reservables
- weighted local travel edges for included places
- recipe knowledge, demand memory, merchandise profile

The snapshot must not include:
- full world clones
- unrelated remote inventories
- cloned heavy vectors per search node
- arbitrary global state not relevant to the current candidate

### Search Node Structure
`PlanningState` is:
- one immutable snapshot
- plus a compact per-node delta overlay

Per-node deltas may include:
- actor place override
- possession / container overrides for referenced entities
- resource quantity overrides for referenced sources
- reservation shadow for already-planned steps
- scalar drive / pain summary updates
- local corpse / target availability overrides

Search nodes must not clone `Vec<Wound>` or `Vec<DemandObservation>` per node.
Those live once in the snapshot.

### Belief Interface
`PlanningState` must implement `BeliefView` so `get_affordances()` can be used as the successor generator for hypothetical states.

## Goal Semantics

Each `GoalKind` must define:
- `is_satisfied(planning_state) -> bool`
- `is_progress_barrier(step, post_state) -> bool`
- `relevant_op_kinds() -> &'static [PlannerOpKind]`

Examples:
- `ConsumeOwnedCommodity` is satisfied when a consume step has lowered the relevant drive below the agent’s medium band in planning state
- `AcquireCommodity(SelfConsume)` is satisfied when the actor controls the desired commodity locally, or ends at a progress barrier that acquires it
- `RestockCommodity` is satisfied when stock is back at the sale point, or ends at a progress barrier that produced / acquired the desired stock for the same top-level goal
- `ReduceDanger` is satisfied when no current attackers remain and derived danger is below the high band
- `ProduceCommodity` is satisfied when the relevant production step completes
- `SellCommodity` is satisfied when the sell step completes

Planning-state effects for bodily-upkeep actions are intentionally conservative:
- consume / sleep / relieve / wash set the relevant drive below the medium band, not necessarily to zero
- healing reduces the pain summary toward below-medium, not to “fully healthy”

This avoids requiring exact physiology simulation inside the planner while still enabling intelligent choices.

## Search Algorithm

### Algorithm
For each top-ranked grounded candidate:
1. build `PlanningSnapshot`
2. run deterministic bounded best-first search over `PlanningState`
3. successors come only from `get_affordances(&planning_state, actor, registry)`
4. filter successors to the current goal’s relevant `PlannerOpKind`s
5. apply planner semantics to get next state
6. stop when:
   - the goal is satisfied, or
   - a valid progress barrier for that same top-level goal is reached

Phase 2 does not do unconstrained generic search across every action family.

### Budgets
Introduce explicit AI planning budget config:

    pub struct PlanningBudget {
        pub max_candidates_to_plan: u8,
        pub max_plan_depth: u8,
        pub max_node_expansions: u16,
        pub beam_width: u8,
        pub switch_margin_permille: Permille,
        pub transient_block_ticks: u32,
        pub structural_block_ticks: u32,
    }

Initial prototype defaults:
- `max_candidates_to_plan = 4`
- `max_plan_depth = 6`
- `max_node_expansions = 128`
- `beam_width = 8`
- `switch_margin_permille = 100`   // 10%
- short / long blocked-intent TTLs are AI constants

These are engineering budgets, not world-simulation laws.

### Budget Exhaustion Rule
If the planner exhausts budget for a candidate:
- that candidate yields no plan this tick
- do not return a partial invalid plan
- try the next candidate
- if all candidates fail and no current valid plan remains, the agent idles this tick

### Mid-Plan Costing
For each successor step:
- `estimated_ticks` must come from `view.estimate_duration(action_def.duration, ...)`
- if duration estimation is unavailable for a mid-plan action, the branch is invalid
- indefinite combat actions are leaf-only and may still be selected as final commitment steps

## Plan Selection

Choose the final plan using deterministic lexicographic ordering:
1. highest `GoalPriorityClass`
2. highest `motive_score`
3. lowest `total_estimated_ticks`
4. deterministic step-sequence ordering

Do not replace a current valid plan unless:
- the current plan became invalid, or
- a new plan is in a strictly higher `GoalPriorityClass`, or
- a same-class new plan beats the current one by `switch_margin_permille`

This avoids thrashing.

## Plan Revalidation

Before executing the next step:
- recompute `get_affordances(view, actor, registry)`
- convert each affordance to the same identity key:
  - `def_id`
  - `targets`
  - `payload_override`
- the planned step is valid only if an identical affordance is still present

This is the canonical revalidation rule.
Do not duplicate precondition logic in E13.

## Failure Handling

### Full Replan Rule
If a step fails revalidation or execution:
- drop the remaining plan
- preserve or clear the top-level goal depending on whether it is still grounded
- mark the agent dirty
- replan from scratch on the next decision pass

Phase 2 does not splice or salvage the remaining tail.

### Deriving BlockingFact
On failure, derive `BlockingFact` in this order:
1. inspect current beliefs against the planned step’s targets
2. inspect whether the relevant affordance family still exists but with different targets
3. use `ReplanNeeded.reason` as a hint
4. fall back to `Unknown`

Examples:
- planned seller still exists but no longer has stock -> `SellerOutOfStock`
- workstation reservation now conflicts -> `ReservationConflict`
- source exists but quantity is zero -> `SourceDepleted`
- target entity dead / gone -> `TargetGone`

Write the resulting `BlockedIntent` into `BlockedIntentMemory`.

### Clearing BlockedIntent
A blocked intent clears when:
- it expires by TTL, or
- the believed blocker is no longer true

Examples:
- seller has stock again
- workstation no longer reserved
- source regenerated
- path reopened
- target reappeared

## Reactive Interrupts

Interrupt evaluation runs each tick but replanning does not.

Inputs:
- derived danger pressure
- homeostatic pressures
- pain pressure
- current wound state
- current action `Interruptibility`

Rules:
- `NonInterruptible`: never voluntarily interrupted by E13
- `InterruptibleWithPenalty`: interrupt only for `Critical` danger / self-care, or when the current plan is invalid
- `FreelyInterruptible`: may interrupt for:
  - higher priority-class survival / danger / heal goals
  - same-class superior plan exceeding the switch margin
  - strictly local opportunistic loot only when no self-care or danger goal is `Medium+`

Opportunity interrupts are intentionally narrow in Phase 2.

## Agent Tick Integration

For each `ControlSource::Ai` agent each tick:

1. skip dead agents entirely
2. derive current pressures and danger
3. update dirty flag if any relevant state changed:
   - plan missing
   - plan finished
   - plan invalidated
   - `ReplanNeeded` received
   - place changed
   - inventory / possessions changed
   - wounds changed
   - relevant threshold band changed
   - blocked intent cleared or expired
4. evaluate interrupts against the current action
5. if replanning is required:
   - generate grounded candidates
   - suppress candidates blocked by still-valid `BlockedIntent`
   - rank candidates
   - plan only the top `max_candidates_to_plan`
   - select the best valid plan
6. if a current valid plan exists:
   - revalidate the next step by affordance identity
   - emit `InputKind::RequestAction` for that step when appropriate
7. if no valid plan exists:
   - emit no input this tick

Idle is explicit Phase 2 behavior.
It is not an action.
It is the lawful result of “no grounded, non-blocked, valid plan exists right now.”

## Component Registration

Register in `component_schema.rs`:
- `UtilityProfile` on `EntityKind::Agent`
- `BlockedIntentMemory` on `EntityKind::Agent`

Do **not** register:
- current plan
- search frontier
- candidate scores
- `PlanningSnapshot`
- `PlanningState`

Those are transient AI runtime data.

## Principle Alignment

### Principle 1: Maximal Emergence Through Causality
The agent loop is grounded in current beliefs, local state, and concrete action affordances.
No authored sequences.

### Principle 2: No Magic Numbers
No abstract world-side danger / greed / enterprise scores are stored.
AI engineering budgets are config, not simulated world causes.

### Principle 3: Concrete State Over Abstract Scores
Stored:
- utility weights
- blocked intent memory
- real needs / wounds / demand memory / stock / cargo / sources

Derived:
- pain
- danger
- enterprise opportunity signal
- candidate set
- plan scores

### Principle 7: Locality of Information
All decision reads still go through `BeliefView`.
`OmniscientBeliefView` is temporary scaffolding only.
Its implementations of hostility / sellers / sources must still use local semantics, not silent global wish fulfillment.

### Principle 8: Dampeners for Positive Feedback
Required dampeners:
- enterprise loop dampened by needs, carry limits, travel time, finite stock, reservations
- combat-success loop dampened by wounds, danger interrupts, finite loot, and leaf-only combat planning
- replan loop dampened by `BlockedIntentMemory`, plan-switch margin, and event-driven replanning
- resource-depletion loop dampened by finite sources, regeneration, travel time, and rising bodily pressures

### Principles 9 and 10
Same action legality and effect pipeline for AI and human.
AI reasoning explained as:
“Agent X chose Y because they believed Z.”

### Principle 11
Diversity comes from:
- `UtilityProfile`
- `DriveThresholds`
- `KnownRecipes`
- `MerchandiseProfile`
- `DemandMemory`
- `BlockedIntentMemory`
- different current beliefs once E14 lands

### Principle 12
E13 does not call system logic directly.
It reads shared state through `BeliefView` and emits `InputEvent`s.

## Known Phase 2 Limitations
These are acceptable and must be documented:
- omniscient adapter means no true information asymmetry yet
- no exploration motivation from ignorance
- no rumor or witness propagation yet
- no lawful group planning yet
- no equipment-readiness reasoning yet
- no reporting / denouncement behavior yet

E13 must not pretend those systems already exist.

## Tests

- [ ] All E13 reads go through `&dyn BeliefView`; no planner code reads `&World`
- [ ] `OmniscientBeliefView` implements all new belief methods
- [ ] `PlannedStep` stores exact ordered `targets` and exact `payload_override`
- [ ] `PlannedStep` converts losslessly to `InputKind::RequestAction`
- [ ] Revalidation is performed by affordance identity, not by duplicate precondition code
- [ ] Search nodes do not clone whole-world state
- [ ] `PlanningState` is transient and never stored as authoritative world state
- [ ] Candidate generation emits only goals grounded by current believed evidence
- [ ] Enterprise goals never outrank `Critical` self-care or danger goals
- [ ] `visible_hostiles_for()` and `current_attackers_of()` are local in semantics
- [ ] Danger is derived from local hostile evidence, never from a stored fear scalar
- [ ] Planner uses only relevant `PlannerOpKind`s for each goal family
- [ ] Planner honors materialization barriers and preserves the top-level goal across them
- [ ] Budget exhaustion returns no invalid partial plan
- [ ] Failure writes a concrete `BlockedIntent`
- [ ] `BlockedIntent` suppresses immediate retries against the same blocker
- [ ] `BlockedIntent` clears on world change or expiry
- [ ] Current valid plan is not replaced by same-class noise unless it beats the switch margin
- [ ] `TradeAcquire`, `Harvest`, `Craft`, and similar barrier steps trigger follow-up replanning as specified
- [ ] Agents prefer controlled stock before external procurement
- [ ] Merchants can fetch their own off-site stock before buying or producing replacement stock
- [ ] Dead agents generate no goals or plans
- [ ] Human / AI control swap preserves the same legal action set and effect pipeline

## Phase 2 Gate
After E13, verify:
- [ ] agents autonomously eat, drink, sleep, wash, and relieve themselves
- [ ] agents can acquire needed goods when concrete reachable paths exist
- [ ] merchants restock via lawful physical paths
- [ ] agents do not thrash between equal plans
- [ ] agents do not retry the same blocked target every tick
- [ ] the world runs for 24+ in-world hours without AI deadlock
- [ ] survival-critical needs consistently outrank commerce and loot
- [ ] combat may be chosen as a leaf commitment, but never requires fake post-combat forecasting

## Acceptance Criteria
- Unified loop:
  beliefs -> grounded candidates -> ranked candidates -> bounded plan search -> exact affordance-keyed execution
- All AI reads go through `&dyn BeliefView`
- `get_affordances()` is the only legality / successor source
- Plans store exact ordered targets, not semantic placeholders
- Planner uses a local semantics table because the engine does not yet expose declarative effects
- Planning is bounded, deterministic, and event-driven
- Failure loops are damped by concrete memory
- Enterprise is grounded in stock, demand, and reachable paths
- No future-epic goals are smuggled into Phase 2
- Current plans are runtime state, not authoritative world state
- `OmniscientBeliefView` remains explicitly temporary scaffolding only
