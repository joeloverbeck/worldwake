# S114: Plan Step Guards and Expectation Monitoring

## Summary

Annotate each `PlannedStep` with explicit *guards* (required-believed-facts, minimum-confidence thresholds, invalidators) and *expectations* (expected observations the step should produce). Revalidation reads guards to classify drift as irrelevant, repairable, plan-invalidating, or goal-changing — not just "affordance still matches." Expectations persist across tick boundaries by reusing the existing `ExpectationStore` / `ExpectationRecord` infrastructure (extended with a new `ExpectationBasis::PlanStepCompletion` variant); sim's existing `check_overdue_expectations` SystemFn still performs the generic `Active -> Overdue` transition, and the AI-side D6 tick step emits `EventTag::ExpectationMismatch` when plan-step records become overdue. Guard templates and expectation templates live on `ActionDef` as serializable declarative specs; closures are never stored. Scoped to four core guard/expectation kinds (immediate / state / informed / regression); danger-spike, counterparty-unwilling, resource-partial, partial-execution-drift kinds are deferred.

## Phase and Status

Phase 9: Belief-First Continual Planning Structural. Status: Draft.

## Crates

- `worldwake-core` — `ExpectationKind` + `ExpectationKindTag`, `StatePredicate`, `ObservationPredicate` types; new `ExpectationBasis::PlanStepCompletion { step_index, kind_tag }` variant; widen `ExpectationMismatchPayload` per FND-28.
- `worldwake-ai` — `PlanGuard`, `PlanExpectation` runtime-only types attached to `PlannedStep`; accessor methods on `PlannedStep`; guard/expectation construction at plan-build time; revalidation upgrade in `plan_revalidation.rs`; plan-adoption writes `ExpectationRecord`s into the agent's `ExpectationStore`; new AI-side tick step (D6) reads `PlanStepCompletion`-basis `Overdue` records, emits `EventTag::ExpectationMismatch`, and routes through `classify_discrepancy`.
- `worldwake-sim` — `ActionDef` gains `guard_template: Option<GuardTemplateSpec>` and `expectation_template: Vec<ExpectationTemplateSpec>` fields (declarative data; serializable). No new SystemFn.
- `worldwake-systems` — existing `check_overdue_expectations` in `expectation_check.rs` gains only the mechanical changes required by the new `ExpectationBasis` variant (exhaustive-match arms if any); it does **not** reach into AI-crate types or emit plan-step-specific events. Plan-step interpretation lives in worldwake-ai per D6.

## Dependencies

- S109 (Typed Discrepancy Taxonomy) — **landed** at `archive/specs/S109-typed-discrepancy-taxonomy.md`. Reuses `Discrepancy::{BeliefStale, BeliefContradicted, MissingObservation, PartialExecutionDrift}` plus `DiscrepancyMemory` / `BlockerMemory`.
- S110 (Decision History Events) — **landed** at `archive/specs/S110-decision-history-events.md`. Reuses `EventTag::ExpectationMismatch`, `EventTag::BlockerRecorded`, `PlanInvalidationReason::ExpectationMismatch`, and widens the pre-declared `ExpectationMismatchPayload` in place (FND-28 — see D7).
- S113 (Belief Envelope) — **landed** at `archive/specs/S113-belief-envelope.md`. Guards read `BeliefValue::{value, confidence, status}` through envelope accessors (`believed_target_location`, `believed_commodity_stock`, `believed_entities_at`).

## Design Goals

- Revalidation classifies drift precisely: a merchant's restock is irrelevant to a `TravelTo(destination)` step; it invalidates a `Purchase(merchant)` step only if a guard referenced that merchant's stock.
- Expectation monitoring catches the "agent arrived, target gone" failure *before* the action starts, not at handler time. The envelope-driven `BeliefContradicted` / `BeliefStale` pipeline (S113) replaces ad-hoc `BlockingFact::AssumptionFailed` traps.
- Scoped guard kinds. Four core kinds cover ~80% of current failure pathways; the remaining kinds land when Phase 10 scenarios surface them.
- Reuse the existing expectation infrastructure. The `ExpectationStore`/`ExpectationRecord`/`ExpectationCheck` trio already ships as of earlier phases; S114 extends that surface rather than standing up a parallel one.
- Declarative, serializable action authoring. Guard and expectation templates are data types on `ActionDef`, not closures, so `ActionDef` continues to round-trip through save/load.

## Non-Goals

- Full PolicyPlan branching. Guards and expectations are the substrate branches will read, but this spec keeps plans linear.
- Automatic repair on guard failure. S114 records the mismatch and returns an invalidation; the existing `handle_plan_failure` path decides whether to replan.
- Step-level contract generation by the planner from first principles. Guards and expectations are authored per-action-kind (in each action's registration) plus narrow planner-side augmentations.
- A new `ActivePlan` ECS component or `step_expectation_state` field. Runtime plan state lives on `AgentDecisionRuntime::current_plan` (runtime-only, `crates/worldwake-ai/src/decision_runtime.rs:152`); persistent per-step expectations are `ExpectationRecord` entries in the agent's existing `ExpectationStore`.
- A new `expectation_monitor_system` SystemFn or a new `PlanInvalidationReason::GuardBreach` variant. `SystemId::ExpectationCheck` (`crates/worldwake-sim/src/system_manifest.rs:121`) is the authoritative tick-phase hook, and `PlanInvalidationReason::ExpectationMismatch { step_index }` is the authoritative invalidation variant.
- Confidence-bearing route-belief envelopes. `RequiredFact::RouteKnown` uses the existing `RuntimeBeliefView::route_exists(from, to)` public-topology seam for boolean reachability. This spec does not add a separate confidence/status envelope for route knowledge; if future planning work needs stale/disputed route beliefs rather than boolean route existence, that narrower extension should be scoped then.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-16 (Uncertainty / Contradiction First-Class) | Guards read `BeliefValue::confidence`. A step that requires `confidence ≥ 700` fails fast when the underlying belief decayed. Guard breaches classify through S109's typed discrepancies. |
| FND-17 (Surprise Comes From Violated Expectation) | Expectations encode what the agent *expected* to observe. `EventTag::ExpectationMismatch` (S110) is the authoritative surprise signal. |
| FND-20 (Resource-Bounded Practical Reasoning) | Revalidation short-circuits on guard failure before running affordance matching. Irrelevant drift is ignored cheaply. |
| FND-26 (Systems Interact Through State) | Guard checks and expectation monitoring read authoritative state (belief store, blocker memory, event log) and the existing `ExpectationStore` component — never a direct cross-system call. |
| FND-28 (No Backward Compatibility) | `ExpectationMismatchPayload` is widened in place per S110's pre-declaration. Old event-log decoding fails — no shim. |
| FND-29A (Causal History) | Expectation mismatches are recorded events (S110), preserving the "what did the agent expect that didn't happen?" audit trail. |

## Deliverables

### D1: `PlanGuard`, `PlanExpectation`, and `ExpectationKind` types

**Core-side (`crates/worldwake-core`)** — serializable, Copy-safe where viable:

```rust
/// Tag form of `ExpectationKind` that persists inside `ExpectationBasis` —
/// intentionally payload-free so `ExpectationBasis` and `ExpectationRecord`
/// retain their current `Copy` derives.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ExpectationKindTag {
    Immediate,
    State,
    Informed,
    Regression,
}
```

**AI-crate side (`crates/worldwake-ai`)** — runtime-only, attached to `PlannedStep`. No `Copy` requirement because `PlannedStep` itself is not `Copy`:

```rust
pub struct PlanGuard {
    pub required_facts: Vec<RequiredFact>,
    pub min_confidence: Permille,
    pub invalidators: Vec<Invalidator>,
}

pub enum RequiredFact {
    TargetPresent { target: EntityId, at_place: EntityId },
    CommodityAvailable { place: EntityId, kind: CommodityKind, min_quantity: Quantity },
    /// Boolean route reachability already exists on `RuntimeBeliefView`
    /// through `route_exists(from, to)`, which exposes public topology
    /// without granting remote entity omniscience. This variant does not
    /// require a separate S113-style belief envelope.
    RouteKnown { from: EntityId, to: EntityId },
    ResourceAccess { resource: EntityId, agent_holds_permission: bool },
}

pub enum Invalidator {
    /// Belief about a specific claim drops below min_confidence or is
    /// contradicted. `claim: BeliefClaimKey` is the S109 key.
    BeliefStatusChange { claim: BeliefClaimKey },
    /// Target entity moved away from `at_place`.
    TargetMoved { target: EntityId },
    /// Commodity stock at `place` drops below `min_quantity`.
    CommodityDepleted { place: EntityId, kind: CommodityKind },
    /// Blocker memory records a new suppressive entry matching
    /// (goal_key, place, target) after `baseline_tick`. The baseline is
    /// captured at plan-adoption time; the invalidator fires if any
    /// `BlockerMemory` entry with `observed_tick > baseline_tick` matches.
    NewBlockerRecorded { baseline_tick: Tick },
}

pub struct PlanExpectation {
    pub kind: ExpectationKind,
    pub observe_by: Option<Tick>, // absolute tick; None = by step completion
}

pub enum ExpectationKind {
    /// Immediate: the step's own completion handler emits an expected event.
    Immediate { event_tag: EventTag },
    /// State: the step leaves the world in a specified state (quantity
    /// increased, entity transferred, claim established).
    State { predicate: StatePredicate },
    /// Informed: the agent perceives an expected evidence artifact or
    /// observation after the step.
    Informed { observation: ObservationPredicate },
    /// Regression: a prior-step side effect should still hold (e.g., the
    /// tool we picked up earlier is still in inventory).
    Regression { predicate: StatePredicate },
}

pub enum StatePredicate {
    CommodityAtPlaceAtLeast { place: EntityId, kind: CommodityKind, quantity: Quantity },
    EntityAtPlace { entity: EntityId, place: EntityId },
    ActorHoldsCommodity { kind: CommodityKind, min_quantity: Quantity },
    ClaimEstablished { claim: BeliefClaimKey },
}

pub enum ObservationPredicate {
    EntityPerceivedAtPlace { entity: EntityId, place: EntityId },
    EvidencePerceived { kind: EvidenceKind, place: EntityId },
}
```

`StatePredicate` and `ObservationPredicate` live in `worldwake-core` (co-located with `ExpectationKindTag`) because the richer `ExpectationKind` — which carries them — sits on the runtime `PlannedStep`, but `StatePredicate` is also referenced from the widened `ExpectationMismatchPayload` (D7) and must therefore be core-owned and serializable. All fields are `EntityId` / `CommodityKind` / `Quantity` / `BeliefClaimKey` — Copy primitives.

**Derive propagation**: `ExpectationKind` is *not* `Copy` (contains enum payloads that own no variable-length data, but future predicate extensions may). `PlanGuard` and `PlanExpectation` are `Clone + Debug + Eq + PartialEq` only — they never enter the save-load path because `PlannedStep` is runtime-only.

### D2: `PlannedStep` extension and accessor methods

Extend `PlannedStep` in `crates/worldwake-ai/src/planner_ops.rs:814` with:

```rust
pub struct PlannedStep {
    // ... existing 7 fields unchanged ...
    pub guard: Option<PlanGuard>,
    pub expectations: Vec<PlanExpectation>,
}
```

`expected_materializations: Vec<ExpectedMaterialization>` stays as-is; conceptually it is a subset of `expectations` expressed as `ExpectationKind::State { predicate: CommodityAtPlaceAtLeast | ... }`, but the two carry different information (`ExpectedMaterialization` binds `HypotheticalEntityId`, `PlanExpectation` is a plain predicate). S114 keeps both; future work may unify.

Add an `impl PlannedStep` block with accessors used by guard/expectation construction at plan-build time:

```rust
impl PlannedStep {
    pub fn primary_target(&self) -> Option<EntityId> {
        self.targets.first().and_then(PlanningEntityRef::entity)
    }
    pub fn target_place(&self) -> Option<EntityId> { /* ... */ }
    pub fn target_claim(&self) -> Option<BeliefClaimKey> { /* ... */ }
    pub fn expected_complete_tick(&self, start_tick: Tick) -> Tick {
        Tick(start_tick.0.saturating_add(self.estimated_ticks as u64))
    }
}
```

Each returns `Option<_>` where the field may be absent (untargeted actions, planner-synthesized payloads). Guard/expectation construction (D3) handles `None` by omitting the corresponding `RequiredFact`/`Invalidator`.

### D3: Declarative guard / expectation authoring on `ActionDef`

Each `ActionDef` registration gains two optional declarative specs (serializable data, not closures):

```rust
// In crates/worldwake-sim/src/action_def.rs, alongside the existing 17 fields:
pub struct ActionDef {
    // ... existing fields ...
    pub guard_template: Option<GuardTemplateSpec>,
    pub expectation_template: Vec<ExpectationTemplateSpec>,
}

pub struct GuardTemplateSpec {
    pub required_facts: Vec<RequiredFactSpec>,
    pub min_confidence: Permille,
    pub invalidators: Vec<InvalidatorSpec>,
}

pub enum RequiredFactSpec {
    TargetPresent,                     // bind from step.primary_target() + step.target_place()
    CommodityAvailable { min_quantity: Quantity }, // bind place+kind from payload
    RouteKnown,                        // bind from step.targets
    ResourceAccess,                    // bind resource from step.primary_target()
}

pub enum InvalidatorSpec {
    TargetMoved,                       // bind target
    BeliefStatusChange,                // bind claim from step.target_claim()
    CommodityDepleted { min_quantity: Quantity },
    NewBlockerRecorded,                // baseline_tick bound at plan-adoption
}

pub struct ExpectationTemplateSpec {
    pub kind_tag: ExpectationKindTag,
    pub observe_by_offset: Option<u32>, // None = by step completion
    pub event_tag: Option<EventTag>,    // required when kind_tag == Immediate
    pub state_predicate_spec: Option<StatePredicateSpec>,  // required when kind_tag ∈ {State, Regression}
    pub observation_predicate_spec: Option<ObservationPredicateSpec>, // required when kind_tag == Informed
}
// StatePredicateSpec / ObservationPredicateSpec mirror their core predicate
// enums but carry binding-source tags instead of resolved EntityIds, e.g.
// `CommodityAtPlaceAtLeast { place_source: PlaceSource::StepTargetPlace, kind_source: KindSource::PayloadCommodity, quantity_source: QuantitySource::Literal(Quantity) }`.
```

The AI crate owns a pure function:

```rust
// crates/worldwake-ai/src/plan_guard_build.rs
pub fn build_plan_guard(
    def: &ActionDef,
    step: &PlannedStep,
    adoption_tick: Tick,
) -> Option<PlanGuard>;

pub fn build_plan_expectations(
    def: &ActionDef,
    step: &PlannedStep,
    adoption_tick: Tick,
) -> Vec<PlanExpectation>;
```

The functions translate `GuardTemplateSpec` / `ExpectationTemplateSpec` into concrete `PlanGuard` / `Vec<PlanExpectation>` by resolving binding-source tags against `step.primary_target()`, `step.target_place()`, `step.target_claim()`, and any payload override. This preserves `ActionDef`'s `Serialize + Deserialize` derives — closures are never stored.

### D4: Persist plan-step expectations through `ExpectationStore`

Extend `ExpectationBasis` in `crates/worldwake-core/src/expectation.rs:22` with a new `Copy`-safe variant (preserves the existing `Copy` derive on both `ExpectationBasis` and `ExpectationRecord`):

```rust
pub enum ExpectationBasis {
    DutyAssignment { office: EntityId },
    DeliveryCommitment { commodity: CommodityKind, quantity: Quantity },
    RoutineReturn,
    EscortObligation { charge: EntityId },
    SocialPromise,
    /// NEW: a plan step expects completion by `deadline_tick`. The rich
    /// `PlanExpectation` (with its `StatePredicate` / `ObservationPredicate`)
    /// lives on the runtime `PlannedStep`; the monitor cross-references by
    /// `(step_index, kind_tag)` against the agent's current plan.
    PlanStepCompletion { step_index: u16, kind_tag: ExpectationKindTag },
}
```

At plan adoption, the AI crate writes one `ExpectationRecord` per `PlanExpectation` with:

- `owner` = the acting agent
- `subject` = the step's primary target (or the agent itself if untargeted)
- `expected_place` = the step's target place (or the agent's current place)
- `deadline_tick` = `step.expected_complete_tick(adoption_tick)` adjusted by the agent's `expectation_tolerance_ticks`
- `grace_ticks` = derived from the profile tolerance
- `basis` = `ExpectationBasis::PlanStepCompletion { step_index, kind_tag }`
- `state` = `ExpectationState::Active`

When the plan is replaced or the step completes successfully, the adoption-time records are resolved (`ExpectationState::Resolved { outcome: Fulfilled }`) or expired (`ExpectationState::Expired`) explicitly through a new AI-crate helper `clear_plan_step_expectations(agent, plan_id)`.

**Cross-crate exhaustive-match update**: `ExpectationBasis` is matched exhaustively in `crates/worldwake-ai/src/ranking.rs:1133-1135`. Adding the new variant requires a match arm — for ranking, `PlanStepCompletion` contributes no ranking-relevant weight and maps to `0` (plan-step expectations are agent-internal, not overdue-social-obligation-grade). Other sites (`per_agent_belief_view.rs`, `save_load.rs`, `expectation_check.rs` tests, golden tests, `search_actions.rs`, `report_actions.rs`, `ask_about_person_actions.rs`) currently construct specific variants and do not exhaustively match — no cascade edit required at those sites.

### D5: Revalidation upgrade — guard-check pass

`plan_revalidation.rs::revalidate_next_step` (`crates/worldwake-ai/src/plan_revalidation.rs:14`) currently returns `bool`. To surface the specific invalidation reason to the caller without plumbing an out-parameter through every call site, introduce a companion helper:

```rust
/// Drop-in boolean form, preserved for callers that only need pass/fail.
/// Internally delegates to `classify_revalidation`.
pub fn revalidate_next_step(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    step: &PlannedStep,
    bindings: &MaterializationBindings,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> bool {
    classify_revalidation(view, actor, step, bindings, registry, handlers).is_valid()
}

pub enum RevalidationOutcome {
    Valid,
    Invalidated { reason: PlanInvalidationReason },
}

pub fn classify_revalidation(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    step: &PlannedStep,
    bindings: &MaterializationBindings,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> RevalidationOutcome {
    // 1. Guard check (NEW)
    if let Some(guard) = &step.guard {
        if let Some(invalidator) = check_guard(view, actor, guard) {
            return RevalidationOutcome::Invalidated {
                reason: PlanInvalidationReason::ExpectationMismatch {
                    step_index: /* from caller context */,
                },
            };
        }
    }
    // 2. Existing affordance match (S108 strictness-aware)
    if requested_affordance_matches(...) {
        RevalidationOutcome::Valid
    } else {
        RevalidationOutcome::Invalidated { reason: PlanInvalidationReason::TargetGone { ... } }
    }
}
```

`PlanInvalidationReason::ExpectationMismatch` is reused (defined at `decision_event_payload.rs:179`). No new `GuardBreach` variant is introduced — the richer invalidator detail (which specific `Invalidator` fired) is carried in the widened `ExpectationMismatchPayload` (D7) when the event is emitted downstream.

Call sites of the existing `revalidate_next_step` continue to compile unchanged. New consumers that need the reason call `classify_revalidation` directly.

### D6: Plan-step mismatch emission + discrepancy classification (AI-side tick step)

The widening of plan-step expectation overdue handling splits along the `Active → Overdue` seam so it respects the one-way `ai → systems → sim → core` crate dependency graph:

**Sim-side (unchanged scope).** `crates/worldwake-systems/src/expectation_check.rs::check_overdue_expectations` continues to own the generic `ExpectationState::Active → Overdue` transition for *every* basis variant including `PlanStepCompletion`. It stays in worldwake-systems, stays driven by `SystemId::ExpectationCheck`, and does **not** reach into AI-crate types. No new belief-view accessor is added to `RuntimeBeliefView`. S114 contributes no change to this function's body beyond whatever is mechanically required by the new `ExpectationBasis` variant (exhaustive matches, if any).

**AI-side (new tick step).** Add a new per-agent tick step in worldwake-ai that runs after sim's `ExpectationCheck` phase completes. It iterates the agent's `ExpectationStore`, filters to records where `basis == ExpectationBasis::PlanStepCompletion { step_index, kind_tag }` and `state == Overdue`, and for each:

1. Resolve the step via `AgentDecisionRuntime::current_plan` (direct in-crate access — no trait indirection). If the plan is absent or `step_index` is out of range, transition the record to `Expired` (stale — the plan moved on) and skip emission.
2. Emit `EventTag::ExpectationMismatch` with a widened `ExpectationMismatchPayload` (D7) carrying the `kind_tag`, the step index, and the `Invalidator`-analogue diagnostic.
3. Route through `classify_discrepancy` (currently private in `crates/worldwake-ai/src/failure_handling.rs:133`; promote to `pub(crate)` or move to a shared AI-crate helper module so the new tick step can call it) to record into `DiscrepancyMemory` / `BlockerMemory` by `ExpectationKind` class:
   - `ExpectationKind::Immediate` mismatch → `Discrepancy::PartialExecutionDrift`
   - `ExpectationKind::State` mismatch → `Discrepancy::BeliefContradicted`
   - `ExpectationKind::Informed` mismatch → `Discrepancy::MissingObservation`
   - `ExpectationKind::Regression` mismatch → `Discrepancy::BeliefContradicted`
4. Transition the record to `ExpectationState::Resolved { outcome: ReturnedLate }` (implementation time may introduce a new outcome variant if none fits; this spec does not mandate one).

Natural placement: a new module alongside `crates/worldwake-ai/src/agent_tick/observation.rs` (e.g., `agent_tick/plan_step_expectations.rs`), invoked from the agent tick entry point after observation gathering and before planning. Exact placement is implementation-time; the constraint is "same tick, after sim's ExpectationCheck transitions records to Overdue, before the agent re-plans on the fresh discrepancy".

The existing non-plan expectations (`DutyAssignment`, `DeliveryCommitment`, `RoutineReturn`, `EscortObligation`, `SocialPromise`) are unchanged by D6. Sim's `check_overdue_expectations` continues to own their `Active → Overdue` transitions exactly as today; only `PlanStepCompletion`-basis records grow the AI-side follow-on step.

**Architectural rationale.** This mirrors D4's plan-adoption wiring: the AI crate owns plan-specific interpretation and writes/reads its own `ExpectationRecord`s; sim owns only the generic state transition. The invariant "worldwake-systems does not depend on worldwake-ai" is preserved. No new SystemFn in the sim manifest; `SystemId::ExpectationCheck` (`crates/worldwake-sim/src/system_manifest.rs:121`) remains the sole sim-side hook.

### D7: Widen `ExpectationMismatchPayload` (FND-28)

S110 pre-declared that S114 would widen this payload in place (`archive/specs/S110-decision-history-events.md:237-244`). Current shape at `crates/worldwake-core/src/decision_event_payload.rs:213-218`:

```rust
pub struct ExpectationMismatchPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub step_index: u16,
    pub expected_materializations: Vec<MaterializationTag>,
}
```

Widened:

```rust
pub struct ExpectationMismatchPayload {
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub step_index: u16,
    pub expected_materializations: Vec<MaterializationTag>,
    /// NEW: which of the four expectation kinds failed. `None` when the
    /// mismatch was detected pre-S114-style via `expected_materializations`
    /// alone and no `PlanStepCompletion` record was present.
    pub expectation_kind: Option<ExpectationKindTag>,
    /// NEW: the breached guard invalidator when the mismatch fired through
    /// revalidation's guard-check pass, or the unmet state predicate when
    /// the mismatch fired through the AI-side overdue-record tick step.
    pub mismatch_detail: Option<MismatchDetail>,
}

pub enum MismatchDetail {
    GuardInvalidator(InvalidatorTag),
    StateUnmet { predicate: StatePredicate },
    ObservationMissing { predicate: ObservationPredicate },
}

/// Tag-only form of `Invalidator` — serializable, discards `EntityId` /
/// `Tick` detail (that detail lives on the rich runtime `Invalidator`).
pub enum InvalidatorTag {
    BeliefStatusChange,
    TargetMoved,
    CommodityDepleted,
    NewBlockerRecorded,
}
```

No backward-compat decode path (FND-28). Save files and event logs pre-dating S114 are not decodable after this spec lands.

### D8: Authoritative-to-AI Impact Rule walkthrough

Per CLAUDE.md's Authoritative-to-AI Impact Rule, D5 gates step execution and therefore counts as a precondition change. Coverage:

1. `get_affordances` — **pass** (guards gate revalidation, not affordance discovery).
2. `generate_candidates` — **confirmed at ticket time**: emitters that would produce candidates duplicating a `NewBlockerRecorded` invalidator's suppression key must consult `BlockerMemory` as already required by S109; no new emitter logic required by S114 directly.
3. `search_plan` — **pass** (guards evaluate post-search; terminal ordering and barrier logic unchanged).
4. `BestEffort` action start — **confirmed at ticket time**: the start path invokes `classify_revalidation` (new seam) rather than `revalidate_next_step`'s boolean form so that a guard breach routes through `handle_plan_failure` with the specific `PlanInvalidationReason::ExpectationMismatch`.
5. `handle_plan_failure` — **pass** (replanning already handles `PlanInvalidationReason::ExpectationMismatch { step_index }`; no new arm needed).
6. Payload revalidation — **confirmed at ticket time**: for actions that use planner-synthesized payloads, the guard-check pass runs *before* `requested_affordance_matches`'s `with_payload_override_validator` delegation, so payload revalidation semantics are unchanged.
7. Golden tests — **must stay green**: existing `golden_planner_pathology`, `golden_survival_*`, and `golden_portfolio_planning` suites must pass; V7 (below) is the new coverage.

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Guards read the agent's own belief store (local, FND-14/14A) and the agent's own `BlockerMemory` (local, per S109). Expectations read the authoritative append-only event log (state, FND-26) and the agent's own `ExpectationStore` (local). No cross-agent information flow introduced. `ExpectationRecord`s written at plan-adoption time and read by `check_overdue_expectations` both live on the same agent — the monitor never queries another agent's store.
2. **Positive-feedback analysis**: A guard that always breaches → plan always invalidates → replan → same guard breaches. Dampener: S109's per-class discrepancy TTL (`stale_belief_backoff_ticks`, `contradicted_belief_backoff_ticks`, etc.) suppresses the goal until TTL expires. Loops cannot run faster than `min(TTL)` for the relevant discrepancy class.
3. **Concrete dampeners**: S109 discrepancy TTLs per class. Additionally, `GuardTemplateSpec::min_confidence` is a `Permille` (bounded [0, 1000]) with per-agent `guard_min_confidence_ceiling` so a guard cannot require impossible certainty.
4. **Stored state vs. derived read-model**:
   - **Authoritative stored state**: `ExpectationRecord` entries inside `ExpectationStore` components (written at plan adoption by the AI crate; transitioned to `Overdue` by sim's `check_overdue_expectations`; transitioned to `Resolved { ReturnedLate }` / `Expired` by the AI-side D6 tick step), and the event-log entries emitted on mismatch. `ActionDef::guard_template` / `expectation_template` are authored design-time data, saved through the existing `ActionDef` serialization path.
   - **Runtime-only**: `PlanGuard` and `Vec<PlanExpectation>` on `PlannedStep`, held by `AgentDecisionRuntime::current_plan`; rebuilt at plan-adoption time by `build_plan_guard` / `build_plan_expectations`. `MismatchDetail` is captured into the event log at mismatch time, then becomes authoritative historical state.

## SystemFn Integration

**No new SystemFn.** `SystemId::ExpectationCheck` (`crates/worldwake-sim/src/system_manifest.rs:121`) remains the sole sim-side hook for expectation-record lifecycle; its backing function `check_overdue_expectations` keeps its current scope (generic `Active → Overdue` state transition for every basis). Tick-phase order is unchanged: `Perception → ExpectationCheck → EvidenceDecay → ItemDecay → Patrol → Compaction`. The D6 interpretation of `PlanStepCompletion`-basis overdue records (step resolution, event emission, discrepancy classification, `Resolved`/`Expired` transition) runs inside the AI agent-tick sequence — per-agent, after sim phases complete — and is not a registered SystemFn.

## Component Registration

No new ECS components.

- `ExpectationStore` already exists on `EntityKind::Agent` (universal, runtime-generated per spec-drafting-rules.md §5 — memory-style component that starts empty and accumulates from plan adoption / duty assignment / delivery commitment).
- `CognitiveProfile` (universal, applied via `spawn_agent()`) gains two new fields per Profile-Driven Parameters below.
- `ActionDef` gains `guard_template` and `expectation_template` fields. `ActionDef` is not an ECS component — it is a design-time record in `ActionDefRegistry`. No scenario contract update required (action registrations are compiled in, not scenario-authored).

## Cross-System Interactions

- **Planner ↔ action registry**: Planner reads each `ActionDef`'s `guard_template` / `expectation_template` at plan-build time via `build_plan_guard` / `build_plan_expectations` (pure functions).
- **Planner ↔ ExpectationStore**: Plan adoption writes `ExpectationRecord`s with `ExpectationBasis::PlanStepCompletion`. Plan completion or replacement clears them.
- **Revalidation ↔ memory**: `classify_revalidation` reads the envelope (S113) plus `BlockerMemory` (S109) to evaluate invalidators.
- **Sim `ExpectationCheck` ↔ state**: `check_overdue_expectations` transitions `Active → Overdue` for every basis including `PlanStepCompletion`; no reach into AI-crate types, no emission.
- **AI tick step ↔ event log**: The new AI-side tick step (D6) iterates `PlanStepCompletion`-basis `Overdue` records, resolves the step via `AgentDecisionRuntime::current_plan`, and emits `EventTag::ExpectationMismatch` with the widened `ExpectationMismatchPayload`.
- **AI tick step ↔ discrepancy memory**: The same AI-side step routes mismatches through `classify_discrepancy` (AI-crate helper) into `DiscrepancyMemory` / `BlockerMemory` by `ExpectationKind` class, then transitions the record to `Resolved { outcome: ReturnedLate }` or `Expired`.

## Profile-Driven Parameters

| Parameter | Profile | Type | Default | Purpose |
|-----------|---------|------|---------|---------|
| `expectation_tolerance_ticks` | `CognitiveProfile` | `u32` | 2 | Slack added to `ExpectationRecord::deadline_tick` for plan-step expectations (maps to `grace_ticks` on the record) |
| `guard_min_confidence_ceiling` | `CognitiveProfile` | `Permille` | `Permille::new(1000)` | Per-agent ceiling: effective `min_confidence = min(guard.min_confidence, profile.ceiling)`. Lower ceilings let less careful agents act on weaker beliefs. |

Per spec-drafting-rules.md §5, both fields land on the universal `CognitiveProfile` component (already registered on every agent via `spawn_agent()` at `crates/worldwake-cli/src/scenario/mod.rs:421`) and require `#[serde(default = "...")]` so existing scenarios deserialize.

## Validation and Falsification

### Unit tests

1. Guard with `TargetPresent` required fact fires `TargetMoved` invalidator when belief-store envelope returns a different `at_place`.
2. Guard with `min_confidence: Permille::new(700)` fails when `BeliefValue::confidence` is `Permille::new(500)`.
3. Irrelevant drift (unrelated merchant restock) does not trigger any guard invalidator.
4. `ExpectationKind::Immediate` with `observe_by = tick+5` fires `ExpectationMismatch` at tick+6 if no `ActionCommitted` event landed for the step.
5. `build_plan_guard` translates a `GuardTemplateSpec::TargetPresent` binding into a concrete `RequiredFact::TargetPresent` using `step.primary_target()` and `step.target_place()`.
6. `build_plan_guard` returns `None` when `ActionDef::guard_template` is `None`.
7. `ExpectationBasis::PlanStepCompletion` variant round-trips through `bincode` with other variants unchanged.
8. `ActionDef` with and without `guard_template = Some(...)` both round-trip through `bincode` and the existing registry load path.

### Integration tests

9. Existing target-gone golden: with guards, the `Discrepancy::BeliefContradicted` replan path is taken (S109), not the `AssumptionFailed` fallback.
10. Survival scenarios pass: no increase in false-positive guard breaches on trivial paths (eat, sleep, wash). Specifically `golden_survival_baseline`, `golden_survival_scattered`, `golden_survival_contested` stay green.
11. `check_overdue_expectations` unit tests continue to pass (existing `RoutineReturn`-basis coverage) and gain a new test that a `PlanStepCompletion` basis transitions `Active → Overdue`; focused AI-tick coverage separately proves the follow-on `ExpectationMismatch` emission.

### Golden test

12. Deferred to `tickets/S114PLASTGUA-015.md`: `archive/tickets/S114PLASTGUA-014.md` landed the last remaining planner/search substrate fix for seller-backed displayed sale stock with known container detail, but the originally drafted fully autonomous stale-window is still disproved on the live branch. Re-author the scenario at the truthful hybrid/local trade-step seam so an agent first selects the remote seller-backed branch, then reaches a concrete local guarded `trade` step, merchant A departs before that step can lawfully enqueue, `ExpectationMismatch` appears in the event log with `expectation_kind: Some(ExpectationKindTag::State)` and `mismatch_detail: Some(GuardInvalidator(TargetMoved))`, `DiscrepancyMemory` records `Discrepancy::BeliefContradicted`, and the agent replans within 2 ticks of the departure.

## Outcome

To be filled in at completion.
