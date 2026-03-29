use crate::goal_model::{
    grounded_goal_epistemic_subjects, grounded_goal_matches_epistemic_barrier,
    RootCandidateSynthesis,
};
use crate::planner_ops::{planner_only_candidates, PlannerOpKind};
use crate::{
    GoalKindPlannerExt, GroundedGoal, PlannerOpSemantics, PlanningEntityRef, PlanningState,
};
use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{ActionDefId, BlockedIntentMemory, EntityId, GoalKind, Tick};
use worldwake_sim::{
    get_affordances_for_defs, ActionDefRegistry, ActionHandlerRegistry, ActionPayload, Affordance,
    QueueForFacilityUsePayload, RuntimeBeliefView,
};

use super::SearchNode;

#[derive(Clone)]
pub(super) struct SearchCandidate {
    pub(super) def_id: ActionDefId,
    pub(super) authoritative_targets: Vec<EntityId>,
    pub(super) planning_targets: Vec<PlanningEntityRef>,
    pub(super) payload_override: Option<ActionPayload>,
    pub(super) planner_only: bool,
    pub(super) trace_index: Option<usize>,
}

pub(super) fn relevant_action_defs(
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) -> BTreeSet<ActionDefId> {
    let relevant_ops = goal.key.kind.relevant_op_kinds();
    semantics_table
        .iter()
        .filter(|(_, sem)| relevant_ops.contains(&sem.op_kind))
        .map(|(def_id, _)| *def_id)
        .collect()
}

pub(super) fn push_root_candidate_trace(
    sink: &mut Option<&mut Vec<crate::decision_trace::RootCandidateTrace>>,
    trace: crate::decision_trace::RootCandidateTrace,
) -> Option<usize> {
    let sink = sink.as_deref_mut()?;
    sink.push(trace);
    Some(sink.len() - 1)
}

pub(super) fn update_root_candidate_outcome(
    sink: &mut Option<&mut Vec<crate::decision_trace::RootCandidateTrace>>,
    trace_index: Option<usize>,
    outcome: crate::decision_trace::RootCandidateOutcome,
) {
    let Some(trace_index) = trace_index else {
        return;
    };
    let Some(sink) = sink.as_deref_mut() else {
        return;
    };
    if let Some(trace) = sink.get_mut(trace_index) {
        trace.outcome = outcome;
    }
}

pub(super) fn root_candidate_payload_status(
    candidate_payload: Option<&ActionPayload>,
    resolved_payload: Option<&ActionPayload>,
) -> crate::decision_trace::RootCandidatePayloadStatus {
    if candidate_payload.is_some() {
        return crate::decision_trace::RootCandidatePayloadStatus::CandidateProvided;
    }
    if resolved_payload.is_some() {
        return crate::decision_trace::RootCandidatePayloadStatus::GoalSynthesized;
    }
    crate::decision_trace::RootCandidatePayloadStatus::None
}

pub(super) fn root_candidate_trace_from_candidate(
    candidate: &SearchCandidate,
    registry: &ActionDefRegistry,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) -> crate::decision_trace::RootCandidateTrace {
    crate::decision_trace::RootCandidateTrace {
        def_id: candidate.def_id,
        action_name: registry
            .get(candidate.def_id)
            .map_or_else(|| "<unknown>".to_string(), |def| def.name.clone()),
        op_kind: semantics_table
            .get(&candidate.def_id)
            .map(|sem| sem.op_kind),
        authoritative_targets: candidate.authoritative_targets.clone(),
        planner_only: candidate.planner_only,
        payload_status: root_candidate_payload_status(candidate.payload_override.as_ref(), None),
        outcome: crate::decision_trace::RootCandidateOutcome::Expanded,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn search_candidates(
    goal: &GroundedGoal,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    blocked: &BlockedIntentMemory,
    current_tick: Tick,
    binding_rejections: Option<&mut Vec<crate::decision_trace::BindingRejection>>,
    root_candidates: Option<&mut Vec<crate::decision_trace::RootCandidateTrace>>,
    root_omissions: Option<&mut Vec<crate::decision_trace::RootOperatorOmissionTrace>>,
    relevant_defs: &BTreeSet<ActionDefId>,
) -> Vec<SearchCandidate> {
    let epistemic_subjects = grounded_goal_epistemic_subjects(goal, &node.state);
    let mut affordance_defs = relevant_defs.clone();
    if !epistemic_subjects.is_empty() {
        affordance_defs.extend(semantics_table.iter().filter_map(|(def_id, semantics)| {
            (semantics.op_kind == PlannerOpKind::AskWitness).then_some(*def_id)
        }));
    }

    let raw_affordance_candidates = get_affordances_for_defs(
        &node.state,
        node.state.snapshot().actor(),
        registry,
        handlers,
        &affordance_defs,
    )
    .into_iter()
    .flat_map(|affordance| {
        search_candidates_from_affordance(goal, &node.state, registry, &affordance)
    })
    .collect::<Vec<_>>();
    let ask_witness_omission = conditional_ask_witness_omission_trace(
        goal,
        &node.state,
        semantics_table,
        &raw_affordance_candidates,
    );
    let affordance_candidates = raw_affordance_candidates
        .into_iter()
        .filter(|candidate| {
            semantics_table
                .get(&candidate.def_id)
                .is_none_or(|semantics| {
                    if epistemic_subjects.is_empty() {
                        return true;
                    }
                    semantics.op_kind == PlannerOpKind::Travel
                        || grounded_goal_matches_epistemic_barrier(
                            goal,
                            &node.state,
                            semantics.op_kind,
                            &candidate.authoritative_targets,
                            candidate.payload_override.as_ref(),
                        )
                })
        })
        .collect::<Vec<_>>();
    let mut candidates = affordance_candidates;
    candidates.extend(
        planner_only_candidates(&node.state, semantics_table)
            .into_iter()
            .map(search_candidate_from_planner),
    );
    candidates.extend(goal_synthesized_candidates(
        goal,
        registry,
        semantics_table,
        relevant_defs,
        &candidates,
    ));
    let mut root_omissions = root_omissions;
    if let Some(root_omissions) = root_omissions.as_mut() {
        record_root_operator_omissions(
            goal,
            registry,
            semantics_table,
            &candidates,
            Some(&mut **root_omissions),
        );
        if let Some(trace) = ask_witness_omission {
            root_omissions.push(trace);
        }
    } else {
        record_root_operator_omissions(goal, registry, semantics_table, &candidates, None);
    }
    let mut root_candidates = root_candidates;
    let mut binding_rejections = binding_rejections;
    let mut filtered = Vec::with_capacity(candidates.len());

    for mut candidate in candidates {
        let trace_index = push_root_candidate_trace(
            &mut root_candidates,
            root_candidate_trace_from_candidate(&candidate, registry, semantics_table),
        );

        if let Some((facility, intended_action)) =
            candidate_blocked_facility_use(&candidate, &node.state, registry)
        {
            update_root_candidate_outcome(
                &mut root_candidates,
                trace_index,
                crate::decision_trace::RootCandidateOutcome::Filtered(
                    crate::decision_trace::RootCandidateFilterReason::BlockedFacilityUse {
                        facility,
                        intended_action,
                    },
                ),
            );
            continue;
        }

        let Some(semantics) = semantics_table.get(&candidate.def_id) else {
            candidate.trace_index = trace_index;
            filtered.push(candidate);
            continue;
        };
        let passes_binding = goal
            .key
            .kind
            .matches_binding(&candidate.authoritative_targets, semantics.op_kind);
        if !passes_binding {
            let required_target = goal.key.entity.or(goal.key.place);
            if let Some(rejections) = binding_rejections.as_deref_mut() {
                rejections.push(crate::decision_trace::BindingRejection {
                    def_id: candidate.def_id,
                    rejected_targets: candidate.authoritative_targets.clone(),
                    required_target,
                });
            }
            update_root_candidate_outcome(
                &mut root_candidates,
                trace_index,
                crate::decision_trace::RootCandidateOutcome::Filtered(
                    crate::decision_trace::RootCandidateFilterReason::BindingMismatch {
                        required_target,
                    },
                ),
            );
            continue;
        }

        if let Some((place, blocking_fact)) = candidate_blocked_by_place(
            &candidate,
            goal,
            node,
            semantics_table,
            blocked,
            current_tick,
        ) {
            update_root_candidate_outcome(
                &mut root_candidates,
                trace_index,
                crate::decision_trace::RootCandidateOutcome::Filtered(
                    crate::decision_trace::RootCandidateFilterReason::PlaceBlocker {
                        place,
                        blocking_fact,
                    },
                ),
            );
            continue;
        }

        candidate.trace_index = trace_index;
        filtered.push(candidate);
    }
    filtered
}

fn goal_synthesized_candidates(
    goal: &GroundedGoal,
    registry: &ActionDefRegistry,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    relevant_defs: &BTreeSet<ActionDefId>,
    existing_candidates: &[SearchCandidate],
) -> Vec<SearchCandidate> {
    relevant_defs
        .iter()
        .filter(|def_id| {
            !existing_candidates
                .iter()
                .any(|candidate| candidate.def_id == **def_id)
        })
        .filter_map(|def_id| {
            let def = registry.get(*def_id)?;
            let semantics = semantics_table.get(def_id)?;
            match goal.synthesized_root_candidate_targets(def, *semantics) {
                RootCandidateSynthesis::Targets(authoritative_targets) => Some(SearchCandidate {
                    def_id: *def_id,
                    authoritative_targets: authoritative_targets.clone(),
                    planning_targets: authoritative_targets
                        .into_iter()
                        .map(PlanningEntityRef::Authoritative)
                        .collect(),
                    payload_override: None,
                    planner_only: false,
                    trace_index: None,
                }),
                RootCandidateSynthesis::NoSynthesisPath
                | RootCandidateSynthesis::UnsupportedGoalOp
                | RootCandidateSynthesis::TargetDerivationFailed => None,
            }
        })
        .collect()
}

fn record_root_operator_omissions(
    goal: &GroundedGoal,
    registry: &ActionDefRegistry,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    candidates: &[SearchCandidate],
    sink: Option<&mut Vec<crate::decision_trace::RootOperatorOmissionTrace>>,
) {
    let Some(sink) = sink else {
        return;
    };
    let candidate_ops = candidates
        .iter()
        .filter_map(|candidate| {
            semantics_table
                .get(&candidate.def_id)
                .map(|sem| sem.op_kind)
        })
        .collect::<BTreeSet<_>>();

    for op_kind in goal.key.kind.relevant_op_kinds() {
        if candidate_ops.contains(op_kind) {
            continue;
        }
        let defs_for_op = semantics_table
            .iter()
            .filter_map(|(def_id, sem)| (sem.op_kind == *op_kind).then_some(*def_id))
            .collect::<Vec<_>>();
        if defs_for_op.is_empty() {
            sink.push(crate::decision_trace::RootOperatorOmissionTrace {
                op_kind: *op_kind,
                reason: crate::decision_trace::RootOperatorOmissionReason::NoMatchingActionDef,
                detail: None,
            });
            continue;
        }

        let mut saw_target_derivation_failure = false;
        let mut saw_unsupported_goal_op = false;
        for def_id in defs_for_op {
            let Some(def) = registry.get(def_id) else {
                continue;
            };
            let Some(semantics) = semantics_table.get(&def_id) else {
                continue;
            };
            match goal.synthesized_root_candidate_targets(def, *semantics) {
                RootCandidateSynthesis::TargetDerivationFailed => {
                    saw_target_derivation_failure = true;
                }
                RootCandidateSynthesis::UnsupportedGoalOp => {
                    saw_unsupported_goal_op = true;
                }
                RootCandidateSynthesis::Targets(_) | RootCandidateSynthesis::NoSynthesisPath => {}
            }
        }

        let reason = if saw_target_derivation_failure {
            crate::decision_trace::RootOperatorOmissionReason::SynthesisTargetDerivationFailed
        } else if saw_unsupported_goal_op {
            crate::decision_trace::RootOperatorOmissionReason::SynthesisUnsupportedGoalOp
        } else {
            crate::decision_trace::RootOperatorOmissionReason::NoAffordanceOrSynthesisPath
        };
        sink.push(crate::decision_trace::RootOperatorOmissionTrace {
            op_kind: *op_kind,
            reason,
            detail: None,
        });
    }
}

fn conditional_ask_witness_omission_trace(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    affordance_candidates: &[SearchCandidate],
) -> Option<crate::decision_trace::RootOperatorOmissionTrace> {
    if goal
        .key
        .kind
        .relevant_op_kinds()
        .contains(&PlannerOpKind::AskWitness)
    {
        return None;
    }

    let ask_witness_defs_exist = semantics_table
        .values()
        .any(|semantics| semantics.op_kind == PlannerOpKind::AskWitness);
    if !ask_witness_defs_exist {
        return Some(crate::decision_trace::RootOperatorOmissionTrace {
            op_kind: PlannerOpKind::AskWitness,
            reason: crate::decision_trace::RootOperatorOmissionReason::NoMatchingActionDef,
            detail: None,
        });
    }

    let epistemic_subjects = grounded_goal_epistemic_subjects(goal, state);
    if epistemic_subjects.is_empty() {
        return Some(crate::decision_trace::RootOperatorOmissionTrace {
            op_kind: PlannerOpKind::AskWitness,
            reason:
                crate::decision_trace::RootOperatorOmissionReason::ConditionalBarrierUnavailable,
            detail: Some(
                crate::decision_trace::RootOperatorOmissionDetail::AskWitness(
                    crate::decision_trace::AskWitnessOmissionDetail::NoStaleEpistemicSubjects,
                ),
            ),
        });
    }

    let ask_witness_candidates = affordance_candidates
        .iter()
        .filter(|candidate| {
            semantics_table
                .get(&candidate.def_id)
                .is_some_and(|semantics| semantics.op_kind == PlannerOpKind::AskWitness)
        })
        .collect::<Vec<_>>();
    if ask_witness_candidates.is_empty() {
        return Some(crate::decision_trace::RootOperatorOmissionTrace {
            op_kind: PlannerOpKind::AskWitness,
            reason:
                crate::decision_trace::RootOperatorOmissionReason::ConditionalBarrierUnavailable,
            detail: Some(
                crate::decision_trace::RootOperatorOmissionDetail::AskWitness(
                    crate::decision_trace::AskWitnessOmissionDetail::NoWitnessAffordance,
                ),
            ),
        });
    }

    None
}

fn candidate_blocked_facility_use(
    candidate: &SearchCandidate,
    state: &PlanningState<'_>,
    registry: &ActionDefRegistry,
) -> Option<(EntityId, ActionDefId)> {
    let facility = candidate.authoritative_targets.first().copied()?;
    let intended_action = intended_exclusive_action(candidate, registry)?;

    state
        .is_facility_use_blocked(facility, intended_action)
        .then_some((facility, intended_action))
}

fn intended_exclusive_action(
    candidate: &SearchCandidate,
    registry: &ActionDefRegistry,
) -> Option<ActionDefId> {
    if let Some(payload) = candidate
        .payload_override
        .as_ref()
        .and_then(ActionPayload::as_queue_for_facility_use)
    {
        return Some(payload.intended_action);
    }

    let payload = candidate
        .payload_override
        .as_ref()
        .or_else(|| registry.get(candidate.def_id).map(|def| &def.payload))?;
    matches!(payload, ActionPayload::Harvest(_) | ActionPayload::Craft(_))
        .then_some(candidate.def_id)
}

pub(super) fn search_candidates_from_affordance(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    registry: &ActionDefRegistry,
    affordance: &Affordance,
) -> Vec<SearchCandidate> {
    let planning_targets = affordance
        .bound_targets
        .iter()
        .copied()
        .map(PlanningEntityRef::Authoritative)
        .collect::<Vec<_>>();
    let base = SearchCandidate {
        def_id: affordance.def_id,
        authoritative_targets: affordance.bound_targets.clone(),
        planning_targets,
        payload_override: affordance.payload_override.clone(),
        planner_only: false,
        trace_index: None,
    };

    let Some(def) = registry.get(affordance.def_id) else {
        return vec![base];
    };
    if def.name != "queue_for_facility_use" {
        return vec![base];
    }
    if base.payload_override.is_some() {
        return vec![base];
    }

    let Some(facility) = affordance.bound_targets.first().copied() else {
        return Vec::new();
    };
    if state
        .snapshot()
        .entities
        .get(&facility)
        .and_then(|entity| entity.facility_queue.as_ref())
        .is_none()
    {
        return Vec::new();
    }
    let Some((workstation_tag, intended_actions)) =
        queue_intended_actions_for(goal, state, registry, facility)
    else {
        return Vec::new();
    };
    if state.is_actor_queued_at_facility(facility) {
        return Vec::new();
    }

    intended_actions
        .into_iter()
        .filter(|action_id| !state.has_actor_facility_grant(facility, *action_id))
        .map(|action_id| SearchCandidate {
            payload_override: Some(ActionPayload::QueueForFacilityUse(
                QueueForFacilityUsePayload {
                    intended_action: action_id,
                },
            )),
            ..base.clone()
        })
        .filter(|candidate| {
            registry
                .get(candidate.def_id)
                .is_some_and(|_| state.workstation_tag(facility) == Some(workstation_tag))
        })
        .collect()
}

fn queue_intended_actions_for(
    goal: &GroundedGoal,
    state: &PlanningState<'_>,
    registry: &ActionDefRegistry,
    facility: EntityId,
) -> Option<(worldwake_core::WorkstationTag, Vec<ActionDefId>)> {
    let workstation_tag = state.workstation_tag(facility)?;
    let actions = match goal.key.kind {
        GoalKind::ProduceCommodity { recipe_id } => registry
            .iter()
            .filter_map(|def| {
                let payload = def.payload.as_craft()?;
                (payload.recipe_id == recipe_id
                    && payload.required_workstation_tag == workstation_tag)
                    .then_some(def.id)
            })
            .collect::<Vec<_>>(),
        GoalKind::AcquireCommodity { commodity, .. }
        | GoalKind::ConsumeOwnedCommodity { commodity }
        | GoalKind::RestockCommodity { commodity } => registry
            .iter()
            .filter_map(|def| {
                if let Some(payload) = def.payload.as_harvest() {
                    return (payload.output_commodity == commodity
                        && payload.required_workstation_tag == workstation_tag)
                        .then_some(def.id);
                }
                def.payload.as_craft().and_then(|payload| {
                    (payload.required_workstation_tag == workstation_tag
                        && payload.outputs.iter().any(|(output, quantity)| {
                            *output == commodity && *quantity > worldwake_core::Quantity(0)
                        }))
                    .then_some(def.id)
                })
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    (!actions.is_empty()).then_some((workstation_tag, actions))
}

pub(super) fn search_candidate_from_planner(
    candidate: crate::planner_ops::PlannerSyntheticCandidate,
) -> SearchCandidate {
    SearchCandidate {
        def_id: candidate.def_id,
        authoritative_targets: Vec::new(),
        planning_targets: candidate.targets,
        payload_override: candidate.payload_override,
        planner_only: true,
        trace_index: None,
    }
}

pub(super) fn unsupported_goal(goal: &GoalKind) -> bool {
    matches!(goal, GoalKind::SellCommodity { .. })
}

/// Returns `Some((place, blocking_fact))` if the candidate is blocked by a
/// place-scoped blocker, `None` otherwise.
fn candidate_blocked_by_place(
    candidate: &SearchCandidate,
    goal: &GroundedGoal,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    blocked: &BlockedIntentMemory,
    current_tick: Tick,
) -> Option<(Option<EntityId>, worldwake_core::BlockingFact)> {
    let place = candidate_action_place(candidate, node, semantics_table);
    let target = candidate.authoritative_targets.first().copied();
    let intent = blocked.find_blocked_for_search(
        &goal.key,
        place,
        target,
        Some(candidate.def_id),
        current_tick,
    )?;
    Some((place, intent.blocking_fact))
}

/// Resolves the place where a candidate action would execute.
///
/// Travel actions use the destination (target place) as their action place.
/// All other actions use the actor's current simulated place.
fn candidate_action_place(
    candidate: &SearchCandidate,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) -> Option<EntityId> {
    let semantics = semantics_table.get(&candidate.def_id)?;
    match semantics.op_kind {
        PlannerOpKind::Travel => candidate.authoritative_targets.first().copied(),
        _ => node
            .state
            .effective_place_ref(PlanningEntityRef::Authoritative(
                node.state.snapshot().actor(),
            )),
    }
}
