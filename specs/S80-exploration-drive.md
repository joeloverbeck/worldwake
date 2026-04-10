# S80: Exploration Drive

## Summary

Add an exploration pressure system where agents with unsatisfied needs and no known path to satisfaction develop a drive to travel to unvisited or poorly-known locations. Currently, agents with unmet needs who lack beliefs about nearby resource sources enter indefinite sleep+relieve loops because no goal kind motivates geographic information-seeking. This spec introduces an exploration goal that emerges from the intersection of unmet needs, limited geographic beliefs, and per-agent curiosity profiles — filling the gap between "I need water" and "I don't know where water is."

## Phase

Phase 7: Consequence Carriers (Adjunct — Simulation Remediation)

## Status

Draft

## Crates

- `worldwake-core` (new goal kind, exploration profile component)
- `worldwake-sim` (new `exploration_profile()` accessor on `GoalBeliefView` trait in `crates/worldwake-sim/src/belief_view.rs`, plus `RuntimeBeliefView` impl and blanket forwarding through the narrow belief-view traits)
- `worldwake-ai` (candidate generation, goal dispatch, goal model `GoalKindPlannerExt` impl, ranking)
- `worldwake-systems` (no changes expected — travel actions already exist)

## Dependencies

- S79 (resource-source consumption affordances) — completed; agents need working consumption chains before exploration adds value; otherwise agents explore, arrive, and still can't harvest/eat
- S81 (golden gaps — simulation remediation) — completed; provides death traceability substrate and golden test coverage that S80 builds upon
- E02 (world topology) — completed
- S38 (learned route/source preferences) — completed (provides route experience and source preference infrastructure)

## Design Goals

- Exploration emerges from unmet needs + limited geographic beliefs, not from a scripted "explore" trigger (FND-01)
- Agents reason about their own ignorance — "I don't know where water is" motivates information-seeking (FND-14)
- Geographic knowledge propagates through travel, testimony, and observation — exploration is one of these paths (FND-15)
- Exploration goals enter the normal priority/planning pipeline once emitted, but the current emitter is a self-care fallback that suppresses itself when non-self-care candidate families are already present (FND-20)
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
| P7 (Locality) | Agents explore places they know about (beliefs) or places adjacent to known places (topology via `GoalBeliefView::adjacent_places_with_travel_ticks()`). No global knowledge. |
| P14 (World State ≠ Belief State) | Agents reason about what they DON'T know — gaps in geographic belief motivate exploration |
| P15 (Knowledge Acquired Locally) | Exploration produces knowledge through direct travel and observation, the primary local acquisition path |
| P20 (Resource-Bounded Reasoning) | Exploration goals compete normally in the goal ranking pipeline; bounded by planning budget |
| P22 (Agent Diversity) | Per-agent `ExplorationProfile` with curiosity weight creates behavioral diversity: cautious vs. adventurous |
| P26 (Systems Interact Through State) | Exploration system reads need levels and belief state; writes travel goals to the goal pipeline. No cross-system calls. |

## Section H: Causal Hooks

### H1. Information-Path Analysis

1. **Need state** → agent's `HomeostaticNeeds` component shows unmet needs (hunger, thirst)
2. **Belief gap detection** → agent's `AgentBeliefStore` lacks beliefs about places with relevant resource sources. The agent can query: "do I know any place where I can satisfy need X?" If no, exploration pressure rises.
3. **Known place enumeration** → agent's beliefs contain place entities. Adjacent-to-known places come from `GoalBeliefView::adjacent_places_with_travel_ticks()` (public structural topology, consistent with how all travel-based goals discover neighbors).
4. **Goal generation** → `ExploreLocation` goal is generated with a target place selected from known-but-unvisited or adjacent-to-known places
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
5. **Consecutive exploration cap**: `ExplorationProfile.max_consecutive_explorations` limits back-to-back exploration, tracked via stored `consecutive_exploration_count`

### H4. Stored State vs. Derived

- **Stored**: `ExplorationProfile` (per-agent component — curiosity_weight, need_activation_threshold, max_consecutive_explorations, visit_lookback_ticks), `consecutive_exploration_count: u8` (per-agent runtime field, reset to 0 when agent pursues a non-exploration goal), `AgentBeliefStore` (existing — tracks known places and resource beliefs)
- **Derived**: exploration pressure (computed from need levels + belief gaps, never stored), candidate exploration targets (computed from beliefs + topology, never stored), "recently visited" filter (derived from belief timestamps in `AgentBeliefStore.known_entities` within `visit_lookback_ticks` window)

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

    /// How far back (in ticks) to look when filtering "recently visited"
    /// places from exploration targets. Places visited within this window
    /// are excluded. 0 = no lookback filtering.
    pub visit_lookback_ticks: u32,

    /// Runtime counter: how many consecutive ExploreLocation goals the
    /// agent has pursued without an intervening non-exploration goal.
    /// Reset to 0 when the agent pursues any other goal kind.
    /// Not scenario-definable — always starts at 0.
    pub consecutive_exploration_count: u8,
}
```

**Default impl**: `curiosity_weight: 500`, `need_activation_threshold: 400`, `max_consecutive_explorations: 3`, `visit_lookback_ticks: 200`, `consecutive_exploration_count: 0`.

Register on `EntityKind::Agent`. Universal profile (always applied with defaults). Add to `AgentDef` in scenario types (all fields except `consecutive_exploration_count` which is runtime-only and always starts at 0). Add `ExplorationProfileDef` only if EntityId references are needed (unlikely for this component).

### 2. GoalKind::ExploreLocation

```rust
GoalKind::ExploreLocation {
    /// Target place to travel to and observe.
    target_place: EntityId,
    /// The unmet need that motivated this exploration.
    motivating_need: HomeostaticNeedId,
}
```

Add to `GoalKind` enum in `crates/worldwake-core/src/goal.rs`. Both `EntityId` and `HomeostaticNeedId` are `Copy`, compatible with `GoalKind`'s existing derives (`Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`). The `motivating_need` field enables the goal model to drop exploration goals when the need is satisfied by other means.

### 3. GoalBeliefView Accessor

Add to `GoalBeliefView` trait in `crates/worldwake-sim/src/belief_view.rs`:

```rust
fn exploration_profile(&self, agent: EntityId) -> Option<ExplorationProfile> {
    let _ = agent;
    None
}
```

Forward through the live `GoalBeliefView` blanket impl path and implement in `RuntimeBeliefView` to read from the ECS store.

### 4. Exploration Goal Generation

In `crates/worldwake-ai/src/candidate_generation.rs` (or a new exploration-specific generator module):

**Trigger conditions** (all must hold):
1. Agent has at least one need above `ExplorationProfile.need_activation_threshold` (read via `GoalBeliefView::homeostatic_needs()`)
2. Agent has no known path to satisfy that need (no believed resource source for the need's commodity at any reachable place via `GoalBeliefView::resource_sources_at()`, OR AcquireCommodity has been blocked/exhausted for this commodity)
3. Agent has not exceeded `max_consecutive_explorations` (checked via `ExplorationProfile.consecutive_exploration_count`)

**Target selection**:
1. Enumerate places the agent believes exist (from `AgentBeliefStore.known_entities` where kind = Place)
2. Add places adjacent to known places via `GoalBeliefView::adjacent_places_with_travel_ticks()` (one hop beyond known — public structural topology, same mechanism all travel-based goals use)
3. Filter out the agent's current place and places visited within `visit_lookback_ticks` (derived from belief timestamps in `AgentBeliefStore.known_entities`)
4. Rank deterministically by: (a) novelty/frontier preference (adjacent-to-known places without a direct place belief first), (b) proximity (fewer hops preferred), (c) oldest surviving place belief, then stable entity-id order
5. Generate `ExploreLocation` goal for the top-ranked candidate

**Counter management**: When the agent tick selects an `ExploreLocation` goal, increment `consecutive_exploration_count`. When any other goal is selected, reset to 0.

### 5. Goal Dispatch Declaration

Register `GoalKind::ExploreLocation` in the goal dispatch infrastructure. The additive dispatch-key and declaration substrate is already landed; remaining work in this spec should treat these as the live baseline rather than future-first additions.

**GoalDispatchKey** (`crates/worldwake-ai/src/goal_dispatch_key.rs`):
- `ExploreLocation` is present in the `GoalDispatchKey` enum
- `GoalDispatchKey::ALL` includes `ExploreLocation`
- `from_goal_kind` maps `GoalKind::ExploreLocation { .. } => GoalDispatchKey::ExploreLocation`

**GoalDispatchDeclaration** (`crates/worldwake-ai/src/goal_dispatch_decl.rs`):
- `EXPLORE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel]` — exploration reuses the existing Travel op; no new `PlannerOpKind` variant needed
- Travel-only declaration substrate is present. Any follow-up changes here should be driven by evidence about invalidation/feasibility strategy quality, not by missing symbol registration.

### 6. GoalKindPlannerExt Implementation

Implement `GoalKindPlannerExt` (`crates/worldwake-ai/src/goal_model.rs:38`) for `GoalKind::ExploreLocation`:

- `ranked_goal_provenance_family()` → new `RankedGoalProvenanceFamily::Exploration` variant (or `None` if no provenance tracking needed initially)
- `relevant_op_kinds()` → `EXPLORE_OPS` (Travel only)
- `relevant_observed_commodities()` → `None` (exploration doesn't target specific commodities)
- `build_payload_override()` → `Ok(None)` (no payload override needed — uses standard travel payload)
- `apply_planner_step()` → simulate travel to `target_place` in planning state
- `is_satisfied()` → true when agent's effective place in planning state == `target_place`
- `goal_relevant_places()` → `vec![target_place]` (guides A* heuristic toward destination)
- `prerequisite_places()` → empty or `{target_place}` depending on whether travel is prerequisite or terminal
- `matches_binding()` → true when `authoritative_targets` contains `target_place` and op is Travel
- `candidate_is_available()` → true (Travel is always available when path exists)

### 7. Goal Ranking Integration

Exploration goal priority integrates with the existing ranking system in `crates/worldwake-ai/src/goal_model.rs`:

- **Priority class**: `GoalPriorityClass::Low` — below direct need satisfaction (ConsumeOwnedCommodity, AcquireCommodity at Medium/High) but above idle/sleep (Background)
- **Motive score**: `need_level.as_raw() * curiosity_weight.as_raw() / 1000` — higher unmet needs and higher curiosity produce stronger exploration drive
- **Invalidation**: Goal is dropped from ranking when:
  - The motivating need drops below `need_activation_threshold`
  - Agent learns about a resource source for the motivating need (belief gap closes)
  - `consecutive_exploration_count >= max_consecutive_explorations`

## SystemFn Integration

No new SystemFn needed. Exploration goals are generated during the existing candidate generation phase of the agent tick. Travel actions are already registered and executed by existing systems. The `consecutive_exploration_count` field on `ExplorationProfile` is updated during goal selection in the agent tick.

## Component Registration

| Component | Crate | EntityKind | Universal? | scenario-definable? |
|-----------|-------|------------|------------|---------------------|
| `ExplorationProfile` | `worldwake-core` | Agent | Yes (Default impl) | Yes (`AgentDef.exploration_profile`) — all fields except `consecutive_exploration_count` which is runtime-only |

Register in `component_schema.rs`. Add `insert_component_exploration_profile` / `get_component_exploration_profile` accessors.

## Cross-System Interactions

- **Needs system → Exploration**: Exploration reads `HomeostaticNeeds` to detect unmet needs (state-mediated)
- **Belief system → Exploration**: Exploration reads `AgentBeliefStore` to detect geographic knowledge gaps (state-mediated)
- **Exploration → Travel system**: Exploration generates travel goals; travel actions execute normally (state-mediated through goal pipeline)
- **Perception system → Exploration outcome**: Upon arrival, perception fires normally and the agent gains beliefs about the new location (existing flow, no changes)
- **Exploration → Candidate generation**: Once beliefs update post-travel, AcquireCommodity candidates may now include harvest actions at the newly discovered location (existing flow)

## Validation Patterns

Suggested golden test scenarios to verify exploration behavior:

1. **Exploration triggers on need + ignorance**: Agent with hunger above `need_activation_threshold`, no believed food source at any reachable place → should generate `GoalKind::ExploreLocation` with `motivating_need: Hunger`
2. **No exploration when satisfaction path exists**: Agent with hunger above threshold BUT a believed food source exists at a known place → should NOT generate ExploreLocation (should generate AcquireCommodity instead)
3. **Consecutive cap respected**: Agent that has pursued `max_consecutive_explorations` ExploreLocation goals in a row → should NOT generate another ExploreLocation until a non-exploration goal intervenes
4. **Arrival yields new beliefs**: Agent completes ExploreLocation travel → perception fires → agent gains beliefs about entities at new location → if food source discovered, exploration pressure for hunger drops
