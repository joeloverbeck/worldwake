# S146: Data-Driven Goal Schema and Per-Goal Planning Budgets

**Status**: Draft

## Summary

Folds in PR-2 (Data-Driven GoalSchema Registry) and PR-17 (Per-Goal Planning Budgets) from `reports/ai-architecture-improvements.md`.

The current candidate-emission path in `crates/worldwake-ai/src/candidate_generation.rs` defines 18+ hand-coded `emit_*` functions, one per goal family (`emit_need_candidates`, `emit_production_candidates`, `emit_enterprise_candidates`, `emit_disposal_candidates`, `emit_bounty_candidates`, `emit_artifact_posting_candidates`, `emit_combat_candidates`, `emit_crime_candidates`, `emit_social_candidates`, `emit_ask_witness_candidates`, `emit_patrol_candidates`, `emit_political_candidates`, `emit_recorded_violation_candidates`, `emit_search_candidates`, `emit_report_found_candidates`, `emit_escort_candidates`, `emit_exploration_candidates`, `emit_proactive_exploration_candidates`, `emit_expectation_violation_candidates`, `emit_opportunity_compiler_candidates`). Each function is independently maintained and called explicitly from `agent_tick/planning.rs`. As the project grows toward "dozens or hundreds of goals" (the user's stated goal), this hand-shape becomes brittle: each new goal family adds another emitter function, another call site, another set of suppression rules, and another lint surface.

`GoalDispatchDeclaration` (`crates/worldwake-ai/src/goal_dispatch_decl.rs:61`) already functions as a partial goal-kind registry — it carries `trace_label`, `relevant_ops`, `invalidation_strategy`, `feasibility_strategy`, `frontier_exhaustion_strategy`, `family_policy`, and `progress_barrier_ops` per `GoalKindDiscriminant`. S146 extends this declaration into the full `GoalSchema` envisioned by the assessment: candidate extractors, satisfaction predicates, ranking-feature hooks, expectation templates, information-gap templates, and **per-goal planning budgets**. The 18+ `emit_*` functions are migrated into a `CandidateExtractor` registry indexed by their declaring `GoalKindDiscriminant`. A new universal `AgentSchemaContextProfile` carries per-agent extractor enable/disable settings (so scenarios can opt into or out of candidate families without code changes).

PR-17's per-goal budget is folded in as `GoalSchema.planning_budget: GoalPlanningBudget`, replacing the current per-agent (`CognitiveProfile.max_plan_depth`, `max_node_expansions`) uniform values for paths where the goal's natural depth differs from the agent's default. The agent's `CognitiveProfile` remains the *ceiling*; per-goal budget overrides choose where within the ceiling the search lives. A self-care `Eat` goal gets a depth-6 plan budget; a `ProduceCommodity` for bread gets a depth-16 budget; an `InvestigateViolation` gets depth-20. None exceed `CognitiveProfile.max_plan_depth`.

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-ai` — owns `GoalSchema`, `CandidateExtractor`, `GoalPlanningBudget`, and the migrated extractor registry. Refactors `candidate_generation.rs` so the 18+ emit functions live as `CandidateExtractor` impls registered against `GoalSchema.candidate_extractors`. Extends `goal_dispatch_decl.rs` to declare the new schema fields.
- `worldwake-core` — adds `goal_schema` module exposing `GoalKindDiscriminant`-keyed schema entries. Adds `GoalPlanningBudget` to the per-goal declaration. No change to authoritative world state.
- `worldwake-sim` — no change.
- `worldwake-systems` — no change.
- `worldwake-cli` — observer Section 7 (planning) renders the per-goal budget that bounded each plan attempt. `AgentSchemaContextProfile` registered through the scenario loader.

## Dependencies

- S138 (Opportunity Compiler, archived) — provides `EffectSchemaIndex` and `Opportunity` substrate; S146's `CandidateExtractor::Opportunity` variant delegates to it.
- S141 (Motive Source Ledger, archived) — provides `MotiveSourceRef`; `GoalSchema.motive_source_hints` declares which motive sources naturally produce each goal kind.
- S134 (Canonical Effect Schema, archived) — provides `EffectSchema` per `ActionDef`; `GoalSchema.satisfaction_predicate` declares which effect facts satisfy each goal.
- S145 (Planning Substrate Hardening) — provides `strategic_budget_for_stages`; per-goal budget composes with it.
- S109 (Typed Discrepancy Taxonomy, archived) — `GoalSchema.invalidator_templates` produces typed `Discrepancy` instances on assumption failure.

## Design Goals

1. **One goal-kind registry, declaratively populated.** Adding a new goal kind means adding a `GoalSchema` entry, not editing 4 different files (`candidate_generation.rs`, `goal_dispatch_decl.rs`, `motive_source_mapping.rs`, `feasibility.rs`).
2. **Per-goal planning budgets are first-class.** `Eat`'s budget differs from `BakeBread`'s budget without per-agent profile tuning.
3. **No silent goal-family loss.** A registry table is enumerable and lintable: a missing field on a `GoalSchema` entry is a compile error.
4. **Backward-compat-free.** The 18 `emit_*` functions are migrated, not aliased. Old call sites are deleted.
5. **Deterministic.** Registry iteration is `BTreeMap`-ordered; extractors run in fixed order; no `HashMap` in authoritative state.
6. **Schema is data, not behavior.** Extractors are function pointers / trait impls; the schema's role is declarative metadata.

## Non-Goals

- **No new authoritative world state.** `GoalSchema` is registry data, not per-agent state.
- **No HTN method decomposition.** That is S147's scope; `GoalSchema.methods: Vec<MethodSchemaId>` is reserved as an empty `Vec` until S147 populates it.
- **No live registry mutation.** The registry is a `const`-style table built at workspace init.
- **No LLM-driven schema generation.** Schemas are author-written Rust, per the assessment's explicit "do not let live LLMs invent plans" guardrail.
- **No new event tag.** Schema registry is build-time data.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `GoalSchema` declares concrete extractor / predicate / budget data; no abstract goal-family score. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | Per-goal planning budgets express how the agent should reason about each goal type, not what outcome they should achieve. |
| FND-22 (Agent Diversity Through Concrete Variation) | `AgentSchemaContextProfile` lets per-agent profiles enable/disable extractor families, supporting the assessment's later archetype work (S152). |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Extractors read belief views and snapshot state; the schema registry is shared data, not cross-system commands. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The 18 `emit_*` functions are migrated and deleted, not preserved as shims. |
| FND-29 (Debuggability Is a Product Feature) | `GoalSchema.explanation_template` carries the structured prose that observer Section 7 uses to render per-goal reasoning; "why was this goal not emitted?" answers from `SuppressionReason` (per S144's D4). |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | The registry is the surface where FND-30 lives at the code level: every `GoalSchema` entry declares its causal hooks, lifecycle, contention, failures, learning updates, and validation criteria. |

## Deliverables

### D1: `GoalSchema` type

```rust
// crates/worldwake-core/src/goal_schema.rs (new)
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoalSchema {
    pub kind: GoalKindDiscriminant,
    pub candidate_extractors: Vec<CandidateExtractorId>,
    pub satisfaction_predicate: SatisfactionPredicateId,
    pub relevant_op_families: BTreeSet<PlannerOpKind>,
    pub methods: Vec<MethodSchemaId>,             // empty until S147
    pub invalidator_templates: Vec<InvalidatorTemplateId>,
    pub expectation_templates: Vec<ExpectationTemplateId>,
    pub information_gap_templates: Vec<InformationGapTemplateId>,
    pub ranking_features: Vec<RankingFeatureId>,
    pub motive_source_hints: Vec<MotiveSourceVariantId>,
    pub explanation_template: ExplanationTemplateId,
    pub planning_budget: GoalPlanningBudget,
}
```

All `*Id` types are typed newtypes (`pub struct CandidateExtractorId(pub u16)`) registered in dispatch tables in `worldwake-ai`. The schema is build-time data; the IDs resolve to concrete `fn`s at the dispatch layer.

### D2: `GoalPlanningBudget`

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GoalPlanningBudget {
    pub max_depth: u8,
    pub max_node_expansions: u16,
    pub repair_budget_fraction: Permille,
    pub max_strategic_expansions: u16,
}

impl GoalPlanningBudget {
    pub const SELF_CARE: Self = Self {
        max_depth: 6,
        max_node_expansions: 96,
        repair_budget_fraction: Permille::new(250),
        max_strategic_expansions: 12,
    };
    pub const TRAVEL_PURCHASE: Self = Self {
        max_depth: 10,
        max_node_expansions: 224,
        repair_budget_fraction: Permille::new(300),
        max_strategic_expansions: 24,
    };
    pub const PRODUCTION: Self = Self {
        max_depth: 16,
        max_node_expansions: 384,
        repair_budget_fraction: Permille::new(400),
        max_strategic_expansions: 48,
    };
    pub const INVESTIGATION: Self = Self {
        max_depth: 20,
        max_node_expansions: 512,
        repair_budget_fraction: Permille::new(450),
        max_strategic_expansions: 64,
    };
    pub const BOUNTY_ESCORT: Self = Self {
        max_depth: 24,
        max_node_expansions: 768,
        repair_budget_fraction: Permille::new(500),
        max_strategic_expansions: 96,
    };
}
```

The agent's `CognitiveProfile.max_plan_depth` becomes a *ceiling*. The search uses `min(cognitive_max, goal_budget.max_depth)`. The default `CognitiveProfile.max_plan_depth = 24` accommodates the deepest preset.

### D3: `CandidateExtractor` trait + registry

```rust
// crates/worldwake-ai/src/goal_schema_registry/extractors.rs (new)
pub trait CandidateExtractor {
    fn extract(&self, ctx: &ExtractorContext<'_>) -> Vec<CandidateOffer>;
    fn id(&self) -> CandidateExtractorId;
    fn is_enabled_for(&self, profile: &AgentSchemaContextProfile) -> bool;
}

pub struct ExtractorContext<'a> {
    pub actor: EntityId,
    pub belief_view: &'a dyn RuntimeBeliefView,
    pub snapshot: &'a PlanningSnapshot,
    pub motives: &'a [MotiveSourceRef],
    pub diagnostics: &'a mut SuppressionLog,
}
```

The current 18 `emit_*` functions become `impl CandidateExtractor` per goal family: `NeedExtractor`, `ProductionExtractor`, `EnterpriseExtractor`, etc. Existing function bodies move into the impl with minimal change; signature converges through `ExtractorContext`.

### D4: Registry build-time table

```rust
// crates/worldwake-ai/src/goal_schema_registry/registry.rs
pub fn build_goal_schema_registry() -> BTreeMap<GoalKindDiscriminant, GoalSchema> {
    let mut registry = BTreeMap::new();
    registry.insert(GoalKindDiscriminant::Eat, GoalSchema {
        kind: GoalKindDiscriminant::Eat,
        candidate_extractors: vec![CandidateExtractorId::Need],
        satisfaction_predicate: SatisfactionPredicateId::HungerSatisfied,
        relevant_op_families: btreeset![PlannerOpKind::Eat],
        methods: vec![],
        invalidator_templates: vec![InvalidatorTemplateId::CommodityAvailableAt],
        expectation_templates: vec![ExpectationTemplateId::ConsumeReducesNeed],
        information_gap_templates: vec![],
        ranking_features: vec![RankingFeatureId::NeedUrgency, RankingFeatureId::TravelCost],
        motive_source_hints: vec![MotiveSourceVariantId::NeedPressureHunger],
        explanation_template: ExplanationTemplateId::SatisfyNeed,
        planning_budget: GoalPlanningBudget::SELF_CARE,
    });
    // ... entries for every GoalKindDiscriminant variant
    registry
}
```

A workspace test asserts every `GoalKindDiscriminant` variant has a registry entry. Missing variants fail to build.

### D5: `AgentSchemaContextProfile` (universal)

```rust
// crates/worldwake-core/src/agent_schema_context_profile.rs (new)
#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct AgentSchemaContextProfile {
    pub disabled_extractors: BTreeSet<CandidateExtractorId>,
    pub enabled_methods: BTreeSet<MethodSchemaId>,
    pub budget_overrides: BTreeMap<GoalKindDiscriminant, GoalPlanningBudget>,
}
```

Universal per FND-22 (agent diversity through concrete variation). Defaults to empty (no overrides). Scenarios opt agents out of expensive extractors (e.g., a peasant lacks `EnterpriseExtractor`).

### D6: Migration of `agent_tick/planning.rs` candidate phase

The current explicit list:
```rust
let mut candidates = vec![];
emit_need_candidates(...);
emit_production_candidates(...);
// ... 16 more
```

Becomes:
```rust
let registry = ai_runtime.goal_schema_registry();
let mut candidates = vec![];
for schema in registry.values() {
    for extractor_id in &schema.candidate_extractors {
        if let Some(extractor) = registry.extractors.get(extractor_id) {
            if extractor.is_enabled_for(&actor.schema_context_profile) {
                candidates.extend(extractor.extract(&ctx));
            }
        }
    }
}
```

`CandidateOffer` continues to convert into `GoalOffer` as today. Suppression flows through the same `SuppressionReason` enum (per S144's D4).

### D7: Per-goal budget application in search

```rust
// crates/worldwake-ai/src/search/mod.rs
let goal_schema = registry.get(&candidate.goal_kind.discriminant())
    .expect("registry covers every GoalKindDiscriminant variant");
let effective_budget = GoalPlanningBudget {
    max_depth: goal_schema.planning_budget.max_depth.min(cognitive.max_plan_depth),
    max_node_expansions: goal_schema.planning_budget.max_node_expansions
        .min(cognitive.max_node_expansions),
    repair_budget_fraction: goal_schema.planning_budget.repair_budget_fraction,
    max_strategic_expansions: goal_schema.planning_budget.max_strategic_expansions
        .min(execution_budget.max_prerequisite_locations() as u16
             * goal_schema.planning_budget.max_strategic_expansions),
};
```

The search dispatch reads `effective_budget` rather than `cognitive.max_plan_depth` directly.

### D8: `PlanAttemptTrace.goal_budget` provenance

The trace records which `GoalPlanningBudget` was applied so S144's diagnostics can attribute exhaustion to budget tier (e.g., "9 of 12 BakeBread plans exhausted PRODUCTION budget").

### D9: Migration validation tests

- `goal_schema_registry_covers_all_kinds()` — every `GoalKindDiscriminant` has an entry.
- `extractor_outputs_match_legacy_emit_*()` — for each migrated extractor, a parity test against a captured fixture from the pre-S146 emit path on `survival-baseline.ron`.
- `per_goal_budget_caps_below_cognitive_ceiling()` — for every preset, depth/expansions respect `CognitiveProfile.max_plan_depth`/`max_node_expansions` ceilings.

### D10: Observer rendering

Observer Section 7 (planning) extends to surface the `GoalPlanningBudget` preset name (`SELF_CARE`, `PRODUCTION`, etc.) and the actual `max_depth` / `max_node_expansions` applied per plan attempt. S144's `PlanningMetrics` aggregates exhaustion-by-preset.

## FND-01 Section H Analysis

### Information-Path Analysis

S146 does not introduce new world-information flows. Candidate emission continues to read agent belief views; the registry only restructures *how* the read happens.

### Positive-Feedback Analysis

Not applicable. The registry is build-time data; no runtime feedback loop.

### Concrete Dampeners

`GoalPlanningBudget.max_strategic_expansions` is a concrete dampener on strategic search; the field is composed with `ExecutionBudget.max_prerequisite_locations` (per S145) and `CognitiveProfile.max_plan_depth` (existing) to bound search.

### Stored State vs. Derived Read-Model List

**Stored state**: `AgentSchemaContextProfile` on `EntityKind::Agent` (universal, defaults to empty). Required for scenario completeness per `docs/spec-drafting-rules.md` Section 5.

**Derived read-model**: `effective_budget` is computed per plan attempt from registry + profile + cognitive ceiling. `GoalSchema` registry is build-time data, not authoritative world state.

## SystemFn Integration

No new `SystemFn`. The registry is consulted during the existing AI tick's candidate-generation and search phases.

## Component Registration

- **New universal component**: `AgentSchemaContextProfile` registered on `EntityKind::Agent` in `worldwake-core/src/component_schema.rs`. Default impl; `expect()` access in runtime per Section 5 of `docs/spec-drafting-rules.md`.
- **No new role-specific components.**

## Cross-System Interactions

- Candidate generation reads belief views and snapshot state (existing).
- Search consumes per-goal budgets (new — reads registry data).
- Observer reads per-attempt budget provenance (new — read-only).

State-mediated per FND-26. No new direct system calls.

## Profile-Driven Parameters

- `AgentSchemaContextProfile.disabled_extractors` — per-agent extractor opt-out.
- `AgentSchemaContextProfile.enabled_methods` — per-agent HTN method opt-in (S147 substrate).
- `AgentSchemaContextProfile.budget_overrides` — per-agent budget overrides keyed by `GoalKindDiscriminant`.

All `Permille` values (where present) bound to [0, 1000].

## Test Plan

- D9 migration validation tests.
- Existing goldens regress unchanged on default profiles.
- New `golden_per_goal_budget.rs`: prove a `BakeBread` goal gets PRODUCTION budget (depth 16); prove an `Eat` goal gets SELF_CARE budget (depth 6).
- Registry-coverage compile-time test (workspace integration test).
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
