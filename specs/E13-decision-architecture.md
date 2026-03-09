# E13: Agent Decision Architecture

## Epic Summary
Implement the unified agent AI: utility scoring, goal selection, GOAP-style planner, plan revalidation, reactive executor, and belief-only planning.

## Phase
Phase 2: Survival & Logistics (final epic before Phase 2 gate)

## Crate
`worldwake-ai`

## Dependencies
- E09 (needs system: utility scoring inputs)
- E10 (production: goals to plan over)
- E11 (trade: goals to plan over)
- E12 (combat: goals to plan over)

## Deliverables

### Utility Scoring
Per spec section 6.1, one hierarchy:
1. Homeostatic and social pressures generate scores
2. Each need maps to a utility score based on urgency
3. Additional pressures: wealth, loyalty, fear, social standing
4. Utility function: `score(need_value, threshold) -> f32`
   - Higher score = more urgent

### Goal Selection
- `select_goal(agent, needs, beliefs) -> Goal`
- Highest-utility goal becomes current objective
- Goal re-evaluation on: tick interval, replan signal, major state change

### Goal Catalog
Per spec section 6.2:
- `Eat` - address hunger
- `Drink` - address thirst
- `Sleep` - address fatigue
- `Relieve` - address bladder
- `Wash` - address hygiene
- `Trade` - buy/sell goods
- `Restock` - merchant restocking
- `Escort` - escort cargo/caravan
- `Raid` - raid caravan/travelers (bandits)
- `Flee` - escape danger
- `ClaimOffice` - claim vacant office
- `SupportClaimant` - support another's claim
- `Heal` - seek or provide healing
- `BuryCorpse` - handle dead body
- `EstablishCamp` - set up new camp

### GOAP-Style Planner
Per spec section 6.3:
- Operates on compact abstract state (not full simulation)
- Input: goal + agent's believed state
- Output: ordered sequence of actions to achieve goal
- Search: forward state-space search with action effects
- Pruning: only consider actions whose preconditions are satisfiable

### Plan Revalidation
- Before each step in the plan:
  - Re-check preconditions against current beliefs
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
  - Danger detection: flee if fear threshold exceeded
  - Urgent need: override plan if critical need
  - Interrupt current action if interruptible
  - Resume or replan after interrupt handled

### Belief-Only Planning
Per spec section 6.3:
- Planner queries agent's beliefs, NOT global world state
- Abstract state constructed from: `KnowsFact`, `BelievesFact` relations
- Plans may be suboptimal or wrong if beliefs are outdated
- This is correct behavior (agents don't have omniscience)

### Agent Tick Integration
- Each tick, for each agent with ControlSource::Ai:
  1. Check for interrupts (danger, critical needs)
  2. If no current plan: select goal → plan
  3. If current plan: validate next step → execute or replan
  4. Progress current action if active

## Invariants Enforced
- 9.11: World/belief separation - planner uses beliefs only, not world state
- 9.12: No player branching - same decision architecture regardless of ControlSource
- 9.14: Dead agents skipped entirely

## Tests
- [ ] T12: No player branching - attach control to merchant, guard, bandit, claimant, farmer without changing rules
- [ ] Plans use beliefs only (mock: agent with false belief plans incorrectly, by design)
- [ ] Goal selection picks highest utility
- [ ] Replan triggers when precondition fails
- [ ] Reactive executor interrupts for danger
- [ ] Dead agents generate no plans
- [ ] GOAP planner finds valid action sequence for simple goals (eat when hungry)
- [ ] Plan revalidation catches stale plans
- [ ] Same agent type produces same plans regardless of ControlSource

## Phase 2 Gate
After E13, verify:
- [ ] Agents autonomously eat, drink, sleep, trade without human input
- [ ] Agents replan when actions fail
- [ ] Merchants restock through physical procurement
- [ ] Basic survival loop runs for 24+ in-world hours without deadlock
- [ ] No agent starves with food available and reachable

## Acceptance Criteria
- Unified decision hierarchy: pressures → utility → goal → plan → execute
- GOAP planner produces valid action sequences
- Plans revalidated at each step
- Belief-only planning enforced
- Reactive interrupts for danger and urgent needs
- Same pipeline for all agent types

## Spec References
- Section 6.1 (one hierarchy, not three competing brains)
- Section 6.2 (goal examples)
- Section 6.3 (planning rules: compact state, revalidation, beliefs only)
- Section 6.4 (human control uses same pipeline)
- Section 9.11 (world/belief separation)
- Section 9.12 (player symmetry)
