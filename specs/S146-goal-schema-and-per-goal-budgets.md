# S146: Data-Driven Goal Schema and Per-Goal Planning Budgets

**Status**: Draft

## Summary

Folds in PR-2 (Data-Driven GoalSchema Registry) and PR-17 (Per-Goal Planning Budgets) from `reports/ai-architecture-improvements.md`.

The current candidate-emission path in `crates/worldwake-ai/src/candidate_generation.rs` defines 20 hand-coded top-level candidate-emission families, one per goal family (`emit_need_candidates`, `emit_production_candidates`, `emit_enterprise_candidates`, `emit_disposal_candidates`, `emit_bounty_candidates`, `emit_artifact_posting_candidates`, `emit_combat_candidates`, `emit_crime_candidates`, `emit_social_candidates`, `emit_ask_witness_candidates`, `emit_patrol_candidates`, `emit_political_candidates`, `emit_recorded_violation_candidates`, `emit_search_candidates`, `emit_report_found_candidates`, `emit_escort_candidates`, `emit_exploration_candidates`, `emit_proactive_exploration_candidates`, `emit_expectation_violation_candidates`, `emit_opportunity_compiler_candidates`). Before ticket 005 they were called explicitly from `candidate_generation.rs::generate_candidates_with_memories_with_travel_horizon_impl`; `agent_tick/planning.rs` consumes generated candidates downstream. As the project grows toward "dozens or hundreds of goals", this hand-shape becomes brittle: each new goal family adds another emitter function, another call site, another set of suppression rules, and another lint surface.

The goal-kind registry now lives as `GoalSchema` (`crates/worldwake-ai/src/goal_schema.rs:61`), renamed in place from `GoalDispatchDeclaration` by `archive/tickets/S146GOASCHGOA-001.md`. It carries `provenance_family`, `trace_label`, `relevant_ops`, `invalidation_strategy`, `feasibility_strategy`, `frontier_exhaustion_strategy`, `family_policy`, and `progress_barrier_ops` per `GoalDispatchKey` (`crates/worldwake-core/src/goal_dispatch_key.rs:6` — the 41-variant discriminant-only enum that already has `Copy`, an `ALL` constant, and a `from_goal_kind(...)` mapping). S146 extends this declaration in place by adding exactly two new fields: `candidate_extractors: &'static [CandidateExtractorId]` (PR-2 fold-in) and `planning_budget: GoalPlanningBudget` (PR-17 fold-in). The 20 `emit_*` functions are migrated into a `CandidateExtractor` registry indexed by `CandidateExtractorId`. A new universal `AgentSchemaContextProfile` carries per-agent extractor opt-out and per-goal budget-override settings (so scenarios can opt agents out of expensive extractor families and tune budget tiers without code changes).

Per FND-28 single-truth: `GoalSchema` is the only goal-kind registry; no parallel core-resident `GoalSchema` is introduced, no `GoalKindDiscriminant` is added (the existing `GoalDispatchKey` is the discriminant). Per FND-28 "no dead paths": only fields backed by concrete S146 deliverables are added; ID-typed pointers to systems that don't yet exist (satisfaction predicates, ranking features, expectation templates, information-gap templates, motive-source hints, invalidator templates, HTN methods, explanation templates) are NOT introduced. Future specs add those fields when they have real backing implementations (e.g., S147 will add `methods: Vec<MethodSchemaId>` when HTN decomposition lands).

PR-17's per-goal budget gives each goal kind its own depth/expansion/repair tuning. The agent's `CognitiveProfile` remains the *ceiling*; per-goal budget chooses where within the ceiling the search lives. `Eat` gets a depth-6 budget; `ProduceCommodity` for bread gets a depth-16 budget; `InvestigateViolation` gets depth-20. The current `CognitiveProfile` defaults (`max_plan_depth = 8`, `max_node_expansions = 224`) are NOT changed — scenarios that want goals to plan past depth 8 must elevate `cognitive_profile.max_plan_depth` per agent. This preserves existing golden behavior and surfaces budget tier as an explicit per-scenario design choice.

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-ai` — owns `GoalSchema` (rename of existing `GoalDispatchDeclaration`), the `CandidateExtractor` trait, and the migrated extractor registry. Refactors `candidate_generation.rs` so the 20 emit functions live as `CandidateExtractor` impls keyed by the core-owned `CandidateExtractorId` values referenced from `GoalSchema.candidate_extractors`. Extends `decision_trace.rs` with the per-attempt `goal_budget` provenance field.
- `worldwake-core` — adds `GoalPlanningBudget` (concrete struct with preset constants) and `AgentSchemaContextProfile` (new universal ECS component on `EntityKind::Agent`). Registers the component in `component_schema.rs`. No change to authoritative world state shape beyond the new component.
- `worldwake-sim` — extends `GoalBeliefView` with a single accessor for `AgentSchemaContextProfile` so the AI crate can read it through the belief-view layer (Pattern: New Component Read by AI Crate).
- `worldwake-systems` — no change.
- `worldwake-cli` — observer Section 8's failed-plan attempt table renders the per-goal budget preset that bounded each failed plan attempt. Scenario loader (`scenario/types.rs` + `scenario/mod.rs`) gains `AgentDef.agent_schema_context_profile` field and the corresponding `spawn_agent` universal-application call.

## Dependencies

- S138 (Opportunity Compiler, archived at `archive/specs/S138-opportunity-compiler.md`) — provides `EffectSchemaIndex` and `Opportunity` substrate; the `OpportunityExtractor` impl (one of the `CandidateExtractor` impls in D3) reads `ctx.opportunities` from the planning snapshot.
- S141 (Motive Source Ledger, archived at `archive/specs/S141-motive-source-ledger.md`) — provides `MotiveSourceRef`; carried by `ExtractorContext` (D3) for extractor access to motive evidence.
- S134 (Canonical Effect Schema, archived at `archive/specs/S134-canonical-effect-schema.md`) — provides `EffectSchema` per `ActionDef`; consumed by extractors via existing planning infrastructure (no new field on `GoalSchema`).
- S145 (Planning Substrate Hardening, archived at `archive/specs/S145-planning-substrate-hardening.md`) — provides `ExecutionBudget::strategic_budget_for_stages`; composed into D7's `effective_budget` computation.
- S109 (Typed Discrepancy Taxonomy, archived at `archive/specs/S109-typed-discrepancy-taxonomy.md`) — `Discrepancy` substrate emitted by extractors on assumption failure (existing mechanism, no new field on `GoalSchema`).
- S144 (Aggregate Scenario Diagnostics, archived at `archive/specs/S144-aggregate-scenario-diagnostics.md`) — provides `PlanningMetrics` for D10 observer aggregation of exhaustion-by-preset.
- S147 (HTN Method Decomposition, draft at `specs/S147-htn-method-decomposition.md`) — will add `methods: Vec<MethodSchemaId>` to `GoalSchema` and `enabled_methods: BTreeSet<MethodSchemaId>` to `AgentSchemaContextProfile` when it lands. Not a hard dependency for S146.

## Design Goals

1. **One goal-kind registry, declaratively populated, in-place extension.** `GoalSchema` (the renamed `GoalDispatchDeclaration`) is the single registry. Adding a new goal kind means adding a `GoalSchema` entry and the supporting `GoalDispatchKey` variant — not editing parallel registries.
2. **Per-goal planning budgets are first-class.** `Eat`'s budget differs from `BakeBread`'s budget without per-agent profile tuning.
3. **No silent goal-family loss.** Every `GoalDispatchKey::ALL` variant must have a `GoalSchema` entry; a runtime test enforces coverage (compile-time enforcement deferred — see D9).
4. **Backward-compat-free.** The 20 top-level candidate-emission families are migrated, not aliased. The old explicit dispatch list in `candidate_generation.rs` is deleted.
5. **Deterministic.** Registry iteration is `BTreeMap`-ordered; extractors run in fixed order; no `HashMap` in authoritative state.
6. **Schema is data, not behavior.** Extractors are function pointers / trait impls; the schema's role is declarative metadata pointing at concrete dispatch.
7. **No fossilized scaffolding.** Only fields backed by S146 deliverables (`planning_budget`, `candidate_extractors`) are added to `GoalSchema`. ID-typed pointers to unimplemented systems are explicitly NOT introduced (FND-28).

## Non-Goals

- **No new authoritative world state besides `AgentSchemaContextProfile`.** `GoalSchema` is registry data, not per-agent state.
- **No HTN method decomposition.** That is S147's scope; `methods` field is NOT added to `GoalSchema` and `enabled_methods` is NOT added to `AgentSchemaContextProfile` by S146 — S147 will add both when it lands.
- **No new typed surfaces for satisfaction, ranking, expectations, information gaps, motive hints, invalidators, or explanations.** These functionalities already exist (`GoalKindPlannerExt::is_satisfied`, `ranking.rs::motive_score`, `Discrepancy`, `MotiveSource`, etc.) via concrete typed paths. Re-aliasing them through opaque ID newtypes inside `GoalSchema` would create FND-28-violating duplicate surfaces.
- **No live registry mutation.** The registry is a `const`-style table built at workspace init.
- **No LLM-driven schema generation.** Schemas are author-written Rust, per the assessment's "do not let live LLMs invent plans" guardrail.
- **No new event tag.** Schema registry is build-time data.
- **No change to `CognitiveProfile` defaults.** Scenarios that want goals to plan past depth 8 must author elevated `cognitive_profile.max_plan_depth` per agent.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `GoalSchema` declares concrete extractor IDs and budget data; no abstract goal-family score. Only fields with concrete S146 implementations are added. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | Per-goal planning budgets express how the agent should reason about each goal type, not what outcome they should achieve. |
| FND-22 (Agent Diversity Through Concrete Variation) | `AgentSchemaContextProfile` lets per-agent profiles disable extractor families and override budget tiers, supporting later archetype work. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Extractors read belief views and snapshot state; the schema registry is shared build-time data, not cross-system commands. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | `GoalDispatchDeclaration` is renamed in place (no parallel `GoalSchema`); the 20 `emit_*` functions are migrated and deleted; no shim layer. Fields backed only by speculative future systems are explicitly NOT added (no fossilized scaffolding). |
| FND-29 (Debuggability Is a Product Feature) | `PlanAttemptTrace.goal_budget` records which budget tier bounded each plan attempt; observer Section 8 renders the preset name for failed plan attempts. |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | Schema additions and `AgentSchemaContextProfile` introduce no new world-information flows; Section H covers the (limited) scope below. |

## Deliverables

### D1: `GoalSchema` type (+2 fields after in-place rename)

`S146GOASCHGOA-001` renamed `GoalDispatchDeclaration` → `GoalSchema` in place across `worldwake-ai`. The struct keeps its 8 existing fields and gains exactly two new fields:

```rust
// crates/worldwake-ai/src/goal_schema.rs
pub struct GoalSchema {
    // existing fields, unchanged:
    pub provenance_family: RankedGoalProvenanceFamily,
    pub trace_label: &'static str,
    pub relevant_ops: &'static [PlannerOpKind],
    pub invalidation_strategy: InvalidationStrategy,
    pub feasibility_strategy: FeasibilityStrategy,
    pub frontier_exhaustion_strategy: FrontierExhaustionStrategy,
    pub family_policy: FamilyPolicy,
    pub progress_barrier_ops: &'static [PlannerOpKind],
    // new fields for S146:
    pub candidate_extractors: &'static [CandidateExtractorId],
    pub planning_budget: GoalPlanningBudget,
}
```

`CandidateExtractorId` is the core-owned 20-variant enum landed by `archive/tickets/S146GOASCHGOA-004.md` in `crates/worldwake-core/src/agent_schema_context_profile.rs`. The `static DECL_*` entries (currently `DECL_CONSUME_OWNED_COMMODITY`, `DECL_ACQUIRE_SELF_CONSUME`, …, ~41 entries) populate the two new fields. The discriminant remains `GoalDispatchKey`; no `GoalKindDiscriminant` is introduced.

### D2: `GoalPlanningBudget`

```rust
// crates/worldwake-core/src/goal_planning_budget.rs (new)
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
        repair_budget_fraction: Permille::new_unchecked(250),
        max_strategic_expansions: 12,
    };
    pub const TRAVEL_PURCHASE: Self = Self {
        max_depth: 10,
        max_node_expansions: 224,
        repair_budget_fraction: Permille::new_unchecked(300),
        max_strategic_expansions: 24,
    };
    pub const PRODUCTION: Self = Self {
        max_depth: 16,
        max_node_expansions: 384,
        repair_budget_fraction: Permille::new_unchecked(400),
        max_strategic_expansions: 48,
    };
    pub const INVESTIGATION: Self = Self {
        max_depth: 20,
        max_node_expansions: 512,
        repair_budget_fraction: Permille::new_unchecked(450),
        max_strategic_expansions: 64,
    };
    pub const BOUNTY_ESCORT: Self = Self {
        max_depth: 24,
        max_node_expansions: 768,
        repair_budget_fraction: Permille::new_unchecked(500),
        max_strategic_expansions: 96,
    };
}
```

The agent's `CognitiveProfile.max_plan_depth` (default = 8 per `crates/worldwake-core/src/cognitive_profile.rs:134`) remains the *ceiling*. The search uses `min(cognitive_max, goal_budget.max_depth)`. Scenarios that need a goal to actually plan past depth 8 must author an elevated `cognitive_profile.max_plan_depth` per agent. D9 includes a clamp-interaction note; the per-goal-budget golden in D9 uses an explicit elevated cognitive profile.

Uses `Permille::new_unchecked` (`crates/worldwake-core/src/numerics.rs:43`) since preset constants are statically known to be `<= 1000`.

### D3: `CandidateExtractor` trait + registry

```rust
// crates/worldwake-ai/src/goal_schema_registry/extractors.rs (new)
pub trait CandidateExtractor {
    fn extract(&self, ctx: &ExtractorContext<'_>) -> Vec<GoalOffer>;
    fn id(&self) -> CandidateExtractorId;
    fn is_enabled_for(&self, profile: &AgentSchemaContextProfile) -> bool {
        !profile.disabled_extractors.contains(&self.id())
    }
}

pub struct ExtractorContext<'a> {
    pub generation: &'a GenerationContext<'a>,
    pub diagnostics: &'a mut CandidateGenerationDiagnostics,
}
```

`ExtractorContext` wraps the existing `GenerationContext` (which already provides `view: &dyn GoalBeliefView`, `agent: EntityId`, `place: EntityId`, `recipes: &RecipeRegistry`, `current_tick: Tick`, `blocked`, `discrepancies`, `violation_memory`, `opportunities`, `current_plan`, etc.) and the existing `CandidateGenerationDiagnostics` (the actual type currently passed to emit functions; `crates/worldwake-ai/src/candidate_generation.rs:197+`). No new types beyond `CandidateExtractor` and `ExtractorContext` are required. The trait deliberately uses `GoalBeliefView` (via `GenerationContext.view`) to preserve parity with existing emit signatures; widening to `RuntimeBeliefView` would force every migrated body to re-cast.

The 20 existing `emit_*` functions become `impl CandidateExtractor` per goal family: `NeedExtractor`, `ProductionExtractor`, `EnterpriseExtractor`, `DisposalExtractor`, `BountyExtractor`, `ArtifactPostingExtractor`, `CombatExtractor`, `CrimeExtractor`, `SocialExtractor`, `AskWitnessExtractor`, `PatrolExtractor`, `PoliticalExtractor`, `RecordedViolationExtractor`, `SearchExtractor`, `ReportFoundExtractor`, `EscortExtractor`, `ExplorationExtractor`, `ProactiveExplorationExtractor`, `ExpectationViolationExtractor`, `OpportunityCompilerExtractor`. Each impl's `extract()` body is the function body of the corresponding `emit_*` function, with the candidates collected into and returned from a local `Vec<GoalOffer>` rather than pushed into a `&mut Vec` parameter. The per-extractor goal-specific input parameters (e.g., `needs`, `thresholds`) are read from `ctx.generation` via existing fields or new helper methods on `GenerationContext` as needed.

Suppression flows through the existing `CandidateGenerationDiagnostics` mechanism (the `CandidateSuppressionDiagnostic` records); no new `SuppressionLog` type is introduced.

### D4: Registry build-time entries (additive)

Each of the existing ~41 `static DECL_*` entries in `crates/worldwake-ai/src/goal_schema.rs` is updated to populate the two new fields. Example:

```rust
static DECL_CONSUME_OWNED_COMMODITY: GoalSchema = GoalSchema {
    // ... existing 8 fields ...
    candidate_extractors: &[CandidateExtractorId::Need],
    planning_budget: GoalPlanningBudget::SELF_CARE,
};
```

The `candidate_extractors` field uses `&'static [CandidateExtractorId]` (matching the existing `relevant_ops: &'static [PlannerOpKind]` convention). Populating it for each entry maps the goal kind to one or more extractor IDs from D3.

`PlannerOpKind` variants are read directly — `relevant_ops` already uses the actual enum (no `btreeset!` macro is required and none exists in the workspace). The eating path uses `PlannerOpKind::Consume` (`planner_ops.rs:16`), not the (nonexistent) `PlannerOpKind::Eat`.

### D5: `AgentSchemaContextProfile` (universal ECS component)

```rust
// crates/worldwake-core/src/agent_schema_context_profile.rs (new)
#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct AgentSchemaContextProfile {
    pub disabled_extractors: BTreeSet<CandidateExtractorId>,
    pub budget_overrides: BTreeMap<GoalDispatchKey, GoalPlanningBudget>,
}
```

Universal per FND-22 (agent diversity through concrete variation). `Default` impl yields empty sets/maps — no override. Scenarios opt agents out of expensive extractors (e.g., a peasant with `disabled_extractors: BTreeSet::from([CandidateExtractorId::Enterprise])`) or override budget tier per goal kind.

The component contains no `EntityId` references (the fields use `CandidateExtractorId` enum values, `GoalDispatchKey`, and `GoalPlanningBudget`), so it does NOT need a `*Def` wrapper type — `AgentSchemaContextProfile` itself serves as the scenario-authorable shape. `MethodSchemaId` / `enabled_methods` are explicitly NOT included; S147 will add them when HTN method decomposition lands.

Defined in `worldwake-core` per the core-residence constraint for ECS components (Pattern: New Component on EntityKind::Agent).

### D6: Migration of `candidate_generation.rs` candidate phase

The current explicit list of candidate-emission family calls is replaced by registry-driven dispatch. The profile is read through the belief-view accessor introduced in D11 (NOT through a `.schema_context_profile` field — `actor` is an `EntityId`, not a struct):

```rust
let extractors = build_extractor_registry();
let profile = ctx.view.agent_schema_context_profile(ctx.agent);
let mut candidates: Vec<GoalOffer> = Vec::new();
for extractor_id in ordered_candidate_extractors_from_goal_schemas() {
    let extractor = extractors
        .get(&extractor_id)
        .expect("extractor registry covers CandidateExtractorId::ALL");
    if !extractor.is_enabled_for(profile) {
        continue;
    }
    let mut ext_ctx = ExtractorContext {
        generation: &ctx,
        diagnostics: &mut ctx.diagnostics,
        prior_candidates: &candidates,
        // pending side-effect buffers omitted for brevity
    };
    candidates.extend(extractor.extract(&mut ext_ctx));
}
```

`ordered_candidate_extractors_from_goal_schemas()` is a deduped, legacy-order read of the `GoalSchema.candidate_extractors` metadata, not a direct "run once per schema entry" loop. This matters because several goal kinds share one extractor family and some goal kinds have multiple producer families (`InvestigateViolation`, `ExploreLocation`). The direct candidate-family call list in `candidate_generation.rs` is deleted. `GoalOffer` conversion remains unchanged.

### D7: Per-goal budget application in search

```rust
// crates/worldwake-ai/src/search/mod.rs
let goal_schema = registry.get(&GoalDispatchKey::from_goal_kind(&candidate.goal_kind))
    .expect("registry covers every GoalDispatchKey variant");
let stage_count = /* derived from candidate's prerequisite stages */;
let effective_budget = GoalPlanningBudget {
    max_depth: goal_schema.planning_budget.max_depth.min(cognitive.max_plan_depth),
    max_node_expansions: goal_schema.planning_budget.max_node_expansions
        .min(cognitive.max_node_expansions),
    repair_budget_fraction: goal_schema.planning_budget.repair_budget_fraction,
    max_strategic_expansions: goal_schema.planning_budget.max_strategic_expansions
        .min(execution_budget.strategic_budget_for_stages(stage_count) as u16),
};
```

The search dispatch reads `effective_budget` rather than `cognitive.max_plan_depth` directly. Composition with `ExecutionBudget::strategic_budget_for_stages` (`crates/worldwake-core/src/execution_budget.rs`, signature: `pub const fn strategic_budget_for_stages(&self, stage_count: usize) -> usize`) caps strategic search by the planner-substrate hardening from S145.

### D8: `PlanAttemptTrace.goal_budget` provenance

Add field to `PlanAttemptTrace` at `crates/worldwake-ai/src/decision_trace.rs:1157`:

```rust
pub struct PlanAttemptTrace {
    // ... existing fields ...
    pub goal_budget: GoalPlanningBudget,
}
```

`GoalPlanningBudget` is `Copy + Serialize + Deserialize` (per D2's derives) so it satisfies whatever trait bounds `PlanAttemptTrace` already requires (verify at implementation time). Constructors of `PlanAttemptTrace` populate `goal_budget` from the `effective_budget` computed in D7. S144's diagnostics aggregate exhaustion-by-preset by inspecting this field.

### D9: Migration validation tests

- `goal_schema_registry_covers_all_keys()` — runtime `#[test]` asserting every variant in `GoalDispatchKey::ALL` has a corresponding `GoalSchema` entry. This is a runtime test, not a compile-time enforcement; achieving compile-time enforcement would require a `match key { ... }` in a const context and is deferred.
- `schema_derived_extractor_order_covers_every_registered_extractor_once()` — runtime `#[test]` asserting `build_extractor_registry()` covers every `CandidateExtractorId::ALL` entry, the schema-derived dispatch sequence preserves the legacy top-level extractor order, and schema-shared extractor IDs are deduped before dispatch. This replaced the originally drafted JSON parity fixture plan because no pre-migration fixture capture target or committed fixture set exists on the live branch; capturing fixtures after migration would be circular evidence.
- `per_goal_budget_caps_below_cognitive_ceiling()` — for every preset, verify `effective_budget` correctly clamps depth/expansions against `CognitiveProfile.max_plan_depth`/`max_node_expansions` for the default-8 cognitive ceiling AND for an elevated 24-depth ceiling.
- `s146_goal_kind_budget_examples_map_to_expected_presets()` and `s146_search_trace_records_per_goal_budget_under_elevated_cognitive_ceiling()` — focused search tests using an explicit `CognitiveProfile { max_plan_depth: 24, max_node_expansions: 768, ... }` profile. They prove `ProduceCommodity` maps to `PRODUCTION`, self-care consume/acquire goals map to `SELF_CARE`, and the applied search trace records those depth/node-expansion limits. This replaced the originally drafted autonomous `golden_per_goal_budget.rs` because the search metadata handoff is the strongest stable proof seam for the per-goal budget contract.

### D10: Observer rendering

Observer Section 8's per-agent failed-plan attempt table (`crates/worldwake-cli/src/bin/observer.rs`) extends to surface the `GoalPlanningBudget` preset name (`SELF_CARE`, `TRAVEL_PURCHASE`, `PRODUCTION`, `INVESTIGATION`, `BOUNTY_ESCORT`, or `CUSTOM` when a profile override doesn't match a preset) and the actual `max_depth` / `max_node_expansions` applied to each rendered failed plan attempt (read from D8's `goal_budget` field). S144's `PlanningMetrics` aggregates exhaustion-by-preset using the same field.

Preset-name rendering is performed by a small `GoalPlanningBudget::preset_name() -> Option<&'static str>` method that returns `Some("SELF_CARE")` etc. when the budget matches a defined preset and `None` otherwise — no separate `ExplanationTemplateId` indirection is introduced.

### D11: `GoalBeliefView` accessor for `AgentSchemaContextProfile` (Pattern: New Component Read by AI Crate)

Add a single accessor to the existing `GoalBeliefView` trait (`crates/worldwake-sim/src/belief_view.rs:317`) and its supertraits as needed:

```rust
// crates/worldwake-sim/src/belief_view.rs
pub trait ProfileBeliefView {
    // ... existing accessors ...
    fn agent_schema_context_profile(&self, agent: EntityId) -> &AgentSchemaContextProfile;
}
```

Backing implementation in `RuntimeBeliefView` reads the component via the standard `world.get_component_agent_schema_context_profile(agent).expect("...")` accessor generated by `with_component_schema_entries!`. Forwarding through `impl_goal_belief_view!` (or whichever blanket-impl macro is current) follows the same pattern as existing profile accessors (e.g., `cognitive_profile`).

The `expect()` access is justified per `docs/spec-drafting-rules.md` Section 5: universal profiles on known agents use `expect()` because every agent is guaranteed to carry the component (D12 guarantees scenario-load population; the `Default` impl from D5 guarantees post-load presence).

### D12: Scenario integration for `AgentSchemaContextProfile` (Pattern: New Component on EntityKind::Agent)

Three coordinated edits in `worldwake-cli`:

1. `crates/worldwake-cli/src/scenario/types.rs`: add field to `AgentDef`:
   ```rust
   #[serde(default)]
   pub agent_schema_context_profile: Option<AgentSchemaContextProfile>,
   ```
   No `*Def` wrapper needed (per D5: no `EntityId` references).

2. `crates/worldwake-cli/src/scenario/mod.rs`, in `spawn_agent()` near the existing universal-profile application block (around line 590+, alongside `metabolism_profile.unwrap_or_default()` etc.):
   ```rust
   let schema_profile = agent_def.agent_schema_context_profile.unwrap_or_default();
   txn.set_component_agent_schema_context_profile(agent_id, schema_profile)?;
   ```

3. `crates/worldwake-core/src/component_schema.rs`: register `AgentSchemaContextProfile` through `with_component_schema_entries!` with the `|kind| kind == EntityKind::Agent` filter, generating `set_component_agent_schema_context_profile`, `get_component_agent_schema_context_profile`, etc. accessors.

## FND-01 Section H Analysis

### Information-Path Analysis

S146 does not introduce new world-information flows. Candidate emission continues to read agent belief views; the registry only restructures *how* the read happens. `AgentSchemaContextProfile` is configuration loaded at scenario time, not perceived from the world.

### Positive-Feedback Analysis

Not applicable. The registry is build-time data; no runtime feedback loop. Per-goal budget composition is bounded by `min()` with `CognitiveProfile` and `ExecutionBudget` ceilings.

### Concrete Dampeners

`GoalPlanningBudget.max_strategic_expansions` is a concrete dampener on strategic search; the field is composed with `ExecutionBudget::strategic_budget_for_stages` (per S145) and `CognitiveProfile.max_plan_depth` / `max_node_expansions` (existing) via the `min()` interactions in D7.

### Stored State vs. Derived Read-Model List

**Stored state**:
- `AgentSchemaContextProfile` on `EntityKind::Agent` (universal, defaults to empty per D5; scenario-authored per D12).
- `PlanAttemptTrace.goal_budget` (per-attempt provenance recorded into the existing trace state per D8).

**Derived read-model / build-time data**:
- `GoalSchema` registry — build-time `static` table; not authoritative world state.
- `CandidateExtractor` registry — build-time function-pointer dispatch table.
- `effective_budget` — computed per plan attempt from registry + profile + cognitive ceiling per D7.

## SystemFn Integration

No new `SystemFn`. The registry is consulted during the existing AI tick's candidate-generation and search phases.

## Component Registration

- **New universal component**: `AgentSchemaContextProfile` registered on `EntityKind::Agent` in `worldwake-core/src/component_schema.rs` per D12. `Default` impl yields empty `disabled_extractors` and empty `budget_overrides`; runtime `expect()` access is justified per `docs/spec-drafting-rules.md` Section 5.
- **No new role-specific components.**

## Cross-System Interactions

- Candidate generation reads belief views and snapshot state via `ExtractorContext.generation` (existing data, restructured access).
- Candidate generation reads `AgentSchemaContextProfile` via the new `GoalBeliefView::agent_schema_context_profile` accessor (D11).
- Search consumes per-goal budgets from the schema registry (D7).
- Observer reads per-attempt budget provenance from `PlanAttemptTrace.goal_budget` (D10, read-only).

State-mediated per FND-26. No new direct system calls.

## Profile-Driven Parameters

- `AgentSchemaContextProfile.disabled_extractors` — per-agent extractor opt-out (set of `CandidateExtractorId`).
- `AgentSchemaContextProfile.budget_overrides` — per-agent budget overrides keyed by `GoalDispatchKey`.

All `Permille` values (where present in `GoalPlanningBudget` presets) are bound to `[0, 1000]` via `Permille::new_unchecked` at the const constants.

## Test Plan

- D9 migration validation tests (registry coverage, schema-derived extractor dispatch order/deduping, budget clamping, concrete per-goal search trace examples).
- Existing goldens regress unchanged on default profiles (per Q3 / Issue 2 resolution: cognitive defaults unchanged → `min(8, preset.max_depth)` for every preset above 8 → existing-golden-effective depth identical to pre-S146).
- Focused search trace regression: prove a `BakeBread`-style `ProduceCommodity` goal gets `PRODUCTION` budget depth/expansions under elevated cognitive profile; prove self-care consume/acquire goals get `SELF_CARE` budget depth/expansions.
- Registry-coverage runtime test (workspace integration test).
- `cargo clippy --workspace --all-targets -- -D warnings` clean.

## Authoritative-to-AI Impact Analysis

D6 restructures candidate emission and D7 changes the budget input source. The seven `AGENTS.md` Authoritative-To-AI Impact Rule points:

1. `get_affordances` — N/A (unchanged).
2. `generate_candidates` — restructured into extractor dispatch. D9 verifies registry coverage, legacy-order schema-derived dispatch, extractor-ID deduping, and existing candidate-generation regression coverage rather than a stale pre-migration JSON fixture set.
3. `search_plan` — budget input source changes. Terminal ordering may shift if effective budget differs from the previous uniform value. Under Q3=(a)'s resolution (cognitive defaults unchanged, every preset clamped to 8 by default), effective budget for default agents is identical to pre-S146; goldens with elevated cognitive ceilings see the new differentiated budgets.
4. `BestEffort` action start — N/A (unchanged).
5. `handle_plan_failure` — N/A (unchanged).
6. Payload revalidation — N/A (unchanged).
7. Golden tests — must pass post-migration under the existing default-profile golden suites; D9 covers the per-goal budget handoff at the stronger focused search-trace layer.
