# Planner Contracts

This document is the authoritative planner-facing contract for planner boundaries that repeatedly show up in AI tickets and regressions:

1. exact-goal terminal operator surfacing
2. planning-snapshot completeness for planner-visible runtime data
3. belief-backed travel cost and route preference
4. decision-trace diagnostics for omitted operators and missing prerequisites
5. HTN method trace fallback and rejection diagnostics

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
- `GoalKind::EstablishBanditCamp` -> `PlannerOpKind::EstablishCamp`
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

### Entity admission and the belief barrier

Remote entity visibility has a separate contract from place-topology visibility.

The governing symbols are:

- [`PlanningSnapshot::build_with_blocked_facility_uses()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs)
- [`collect_entities()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs)
- [`build_snapshot_entity()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs)
- [`build_snapshot_places()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs)

The contract is:

- The actor's current place remains an authoritative planner-visible local surface. Co-located entities can enter the snapshot from authoritative runtime state because they are part of the actor's immediate local world.
- Remote places inside travel horizon do not grant omniscient remote entity visibility. Remote entities must enter the snapshot through remembered entity beliefs or explicit grounded evidence, not raw `entities_at(place)` truth alone.
- For non-self entities, planner-facing `effective_place` is belief/last-seen state unless the entity is same-tick co-located with the actor or directly possessed by the actor. Those two authoritative reads are lawful FND-14A physical-observation exceptions; all other remote location reads must come from the actor's belief or memory carriers.
- Planning and UI control visibility use belief-gated `ControlBeliefView::can_control`. It may expose co-located unowned physical items through the FND-14A local shortcut, but social ownership, rights, and effective control require an explicit belief-accessible entity. Authoritative dispatch and commit legality continue to use `World::can_exercise_control`.
- If a remote entity is admitted through belief carriage, snapshot fields that already exist on `BelievedEntityState` such as place, alive state, inventory, workstation tag, resource source, wounds, and courage must preserve the believed values instead of silently replacing them with authoritative runtime values.
- Explicit grounded evidence may still force a remote entity into the snapshot, but that is the evidence carrier doing the work. It is not a second general remote-entity fallback.
- Place topology is still broader than entity visibility. The planner may know adjacent places from the authoritative travel graph without automatically knowing which facilities, items, corpses, or offices currently occupy them.

When reassessing a planner ticket, state separately whether the issue is about place-topology inclusion, remote entity admission, or belief-backed field carriage. Do not collapse those into one generic "snapshot completeness" claim.

### Planner-visible fields are source-scoped

Entity admission does not make every current authoritative field on that entity
planner-visible. Every planner- or player-facing belief-view accessor must name
which source class makes its value lawful:

This is the application of FND-14B to belief-view accessors: planner-visible
inputs must be belief-backed, same-tick local physical observations, or another
lawful source class rather than raw remote world truth.

- Self: facts about the observing actor.
- Same-tick local physical observation: directly perceivable physical facts about
  entities at the actor's effective place, such as kind, item-lot
  commodity/quantity, workstation tags, resource-source availability, container
  contents, encumbrance/load, carry capacity, displayed sale-listing existence,
  and co-located workstation busy/idle state.
- Direct possession: directly possessed entities whose physical load/capacity or
  containment facts are observable through the actor's own inventory.
- Belief or memory: remote or delayed facts that arrived through the actor's
  belief, memory, testimony, report, record, or other explicit evidence carrier.
- Public topology: place-graph facts that are intentionally broader than entity
  visibility and do not imply knowledge of remote occupants or contents.

For S158, this rule governs the economic accessors `has_sale_listing`,
`seller_for_sale_lot`, and `listed_sale_lots_at`; the production accessor
`has_production_job`; the physical accessors `carry_capacity` and
`load_of_entity`; and the contention accessors `facility_queue_position`,
`facility_grant`, `extraction_slot_queue_position`,
`actor_holds_extraction_slot_grant`, and `contention_queue_is_full`.
Remote values for those fields must come from an existing belief or memory
carrier, such as `EntityBeliefAspect::Activity`,
`EntityBeliefAspect::ContentionState`, or opportunity memory. If no such carrier
exists, the accessor returns unknown, empty, or false; it must not fall back to
current world state.

The control and rights value path remains governed by the existing control
language above. Stricter belief-backing for rights/control values requires a
future believed-rights or jurisdiction aspect rather than a hidden fallback in
the planner view.

### Current duration dependency inventory

The live inventory is:

- `TargetConsumable`
- `ActorMetabolism`
- `BanditCampEstablishmentProfile`
- `ActorTradeDisposition`
- `ActorTheftDisposition`
- `ActorInvestigationDisposition`
- `ActorWitnessQueryDisposition`
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

### Belief-backed travel cost

Travel planning has an additional snapshot-backed contract that is easy to misstate if you reconstruct it from older tickets.

The governing symbols are:

- [`PlanningSnapshot::min_travel_ticks()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs)
- [`PlanningSnapshot::min_perceived_travel_cost_to_any()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs)
- [`PlanningSnapshot::direct_perceived_travel_cost()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs)
- [`PlanningSnapshot::direct_perceived_travel_breakdown()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs)
- [`route_threat::route_threat_estimate()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/route_threat.rs)
- [`search::heuristic::compute_heuristic()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/heuristic.rs)
- [`search::heuristic::prune_travel_away_from_goal()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/heuristic.rs)
- [`search::frontier::compare_search_nodes()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/frontier.rs)
- [`search::transition::build_successor_detailed()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/transition.rs)

The contract is:

- Travel still executes over the authoritative adjacency graph. The planner does not invent a second topology.
- Raw travel duration remains the authoritative runtime quantity used by `DurationExpr::TravelToTarget` and action execution.
- Route preference is planner-local and belief-backed. The planner may rank or prune travel branches using perceived travel cost derived from the actor's remembered entity state, remembered social conflict, and live `belief_confidence()` aging.
- That perceived cost must stay snapshot-backed. The planner must not query authoritative world threat state directly.
- The same perceived cost model must be used consistently across search ordering and travel pruning. Do not bolt a one-off penalty onto one planner stage while leaving the others on raw travel ticks.
- Ignorance remains first-class: when the actor lacks relevant danger beliefs, perceived travel cost collapses back to raw travel cost.

When reassessing a travel-planning ticket, state explicitly whether the claim concerns authoritative travel duration, planner-local perceived travel cost, or both. Do not collapse them into one vague "route cost" claim.

### Comparative travel-branch traceability

The live comparative route-choice trace contract lives in:

- [`TravelSuccessorTrace`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
- [`TravelPruningTrace`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
- [`SelectedPlanSearchProvenance`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
- [`summarize_search_provenance()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)

The contract is:

- Comparative route diagnostics reuse the existing planner-owned travel-pruning and selected-plan provenance path. There is no second route-debug subsystem.
- Each traced travel successor must stay concrete: destination, raw edge ticks, perceived threat contribution, penalty ticks, resulting direct perceived cost, remaining perceived cost to goal, and projected total perceived cost.
- The selected-plan summary may additionally expose the winning root travel destination when the chosen plan starts with travel. This identifies which retained branch actually won without mutating the raw root expansion record after the fact.
- These fields explain the current live planner arithmetic only. They must not introduce a parallel "route quality" score or authoritative threat read.

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

## 4. HTN Method Trace Fallback And Rejection

The HTN method trace contract lives in:

- [`crates/worldwake-ai/src/htn/selector.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/htn/selector.rs)
- [`crates/worldwake-ai/src/search/strategic.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/strategic.rs)
- [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)

`MethodPlanAttemptTrace` is a transient debug read-model, not authoritative
state. It is not serialized save or replay state and must not become a second
source of truth for planner legality.

The contract is:

- `MethodRegistry` is the method-assignment authority for goal kinds.
- `select_method_with_recipes()` returns both the selected method and rejected
  candidate methods for the goal kind.
- Each `RejectedMethodTrace` records the rejected `method_id` and the first
  failed `MethodPrecondition`.
- `MethodPlanAttemptTrace.method_id` records the selected method when method
  decomposition produced strategic stages.
- `MethodPlanAttemptTrace.rejected_methods` records contrastive "why not?"
  data for considered methods that failed preconditions.
- `MethodPlanAttemptTrace.fallback_reason` records explicit flat-GOAP fallback:
  `NoViableMethod` when no method survived selection, or
  `MethodProducedNoStages` when a selected method produced no strategic stages.
- Fallback remains legal unless a future method-required schema contract proves
  that flat fallback would satisfy the wrong semantic condition.

Do not infer fallback from `method_trace: None`. For HTN-capable goal kinds, use
the trace's selected method, rejected methods, and fallback reason together.

## 5. How To Use This In Tickets

For planner-driven tickets:

- name the live `GoalKind`
- name the exact operator surface under audit
- state whether the behavior is plain GOAP/affordance search, HTN method decomposition over existing affordances, both with fallback, or method-required
- for HTN method work, name the reusable pursuit pattern that justifies method registration rather than plain GOAP; for method-required work, name the explicit schema contract that makes flat fallback unlawful
- name whether travel reasoning depends on authoritative duration, perceived travel cost, or neither
- state whether the terminal binding comes from `GoalKind` identity, grounded evidence, or neither
- state whether the proof boundary is root omission tracing, surfaced-candidate skip tracing, same-goal sibling stop tracing, selection branch attribution, snapshot/state parity, or another lower-layer planner test
- for HTN method work, state whether the proof boundary is selector rejection,
  selected-method trace, fallback-reason trace, or a full autonomous/golden
  planning attempt

If traces prove the immediate result but not enough provenance to explain the architecture, follow `docs/precision-rules.md`: prove the behavior at the strongest lower layer and open a follow-up traceability ticket if the missing explanation surface matters.

## 6. Architectural Guardrails

These planner contracts exist to preserve Worldwake's core design rules:

- Explainable emergence: planner choices should be traceable to explicit contracts, not hidden fallback behavior.
- Locality of information: planning reads belief-backed snapshot state, not omniscient runtime state.
- Concrete state over abstractions: duration dependencies and target bindings are named as concrete data contracts.
- No workaround architecture: do not add a parallel synthesis path, duplicate duration inventory, or ad hoc debug output when the contract already has an authoritative boundary.
