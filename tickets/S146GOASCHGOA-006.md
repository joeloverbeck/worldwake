# S146GOASCHGOA-006: Per-goal budget application in search + `PlanAttemptTrace.goal_budget` provenance

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `search/mod.rs` reads per-goal budget from registry; `PlanAttemptTrace` gains provenance field (decision-trace layer)
**Deps**: 002, 004

## Problem

S146 PR-17's per-goal budgets only matter at the search-dispatch boundary, where the planner currently reads uniform `CognitiveProfile.max_plan_depth` and `max_node_expansions` for every goal. After ticket 004 populates `GoalSchema.planning_budget`, the search layer must compose that per-goal budget with the agent's cognitive ceiling and the planner-substrate (S145) `ExecutionBudget::strategic_budget_for_stages` to derive an `effective_budget`. The applied budget is recorded onto every `PlanAttemptTrace` so ticket 008's observer rendering and S144's `PlanningMetrics` can attribute exhaustion to budget tier.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `CognitiveProfile` defaults at `crates/worldwake-core/src/cognitive_profile.rs:129-171`: `max_plan_depth = 8` (`:134`), `max_node_expansions = 224` (`:137`). `ExecutionBudget::strategic_budget_for_stages(stage_count: usize) -> usize` exists on `crates/worldwake-core/src/execution_budget.rs` as a `pub const fn` returning `2 * stages * max_prerequisite_locations()`. `PlanAttemptTrace` is at `crates/worldwake-ai/src/decision_trace.rs:1157`, deriving only `Clone, Debug` (no `Serialize/Deserialize`) — adding a new field requires no save-format bump. Existing trace tests: `repair_attempt_trace_roundtrips_through_bincode:2715`, `format_goal_kind_emits_acquire_quantity_fields:2667`, plus ~13 more between lines 2667–3155.
2. Per `specs/S146-goal-schema-and-per-goal-budgets.md` D7 + D8: the search reads `goal_schema.planning_budget` per candidate's `GoalDispatchKey`, composes via `min()` with cognitive ceiling and `strategic_budget_for_stages`. The trace field is `goal_budget: GoalPlanningBudget` populated from the computed `effective_budget` at every PlanAttemptTrace construction site. Per Q3=(a) resolution: cognitive defaults are unchanged — every preset above depth 8 silently clamps to 8 for default-cognitive-profile agents, and scenarios that need deeper search must author elevated `cognitive_profile.max_plan_depth` per agent (ticket 007's golden exercises this).
3. Shared abstraction boundary under audit: `effective_budget` is the search-layer read-model derived from (per-goal `planning_budget` from ticket 004, per-agent `CognitiveProfile`, per-agent `ExecutionBudget`). The `PlanAttemptTrace.goal_budget` field records the `effective_budget` actually applied — this is the debugging contract per FND-29. The trace field is provenance (what was used), not a duplicate of the schema's static budget.
4. Failing-golden / invariant restatement: D7 changes the budget input source; existing goldens that run under default cognitive profile see no behavioral change because every preset above depth 8 clamps to 8 (per Q3 resolution). Ticket 007's `golden_per_goal_budget.rs` uses elevated cognitive profile to exercise the differentiation.
5. Live `GoalKind` surface under test: all GoalKind variants; each is mapped to a `GoalDispatchKey` via existing `from_goal_kind` (`crates/worldwake-ai/src/goal_dispatch_key.rs`), which keys the populated `GoalSchema.planning_budget`. The current operator/affordance surface is unchanged — only the budget reading is rerouted.
6. AI-regression layer: this ticket modifies the plan-search phase (P12 phase distinction). Intended verification layer is runtime `agent_tick` decision-trace coverage. The `PlanAttemptTrace.goal_budget` field is the proof surface for "which budget was applied"; existing `PlanSearchOutcome` exhaustion variants remain the proof surface for "did the budget exhaust."
7. Ordering layer: this ticket may shift terminal ordering when effective budget differs from pre-S146 uniform value. Under default cognitive profile, every preset clamps to 8 → no terminal ordering shift. Under elevated cognitive profile, deeper budgets produce different terminal ordering — exercised by ticket 007's golden. The divergence depends on **delayed system resolution** (the per-goal budget feeds into the planner's expansion budget, which affects which plans complete first under exhaustion).
13. Adjacent contradictions:
   - `PlanAttemptTrace` constructors are scattered across `agent_tick/planning.rs` and `search/mod.rs`. Each construction site must populate `goal_budget` from the search-computed `effective_budget`. Classified as **required consequence** — adding a non-`Option` field forces this. To bridge: confirm the construction-site count during implementation (`rg '^\s*PlanAttemptTrace\s*\{$' crates/worldwake-ai/src/`) and ensure each is updated. If the count is high (>10), surface in the implementation pass.
   - The `max_strategic_expansions` clamp formula in D7 reads `goal_schema.planning_budget.max_strategic_expansions.min(execution_budget.strategic_budget_for_stages(stage_count) as u16)`. The `stage_count` is the number of prerequisite stages in the current candidate's strategic itinerary — derive from the candidate's prerequisite-stages structure during implementation. Classified as **required consequence** — the spec's formula assumes `stage_count` is reachable at the budget-composition site.

## Architecture Check

1. FND-3 (concrete state): per-goal budget is concrete typed data (`u8`, `u16`, `Permille`), not an abstract score. `effective_budget` is a derived read-model per FND-3.
2. FND-29 (debuggability): `PlanAttemptTrace.goal_budget` makes "which budget bounded this attempt" trivially answerable post-hoc. S144's `PlanningMetrics` can aggregate exhaustion-by-preset using this single field.
3. FND-12 (performance compresses computation, not causality): per-goal budget changes the planner's expansion budget for goals authored with deeper presets, but only when the agent's cognitive ceiling allows it. The compose-via-`min()` rule ensures world meaning never changes — only how deep the search explores.
4. CLAUDE.md determinism: no float, no `HashMap`. All budget values are integer-typed.

## Verification Layers

1. `effective_budget` correctly clamps depth/expansions against `CognitiveProfile` ceiling → focused unit test in `search/mod.rs::#[cfg(test)]` covering each preset under default-8 ceiling and under elevated-24 ceiling
2. `effective_budget.max_strategic_expansions` correctly composes via `strategic_budget_for_stages` → focused unit test asserting the composition formula
3. `PlanAttemptTrace.goal_budget` records the applied budget at every construction site → decision-trace assertion in existing `agent_tick/planning.rs` tests (extend `consume_goal`-pattern tests starting line 2668)
4. Existing trace tests (`repair_attempt_trace_roundtrips_through_bincode:2715`, etc.) continue to pass with new field — `Clone, Debug` derive already covers the field; no save-format bump needed because trace is not serialized
5. AI-regression layer: runtime `agent_tick` decision-trace coverage (extended planning.rs inline tests). Local needs-only harness is sufficient for the unit-level budget composition; full action registries needed for the trace-population assertions.

## What to Change

### 1. `effective_budget` computation in `search/mod.rs`

In the dispatch boundary that currently reads `cognitive.max_plan_depth` and `cognitive.max_node_expansions`:

```rust
let goal_dispatch_key = GoalDispatchKey::from_goal_kind(&candidate.goal_kind);
let goal_schema = registry.get(&goal_dispatch_key)
    .expect("registry covers every GoalDispatchKey variant");
let stage_count = candidate.prerequisite_stages.len();
let effective_budget = GoalPlanningBudget {
    max_depth: goal_schema.planning_budget.max_depth.min(cognitive.max_plan_depth),
    max_node_expansions: goal_schema.planning_budget.max_node_expansions
        .min(cognitive.max_node_expansions),
    repair_budget_fraction: goal_schema.planning_budget.repair_budget_fraction,
    max_strategic_expansions: goal_schema.planning_budget.max_strategic_expansions
        .min(execution_budget.strategic_budget_for_stages(stage_count) as u16),
};
```

Subsequent search dispatch reads `effective_budget.max_depth` and `effective_budget.max_node_expansions` instead of `cognitive.*` directly.

Likely site: search/mod.rs near current `cognitive.max_plan_depth`/`max_node_expansions` reads (named in Step 2 spot-checks but exact lines vary; confirm during implementation via `grep -n "max_plan_depth\|max_node_expansions" crates/worldwake-ai/src/search/mod.rs`).

### 2. Add `goal_budget` field to `PlanAttemptTrace`

In `crates/worldwake-ai/src/decision_trace.rs:1157`:

```rust
#[derive(Clone, Debug)]
pub struct PlanAttemptTrace {
    pub goal: GoalKey,
    pub opportunity_anchor: OpportunityAnchor,
    pub outcome: PlanSearchOutcome,
    pub strategic_budget: Option<StrategicBudgetTrace>,
    // ... existing remaining fields ...
    /// Per-goal planning budget applied during this attempt — composed from
    /// the goal's GoalSchema preset, the agent's CognitiveProfile ceiling,
    /// and ExecutionBudget::strategic_budget_for_stages.
    pub goal_budget: GoalPlanningBudget,
}
```

### 3. Populate `goal_budget` at every PlanAttemptTrace construction site

During implementation, run `rg 'PlanAttemptTrace\s*\{' crates/worldwake-ai/src/` to enumerate construction sites. Each site receives `goal_budget: effective_budget` (or the equivalent derived value at that call boundary). Sites that don't already have access to `effective_budget` are extended to receive it via parameter.

### 4. Focused unit tests

In `crates/worldwake-ai/src/search/mod.rs::#[cfg(test)]` (or a new `crates/worldwake-ai/src/search/budget_composition.rs` if the search module is large):

```rust
#[test]
fn per_goal_budget_caps_below_cognitive_ceiling() {
    let cognitive = CognitiveProfile { max_plan_depth: 8, max_node_expansions: 224, .. };
    let presets = [
        (GoalPlanningBudget::SELF_CARE, 6, 96),
        (GoalPlanningBudget::TRAVEL_PURCHASE, 8, 224),  // depth 10 clamps to 8
        (GoalPlanningBudget::PRODUCTION, 8, 224),       // depth 16 clamps to 8
        (GoalPlanningBudget::INVESTIGATION, 8, 224),
        (GoalPlanningBudget::BOUNTY_ESCORT, 8, 224),
    ];
    // assert each composes to (expected_depth, expected_expansions)
}

#[test]
fn per_goal_budget_used_at_elevated_cognitive_ceiling() {
    let cognitive = CognitiveProfile { max_plan_depth: 24, max_node_expansions: 768, .. };
    // assert each preset composes to its preset values
}

#[test]
fn strategic_expansions_clamp_against_stage_count() {
    // assert min(preset.max_strategic_expansions, exec_budget.strategic_budget_for_stages(N))
}
```

## Files to Touch

- `crates/worldwake-ai/src/search/mod.rs` (modify — replace direct cognitive reads with `effective_budget` computation; thread the budget into search dispatch)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — add `goal_budget` field to `PlanAttemptTrace` at `:1157`)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — every `PlanAttemptTrace` constructor populates `goal_budget` from the search-computed `effective_budget`; inline tests starting line 2602+ extended where they assert on attempt traces)
- Likely: any other site that constructs `PlanAttemptTrace` — discover via `rg 'PlanAttemptTrace\s*\{' crates/worldwake-ai/src/`

## Out of Scope

- Adjusting `CognitiveProfile` defaults — explicitly NOT changed per Q3=(a) resolution; existing-golden behavior preserved.
- Observer rendering of `goal_budget` — owned by ticket 008.
- Parity fixtures or new goldens — owned by ticket 007.
- Per-agent `budget_overrides` from `AgentSchemaContextProfile` — the schema-level `planning_budget` is read in this ticket; the per-agent override path is a sibling feature deliberately deferred (no S146 ticket implements override reads yet; sub-spec or future ticket can add). Document this absence in Out of Scope so reviewers know it's intentional.

## Acceptance Criteria

### Tests That Must Pass

1. `per_goal_budget_caps_below_cognitive_ceiling()` — new unit test
2. `per_goal_budget_used_at_elevated_cognitive_ceiling()` — new unit test
3. `strategic_expansions_clamp_against_stage_count()` — new unit test
4. Existing trace tests (`repair_attempt_trace_roundtrips_through_bincode:2715`, etc.) pass with the new field populated
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. Clippy clean: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `effective_budget.max_depth <= min(cognitive.max_plan_depth, goal_schema.planning_budget.max_depth)` for every dispatched plan attempt.
2. `effective_budget.max_node_expansions <= min(cognitive.max_node_expansions, goal_schema.planning_budget.max_node_expansions)`.
3. `effective_budget.max_strategic_expansions <= min(goal_schema.planning_budget.max_strategic_expansions, execution_budget.strategic_budget_for_stages(stage_count) as u16)`.
4. `PlanAttemptTrace.goal_budget` records the actual `effective_budget` applied (not the preset, not the schema's static value).
5. No new save-format bump (PlanAttemptTrace is `Clone, Debug` only, not `Serialize/Deserialize`).
6. CLAUDE.md determinism: no `HashMap` or floats introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/mod.rs` `#[cfg(test)]` (or `search/budget_composition.rs` if new) — 3 unit tests per "Focused unit tests" above
2. `crates/worldwake-ai/src/agent_tick/planning.rs` `#[cfg(test)]` — extend existing trace-assertion tests to cover `goal_budget` population

### Commands

1. `cargo test -p worldwake-ai per_goal_budget`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `scripts/verify.sh`
