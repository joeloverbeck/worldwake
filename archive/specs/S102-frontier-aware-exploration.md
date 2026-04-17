# S102: Frontier-Aware Need-Driven Exploration

## Summary

The S80 exploration system allows agents to discover locations by traveling to places adjacent to known places in the topology. However, three interacting problems cause agents to starve or suffer when resources exist at locations beyond their one-hop horizon:

1. **Exploration gate false positive**: `need_has_known_acquisition_path()` suppresses exploration when the agent believes resources exist at a reachable location — even when the planner repeatedly budget-exhausts trying to plan acquisition. The "known path" is unreachable in practice, but exploration never fires because it looks reachable in theory.

2. **One-hop adjacency ceiling**: `select_exploration_target()` builds candidates from known places plus one hop of topology adjacency. After exhausting all one-hop targets, exploration stops. Places two or more hops from known locations are invisible.

3. **Belief decay breaks exploration chains**: S101 activation-based belief decay can evict place beliefs between consecutive exploration rounds. An agent that briefly visits an intermediate location loses the belief about it before the next exploration cycle can use it as a stepping-stone to deeper targets.

Evidence: The needs-starvation-diagnostic on `scenarios/cli-evaluation.ron` found Guard Theron died of hunger (tick 769), Kael's hunger saturated at 1000‰ for 683 ticks, and Forager Lina's dirtiness hit 1000‰ for 810 ticks — all while resources existed at reachable but undiscovered locations.

## Phase

Post-Phase 7 adjunct (extends S80 — exploration drive)

## Status

COMPLETED

## Crates

- `worldwake-core` (ExplorationProfile new fields, new AcquisitionExhaustionTracker component, HomeostaticNeedId::VARIANT_COUNT constant)
- `worldwake-ai` (candidate generation gate logic, target selection BFS, planner failure tracking, tracker reset in candidate generation)
- `worldwake-sim` (GoalBeliefView / ProfileBeliefView accessor for exhaustion count)
- `worldwake-systems` (travel commit activation boost for exploration-chain belief protection)
- `worldwake-cli` (scenario system support for new ExplorationProfile fields)

## Dependencies

- S80 (Exploration Drive) — completed. This spec extends S80's candidate generation and target selection.
- S101 (Activation-Based Belief Decay) — completed. Change 3 interacts with S101's activation system.
- S100 (Tiered Belief Retention) — completed. Provides infrastructure retention; this spec adds an explicit boost for exploration-discovered places on top of S100's time-based retention.

## Design Goals

- Agents can discover locations up to N hops from known places, where N is configurable per-agent (FND-22)
- Exploration fires when planner-known paths are unreliable, not just when paths are completely unknown (FND-20)
- Exploration-chain beliefs persist long enough for multi-hop discovery (FND-14, FND-16)
- All discovery still occurs through physical travel and local observation — no omniscience (FND-07, FND-15)
- Changes extend existing ExplorationProfile and belief systems — no new goal kinds or action types

## Non-Goals

- Need-directed exploration targeting (biasing targets by facility type) — requires facility metadata in topology or hearsay systems that don't exist yet
- Changing how ExploreLocation goals dispatch, plan, or execute — the goal kind, planner model, and travel execution are unchanged
- Modifying S101 activation decay rates or S100 retention windows — this spec adds a one-time boost, not a systemic change to decay mechanics
- "Scout" or "cartography" actions — exploration remains a self-care fallback using existing travel
- Allowing exploration when `curiosity_weight` is 0 or `max_consecutive_explorations` is exceeded — existing caps remain in force

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P1 (Maximal Emergence) | Multi-hop discovery enables agents to find resources through incremental exploration chains, not authored knowledge. Starvation at barren locations becomes survivable through emergent geographic discovery. |
| P3 (Concrete State Over Abstract Scores) | Acquisition failure counter is concrete per-need event tracking, not an abstract "exploration urgency score." Counter increments on specific plan outcomes, resets on specific need satisfaction events. |
| P7 (Locality) | Frontier expansion at depth 1 uses topology outgoing edges of known (visited) places — physical roads the agent has observed. At depth > 1, the BFS queries topology edges from unvisited places as a **planning heuristic**: the agent infers "roads probably continue beyond places I know about" without knowing what's at those destinations. This is a pragmatic approximation — agents still must physically travel to each place to discover what's there. The topology query provides route-existence hints, not knowledge of destinations. See Deliverable 5 for full discussion. |
| P8 (Every Action Has Cost) | Deeper exploration requires more travel ticks, more fatigue, more exposure. The physical cost of multi-hop exploration is its own dampener. |
| P14 (World State ≠ Belief State) | Agents still plan from beliefs only. Frontier places appear as "I know there's a road leading somewhere I haven't been" — not "I know what's at that place." |
| P15 (Knowledge Acquired Locally) | Agents discover frontier places by observing topology edges at their current location, then traveling and perceiving. Each hop produces knowledge locally. Depth > 1 frontier hints are a planning heuristic; actual knowledge is acquired only upon physical arrival. |
| P16 (Ignorance as First-Class) | Budget exhaustion is a signal of practical ignorance — the agent believes a path exists but can't operationalize it. Treating this as "effectively unknown" respects the gap between belief and capability. |
| P20 (Resource-Bounded Reasoning) | Exploration goals still compete in the normal ranking pipeline. The failure threshold prevents premature exploration; frontier depth caps the search space. |
| P22 (Agent Diversity) | `frontier_depth` and `acquisition_failure_threshold` are per-agent profile parameters. Cautious agents explore shallowly; adventurous agents range further. |
| P26 (Systems Interact Through State) | Planner failure counts are stored state. Exploration reads them. No cross-system function calls. |
| P28 (No Backward Compatibility) | The old one-hop-only behavior is replaced, not shimmed. S80's non-goal "Discovery of entirely unknown places" is superseded — agents now discover places beyond one hop, but still only via physical topology traversal. |

## Section H: Causal Hooks

### H.1 Motivating Gap (Information-Path Analysis)

**Current path (S80)**:
1. Agent has unmet need → checks belief store for known resource sources
2. If no known source: generates ExploreLocation targeting known + one-hop-adjacent places
3. If known source exists: suppresses exploration, planner tries AcquireCommodity
4. Planner budget-exhausts → no action taken → need rises → planner tries again → budget-exhausts again → indefinite loop

**Gap**: Step 4 has no feedback to step 2. The agent is stuck in a planning loop with a "known" path it can't execute.

**New path (S102)**:
1. Agent has unmet need → checks belief store for known resource sources
2. If known source AND acquisition failure count below threshold: planner tries AcquireCommodity (unchanged)
3. If planner budget-exhausts: increment acquisition failure count for this need
4. When count >= `acquisition_failure_threshold`: treat "known path" as unreliable → exploration fires
5. ExploreLocation targets include places up to `frontier_depth` hops from known places via topology
6. Agent travels to frontier place → perceives entities there → beliefs updated
7. Newly explored place receives synthetic presentation ticks → persists in beliefs for next exploration round
8. Next cycle: new place's adjacencies become candidates, enabling further exploration
9. Eventually: agent discovers place with the needed resource → AcquireCommodity succeeds → counter resets

All information enters through topology edges (physical roads) and perception (local observation at arrival). No global queries at any step.

### H.2 Positive-Feedback Analysis

**Potential loop**: Exploration succeeds → agent discovers resources → need satisfied → counter resets → agent no longer explores.

This is a **self-limiting** loop: success terminates the behavior. No amplification.

**Potential loop**: Exploration fails (frontier place has no resources) → counter stays high → agent explores again → explores more frontier places.

This is bounded by physical dampeners (H.3). Each exploration costs travel time and consumes the frontier. The topology is finite.

### H.3 Concrete Dampeners

1. **Travel cost**: Each exploration hop requires travel ticks, consuming time, accumulating fatigue, and exposing the agent to route hazards (FND-08)
2. **Finite topology**: The place graph has a finite number of nodes. Frontier places are consumed by visitation. Multi-hop exploration terminates when all reachable places within `frontier_depth` have been visited.
3. **Consecutive exploration cap**: `max_consecutive_explorations` still limits back-to-back exploration goals (S80 dampener, unchanged)
4. **Visit lookback**: `visit_lookback_ticks` prevents re-exploring recently visited places (S80 dampener, unchanged)
5. **Threshold gate**: Exploration only fires when needs exceed `need_activation_threshold` AND the acquisition failure count reaches threshold. The counter serves as a "tried everything reasonable" check before exploring.
6. **Need satisfaction reset**: Successfully satisfying the need resets the acquisition failure counter, halting exploration for that need.

### H.4 Stored State vs. Derived

**New stored state**:
- `AcquisitionExhaustionTracker` component: `counts: [u8; HomeostaticNeedId::VARIANT_COUNT]` indexed by `HomeostaticNeedId` variant discriminant. Per-agent, runtime-only (not scenario-definable, always starts at zero). Incremented on budget-exhaustion of need-satisfying goals. Reset to zero when the corresponding need drops below `need_activation_threshold` (checked lazily during candidate generation).
- `ExplorationProfile.frontier_depth: u16` — stored per-agent parameter, scenario-definable
- `ExplorationProfile.acquisition_failure_threshold: u8` — stored per-agent parameter, scenario-definable
- `ExplorationProfile.exploration_arrival_boost: Permille` — stored per-agent parameter, scenario-definable

**Derived (never stored)**:
- Frontier candidate set (computed from beliefs + multi-hop topology BFS, never stored)
- "Is known path unreliable?" (derived from `AcquisitionExhaustionTracker` count vs. threshold)
- Synthetic presentation ticks (transient side-effect of arriving at exploration target)

## Deliverables

### 1. ExplorationProfile New Fields

Add three fields to `ExplorationProfile` in `crates/worldwake-core/src/exploration.rs`:

```rust
/// Maximum topology hops beyond known places to consider as
/// exploration targets. 1 = current behavior (adjacent only).
/// 2 = two hops (default). Higher values search deeper frontier
/// but incur more travel cost.
pub frontier_depth: u16,

/// Number of consecutive budget-exhausted AcquireCommodity or
/// ProduceCommodity plan searches for a need before exploration
/// overrides the "known acquisition path" gate. Prevents exploration
/// from firing prematurely while the planner still has a chance.
pub acquisition_failure_threshold: u8,

/// Controls how many synthetic presentation ticks are pushed to the
/// destination place's belief when the agent arrives at an
/// ExploreLocation target. Higher values make explored places resist
/// S101 decay longer, enabling multi-hop chains.
/// Maps to tick count: value * MAX_PRESENTATION_TICKS / 1000.
/// Range: [0, 1000] where 0 = no boost, 500 = default (4 ticks).
pub exploration_arrival_boost: Permille,
```

**Defaults**: `frontier_depth: 2`, `acquisition_failure_threshold: 3`, `exploration_arrival_boost: Permille::new_unchecked(500)`.

Update the `Default` impl accordingly. All three fields are scenario-definable via `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs`. The `ExplorationProfile` struct remains `Copy` (all fields are `Copy` types).

### 2. AcquisitionExhaustionTracker Component

New component in `crates/worldwake-core/src/exploration.rs` (alongside `ExplorationProfile`):

```rust
/// Tracks per-need budget exhaustion counts for the exploration gate.
/// Runtime-only — not scenario-definable, always starts at zero.
///
/// This complements the existing per-goal `exhaustion_cache` in
/// `AgentDecisionRuntime` (which tracks `consecutive_failures` per
/// `OpportunityKey` for retry backoff). This tracker aggregates at
/// per-need granularity: when ALL acquisition paths for a given need
/// are budget-exhausted, exploration should fire for that need.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct AcquisitionExhaustionTracker {
    /// Indexed by HomeostaticNeedId discriminant.
    /// Saturates at u8::MAX rather than wrapping.
    counts: [u8; HomeostaticNeedId::VARIANT_COUNT],
}

impl AcquisitionExhaustionTracker {
    pub fn increment(&mut self, need: HomeostaticNeedId) {
        let idx = need as usize;
        self.counts[idx] = self.counts[idx].saturating_add(1);
    }

    pub fn reset(&mut self, need: HomeostaticNeedId) {
        self.counts[need as usize] = 0;
    }

    pub fn count(&self, need: HomeostaticNeedId) -> u8 {
        self.counts[need as usize]
    }
}
```

Also add to `HomeostaticNeedId` in `crates/worldwake-core/src/needs.rs`:

```rust
impl HomeostaticNeedId {
    pub const VARIANT_COUNT: usize = 5;
}
```

Register on `EntityKind::Agent` in `crates/worldwake-core/src/component_schema.rs` via the `with_component_schema_entries!` macro. Universal component (always applied with defaults). Not exposed in scenario RON (runtime-only state). Added via `create_agent()` in `crates/worldwake-core/src/world.rs`.

### 3. Planner Failure Tracking

In `crates/worldwake-ai`, at the point where plan search returns a budget-exhaustion result for an `AcquireCommodity` or `ProduceCommodity` goal:

1. Determine which `HomeostaticNeedId` the goal's commodity satisfies. The existing mapping uses filter functions in `candidate_generation.rs` (`relieves_hunger`, `relieves_thirst`, `relieves_dirtiness`) and `CommodityConsumableProfile` fields (hunger_relief_per_unit, thirst_relief_per_unit, bladder_fill_per_unit). A reverse mapping from `CommodityKind` to `HomeostaticNeedId` can be derived from these: check each `CommodityKind`'s `spec().consumable_profile` for non-zero relief values.
2. Call `tracker.increment(need_id)` on the agent's `AcquisitionExhaustionTracker`

The tracker is exposed through `GoalBeliefView` so the exploration gate can read it (see Deliverable 7).

### 4. Exploration Gate Modification

In `emit_exploration_candidates()` in `crates/worldwake-ai/src/candidate_generation.rs`, modify the suppression condition:

**Current** (line ~2379):
```rust
if pressure < profile.need_activation_threshold
    || any_local_need_relief(ctx.view, ctx.agent, ctx.place, matches_need)
    || need_has_known_acquisition_path(ctx, matches_need)
{
    continue;
}
```

**New**:
```rust
if pressure < profile.need_activation_threshold {
    // Need is below threshold — also reset the tracker for this need
    // so the counter doesn't carry stale state from previous episodes.
    if let Some(mut tracker) = ctx.view.acquisition_exhaustion_tracker(ctx.agent) {
        if tracker.count(need_id) > 0 {
            tracker.reset(need_id);
            // Write back via world transaction (exact mechanism depends on
            // whether the view provides mutable access or requires a
            // separate write path through the action execution context).
        }
    }
    continue;
}

let exhaustion_count = ctx.view.acquisition_exhaustion_count(ctx.agent, need_id);
let path_reliable = exhaustion_count < profile.acquisition_failure_threshold;

if any_local_need_relief(ctx.view, ctx.agent, ctx.place, matches_need)
    || (need_has_known_acquisition_path(ctx, matches_need) && path_reliable)
{
    continue;
}
```

When `path_reliable` is false (exhaustion count >= threshold), the "known acquisition path" gate is bypassed, allowing exploration to fire even though the agent believes resources exist somewhere.

**Tracker reset**: The reset is performed here in candidate generation (rather than in the needs/metabolism tick) to avoid cross-concern coupling with `worldwake-systems`. When `pressure < profile.need_activation_threshold`, the counter for that need is reset. This keeps exploration-awareness contained within the AI crate. The reset is lazy (happens on next candidate generation pass), which is acceptable since the gate also checks `pressure < threshold` first — a low-pressure need wouldn't trigger exploration regardless of counter value.

**Implementation note**: The reset requires mutable access to the `AcquisitionExhaustionTracker` component. Since `emit_exploration_candidates` currently operates through the read-only `GoalBeliefView`, the reset may need to be deferred to a write phase (e.g., accumulated as a pending mutation and applied after candidate generation). The exact mechanism should follow the existing pattern for mutations during the decision tick.

### 5. Multi-Hop Frontier Target Selection

In `select_exploration_target()` in `crates/worldwake-ai/src/candidate_generation.rs`, replace the single-hop adjacency expansion with a BFS up to `frontier_depth` hops:

**Current** (line ~4275):
```rust
let mut candidates = BTreeMap::<EntityId, Option<Tick>>::new();
for (place, observed_tick) in &known_places {
    candidates.insert(*place, Some(*observed_tick));
    for (adjacent, _) in ctx.view.adjacent_places_with_travel_ticks(*place) {
        candidates.entry(adjacent).or_insert(None);
    }
}
```

**New**:
```rust
let mut candidates = BTreeMap::<EntityId, Option<Tick>>::new();
// Seed with known places (depth 0)
for (place, observed_tick) in &known_places {
    candidates.insert(*place, Some(*observed_tick));
}

// BFS frontier expansion up to frontier_depth hops
let mut frontier: Vec<EntityId> = known_places.iter().map(|(p, _)| *p).collect();
for _depth in 0..profile.frontier_depth {
    let mut next_frontier = Vec::new();
    for place in &frontier {
        for (adjacent, _) in ctx.view.adjacent_places_with_travel_ticks(*place) {
            if candidates.entry(adjacent).or_insert(None).is_none() {
                // New frontier place discovered
                next_frontier.push(adjacent);
            }
        }
    }
    if next_frontier.is_empty() {
        break; // No more frontier to expand
    }
    frontier = next_frontier;
}
```

**FND-07 locality note**: At depth 1, the BFS follows outgoing edges of known (visited) places — roads the agent has physically observed. At depth > 1, the BFS queries `adjacent_places_with_travel_ticks()` for places the agent has NOT visited. This is a **pragmatic planning heuristic**: the agent infers "roads probably continue beyond places I know about" without knowing what's at those destinations. The agent does not gain knowledge of destination contents — only route-existence hints. Actual knowledge of each place is acquired only upon physical arrival and local perception (FND-15). This approximation enables meaningful multi-hop exploration while maintaining the core locality guarantee: all resource/entity discovery still requires physical presence.

**Ranking adjustment**: The existing ranking tuple `(novelty, proximity, age)` naturally favors frontier places (they have `observed_tick = None` → highest novelty). Among frontier places, proximity (fewest travel ticks) breaks ties, which favors closer frontiers. No ranking changes needed.

### 6. Exploration-Chain Belief Protection

When an agent arrives at an ExploreLocation target place (the travel action commits), push synthetic presentation ticks to the target place's entity in the agent's belief store to resist S101 activation decay:

1. In the travel commit handler (`crates/worldwake-systems/src/travel_actions.rs`, `commit_travel()` at line ~264), after setting the ground location and emitting movement trace evidence:
2. Check if the travel was motivated by an ExploreLocation goal (inspect the active goal / intention frame)
3. Read `ExplorationProfile.exploration_arrival_boost` for the agent
4. Compute synthetic tick count: `boost.value() * BelievedEntityState::MAX_PRESENTATION_TICKS as u32 / 1000` (e.g., 500‰ → 4 ticks)
5. Call `push_presentation_tick(current_tick, MAX_PRESENTATION_TICKS)` on the target place entity's `BelievedEntityState` in the agent's `AgentBeliefStore`, repeated for the computed tick count

This ensures the explored place's belief activation is high enough to survive at least one full exploration cycle before S101 decay evicts it. With default boost of 500‰ (4 synthetic ticks) and S101's power-law decay, this provides several hundred ticks of persistence.

**Interaction with S100**: S100 already gives infrastructure entities (places, facilities) a longer retention window via `infrastructure_retention_ticks`. The exploration arrival boost stacks with S100's retention, providing additional protection specifically for exploration-discovered places that the agent has just visited and may need as stepping-stones.

### 7. GoalBeliefView / ProfileBeliefView Extensions

Add the exhaustion tracker accessor following the existing belief-view trait chain pattern:

**Step 1**: Add to `ProfileBeliefView` trait in `crates/worldwake-sim/src/belief_view.rs` (alongside `exploration_profile()`):

```rust
fn acquisition_exhaustion_count(&self, agent: EntityId, need: HomeostaticNeedId) -> u8 {
    let _ = (agent, need);
    0
}
```

**Step 2**: Add forwarding in the blanket `GoalBeliefView` impl (at line ~916+):

```rust
fn acquisition_exhaustion_count(&self, agent: EntityId, need: HomeostaticNeedId) -> u8 {
    ProfileBeliefView::acquisition_exhaustion_count(self, agent, need)
}
```

**Step 3**: Implement in `PerAgentBeliefView` (runtime belief view) to read from the `AcquisitionExhaustionTracker` ECS component via world state.

**Step 4**: Add the method signature to `GoalBeliefView` trait with default returning 0 (for test belief views that don't implement it).

## SystemFn Integration

No new system functions. Changes integrate into existing systems:

- **Candidate generation** (`worldwake-ai`): Modified `emit_exploration_candidates()` and `select_exploration_target()` — called from existing goal generation pipeline
- **Plan search outcome tracking** (`worldwake-ai`): Modified plan failure handler to increment `AcquisitionExhaustionTracker`
- **Tracker reset** (`worldwake-ai`): Performed lazily in `emit_exploration_candidates()` when need drops below threshold
- **Travel commit** (`worldwake-systems`): Modified to push synthetic presentation ticks when ExploreLocation travel completes

All modifications are within existing system tick boundaries. No new tick phases or ordering changes.

## Component Registration

| Component | Kind | Registration | Scenario |
|-----------|------|-------------|----------|
| `ExplorationProfile` (modified) | Universal | `EntityKind::Agent` (existing) | Existing fields + 3 new fields via `AgentDef` |
| `AcquisitionExhaustionTracker` (new) | Universal, runtime-only | `EntityKind::Agent` via `component_schema.rs` | Not scenario-definable; `Default::default()` in `create_agent()` |

## Cross-System Interactions (FND-26)

| System A | System B | Interaction Medium |
|----------|----------|-------------------|
| Planner (search.rs) | Exploration gate (candidate_generation.rs) | `AcquisitionExhaustionTracker` component (stored state) |
| Exploration gate (candidate_generation.rs) | `AcquisitionExhaustionTracker` | Lazy reset when need < threshold (stored state) |
| Travel commit handler | S101 activation system | `AgentBeliefStore` synthetic presentation ticks (stored state) |
| Exploration target selection | World topology | `adjacent_places_with_travel_ticks()` (existing read-only query) |

No direct function calls between systems. All interactions are mediated through ECS components.

## Validation Patterns

### Golden Test 1: Planner Failure Unlocks Exploration

**Setup**: 2 places (Village with Well, Trail with nothing). Agent at Trail with hunger=800‰, known_recipes=["Harvest Water"] (no food recipes). Agent believes Grain exists at Village (so `need_has_known_acquisition_path()` returns true). CognitiveProfile with low max_node_expansions (forces budget exhaustion on AcquireCommodity for food).

**Expect**: After `acquisition_failure_threshold` budget-exhausted plan attempts, ExploreLocation fires. Agent travels to Village and picks up Grain. Without S102, agent loops Sleep indefinitely because exploration is suppressed by the false-positive gate.

### Golden Test 2: Multi-Hop Frontier Discovery

**Setup**: 3 places in a chain: Forest → Village → Fields (FieldPlot). Agent at Forest with hunger=800‰, known_recipes=["Harvest Grain"]. Agent knows about Forest only. `frontier_depth: 2`.

**Expect**: Agent's exploration candidates include Village (1 hop) and Fields (2 hops). Agent explores Village first (closer), then Fields. At Fields, discovers FieldPlot. Harvests Grain. Without S102, agent can only reach Village (1 hop), finds no FieldPlot, and exploration stops.

### Golden Test 3: Exploration-Chain Belief Persistence

**Setup**: 3 places: Forest → Village → Inn (WashBasin). Agent at Forest with dirtiness=800‰, exploration_profile with exploration_arrival_boost=500. No perception_profile (limited observation capacity). `frontier_depth: 2`.

**Expect**: Agent explores Village (1 hop). Synthetic presentation ticks ensure Village belief persists. Next cycle, Village's adjacency to Inn is visible. Agent explores Inn, discovers WashBasin, washes. Without S102, Village belief may decay before second exploration round.

### Golden Test 4: Counter Reset on Need Satisfaction

**Setup**: Same as Test 1, but after agent finds food and eats (hunger drops below `need_activation_threshold`), verify `AcquisitionExhaustionTracker` count for Hunger resets to 0. Agent should not spuriously explore for food when hunger is satisfied.

## Outcome

Completed on 2026-04-14.

Landed the full frontier-aware exploration follow-on across `worldwake-core`, `worldwake-ai`, `worldwake-sim`, `worldwake-systems`, and `worldwake-cli`. `ExplorationProfile` now carries `frontier_depth`, `acquisition_failure_threshold`, and `exploration_arrival_boost`; agents now store per-need `AcquisitionExhaustionTracker` state; belief-view accessors expose that tracker to AI; candidate generation stops suppressing exploration after repeated budget exhaustion, expands the frontier by BFS out to configured depth, and resets exhaustion lazily on need satisfaction; travel arrival reinforces explored-place beliefs so multi-hop chains persist; and authored scenarios can override the new exploration profile fields.

The final S102 proof landed in `crates/worldwake-ai/tests/golden_exploration.rs` with goldens for budget-exhaustion unlock, staged multi-hop frontier discovery, exploration-chain belief reinforcement, and lazy counter reset. Generated golden inventory/index/detail docs were refreshed as owned fallout.

Deviation from the original narrative: the exploration-chain persistence control run did not justify the stronger claim that zero arrival boost must stall the chain entirely under the live cadence. The implemented and archived contract was narrowed to the honest comparative effect: stronger retained intermediate-place belief state plus boosted-run second-hop discovery. The generated-doc refresh also touched adjacent inventory outputs beyond the initially expected exploration-specific files, which was accepted as in-scope fallout.

Verification completed with:
- `cargo test -p worldwake-ai --test golden_exploration -- --list`
- `cargo test -p worldwake-ai --test golden_exploration golden_s102_gate_unlock_after_budget_exhaustion -- --exact`
- `cargo test -p worldwake-ai --test golden_exploration golden_s102_multi_hop_frontier_discovery -- --exact`
- `cargo test -p worldwake-ai --test golden_exploration golden_s102_exploration_chain_belief_persistence -- --exact`
- `cargo test -p worldwake-ai --test golden_exploration golden_s102_counter_reset_on_need_satisfaction -- --exact`
- `python3 scripts/golden_inventory.py --write --check-docs`
- `cargo test -p worldwake-ai`
- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
