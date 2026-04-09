# S81: Golden E2E Gaps -- Simulation Remediation

## Summary

The simulation remediation report (`reports/simulation-remediation.md`) identified three golden test gaps and one missing component field. Since then, the S79 ticket slices archived at `archive/tickets/S79RESSOUCON-003.md` and `archive/tickets/S79RESSOUCON-004.md` have closed the live apple/eat proof gap and the water-source runtime contract gap. This spec therefore retains: (1) a multi-agent convergence test verifying agents at barren locations with remote resource beliefs don't collapse into prolonged sleep+relieve loops, (2) an agent death traceability test verifying death from unmet needs is explicit, causally traceable, and halts post-death planning, and, if still desired, (3) the now-unblocked water/drink resource-source golden follow-up. As a prerequisite deliverable, this spec adds a `DeathCause` field to the `DeadAt` component so death events carry traceable cause information.

## Phase

Phase 7: Consequence Carriers (Adjunct — Simulation Remediation)

## Status

Draft

## Crates

- `worldwake-ai` (golden tests in `tests/golden_simulation_gaps.rs`)
- `worldwake-core` (DeadAt extension with DeathCause)
- `worldwake-systems` (needs/mortality system — set DeathCause on death)

## Dependencies

- S79 (resource-source consumption affordances) — the apple/eat harvest-to-consume proof landed via `archive/tickets/S79RESSOUCON-003.md`, and the water-source runtime contract landed via `archive/tickets/S79RESSOUCON-004.md`; any remaining water/drink golden follow-up is now unblocked rather than runtime-blocked
- S76 (golden gaps — simulation observer) — completed; S81 extends coverage beyond S76's single-agent scenarios

## Design Goals

- Each scenario exercises a multi-system chain, not a single unit
- Tests should fail for the specific observed pathology
- GT-1 extends S76-B (single agent, 300 ticks) to multi-agent scale (3+ agents, 600+ ticks)
- GT-2 verifies a previously untested path: death from unmet needs
- GT-3, if still retained, verifies the remaining water/drink branch of the S79 resource-source affordance fix rather than recreating the completed apple/eat proof

## Non-Goals

- Fixing the root cause of missing affordances (that is S79)
- Exploration mechanics (that is S80)
- Plan search budget tuning (CognitiveProfile already supports this)
- Observer tooling improvements

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P1 (Maximal Emergence) | GT-1 verifies multi-agent emergent survival behavior, not scripted outcomes |
| P4 (Persistent Identity / Explicit Transfer) | GT-2 verifies death is a persistent, traceable state transition with explicit cause |
| P5 (Carriers of Consequence) | DeathCause makes death a richer carrier of downstream consequence (investigation, mourning, succession) |
| P8 (Preconditions, Duration, Cost) | GT-3 verifies the full precondition chain: resource source → harvest → possession → consume |
| P10 (Outcomes Leave Aftermath) | GT-2 verifies death leaves explicit aftermath (DeadAt with cause, event log entry) |
| P20 (Resource-Bounded Reasoning) | GT-1 verifies agents plan within budget at multi-agent scale |
| P26 (Systems Interact Through State) | All scenarios chain 3+ systems through state, not direct calls |
| P29 (Debuggability) | DeathCause improves death event debuggability — "why did this agent die?" has a concrete answer |

## Section H: Causal Hooks

No new causal hooks for the golden tests. DeathCause is a component extension, not a new system.

### Information-Path Analysis

- GT-1: Unmet needs → candidate generation → planner search across 3+ agents → travel/harvest/consume chains. Beliefs about remote resources seeded.
- GT-2: Unmet needs → need escalation → mortality threshold → DeadAt with DeathCause → post-death planning halt.
- GT-3: Agent at resource source location → harvest affordance generated → planner chains harvest → eat/drink.

### Positive-Feedback Analysis

No positive-feedback loops introduced. Tests only.

### Concrete Dampeners

N/A (no feedback loops).

### Stored State vs. Derived

- **Stored (new)**: `DeathCause` variant on `DeadAt` component
- **Derived**: None new

---

## Deliverable D1: DeathCause Component Extension

### DeathCause Enum

```rust
/// Cause of an agent's death, set alongside DeadAt.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum DeathCause {
    /// Died from an unmet need reaching lethal threshold.
    NeedDeprivation { need: HomeostaticNeedId },
    /// Died from combat wounds.
    CombatWounds,
}
```

Located in `crates/worldwake-core/src/combat.rs` (alongside existing `DeadAt`).

### DeadAt Extension

Change `DeadAt` from a tuple struct to a named-field struct:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct DeadAt {
    pub tick: Tick,
    pub cause: DeathCause,
}

impl Component for DeadAt {}
```

All existing `DeadAt(tick)` construction sites must be updated to `DeadAt { tick, cause }`.

### Mortality System Update

In the needs/mortality system (wherever `DeadAt` is currently set for need-based death), set `cause: DeathCause::NeedDeprivation { need }` with the specific `HomeostaticNeedId` that triggered death.

In the combat system (wherever `DeadAt` is set for combat death), set `cause: DeathCause::CombatWounds`.

### Event Log

Emit a death event to the event log when `DeadAt` is set, including the cause, tick, agent, and location. If a death event is already emitted, extend it with the cause field.

---

## Proposed Scenarios

### Scenario S81-A: Multi-Agent Convergence Does Not Cause Prolonged Behavioral Collapse

**Source finding**: Remediation GT-1, observer Finding 2 (Action Loops), Finding 3 (Stuck Agents)

**Description**: 3+ agents at a resource-barren location with seeded beliefs about remote resource locations. Run for 600+ ticks. Assert no agent enters a sleep+relieve-only loop for more than 200 consecutive ticks. At least one agent commits a `travel` action toward a resource-bearing location within 300 ticks.

**Setup**:
- 3 agents at a barren indoor location (no food/water sources)
- Remote places with food and water resource sources exist in the topology
- Each agent starts with seeded beliefs about at least one remote resource location
- Each agent has recipe knowledge for basic harvest actions
- Run for 600 ticks

**Assertions**:
1. No agent has >200 consecutive ticks where the only actions are `sleep` and `relieve`
2. At least one agent starts a `travel` action within 300 ticks
3. At least one agent reaches a resource-bearing location by tick 600

**GoalKinds exercised**: `AcquireCommodity`, `ConsumeOwnedCommodity`, `Sleep`, `Relieve`
**ActionDomains exercised**: Travel, Production (harvest), Needs (eat/drink/sleep/relieve)

**Why it is not a duplicate**: S76-B (`golden_max_idle_under_remote_resource_scarcity`) tests 1 agent for 300 ticks. The observer report shows qualitatively different failure at 3+ agents / 600+ ticks due to candidate explosion and contention effects.

### Scenario S81-B: Agent Death from Unaddressed Needs Is Traceable

**Source finding**: Remediation GT-2, observer Finding 3 (Stuck Agents), Finding 6 (Unaddressed Needs)

**Description**: One agent at a location with no eat/drink affordances and no beliefs about remote resources. Run until the agent dies or 600 ticks elapse.

**Setup**:
- 1 agent at a barren indoor location (no food, water, or resource sources)
- No seeded beliefs about remote resources
- No recipe knowledge for harvests
- Agent has default metabolism (needs escalate over time)

**Assertions**:
1. The agent dies (DeadAt component is set) within 600 ticks
2. `DeadAt.cause` is `DeathCause::NeedDeprivation { need }` where `need` is `HomeostaticNeedId::Hunger` or `HomeostaticNeedId::Thirst`
3. The event log contains a death event with the cause, tick, and agent
4. After death, no further planning or action attempts occur for the dead agent (assert no actions started after `DeadAt.tick`)

**GoalKinds exercised**: `ConsumeOwnedCommodity` (attempted, fails), `Sleep`, `Relieve`
**ActionDomains exercised**: Needs (sleep, relieve — only available actions)

**Why it is not a duplicate**: No existing golden test verifies the death-from-unmet-needs path. `golden_supply_chain.rs` asserts agents stay alive; this test asserts agents die correctly when they cannot sustain themselves.

### Scenario S81-C: Harvest-to-Consume Chain Works at Resource Source Locations

**Source finding**: Remediation GT-3, observer Finding 6 (Unaddressed Needs)

**Description**: Agents at locations with resource sources can plan and execute the harvest → eat/drink chain.

**Setup**:
- Agent A at a location with a Water resource source on a Facility (e.g., Well). Agent A has the harvest:Harvest Water recipe knowledge.
- Agent B at a location with an Apple resource source on a Facility (e.g., OrchardRow). Agent B has the harvest:Harvest Apples recipe knowledge.
- Both agents start with elevated hunger/thirst needs

**Assertions**:
1. Agent A's affordance set includes a harvest action for Water within the first planning cycle
2. Agent B's affordance set includes a harvest action for Apples within the first planning cycle
3. Agent A successfully executes: harvest water → drink within 100 ticks
4. Agent B successfully executes: harvest apples → eat within 100 ticks
5. After consumption, the corresponding need level has decreased

**GoalKinds exercised**: `AcquireCommodity`, `ConsumeOwnedCommodity`
**ActionDomains exercised**: Production (harvest), Needs (eat/drink)

**Why it is not a duplicate**: S76-C (`golden_perception_forms_resource_source_beliefs`) tests belief formation about resource sources, not affordance generation or the harvest-to-consume action chain. This test verifies the full chain from resource source to satisfied need.

---

## SystemFn Integration

No new SystemFn. DeathCause is set within the existing mortality check that sets DeadAt.

## Component Registration

| Component | Change | Crate |
|-----------|--------|-------|
| `DeadAt` | Extended with `cause: DeathCause` field | `worldwake-core` |
| `DeathCause` | New enum | `worldwake-core` |

No new component registration in `component_schema.rs` — `DeadAt` is already registered. The struct shape changes but the component identity stays the same.

## Cross-System Interactions

- **Needs system → DeadAt**: Mortality check sets `DeadAt { tick, cause: NeedDeprivation }` (existing interaction, extended with cause)
- **Combat system → DeadAt**: Combat sets `DeadAt { tick, cause: CombatWounds }` (existing interaction, extended with cause)
- **Planning system → DeadAt**: Planning checks `DeadAt` to skip dead agents (existing, no change)
