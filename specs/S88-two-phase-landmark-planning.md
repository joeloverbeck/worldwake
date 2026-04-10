# S88: Two-Phase Landmark-Guided Planning

## Summary

Replace the GOAP planner's flat A* forward search with a two-phase architecture: a belief-driven **strategic planner** that sequences location visits, followed by a **tactical planner** that uses landmark-based preferred operators to find concrete action plans at each location.

The current planner generates 1400–2600 search candidates per expansion (cartesian product of action_defs × target_entities × payload_variants), consuming its entire expansion budget (224–300) at depth 0–1. This prevents multi-location plans (travel + acquire + consume) from ever being found, causing behavioral collapse across all agents.

This spec supersedes S86 (Planner Pre-Expansion Candidate Heuristics) and S87 (Observer Diagnostic Gaps). S86's pre-expansion scoring is a heuristic patch on the symptom; this spec eliminates the architectural root cause. S87's observer diagnostics are independent tooling that may be reconsidered as a separate future spec.

**Phase**: 7 (Adjunct — Simulation Remediation Phase 3)
**Status**: Draft
**Crates**: `worldwake-ai`, `worldwake-core`
**Dependencies**: None (operates on existing planner infrastructure from E13, S53, S73, S83)
**Supersedes**: S86, S87

## Design Goals

- Enable agents to find multi-location plans (travel to location B + acquire resource + consume) within the existing expansion budget
- Reduce per-expansion candidate count from 1400+ to ~20–50 by decomposing multi-location goals into locality-scoped tactical sub-problems via the strategic planner
- Provide landmark-based search guidance that understands goal substructure (harvest precedes pick_up precedes consume)
- Operate on agent beliefs, never world truth (FND-14)
- Make cognitive parameters per-agent for diversity (FND-22)
- Maintain full debuggability of planning decisions (FND-29)

## Non-Goals

- Hierarchical Task Networks or authored decomposition methods (violates FND-1)
- Raising `max_node_expansions` as the primary fix (treats the symptom)
- Modifying the authoritative action framework or world validation
- Observer diagnostic improvements (deferred to a future spec)

## Research Basis

- Richter & Westphal (2010), "The LAMA Planner," JAIR 39:127–177 — landmark extraction, preferred operators, dual open lists
- Richter & Helmert (2009), "Preferred Operators and Deferred Evaluation in Satisficing Planning," ICAPS — deferred heuristic evaluation
- Maggiore et al. (2013), "LGOAP: Adaptive Layered Planning for Real-Time Videogames," CIG — layered GOAP communication pattern (adapted; hand-authored layers replaced with auto-derived strategic/tactical split)
- Orkin (2004), "Applying Goal-Oriented Action Planning to Games" — spatial planning via precondition chaining in F.E.A.R.
- Kaelbling & Lozano-Perez (2011), "Hierarchical Planning in the Now," ICRA — domain restriction via suggesters

Key finding: no shipped GOAP game has successfully used flat A* forward search with a branching factor of 1400+. F.E.A.R. operated with ~20 actions. The architecture must change.

## FOUNDATIONS Alignment

| Principle | How This Spec Satisfies It |
|-----------|----------------------------|
| FND-1 (Maximal Emergence) | Both phases use search, not authored decompositions. Plan structure emerges from beliefs + goals. No pre-scripted action sequences. |
| FND-3 (Concrete State) | Landmarks are concrete state propositions (has(Water), at(Well_location)), not abstract scores. Derived from goal structure and beliefs. |
| FND-7 (Locality) | Strategic planner uses only believed travel costs and believed place contents from `PlanningSnapshot`. No global queries. |
| FND-12 (Perf Compresses Computation) | Strategic decomposition and landmarks reduce search computation. All lawfully reachable plans remain reachable; search is guided, not pruned of legal options. |
| FND-14 (Belief-Only Planning) | Both phases operate on `PlanningSnapshot` belief surface. Landmarks are belief-dependent — an agent who doesn't know about a Well has no landmark requiring it. |
| FND-16 (Ignorance First-Class) | Missing beliefs produce exploration itineraries. Wrong beliefs produce belief contradiction on arrival, triggering replanning. |
| FND-20 (Bounded Reasoning) | FND-20 explicitly authorizes "agent-local summaries, heuristics, and bounded lookahead derived from accessible belief state." Landmarks are exactly this: bounded lookahead over the agent's believed operator effects. The strategic phase is an agent-local reasoning summary. |
| FND-22 (Agent Diversity) | `landmark_extraction_depth` on `CognitiveProfile` and `preferred_operator_boost` on `ExecutionBudget` are per-agent parameters. A shrewd agent extracts deeper landmark chains and follows them systematically; a simple agent uses shallower extraction. |
| FND-28 (No Backward Compat) | S86 and S87 are superseded, not shimmed. `max_candidates_per_expansion` (S86) is not added. |
| FND-29 (Debuggability) | Landmarks, strategic plan, preferred operator status, and landmark achievement are all recorded in the decision trace. Answers: "why did the agent go there?" (strategic plan) and "why did it try that action first?" (preferred operator from landmark). |

## Section H — Causal Hooks Declaration

### H.1 — Motivating consequence gap

The planner budget-exhausts on every goal requiring travel (AcquireCommodity, ProduceCommodity, TreatWounds at remote locations), producing behavioral collapse where agents cycle sleep+relieve. In the cli-evaluation scenario: Guard Theron dies from hunger, Kael and Merchant Vara collapse to sleep loops, Forager Lina goes idle for 700+ ticks. Zero trade, zero crafting, 3 of 5 locations never visited. The root cause is that 1400–2600 search candidates at each expansion consume the entire budget at depth 0, preventing multi-step plans from being explored.

### H.2 — Entities, relations, records introduced

No new world-state entities, relations, or records. All new state is planner-internal (agent cognitive model):

- `StrategicPlan` — transient planning result, not stored as component. Contains ordered `(destination, tactical_sub_goal, estimated_travel_cost)` tuples.
- `LandmarkSet` — transient planning result. Contains fact landmarks with orderings, computed per planning call from the agent's believed operator set.
- New fields on existing components (see D1, D2).

### H.3 — Actions or world processes that mutate them

None. This is entirely planner-internal. No new actions or world processes.

### H.4 — Information produced, travel, observability

Diagnostic only: decision trace gains strategic plan summary, landmark set, and per-expansion preferred operator status. These appear in debug tooling, not in world state.

### H.5 — Conserved quantities

None affected.

### H.6 — Scarce capacities, contention

None introduced.

### H.7 — Partial failures, aftermath

**Tactical failure at planned location**: If the tactical planner cannot find a plan for the strategic sub-goal at the destination, this manifests as `BudgetExhausted` or `FrontierExhausted` — the same failure modes as today. The strategic planner re-runs with updated beliefs (perception updated on arrival), producing a revised itinerary. If no known location satisfies the goal, the agent falls back to exploration or social query goals.

**Landmark extraction produces empty set**: If no landmarks can be extracted (goal is trivially satisfiable or operator set is degenerate), the tactical planner runs without preferred operators — equivalent to the current planner behavior. This is a graceful degradation, not a failure.

### H.8 — Positive feedback loops amplified

None.

### H.9 — Physical dampeners

N/A.

### H.10 — Cross-system interaction

None. The change is internal to the planner search pipeline in `worldwake-ai`. It reads from `CognitiveProfile` and `ExecutionBudget` (core components) and `PlanningSnapshot` (AI-internal belief surface). No cross-system calls.

## Information-path analysis

No information paths are introduced or modified. The strategic planner reads existing belief state from `PlanningSnapshot` (which is populated by the perception system through existing information paths). Landmarks are derived from this same belief state. No new information enters the agent's cognitive model.

## Positive-feedback analysis

No amplifying loops introduced. The strategic planner runs once per planning call and produces a fixed-length itinerary. Landmark extraction runs once per tactical planning call. Neither feeds back into itself.

## Stored state vs. derived read-model list

| Item | Classification | Justification |
|------|---------------|---------------|
| `StrategicPlan` | Transient derived | Computed per planning call from beliefs. Not stored as component. Does not survive save/load. |
| `LandmarkSet` | Transient derived | Computed per tactical planning call from believed operators. Not stored. Does not survive save/load. |
| `landmark_extraction_depth` on `CognitiveProfile` | Authoritative stored | Per-agent cognitive parameter. Scenario-definable. |
| `preferred_operator_boost` on `ExecutionBudget` | Authoritative stored | Per-agent search parameter. Scenario-definable. |
| Decision trace entries | Diagnostic | Debug-only output. Not authoritative state. |

## Deliverables

### D1: `landmark_extraction_depth` on `CognitiveProfile`

**File**: `crates/worldwake-core/src/cognitive_profile.rs`

Add to `CognitiveProfile`:

```rust
/// Maximum depth of landmark chain extraction during tactical planning.
/// Higher values produce more landmarks for better search guidance at
/// increased extraction cost. 0 = no landmarks (preferred operators disabled).
pub landmark_extraction_depth: u8,
```

Default: `4`. Range: 0–8. Scenario-definable: the CLI's `AgentDef` uses `Option<CognitiveProfile>` directly (`crates/worldwake-cli/src/scenario/types.rs:86`), and `spawn_agent()` applies it via `unwrap_or_default()` (`crates/worldwake-cli/src/scenario/mod.rs:368`). New fields are picked up automatically — no CLI changes needed.

### D2: `preferred_operator_boost` on `ExecutionBudget`

**File**: `crates/worldwake-core/src/execution_budget.rs`

Add to `ExecutionBudget`:

```rust
/// Number of consecutive preferred-operator expansions before alternating
/// to the regular queue. Higher values focus search more aggressively on
/// landmark-derived actions. 0 = no boosting (dual queue alternates 1:1).
pub preferred_operator_boost: u8,
```

Default: `2`. Range: 0–8. Scenario-definable: same mechanism as D1 — `AgentDef` uses `Option<ExecutionBudget>` directly (`types.rs:88`), and `spawn_agent()` applies via `unwrap_or_default()` (`scenario/mod.rs:369`). No CLI changes needed.

### D3: Strategic planner

**File**: `crates/worldwake-ai/src/search/strategic.rs` (new)

The strategic planner operates on the agent's belief model to produce a location-visit itinerary.

**Input**:
- `PlanningSnapshot` — the agent's belief surface
- `GroundedGoal` — the concrete goal being planned for
- `ExecutionBudget` — agent's cognitive parameters

**State representation**: `(believed_agent_location, set_of_unvisited_goal_relevant_places)`. The state is abstract — it tracks only which location the agent believes it is at and which goal-relevant locations remain to visit.

**Operators**: Only `Travel(destination)` where:
- Precondition: agent has a belief about the destination (the place exists in the agent's belief store)
- Cost: `min_perceived_travel_cost(current, destination)` from `PlanningSnapshot`
- Effect: agent is at destination

**Goal**: The agent is at a location where `goal_relevant_places()` indicates the concrete goal can be satisfied. Uses the existing `goal_relevant_places()` and `prerequisite_places()` infrastructure from `goal_model.rs`.

**Search**: Best-first over the believed place graph. With 5–15 known places and travel as the only operator, this completes in microseconds. No beam truncation needed. Budget: at most `max_prerequisite_locations * 2` expansions (trivially small).

**Output**:

```rust
pub struct StrategicPlan {
    pub steps: Vec<StrategicStep>,
}

pub struct StrategicStep {
    pub destination: EntityId,
    pub sub_goal: TacticalSubGoal,
    pub estimated_travel_ticks: u32,
}

pub enum TacticalSubGoal {
    /// Achieve the original concrete goal at this location
    SatisfyGoal,
    /// Acquire a prerequisite resource at this location before continuing
    AcquirePrerequisite(CommodityKind),
    /// Explore this location to update beliefs
    Explore,
    /// Ask co-located agents about resource locations
    SocialQuery(CommodityKind),
}
```

**When beliefs are empty** (no known location has the needed resource):
1. If `ExplorationProfile` is present and exploration is enabled: produce exploration itinerary — visit nearest adjacent places the agent has not recently visited. The strategic planner derives exploration targets from `ExplorationProfile` and the agent's believed adjacent places in `PlanningSnapshot`, independently of the candidate generation pipeline (the private `emit_exploration_candidates()` in `candidate_generation.rs` serves a different purpose — it generates tactical exploration candidates, not strategic exploration itineraries).
2. If co-located agents exist: include `TacticalSubGoal::SocialQuery` targeting `AskWitness` or `Consult` actions.
3. If neither applies: return empty strategic plan. The planner falls through to local-only tactical planning (current behavior).

### D4: Locality-scoped candidate generation via strategic decomposition

**File**: `crates/worldwake-ai/src/search/candidates.rs` (integration point)

**Note on existing locality scoping**: The current affordance system already provides locality scoping through `RuntimeBeliefView`. Specifically:
- `get_affordances_for_defs()` (`affordance_query.rs:60`) passes the `PlanningState` as a `RuntimeBeliefView`, and `enumerate_targets()` (`affordance_query.rs:463`) uses `view.effective_place(actor)` to scope `EntityAtActorPlace` targets to the agent's hypothetical location.
- `goal_synthesized_candidates()` (`candidates.rs:302`) passes `state.effective_place(actor)` to `synthesized_root_candidate_targets()`, which locality-filters.
- `planner_only_candidates()` (`planner_ops.rs:831`) operates on direct possessions only — inherently scoped.

**How strategic decomposition achieves candidate reduction**: The candidate reduction from 1400+ to ~20–50 comes from the strategic phase (D3) decomposing a multi-location goal into locality-scoped tactical sub-problems. Each tactical search targets a specific sub-goal at a specific location, which narrows the `relevant_defs` set (only operators relevant to the sub-goal are enumerated). Additionally, remote locations typically have fewer entities than the starting location. The existing locality scoping mechanisms ensure candidates at each tactical location are correctly filtered.

**Integration**: When a strategic plan is active, the tactical search receives the narrowed sub-goal context. The existing `search_candidates()` function continues to use `relevant_defs` (from `GoalDispatchDeclaration.relevant_ops`) to scope which action defs are enumerated. No modifications to `search_candidates()` itself are required — the candidate reduction emerges from the sub-goal narrowing, not from changes to the candidate generation pipeline.

### D5: Landmark extraction

**File**: `crates/worldwake-ai/src/search/landmarks.rs` (new)

Landmark extraction adapted from LAMA (Richter & Westphal 2010) for GOAP operators.

**Input**:
- Initial state: the planning state at the start of tactical search (believed local state)
- Goal facts: the set of propositions that must be true for the tactical sub-goal to be satisfied
- Operators: the set of GOAP operators available at the current location (from candidate generation), represented as `(preconditions: BTreeSet<Fact>, add_effects: BTreeSet<Fact>, del_effects: BTreeSet<Fact>)`
- `landmark_extraction_depth`: maximum chain length from `CognitiveProfile`

**Fact representation**: Planning propositions represented as an enum:

```rust
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum PlanningFact {
    /// Agent is at this place
    AtPlace(EntityId),
    /// Agent possesses this commodity
    HasCommodity(CommodityKind),
    /// Agent possesses this specific entity
    HasEntity(EntityId),
    /// A facility at the current place is available for use
    FacilityAvailable(EntityId),
    /// A specific entity exists at the current place
    EntityPresent(EntityId),
    /// Agent's need is below threshold (goal-terminal condition)
    NeedSatisfied(HomeostaticNeedId),
}
```

**Algorithm** (delete-relaxation landmark extraction):

```rust
pub fn extract_landmarks(
    initial_facts: &BTreeSet<PlanningFact>,
    goal_facts: &BTreeSet<PlanningFact>,
    operators: &[PlanningOperator],
    max_depth: u8,
) -> LandmarkSet
```

1. Initialize `landmarks` with all `goal_facts`. Add to processing queue.
2. For each landmark `psi` in the queue (up to `max_depth` iterations):
   - If `psi` is true in `initial_facts`: skip (already achieved).
   - Find all operators whose `add_effects` contain `psi` (achievers).
   - If no achievers exist: `psi` is unachievable — mark and continue.
   - Compute `shared_preconditions`: intersection of all achievers' preconditions.
   - For each shared precondition `prec` not already in `landmarks`:
     - Add `prec` to `landmarks` and queue.
     - Add ordering `(prec, psi)` — `prec` must be achieved before `psi`.
3. Return `LandmarkSet { landmarks, orderings }`.

**Preferred operator derivation**:

```rust
pub fn preferred_operators(
    landmarks: &LandmarkSet,
    current_facts: &BTreeSet<PlanningFact>,
    candidates: &[SearchCandidate],
    operators: &[PlanningOperator],
) -> BTreeSet<usize>  // indices into candidates
```

An operator is **preferred** if it achieves a landmark whose ordering predecessors are all already achieved in `current_facts`. Returns the set of candidate indices that are preferred.

**Computational cost**: For ~50–100 operators and ~30–50 facts, extraction completes in microseconds. The queue processes at most `max_depth × |shared_preconditions|` iterations. This is feasible per-planning-call.

### D6: Dual open list with preferred operator boosting

**File**: `crates/worldwake-ai/src/search/frontier.rs`

Replace the single `BinaryHeap<FrontierEntry>` with a dual open list:

```rust
pub struct DualFrontier {
    regular: BinaryHeap<FrontierEntry>,
    preferred: BinaryHeap<FrontierEntry>,
    boost_remaining: u8,
    use_preferred_next: bool,
}
```

**Insert**: All successors go into `regular`. Successors generated by preferred operators also go into `preferred`.

**Pop**: If `boost_remaining > 0` or `use_preferred_next`, pop from `preferred` (decrement boost). Otherwise pop from `regular`. Toggle `use_preferred_next` after each pop. This alternates: preferred → regular → preferred → ..., with burst priority for preferred when `boost_remaining > 0`.

**Boost trigger**: When the search finds a successor with a lower heuristic than any previously seen, set `boost_remaining = preferred_operator_boost` from `ExecutionBudget`. This temporarily prioritizes preferred successors when progress is being made.

**Fallback**: If `preferred` is empty when preferred pop is attempted, fall through to `regular`. If both are empty, return `None` (frontier exhausted).

The existing `FrontierEntry` struct and ordering logic are unchanged. The dual list only changes which entries are popped first.

### D7: Landmark count heuristic

**File**: `crates/worldwake-ai/src/search/heuristic.rs`

Add a landmark count heuristic alongside the existing spatial distance heuristic:

```rust
pub fn compute_landmark_heuristic(
    landmarks: &LandmarkSet,
    current_facts: &BTreeSet<PlanningFact>,
) -> u32
```

Returns the count of landmarks not yet achieved in `current_facts` whose ordering predecessors ARE all achieved (i.e., actionable landmarks). This counts "how many mandatory milestones remain."

**Integration with existing heuristic**: The combined heuristic is:

```rust
let spatial_h = compute_heuristic(snapshot, state, goal_relevant_places);
let landmark_h = compute_landmark_heuristic(&landmarks, &current_facts);
let combined_h = spatial_h.max(landmark_h);
```

Taking the max of both heuristics preserves the existing search's guidance properties. The spatial heuristic provides distance guidance; the landmark heuristic provides subgoal-structure guidance. Note: the current search uses beam truncation (`ExecutionBudget.beam_width`), so optimality is already not guaranteed — the combined heuristic improves guidance quality within the existing satisficing search framework.

### D8: Integration into search_plan()

**File**: `crates/worldwake-ai/src/search/mod.rs`

Modify `search_plan()` to:

1. **Run strategic planning** before the tactical search loop:
   ```rust
   let strategic_plan = strategic::plan(snapshot, goal, execution_budget);
   ```
   If a strategic plan is found with steps, the first step's destination becomes the travel target. The tactical search proceeds with the strategic plan as context.

2. **Extract landmarks** for the tactical sub-goal:
   ```rust
   let landmark_set = if cognitive.landmark_extraction_depth > 0 {
       landmarks::extract_landmarks(&initial_facts, &goal_facts, &operators, cognitive.landmark_extraction_depth)
   } else {
       LandmarkSet::empty()
   };
   ```

3. **Replace frontier** with `DualFrontier`:
   ```rust
   let mut frontier = DualFrontier::new(execution_budget.preferred_operator_boost);
   ```

4. **In the expansion loop**: After generating candidates (locality-scoped via the existing `RuntimeBeliefView` mechanism), compute preferred operators from landmarks and insert into the dual frontier accordingly.

5. **Heuristic computation**: Use combined spatial + landmark heuristic.

### D9: Decision trace enrichment

**File**: `crates/worldwake-ai/src/decision_trace.rs`

Add strategic plan and landmark trace data to `PlanAttemptTrace` (per-attempt, since each goal attempt may produce a different strategic plan):

```rust
pub struct PlanAttemptTrace {
    // ... existing fields ...
    pub strategic_plan: Option<Vec<StrategicStepTrace>>,
    pub landmarks_extracted: u16,
    pub landmark_orderings: u16,
}

pub struct StrategicStepTrace {
    pub destination: EntityId,
    pub sub_goal: String,  // debug representation
    pub estimated_travel_ticks: u32,
}
```

Add per-expansion landmark guidance data to `SearchExpansionSummary` (per-expansion, since preferred operator status changes at each expansion):

```rust
pub struct SearchExpansionSummary {
    // ... existing fields ...
    pub preferred_candidates: u16,
    pub landmark_heuristic: u32,
}
```

### D10: Golden tests

**File**: `crates/worldwake-ai/tests/golden_two_phase_planning.rs` (new)

**Scenario 1 — Multi-location resource acquisition**: Two places: barren location A (agent starts here), resource location B (Well + OrchardRow, 1 hop away). Agent has beliefs about location B (knows Well exists). Assert: (a) strategic plan contains B as destination, (b) tactical plan at B finds queue→draw→drink sequence, (c) full plan found within expansion budget (not budget-exhausted), (d) candidate count per expansion < 100.

**Scenario 2 — Belief-only planning (no omniscience)**: Agent at location A. Resource exists at location C (2 hops away) but agent has NO beliefs about C. Assert: (a) strategic plan does NOT include C, (b) agent produces exploration or social query itinerary, (c) no plan to the unknown resource is found. This proves FND-14 compliance — the planner never uses information the agent doesn't have.

**Scenario 3 — Landmark correctness**: AcquireCommodity(Water) goal at location with Well. Assert: (a) landmarks include AtPlace(well_location) and HasCommodity(Water), (b) ordering: AtPlace precedes HasCommodity, (c) preferred operators at depth 0 include Travel to well location.

**Scenario 4 — Agent cognitive diversity**: Two agents at same barren location with different `landmark_extraction_depth` (2 vs 6). Both have beliefs about remote resource. Assert: agent with depth 6 finds multi-step plan more reliably than agent with depth 2 (different expansion profiles, potentially different plan-found rates).

**Scenario 5 — Regression guard (branching factor)**: Reproduce the 1400+ candidate scenario from the simulation observer report. Assert: with strategic decomposition, the tactical search at each location produces fewer than 100 candidates per expansion. Without strategic decomposition (single flat search), candidate count exceeds 1000.

**Scenario 6 — Graceful degradation**: Agent with `landmark_extraction_depth = 0` (landmarks disabled). Assert: planner still functions using spatial heuristic only, equivalent to current behavior. Strategic decomposition still reduces candidate count via sub-goal narrowing.

## SystemFn Integration

No new SystemFn. Strategic planning and landmark extraction are internal to `search_plan()`, which is called from the existing agent decision tick via `agent_tick.rs`.

## Component Registration

`CognitiveProfile` and `ExecutionBudget` are already registered on `EntityKind::Agent`. New fields are added to existing structs with defaults. Per spec-drafting-rules Section 5: both are universal profiles with `Default` impls. The new fields do not change the registration — they extend existing components.

## Cross-System Interactions

None. The change is entirely within the `worldwake-ai` planner search pipeline. It reads from `CognitiveProfile` and `ExecutionBudget` (core components, read-only) and `PlanningSnapshot` (AI-internal belief surface, read-only). No cross-system calls or state mutations.

## Profile-Driven Parameters

| Parameter | Component | Type | Default | Range | Effect |
|-----------|-----------|------|---------|-------|--------|
| `landmark_extraction_depth` | `CognitiveProfile` | `u8` | 4 | 0–8 | Max landmark chain length. 0 disables landmarks (no preferred operators). Higher = deeper lookahead, better guidance, slightly more extraction cost. |
| `preferred_operator_boost` | `ExecutionBudget` | `u8` | 2 | 0–8 | Consecutive preferred-operator expansions after progress. Higher = more aggressive landmark following. 0 = alternates 1:1. |
