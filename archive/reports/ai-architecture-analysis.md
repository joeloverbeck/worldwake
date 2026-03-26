**Status**: COMPLETED

# Worldwake AI Architecture: Technical Analysis Report

**Date**: 2026-03-22
**Scope**: Complete AI decision architecture as evidenced by golden E2E test suite (139 tests) and source code analysis
**Audience**: External LLM researcher evaluating architectural quality

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Per-Tick Decision Pipeline](#2-per-tick-decision-pipeline)
3. [Key Abstractions and Data Structures](#3-key-abstractions-and-data-structures)
4. [Candidate Generation](#4-candidate-generation)
5. [Goal Ranking and Pressure System](#5-goal-ranking-and-pressure-system)
6. [GOAP Plan Search](#6-goap-plan-search)
7. [Plan Selection and Journey Commitment](#7-plan-selection-and-journey-commitment)
8. [Goal Switching and Interrupt Evaluation](#8-goal-switching-and-interrupt-evaluation)
9. [Failure Recovery and Blocked Intent Memory](#9-failure-recovery-and-blocked-intent-memory)
10. [Belief View Constraint](#10-belief-view-constraint)
11. [Affordance System](#11-affordance-system)
12. [Action Framework Integration](#12-action-framework-integration)
13. [What the Golden Tests Prove](#13-what-the-golden-tests-prove)
14. [Architectural Observations](#14-architectural-observations)

---

## 1. System Overview

Worldwake is a causality-first emergent micro-world simulation. The AI system drives autonomous agent behavior in a discrete-tick, place-graph world where all state changes propagate through an append-only event log.

**Core design constraints:**
- **Belief-only planning**: Agents never read authoritative world state directly. All perception flows through a `BeliefView` interface.
- **Determinism**: `ChaCha8Rng` seeded randomness, `BTreeMap`/`BTreeSet` only (no `HashMap`), no floats, no wall-clock time.
- **Conservation**: Items cannot be created or destroyed except through explicit actions; enforced by `verify_conservation`.
- **Append-only event log**: The causal source of truth. Events link to prior causes, forming an audit trail.
- **No central orchestration**: Agent behavior emerges from individual decision-making against local beliefs.

**Crate structure relevant to AI:**
```
worldwake-core    → IDs, types, ECS store, topology, items, relations
worldwake-sim     → Action framework, scheduler, tick loop, belief view trait
worldwake-systems → Domain action handlers (eat, harvest, trade, combat, travel, etc.)
worldwake-ai      → Decision architecture (this report's focus)
```

The AI crate (`worldwake-ai`) contains ~15 modules totaling approximately 5,000–6,000 lines of Rust. It implements a pressure-based GOAP (Goal-Oriented Action Planning) architecture with goal ranking, multi-step plan search, journey commitment tracking, and failure recovery.

---

## 2. Per-Tick Decision Pipeline

Each tick, the simulation calls `step_tick()` which invokes the AI driver (`AgentTickDriver`) to produce inputs for each autonomous agent. The driver processes agents in deterministic order (BTreeMap iteration over `EntityId`).

### Complete Pipeline Flow

```
step_tick() [tick_step.rs]
  │
  ├─ 1. produce_tick_inputs()     ← AI generates InputEvents
  ├─ 2. abort_actions_for_dead()  ← Safety: dead actors can't continue
  ├─ 3. process_inputs()          ← RequestAction / CancelAction
  ├─ 4. progress_active_actions() ← Advance each active action 1 tick
  ├─ 5. run_systems()             ← Metabolic drain, resource regen, needs
  ├─ 6. abort_actions_for_dead()  ← Post-system safety
  ├─ 7. emit_end_of_tick_marker()
  └─ 8. increment_tick()
```

### Per-Agent Decision Flow (inside `process_agent()`)

The `AgentTickDriver.process_agent()` method in `agent_tick.rs` (~950 lines) implements the core decision loop. For each agent per tick:

```
process_agent(agent) [agent_tick.rs]
  │
  ├─ PHASE 0: EARLY RETURNS
  │   └─ Skip if agent is dead
  │
  ├─ PHASE 1: IN-FLIGHT RECONCILIATION
  │   ├─ Sync runtime with committed/aborted actions from scheduler
  │   ├─ Handle replan signals (ReplanNeeded from failed actions)
  │   ├─ Record action start failures
  │   └─ Update materialization bindings from committed outcomes
  │
  ├─ PHASE 2: READ PHASE (Belief Snapshot & Ranking)
  │   ├─ Build PerAgentBeliefView from world + agent's belief store
  │   ├─ Clear resolved blockers from BlockedIntentMemory
  │   ├─ Generate goal candidates
  │   ├─ Rank candidates by priority class + motive score
  │   ├─ Build DecisionContext (self-care pressure, danger level)
  │   └─ Detect "dirtiness" (observation changes since last tick)
  │
  ├─ PHASE 3A: ACTIVE ACTION PATH (agent is mid-action)
  │   ├─ Revalidate current plan's next step against current affordances
  │   ├─ Evaluate interrupt triggers against ranked challengers
  │   └─ Decision: continue current action OR interrupt for replan
  │
  ├─ PHASE 3B: PLANNING PATH (agent is idle)
  │   ├─ Check plan continuation optimization (snapshot-only change)
  │   ├─ Build PlanningSnapshot from merged evidence sets
  │   ├─ Search for plans for top N candidates (budget-limited)
  │   └─ Select best plan (with goal-switching logic)
  │
  ├─ PHASE 4: EXECUTION
  │   ├─ Resolve planning targets to authoritative entities
  │   ├─ Revalidate next step against current affordances
  │   ├─ If valid: enqueue RequestAction input to scheduler
  │   └─ If invalid: handle_plan_failure → record BlockedIntent
  │
  └─ PHASE 5: FINALIZATION
      ├─ Update observation snapshots (needs, wounds, inventory sigs)
      └─ Update blocked memory timestamps
```

**Source**: `crates/worldwake-ai/src/agent_tick.rs`

---

## 3. Key Abstractions and Data Structures

## Outcome

- Completion date: 2026-03-26
- What actually changed: Archived this report as historical analysis for a superseded AI architecture version and moved it to `archive/reports/`.
- Deviations from original plan: None. The report content was preserved; only archival metadata was added.
- Verification results: Confirmed the archived file exists under `archive/reports/` and the original path no longer exists under `reports/`.

### AgentDecisionRuntime (`decision_runtime.rs`)

Persistent per-agent state stored **outside** the ECS world (in `AgentTickDriver.runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>`). This is explicitly NOT a world component — it persists across ticks but is not part of the authoritative simulation state.

```rust
pub struct AgentDecisionRuntime {
    // Current plan state
    pub current_goal: Option<GoalKey>,
    pub current_plan: Option<PlannedPlan>,
    pub current_step_index: usize,
    pub step_in_flight: bool,

    // Journey commitment tracking
    pub journey_committed_goal: Option<GoalKey>,
    pub journey_committed_destination: Option<EntityId>,
    pub journey_commitment_state: JourneyCommitmentState, // Active | Suspended
    pub journey_established_at: Option<Tick>,
    pub journey_last_progress_tick: Option<Tick>,
    pub consecutive_blocked_leg_ticks: u32,
    pub last_journey_clear_reason: Option<JourneyClearReason>,

    // Observation snapshots (for dirtiness detection)
    pub last_needs: Option<HomeostaticNeeds>,
    pub last_wounds: Vec<Wound>,
    pub last_commodity_signature: Vec<(CommodityKind, Quantity)>,
    pub last_unique_item_signature: Vec<(UniqueItemKind, u32)>,
    pub last_facility_access_signature: Vec<(EntityId, bool, Option<ActionDefId>)>,
    pub last_effective_place: Option<EntityId>,
    pub last_priority_class: Option<GoalPriorityClass>,

    // Plan execution state
    pub dirty: bool,
    pub materialization_bindings: MaterializationBindings,
    pub queued_facility_intents: BTreeMap<EntityId, QueuedFacilityIntent>,
}
```

**Observation**: The runtime carries 17+ fields of persistent state per agent. The `dirty` flag is a single boolean that collapses multiple possible change reasons into one signal, though expanded `DirtyReason` variants exist for tracing.

### GroundedGoal

A goal candidate generated fresh each tick, consisting of:
- `key: GoalKey` — unique identity (goal kind + parameters, e.g., ConsumeOwnedCommodity(Bread))
- `evidence_entities: BTreeSet<EntityId>` — entities supporting this candidate
- `evidence_places: BTreeSet<EntityId>` — places supporting this candidate

### RankedGoal

Wraps `GroundedGoal` with scoring:
- `priority_class: GoalPriorityClass` — Background | Low | Medium | High | Critical
- `motive_score: u32` — urgency magnitude (typically 100s–1000s)
- `provenance: Option<RankedGoalProvenance>` — diagnostic tracking of how score was derived

### PlannedPlan

A multi-step plan produced by GOAP search:
- `goal: GoalKey` — which goal this plan satisfies
- `steps: Vec<PlannedStep>` — ordered sequence of actions
- `terminal_kind: PlanTerminalKind` — why the plan ends (GoalSatisfied, MaterializationBarrier, etc.)

### PlannedStep

A single step in a plan:
- `def_id: ActionDefId` — which action to execute
- `targets: Vec<PlanningEntityRef>` — either Authoritative(EntityId) or Hypothetical(HypotheticalEntityId)
- `op_kind: PlannerOpKind` — semantic classification (Travel, Consume, Trade, Harvest, Craft, etc.)
- `payload_override: Option<ActionPayload>` — domain-specific parameters
- `estimated_ticks: u32` — duration estimate used for plan cost
- `is_materialization_barrier: bool` — true if this step creates new entities

### PlannerOpSemantics (`planner_ops.rs`)

Maps action definitions to planning-level semantics:
```rust
pub struct PlannerOpSemantics {
    pub op_kind: PlannerOpKind,
    pub may_appear_mid_plan: bool,
    pub is_materialization_barrier: bool,
    pub transition_kind: PlannerTransitionKind,
    pub relevant_goal_kinds: &'static [GoalKindTag],
}
```

There are 21 `PlannerOpKind` variants: Travel, Consume, Sleep, Relieve, Wash, Trade, QueueForFacilityUse, Harvest, Craft, MoveCargo, Heal, Loot, Bury, Tell, ConsultRecord, Attack, Defend, Bribe, Threaten, DeclareSupport, PressForceClaim, YieldForceClaim.

Each variant declares which goal families it is relevant to via static `&[GoalKindTag]` arrays. For example, Travel is relevant to 14 goal kinds (almost all); Consume is relevant only to ConsumeOwnedCommodity.

### PlanningSnapshot (`planning_snapshot.rs`)

A deterministic read-only copy of all relevant world state for planning. Built once per planning phase from the merged evidence sets of all top candidates (avoiding N separate snapshot rebuilds). Includes topology for heuristic travel distance calculation.

### PlanningState (`planning_state.rs`)

Mutable simulation state used during GOAP search. Tracks hypothetical overrides (e.g., "after I travel here, I'll be at place X"), shadow inventories, and removed entities. This enables the search to simulate multi-step plans without mutating the real world.

### PlanningBudget (`budget.rs`)

Tunable constraints governing planning cost:
```rust
pub struct PlanningBudget {
    pub max_candidates_to_plan: u8,       // 4
    pub max_plan_depth: u8,               // 8 steps max
    pub snapshot_travel_horizon: u8,      // 6 hops
    pub max_prerequisite_locations: u8,   // 3
    pub max_node_expansions: u16,         // 512
    pub beam_width: u8,                   // 8
    pub switch_margin_permille: Permille, // 100 (10%)
    pub transient_block_ticks: u32,       // 20
    pub structural_block_ticks: u32,      // 200
}
```

---

## 4. Candidate Generation

`generate_candidates_with_travel_horizon()` in `candidate_generation.rs` enumerates goal candidates from the agent's beliefs each tick.

### Candidate Sources

| Source | Goal Kinds Generated |
|--------|---------------------|
| Homeostatic needs (hunger, thirst, fatigue, bladder, dirtiness) | ConsumeOwnedCommodity, AcquireCommodity, Sleep, Relieve, Wash |
| Pain/wounds | TreatWounds (self or other) |
| Danger (attackers, hostiles) | EngageHostile, ReduceDanger |
| Enterprise signals (merchant restock gaps) | RestockCommodity, SellCommodity, ProduceCommodity |
| Social signals (relayable beliefs, listener knowledge) | ShareBelief (Tell) |
| Political signals (office vacancies, institutional beliefs) | ClaimOffice, SupportCandidateForOffice |
| Corpse proximity | LootCorpse, BuryCorpse |

### Filtering Mechanisms

1. **BlockedIntentMemory**: Goals that recently failed are temporarily suppressed (TTL-based).
2. **Travel horizon**: Candidate search is limited to places within N travel steps of the agent.
3. **Evidence collection**: Each candidate tracks which entities and places support it, enabling targeted snapshot construction.
4. **Zero-motive filter**: Goals with zero urgency score are excluded from planning (no motive → no action).

### Evidence Merging

When building the planning snapshot, evidence sets from the top N candidates are merged into a single set. This means the snapshot is built once for all candidates rather than per-candidate, which is efficient but means the snapshot may contain entities only relevant to lower-priority goals.

---

## 5. Goal Ranking and Pressure System

### Pressure Derivation (`pressure.rs`)

Two pressure dimensions are computed per agent per tick:

**Pain pressure** (`derive_pain_pressure`): Sum of all wound severities (each wound contributes its `severity: Permille`, saturating at 1000).

**Danger pressure** (`derive_danger_pressure`): Situational threat level computed from:
- Number of current attackers
- Presence of visible hostiles
- Wound state and incapacitation
- Mapped to threshold bands: 2+ attackers or (1 attacker + wounded) → Critical; 1 attacker or (hostiles + wounded) → High; hostiles visible → Medium

These pressures are classified using per-agent `ThresholdBand` structs with four levels (low, medium, high, critical), producing a `GoalPriorityClass` for each dimension.

### DecisionContext

```rust
pub struct DecisionContext {
    pub max_self_care_class: GoalPriorityClass,  // highest from any need
    pub danger_class: GoalPriorityClass,          // from danger assessment
}
```

Built once per tick in `build_decision_context()` by taking the maximum priority class across all five homeostatic needs and the danger assessment.

### Ranking Pipeline (`rank_candidates()` in `ranking.rs`)

```
For each candidate:
  1. Suppression check:
     - If danger is high and candidate is non-survival → suppress
     - If self-care pressure high and candidate is non-essential → suppress
  2. Priority class assignment:
     - Drive goals: classified by need level against ThresholdBand
     - Danger goals: elevated based on danger assessment
     - Enterprise/social/political: typically Background or Low
  3. Motive score computation:
     - Drive goals: need_intensity × utility_weight (from UtilityProfile)
     - Enterprise: restock gap magnitude
     - Pain/danger: pressure permille value
  4. Output: RankedGoal { priority_class, motive_score, provenance }

Sort by: (priority_class DESC, motive_score DESC)
```

### UtilityProfile

Per-agent utility weights that create behavioral diversity:
- `hunger_weight`, `thirst_weight`, `fatigue_weight`, `bladder_weight`, `dirtiness_weight`
- `pain_weight`, `danger_weight`
- `enterprise_weight`, `social_weight`, `care_weight`
- `political_weight`

Different agents with different profiles make genuinely different decisions for the same world state — a merchant with high `enterprise_weight` will prioritize restocking over self-care that a farmer with high `hunger_weight` would choose first.

### Suppression

Goals outside survival categories are suppressed when the agent faces high self-care or danger pressure. For example:
- Under hunger pressure, political goals (ClaimOffice) are suppressed
- Under danger, non-combat goals may be suppressed
- Loot/bury goals are suppressed when the agent has unmet self-care needs

Suppressed goals are tracked in `RankingOutcome.suppressed` for diagnostic visibility.

---

## 6. GOAP Plan Search

`search_plan()` in `search.rs` (~880 lines) implements a best-first A*-style search over the action space.

### Search Structure

```
Root Node:
  state = PlanningState (mutable copy of snapshot)
  steps = []
  total_estimated_ticks = 0
  heuristic_ticks = min travel distance to goal-relevant place

Frontier: BinaryHeap<FrontierEntry>
  Ordering: f = g (total_estimated_ticks) + h (heuristic_ticks), lower is better

Loop:
  while frontier not empty AND expansions < max_node_expansions:
    node = frontier.pop()

    if goal.is_satisfied(node.state):
      return Found(plan from node.steps)

    if node.steps.len() >= max_plan_depth:
      continue (prune)

    candidates = get_affordances_for_defs(
      goal-relevant action defs only,
      node.state as belief view
    )

    for each candidate:
      new_state = apply_hypothetical_transition(node.state, candidate)
      new_steps = node.steps + [candidate_as_step]
      new_node = SearchNode { state: new_state, steps: new_steps, ... }
      frontier.push(new_node)

    expansions += 1

  return BudgetExhausted or FrontierExhausted
```

### Key Search Mechanics

**Action filtering by relevance**: Not all 21+ action types are expanded at each node. Each `PlannerOpKind` declares which `GoalKindTag`s it is relevant to. Only actions whose semantics include the current goal's tag are considered. This dramatically prunes the search space.

**Heuristic**: Minimum travel ticks from the agent's current simulated position to the nearest goal-relevant place (from evidence sets). Zero when already at a relevant place. This provides spatial guidance for multi-step plans requiring travel.

**Beam width**: After each expansion round, the frontier is pruned to the top `beam_width` entries. Default is 8. This prevents combinatorial explosion but can miss solutions that require exploring unpopular branches.

**Materialization barriers**: Some actions (e.g., Harvest, Craft) create new entities that don't exist yet in the world. During search, these are represented as `HypotheticalEntityId`s. Steps that create hypothetical entities are marked as `is_materialization_barrier = true`. If the search can't find a fully satisfying plan within budget, it may return the best barrier plan — a partial plan that gets the agent to the materialization point.

**Hypothetical state transitions**: `apply_hypothetical_transition()` updates the `PlanningState` to simulate the effect of an action without touching the real world. For example:
- Travel: moves the actor's simulated position
- Harvest: creates a hypothetical item lot
- Consume: removes a commodity from simulated inventory
- Trade: transfers commodities between simulated agents

**Plan search results**:
```rust
pub enum PlanSearchResult {
    Found(PlannedPlan),
    BudgetExhausted,
    FrontierExhausted,
    Unsupported,
}
```

### Budget Constraints

| Parameter | Default | Effect |
|-----------|---------|--------|
| `max_candidates_to_plan` | 4 | Only top 4 ranked goals get plan searches |
| `max_plan_depth` | 8 | Maximum steps in any plan |
| `max_node_expansions` | 512 | Total frontier expansions before declaring BudgetExhausted |
| `beam_width` | 8 | Frontier pruning limit |
| `snapshot_travel_horizon` | 6 | How many hops away to include in snapshot |
| `max_prerequisite_locations` | 3 | Locations considered for prerequisite acquisition |

---

## 7. Plan Selection and Journey Commitment

### Plan Selection (`select_best_plan()` in `plan_selection.rs`)

After searching for plans for the top N candidates, the system must choose which plan to adopt. The selection respects plan stability (don't thrash between equal options) and journey commitment (don't abandon multi-step travel lightly).

```
1. Collect all (priority_class, motive_score, plan) triples
2. Sort by (priority_class DESC, motive_score DESC)
3. If agent has no current plan → return best available plan

4. If agent has a current plan:
   For each challenger (sorted by score):
     a. If challenger refreshes journey commitment → adopt challenger
     b. If challenger is same goal as current → adopt challenger (refreshed plan)
     c. If relation-aware goal switch passes → adopt challenger

   If no challenger beats current plan, but current goal has no plan → adopt best
   Otherwise → retain current plan (stability)
```

### Journey Commitment

Multi-step travel plans establish a "journey commitment" that raises the threshold for switching to a new goal. This prevents agents from constantly abandoning travel when minor need fluctuations occur.

**Journey state machine**:
- `JourneyCommitmentState::Active` — agent is en route to a committed destination
- `JourneyCommitmentState::Suspended` — journey paused (e.g., eating during travel)

**JourneyPlanRelation** classifies how a new plan relates to the committed journey:
- `NoCommitment` — no active journey
- `RefreshesCommitment` — new plan continues toward same destination
- `SuspendsCommitment` — new plan temporarily diverts (e.g., eat, then resume)
- `AbandonsCommitment` — new plan goes somewhere else entirely

**Journey switch margins**: When a journey is active, the `switch_margin` for same-class goal switching is elevated (from the default 100 permille to a per-agent `route_replan_margin`), making it harder for same-priority challengers to displace the journey.

**Journey clear reasons**: `GoalSatisfied`, `Reprioritized`, `PlanFailed`, `PatienceExhausted`, `Death`, `LostTravelPlan`.

---

## 8. Goal Switching and Interrupt Evaluation

### Goal Switching (`goal_switching.rs`)

The fundamental comparison function:

```rust
fn compare_goal_switch(
    current_class, current_motive,
    challenger_class, challenger_motive,
    margin: Permille,
) -> Option<GoalSwitchKind>
```

Logic:
1. If challenger has **higher priority class** → `HigherPriorityGoal` (always switch)
2. If challenger has **lower priority class** → `None` (never switch)
3. If **same class**: switch only if `challenger_motive >= current_motive + (current_motive × margin / 1000)`

The margin prevents oscillation. Default is 100 permille (10%), meaning a same-class challenger needs 10% higher motive to displace the current goal.

### Interrupt Evaluation (`evaluate_interrupt()` in `interrupts.rs`)

When an agent has an active action, the interrupt system decides whether to abort it:

```
1. If action is NonInterruptible → NoInterrupt

2. If current plan step is invalid → InterruptForReplan(PlanInvalid)

3. Find best challenger (highest-ranked candidate ≠ current goal)

4. Match action interruptibility:

   NonInterruptible:
     → NoInterrupt

   InterruptibleWithPenalty:
     → Only interrupt for Critical-priority challengers
     → Check goal_family_policy.penalty_interrupt eligibility

   FreelyInterruptible:
     → Evaluate relation-aware goal switch
     → Check if challenger has higher priority or clears margin
     → Opportunistic goals (loot) checked for stress level
```

**Interruptibility levels** (declared per-action in `ActionDef`):
- `NonInterruptible` — cannot be interrupted under any circumstances
- `InterruptibleWithPenalty` — only interrupted by Critical survival/danger
- `FreelyInterruptible` — interrupted by any sufficient challenger

---

## 9. Failure Recovery and Blocked Intent Memory

### handle_plan_failure() (`failure_handling.rs`)

When an action fails (start failure, replan signal, or step revalidation failure):

```
1. Clear current plan from runtime
2. Clear journey commitment (JourneyClearReason::PlanFailed)
3. Clear materialization bindings

4. Derive blocking_fact from failure context:
   - TargetGone: target entity no longer exists
   - NoKnownPath: travel to destination impossible
   - SellerUnavailable: trade partner not present
   - PrerequisitesUnfulfilled: craft inputs missing
   - NoConsumableAvailable: no food/medicine/etc.
   - Unknown: fallback for undiagnosed failures

5. Record BlockedIntent:
   { goal_key, blocking_fact, related_entity, related_place,
     observed_tick, expires_tick = current + TTL }

6. Set runtime.dirty = true (triggers replan next tick)
```

### BlockedIntentMemory

A per-agent memory of temporarily blocked goals:
- `intents: Vec<BlockedIntent>` — each with a TTL
- `expire(current_tick)` — removes expired intents
- `clear_resolved_blockers()` — removes intents whose blocking conditions are no longer true (e.g., resource regenerated)

**TTL values** (from PlanningBudget):
- `transient_block_ticks: 20` — for transient failures (resource temporarily unavailable)
- `structural_block_ticks: 200` — for structural failures (no path exists)

### Recovery Loop

The recovery cycle is:
1. Action fails → `handle_plan_failure` records BlockedIntent
2. Next tick → candidate generation filters out blocked goals
3. Agent pursues alternative goals
4. After TTL expires (or blocker resolves) → blocked goal re-enters candidate pool
5. Agent may retry the previously blocked goal

---

## 10. Belief View Constraint

The belief view is the fundamental information boundary between agents and the world.

### Trait Hierarchy

```
RuntimeBeliefView (low-level)
  └─ GoalBeliefView (high-level, used by candidate generation + ranking)
       └─ PerAgentBeliefView (concrete implementation)
```

### PerAgentBeliefView

Three data categories with different trust levels:

| Category | Source | Trust Level |
|----------|--------|-------------|
| Self-authoritative | Direct world read for agent's own state | Always current |
| Observed | AgentBeliefStore (recorded at perception events) | May be stale |
| Public structure | Topology, facilities, resource source types | Immutable |

**Self-authoritative data**: Agent's own position, inventory, homeostatic needs, wounds, combat profile, control source. Read directly from world because an agent always has perfect knowledge of its own state.

**Observed data**: Beliefs about other entities — positions, alive/dead status, inventory estimates. Stored in `AgentBeliefStore` with `BelievedEntityState { alive, last_known_place }`. Updated only when the agent perceives events (witnesses, direct observation, social reports).

**Public structure**: Place graph topology, facility types, resource source locations, workstation tags. Treated as common knowledge (everyone knows the map layout).

### Information Staleness

Beliefs can become stale:
- An agent believes bread exists at the Orchard, but another agent consumed it
- An agent believes a target is at the Village Square, but they traveled away
- An agent's wound belief from a report is insufficient for care (direct observation required)

Golden tests explicitly validate stale-belief scenarios (Scenario 36: stale belief → travel → re-observation → replan).

### Current Limitation: OmniscientBeliefView

A stand-in `OmniscientBeliefView` exists for testing/bootstrapping that provides perfect world knowledge. The `PerAgentBeliefView` is the proper implementation used in production golden tests.

---

## 11. Affordance System

### ActionDef: Declarative Action Blueprints

Every action type is defined declaratively as an `ActionDef` (`action_def.rs`):
- Actor constraints (preconditions on the actor)
- Target specifications (what entities to bind)
- Preconditions (stateful checks on bound targets)
- Reservation requirements (exclusive locks during execution)
- Duration (fixed, dynamic, or travel-time-based)
- Body cost per tick (metabolic drain)
- Interruptibility level
- Commit conditions (must hold at action completion)
- Event tags (causal classification)
- Payload type (domain-specific data)
- Handler ID (execution logic)

### Affordance Generation (`affordance_query.rs`)

`get_affordances()` converts action definitions into concrete executable options:

```
For each ActionDef:
  1. Check actor_constraints against agent's belief view → skip if any fail
  2. Enumerate all target bindings:
     - Each TargetSpec (e.g., EntityAtActorPlace { kind: Facility })
       generates candidate entities from the belief view
     - Only entities matching the constraint are bound
  3. Filter bindings by preconditions (via belief view)
  4. Expand payload variants (handler may generate multiple)
  5. Return sorted, deduped Affordances
```

**Critical**: Affordances are **always evaluated against a belief view**, not the authoritative world. An agent can only choose actions on targets it believes exist.

### Affordance Structure

```rust
pub struct Affordance {
    pub def_id: ActionDefId,
    pub actor: EntityId,
    pub bound_targets: Vec<EntityId>,
    pub payload_override: Option<ActionPayload>,
    pub explanation: Option<String>,
}
```

### Two Uses of Affordances

1. **Authoritative affordances** (at action start): Validated against the real world at the moment of execution. If the world state has changed since planning, the action may fail to start.
2. **Planning affordances** (during search): Validated against the `PlanningSnapshot` or `PlanningState`. Used to explore possible future actions.

This dual-use creates a gap: a plan constructed from planning affordances may fail when the first step is attempted against authoritative affordances. The system handles this via start failure recovery (Section 9).

---

## 12. Action Framework Integration

### Tick Loop Integration

```
step_tick() [tick_step.rs]
  │
  ├─ AI produces InputEvents (RequestAction, CancelAction)
  │
  ├─ Scheduler processes inputs:
  │   ├─ For each RequestAction:
  │   │   ├─ Look up matching affordance (belief-validated)
  │   │   ├─ Call start_action() → validates against real world
  │   │   ├─ If valid: create ActionInstance, insert into scheduler
  │   │   └─ If invalid: record ActionStartFailure
  │   │
  │   └─ For each CancelAction: abort the targeted action
  │
  ├─ Progress active actions:
  │   For each ActionInstance:
  │     ├─ Call handler.tick() → Continue or Complete
  │     ├─ Apply body costs (metabolic drain per tick)
  │     └─ If Complete: call handler.commit() → CommitOutcome
  │         ├─ Apply world mutations via WorldTxn
  │         ├─ Log ActionCommitted event
  │         └─ Record materializations (new entities)
  │
  └─ Run systems:
      ├─ needs_system() — apply homeostatic drain, create deprivation wounds
      ├─ resource_regeneration_system() — regenerate resource sources
      ├─ trade_system_tick() — age merchant demand memories
      └─ run_combat_system() — resolve wounds, bleed damage
```

### Action Handler Lifecycle

Each action handler implements four methods:
- `start()` — initialize action state, validate preconditions
- `tick()` — per-tick progress (return Continue or Complete)
- `commit()` — finalize action, apply mutations, return CommitOutcome
- `abort()` — clean up on interruption

**CommitOutcome** includes:
- `materializations` — new entities created (for binding resolution)
- State changes applied through WorldTxn (atomic commit)

### Scheduler

The `Scheduler` maintains:
- `active_actions: BTreeMap<ActionInstanceId, ActionInstance>`
- `input_queue: InputQueue` (deterministic ordering by tick + sequence)
- `pending_replans: Vec<ReplanNeeded>` (failure signals for AI)
- `committed_actions: Vec<CommittedAction>` (success signals for AI)
- `action_start_failures: Vec<ActionStartFailure>` (rejection signals for AI)

The scheduler is the **only** gateway for action lifecycle management. The AI cannot directly invoke handlers.

---

## 13. What the Golden Tests Prove

### Coverage Statistics

| Metric | Value |
|--------|-------|
| Total proven tests | 139 |
| Test files | 10 (golden_ai_decisions, golden_combat, golden_trade, golden_production, golden_social, golden_care, golden_emergent, golden_offices, golden_supply_chain, golden_determinism) |
| GoalKind coverage | 19/19 (100%) |
| ActionDomain coverage | 11/11 (100%) |
| Homeostatic needs tested | 5/5 |
| Topology places used | 10/12 |
| Cross-system interaction chains | 73+ proven |

### Key Emergent Behaviors Demonstrated

**1. Pressure-driven prioritization**: Pain/danger multiply motive scores; critical wounds suppress politics. Validated in `golden_care.rs` (wound vs hunger priority resolved by UtilityProfile weights) and `golden_offices.rs` (wounded politician ordering).

**2. Agent behavioral diversity**: Different `UtilityProfile` weights produce genuinely different decisions. Validated across care, trade, and political scenarios.

**3. Multi-step plan execution**: Agents execute 3–8 step plans including travel → acquire → craft → return → stock. Validated in `golden_supply_chain.rs` (merchant restock cycles, prerequisite-aware craft-restock).

**4. Goal invalidation**: One agent's success invalidates another's goal; the second agent replans. Validated in `golden_ai_decisions.rs`.

**5. Failure recovery**: Failed starts record BlockedIntents; agents pivot to alternatives; retry after TTL or resource regeneration. Validated in `golden_production.rs` (contested harvest), `golden_trade.rs` (trade start failure recovery).

**6. Journey suspension/resumption**: Multi-hop travel suspended for higher-priority needs, resumed afterward. Validated in `golden_ai_decisions.rs`.

**7. Stale belief correction**: Agent travels to believed resource, finds it depleted, re-observes, replans. Validated in `golden_social.rs`.

**8. Political emergence**: Coalition building through bribery, threats, loyalty, declarations of support — all emerging from individual agent decisions without central orchestration. Validated in `golden_offices.rs` (14+ political scenarios).

**9. Cross-system chains**: Needs → AI → action → production → materialization → transport → consumption. Metabolism → deprivation → wounds → death → loot → self-care. Validated across all test files.

**10. Deterministic replay**: Identical seeds produce identical outcomes. Validated in `golden_determinism.rs`.

### Assertion Hierarchy

Tests use a semantic assertion hierarchy (preferred to weakest):
1. **Request-resolution traces** — Prove pre-start rejection or binding path
2. **Authoritative world state** — Prove durable outcomes (ownership, location, quantities)
3. **Action traces** — Prove lifecycle ordering (started, committed, aborted)
4. **Decision traces** — Prove AI reasoning (candidates, ranking, plan selection)
5. **Event log** — Prove event provenance, tags, visibility

---

## 14. Architectural Observations

This section presents observations about the architecture for external evaluation. These are factual observations, not prescriptive recommendations.

### 14.1 Complexity Distribution

**Observation**: The `process_agent()` function in `agent_tick.rs` is ~950 lines and handles 5 distinct phases. It is the central orchestrator that integrates all other AI modules. Similarly, `search_plan()` in `search.rs` is ~880 lines.

**Implication**: Two functions contain the bulk of the integration logic. Changes to the decision pipeline almost always touch `process_agent()`. The function manages early returns, in-flight reconciliation, snapshot construction, planning, interrupt evaluation, execution, and failure handling in a single control flow.

### 14.2 State Surface Area

**Observation**: `AgentDecisionRuntime` has 17+ fields of persistent state including 6 journey-related fields, 6 observation snapshot fields, materialization bindings, facility queue intents, and the plan/goal/step tracking. The `dirty` flag collapses multiple change reasons into a single boolean.

**Implication**: Per-agent state is rich and growing. Each new feature (journeys, facilities, materializations) has added fields to the runtime. The single `dirty` flag means the system can only answer "something changed" — not "what changed" — outside of tracing mode.

### 14.3 Plan Search Architecture

**Observation**: The GOAP search is a best-first A* with beam width pruning. It operates on a simulated `PlanningState` that tracks hypothetical entity creation, position overrides, and inventory shadows. The heuristic is spatial (minimum travel ticks to goal-relevant place).

**Implication**: The search is goal-directed and spatially guided, but the beam width (default 8) means it can miss solutions requiring exploration of less-immediately-promising branches. The search produces a single linear plan — there is no conditional branching, contingency planning, or plan monitoring during execution beyond step revalidation.

### 14.4 Planning-Execution Gap

**Observation**: Plans are constructed against a `PlanningSnapshot` (belief state at planning time). Execution happens against the authoritative world, which may have changed. The only bridge is step revalidation and start failure recovery.

**Implication**: The system is optimistic-then-recover: plan optimistically, attempt execution, handle failures reactively. This works well for the current domain (small agent counts, moderate plan lengths). The recovery mechanism (BlockedIntentMemory with TTL) is coarse — it blocks entire goals rather than adjusting plan details.

### 14.5 Goal Switching Granularity

**Observation**: Goal switching operates at the goal level, not the plan level. When a challenger goal wins, the entire current plan is abandoned and a new plan is searched. There is no mechanism to merge plans, share subgoals, or interleave actions from different goals.

**Implication**: An agent pursuing "travel to orchard, harvest wheat, travel back, eat" that gets interrupted by thirst will abandon the harvest plan entirely and search for a drink plan from scratch. If the drink happens to be at the orchard, this is wasteful. The journey commitment system partially mitigates this for travel-heavy plans but doesn't generalize to arbitrary plan segments.

### 14.6 Candidate-Ranking-Search Coupling

**Observation**: The pipeline is strictly sequential: generate all candidates → rank all → search plans for top N → select best. Candidates are generated independently (no awareness of what plans are feasible). Ranking is independent of plan existence (a goal scores high even if no plan exists for it).

**Implication**: An agent may spend all 4 planning slots on high-priority goals that turn out to have no feasible plans (BudgetExhausted), while a lower-ranked goal with an obvious 1-step plan goes unplanned. The system handles this by falling back to the current plan or doing nothing, but doesn't adaptively allocate planning budget.

### 14.7 Belief View Completeness

**Observation**: The `PerAgentBeliefView` categorizes data into self-authoritative (always current), observed (may be stale), and public structure (immutable). Self-authoritative data includes the agent's own needs, wounds, inventory, and position.

**Implication**: The self-authoritative category is broad — agents have perfect knowledge of their own detailed internal state. This is a simplification; a more constrained model might have agents discover their own health status through symptoms rather than direct inspection. The immutable public structure assumption means all agents share identical knowledge of the place graph, which may not be desired for exploration or fog-of-war scenarios.

### 14.8 Action Definition Explosion

**Observation**: There are 21 `PlannerOpKind` variants, each with static arrays of relevant `GoalKindTag`s. The mapping from `ActionDef` to `PlannerOpSemantics` is maintained in a `classify_action_def()` function that pattern-matches on `(domain, name)` pairs.

**Implication**: Adding a new action type requires: (1) defining the ActionDef, (2) implementing the handler, (3) adding a PlannerOpKind variant, (4) declaring relevant goal kinds, (5) implementing the hypothetical transition, (6) updating classify_action_def. This is a 6-point integration for each new action, spread across multiple modules and crates.

### 14.9 Observation Snapshot Drift Detection

**Observation**: The dirtiness detection system compares current observations (needs, wounds, commodity signatures, facility access) against snapshots from the previous tick. If nothing has changed AND the top goal is the same, the system can skip full replanning and just revalidate the current plan's next step.

**Implication**: This is a performance optimization that avoids redundant replanning when the world is stable. However, the comparison is field-by-field on pre-selected observation dimensions. If a new dimension is added to the world (e.g., reputation, social standing) without updating the snapshot fields, dirtiness detection will miss it.

### 14.10 Module Dependency Structure

```
agent_tick.rs (orchestrator)
  ├─ candidate_generation
  ├─ ranking
  ├─ search
  │   ├─ planning_snapshot
  │   ├─ planning_state
  │   ├─ planner_ops
  │   └─ goal_model
  ├─ plan_selection
  ├─ interrupts
  │   ├─ goal_switching
  │   ├─ journey_switch_policy
  │   └─ goal_policy
  ├─ failure_handling
  ├─ plan_revalidation
  ├─ pressure
  ├─ enterprise
  └─ decision_runtime
```

**Observation**: The modules form a shallow hierarchy with `agent_tick` as the sole top-level integrator. There is no intermediate abstraction layer between `agent_tick` and the individual pipeline stages. All module outputs flow upward to `agent_tick`, which sequences them.

**Implication**: The architecture is a pipeline, not a framework. Adding a new pipeline stage (e.g., plan monitoring, social reasoning, emotional state) requires modifying `agent_tick` directly. There is no plugin or extension mechanism for the decision pipeline.

### 14.11 Blocked Intent Recovery Resolution

**Observation**: `clear_resolved_blockers()` checks whether a blocking condition has been resolved (e.g., resource regenerated, target reappeared) and removes the blocked intent early. The check is per-`BlockingFact` variant with custom resolution logic.

**Implication**: The resolution check is proactive (runs every tick) and fact-specific. Adding a new `BlockingFact` variant requires adding corresponding resolution logic. The TTL fallback ensures eventual recovery even if resolution logic is incomplete, but the `BlockingFact::Unknown` variant (fallback for undiagnosed failures) has no resolution logic — it simply expires after `structural_block_ticks` (200 ticks).

### 14.12 Political and Social Emergence

**Observation**: Political behavior (office claims, coalition building, succession) and social behavior (belief sharing, gossip chains) emerge from the same candidate → rank → plan → execute pipeline as physiological needs. There is no special "political AI" or "social AI" — these are just goal kinds with enterprise/social/political weights.

**Implication**: This is architecturally elegant — all behavior uses the same framework. However, political reasoning (e.g., "should I support candidate A or B?") is reduced to utility weight comparison rather than strategic reasoning about alliances, loyalty, or long-term consequences. The system can produce political behavior but cannot reason about political strategy.

### 14.13 Scalability Considerations

**Observation**: Per-agent per-tick, the pipeline performs: candidate generation (iterates over beliefs), ranking (iterates over candidates), plan search (up to 512 node expansions per candidate × 4 candidates = 2048 max), plan selection, and step execution. PlanningSnapshot construction merges evidence from all top candidates.

**Implication**: The per-agent cost is bounded by budget parameters but grows with world complexity (more entities → more affordances per expansion → more candidates to evaluate). The system is designed for small-to-moderate agent counts. No observed parallelism between agents — they are processed sequentially in BTreeMap order.

### 14.14 Diagnostic Infrastructure

**Observation**: The system includes comprehensive diagnostic infrastructure:
- `DecisionTraceSink` — records full per-agent per-tick decision pipeline (candidates, ranking, planning, selection, outcome)
- `ActionTraceSink` — records action lifecycle events (Started, Committed, Aborted, StartFailed)
- Both are opt-in and zero-cost when disabled

**Implication**: The diagnostic tooling is mature and well-integrated. Golden tests routinely use traces to validate internal reasoning, not just outcomes. This is a strength — it makes the decision pipeline observable and debuggable.

---

## Appendix A: File Reference

| Module | File | Approx. Lines | Purpose |
|--------|------|---------------|---------|
| agent_tick | `crates/worldwake-ai/src/agent_tick.rs` | ~950 | Per-agent per-tick orchestrator |
| search | `crates/worldwake-ai/src/search.rs` | ~880 | GOAP plan search |
| candidate_generation | `crates/worldwake-ai/src/candidate_generation.rs` | ~600 | Goal enumeration from beliefs |
| ranking | `crates/worldwake-ai/src/ranking.rs` | ~300 | Priority/motive scoring |
| plan_selection | `crates/worldwake-ai/src/plan_selection.rs` | ~80 | Best plan choice |
| interrupts | `crates/worldwake-ai/src/interrupts.rs` | ~200 | Active-action interrupt eval |
| goal_switching | `crates/worldwake-ai/src/goal_switching.rs` | ~80 | Priority comparison logic |
| failure_handling | `crates/worldwake-ai/src/failure_handling.rs` | ~200 | Blocked intent recording |
| decision_runtime | `crates/worldwake-ai/src/decision_runtime.rs` | ~200 | Persistent per-agent state |
| planner_ops | `crates/worldwake-ai/src/planner_ops.rs` | ~400 | Action semantics for planning |
| planning_snapshot | `crates/worldwake-ai/src/planning_snapshot.rs` | ~300 | Belief state snapshot |
| planning_state | `crates/worldwake-ai/src/planning_state.rs` | ~400 | Mutable search state |
| pressure | `crates/worldwake-ai/src/pressure.rs` | ~100 | Pain/danger derivation |
| budget | `crates/worldwake-ai/src/budget.rs` | ~60 | Planning constraints |
| enterprise | `crates/worldwake-ai/src/enterprise.rs` | ~200 | Merchant logic |
| affordance_query | `crates/worldwake-sim/src/affordance_query.rs` | ~400 | Affordance generation |
| belief_view | `crates/worldwake-sim/src/belief_view.rs` | ~200 | Belief trait definitions |
| tick_step | `crates/worldwake-sim/src/tick_step.rs` | ~300 | Tick execution loop |

## Appendix B: Golden Test File Map

| File | Tests | Focus |
|------|-------|-------|
| golden_ai_decisions.rs | Core AI | Goal invalidation, interrupts, blocked intent, deprivation cascades |
| golden_combat.rs | Combat | Death, loot, wound lifecycle, loot suppression |
| golden_trade.rs | Trade | Negotiation, merchant restock, craft-restock, start-failure recovery |
| golden_production.rs | Production | Multi-recipe craft, remote acquisition, carry capacity |
| golden_social.rs | Social | Tell chains, memory expiry, entity discovery, stale belief correction |
| golden_care.rs | Care | Wound/hunger priority, self-care, healer travel, deprivation worsening |
| golden_emergent.rs | Emergence | Loot→care chains, rumor→travel→discovery, recovery-aware promotion |
| golden_offices.rs | Politics | Office claims, coalitions, bribery, succession, remote discovery |
| golden_supply_chain.rs | Economy | Merchant restock, consumer trade, craft-restock |
| golden_determinism.rs | Determinism | Replay validation |
