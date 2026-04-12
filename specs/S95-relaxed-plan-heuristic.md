# S95: Relaxed-Plan Heuristic (FF-Style)

## Summary

Add an FF-style relaxed-plan heuristic to the tactical A* search, improving search guidance beyond the current landmark-count heuristic. The relaxed planning graph (RPG) is built over the existing `PlanningFact` vocabulary using delete-relaxation. It produces two outputs: an integer heuristic value `h_ff` (relaxed plan length) that provides more informative distance estimates, and a "helpful actions" set that provides more selective preferred-operator identification than the current landmark-based approach. This is a planner-internal optimization — no new actions, no world state mutation, no new systems.

## Phase

Phase 7: Consequence Carriers (Adjunct — Planner Infrastructure)

## Status

Draft

## Crates

- `worldwake-ai` (RPG construction, relaxed plan extraction, helpful actions, search integration, decision trace)
- `worldwake-core` (`CognitiveProfile` extension)

## Dependencies

- S88 (Two-Phase Landmark-Guided Planning) — completed. Provides the `PlanningFact`, `PlanningOperator`, `LandmarkSet`, and `DualFrontier` infrastructure this spec builds on.
- S89 (Universal Two-Phase Planning) — completed. Ensures all goal families flow through tactical planning where the RPG heuristic applies.
- S94 (Commodity-Relevance Candidate Pruning) — completed. Reduces candidate counts before search, ensuring the RPG operates on a pruned operator set.

## Design Goals

- Improve search guidance quality: the RPG-based `h_ff` estimates plan distance more accurately than counting unachieved landmarks
- Improve preferred-operator selection: "helpful actions" from the relaxed plan are a more selective subset than all landmark-achieving operators
- Maintain determinism: all RPG construction uses `BTreeSet`/`BTreeMap`, integer arithmetic, no floats
- Per-agent configurability: agents can independently enable/disable the FF heuristic via `CognitiveProfile`
- Observability: RPG heuristic values and helpful-action counts appear in decision traces

## Non-Goals

- LMCut or operator-counting heuristics — deferred to a future spec if landmark-count proves insufficient after FF deployment
- Weighted A* or anytime search — the RPG produces a better heuristic within the existing search framework, not a different search strategy
- Precomputed or cached RPGs — the RPG is built per-expansion from the current state's available operators, matching the existing per-expansion operator collection pattern
- Replacing the spatial heuristic — travel cost remains the floor; `h_ff` supplements it
- Per-successor RPG computation — `h_ff` is computed once per expansion from the expanding node's state. Computing a full RPG per-successor would be prohibitively expensive. The primary guidance improvement comes from helpful-action pruning (which differentiates between successors); the `h_ff` value differentiates nodes across different expansions.

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P12 (Performance May Compress Computation) | The RPG is a planner-internal computation optimization. It changes how the search estimates distance, not what the world means. Delete-relaxation is a standard AI planning technique for heuristic derivation. |
| P20 (Resource-Bounded Practical Reasoning) | Better heuristics let agents find feasible plans with fewer node expansions, improving reasoning efficiency within the same resource bounds (`max_node_expansions`, `beam_width`). |
| P22 (Agent Diversity) | Per-agent `use_ff_heuristic` flag allows cognitive diversity — some agents reason with stronger heuristics, others use simpler spatial-only guidance. |
| P29 (Debuggability) | `h_ff` values and helpful-action counts in decision traces make search guidance inspectable: "why did the planner prefer this expansion?" |
| P31 (Validation and Falsification) | Relaxed plan extraction provides a verifiable lower bound on plan length. Dead-end detection (RPG cannot reach goal facts) is an explicit signal, not a silent budget exhaustion. |

## Deliverables

### 1. Relaxed Planning Graph Types

New types in `crates/worldwake-ai/src/search/landmarks.rs`:

```rust
/// Result of building an RPG and extracting a relaxed plan.
pub(super) struct RelaxedPlanResult {
    /// Number of operators in the extracted relaxed plan (delete-relaxed
    /// plan length). This is the h_ff heuristic value.
    pub(super) h_ff: u32,
    /// Indices into the operators slice for layer-0 operators whose
    /// add_effects were used by the relaxed plan. These are "helpful
    /// actions" — candidates that make immediate progress toward the goal
    /// under delete-relaxation.
    pub(super) helpful_action_indices: BTreeSet<usize>,
}
```

Internal (non-pub) intermediate structures for RPG construction:

- `first_achiever: BTreeMap<PlanningFact, (u8, usize)>` — maps each fact to the `(layer, operator_index)` that first achieved it during forward RPG expansion.
- Fact layers represented as accumulated `BTreeSet<PlanningFact>` (union of all facts reachable up to each depth).

### 2. RPG Construction Algorithm

New function `compute_ff_heuristic` in `landmarks.rs`:

```rust
pub(super) fn compute_ff_heuristic(
    initial_facts: &BTreeSet<PlanningFact>,
    goal_facts: &BTreeSet<PlanningFact>,
    operators: &[PlanningOperator],
) -> Option<RelaxedPlanResult>
```

**Forward phase** (RPG construction):
1. Start with `initial_facts` as layer 0.
2. Each iteration: find operators whose preconditions are a subset of the accumulated facts (delete-relaxation — ignore `del_effects`). Union their `add_effects` into the accumulated fact set. Record the first achiever `(layer, op_index)` for each newly added fact.
3. Stop when all `goal_facts` are reached (proceed to backward phase) or when no new facts are added (return `None` — dead end detected).
4. Maximum layer depth bounded by `operators.len()` to prevent pathological cases.

**Backward phase** (relaxed plan extraction):
1. Start with `goal_facts` as open goals at the deepest layer.
2. For each open goal, look up its `first_achiever`. Mark that operator as selected. Add that operator's preconditions to the open-goal set for the preceding layer (unless already in `initial_facts`).
3. Recurse backward to layer 0.
4. `h_ff` = number of distinct selected operators.
5. Helpful actions = operators selected at layer 0 whose `add_effects` intersect the open goals at layer 1.

**Determinism**: All iteration uses `BTreeSet`/`BTreeMap`. Operator indices provide stable tie-breaking. No floats.

### 3. Search Integration

In `crates/worldwake-ai/src/search/mod.rs`, the integration follows a two-pass pattern within each expansion:

**Pass 1 — Successor construction** (existing flow, lines ~480-529): Build successors via `build_successor_detailed` as-is. Each successor receives `heuristic_ticks = max(spatial_h, landmark_h)` from `transition.rs:199-206`. Collect `successor_operators` from non-terminal successors.

**Pass 2 — RPG computation and retroactive update** (after line 529, alongside landmark extraction at ~line 612):

1. When `cognitive.use_ff_heuristic` is `true` and `successor_operators` is non-empty:
   - Compute `current_facts = planning_facts_from_state(&node.state)`
   - Compute `goal_facts` from the tactical goal (reuse `tactical_goal.goal_facts()` already called for landmarks)
   - Call `compute_ff_heuristic(&current_facts, &goal_facts, &successor_operators)`
2. If the RPG returns `Some(result)`:
   - Retroactively update each successor's `heuristic_ticks`: recompute as `max(spatial_h, result.h_ff)` where `spatial_h` is the successor's original spatial heuristic (stored or recomputed from `compute_heuristic`). This replaces the landmark-based heuristic component for this expansion's successors.
   - Use `result.helpful_action_indices` instead of `preferred_operators()` to mark successors for the preferred queue.
3. If the RPG returns `None` (dead end):
   - Leave successor `heuristic_ticks` unchanged (spatial + landmark as-is).
   - Fall back to existing landmark-based preferred operators.

**Heuristic combination**: When FF is enabled and produces a result, `h_ff` replaces `landmark_heuristic` in the successor heuristic formula — `heuristic_ticks = max(spatial_h, h_ff)`. The `compute_landmark_heuristic` result is superseded for that expansion's successors. The spatial heuristic captures real travel cost that the abstract fact space cannot represent. The RPG captures planning structure that pure spatial distance misses. Taking the max preserves admissibility properties while getting the best of both.

**RPG lifecycle**: Unlike landmark extraction (which occurs once, guarded by `landmark_set.landmarks.is_empty()`), the RPG is recomputed each expansion using that expansion's `successor_operators`. This reflects the fact that available operators change as the search progresses through different states.

When `use_ff_heuristic` is `false`, the search path is identical to current behavior — spatial heuristic + landmark preferred operators.

### 4. CognitiveProfile Extension

In `crates/worldwake-core/src/cognitive_profile.rs`, add:

```rust
/// Whether this agent uses the FF-style relaxed-plan heuristic for
/// tactical search guidance. When `false`, search uses spatial heuristic
/// only with landmark-based preferred operators (pre-S95 behavior).
#[serde(default = "default_use_ff_heuristic")]
pub use_ff_heuristic: bool,
```

Default: `true`. Agents with `landmark_extraction_depth: 0` already disable landmarks; `use_ff_heuristic` independently controls the RPG heuristic. Simple agents (low cognitive capacity) can set both to `false`.

Scenario integration: `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs` already uses `Option<CognitiveProfile>` directly (no separate `*Def` wrapper needed). Since `CognitiveProfile` derives `Deserialize` and `use_ff_heuristic` has a `#[serde(default)]` annotation, existing scenarios without the field automatically receive the default value. No changes to `types.rs` or `spawn_agent()` required.

### 5. Decision Trace Additions

In `crates/worldwake-ai/src/decision_trace.rs`, add two fields to `SearchExpansionSummary`:

```rust
/// The FF relaxed-plan heuristic value at this expansion, or `None` if
/// FF is disabled, no operators were available, or the RPG detected a
/// dead end.
pub ff_heuristic: Option<u32>,
/// Number of helpful actions identified from the relaxed plan.
pub helpful_action_count: u16,
```

These parallel the existing `landmark_heuristic: u32` and `preferred_candidates: u16` fields.

The observer binary (`crates/worldwake-cli/src/bin/observer.rs`) should format `ff_heuristic` alongside existing landmark diagnostics when present.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: N/A — the RPG is a planner-internal computation. No new information enters or leaves the agent's belief state. The heuristic reads the same `PlanningFact` set already derived from beliefs.
2. **Positive-feedback analysis**: No amplifying loops. The RPG is a stateless computation per expansion — it does not persist, accumulate, or feed back into world state.
3. **Concrete dampeners**: N/A (no loops).
4. **Stored state vs. derived read-model**: The RPG and relaxed plan are transient derived computations, discarded after each expansion. `h_ff` is used only as a search-node ordering parameter. No new authoritative state is introduced.

## SystemFn Integration

No new SystemFn. The RPG is computed inside the existing `search_plan` function during node expansion. It does not add a new tick-phase system.

## Component Registration

One existing component modified:

- `CognitiveProfile` — add `use_ff_heuristic: bool` field. No new component registration needed; `CognitiveProfile` is already registered on `EntityKind::Agent`.

Scenario contract: `AgentDef.cognitive_profile: Option<CognitiveProfile>` in `types.rs` handles the new field automatically via serde defaults. No `*Def` wrapper needed. Universal profile with `Default` impl already exists.

## Cross-System Interactions

None. The RPG is entirely planner-internal. It reads `PlanningFact`s derived from the agent's belief snapshot (already encapsulated in `PlanningState`) and produces search-ordering parameters consumed within the same `search_plan` call. No cross-crate or cross-system dependencies are introduced.

## Profile-Driven Parameters

| Parameter | Profile | Type | Default | Purpose |
|-----------|---------|------|---------|---------|
| `use_ff_heuristic` | `CognitiveProfile` | `bool` | `true` | Enable/disable RPG heuristic per agent |
| `landmark_extraction_depth` | `CognitiveProfile` | `u8` | `4` | (Existing) Controls landmark chain depth; interacts with FF as fallback when FF returns `None` |

## Validation and Falsification

### Unit Tests (in `landmarks.rs`)

1. **RPG fixpoint — no operators**: When no operators are provided and goal facts are not in initial facts, `compute_ff_heuristic` returns `None`.
2. **Goal already satisfied**: When goal facts are a subset of initial facts, `h_ff = 0`, helpful actions empty.
3. **Linear chain**: A→B→C with two operators, `h_ff = 2`. Helpful actions contain only the layer-0 operator.
4. **Delete-relaxation correctness**: Operator with `del_effects` on a needed fact still allows parallel achievement under relaxation. Verify `h_ff` counts correctly.
5. **Helpful action selectivity**: Only layer-0 operators whose effects were used appear in the helpful set — not all applicable operators.
6. **Determinism**: Same inputs produce same `h_ff` and same helpful-action indices across runs.

### Integration Tests (in `search/` test modules)

7. **FF vs spatial heuristic combination**: Verify `heuristic_ticks = max(spatial_h, h_ff)` on successor nodes after retroactive update.
8. **FF disabled via profile**: Agent with `use_ff_heuristic: false` produces `ff_heuristic: None` in expansion summaries.
9. **Fallback on dead end**: When RPG returns `None`, verify search uses landmark-based preferred operators and leaves successor heuristic_ticks unchanged.

### Golden Test Extensions

10. **Existing water-acquisition golden tests**: Assert `ff_heuristic` is populated and `helpful_action_count > 0` in expansion summaries for the `AcquireCommodity(Water)` scenarios that exercise multi-step tactical search.
