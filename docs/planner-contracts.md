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

Decision traces should explain planner failures at the planner boundary, not force source-diving into unrelated modules.

### Omitted relevant operators

When a relevant operator never becomes a root candidate, the planner records a `RootOperatorOmissionTrace` with one of these reasons:

- `NoMatchingActionDef`
- `NoAffordanceOrSynthesisPath`
- `SynthesisUnsupportedGoalOp`
- `SynthesisTargetDerivationFailed`

Use these when the question is "why did this relevant operator never show up at all?"

### Surfaced root candidates that still fail

When a root candidate exists but is filtered or skipped later, the planner records `RootCandidateTrace` with `RootCandidateOutcome`.

For missing planner-visible duration inputs, the key diagnostic is:

- `RootCandidateSkipReason::DurationEstimateFailed { dependency }`

The `dependency` value must come from `PlannerDurationDependency`, not from ad hoc strings or broad "duration failed" buckets.

Use these when the question is "the operator surfaced, so why did it still not expand?"

## 4. How To Use This In Tickets

For planner-driven tickets:

- name the live `GoalKind`
- name the exact operator surface under audit
- state whether the terminal binding comes from `GoalKind` identity, grounded evidence, or neither
- state whether the proof boundary is root omission tracing, surfaced-candidate skip tracing, snapshot/state parity, or another lower-layer planner test

If traces prove the immediate result but not enough provenance to explain the architecture, follow `docs/precision-rules.md`: prove the behavior at the strongest lower layer and open a follow-up traceability ticket if the missing explanation surface matters.

## 5. Architectural Guardrails

These planner contracts exist to preserve Worldwake's core design rules:

- Explainable emergence: planner choices should be traceable to explicit contracts, not hidden fallback behavior.
- Locality of information: planning reads belief-backed snapshot state, not omniscient runtime state.
- Concrete state over abstractions: duration dependencies and target bindings are named as concrete data contracts.
- No workaround architecture: do not add a parallel synthesis path, duplicate duration inventory, or ad hoc debug output when the contract already has an authoritative boundary.
