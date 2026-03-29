# Planner Contracts

This document is the authoritative planner-facing contract for three boundaries that repeatedly show up in AI tickets and regressions:

1. exact-goal terminal operator surfacing
2. planning-snapshot completeness for planner-visible runtime data
3. decision-trace diagnostics for omitted operators and missing prerequisites

Use this doc when a ticket touches planner root candidates, snapshot-backed planning state, or AI traceability. Keep `docs/FOUNDATIONS.md` as the design authority and `docs/precision-rules.md` as the claim-writing authority. This file exists to make the live planner architecture explicit, not to duplicate either of those documents.

## Why This Exists

The planner boundary is easy to misdescribe if you reconstruct it from one bugfix or one golden. Worldwake now has explicit contracts in code, but without one repository doc future tickets can still regress into stale narratives such as:

- "the operator is missing" when the real gap is root synthesis or omission diagnostics
- "the snapshot forgot some belief data" without naming the centralized duration dependency inventory
- "the trace did not explain it" without distinguishing omitted operators from surfaced-and-skipped candidates

The clean architecture is one explicit contract, not ticket lore.

## 1. Exact-Goal Terminal Surfacing

The root synthesis contract lives in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) and [`crates/worldwake-ai/src/search/candidates.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs).

The governing symbols are:

- [`GoalKindPlannerExt::relevant_op_kinds()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
- [`GoalKindPlannerExt::build_payload_override()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
- [`GoalKindPlannerExt::matches_binding()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
- [`GroundedGoal::synthesized_root_candidate_targets()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs#L1578)
- [`search::candidates::goal_synthesized_candidates()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs)

The contract is:

- `GoalKind` declares the relevant operator family through `relevant_op_kinds()`.
- A root candidate may be synthesized when the grounded goal already carries the canonical terminal binding and `synthesized_root_candidate_targets()` can derive the authoritative targets for that operator.
- Payload construction stays centralized in `build_payload_override()`. Root synthesis is not a second payload system.
- Binding checks stay centralized in `matches_binding()`. Exact-bound terminal ops must still match the goal's canonical target identity.
- If a goal family is intentionally deferred, it must stay absent by exposing no live terminal surfacing path instead of silently falling through generic synthesis.

### Live synthesized terminal families

Today the explicit synthesis surface is:

- `GoalKind::ShareBelief` -> `PlannerOpKind::Tell`
- `GoalKind::InvestigateViolation` -> `PlannerOpKind::Investigate`
- `GoalKind::Accuse` -> `PlannerOpKind::Accuse`
- `GoalKind::ClaimOffice` -> `PlannerOpKind::PressForceClaim`
- exact local trade goals backed by one grounded evidence entity:
  `GoalKind::AcquireCommodity`, `GoalKind::ConsumeOwnedCommodity`, `GoalKind::RestockCommodity`, and `GoalKind::TreatWounds` -> `PlannerOpKind::Trade`

Trade is the important special case: its exact terminal binding comes from `GroundedGoal.evidence_entities`, not from raw `GoalKind` identity alone.

### Deliberately absent surfaces

- `GoalKind::PunishAccused` remains deferred. Its `relevant_op_kinds()` surface is intentionally empty, so it should not appear through generic root synthesis.

When reassessing a ticket, name the live goal family and the exact root operator surface it depends on. Do not write the ticket against an older narrative.

## 2. Planning Snapshot Completeness

The snapshot completeness contract lives across:

- [`PlanningSnapshot`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs)
- [`PlanningState`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs)
- [`PlannerDurationDependency`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_duration_contract.rs)

The planner does not get to read authoritative runtime state directly. If successor construction needs runtime-visible data, that data must survive into snapshot-backed planning state.

For dynamic action durations, the authoritative planner-local inventory is [`PlannerDurationDependency`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_duration_contract.rs). This is the single source of truth for planner-supported non-fixed duration dependencies.

### Current duration dependency inventory

The live inventory is:

- `TargetConsumable`
- `ActorMetabolism`
- `ActorTradeDisposition`
- `ActorTheftDisposition`
- `ActorInvestigationDisposition`
- `ActorDefendStance`
- `CombatWeapon`
- `TargetTreatment`
- `ConsultRecord`
- `TravelToTarget`

The contract is:

- if a planner-supported non-fixed `DurationExpr` exists, it must map into `PlannerDurationDependency`
- the needed data must be preserved in `PlanningSnapshot`
- the same data must be readable through `PlanningState`
- focused parity coverage must prove snapshot-backed estimation stays aligned with runtime `estimate_duration_from_beliefs()`

This inventory is planner-local on purpose. It aligns to runtime semantics without creating a cross-crate alias layer or letting the planner reach around the belief boundary.

## 3. Traceability For Omitted Operators And Missing Prerequisites

The traceability contract lives in:

- [`crates/worldwake-ai/src/search/candidates.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs)
- [`crates/worldwake-ai/src/search/transition.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/transition.rs)
- [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
- [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)

Decision traces should explain planner failures at the planner boundary, not force source-diving into unrelated modules.

### Omitted relevant operators

When a relevant operator never becomes a root candidate, the planner records a `RootOperatorOmissionTrace` with one of these reasons:

- `NoMatchingActionDef`
- `NoAffordanceOrSynthesisPath`
- `SynthesisUnsupportedGoalOp`
- `SynthesisTargetDerivationFailed`

Use these when the question is "why did this relevant operator never show up at all?"

### Omitted conditional epistemic barriers

`PlannerOpKind::AskWitness` is not part of `GoalKind::RestockCommodity`'s `relevant_op_kinds()` surface. It is a conditional epistemic barrier candidate injected at root search only when `grounded_goal_epistemic_subjects()` derives stale subjects.

When that conditional barrier is absent at the root, the planner still records `RootOperatorOmissionTrace`, but with:

- `RootOperatorOmissionReason::ConditionalBarrierUnavailable`
- `RootOperatorOmissionDetail::AskWitness(AskWitnessOmissionDetail::NoStaleEpistemicSubjects)` when the stale-subject derivation is empty
- `RootOperatorOmissionDetail::AskWitness(AskWitnessOmissionDetail::NoWitnessAffordance)` when stale subjects exist but no co-located `ask_witness` affordance target exists

Do not infer this from missing `AskWitness` root candidates in end-to-end traces. Use the omission detail directly.

Do not overstate this contract: the current planner snapshot path does not preserve `ask_witness_memory`, so planner-search-visible payload-suppression distinctions are not live at this boundary.

### Surfaced root candidates that still fail

When a root candidate exists but is filtered or skipped later, the planner records `RootCandidateTrace` with `RootCandidateOutcome`.

For missing planner-visible duration inputs, the key diagnostic is:

- `RootCandidateSkipReason::DurationEstimateFailed { dependency }`

The `dependency` value must come from `PlannerDurationDependency`, not from ad hoc strings or broad "duration failed" buckets.

Use these when the question is "the operator surfaced, so why did it still not expand?"

### Same-goal sibling planning stop provenance

Opportunity-scoped same-goal continuation now has an explicit trace contract in:

- [`build_candidate_plans()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
- [`summarize_same_goal_planning_trace()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
- [`PlanSearchTrace.same_goal_trace`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)

The contract is:

- `PlanSearchTrace.attempts` remains the authoritative admitted-attempt order.
- Once one admitted attempt finds a plan, `build_candidate_plans()` continues only through later contiguous siblings with the same `GoalKey`.
- The sibling scan stops for exactly one bounded reason recorded in `SameGoalPlanningStopReason`:
  - `EncounteredDifferentGoal { next_goal }`
  - `ReachedCandidatePlanCap`
  - `ExhaustedAdmittedOpportunities`
- `SameGoalPlanningTrace.continuation_trigger` records which found opportunity first enabled same-goal continuation. It is `None` when no admitted attempt found a plan.

Do not reconstruct this from list length alone. Use `same_goal_trace`.

### Same-goal branch attribution at selection

Selection provenance for same-goal replans lives in:

- [`determine_selected_plan_source()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
- [`summarize_plan_replacement()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
- [`SelectionTrace.selected_plan_source`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
- [`SelectionTrace.plan_replacement`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)

The contract is:

- `selected_plan_source` still answers the coarse source question: `SearchSelection`, `RetainedCurrentPlan`, or `SnapshotContinuation`.
- When a fresh search displaces an existing branch, `plan_replacement.kind` records the finer branch attribution:
  - `SameGoalBranchRefreshed`
  - `SameGoalSiblingReplaced`
  - `GoalChanged`
- Same-goal refresh vs same-goal sibling replacement is determined by comparing the current plan's canonical `OpportunityKey` with the selected fresh plan's `OpportunityKey`. This stays opportunity-scoped; there is no second alias path at the `GoalKey` level.

Do not overload `selected_plan_source` to answer branch-identity questions it does not own by itself. Read both fields together.

## 4. How To Use This In Tickets

For planner-driven tickets:

- name the live `GoalKind`
- name the exact operator surface under audit
- state whether the terminal binding comes from `GoalKind` identity, grounded evidence, or neither
- state whether the proof boundary is root omission tracing, surfaced-candidate skip tracing, same-goal sibling stop tracing, selection branch attribution, snapshot/state parity, or another lower-layer planner test

If traces prove the immediate result but not enough provenance to explain the architecture, follow `docs/precision-rules.md`: prove the behavior at the strongest lower layer and open a follow-up traceability ticket if the missing explanation surface matters.

## 5. Architectural Guardrails

These planner contracts exist to preserve Worldwake's core design rules:

- Explainable emergence: planner choices should be traceable to explicit contracts, not hidden fallback behavior.
- Locality of information: planning reads belief-backed snapshot state, not omniscient runtime state.
- Concrete state over abstractions: duration dependencies and target bindings are named as concrete data contracts.
- No workaround architecture: do not add a parallel synthesis path, duplicate duration inventory, or ad hoc debug output when the contract already has an authoritative boundary.
