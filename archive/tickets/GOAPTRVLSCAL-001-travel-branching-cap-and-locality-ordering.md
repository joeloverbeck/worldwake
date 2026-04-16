# GOAPTRVLSCAL-001: Travel Branching Cap and Locality-First Ordering

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — AI planner search loop, CognitiveProfile
**Deps**: None (the immediate Sleep/Wash/speculation fixes from the survival-scattered work are already merged)

## Problem

The GOAP planner's search space scales poorly with topology size because Travel operators create O(N) candidates per expansion for N reachable places. Goals that legitimately need Travel (e.g., Wash, AcquireCommodity, Relieve) still explore all reachable destinations even when most are irrelevant to the goal. The survival-scattered scenario work fixed three instances of this:

1. **Sleep** — removed Travel from `SLEEP_OPS` (Sleep has no location precondition)
2. **Wash** — fixed `goal_relevant_places` to return WashBasin locations (enables travel pruning when known)
3. **speculative_acquisition** — disabled in scattered scenario; the feature generated AcquireCommodity candidates at every known place regardless of evidence, crowding out ExploreLocation and violating FND-14

The general concern remains: **Wash still exhausts budget before the agent discovers any WashBasin** (goal_relevant_places returns empty when the basin is undiscovered, disabling travel pruning — identical to the Sleep bug). The survival-scattered test excludes Wash from the budget exhaustion assertion for this reason. Additionally, as topologies grow beyond 6 places, any goal with Travel in its operators risks budget exhaustion if travel pruning has insufficient signal.

Two complementary techniques from the GOAP/automated planning literature address this:

1. **Travel branching cap** (per-agent profile parameter): Hard-limit the number of Travel candidates expanded per node, retaining only those with lowest perceived cost to the nearest goal-relevant place. This is a safety valve that prevents pathological blowup even when heuristic pruning is weak — including the Wash-before-discovery case.

2. **Locality-first preferred guidance**: Use the existing `DualFrontier` preference lane to keep FF-helpful non-Travel successors preferred unconditionally, while FF-helpful Travel successors become preferred only when they are still goal-directed under the planner's perceived travel-cost model. This narrows relaxed-plan guidance toward "do Y right here" before "travel to X, then do Y" without adding a second successor-construction pass or suppressing lawful travel branches outright.

Both techniques are profile-driven (per-agent tunable via `CognitiveProfile`), aligned with FND-20 (resource-bounded practical reasoning) and FND-22 (agent diversity through concrete variation).

## Related

- **SPECACQRMV-001**: Removes `speculative_acquisition` from the architecture entirely. That ticket handles the FND-14 violation and oscillation-loop pathology discovered during the survival-scattered investigation. This ticket (GOAPTRVLSCAL-001) addresses the remaining Travel scaling concern for goals that legitimately include Travel but lack goal-relevant places before discovery (primarily Wash before the agent finds a WashBasin).

## Assumption Reassessment (2026-04-16)

1. **Current planner search loop**: `crates/worldwake-ai/src/search/mod.rs:303-802` — forward A* with FF heuristic, `DualFrontier` preferred/regular queues, beam filtering. Candidates generated in `search/candidates.rs`. Travel pruning in `prune_travel_away_from_goal_with_expansion_trace` (search/mod.rs:497-504). Confirmed 2026-04-16.
2. **CognitiveProfile**: `crates/worldwake-core/src/cognitive_profile.rs` — contains `max_candidates_per_expansion`, `max_node_expansions`, `landmark_extraction_depth`, and `use_ff_heuristic`. No travel-specific branching cap exists. Confirmed 2026-04-16.
3. **ExecutionBudget**: `crates/worldwake-core/src/ai.rs` — contains `beam_width`, `preferred_operator_boost`, `max_prerequisite_locations`. Confirmed 2026-04-16.
4. **PlannerOpKind::Travel**: Used by 20+ goal dispatch declarations in `goal_dispatch_decl.rs`. Travel candidates come from `get_affordances_for_defs` which returns one affordance per reachable neighbor. Confirmed 2026-04-16.
5. **DualFrontier**: `search/mod.rs` — preferred queue holds successors from FF-marked helpful actions; regular queue holds the rest. The `preferred_operator_boost` biases selection toward preferred. Already supports the priority separation needed for locality-first ordering. Confirmed 2026-04-16.
6. **Survival-scattered scenario**: Reassessed from a live 1024-expansion baseline, then updated in this ticket to run with `max_travel_candidates_per_expansion: 4` and `max_node_expansions: 640`. `cargo test -p worldwake-ai --test golden_survival_scattered` passes with that lower budget. Confirmed 2026-04-16.
7. This is a planner search efficiency ticket. No golden scenario motivates it directly — it is a preventive architectural improvement informed by the survival-scattered investigation.

## Architecture Check

1. **Travel branching cap**: A single new field on `CognitiveProfile` (`max_travel_candidates_per_expansion: Option<u16>`) with ~20 lines of filtering logic after travel pruning. When the cap is reached, retain only the N lowest-cost travel candidates by perceived travel cost to the nearest goal-relevant place. `None` means no cap (current behavior). This is cleaner than inflating `max_node_expansions` per scenario, which wastes budget on all goals rather than constraining the problematic dimension.
2. **Locality-first preferred guidance**: Leverages the existing `DualFrontier` and `preferred_operator_boost` mechanism. Non-Travel candidates that appear in the FF relaxed plan are always preferred. Travel candidates are preferred only when they appear in the relaxed plan AND lead toward a goal-relevant place. This refines the existing FF heuristic integration without introducing a second expansion phase or changing the lawful successor set.
3. No backward-compatibility shims. The `Option<u16>` default of `None` preserves current behavior for all existing scenarios and profiles. Scenarios can opt in by setting the field.

## Verification Layers

1. **Travel cap reduces candidate count** → focused unit test in `search/` that verifies candidate count after filtering matches the cap
2. **Locality-first ordering prefers local actions** → focused unit test showing non-Travel candidates enter preferred queue when FF marks them helpful
3. **No regression on existing golden tests** → `cargo test -p worldwake-ai` (all golden suites)
4. **Survival-scattered passes with lower max_node_expansions** → reduce scattered scenario from 800 to 640 expansions and verify all 5 golden tests still pass (validates the cap is effective)
5. Single-layer ticket (planner search internals). No cross-system or authoritative state changes.

## What to Change

### 1. Add `max_travel_candidates_per_expansion` to `CognitiveProfile`

Add `pub max_travel_candidates_per_expansion: Option<u16>` to `CognitiveProfile` in `crates/worldwake-core/src/cognitive_profile.rs`. Default: `None` (no cap). Scenario-definable via `AgentDef`.

### 2. Travel candidate capping in search loop

In `crates/worldwake-ai/src/search/mod.rs`, after `prune_travel_away_from_goal_with_expansion_trace` and before building successors: if the cap is set and travel candidates exceed it, sort travel candidates by `min_perceived_travel_cost_to_any` of the goal-relevant places, retain only the top N.

### 3. Locality-first preferred operator boost

In `crates/worldwake-ai/src/search/mod.rs`, when assigning successors to the `DualFrontier`: non-Travel candidates that are FF-helpful go to the preferred queue unconditionally. Travel candidates that are FF-helpful go to the preferred queue only if their destination is a goal-relevant place or on a shortest path to one. This ticket does not add a second local-vs-travel successor-construction phase; it narrows the existing preferred-lane guidance.

### 4. Scenario verification

Update `scenarios/survival-scattered.ron` to set `max_travel_candidates_per_expansion: 4` (or similar) and reduce `max_node_expansions` back to 640 to verify the cap is effective.

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (modify — add field + serde/default coverage)
- `crates/worldwake-core/src/delta.rs` (modify — sample `CognitiveProfile` literal fallout)
- `crates/worldwake-ai/src/search/mod.rs` (modify — travel capping + FF preferred-queue gating)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — add trace filter reason for capped travel candidates)
- `crates/worldwake-ai/src/search/tests.rs` (modify — focused travel-cap and locality-ordering tests)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — `CognitiveProfile` literal fallout)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — `CognitiveProfile` literal fallout)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — `CognitiveProfile` literal fallout)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — `CognitiveProfile` literal fallout)
- `crates/worldwake-ai/src/goal_model.rs` (modify — `CognitiveProfile` literal fallout)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — scenario default-omission coverage)
- `crates/worldwake-ai/tests/golden_survival_scattered.rs` (modify — live budget commentary)
- `scenarios/survival-scattered.ron` (modify — set cap, reduce max_node_expansions)

## Out of Scope

- Lazy travel injection (full on-demand operator generation) — longer-term refactor
- Strategic plan as travel gate (hard constraint on travel destinations from strategic planner) — separate ticket
- Fixes to specific goal dispatch declarations — Sleep and Wash already fixed

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: travel cap reduces candidate count to configured max
2. New focused test: non-Travel FF-helpful candidates always enter preferred queue
3. Existing suite: `cargo test -p worldwake-ai` (all golden + unit tests)
4. `survival-scattered` passes with `max_node_expansions: 640` when cap is enabled

### Invariants

1. When `max_travel_candidates_per_expansion` is `None`, behavior is identical to current (no cap)
2. When the cap is set, retained travel candidates are those with lowest perceived cost to goal-relevant places
3. Non-Travel candidates are never affected by the cap
4. The cap operates on the planning state (agent beliefs), not world state (FND-14 compliance)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests` — focused unit test for travel candidate capping
2. `crates/worldwake-ai/src/search/tests` — focused unit test for locality-first preferred queue assignment
3. `crates/worldwake-ai/tests/golden_survival_scattered.rs` — verify passes with reduced `max_node_expansions`

### Commands

1. `cargo test -p worldwake-ai --lib -- search::tests`
2. `cargo test -p worldwake-ai --test golden_survival_scattered`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-16.

- Added `CognitiveProfile.max_travel_candidates_per_expansion: Option<u16>` with default/serde coverage and propagated the new field through shared test/sample literals.
- Added search-loop travel capping after travel-away pruning and before per-expansion candidate-budget enforcement, using planning-snapshot perceived travel cost and falling back to direct perceived edge cost when no goal-relevant places are known.
- Tightened FF preferred-queue assignment so helpful non-travel successors always stay preferred, while helpful travel successors are preferred only when they are goal-directed under the planner's perceived travel-cost model.
- Updated `survival-scattered` to use `max_travel_candidates_per_expansion: 4` and restored `max_node_expansions: 640`; the scattered golden now passes at that budget.

## Deviations

- No `crates/worldwake-cli/src/scenario/mod.rs` change was required. The scenario path already carries `Option<CognitiveProfile>` directly, so the new field landed through the shared type and defaulted lawfully.
- The travel cap now has an explicit trace/filter reason (`TravelCandidateCap`) even though the original ticket only required search-loop behavior. This keeps root and expansion candidate inventories honest when the cap removes a branch.
- Reassessment against the live search loop and FOUNDATIONS principle 12 narrowed the locality-first portion of the ticket from a stronger local-vs-travel expansion partition to preferred-queue guidance only. The landed change biases selection order without suppressing lawful successor construction through a second optimization-only phase.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib search::tests::travel_candidate_cap_keeps_lowest_cost_local_travel_when_goal_places_unknown -- --exact`
- Passed `cargo test -p worldwake-ai --lib search::tests::ff_helpful_non_travel_candidates_always_stay_preferred -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_survival_scattered`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
