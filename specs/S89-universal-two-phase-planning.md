# S89: Universal Two-Phase Planning

## Summary

Remove the `goal_supports_two_phase()` whitelist that restricts the S88 two-phase (strategic + tactical) planner architecture to only `TreatWounds` and `ProduceCommodity` goals. Make two-phase planning universal for all 34 GoalKind variants by introducing a `TravelToGoal` tactical goal variant that handles spatial scoping for any goal whose strategic destination is remote. Delete the flat-search fallback path per FND-28.

The S88 architecture works — strategic planning correctly identifies remote destinations for all goal kinds. But the tactical phase ignores that information for 32 of 34 goal kinds because `goal_supports_two_phase()` gates tactical goal construction, and `TacticalSubGoal::SatisfyGoal` maps to `tactical_goal = None`. This spec closes both gaps.

**Evidence**: Simulation observer report on `cli-evaluation.ron` (seed 7777, 1440 ticks) shows:
- Guard Theron died at tick 422 from hunger — `AcquireCommodity(Water)` budget-exhausted at 224 expansions, 2085 candidates, depth 6
- Kael and Merchant Vara collapsed to sleep+relieve loops from tick 500 onward — same budget-exhaustion pattern
- 41 budget-exhausted plan searches across all agents, all for non-whitelisted goal kinds

**Phase**: 7 (Adjunct — Simulation Remediation)
**Status**: DRAFT
**Crates**: `worldwake-ai`
**Dependencies**: S88 (completed)
**Supersedes**: None (extends S88's scope)

## Design Goals

- Eliminate budget-exhaustion as a structural failure mode for multi-location goals by ensuring all GoalKind variants receive tactical scoping when their goal destination is remote
- Remove the `goal_supports_two_phase()` whitelist and flat-search fallback path (FND-28)
- Preserve graceful behavior for local-only goals (Sleep, Relieve, etc.) — when `goal_relevant_places()` returns empty or points to the current location, no tactical scoping is applied
- Maintain all S88 guarantees: belief-only planning (FND-14), per-agent cognitive parameters (FND-22), full debuggability (FND-29)

## Non-Goals

- Extending `missing_commodities()` to additional GoalKind variants — current handling (TreatWounds, ProduceCommodity) is sufficient; other goals don't have commodity prerequisites that require multi-stage strategic plans
- Fixing the FreeCarryCapacity 0-step plan dispatch deadlock — separate architectural issue (planner-execution boundary, not planner search)
- Raising `max_node_expansions` as a mitigation
- Modifying strategic planning logic or landmark extraction algorithms

## FOUNDATIONS Alignment

| Principle | How This Spec Satisfies It |
|-----------|----------------------------|
| FND-20 (Bounded Reasoning) | Two-phase decomposition makes multi-location reasoning tractable within existing expansion budgets. FND-20 explicitly authorizes "agent-local summaries, heuristics, and bounded lookahead." Universal two-phase extends this tractability to all goal kinds. |
| FND-28 (No Backward Compat) | Deletes `goal_supports_two_phase()` — the whitelist IS the backward-compatibility layer maintaining the old flat-search path alongside the new two-phase architecture. FND-28 mandates removing it, not maintaining both paths. |
| FND-14 (Belief-Only Planning) | No change — `TravelToGoal` uses `effective_place()` from `PlanningState`, which reads belief surface only. |
| FND-22 (Agent Diversity) | No change — per-agent `landmark_extraction_depth` and `preferred_operator_boost` continue to govern search behavior. |
| FND-29 (Debuggability) | Decision trace records the active tactical goal variant (`TravelToGoal { destination }` or `AcquirePrerequisite { commodity, destination }`), answering "why did the agent scope its search to this location?" |
| FND-12 (Perf Compresses Computation) | Tactical scoping reduces candidate count from 1400-2600 to ~20-50 for remote goals. All lawfully reachable plans remain reachable — search is guided by travel advancement, not pruned of legal options. |

## Section H — Causal Hooks Declaration

### H.1 — Motivating consequence gap

With S88 completed, only `TreatWounds` and `ProduceCommodity` benefit from two-phase decomposition. The remaining 32 GoalKind variants still use flat unscoped tactical search. For any multi-location goal (AcquireCommodity, Patrol, InvestigateViolation, ShareBelief, EscortToSafety, etc.), the planner budget-exhausts because the search space (1400-2600 candidates per expansion, depth 4-9) exceeds the expansion budget (150-300). This produces agent behavioral collapse (sleep-only loops), agent death (hunger/thirst deprivation), and total economic stagnation.

The S88 two-phase infrastructure already solves this problem — but only for two goal kinds. This spec extends the solution universally.

### H.2 — Entities, relations, records introduced

None. All changes are planner-internal. `TravelToGoal` is a transient tactical goal variant used during search, not stored as world state.

### H.3 — Actions or world processes that mutate them

None.

### H.4 — Information produced, travel, observability

Diagnostic only: decision trace gains `tactical_goal` field recording the active tactical goal variant. Appears in debug tooling, not in world state.

### H.5 — Conserved quantities

None affected.

### H.6 — Scarce capacities, contention

None introduced.

### H.7 — Partial failures, aftermath

Same as S88. If the tactical planner cannot find a plan at the strategic destination, this manifests as `BudgetExhausted` or `FrontierExhausted`. The agent re-plans on the next tick with updated beliefs. If no known location satisfies the goal, the strategic phase falls back to exploration or social query itineraries.

### H.8 — Positive feedback loops amplified

None.

### H.9 — Physical dampeners

N/A.

### H.10 — Cross-system interaction

None. All changes are internal to `worldwake-ai` planner search pipeline.

## Information-path analysis

No information paths introduced or modified. The `TravelToGoal` tactical goal reads existing belief state from `PlanningSnapshot`. No new information enters the agent's cognitive model.

## Positive-feedback analysis

No amplifying loops introduced. The tactical goal is computed once per planning call from the strategic milestone. It does not feed back into itself.

## Stored state vs. derived read-model list

| Item | Classification | Justification |
|------|---------------|---------------|
| `TacticalGoal::TravelToGoal` | Transient derived | Computed per planning call from strategic plan. Not stored as component. Does not survive save/load. |
| `SearchTraceMetadata::tactical_goal` | Diagnostic | Debug trace field. Not authoritative state. |

## Deliverables

### D1: `TacticalGoal::TravelToGoal` variant

**File**: `crates/worldwake-ai/src/search/mod.rs`

Add to the `TacticalGoal` enum:

```rust
TravelToGoal {
    destination: worldwake_core::EntityId,
},
```

Implement all trait/match arms:

- **`progress_barrier_satisfied`**: `state.effective_place(actor) == Some(*destination)` — barrier satisfied when actor is at destination. Local goals whose strategic destination matches the current location pass through instantly.
- **`goal_facts`**: `BTreeSet::from([PlanningFact::AtPlace(*destination)])` — enables landmark extraction to recognize travel as a goal fact.
- **Candidate filter** (`apply_tactical_candidate_filter`): When actor is not at destination, retain only travel candidates advancing toward destination (reuse `travel_advances_toward_destination`). When actor is at destination, retain non-travel candidates (goal-satisfying actions). Identical to the `Explore` / `AcquirePrerequisite` pattern.

### D2: `SatisfyGoal` maps to `TravelToGoal`

**File**: `crates/worldwake-ai/src/search/mod.rs`

In `TacticalGoal::from_strategic_step`, change `SatisfyGoal` handling from:

```rust
strategic::TacticalSubGoal::SatisfyGoal => None,
```

to:

```rust
strategic::TacticalSubGoal::SatisfyGoal => Some(Self::TravelToGoal {
    destination: step.destination,
}),
```

### D3: Delete `goal_supports_two_phase()` and unconditional tactical goal construction

**File**: `crates/worldwake-ai/src/search/mod.rs`

- Delete the `goal_supports_two_phase` function entirely.
- Change tactical goal construction from:

```rust
let tactical_goal = goal_supports_two_phase(goal).then(|| {
    TacticalGoal::from_strategic_step(...)
}).flatten();
```

to:

```rust
let tactical_goal = TacticalGoal::from_strategic_step(
    strategic_plan.as_ref().and_then(|plan| plan.steps.first()),
);
```

### D4: Heuristic scoping for `TravelToGoal`

**File**: `crates/worldwake-ai/src/search/heuristic.rs`

In `tactical_goal_places`, add `TravelToGoal` to the existing pattern:

```rust
TacticalGoal::AcquirePrerequisite { destination, .. }
| TacticalGoal::Explore { destination }
| TacticalGoal::TravelToGoal { destination } => Some(*destination),
```

### D5: Decision trace enrichment

**File**: `crates/worldwake-ai/src/search/mod.rs`

Add `tactical_goal: Option<String>` field to `SearchTraceMetadata`. Record the active tactical goal variant after construction for debuggability (FND-29).

### D6: Tests

**File**: `crates/worldwake-ai/src/search/tests.rs`

New tests:

1. **`search_acquire_commodity_uses_travel_to_goal`** — Actor at place A with no water, water source at place B (connected by travel). Create `AcquireCommodity { commodity: Water, purpose: SelfConsume }` goal. Assert plan is found (not budget-exhausted) and starts with Travel toward B.

2. **`search_patrol_uses_travel_to_goal_for_remote_place`** — Actor at place A, patrol target at place B. Create `Patrol { place: B }` goal. Assert plan routes to B without budget exhaustion.

3. **`search_investigate_uses_travel_to_goal`** — Actor at place A, violation at place B. Create `InvestigateViolation { violation_id, place: B }` goal. Assert plan routes to B.

4. **`search_local_sleep_has_no_tactical_goal`** — Actor at place with sleep affordance. Create `Sleep` goal. Verify planning succeeds. Strategic plan should be empty or have destination at current place, resulting in `tactical_goal = None` or immediate barrier satisfaction.

5. **`search_travel_to_goal_barrier_satisfied_at_destination`** — Unit test: construct `TravelToGoal { destination: X }`, verify `progress_barrier_satisfied` returns true when actor is at X, false when at Y.

6. **`search_travel_to_goal_candidate_filter`** — Unit test: verify `apply_tactical_candidate_filter` with `TravelToGoal` retains only travel-advancing candidates when actor is not at destination.

Existing S88 tests must continue to pass unchanged:
- `search_treat_wounds_uses_two_phase_pick_up_before_heal`
- `search_treat_wounds_with_zero_landmarks_preserves_two_phase_plan_shape`
- `search_produce_commodity_uses_two_phase_pick_up_before_craft`
- `search_produce_commodity_with_zero_landmarks_preserves_two_phase_plan_shape`
- `search_trace_metadata_records_two_phase_strategic_and_landmark_details`

## Behavioral Guarantees

### Local goals are unaffected

Goals with empty `goal_relevant_places()` (Sleep, Relieve, Wash, ReduceDanger, FreeCarryCapacity) produce no strategic stages. The strategic planner returns either `None` or an exploration fallback. In both cases, `tactical_goal` is `None` and the tactical search proceeds as an unscoped local search — identical to current behavior.

### Remote goals gain tactical scoping

Any goal whose `goal_relevant_places()` points to a location other than the actor's current place will receive `TravelToGoal` tactical scoping. This narrows the candidate set to travel-advancing actions, reducing per-expansion candidates from 1400-2600 to ~5-20 (only travel actions toward the destination). Once the actor reaches the destination (barrier satisfied), the tactical goal is consumed and the remaining goal-satisfying search proceeds unscoped over the local action space (~20-50 candidates).

### Prerequisites still work

Goals with commodity prerequisites (TreatWounds, ProduceCommodity) continue to produce `AcquirePrerequisite` tactical goals via `missing_commodities()`. The `SatisfyGoal → TravelToGoal` mapping only applies to the final stage — if prerequisite stages exist, they are processed first as before.

## Verification

```bash
# All existing tests pass (including S88 golden tests)
cargo test -p worldwake-ai

# New tests pass
cargo test -p worldwake-ai search_acquire_commodity_uses_travel_to_goal
cargo test -p worldwake-ai search_patrol_uses_travel_to_goal
cargo test -p worldwake-ai search_investigate_uses_travel_to_goal
cargo test -p worldwake-ai search_local_sleep_has_no_tactical_goal
cargo test -p worldwake-ai search_travel_to_goal_barrier
cargo test -p worldwake-ai search_travel_to_goal_candidate_filter

# Full workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Simulation observer re-run to verify behavioral improvement
cargo run -p worldwake-cli --bin observer -- scenarios/cli-evaluation.ron --ticks 1440 --output reports/simulation-observer-dump.md
```

Expected observer improvements:
- Zero budget-exhausted plan searches for `AcquireCommodity` goals
- Agents at Dusty Trail successfully plan travel-to-water sequences
- Guard Theron survives (or at least attempts to address hunger/thirst)
- Reduced sustained critical need durations across all agents
