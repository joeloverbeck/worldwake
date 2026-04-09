# S80: Exploration Drive

## Summary

Add an exploration pressure system where agents with unsatisfied needs and no known path to satisfaction develop a drive to travel to unvisited or poorly-known locations. Currently, agents with unmet needs who lack beliefs about nearby resource sources enter indefinite sleep+relieve loops because no goal kind motivates geographic information-seeking. This spec introduces an exploration goal that emerges from the intersection of unmet needs, limited geographic beliefs, and per-agent curiosity profiles — filling the gap between "I need water" and "I don't know where water is."

## Phase

Phase 7: Consequence Carriers (Adjunct — Simulation Remediation)

## Status

Draft

## Crates

- `worldwake-core` (new goal kind, exploration profile component)
- `worldwake-ai` (candidate generation, goal dispatch, goal model)
- `worldwake-sim` (no changes expected)
- `worldwake-systems` (no changes expected — travel actions already exist)

## Dependencies

- S79 (resource-source consumption affordances) — agents need working consumption chains before exploration adds value; otherwise agents explore, arrive, and still can't harvest/eat
- E02 (world topology) — completed
- S38 (learned route/source preferences) — completed (provides route experience and source preference infrastructure)

## Design Goals

- Exploration emerges from unmet needs + limited geographic beliefs, not from a scripted "explore" trigger (FND-01)
- Agents reason about their own ignorance — "I don't know where water is" motivates information-seeking (FND-14)
- Geographic knowledge propagates through travel, testimony, and observation — exploration is one of these paths (FND-15)
- Exploration goals compete with other goals through the normal priority/planning pipeline (FND-20)
- Per-agent curiosity weight varies — some agents are homebodies, others are natural explorers (FND-22)
- Exploration targets are belief-gated: agents only consider places they have heard about or that are topologically adjacent to known places

## Non-Goals

- Systematic cartography or map-building mechanics
- Explicit "scout" or "explorer" role with special actions — exploration uses existing travel actions
- Exploration as a permanent background activity — it activates only when needs are unmet and no known satisfaction path exists
- Random wandering — exploration targets are selected from the agent's known or adjacent-to-known place set
- Discovery of entirely unknown places through special mechanics — agents discover places by traveling to adjacent places in the topology

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P1 (Maximal Emergence) | Exploration arises from unmet needs + limited beliefs, not scripted triggers. Discovering resource-rich places enables downstream economic activity. |
| P2 (No Ungrounded Triggers) | Exploration pressure derives from concrete need levels and belief gaps, not an abstract curiosity dial |
| P5 (Carriers of Consequence) | Geographic knowledge gained through exploration propagates: the explorer may tell others, changing collective behavior |
| P7 (Locality) | Agents explore places they know about (beliefs) or places adjacent to known places (topology). No global knowledge. |
| P14 (World State ≠ Belief State) | Agents reason about what they DON'T know — gaps in geographic belief motivate exploration |
| P15 (Knowledge Acquired Locally) | Exploration produces knowledge through direct travel and observation, the primary local acquisition path |
| P20 (Resource-Bounded Reasoning) | Exploration goals compete normally in the goal ranking pipeline; bounded by planning budget |
| P22 (Agent Diversity) | Per-agent `ExplorationProfile` with curiosity weight creates behavioral diversity: cautious vs. adventurous |
| P26 (Systems Interact Through State) | Exploration system reads need levels and belief state; writes travel goals to the goal pipeline. No cross-system calls. |

## Section H: Causal Hooks

### H1. Information-Path Analysis

1. **Need state** → agent's NeedLevels component shows unmet needs (hunger, thirst)
2. **Belief gap detection** → agent's AgentBeliefStore lacks beliefs about places with relevant resource sources. The agent can query: "do I know any place where I can satisfy need X?" If no, exploration pressure rises.
3. **Known place enumeration** → agent's beliefs contain place entities. Adjacent-to-known places come from topology (agent can query edges from known places to discover neighbors).
4. **Goal generation** → ExploreLocation goal is generated with a target place selected from known-but-unvisited or adjacent-to-known places
5. **Planning** → planner finds a travel plan to the target place using existing travel actions
6. **Execution** → agent travels, arrives, perceives new location, gains beliefs about entities/resources there
7. **Downstream** → new beliefs about resource sources enable AcquireCommodity → harvest → consume chains

All information enters through perception and existing belief state. No global queries.

### H2. Positive-Feedback Analysis

**Potential loop**: Exploration succeeds → agent finds resources → agent survives → agent explores more.

This is a healthy positive loop bounded by physical dampeners (H3).

### H3. Concrete Dampeners

1. **Travel cost**: Exploration requires travel, which consumes time, body energy, and exposes the agent to route hazards (FND-08)
2. **Need urgency**: As needs become critical, ConsumeOwnedCommodity and other survival goals outprioritize exploration
3. **Diminishing returns**: Once the agent discovers a resource source, the belief gap closes and exploration pressure for that need drops
4. **Place exhaustion**: Finite topology means finite exploration targets; visited places are no longer candidates

### H4. Stored State vs. Derived

- **Stored**: `ExplorationProfile` (per-agent component), `AgentBeliefStore` (existing — tracks known places and resource beliefs)
- **Derived**: exploration pressure (computed from need levels + belief gaps, never stored), candidate exploration targets (computed from beliefs + topology, never stored)

## Deliverables

### 1. ExplorationProfile Component

```rust
/// Per-agent exploration disposition.
/// Universal profile — all agents get one (with defaults).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExplorationProfile {
    /// Weight multiplier for exploration goal pressure relative to base
    /// need-driven urgency. Higher = more willing to explore when needs
    /// are unmet. Range: [0, 1000] where 0 = never explores,
    /// 500 = default, 1000 = highly exploratory.
    pub curiosity_weight: Permille,

    /// Minimum unmet need level (any single need) before exploration
    /// pressure activates. Prevents exploration when needs are mild.
    pub need_activation_threshold: Permille,

    /// Maximum number of consecutive exploration goals before the agent
    /// pauses exploration (prevents infinite wandering). 0 = no limit.
    pub max_consecutive_explorations: u8,
}
```

**Default impl**: `curiosity_weight: 500`, `need_activation_threshold: 400`, `max_consecutive_explorations: 3`.

Register on `EntityKind::Agent`. Universal profile (always applied with defaults). Add to `AgentDef` in scenario types. Add `ExplorationProfileDef` if EntityId references are needed (unlikely for this component).

### 2. GoalKind::ExploreLocation

```rust
GoalKind::ExploreLocation {
    /// Target place to travel to and observe.
    target_place: EntityId,
    /// The unmet need that motivated this exploration.
    motivating_need: HomeostaticNeedId,
}
```

Add to `GoalKind` enum in `crates/worldwake-core/src/goal.rs`. The `motivating_need` field enables the goal model to drop exploration goals when the need is satisfied by other means.

### 3. Exploration Goal Generation

In `crates/worldwake-ai/src/candidate_generation.rs` (or a new exploration-specific generator module):

**Trigger conditions** (all must hold):
1. Agent has at least one need above `ExplorationProfile.need_activation_threshold`
2. Agent has no known path to satisfy that need (no believed resource source for the need's commodity at any reachable place, OR AcquireCommodity has been blocked/exhausted for this commodity)
3. Agent has not exceeded `max_consecutive_explorations`

**Target selection**:
1. Enumerate places the agent believes exist (from `AgentBeliefStore.known_entities` where kind = Place)
2. Add places adjacent to known places in the topology (one hop beyond known, if the agent has topology beliefs)
3. Filter out the agent's current place and recently visited places (within a configurable lookback)
4. Rank by: (a) proximity (fewer hops preferred), (b) novelty (less-visited preferred), (c) random tiebreak via agent's RNG seed for diversity
5. Generate `ExploreLocation` goal for the top-ranked candidate

### 4. Goal Dispatch Declaration

Register `GoalKind::ExploreLocation` in the goal dispatch system:
- **Planning domain**: Travel (reuse existing travel action planning)
- **Terminal action**: `travel` to target_place
- **Goal achieved**: Agent is at `target_place`
- **Invalidation**: Need that motivated exploration drops below threshold, OR agent learns about a resource source for the motivating need (belief gap closes)

### 5. Goal Ranking Integration

Exploration goal priority should be:
- Below direct need satisfaction (ConsumeOwnedCommodity, AcquireCommodity with known path)
- Above idle/sleep when the need is above threshold and no satisfaction path is known
- Modulated by `ExplorationProfile.curiosity_weight`

The exact ranking formula integrates with the existing utility-based goal ranking in `crates/worldwake-ai/src/goal_model.rs`.

## SystemFn Integration

No new SystemFn needed. Exploration goals are generated during the existing candidate generation phase of the agent tick. Travel actions are already registered and executed by existing systems.

## Component Registration

| Component | Crate | EntityKind | Universal? | scenario-definable? |
|-----------|-------|------------|------------|---------------------|
| `ExplorationProfile` | `worldwake-core` | Agent | Yes (Default impl) | Yes (`AgentDef.exploration_profile`) |

Register in `component_schema.rs`. Add `insert_component_exploration_profile` / `get_component_exploration_profile` accessors.

## Cross-System Interactions

- **Needs system → Exploration**: Exploration reads NeedLevels to detect unmet needs (state-mediated)
- **Belief system → Exploration**: Exploration reads AgentBeliefStore to detect geographic knowledge gaps (state-mediated)
- **Exploration → Travel system**: Exploration generates travel goals; travel actions execute normally (state-mediated through goal pipeline)
- **Perception system → Exploration outcome**: Upon arrival, perception fires normally and the agent gains beliefs about the new location (existing flow, no changes)
- **Exploration → Candidate generation**: Once beliefs update post-travel, AcquireCommodity candidates may now include harvest actions at the newly discovered location (existing flow)
