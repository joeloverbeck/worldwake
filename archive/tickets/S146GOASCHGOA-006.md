# S146GOASCHGOA-006: Per-goal budget application in search + `PlanAttemptTrace.goal_budget` provenance

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `search/mod.rs` reads per-goal budget from registry; `PlanAttemptTrace` gains provenance field (decision-trace layer)
**Deps**: archive/tickets/S146GOASCHGOA-002.md, archive/tickets/S146GOASCHGOA-004.md

## Problem

S146 PR-17's per-goal budgets only matter at the search-dispatch boundary, where the planner currently reads uniform `CognitiveProfile.max_plan_depth` and `max_node_expansions` for every goal. After ticket 004 populates `GoalSchema.planning_budget`, the search layer must compose that per-goal budget with the agent's cognitive ceiling and the planner-substrate (S145) `ExecutionBudget::strategic_budget_for_stages` to derive an `effective_budget`. The applied budget is recorded onto every `PlanAttemptTrace` so ticket 008's observer rendering and S144's `PlanningMetrics` can attribute exhaustion to budget tier.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `CognitiveProfile` defaults at `crates/worldwake-core/src/cognitive_profile.rs:129-171`: `max_plan_depth = 8` (`:134`), `max_node_expansions = 224` (`:137`). `ExecutionBudget::strategic_budget_for_stages(stage_count: usize) -> usize` exists on `crates/worldwake-core/src/execution_budget.rs` as a `pub const fn` returning `2 * stages * max_prerequisite_locations()`. `PlanAttemptTrace` is at `crates/worldwake-ai/src/decision_trace.rs:1157`, deriving only `Clone, Debug` (no `Serialize/Deserialize`) — adding a new field requires no save-format bump. Existing trace tests: `repair_attempt_trace_roundtrips_through_bincode:2715`, `format_goal_kind_emits_acquire_quantity_fields:2667`, plus ~13 more between lines 2667–3155.
2. Per `archive/specs/S146-goal-schema-and-per-goal-budgets.md` D7 + D8: the search reads `goal_schema.planning_budget` per candidate's `GoalDispatchKey`, composes via `min()` with cognitive ceiling and `strategic_budget_for_stages`. The trace field is `goal_budget: GoalPlanningBudget` populated from the computed `effective_budget` at every PlanAttemptTrace construction site. Per Q3=(a) resolution: cognitive defaults are unchanged — every preset above depth 8 silently clamps to 8 for default-cognitive-profile agents, and scenarios that need deeper search must author elevated `cognitive_profile.max_plan_depth` per agent (ticket 007's golden exercises this).
3. Shared abstraction boundary under audit: `effective_budget` is the search-layer read-model derived from (per-goal `planning_budget` from ticket 004, per-agent `CognitiveProfile`, per-agent `ExecutionBudget`). The `PlanAttemptTrace.goal_budget` field records the `effective_budget` actually applied — this is the debugging contract per FND-29. The trace field is provenance (what was used), not a duplicate of the schema's static budget.
4. Failing-golden / invariant restatement: D7 changes the budget input source; existing goldens that run under default cognitive profile see no behavioral change because every preset above depth 8 clamps to 8 (per Q3 resolution). The later archived ticket 007 replaced the drafted autonomous `golden_per_goal_budget.rs` with focused search-trace validation under an elevated cognitive profile to exercise the differentiation.
5. Live `GoalKind` surface under test: all GoalKind variants; each is mapped to a `GoalDispatchKey` via existing `from_goal_kind` (`crates/worldwake-core/src/goal_dispatch_key.rs`), which keys the populated `GoalSchema.planning_budget`. The current operator/affordance surface is unchanged — only the budget reading is rerouted.
6. AI-regression layer: this ticket modifies the plan-search phase (P12 phase distinction). Intended verification layer is runtime `agent_tick` decision-trace coverage. The `PlanAttemptTrace.goal_budget` field is the proof surface for "which budget was applied"; existing `PlanSearchOutcome` exhaustion variants remain the proof surface for "did the budget exhaust."
7. Ordering layer: this ticket may shift terminal ordering when effective budget differs from pre-S146 uniform value. Under default cognitive profile, every preset clamps to 8 → no terminal ordering shift. Under elevated cognitive profile, deeper budgets produce different terminal ordering — exercised by ticket 007's golden. The divergence depends on **delayed system resolution** (the per-goal budget feeds into the planner's expansion budget, which affects which plans complete first under exhaustion).
13. Adjacent contradictions:
   - `PlanAttemptTrace` constructors are scattered across `agent_tick/planning.rs` and `search/mod.rs`. Each construction site must populate `goal_budget` from the search-computed `effective_budget`. Classified as **required consequence** — adding a non-`Option` field forces this. To bridge: confirm the construction-site count during implementation (`rg '^\s*PlanAttemptTrace\s*\{$' crates/worldwake-ai/src/`) and ensure each is updated. If the count is high (>10), surface in the implementation pass.
   - The `max_strategic_expansions` clamp formula in D7 reads `goal_schema.planning_budget.max_strategic_expansions.min(execution_budget.strategic_budget_for_stages(stage_count) as u16)`. The `stage_count` is the number of prerequisite stages in the current candidate's strategic itinerary — derive from the candidate's prerequisite-stages structure during implementation. Classified as **required consequence** — the spec's formula assumes `stage_count` is reachable at the budget-composition site.

## Architecture Check

1. FND-3 (concrete state): per-goal budget is concrete typed data (`u8`, `u16`, `Permille`), not an abstract score. `effective_budget` is a derived read-model per FND-3.
2. FND-29 (debuggability): `PlanAttemptTrace.goal_budget` makes "which budget bounded this attempt" trivially answerable post-hoc. S144's `PlanningMetrics` can aggregate exhaustion-by-preset using this single field.
3. FND-12 (performance compresses computation, not causality): per-goal budget changes the planner's expansion budget for goals authored with deeper presets, but only when the agent's cognitive ceiling allows it. The compose-via-`min()` rule ensures world meaning never changes — only how deep the search explores.
4. `AGENTS.md` determinism: no float, no `HashMap`. All budget values are integer-typed.

## Verified Layers

1. `effective_budget` depth and node-expansion clamping against `CognitiveProfile` ceiling is covered by `search::tests::per_goal_budget_caps_below_cognitive_ceiling` and `search::tests::per_goal_budget_used_at_elevated_cognitive_ceiling`.
2. `effective_budget.max_strategic_expansions` composition against `ExecutionBudget::strategic_budget_for_stages` is covered by `search::tests::strategic_expansions_clamp_against_stage_count`.
3. `PlanAttemptTrace.goal_budget` provenance is covered by `agent_tick::planning::tests::plan_search_trace_converts_two_phase_trace_metadata`.
4. Shared `PlanAttemptTrace` constructor fallout is covered by `cargo test -p worldwake-ai`, `cargo test --workspace`, and CI-matching all-target clippy.
5. `PlanAttemptTrace` remains a non-save trace model; no save-format bump was required.

## Landed Changes

### 1. `effective_budget` computation in `search/mod.rs`

`search_plan_with_trace_metadata_and_source` now derives the schema preset from `GoalDispatchKey::from_goal_kind(...).declaration().planning_budget`, composes it with the agent cognitive ceiling, and applies the resulting `GoalPlanningBudget` to tactical search depth and node-expansion limits.

### 2. Strategic budget cap integration

`search/strategic.rs::plan_with_budget_trace` now accepts the schema-level `max_strategic_expansions` cap, records the stage count in `StrategicSearchResult`, and caps its stage-aware strategic loop with `min(schema cap, ExecutionBudget::strategic_budget_for_stages(stage_count))`.

### 3. `PlanAttemptTrace.goal_budget`

`PlanAttemptTrace` now carries `goal_budget: GoalPlanningBudget`. The production conversion path in `agent_tick/planning.rs::plan_search_result_to_trace` copies the applied search metadata into the trace, while manual trace fixtures in AI diagnostics, survival forensics, golden harnesses, and observer tests were updated for the new shared shape.

### 4. Focused tests

Added three `search::tests` budget-composition tests and extended the existing two-phase trace conversion test to assert that `goal_budget` is preserved.

## Landed Files

- `crates/worldwake-ai/src/search/mod.rs`
- `crates/worldwake-ai/src/search/strategic.rs`
- `crates/worldwake-ai/src/search/tests.rs`
- `crates/worldwake-ai/src/decision_trace.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`
- `crates/worldwake-ai/src/survival_forensics.rs`
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs`
- `crates/worldwake-cli/src/bin/observer.rs`

## Out of Scope

- Adjusting `CognitiveProfile` defaults — explicitly NOT changed per Q3=(a) resolution; existing-golden behavior preserved.
- Observer rendering of `goal_budget` — owned by ticket 008.
- Parity fixtures or new goldens — owned by ticket 007.
- Per-agent `budget_overrides` from `AgentSchemaContextProfile` — the schema-level `planning_budget` is read in this ticket; the per-agent override path is a sibling feature deliberately deferred (no S146 ticket implements override reads yet; sub-spec or future ticket can add). Document this absence in Out of Scope so reviewers know it's intentional.

## Acceptance Result

### Tests Passed

1. `per_goal_budget_caps_below_cognitive_ceiling()`
2. `per_goal_budget_used_at_elevated_cognitive_ceiling()`
3. `strategic_expansions_clamp_against_stage_count()`
4. `agent_tick::planning::tests::plan_search_trace_converts_two_phase_trace_metadata`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `effective_budget.max_depth <= min(cognitive.max_plan_depth, goal_schema.planning_budget.max_depth)` for every dispatched plan attempt.
2. `effective_budget.max_node_expansions <= min(cognitive.max_node_expansions, goal_schema.planning_budget.max_node_expansions)`.
3. `effective_budget.max_strategic_expansions <= min(goal_schema.planning_budget.max_strategic_expansions, execution_budget.strategic_budget_for_stages(stage_count) as u16)`.
4. `PlanAttemptTrace.goal_budget` records the actual `effective_budget` applied (not the preset, not the schema's static value).
5. No new save-format bump (PlanAttemptTrace is `Clone, Debug` only, not `Serialize/Deserialize`).
6. `AGENTS.md` determinism: no `HashMap` or floats introduced.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — added three budget-composition unit tests.
2. `crates/worldwake-ai/src/agent_tick/planning.rs` — extended two-phase trace metadata conversion coverage to assert `goal_budget`.

### Commands Run

1. `cargo test -p worldwake-ai per_goal_budget`
2. `cargo test -p worldwake-ai strategic_expansions_clamp_against_stage_count`
3. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::plan_search_trace_converts_two_phase_trace_metadata -- --exact`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-17.

- Landed schema-driven per-goal budget application at the search boundary.
- Recorded the applied effective budget on every `PlanAttemptTrace`.
- Extended strategic-search budget tracing so the schema cap and stage-aware execution cap compose in one place.
- Updated shared trace fixtures in diagnostics, observer, and golden helper code for the new trace shape.

## Deviations

- The implementation records `StrategicSearchResult.stages_count` from the live strategic planner instead of relying on a non-existent `candidate.prerequisite_stages` field from the ticket sketch.
- `SearchTraceMetadata::default()` uses `GoalPlanningBudget::TRAVEL_PURCHASE` for test-only/manual fixture construction; production search metadata overwrites it with the computed effective budget before trace conversion.

## Verification Result

- Passed `cargo test -p worldwake-ai per_goal_budget`
- Passed `cargo test -p worldwake-ai strategic_expansions_clamp_against_stage_count`
- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::plan_search_trace_converts_two_phase_trace_metadata -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
