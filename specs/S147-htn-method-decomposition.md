# S147: HTN Method Decomposition for Long Lawful Pursuits

**Status**: Draft

## Summary

The external assessment in `reports/ai-architecture-improvements.md` Section 7 (Upgrade D) recommends adding HTN-style methods *above* the existing GOAP planner, applied to goals where naive forward search has prohibitive cost: `FulfillBounty`, `InvestigateViolation`, `Accuse`, `PunishAccused`, `ProduceCommodity`, `RestockCommodity`, `MoveCargo`, `EscortToSafety`, `SearchForMissing`, `ClaimOffice`, `SupportCandidateForOffice`, plus future caravan/patrol/construction/repair/inheritance work. Phase 11 explicitly deferred HTN methods until "S134 (effect schemas), S138 (opportunity compiler), and S141 (motive sources) land" — all archived. The deferral condition is met.

The current two-tier planner (`crates/worldwake-ai/src/search/strategic.rs:35`) already does implicit HTN-like decomposition: `StrategicStage` decomposes a goal into `Goal` and `Acquire(CommodityKind)` stages. The decomposition is generic: any goal with a missing-commodity assumption emits an acquire stage. This works for `Eat` (acquire food → eat) and `BakeBread` (acquire grain → acquire flour → acquire bread-recipe-input → bake) but expresses the decomposition in code, not data.

S147 lands `MethodSchema` as the data-driven layer above strategic decomposition. A `MethodSchema` is a *lawful pursuit pattern*: "how a guard investigates," "how a hunter fulfills a bounty," "how a merchant restocks," not "what story beat should happen." Every leaf remains an ordinary `ActionDef`. Every artifact remains world state. Methods describe search control and reusable craft knowledge per FND-20.

The first ship covers `FulfillBounty`, `ProduceCommodity`, `RestockCommodity`, `InvestigateViolation`, and `EscortToSafety`. Methods are author-written Rust (built-time data, like S146's `GoalSchema` registry); they live in `crates/worldwake-ai/src/htn/methods.rs`. The planner consults method options before forward search: when a method's preconditions are satisfied, the planner substitutes its subgoals into the strategic itinerary. When no method applies, the planner falls back to flat GOAP as today (no behavioral regression on goals without methods).

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-ai` — owns `htn` module: `MethodSchema`, `MethodSchemaId`, `MethodSelector`, the method registry, and the planner integration point in `search/strategic.rs`.
- `worldwake-core` — exposes `MethodSchemaId` newtype and shared `SubgoalTemplate` types referenced by the `GoalSchema.methods` field added in this spec.
- `worldwake-sim` — no change.
- `worldwake-systems` — no change.
- `worldwake-cli` — observer Section 7 extends to surface the method chosen per plan attempt and its decomposition trace.

## Dependencies

- S146 (Goal Schema and Per-Goal Budgets, archived at `archive/specs/S146-goal-schema-and-per-goal-budgets.md`, hard dep) — provides the `GoalSchema` registry substrate and `AgentSchemaContextProfile`; this S147 spec adds `GoalSchema.methods: Vec<MethodSchemaId>` and `AgentSchemaContextProfile.enabled_methods`.
- S138 (Opportunity Compiler, archived, hard dep) — `MethodSchema.required_claims` references `Opportunity` matches.
- S141 (Motive Source Ledger, archived, hard dep) — method selection consumes `MotiveSourceRef`s to bias method choice (Loyalty → group hunt, Revenge → direct hunt).
- S134 (Canonical Effect Schema, archived, hard dep) — `MethodSchema.expected_artifacts` references `EffectSchema` post-conditions for verifying method completion.
- S109 (Typed Discrepancy Taxonomy, archived) — method failure modes produce typed `Discrepancy` per `MethodSchema.failure_modes`.
- S125 (Institutional Treasuries and Bounty Funding, archived) — `FulfillBounty` method uses treasury-backed reward release.

## Design Goals

1. **Methods are lawful pursuit patterns, not story beats.** Per FND-20: a method describes how an agent pursues a world condition under beliefs; it does not author the outcome.
2. **Methods are data, not behavior.** The registry is build-time `MethodSchema` entries; per-tick computation is a deterministic method selector.
3. **GOAP remains the executor.** Method leaves are ordinary `ActionDef`s; the strategic + tactical search remains the runtime engine.
4. **No silent privilege.** Methods cannot read off-place world state or invoke other systems. They only constrain *which* subgoals get attempted.
5. **Per-agent method enablement.** Some agents naturally know more methods than others (a hunter knows GroupHunt; a peasant does not). `AgentSchemaContextProfile.enabled_methods` (added by S147 on the S146 profile) gates which methods apply.
6. **Deterministic method selection.** Same belief state, same enabled methods, same motive sources → same method choice. No randomness in method dispatch.

## Non-Goals

- **No method authoring DSL.** Methods are Rust structs; LLM-driven method generation is out of scope (per the assessment's explicit guardrail).
- **No story-beat methods.** "Hero answers the call" or "merchant betrays guard" are not methods; they would violate FND-20.
- **No new top-level reasoning framework.** Methods sit *under* the archived S146 `GoalSchema` registry, not above it.
- **No method-only goals.** Every method-decomposed goal must also have a fallback GOAP path. Removing all methods returns the goal to today's behavior.
- **No method learning.** Methods are author-written. Per-method learning (PR-8 / habit memory) is deferred.
- **No method-internal scheduling.** Sub-goals execute through the agenda manager (S115); methods do not have internal tick loops.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | Methods encode reusable affordance composition; they do not encode plot progression. The "test" — "this kind of agent pursues this kind of world condition under these beliefs" — is exactly satisfied. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Methods read belief views and snapshot state; they produce subgoal templates that the planner consumes. No cross-system command. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | Methods extend the planner; the no-method fallback is the actual current path, not a legacy shim. |
| FND-29 (Debuggability Is a Product Feature) | Method choice and decomposition are recorded in `PlanAttemptTrace.method_trace`; observer Section 7 surfaces them. |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | Every `MethodSchema` carries `failure_modes`, `expected_artifacts`, `required_claims`, and `preconditions` — the FND-30 fields. |

## Deliverables

### D1: `MethodSchema`

```rust
// crates/worldwake-ai/src/htn/method_schema.rs (new)
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MethodSchema {
    pub id: MethodSchemaId,
    pub goal_kind: GoalKindDiscriminant,
    pub preconditions: Vec<MethodPrecondition>,
    pub subgoals: Vec<SubgoalTemplate>,
    pub expected_artifacts: Vec<ArtifactTemplate>,
    pub required_claims: Vec<ClaimRequirement>,
    pub failure_modes: Vec<MethodFailureMode>,
    pub explanation_template: ExplanationTemplateId,
    pub motive_bias: Vec<MotiveBias>,           // S141 integration
    pub planning_budget_hint: Option<GoalPlanningBudget>,   // S146 override
}

pub enum MethodPrecondition {
    BeliefHolds(BeliefPredicate),
    MotiveSourcePresent(MotiveSourceVariantId),
    AgentRole(RoleTag),
    LocationKnown(EntityCriterion),
}

pub enum SubgoalTemplate {
    AcquireCommodity { commodity: CommodityKind, min_quantity: Quantity },
    TravelTo(LocationTemplate),
    ObserveTarget(EntityCriterion),
    AskWitness(TopicTemplate),
    InspectArtifact(ArtifactTemplate),
    PerformAction(PlannerOpKind, PayloadTemplate),
    ResolveCoordination(ClaimRequirement),
    ReturnTo(LocationTemplate),
}

pub struct MotiveBias {
    pub motive_variant: MotiveSourceVariantId,
    pub weight: Permille,
}

pub enum MethodFailureMode {
    PreconditionLost(BeliefPredicate),
    SubgoalUnachievable(usize),         // index into subgoals
    ArtifactNotProduced(ArtifactTemplate),
    ClaimDenied(ClaimRequirement),
    Timeout(u32),                        // ticks
}
```

`Permille` for all weight values. No floats.

### D2: First-ship methods

Each method below is shipped as a `MethodSchema` entry in `build_method_registry()`. Names match the assessment's Section 7 examples.

**FulfillBounty methods**:
- `FulfillBountyDirect`: preconditions — agent believes bounty record + target last-seen + reward terms. Subgoals — `AcquireCommodity` for weapons/supplies if missing → `TravelTo` last-seen → `ObserveTarget` → `PerformAction(Subdue/Kill)` → collect proof → `ReturnTo` bounty issuer → `PerformAction(ClaimBounty)`. Failure modes — `PreconditionLost(BountyExpired)`, `SubgoalUnachievable(track_target)`.
- `FulfillBountyInvestigation`: preconditions — bounty known but target location uncertain, witnesses or records believed available. Subgoals — `AskWitness(target_whereabouts)` or `InspectArtifact(violation_record)` → update lead → resume FulfillBountyDirect.
- `FulfillBountyGroupHunt`: preconditions — target believed dangerous, ally or bounty office available. Subgoals — `PerformAction(RecruitAlly)` → synchronize at staging → `TravelTo` confrontation place → confront. Motive bias toward `Loyalty(ally)`.

**ProduceCommodity methods**:
- `ProduceFromOwnedStock`: precondition — agent owns inputs and workstation in same place. Subgoals — `PerformAction(ProduceCommodity)`.
- `ProduceWithGather`: precondition — agent missing some inputs but knows extractable sources. Subgoals — for each missing input: `AcquireCommodity` → return to workstation → produce.
- `ProduceWithPurchase`: precondition — agent missing inputs but knows seller. Subgoals — `TravelTo` seller → `PerformAction(BuyCommodity)` → return → produce.

**RestockCommodity methods**:
- `RestockFromHarvest`: precondition — own commodity below threshold, source known. Subgoals — `AcquireCommodity` from source.
- `RestockFromMarket`: precondition — own commodity below threshold, seller known. Subgoals — `TravelTo` seller → `PerformAction(BuyCommodity)`.

**InvestigateViolation methods**:
- `InvestigateOnScene`: precondition — violation record believed at known place, agent at place. Subgoals — `InspectArtifact(VialolationEvidence)` → record finding.
- `InvestigateByWitness`: precondition — violation believed, witness names known. Subgoals — `AskWitness(violation_circumstances)` → record finding.
- `InvestigateByLedger`: precondition — violation believed, institutional record believed extant. Subgoals — `TravelTo` office → `InspectArtifact(Ledger)` → record finding.

**EscortToSafety methods**:
- `EscortToHome`: precondition — escortee believed safe at home. Subgoals — synchronize at escortee place → `TravelTo` home with escortee → confirm arrival.
- `EscortToOffice`: precondition — institutional protection believed available. Subgoals — `TravelTo` office with escortee → `PerformAction(HandOffToOffice)`.

### D3: `MethodSelector`

```rust
// crates/worldwake-ai/src/htn/selector.rs
pub fn select_method(
    goal: &GoalOffer,
    registry: &MethodRegistry,
    profile: &AgentSchemaContextProfile,
    belief_view: &dyn RuntimeBeliefView,
    motives: &[MotiveSourceRef],
) -> Option<&MethodSchema>;
```

Deterministic selection:
1. Filter methods to those whose `goal_kind` matches and `id ∈ profile.enabled_methods`.
2. Filter to methods whose `preconditions` are satisfied per belief view.
3. Rank remaining methods by motive-source bias score (sum of `MotiveBias.weight` for present motive variants).
4. Tie-break by `MethodSchemaId` (stable integer order).
5. Return the top method or `None` (fall back to flat GOAP).

### D4: Planner integration

```rust
// crates/worldwake-ai/src/search/strategic.rs
fn build_stages_with_method(...) -> Vec<StrategicStage> {
    if let Some(method) = select_method(&goal, &registry, &profile, &belief_view, &motives) {
        return method.subgoals.iter()
            .flat_map(|template| template_to_stages(template, &belief_view))
            .collect();
    }
    build_stages_default(...)   // existing flat-GOAP decomposition
}
```

When a method is chosen, its subgoals expand into `StrategicStage`s via the existing decomposition machinery. The strategic search itinerates over the method's stages. Tactical search remains unchanged.

### D5: `MethodPlanAttemptTrace`

```rust
pub struct MethodPlanAttemptTrace {
    pub method_id: Option<MethodSchemaId>,    // None = flat GOAP fallback
    pub subgoals_attempted: Vec<SubgoalAttemptResult>,
    pub failure_mode: Option<MethodFailureMode>,
}
```

Surfaced through `PlanAttemptTrace` to observer Section 7 and to `ScenarioDiagnosticsReport.planning.method_usage` (extension to S144).

### D6: Method failure → typed `Discrepancy`

`MethodFailureMode` variants map to S109 `Discrepancy` variants:
- `PreconditionLost(...)` → `Discrepancy::BeliefStale` or `Discrepancy::BeliefContradicted`.
- `SubgoalUnachievable(...)` → `Discrepancy::SearchBudgetExhausted` with method context.
- `ArtifactNotProduced(...)` → `Discrepancy::PartialExecutionDrift`.
- `ClaimDenied(...)` → `Discrepancy::NoLegalBinding`.
- `Timeout(...)` → `Discrepancy::PartialExecutionDrift` with method context.

This preserves the existing discrepancy-and-blocker memory paths (S109).

### D7: `AgentSchemaContextProfile.enabled_methods` defaults

By default, every agent has `enabled_methods = registry.all_method_ids()` (full method library available). Scenarios opt agents out by setting `enabled_methods` to a narrower set. The S111 `ProfileHomogeneity` lint warns when every agent in a scenario has identical method enablement.

### D8: Method registry build-time table

```rust
// crates/worldwake-ai/src/htn/registry.rs
pub fn build_method_registry() -> MethodRegistry {
    let mut registry = MethodRegistry::default();
    registry.insert(FulfillBountyDirect::schema());
    registry.insert(FulfillBountyInvestigation::schema());
    registry.insert(FulfillBountyGroupHunt::schema());
    registry.insert(ProduceFromOwnedStock::schema());
    // ... etc
    registry
}
```

Workspace test asserts every entry's `goal_kind` references a real `GoalKindDiscriminant` and every `SubgoalTemplate`'s referenced `PlannerOpKind` exists.

### D9: Observer Section 7 method rendering

For each plan attempt, observer Section 7 prints:
```
Plan attempt: BakeBread (Method: ProduceWithGather)
  Subgoal 1: AcquireCommodity(Grain) ✓
  Subgoal 2: AcquireCommodity(Flour) ✓
  Subgoal 3: PerformAction(Bake) — Pending
```

Failed method attempts surface the `MethodFailureMode` with structured prose.

### D10: Golden coverage

`golden_htn_methods.rs` covers:
- `FulfillBountyDirect` end-to-end: agent has direct target knowledge → method selected → bounty fulfilled.
- `FulfillBountyInvestigation` → `FulfillBountyDirect` chain: agent lacks target location → investigation method → after witness ask, fallback to direct hunt.
- `ProduceWithGather` for a 3-input recipe: prove method substitutes stages.
- Method-disabled fallback: agent with `enabled_methods = {}` for ProduceCommodity falls back to flat GOAP.
- Method failure → `Discrepancy::PartialExecutionDrift` recorded.
- Determinism: same scenario + seed → identical method choices.

## FND-01 Section H Analysis

### Information-Path Analysis

Methods consume agent belief views. Method selection reads `MotiveSourceRef`s (S141 substrate) and `BeliefPredicate` evaluations. No global truth is consulted. Methods can fail because the agent's beliefs are wrong; the resulting `MethodFailureMode` flows through S109 typed discrepancy back into the existing belief/blocker chain.

### Positive-Feedback Analysis

Methods do not introduce new amplifying loops. Method choice depends on existing state (beliefs, motives); method execution produces existing actions; action effects update state through the existing event log.

### Concrete Dampeners

Method timeouts (`MethodFailureMode::Timeout(u32)`) and `planning_budget_hint` cap method-driven search per FND-11. The method itself does not loop unboundedly; subgoals execute through the agenda manager which already handles patience limits.

### Stored State vs. Derived Read-Model List

**Stored state**:
- `AgentSchemaContextProfile.enabled_methods` — universal per-agent field added by S147 on the archived S146 profile.
- `MethodPlanAttemptTrace` on `PlanAttemptTrace` — per-tick trace; not authoritative.

**Derived read-model**:
- Method registry is build-time data.
- Method-selection output is per-tick derivation.

## SystemFn Integration

No new `SystemFn`. Method selection runs inside the existing agent tick's planning phase.

## Component Registration

No new ECS component. `AgentSchemaContextProfile.enabled_methods` is added by S147 to the universal profile registered by archived S146.

## Cross-System Interactions

- Method selector reads `BeliefView` (existing), `MotiveSourceRef`s (S141), and `AgentSchemaContextProfile` (archived S146 profile, extended by S147).
- Method-driven subgoals execute through existing `ActionDef`s.
- Method failure modes flow through S109's typed-discrepancy chain.

All interactions are state-mediated per FND-26.

## Profile-Driven Parameters

- `AgentSchemaContextProfile.enabled_methods` — per-agent method enablement added by S147.
- `MethodSchema.motive_bias[].weight` — `Permille`-bounded.
- `MethodSchema.planning_budget_hint` — optional per-method override of `GoalPlanningBudget` (per S146).

No floats.

## Test Plan

- D10 golden coverage (5 scenarios above).
- D8 registry validation tests.
- Method-selector determinism unit tests.
- Existing goldens unchanged (default scenarios have all methods enabled but flat-GOAP fallback covers them).
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
