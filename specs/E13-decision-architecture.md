# E13: Agent Decision Architecture

## Epic Summary
Implement the unified agent AI: utility scoring with `Permille`, goal selection, GOAP-style planner, plan revalidation, reactive executor, and belief-only planning. All code MUST use `&dyn BeliefView` so E14 can substitute per-agent beliefs.

## Phase
Phase 2: Survival & Logistics (final epic before Phase 2 gate)

## Crate
`worldwake-ai`

## Dependencies
- E09 (needs system: `HomeostaticNeeds`, `AgentCondition`, `MetabolismProfile`)
- E10 (production: `RecipeDefinition`, `CarryCapacity` — goals to plan over)
- E11 (trade: negotiation — goals to plan over)
- E12 (combat: `WoundList`, `CombatProfile` — goals to plan over)

## Deliverables

### UtilityProfile Component
Per-agent utility weights enabling Principle 11 (agent diversity). All weights are `Permille`:
- `hunger_weight: Permille` — importance of addressing hunger
- `thirst_weight: Permille` — importance of addressing thirst
- `fatigue_weight: Permille` — importance of addressing fatigue
- `bladder_weight: Permille` — importance of addressing bladder
- `dirtiness_weight: Permille` — importance of addressing dirtiness
- `pain_weight: Permille` — importance of addressing pain
- `fear_weight: Permille` — importance of fleeing/avoiding danger
- `greed_weight: Permille` — importance of acquiring coin (applied to coin deficit derived from holdings)
- `sociability_weight: Permille` — importance of social interaction

Different agents have different `UtilityProfile` values seeded at creation. Two guards with identical beliefs may choose different goals because their weights differ.

### UrgencyThresholds Component
Per-agent urgency thresholds (all `Permille`):
- `low: Permille` — minor discomfort, may seek to address
- `medium: Permille` — significant pressure, will prioritize
- `high: Permille` — urgent, overrides most other goals
- `critical: Permille` — emergency, overrides everything except immediate danger

No hardcoded global threshold constants. Referenced by E09 consumption action preconditions.

### Utility Scoring
- `score(need: Permille, threshold: Permille) -> Permille`
  - Higher score = more urgent
- Each homeostatic need (hunger, thirst, fatigue, bladder, dirtiness) produces a score weighted by the agent's `UtilityProfile`
- Each condition (pain, fear) produces a score weighted by the agent's `UtilityProfile`
- Additional pressures:
  - `greed_weight` applied to coin deficit (derived from holdings — not a stored need)
  - `sociability_weight` applied to social interaction needs
- **Loyalty is NOT a utility input** — it is a relation in `RelationTables`, not a pressure to be scored
- **social_standing is NOT a utility input** — it is derived from reputation events (future system)
- **wealth_pressure is NOT a utility input** — coin deficit is derived from holdings

### OmniscientBeliefView as Temporary P10 Violation
`OmniscientBeliefView` (from `crates/worldwake-sim/src/omniscient_belief_view.rs`) is an explicit, documented temporary violation of Principle 10 (Intelligent Agency Over Behavioral Scripts). It provides omniscient world state access where per-agent beliefs should be used.

**All E13 code MUST use `&dyn BeliefView`**, never `&World` or concrete types. This ensures E14 can substitute per-agent belief stores without changing E13 code. The `OmniscientBeliefView` adapter is used only at the call site in `tick_step.rs`.

### Required BeliefView Trait Extensions
The existing `BeliefView` trait (in `crates/worldwake-sim/src/belief_view.rs`) must be extended with:
- `homeostatic_needs(agent: EntityId) -> Option<HomeostaticNeeds>` — agent's believed needs
- `agent_condition(agent: EntityId) -> Option<AgentCondition>` — agent's believed pain/fear
- `wounds(agent: EntityId) -> Vec<Wound>` — agent's believed wound state
- `agents_selling_at(place: EntityId, commodity: CommodityKind) -> Vec<EntityId>` — believed sellers at a location
- `available_production_slots(facility: EntityId) -> Option<u32>` — believed open production slots

These extensions allow E13 to query agent state through beliefs, not world state.

### Goal Selection
- `select_goal(agent: EntityId, view: &dyn BeliefView, profile: &UtilityProfile, thresholds: &UrgencyThresholds) -> Goal`
- Highest-utility goal becomes current objective
- Goal re-evaluation on: tick interval, replan signal, major state change

### Goal Catalog
Per spec section 6.2. Each goal references `ActionDefId` values, not prose descriptions:
- `Eat` — address hunger → maps to `ActionDefId::Eat`
- `Drink` — address thirst → maps to `ActionDefId::Drink`
- `Sleep` — address fatigue → maps to `ActionDefId::Rest`
- `Relieve` — address bladder → maps to `ActionDefId::Toilet`
- `Wash` — address dirtiness → maps to `ActionDefId::Wash`
- `Trade` — buy/sell goods → maps to `ActionDefId::Negotiate`
- `Restock` — merchant restocking → maps to sequence: `Travel` → `Negotiate` → `Travel`
- `Escort` — escort cargo/caravan → maps to sequence: `PickUp` → `Travel` → `PutDown`
- `Raid` — raid caravan/travelers → maps to `ActionDefId::Attack`
- `Flee` — escape danger → maps to `ActionDefId::Travel` (to safe location)
- `ClaimOffice` — claim vacant office → maps to `ActionDefId::ClaimOffice`
- `SupportClaimant` — support another's claim → maps to `ActionDefId::SupportClaimant`
- `Heal` — seek or provide healing → maps to `ActionDefId::Heal`
- `BuryCorpse` — handle dead body → maps to `ActionDefId::Bury`
- `EstablishCamp` — set up new camp → maps to `ActionDefId::EstablishCamp`

### GOAP-Style Planner
Per spec section 6.3:
- Operates on compact `PlanningState` struct (derived from `BeliefView`, never stored as authoritative state)
- `PlanningState` contains: agent's believed needs, condition, inventory, location, nearby entities — all queried from `&dyn BeliefView`
- Input: goal + agent's `PlanningState`
- Output: ordered sequence of `ActionDefId` values to achieve goal
- Search: forward state-space search with action effects
- Pruning: only consider actions whose preconditions are satisfiable in planning state

### Plan Revalidation
- Before each step in the plan:
  - Re-check preconditions against current beliefs (via `&dyn BeliefView`)
  - If preconditions false → trigger replan
  - Emit replan reason event

### Broken Preconditions → Replan
- When precondition fails:
  - Record which precondition and why
  - Discard remaining plan
  - Re-run goal selection (goal may have changed)
  - Generate new plan for selected goal

### Reactive Executor
- Handles interrupts between plan steps:
  - Danger detection: flee if fear exceeds agent's `UrgencyThresholds.critical`
  - Urgent need: override plan if any need exceeds agent's `UrgencyThresholds.critical`
  - Interrupt current action if interruptible
  - Resume or replan after interrupt handled

### Agent Tick Integration
- Each tick, for each agent with `ControlSource::Ai`:
  1. Check for interrupts (danger, critical needs) — using agent's own thresholds
  2. If no current plan: select goal → plan
  3. If current plan: validate next step → execute or replan
  4. Progress current action if active

## Component Registration
New components to register in `component_schema.rs`:
- `UtilityProfile` — on `EntityKind::Agent`
- `UrgencyThresholds` — on `EntityKind::Agent`

## Cross-System Interactions (Principle 12)
- **E09 → E13**: Reads `HomeostaticNeeds` and `AgentCondition` via `BeliefView` for utility scoring
- **E12 → E13**: Reads `WoundList` via `BeliefView` to assess danger and prioritize healing
- **E10 → E13**: Reads available recipes/facilities via `BeliefView` for production planning
- **E11 → E13**: Restock goals drive merchant planning; trade affordances inform goal selection
- **E13 → all**: Emits `InputEvent::RequestAction` to scheduler, which routes to appropriate system handler

## FND-01 Section H

### Information-Path Analysis
- All decision inputs come through `&dyn BeliefView`, which queries the agent's beliefs (or world state via `OmniscientBeliefView` temporarily)
- The agent does not query global state — it queries what it believes about the world
- Goal selection is local: based on agent's own needs, condition, and beliefs
- Plan generation uses only information available through `BeliefView`
- In E14, `BeliefView` will be backed by per-agent belief stores updated through perception events, completing the information-path chain

### Positive-Feedback Analysis
- **Success → confidence → risk-taking → more success**: agents that succeed in trading/raiding might take on more ambitious goals. However, this is bounded by physical constraints (carry capacity, needs pressure, travel time).
- **Failure → replanning → same failure**: an agent might repeatedly attempt an infeasible goal. The replan mechanism could loop.

### Concrete Dampeners
- **Success amplification**: physical needs (hunger, thirst, fatigue) are the dampener — even a successful raider must eat and sleep. Homeostatic needs interrupt ambitious plans with biological imperatives.
- **Replan loop**: the dampener is goal re-evaluation. When a plan fails, goal selection re-runs with updated beliefs. If the same goal is selected but keeps failing, the agent's needs will shift priority (hunger increasing while stuck in a loop) and eventually force a different goal. Biological time pressure prevents infinite replanning on the same failed goal.

### Stored State vs. Derived Read-Model
**Stored (authoritative)**:
- `UtilityProfile` component (per-agent weights)
- `UrgencyThresholds` component (per-agent thresholds)
- Current plan (sequence of `ActionDefId` values — stored on agent, cleared on replan)
- Current goal (stored on agent, cleared on goal re-evaluation)

**Derived (transient read-model)**:
- `PlanningState` (derived from `BeliefView` at planning time — never stored as authoritative state)
- Utility scores (computed per tick from needs × weights — never stored)
- Urgency levels (computed from need value vs. threshold — never stored)
- Available affordances (computed from `BeliefView` — never stored)

## Invariants Enforced
- 9.11: World/belief separation — planner uses beliefs only, not world state
- 9.12: No player branching — same decision architecture regardless of `ControlSource`
- 9.14: Dead agents skipped entirely
- Principle 3: No stored utility scores or urgency levels — all derived on demand
- Principle 11: Agent diversity via `UtilityProfile` and `UrgencyThresholds`

## Tests
- [ ] T12: No player branching — attach control to merchant, guard, bandit, claimant, farmer without changing rules
- [ ] Plans use `&dyn BeliefView` only (mock: agent with false belief plans incorrectly, by design)
- [ ] Goal selection picks highest utility using `Permille` scoring
- [ ] Different `UtilityProfile` weights produce different goal selections for same needs
- [ ] Different `UrgencyThresholds` produce different interrupt behavior
- [ ] Replan triggers when precondition fails
- [ ] Reactive executor interrupts for danger (fear > agent's critical threshold)
- [ ] Dead agents generate no plans
- [ ] GOAP planner finds valid action sequence for simple goals (eat when hungry)
- [ ] Plan revalidation catches stale plans
- [ ] Same agent type produces same plans regardless of `ControlSource`
- [ ] `PlanningState` is never stored as authoritative state
- [ ] No social_standing, loyalty, or wealth_pressure in utility scoring inputs
- [ ] Greed weight applied to coin deficit (derived from holdings)
- [ ] All thresholds from per-agent `UrgencyThresholds`, not global constants

## Phase 2 Gate
After E13, verify:
- [ ] Agents autonomously eat, drink, sleep, trade without human input
- [ ] Agents replan when actions fail
- [ ] Merchants restock through physical procurement
- [ ] Basic survival loop runs for 24+ in-world hours without deadlock
- [ ] No agent starves with food available and reachable

## Acceptance Criteria
- Unified decision hierarchy: pressures → utility → goal → plan → execute
- All utility scoring uses `Permille`, not `f32`
- GOAP planner produces valid action sequences using `&dyn BeliefView`
- Plans revalidated at each step
- Belief-only planning enforced (all queries through `BeliefView`)
- Reactive interrupts for danger and urgent needs (per-agent thresholds)
- Same pipeline for all agent types
- Agent diversity via `UtilityProfile` and `UrgencyThresholds` (Principle 11)
- `OmniscientBeliefView` documented as temporary P10 violation

## Spec References
- Section 6.1 (one hierarchy, not three competing brains)
- Section 6.2 (goal examples)
- Section 6.3 (planning rules: compact state, revalidation, beliefs only)
- Section 6.4 (human control uses same pipeline)
- Section 9.11 (world/belief separation)
- Section 9.12 (player symmetry)
- `docs/FOUNDATIONS.md` Principles 2, 3, 10, 11, 12
