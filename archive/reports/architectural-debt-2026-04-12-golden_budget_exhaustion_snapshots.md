# Architectural Debt Analysis: golden_budget_exhaustion_snapshots

**Status**: COMPLETED

**Date**: 2026-04-12
**Input**: `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs`
**Source modules analyzed**: ~166 (48 ai + 70 core + 48 sim)
**Crates touched**: worldwake-ai, worldwake-core, worldwake-sim
**Prior reports consulted**: none

## Executive Summary

Analysis of the budget exhaustion golden test suite found **one Medium-severity finding** and **one Low-severity finding**, both from cross-lens reinforcement. The dominant pattern is a split in exhaustion-related authority: search termination semantics are defined in `search/mod.rs`, retry/invalidation state lives in `decision_runtime.rs`, and invalidation logic lives in `exhaustion.rs` — three modules each owning a piece of "what happens when search fails." A secondary finding concerns the dual parameter structures (`ExecutionBudget` + `CognitiveProfile`) that jointly control search termination without a unifying abstraction. However, both findings carry substantial counter-evidence suggesting the current architecture may be intentionally factored. Most analyzed clusters showed acceptable architecture — the search pipeline, belief view construction, and planning snapshot are well-separated along responsibility lines.

## Scenario Families

| Family | Tests | Domain Concepts | Key Assertions |
|--------|-------|----------------|----------------|
| Plan search success | 3 (`merchant_vara_water`, `guard_theron_water`, `kael_water_late_game`) | harvest plan, recipe knowledge, belief-based planning | `PlanSearchResult::Found(_)` — planner finds valid plans |
| Budget exhaustion regression | 2 (`merchant_vara_apple`, `merchant_vara_treat_wounds`) | max_node_expansions, candidate explosion, commodity noise | Exact `BudgetExhausted(300)` expansion count |
| Frontier exhaustion regression | 1 (`kael_treat_wounds_vara`) | frontier depletion, limited search space | Exact `FrontierExhausted(54)` expansion count |
| Candidate inventory report | 1 (ignored: `generate_residual_candidate_report`) | per-expansion trace, root candidate filtering | Report generation, no assertions |

## Traceability Summary

| Module | Scenario Families | Confidence | Strategy |
|--------|------------------|------------|----------|
| `worldwake-ai/src/search/mod.rs` | All families | High | `search_plan` is the direct target; defines `PlanSearchResult` |
| `worldwake-ai/src/search/candidates.rs` | All families | High | Candidate expansion exercised by every search |
| `worldwake-ai/src/search/frontier.rs` | Budget/frontier exhaustion | High | Frontier management; `DualFrontier` uses `preferred_operator_boost` |
| `worldwake-ai/src/candidate_generation.rs` | Plan search success, candidate inventory | High | `generate_candidates` called in test helpers |
| `worldwake-ai/src/planning_snapshot.rs` | All families | High | `build_planning_snapshot` called per scenario |
| `worldwake-ai/src/decision_trace.rs` | Candidate inventory, exhaustion regression | High | `SearchExpansionSummary`, `RootCandidateOutcome` types |
| `worldwake-ai/src/exhaustion.rs` | Budget/frontier exhaustion | Medium | Invalidation conditions exercised indirectly via retry state |
| `worldwake-ai/src/decision_runtime.rs` | Budget/frontier exhaustion | Medium | `ExhaustionRetryState`, `ExhaustionEntry` used downstream |
| `worldwake-core/src/execution_budget.rs` | All families | High | `ExecutionBudget` parameterizes every search |
| `worldwake-core/src/cognitive_profile.rs` | All families | High | `CognitiveProfile` parameterizes every search |
| `worldwake-sim/src/per_agent_belief_view.rs` | All families | High | `PerAgentBeliefView` constructed per scenario |

## Findings

### F1: Exhaustion Authority Split Across Three Modules

**Lens Source**: Merged (Lens A structural scatter + Lens B split protocol)
**Fracture Type**: Split protocol + Authority leak
**Severity**: Medium
**Confidence**: Medium
**Scope**: `worldwake-ai/src/search/mod.rs`, `worldwake-ai/src/decision_runtime.rs`, `worldwake-ai/src/exhaustion.rs`

**Owned truth**: The "what to do when plan search fails to find a solution" lifecycle — from search termination through retry eligibility to invalidation conditions.

**Invariants**:
- A `BudgetExhausted` result must always carry the exact expansion count used
- An exhaustion entry must always have invalidation conditions derived from the goal kind
- Retry eligibility must correctly distinguish budget (retryable with reduced budget) from frontier (not retryable without state change)

**Owner boundary**: Currently split across three modules in `worldwake-ai`

**Evidence**:
- `search/mod.rs:195-206` — defines `PlanSearchResult` enum with `BudgetExhausted`/`FrontierExhausted` variants
- `search/mod.rs:344-350` — primary budget check returns `BudgetExhausted`
- `search/mod.rs:456-463` — secondary safety valve also returns `BudgetExhausted` (candidates overflow)
- `search/mod.rs:307` — returns `FrontierExhausted` for early tactical failure
- `search/mod.rs:722-732` — returns `FrontierExhausted` at end of main loop
- `decision_runtime.rs:60-63` — defines `ExhaustionRetryState` (FrontierExhausted, BudgetRetryPending)
- `decision_runtime.rs:66-68` — defines `ExhaustionEntry` (retry_state + invalidation_conditions)
- `exhaustion.rs:60-120` — `derive_invalidation_conditions` dispatches on GoalKind
- `agent_tick/planning.rs:609-640` — 13+ match arms interpreting exhaustion results with local logic

**Modules affected**: search/mod.rs, decision_runtime.rs, exhaustion.rs, agent_tick/planning.rs
**Scenario families explained**: Budget exhaustion regression, Frontier exhaustion regression
**Expected simplification**: If exhaustion lifecycle were consolidated, the 13 match arms in agent_tick/planning.rs could be replaced with a single dispatch to the exhaustion authority. Retry state construction and invalidation condition derivation would happen in one place.

**FOUNDATIONS alignment**:
- P20 (Resource-bounded reasoning): Aligned — the current design correctly implements bounded search with different termination modes
- P26 (Systems interact through state): Strained — the three modules interact through shared enums rather than through a clear state boundary; `agent_tick/planning.rs` must understand internal search semantics to handle results
- P27 (Derived summaries are caches): Aligned — `ExhaustionEntry` is properly derived from search results, not elevated to truth
- P28 (No backward compatibility): Aligned — no compatibility layers detected

**Counter-evidence**: The split may be intentional architectural layering:
1. `search/mod.rs` owns "how to search" (pure search algorithm)
2. `decision_runtime.rs` owns "what to remember" (agent decision state)
3. `exhaustion.rs` owns "when to retry" (invalidation policy)
This is a clean separation-of-concerns pattern. The 13 match arms in planning.rs may be unavoidable complexity — each handles a distinct combination of search result + frame state + retry policy. The test file itself only checks the search-level results (`PlanSearchResult`), not the higher-level retry machinery, suggesting the tests validate the correct layer.

---

### F2: Dual Budget Parameter Structures Without Unifying Abstraction

**Lens Source**: Lens A (structural scatter)
**Severity**: Low
**Confidence**: Low
**Scope**: `worldwake-core/src/execution_budget.rs`, `worldwake-core/src/cognitive_profile.rs`, `worldwake-ai/src/search/mod.rs`

**Owned truth**: The combined "search termination policy" — when to stop expanding, how wide to search, how deep to go.

**Invariants**:
- `beam_width` (ExecutionBudget) shapes frontier width
- `max_node_expansions` (CognitiveProfile) caps total search effort
- `max_plan_depth` (CognitiveProfile) caps depth
- `max_candidates_per_expansion` (CognitiveProfile) caps breadth per node
- `preferred_operator_boost` (ExecutionBudget) affects frontier ordering

**Owner boundary**: Both structs live in worldwake-core, consumed by worldwake-ai

**Evidence**:
- `search/mod.rs:230-243` — `search_plan` accepts both `cognitive: &CognitiveProfile` and `execution_budget: &ExecutionBudget` as separate parameters
- `search/mod.rs:319` — uses `cognitive.max_node_expansions` for budget
- `search/mod.rs:642` — uses `execution_budget.beam_width` for truncation
- `search/frontier.rs:51,79` — uses `execution_budget.preferred_operator_boost`
- `search/strategic.rs:92,112` — uses `execution_budget.max_prerequisite_locations`

**Modules affected**: search/mod.rs, search/frontier.rs, search/strategic.rs
**Scenario families explained**: All families (every test configures both parameters)
**Expected simplification**: A unified "search policy" parameter could reduce the surface area. Sub-modules would receive only the slice of policy they need rather than reaching into two separate structs.

**FOUNDATIONS alignment**:
- P20 (Resource-bounded reasoning): Aligned — both structures model genuine cognitive and execution constraints
- P22 (Agent diversity through concrete variation): Aligned — separating cognitive from execution parameters enables different agent archetypes (a smart agent with limited execution budget vs. a simple agent with generous execution budget)
- P3 (Concrete state over abstract scores): Aligned — both structs are concrete per-agent parameters, not abstract scores

**Counter-evidence**: The split between "cognitive" (how the agent thinks) and "execution" (how much compute the agent gets) is semantically meaningful. CognitiveProfile models the agent's reasoning capacity (per P20, P22), while ExecutionBudget models compute compression (per P12). Merging them would conflate two distinct concerns. The test file explicitly configures them separately (`merchant_vara_cognitive_profile()` vs `merchant_vara_execution_budget()`), which suggests the domain distinction is load-bearing. Additionally, the file-count threshold (5+ files) is barely met — only 4 files access ExecutionBudget fields.

---

## Acceptable Architecture

**Search pipeline structure**: The `search/` module is well-factored with clear sub-responsibilities: `mod.rs` owns the main loop, `candidates.rs` handles expansion, `frontier.rs` manages the dual frontier, `heuristic.rs` computes costs, `landmarks.rs` extracts landmarks, `strategic.rs` handles strategic pre-planning, and `transition.rs` manages state transitions. Each file has a single clear responsibility. This is complex but correctly architected.

**Planning snapshot**: `build_planning_snapshot` in `planning_snapshot.rs` is a pure function that builds a belief-projected world view. While called from multiple sites (14+), this is appropriate — it's a data-projection utility analogous to a database view. Each call site needs a fresh snapshot for its specific context (different agent, different tick, different evidence set). A factory or cache would be incorrect here because snapshots are inherently contextual.

**Belief view construction**: `PerAgentBeliefView::from_world_at_tick_with_recipes` is called from ~10 production sites. This scatter is acceptable because each site operates in a different execution context (observation phase, candidate evaluation, active action monitoring, frame setup, frontier initialization). The belief view is lightweight (references, no copies) and fundamentally per-context. Caching would violate P14 (world state is not belief state) since each construction captures a specific temporal context.

**Candidate generation**: `generate_candidates` has only 2 production call sites (observation.rs, goal_explanation.rs), showing clean centralization. The 70+ test call sites demonstrate thorough coverage, not scatter.

**Decision trace types**: All trace-related types (`SearchExpansionSummary`, `RootCandidateOutcome`, `ExpansionCandidateOutcome`, etc.) are correctly centralized in `decision_trace.rs`. They serve as a unified vocabulary for observability, aligned with P29 (debuggability is a product feature).

**Temporal coupling**: The git history shows minimal cross-crate coupling (only 6 commits touching both ai and core in 6 months, concentrated around `world/ownership.rs`). This suggests crate boundaries are healthy.

## Needs Investigation

| Signal | Type Suspected | One Signal Found | Second Signal to Look For |
|--------|---------------|-----------------|--------------------------|
| `agent_tick/planning.rs` size | Overloaded abstraction | 13+ exhaustion match arms, 5 belief view constructions, 2 cognitive profile constructions | Check total line count and whether it handles multiple lifecycle roles that should be separated (planning vs. frame management vs. exhaustion handling) |
| Strategic planner independent budget | Split protocol | `search/strategic.rs:128-130` uses local `search_budget` (hardcoded 50), not tied to `CognitiveProfile.max_node_expansions` | Check whether strategic pre-planning is intentionally independent or whether it should respect the same budget authority |
| ExecutionBudget lacks constructor validation | Boundary inversion | Core exports unchecked struct with `pub` fields; all enforcement in AI module | Check whether `beam_width=0` or `max_prerequisite_locations=0` can cause panics or silent failures in search/ |

## Proposals

No Critical or High severity findings — no proposals generated.

## Codebase Health Observations

- **Clean crate boundaries**: Only 6 commits in 6 months touched both worldwake-ai and worldwake-core, and those were concentrated around a single module (ownership.rs). No worldwake-sim<->worldwake-ai coupling detected. This is excellent for a multi-crate workspace.

- **Strong test isolation**: The golden test harness (`golden_harness/mod.rs`) provides a comprehensive, well-factored test infrastructure with ~1100+ lines of helpers. It correctly separates world setup, agent configuration, belief seeding, and assertion patterns. Each golden test creates a self-contained scenario snapshot rather than depending on shared mutable state.

- **Effective centralization of trace vocabulary**: All decision trace types live in a single module (`decision_trace.rs`), providing a uniform observability surface. The test file's `append_scenario_report` function demonstrates that the trace API is expressive enough to produce detailed human-readable diagnostics.

- **Deterministic exhaustion contracts**: The test suite pins exact expansion counts (`BudgetExhausted(300)`, `FrontierExhausted(54)`), which serves as a regression guard against search behavior changes. This is a particularly strong testing pattern for GOAP planner correctness.

## Outcome

Completed on 2026-04-12.

- The report's actionable `ExecutionBudget` boundary concern was implemented by `EXECBUDVAL-001`, which added validated construction, private fields, accessor-based consumers, and deserialize-time invariant enforcement.
- No separate follow-up ticket was needed from this report after that implementation landed.
- Verification results for the exploited concern were recorded on the completed ticket and included focused core tests, focused AI tests, workspace tests, and CI-matching clippy.
