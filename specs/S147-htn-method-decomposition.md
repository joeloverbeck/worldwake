# S147: HTN Method Decomposition for Long Lawful Pursuits

**Status**: Draft

## Summary

The external assessment in `reports/ai-architecture-improvements.md` Section 7 (Upgrade D) recommends adding HTN-style methods *above* the existing GOAP planner, applied to goals where naive forward search has prohibitive cost: `FulfillBounty`, `InvestigateViolation`, `Accuse`, `PunishAccused`, `ProduceCommodity`, `RestockCommodity`, `MoveCargo`, `EscortToSafety`, `SearchForMissing`, `ClaimOffice`, `SupportCandidateForOffice`, plus future caravan/patrol/construction/repair/inheritance work. Phase 11 explicitly deferred HTN methods until "S134 (effect schemas), S138 (opportunity compiler), and S141 (motive sources) land" — all archived. The deferral condition is met.

The current two-tier planner already does implicit HTN-like decomposition: `build_stages` in `crates/worldwake-ai/src/search/strategic.rs:324` produces a sequence of `StrategicStage` values whose `kind` is `Goal` or `Acquire(CommodityKind)`. The decomposition is generic: any goal with a missing-commodity assumption emits an acquire stage. This works for `ConsumeOwnedCommodity` (acquire food → consume) and for `ProduceCommodity { recipe_id: "Bake Bread" }` (acquire grain → acquire flour → craft) but expresses the decomposition in code, not data.

S147 lands `MethodSchema` as the data-driven layer above strategic decomposition. A `MethodSchema` is a *lawful pursuit pattern*: "how a guard investigates," "how a hunter fulfills a bounty," "how a merchant restocks," not "what story beat should happen." Every leaf remains an ordinary `ActionDef`. Every artifact remains world state. Methods describe search control and reusable craft knowledge per FND-20.

The first ship covers `FulfillBounty`, `ProduceCommodity`, `RestockCommodity`, `InvestigateViolation`, and `EscortToSafety`. Methods are author-written Rust (build-time data, like S146's `GoalSchema` registry); they live in `crates/worldwake-ai/src/htn/methods.rs`. The planner consults method options before forward search: when a method's preconditions are satisfied, the planner substitutes its subgoals into the strategic itinerary. When no method applies, the planner falls back to flat GOAP as today (no behavioral regression on goals without methods).

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-ai` — owns the `htn` module: `MethodSchema`, `MethodSelector`, the method registry, the planner integration in `search/strategic.rs::build_stages`, the per-trace `MethodPlanAttemptTrace` addition to `PlanAttemptTrace`, the `methods: &'static [MethodSchemaId]` field added to `GoalSchema`, and the inline-defined supporting types (`MethodPrecondition`, `SubgoalTemplate`, `MotiveBias`, `MethodFailureMode`, `BeliefPredicate`, `EntityCriterion`, `RoleTag`, `LocationTemplate`, `TopicTemplate`, `PayloadTemplate`, `ArtifactTemplate`, `ClaimRequirement`, `ExplanationTemplateId`). Reason for ai residence: `SubgoalTemplate::PerformAction(PlannerOpKind, …)` references `PlannerOpKind` at `crates/worldwake-ai/src/planner_ops.rs:13`, and `worldwake-core` cannot depend on `worldwake-ai` per the workspace layering `core → sim → systems → ai → cli`.
- `worldwake-core` — exposes the `MethodSchemaId` newtype (so save/replay payloads can name methods), the two payload-free discriminant mirror enums `MotiveSourceDiscriminant` (mirror of `motive_source.rs:14` `MotiveSource`) and `GoalKindDiscriminant` (mirror of `goal.rs:62` `GoalKind`), the `disabled_methods: BTreeSet<MethodSchemaId>` field added to `AgentSchemaContextProfile` at `agent_schema_context_profile.rs:54`, and the new `Discrepancy::MethodFailure(MethodFailureContext)` variant at `discrepancy.rs:9`.
- `worldwake-sim` — no change.
- `worldwake-systems` — no change.
- `worldwake-cli` — observer Section 8 extends failed plan-attempt rows with method trace detail, and Section 13 surfaces `PlanningMetrics.method_usage`; no scenario-types change is required because `AgentSchemaContextProfile` already has a universal scenario surface and `disabled_methods` defaults to empty.

## Dependencies

- S146 (Goal Schema and Per-Goal Budgets, archived at `archive/specs/S146-goal-schema-and-per-goal-budgets.md`, hard dep) — provides the `GoalSchema` registry substrate (at `crates/worldwake-ai/src/goal_schema.rs:63`) and `AgentSchemaContextProfile` (at `crates/worldwake-core/src/agent_schema_context_profile.rs:54`); this S147 spec adds `GoalSchema.methods` (D11) and `AgentSchemaContextProfile.disabled_methods` (D7).
- S138 (Opportunity Compiler, archived at `archive/specs/S138-opportunity-compiler.md`, hard dep) — `MethodSchema.required_claims` references `Opportunity` matches via the `ClaimRequirement` template type defined in D1.
- S141 (Motive Source Ledger, archived at `archive/specs/S141-motive-source-ledger.md`, hard dep) — method selection consumes `MotiveSourceRef`s (at `crates/worldwake-core/src/motive_source.rs:25`) to bias method choice (Loyalty → group hunt, Revenge → direct hunt). S147 adds the core-side `MotiveSourceDiscriminant` mirror in D12 so `MotiveBias` can key on motive *kind* without committing to a specific `WoundId`/`EntityId` payload.
- S134 (Canonical Effect Schema, archived at `archive/specs/S134-canonical-effect-schema.md`, hard dep) — `MethodSchema.expected_artifacts` references `EffectSchema` post-conditions (at `crates/worldwake-sim/src/effect_schema.rs:9`) for verifying method completion.
- S109 (Typed Discrepancy Taxonomy, archived at `archive/specs/S109-typed-discrepancy-taxonomy.md`, hard dep) — D6 extends the `Discrepancy` enum at `crates/worldwake-core/src/discrepancy.rs:9` with a single `MethodFailure(MethodFailureContext)` variant; `MethodFailureMode` values (D1) project into a `MethodFailureKind` discriminant carried by that context.
- S125 (Institutional Treasuries and Bounty Funding, archived at `archive/specs/S125-institutional-treasuries-and-bounty-funding.md`) — `FulfillBounty` methods (D2) use treasury-backed reward release.
- S111 (Scenario Profile Homogeneity Lints, archived at `archive/specs/S111-scenario-homogeneity-lints.md`) — D7 extends the `ProfileHomogeneity` lint at `crates/worldwake-cli/src/scenario/lints.rs:28` so `disabled_methods` participates in the existing checked profile-variation axes.
- S144 (Aggregate Scenario Diagnostics, archived at `archive/specs/S144-aggregate-scenario-diagnostics.md`) — D5 extends the `PlanningMetrics` struct at `crates/worldwake-ai/src/scenario_diagnostics/mod.rs:32` with a `method_usage` aggregation.
- S115 (Agenda Manager, archived at `archive/specs/S115-agenda-manager.md`) — sub-goals execute through `tick_agenda` and `AgendaState` at `crates/worldwake-ai/src/agenda_manager.rs`; methods do not have internal tick loops.

## Design Goals

1. **Methods are lawful pursuit patterns, not story beats.** Per FND-20: a method describes how an agent pursues a world condition under beliefs; it does not author the outcome.
2. **Methods are data, not behavior.** The registry is build-time `MethodSchema` entries; per-tick computation is a deterministic method selector.
3. **GOAP remains the executor.** Method leaves are ordinary `ActionDef`s; the strategic + tactical search remains the runtime engine.
4. **No silent privilege.** Methods cannot read off-place world state or invoke other systems. They only constrain *which* subgoals get attempted.
5. **Per-agent method enablement via denylist.** Some agents naturally have access to fewer methods than others (a peasant should not invoke `FulfillBountyGroupHunt`). `AgentSchemaContextProfile.disabled_methods` (added by S147) opts agents *out* of specific methods; empty default means all methods are available. The denylist convention mirrors `AgentSchemaContextProfile.disabled_extractors` and aligns with FND-22 (diversity through concrete per-agent variation).
6. **Deterministic method selection.** Same belief state, same disabled-methods set, same motive sources → same method choice. No randomness in method dispatch.

## Non-Goals

- **No method authoring DSL.** Methods are Rust structs; LLM-driven method generation is out of scope (per the assessment's explicit guardrail).
- **No story-beat methods.** "Hero answers the call" or "merchant betrays guard" are not methods; they would violate FND-20.
- **No new top-level reasoning framework.** Methods sit *under* the archived S146 `GoalSchema` registry, not above it.
- **No method-only goals.** Every method-decomposed goal must also have a fallback GOAP path. Disabling all methods for a given goal returns the goal to today's behavior.
- **No method learning.** Methods are author-written. Per-method learning (PR-8 / habit memory) is deferred.
- **No method-internal scheduling.** Sub-goals execute through `tick_agenda` (S115); methods do not have internal tick loops.
- **No new authoritative state on method success.** A successful method run produces only ordinary action effects (event log entries from leaf `ActionDef`s) plus the per-tick `MethodPlanAttemptTrace`. The trace is not authoritative state; the action effects are.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-7 (Locality of Motion, Interaction, Communication) | Method selector reads only the agent's belief view, profile, and motive ledger — no global state. |
| FND-14 (World State Is Not Belief State) | All method preconditions evaluate `BeliefPredicate`s against the agent's belief view; world state is never read by selection. |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | Methods encode reusable affordance composition; they do not encode plot progression. The principle's test — "how this kind of agent pursues this kind of world condition under these beliefs" — is exactly satisfied. |
| FND-22 (Agent Diversity Through Concrete Variation) | `AgentSchemaContextProfile.disabled_methods` allows scenarios to author per-role method access (peasants opt out of `FulfillBountyGroupHunt`, guards opt out of nothing, etc.). Diversity is concrete state, not abstract score. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Methods read belief views and snapshot state; they produce subgoal templates that the planner consumes. No cross-system command. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | Methods extend the planner; the no-method fallback is the actual current path of `build_stages`, not a legacy shim. The new `Discrepancy::MethodFailure` variant lives alongside existing variants; no parallel "method-aware BeliefStale" coexists with non-method `BeliefStale`. |
| FND-29 (Debuggability Is a Product Feature) | Method choice and decomposition are recorded in `PlanAttemptTrace.method_trace`; observer Section 8 surfaces per-attempt details and Section 13 surfaces aggregate method usage. |
| FND-29A (Causal History Is Authoritative, Append-Only) | Method failures attribute through the typed `Discrepancy::MethodFailure` variant (D6), so later state changes are explainable from authoritative state, not from optional trace logs. |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | Section H below covers all 18 declared hooks for the new HTN module and the field extensions. |

## Deliverables

### D1: `MethodSchema` and supporting type surface

All types in this deliverable live in `worldwake-ai`. The HTN module structure:

```
crates/worldwake-ai/src/htn/
├── mod.rs
├── method_schema.rs    (D1 types)
├── selector.rs         (D3)
├── registry.rs         (D8)
└── methods.rs          (D2 first-ship method definitions)
```

```rust
// crates/worldwake-ai/src/htn/method_schema.rs
use worldwake_core::{
    CommodityKind, EntityId, GoalKindDiscriminant, GoalPlanningBudget,
    MethodSchemaId, MotiveSourceDiscriminant, Permille, Quantity, WorkstationTag,
};
use crate::planner_ops::PlannerOpKind;

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
    pub motive_bias: Vec<MotiveBias>,
    pub planning_budget_hint: Option<GoalPlanningBudget>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MethodPrecondition {
    BeliefHolds(BeliefPredicate),
    MotiveSourcePresent(MotiveSourceDiscriminant),
    AgentRole(RoleTag),
    LocationKnown(EntityCriterion),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubgoalTemplate {
    AcquireCommodity { commodity: CommodityTemplate, min_quantity: Quantity },
    TravelTo(LocationTemplate),
    ObserveTarget(EntityCriterion),
    AskWitness(TopicTemplate),
    InspectArtifact(ArtifactTemplate),
    PerformAction(PlannerOpKind, PayloadTemplate),
    ResolveCoordination(ClaimRequirement),
    ReturnTo(LocationTemplate),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MotiveBias {
    pub motive_variant: MotiveSourceDiscriminant,
    pub weight: Permille,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MethodFailureMode {
    PreconditionLost(BeliefPredicate),
    SubgoalUnachievable(usize),         // index into subgoals
    ArtifactNotProduced(ArtifactTemplate),
    ClaimDenied(ClaimRequirement),
    Timeout(u32),                        // ticks
}

// --- Supporting template types (inline-defined for first-ship scope) ---

/// Belief predicates evaluated against the agent's belief view in method
/// preconditions and failure modes. Variants cover only the surfaces the
/// first-ship methods require; new variants are added as methods need them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BeliefPredicate {
    BountyRecordExists { bounty: EntityTemplate },
    BountyExpired { bounty: EntityTemplate },
    TargetLastSeenKnown { target: EntityTemplate },
    WitnessNamesKnown { violation: EntityTemplate },
    InstitutionalRecordBelievedExtant { violation: EntityTemplate },
    ResourceSourceKnown { commodity: CommodityTemplate },
    SellerKnown { commodity: CommodityTemplate },
    OwnedCommodityBelowThreshold { commodity: CommodityTemplate, threshold: Quantity },
    OwnsInputsForRecipe { recipe: RecipeTemplate },
    EscorteeBelievedSafeAt { escortee: EntityTemplate },
    AllyOrBountyOfficeAvailable,
    TargetBelievedDangerous { target: EntityTemplate },
}

/// Identifies a kind of entity by lawful, perceivable properties.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntityCriterion {
    Target(EntityTemplate),
    Workstation(WorkstationTag),
    ResourceSource(CommodityTemplate),
    Seller(CommodityTemplate),
    Witness { topic: TopicTemplate },
    ViolationEvidence { violation: EntityTemplate },
    Ledger { institution: EntityTemplate },
}

/// Coarse role discriminator for method enablement preconditions; mirrors
/// existing institutional role concepts without introducing a new authority
/// surface. Scenarios already author per-role data through agent membership
/// in offices; `RoleTag` here is a read-side classifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RoleTag {
    Hunter,
    Guard,
    Merchant,
    Magistrate,
    Crafter,
    Caravaneer,
    Civilian,
}

/// Names a location relative to the agent's beliefs and the method's context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocationTemplate {
    LastKnownTargetPlace { target: EntityTemplate },
    NearestSellerOf { commodity: CommodityTemplate },
    AgentHome,
    BountyIssuerPlace { bounty: EntityTemplate },
    OfficePlace { institution: EntityTemplate },
    EscorteeHome { escortee: EntityTemplate },
    KnownWorkstationFor { recipe: RecipeTemplate },
    StagingPlaceForConfrontation { target: EntityTemplate },
}

/// Witness-ask topic template.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TopicTemplate {
    TargetWhereabouts { target: EntityTemplate },
    ViolationCircumstances { violation: EntityTemplate },
}

/// Payload synthesis strategy for a templated `PerformAction` subgoal. The
/// planner materializes `Explicit` payloads as-is and resolves `FromContext`
/// payloads from the active method's per-subgoal binding context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PayloadTemplate {
    FromContext,
    Explicit(PayloadValueTemplate),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PayloadValueTemplate {
    Trade { commodity: CommodityTemplate, quantity: Quantity },
    Craft { recipe: RecipeTemplate },
    Attack { target: EntityTemplate },
    ClaimBounty { bounty: EntityTemplate },
    EscortToSafety { escortee: EntityTemplate, destination: LocationTemplate },
}

/// References a class of evidence artifact the method expects to inspect or
/// produce. Each variant resolves through perception against world artifacts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArtifactTemplate {
    ViolationEvidence { violation: EntityTemplate },
    Ledger { institution: EntityTemplate },
    BountyProof { bounty: EntityTemplate, target: EntityTemplate },
}

/// Names a coordination requirement (queue slot, office authority, resource
/// source access). Resolved against `Opportunity` matches from the S138
/// opportunity compiler at runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClaimRequirement {
    OfficeAuthority { office: EntityTemplate },
    ResourceSourceAccess { commodity: CommodityTemplate, place: EntityTemplate },
    BountyIssuance { bounty: EntityTemplate },
    FacilityQueueSlot { facility: EntityTemplate },
}

/// Symbolic entity binding used by build-time methods. The selector/planner
/// resolves these against the runtime goal, belief view, or a concrete fixed
/// entity. This avoids hidden sentinel `EntityId`s in method definitions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityTemplate {
    GoalPrimaryEntity,
    GoalSecondaryEntity,
    GoalPlace,
    BountyTarget,
    Violation,
    Institution,
    Escortee,
    Fixed(EntityId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommodityTemplate {
    GoalCommodity,
    RecipeInput { recipe: RecipeTemplate, ordinal: u8 },
    Fixed(CommodityKind),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecipeTemplate {
    GoalRecipe,
    Fixed(u32),
}

/// Build-time identifier for an explanation template string. The actual
/// rendering table lives in `htn/explanation_templates.rs` and is consumed by
/// the observer (D9). Stable across releases — used in trace serialization.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ExplanationTemplateId(pub u32);
```

`Permille` for all weight values. No floats. Every embedded type derives or composes from `Copy`/`Clone`/`Eq` as the outer `MethodSchema` requires (verified via the `Hash` requirement on `MotiveSourceDiscriminant` and `GoalKindDiscriminant` per D12). Runtime-specific bounty, target, violation, institution, escortee, commodity, and recipe values are represented as explicit template bindings rather than concrete sentinel IDs; selector and planner integration resolve them from the live goal and belief view.

**Note on supporting type scope**: The variant lists above cover the first-ship methods (D2) exhaustively. Future methods may add variants; each addition is a deliverable on the spec that introduces the new method.

### D2: First-ship methods

Each method is shipped as a `MethodSchema` entry in `build_method_registry()` (D8). All `PerformAction` subgoals reference real `PlannerOpKind` variants verified at `crates/worldwake-ai/src/planner_ops.rs:13`.

**`FulfillBounty` methods**:
- `FulfillBountyDirect`: preconditions — `BeliefHolds(BountyRecordExists { bounty })`, `BeliefHolds(TargetLastSeenKnown { target })`. Subgoals — `AcquireCommodity` for weapons/supplies if missing → `TravelTo(LastKnownTargetPlace { target })` → `ObserveTarget(Target(target))` → `PerformAction(Attack, Explicit(Attack { target }))` → `TravelTo(BountyIssuerPlace { bounty })` → `PerformAction(ClaimBounty, Explicit(ClaimBounty { bounty }))`. Failure modes — `PreconditionLost(BountyExpired { bounty })`, `SubgoalUnachievable(<track index>)`.
- `FulfillBountyInvestigation`: preconditions — `BeliefHolds(BountyRecordExists { bounty })` but not `TargetLastSeenKnown`; `BeliefHolds(WitnessNamesKnown { violation })` or `BeliefHolds(InstitutionalRecordBelievedExtant { violation })`. Subgoals — `AskWitness(TargetWhereabouts { target })` or `InspectArtifact(ViolationEvidence { violation })` → re-evaluate. After belief update, the planner re-runs method selection; success of this method does not directly satisfy the bounty goal — instead it expands the agent's information so `FulfillBountyDirect` can subsequently match.
- `FulfillBountyGroupHunt`: preconditions — `BeliefHolds(TargetBelievedDangerous { target })`, `BeliefHolds(AllyOrBountyOfficeAvailable)`. Subgoals — `PerformAction(DeclareSupport, ...)` (recruit signal via the existing `PlannerOpKind::DeclareSupport` variant) → `TravelTo(StagingPlaceForConfrontation { target })` → confront. Motive bias toward `MotiveSourceDiscriminant::Loyalty` (high weight). **Default denylist**: most non-hunter, non-guard roles include `FulfillBountyGroupHunt` in their `disabled_methods` set (see D7 universal-component note).

**`ProduceCommodity` methods**:
- `ProduceFromOwnedStock`: precondition — `BeliefHolds(OwnsInputsForRecipe { recipe: GoalRecipe })`, `LocationKnown(Workstation(...))`, and agent at workstation. Subgoals — `PerformAction(Craft, Explicit(Craft { recipe: GoalRecipe }))`.
- `ProduceWithGather`: precondition — agent missing some inputs but `BeliefHolds(ResourceSourceKnown { commodity: RecipeInput { recipe: GoalRecipe, ordinal: 0 } })` for the first unresolved input. Subgoals — `AcquireCommodity { commodity: RecipeInput { recipe: GoalRecipe, ordinal: 0 }, min_quantity }` → `TravelTo(KnownWorkstationFor { recipe: GoalRecipe })` → `PerformAction(Craft, Explicit(Craft { recipe: GoalRecipe }))`.
- `ProduceWithPurchase`: precondition — agent missing inputs but `BeliefHolds(SellerKnown { commodity: RecipeInput { recipe: GoalRecipe, ordinal: 0 } })`. Subgoals — `TravelTo(NearestSellerOf { commodity: RecipeInput { recipe: GoalRecipe, ordinal: 0 } })` → `PerformAction(Trade, Explicit(Trade { commodity, quantity }))` → `TravelTo(KnownWorkstationFor { recipe: GoalRecipe })` → `PerformAction(Craft, Explicit(Craft { recipe: GoalRecipe }))`.

**`RestockCommodity` methods**:
- `RestockFromHarvest`: precondition — `BeliefHolds(OwnedCommodityBelowThreshold { commodity, threshold })`, `BeliefHolds(ResourceSourceKnown { commodity })`. Subgoals — `AcquireCommodity { commodity, min_quantity }` from source.
- `RestockFromMarket`: precondition — `OwnedCommodityBelowThreshold` plus `SellerKnown { commodity }`. Subgoals — `TravelTo(NearestSellerOf { commodity })` → `PerformAction(Trade, Explicit(Trade { commodity, quantity }))`.

**`InvestigateViolation` methods**:
- `InvestigateOnScene`: precondition — violation believed at known place, agent at place. Subgoals — `InspectArtifact(ViolationEvidence { violation })` → record finding via `PerformAction(Investigate, FromContext)`.
- `InvestigateByWitness`: precondition — `BeliefHolds(WitnessNamesKnown { violation })`. Subgoals — `AskWitness(ViolationCircumstances { violation })` → record finding via `PerformAction(Investigate, FromContext)`.
- `InvestigateByLedger`: precondition — `BeliefHolds(InstitutionalRecordBelievedExtant { violation })`. Subgoals — `TravelTo(OfficePlace { institution })` → `InspectArtifact(Ledger { institution })` → record finding via `PerformAction(Investigate, FromContext)`.

**`EscortToSafety` methods**:
- `EscortToHome`: precondition — `BeliefHolds(EscorteeBelievedSafeAt { escortee })` with `escortee.home`. Subgoals — synchronize at escortee place → `PerformAction(EscortToSafety, Explicit(EscortToSafety { escortee, destination: EscorteeHome { escortee } }))` → confirm arrival via `ObserveTarget(Target(escortee))`.
- `EscortToOffice`: precondition — institutional protection believed available (`OfficeAuthority`). Subgoals — `PerformAction(EscortToSafety, Explicit(EscortToSafety { escortee, destination: OfficePlace { institution } }))`.

### D3: `MethodSelector`

```rust
// crates/worldwake-ai/src/htn/selector.rs
pub fn select_method<'r>(
    actor: EntityId,
    goal: &GoalOffer,
    registry: &'r MethodRegistry,
    profile: &AgentSchemaContextProfile,
    belief_view: &dyn RuntimeBeliefView,
    motives: &[MotiveSourceRef],
) -> Option<&'r MethodSchema>;
```

Deterministic selection:
1. Filter methods to those whose `goal_kind` matches `GoalKindDiscriminant::from(&goal.key.kind)` and whose `id ∉ profile.disabled_methods`.
2. Filter to methods whose `preconditions` evaluate to `true` per `belief_view` for `actor`. Each `MethodPrecondition` variant evaluates against the existing accessors on `RuntimeBeliefView` (defined at `crates/worldwake-sim/src/belief_view.rs:1588`) — no new trait accessor is required because `BeliefPredicate` variants compose existing reads. `actor` is required because every belief read is agent-relative under FND-14.
3. Rank remaining methods by motive-source bias score. Formula (integer, no floats):
   - For each `bias ∈ method.motive_bias`, add `bias.weight.value()` when any present `MotiveSourceRef m` has `MotiveSourceDiscriminant::from(&m.source) == bias.motive_variant`.
   - `MotiveSourceRef` carries source identity and introduction tick, not a per-source weight; source magnitude is already reflected in the upstream ranked `GoalOffer` / `motive_source_contributions` path. Method selection is a within-goal method-bias choice, not a second goal-ranking pass.
4. Tie-break by `MethodSchemaId` (stable integer order).
5. Return the top method or `None` (fall back to flat GOAP).

### D4: Planner integration

`build_stages` at `crates/worldwake-ai/src/search/strategic.rs:324` (called from `strategic.rs:119`) is modified in place rather than introducing a parallel function. The current implementation is the no-method fallback; the new integration prefixes a method-selection branch:

```rust
// crates/worldwake-ai/src/search/strategic.rs
fn build_stages(
    actor: EntityId,
    goal: &GoalOffer,
    profile: &AgentSchemaContextProfile,
    registry: &MethodRegistry,
    belief_view: &dyn RuntimeBeliefView,
    motives: &[MotiveSourceRef],
    /* existing parameters */
) -> Vec<StrategicStage> {
    if let Some(method) = select_method(actor, goal, registry, profile, belief_view, motives) {
        return method.subgoals.iter()
            .flat_map(|template| template_to_stages(template, goal, belief_view))
            .collect();
    }
    // Existing flat-GOAP decomposition (unchanged)
    build_stages_default(goal, /* existing parameters */)
}
```

When a method is chosen, its subgoals expand into `StrategicStage`s via the existing decomposition machinery — `template_to_stages` is a private helper in `search/strategic.rs` that first resolves `EntityTemplate`, `CommodityTemplate`, and `RecipeTemplate` bindings against the live goal, planner-visible beliefs, and recipe registry, then maps each `SubgoalTemplate` to one or more `StrategicStage` values whose `kind` is `Goal` or `Acquire(CommodityKind)`. The strategic search iterates over these stages as today. Tactical search remains unchanged. `PlanningSnapshot`/`PlanningState` carry the actor's `AgentSchemaContextProfile` so the method selector can honor disabled methods without reading authoritative world state.

### D5: `MethodPlanAttemptTrace` and `PlanAttemptTrace.method_trace`

```rust
// crates/worldwake-ai/src/decision_trace.rs (struct extension at line 1185)
pub struct PlanAttemptTrace {
    // ... existing fields ...
    pub method_trace: Option<MethodPlanAttemptTrace>,    // NEW
}

pub struct MethodPlanAttemptTrace {
    pub method_id: Option<MethodSchemaId>,
    pub subgoals_attempted: Vec<SubgoalAttemptResult>,
    pub failure_mode: Option<MethodFailureMode>,
    pub motive_score: u32,                    // 0..=1_000_000 per D3
}

pub struct SubgoalAttemptResult {
    pub template_index: usize,
    pub kind: SubgoalAttemptKind,
    pub outcome: SubgoalAttemptOutcome,
}

pub enum SubgoalAttemptKind { /* one variant per SubgoalTemplate */ }
pub enum SubgoalAttemptOutcome { Pending, Succeeded, Failed }
```

`method_trace` is `Option<…>` so flat-GOAP plan attempts (no method selected) record `None` rather than synthesize a trace. In the first shipped trace surface, selected method subgoals are recorded as `Pending` at selection time; later execution/golden work may assert success or failure only at the layer that observes action lifecycle outcomes.

`MethodPlanAttemptTrace` is also surfaced through `ScenarioDiagnosticsReport.planning.method_usage` — a new `BTreeMap<Option<MethodSchemaId>, MethodUsageCounts>` field added to `PlanningMetrics` at `crates/worldwake-ai/src/scenario_diagnostics/mod.rs:32`. `MethodUsageCounts` records `attempts`, `selected_count`, `fallback_count`, and `failure_count`.

### D6: `Discrepancy::MethodFailure(MethodFailureContext)` variant

Per the "Discrepancy as Failure-Attribution Surface" pattern (option 1), a single new variant is added to the `Discrepancy` enum at `crates/worldwake-core/src/discrepancy.rs:9`:

```rust
// crates/worldwake-core/src/discrepancy.rs
pub enum Discrepancy {
    // ... existing variants (unit, unchanged) ...
    MethodFailure(MethodFailureContext),   // NEW
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct MethodFailureContext {
    pub method_id: MethodSchemaId,
    pub kind: MethodFailureKind,
    pub subgoal_index: Option<u32>,        // index into method.subgoals when applicable
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum MethodFailureKind {
    PreconditionLost,
    SubgoalUnachievable,
    ArtifactNotProduced,
    ClaimDenied,
    Timeout,
}
```

`MethodFailureMode` (ai-side, D1) projects into `MethodFailureKind` (core-side) via a `From<&MethodFailureMode> for MethodFailureKind` impl in `htn/method_schema.rs`. The richer ai-side payload (`BeliefPredicate`, `ArtifactTemplate`, etc.) remains on `MethodPlanAttemptTrace` for observer rendering (D9); the typed-discrepancy variant carries only the `Copy`/`Hash`-safe core-side projection so it composes with existing blocker-memory consumers.

**Workspace exhaustive-match audit (deliverable scope)**: grep `match` sites on `Discrepancy` across all crates and add a new arm to each genuinely-exhaustive match. Per the Discrepancy pattern note, the construction sites (`Err(Discrepancy::X)`) require no change; only destructuring match sites do. Estimated ~10–20 audit sites based on the ~145 total `Discrepancy` use sites being construction-dominant. The audit is a single-pass mechanical change; sites unable to handle method failures route to a `_` arm with a debug log per crate convention.

### D7: `AgentSchemaContextProfile.disabled_methods`

```rust
// crates/worldwake-core/src/agent_schema_context_profile.rs (struct extension at line 54)
pub struct AgentSchemaContextProfile {
    pub disabled_extractors: BTreeSet<CandidateExtractorId>,
    pub budget_overrides: BTreeMap<GoalDispatchKey, GoalPlanningBudget>,
    #[serde(default)]
    pub disabled_methods: BTreeSet<MethodSchemaId>,    // NEW
}
```

**Semantics**: `disabled_methods` is a **denylist**, matching the existing `disabled_extractors` field. Empty default means every method in the registry is available for the agent. Scenarios opt agents *out* of specific methods (e.g., a peasant scenario sets `disabled_methods = { FulfillBountyGroupHunt, FulfillBountyDirect, FulfillBountyInvestigation }` to model lack of bounty pursuit). The denylist convention aligns with FND-22 (diversity through concrete per-agent variation) and avoids the design contradiction where an allowlist default of "all enabled" would conflict with role-specific methods being default-available.

**Universal classification**: `AgentSchemaContextProfile` is already a universal-per-agent component registered in `worldwake-core` per archived S146 (no `*Def` wrapper; surfaced directly in `crates/worldwake-cli/src/scenario/types.rs:595` as an optional scenario field with `Default` impl). `disabled_methods` inherits this classification — the `#[serde(default)]` attribute ensures existing scenario RON deserialization continues to work without modification.

**S111 lint extension**: `ProfileHomogeneity` at `crates/worldwake-cli/src/scenario/lints.rs:28` extends its checked variation axes to include `AgentSchemaContextProfile.disabled_methods` alongside the existing S147 schema-context fields. The lint keeps the S111 rule shape: AI populations with more than two agents fail only when no checked profile axis varies, and the failure detail names `agent_schema_context_profile.disabled_methods` so method-denylist homogeneity is inspectable.

### D8: Method registry build-time table + validation tests

```rust
// crates/worldwake-ai/src/htn/registry.rs
pub fn build_method_registry() -> MethodRegistry {
    let mut registry = MethodRegistry::default();
    registry.insert(methods::fulfill_bounty_direct());
    registry.insert(methods::fulfill_bounty_investigation());
    registry.insert(methods::fulfill_bounty_group_hunt());
    registry.insert(methods::produce_from_owned_stock());
    registry.insert(methods::produce_with_gather());
    registry.insert(methods::produce_with_purchase());
    registry.insert(methods::restock_from_harvest());
    registry.insert(methods::restock_from_market());
    registry.insert(methods::investigate_on_scene());
    registry.insert(methods::investigate_by_witness());
    registry.insert(methods::investigate_by_ledger());
    registry.insert(methods::escort_to_home());
    registry.insert(methods::escort_to_office());
    registry
}
```

**Validation tests** (in `crates/worldwake-ai/tests/htn_registry_validation.rs`):

1. **Goal-kind reachability**: every method's `goal_kind` resolves to a real `GoalKindDiscriminant` (i.e., the underlying `GoalKind` variant exists). Iterates `GoalKindDiscriminant::ALL` constant.
2. **Action reachability**: every `SubgoalTemplate::PerformAction(op, _)` references a real `PlannerOpKind` variant (compile-time guaranteed by the type system, but the test verifies via `op` matching `PlannerOpKind::ALL`-style iteration to catch enum extensions that drop variants).
3. **MethodSchemaId uniqueness**: no two entries share an id.
4. **Per-method failure-mode coverage**: every method declares at least one entry in `failure_modes`.
5. **Motive bias bounds**: every `MotiveBias.weight` is `Permille::clamped()` (in 0..=1000); no underflow/overflow.

### D9: Observer Section 7 method rendering

For failed plan attempts, observer Section 8 prints a Method column and method-trace detail lines:

```
Plan attempt: ProduceCommodity{recipe="Bake Bread"} (Method: ProduceWithGather)
  Subgoal 1: AcquireCommodity(Grain, ≥3) ✓
  Subgoal 2: AcquireCommodity(Flour, ≥2) ✓
  Subgoal 3: TravelTo(KnownWorkstationFor{recipe="Bake Bread"}) ✓
  Subgoal 4: PerformAction(Craft, Bake Bread) — Pending

Plan attempt: ConsumeOwnedCommodity{commodity=Bread} (Method: <none — flat GOAP fallback>)
  (no method-decomposed trace; see strategic-stage trace above)
```

Failed method attempts surface the `MethodFailureMode` with structured prose, e.g.:

```
Plan attempt: FulfillBounty{bounty=#42} (Method: FulfillBountyDirect)
  Subgoal 1: TravelTo(LastKnownTargetPlace{target=#7}) ✓
  Subgoal 2: ObserveTarget(Target{target=#7}) ✗
  Failure: SubgoalUnachievable(index=1) — Discrepancy::MethodFailure(SubgoalUnachievable)
```

The format follows the existing observer conventions: compact table columns for scanability, indented method detail lines, and explicit `Pending`/`Succeeded`/`Failed` subgoal status labels.

### D10: Golden coverage

`golden_htn_methods.rs` (under `crates/worldwake-ai/tests/`) is staged:
- Landed first seam (`archive/tickets/S147HTNMETDEC-011.md`): `ProduceWithGather` selector proof from the actor belief view plus `GoalOffer` evidence places; method-disabled `ProduceCommodity` fallback to flat GOAP; deterministic replay for both observations.
- Active remainder (`tickets/S147HTNMETDEC-013.md`): autonomous generated-candidate method trace propagation, then the remaining `FulfillBountyDirect`, `FulfillBountyInvestigation`, escort/failure, and typed `Discrepancy::MethodFailure(MethodFailureContext)` goldens once the evidence bridge is truthful.
- Determinism remains required for each landed scenario via repeated fixed-seed observations asserted against `MethodPlanAttemptTrace` and decision-trace output.

### D11: `GoalSchema.methods` field

```rust
// crates/worldwake-ai/src/goal_schema.rs (struct extension at line 63)
pub struct GoalSchema {
    // ... existing fields ...
    pub methods: &'static [MethodSchemaId],    // NEW: ordered list of methods this goal may decompose through
}
```

The field is const-populated on each `static GoalSchema` declaration. Method order in the slice is the author-controlled tie-break order before D3 motive bias. Current declarations may use empty `&[]` anchors until the method registry deliverable installs real method IDs. The field is a slice (not `Vec`) so the existing const-static declaration pattern remains intact while preserving author-controlled ordering.

A test in `crates/worldwake-ai/tests/goal_schema_methods.rs` asserts that a populated slice preserves iteration order and that current static declarations expose the field. Registry-resolution tests belong to the method registry deliverable that installs method IDs.

### D12: Core-side discriminant mirrors

```rust
// crates/worldwake-core/src/motive_source.rs (extension)

/// Payload-free mirror of MotiveSource for keying biases and aggregations
/// without committing to a specific WoundId/EntityId/OpportunityKey payload.
/// Variants are 1:1 with MotiveSource per the Core-Side Mirror Enum precedent
/// (BeliefStatusTag).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum MotiveSourceDiscriminant {
    NeedPressure,
    Pain,
    OfficeDuty,
    Loyalty,
    Greed,
    Shame,
    Revenge,
}

impl From<&MotiveSource> for MotiveSourceDiscriminant {
    fn from(source: &MotiveSource) -> Self {
        match source {
            MotiveSource::NeedPressure { .. } => MotiveSourceDiscriminant::NeedPressure,
            MotiveSource::Pain { .. }         => MotiveSourceDiscriminant::Pain,
            MotiveSource::OfficeDuty { .. }   => MotiveSourceDiscriminant::OfficeDuty,
            MotiveSource::Loyalty { .. }      => MotiveSourceDiscriminant::Loyalty,
            MotiveSource::Greed { .. }        => MotiveSourceDiscriminant::Greed,
            MotiveSource::Shame { .. }        => MotiveSourceDiscriminant::Shame,
            MotiveSource::Revenge { .. }      => MotiveSourceDiscriminant::Revenge,
        }
    }
}

impl MotiveSource {
    pub fn discriminant(&self) -> MotiveSourceDiscriminant { self.into() }
}
```

```rust
// crates/worldwake-core/src/goal.rs (extension)

/// Payload-free mirror of GoalKind. Variants are 1:1 with GoalKind (~30 variants
/// — see GoalKind at line 62). Used by HTN method dispatch keys and by any
/// aggregation that must key on goal kind without payload (e.g., per-kind
/// method-usage counts in D5).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum GoalKindDiscriminant {
    ConsumeOwnedCommodity, AcquireCommodity, Sleep, Relieve, Wash,
    FreeCarryCapacity, EngageHostile, RaidTarget, ReduceDanger,
    RegroupWithFaction, EstablishBanditCamp, TreatWounds, SearchForMissing,
    ReportMissing, ReportFound, EscortToSafety, ProduceCommodity,
    SellCommodity, RestockCommodity, MoveCargo, LootCorpse, BuryCorpse,
    FulfillBounty, PostBounty, PostNotice, ShareBelief, AskWitness,
    ClaimOffice, SupportCandidateForOffice, InvestigateViolation, Patrol,
    ExploreLocation, StealItem, Accuse, PunishAccused,
}

impl GoalKindDiscriminant {
    pub const ALL: &[GoalKindDiscriminant] = &[ /* exhaustive enumeration */ ];
}

impl From<&GoalKind> for GoalKindDiscriminant {
    fn from(kind: &GoalKind) -> Self { /* exhaustive match */ }
}

impl GoalKind {
    pub fn discriminant(&self) -> GoalKindDiscriminant { self.into() }
}
```

**Note on mirror maintenance**: both mirrors are 1:1 with their source enums. A unit test in `crates/worldwake-core/tests/discriminant_mirrors.rs` enumerates every source variant and asserts the discriminant projection round-trips through a sample-construction fixture, catching any source-enum extension that forgets to add the corresponding mirror variant.

## FND-01 Section H Analysis

### 1. Motivating downstream consequence

Existing flat-GOAP search hits prohibitive cost on goals with multi-step lawful pursuits (FulfillBounty, ProduceCommodity with crafting chains, InvestigateViolation across multiple evidence types). Without method decomposition, these goals either (a) exhaust the planner's per-goal budget and never plan, or (b) require the budget to be set high enough that single-tick search dominates ai-tick cost. Existing systems cannot already produce this because the implicit decomposition in `build_stages` is fixed and generic (any goal → acquire-then-act); only data-driven methods can encode the FulfillBountyGroupHunt vs. FulfillBountyDirect choice.

### 2. New entities, relations, records

- `MethodSchema` registry (build-time data in ai).
- `MethodSchemaId` newtype (core).
- Two core-side discriminant mirrors (`MotiveSourceDiscriminant`, `GoalKindDiscriminant`).
- One new `Discrepancy::MethodFailure(MethodFailureContext)` variant (core).
- One new field on `AgentSchemaContextProfile.disabled_methods` (core).
- One new field on `GoalSchema.methods` (ai).
- One new field on `PlanAttemptTrace.method_trace` (ai).
- One new aggregation surface on `PlanningMetrics.method_usage` (ai/scenario_diagnostics).

### 3. Actions / world processes that mutate them

No new actions. Existing actions are method *leaves*; `PlannerOpKind` variants are referenced by `SubgoalTemplate::PerformAction`. The method registry is build-time and never mutated at runtime. `MethodPlanAttemptTrace` is written by the selector and read by the observer; it is per-tick derivation, not authoritative state.

### 4. Information production / propagation / observability

Method choice is observable through `PlanAttemptTrace.method_trace` (D5). The trace is per-agent per-tick and exposed through observer Section 8 failed-plan details plus Section 13 aggregate method usage (D9). Method failures are observable through `Discrepancy::MethodFailure` (D6) on the typed-discrepancy channel, which is already consumed by existing blocker-memory and learning systems (S109 chain).

### 5. Conserved quantities / source/sink paths

None — methods do not introduce conserved quantities. Action leaves continue to conserve through the existing `verify_conservation` invariant.

### 6. Scarce capacities / reservations / contention

None at the method level. `MethodSchema.required_claims` references existing `Opportunity` matches (from S138's opportunity compiler), which already encode reservation/queue semantics. Methods do not create new contention surfaces.

### 7. Partial failures, degraded states, aftermath

`MethodFailureMode` (D1) and the typed `Discrepancy::MethodFailure(MethodFailureContext)` (D6) jointly cover this. Failed method attempts leave (a) the typed discrepancy on the planner trace (authoritative), (b) the richer `MethodPlanAttemptTrace` (derived), and (c) any partial action effects from leaves that did complete (authoritative event log entries). Recovery is handled by the existing `handle_plan_failure` chain. The AGENTS.md authoritative-to-AI impact rule does not trigger because method selection is upstream of action validation.

### 8. Positive feedback loops amplified

None. Method choice depends on existing state (beliefs, motives, profile); method execution produces existing action effects.

### 9. Physical dampeners

- `MethodFailureMode::Timeout(u32)` per method instance.
- `MethodSchema.planning_budget_hint: Option<GoalPlanningBudget>` overrides the parent goal's budget for method-driven search.
- `tick_agenda` (S115) caps method-driven subgoal pursuit through existing patience limits.

### 10. Agent-local learning / habit / trust updates

None in this spec. Future PR-8 / habit memory may add method preference reinforcement; that is explicitly out of scope per Non-Goals.

### 11. How agents become wrong, how they correct

Methods can be selected on stale beliefs (e.g., `TargetLastSeenKnown` is stale → `FulfillBountyDirect` selected → target absent at last-known place → `MethodFailureMode::SubgoalUnachievable`). Correction flows through:
1. Subgoal observation returns mismatch.
2. `Discrepancy::MethodFailure` posted on the typed channel.
3. Existing belief-correction path (S109) updates the relevant `BeliefPredicate` source state.
4. Next-tick method selection re-evaluates preconditions with updated belief state.

Provenance: `MethodFailureContext.method_id` + `subgoal_index` identify which method-substrate produced the discrepancy, enabling debugging of method-attribution chains.

### 12. Lifecycle states / transitions / visibility

`MethodSchema` is build-time data; it has no lifecycle. `MethodPlanAttemptTrace` is per-tick and ephemeral.

### 13. Temporal / spatial resolution / scheduling

Method selection runs inside the existing per-agent planning phase of each tick. No new scheduling. Determinism per Design Goal #6.

### 14. Boundary conditions / external drivers

None — methods are purely internal to ai decision-making.

### 15. Derived views / caches / optimizations

`MethodPlanAttemptTrace` is a per-tick derived view of the selection process. `PlanningMetrics.method_usage` is a scenario-aggregate derived view over per-tick traces. Both are caches per FND-27 and never become source of truth.

### 16. Causal records / event identities / provenance links

`Discrepancy::MethodFailure(MethodFailureContext)` carries `MethodSchemaId` + `MethodFailureKind` + `subgoal_index`, sufficient to reconstruct which method substrate produced the failure. `PlanAttemptTrace.method_trace` carries the full `MethodPlanAttemptTrace` with subgoal-attempt details for debugging.

### 17. Target patterns / regression cases / falsification checks

D10 golden coverage provides scenario-level regression. D8 validation tests provide registry-level invariants. The `discriminant_mirrors.rs` test in D12 provides type-level invariant on mirror completeness.

### 18. Save/load / replay / offscreen compression

`MethodSchema` is build-time and not serialized. `Discrepancy::MethodFailure(MethodFailureContext)` derives `Serialize, Deserialize` (per D6), and `MethodFailureContext` carries only `Copy`-safe core-side types so it round-trips through the existing save/load and replay paths. The `disabled_methods` field on `AgentSchemaContextProfile` has `#[serde(default)]` (per D7), so existing serialized scenarios deserialize unchanged. `PlanAttemptTrace.method_trace` is on the in-memory decision trace surface, not the save-format surface; scenario diagnostics expose method usage through the serde-derived `PlanningMetrics.method_usage` field.

## SystemFn Integration

No new `SystemFn`. Method selection runs inside the existing agent tick's planning phase, specifically inside the modified `build_stages` (D4).

## Component Registration

No new ECS component. Two component-bearing structs are extended:

1. `AgentSchemaContextProfile` (registered on `EntityKind::Agent` per archived S146): gains `disabled_methods: BTreeSet<MethodSchemaId>` with `#[serde(default)]`. Universal classification, no `*Def` wrapper needed.
2. `GoalSchema` (registry struct, not an ECS component): gains `methods: &'static [MethodSchemaId]`. Const-static declaration only.

## Cross-System Interactions

- Method selector reads `BeliefView` for the selected actor (existing trait at `crates/worldwake-sim/src/belief_view.rs:1588`), `MotiveSourceRef`s (from S141 ledger at `crates/worldwake-core/src/motive_source.rs:25`), and `AgentSchemaContextProfile` (archived S146 profile, extended by D7).
- Method-driven subgoals execute through existing `ActionDef`s registered in `ActionDefRegistry`.
- Method failures flow through `Discrepancy::MethodFailure` (D6) — typed-discrepancy chain consumed by existing blocker-memory and learning systems (S109).
- Aggregated method usage flows through `PlanningMetrics.method_usage` (D5) — observer / diagnostics consumed surface.

All interactions are state-mediated per FND-26. No system imperatively invokes another.

## Profile-Driven Parameters

- `AgentSchemaContextProfile.disabled_methods: BTreeSet<MethodSchemaId>` — per-agent denylist of methods (D7).
- `MethodSchema.motive_bias[].weight: Permille` — per-method bias weights (D1).
- `MethodSchema.planning_budget_hint: Option<GoalPlanningBudget>` — optional per-method override of the parent goal's budget (D1).

No floats.

## Test Plan

- D10 golden coverage: first selector/fallback seam landed in `archive/tickets/S147HTNMETDEC-011.md`; autonomous method trace propagation and remaining full-D10 narratives are owned by `tickets/S147HTNMETDEC-013.md`.
- D8 registry validation tests (5 invariants).
- D11 `GoalSchema.methods` resolution tests.
- D12 discriminant-mirror completeness tests.
- Method-selector determinism unit tests.
- D6 workspace exhaustive-match audit — `cargo build --workspace` proves all match sites resolved; per-arm content tested by failure-mode goldens.
- Existing goldens unchanged (default scenarios have empty `disabled_methods` and the flat-GOAP fallback covers all current behavior).
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
