use crate::goal_model::{
    RootCandidateSynthesis, grounded_goal_allows_local_epistemic_resolution,
    grounded_goal_epistemic_subjects, grounded_goal_matches_epistemic_barrier,
};
use crate::planner_ops::{PlannerOpKind, planner_only_candidates};
use crate::{GoalKindPlannerExt, GoalOffer, PlannerOpSemantics, PlanningEntityRef, PlanningState};
use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    ActionDefId, BlockerMemory, ContentionStatus, EntityId, GoalKind, OmissionReason,
    OpportunityAnchor, Tick,
};
use worldwake_sim::{
    ActionDefRegistry, ActionHandlerRegistry, ActionPayload, Affordance, EconomicBeliefView,
    FacilityBeliefView, InventoryBeliefView, QueueForFacilityUsePayload, RecipeRegistry,
    SpatialBeliefView, get_affordances_for_defs,
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
    pub(super) expansion_trace_index: Option<usize>,
}

pub(super) struct CandidateTraceSinks<'a> {
    pub(super) binding_rejections: Option<&'a mut Vec<crate::decision_trace::BindingRejection>>,
    pub(super) expansion_candidates:
        Option<&'a mut Vec<crate::decision_trace::ExpansionCandidateTrace>>,
    pub(super) root_candidates: Option<&'a mut Vec<crate::decision_trace::RootCandidateTrace>>,
    pub(super) root_omissions:
        Option<&'a mut Vec<crate::decision_trace::RootOperatorOmissionTrace>>,
}

pub(super) struct CandidateFilterTraceSinks<'a> {
    pub(super) expansion_candidates:
        Option<&'a mut Vec<crate::decision_trace::ExpansionCandidateTrace>>,
    pub(super) root_candidates: Option<&'a mut Vec<crate::decision_trace::RootCandidateTrace>>,
}

#[derive(Clone, Copy)]
pub(super) struct CandidateSearchContext<'a> {
    pub(super) semantics_table: &'a BTreeMap<ActionDefId, PlannerOpSemantics>,
    pub(super) registry: &'a ActionDefRegistry,
    pub(super) handlers: &'a ActionHandlerRegistry,
    pub(super) blocked: &'a BlockerMemory,
    pub(super) current_tick: Tick,
    pub(super) relevant_defs: &'a BTreeSet<ActionDefId>,
    pub(super) candidate_source: crate::decision_trace::CandidateSource,
}

#[derive(Clone, Copy)]
pub(super) struct CommodityFilterContext<'a> {
    pub(super) tactical_goal: Option<&'a super::TacticalGoal>,
    pub(super) semantics_table: &'a BTreeMap<ActionDefId, PlannerOpSemantics>,
    pub(super) registry: &'a ActionDefRegistry,
    pub(super) recipes: &'a RecipeRegistry,
}

pub(super) fn relevant_action_defs(
    goal: &GoalOffer,
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

pub(super) fn push_expansion_candidate_trace(
    sink: &mut Option<&mut Vec<crate::decision_trace::ExpansionCandidateTrace>>,
    trace: crate::decision_trace::ExpansionCandidateTrace,
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

pub(super) fn update_expansion_candidate_outcome(
    sink: &mut Option<&mut Vec<crate::decision_trace::ExpansionCandidateTrace>>,
    trace_index: Option<usize>,
    outcome: crate::decision_trace::ExpansionCandidateOutcome,
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
    omitted_anchor: Option<OmissionReason>,
    source: crate::decision_trace::CandidateSource,
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
        omitted_anchor,
        source,
    }
}

pub(super) fn expansion_candidate_trace_from_candidate(
    candidate: &SearchCandidate,
    registry: &ActionDefRegistry,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) -> crate::decision_trace::ExpansionCandidateTrace {
    crate::decision_trace::ExpansionCandidateTrace {
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
        outcome: crate::decision_trace::ExpansionCandidateOutcome::Skipped(
            crate::decision_trace::RootCandidateSkipReason::MissingSemantics,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(super) fn search_candidates(
    goal: &GoalOffer,
    node: &SearchNode<'_>,
    context: CandidateSearchContext<'_>,
    binding_rejections: Option<&mut Vec<crate::decision_trace::BindingRejection>>,
    root_candidates: Option<&mut Vec<crate::decision_trace::RootCandidateTrace>>,
    root_omissions: Option<&mut Vec<crate::decision_trace::RootOperatorOmissionTrace>>,
) -> Vec<SearchCandidate> {
    search_candidates_with_expansion_trace(
        goal,
        node,
        context,
        CandidateTraceSinks {
            binding_rejections,
            expansion_candidates: None,
            root_candidates,
            root_omissions,
        },
    )
}

pub(super) fn search_candidates_with_expansion_trace(
    goal: &GoalOffer,
    node: &SearchNode<'_>,
    context: CandidateSearchContext<'_>,
    trace_sinks: CandidateTraceSinks<'_>,
) -> Vec<SearchCandidate> {
    let CandidateSearchContext {
        semantics_table,
        registry,
        handlers,
        blocked,
        current_tick,
        relevant_defs,
        candidate_source,
    } = context;
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
        search_candidates_from_affordance(goal, &node.state, registry, handlers, &affordance)
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
                            &epistemic_subjects,
                            semantics.op_kind,
                            &candidate.authoritative_targets,
                            candidate.payload_override.as_ref(),
                        )
                        || grounded_goal_allows_local_epistemic_resolution(
                            goal,
                            semantics.op_kind,
                            &candidate.authoritative_targets,
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
        &node.state,
        registry,
        semantics_table,
        relevant_defs,
        &candidates,
    ));
    let CandidateTraceSinks {
        binding_rejections,
        expansion_candidates,
        root_candidates,
        root_omissions,
    } = trace_sinks;
    let mut root_omissions = root_omissions;
    if let Some(root_omissions) = root_omissions.as_mut() {
        record_root_operator_omissions(
            goal,
            &node.state,
            registry,
            semantics_table,
            &candidates,
            Some(&mut **root_omissions),
        );
        if let Some(trace) = ask_witness_omission {
            root_omissions.push(trace);
        }
    } else {
        record_root_operator_omissions(
            goal,
            &node.state,
            registry,
            semantics_table,
            &candidates,
            None,
        );
    }
    let mut root_candidates = root_candidates;
    let mut expansion_candidates = expansion_candidates;
    let mut binding_rejections = binding_rejections;
    let mut filtered = Vec::with_capacity(candidates.len());

    for mut candidate in candidates {
        let expansion_trace_index = push_expansion_candidate_trace(
            &mut expansion_candidates,
            expansion_candidate_trace_from_candidate(&candidate, registry, semantics_table),
        );
        let omitted_anchor = omitted_anchor_for_candidate(goal, &node.state, &candidate);
        let trace_index = push_root_candidate_trace(
            &mut root_candidates,
            root_candidate_trace_from_candidate(
                &candidate,
                registry,
                semantics_table,
                omitted_anchor,
                candidate_source,
            ),
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
            update_expansion_candidate_outcome(
                &mut expansion_candidates,
                expansion_trace_index,
                crate::decision_trace::ExpansionCandidateOutcome::Filtered(
                    crate::decision_trace::ExpansionCandidateFilterReason::BlockedFacilityUse {
                        facility,
                        intended_action,
                    },
                ),
            );
            continue;
        }

        let Some(semantics) = semantics_table.get(&candidate.def_id) else {
            candidate.trace_index = trace_index;
            candidate.expansion_trace_index = expansion_trace_index;
            filtered.push(candidate);
            continue;
        };
        let passes_binding = goal
            .key
            .kind
            .matches_binding(&candidate.authoritative_targets, semantics.op_kind);
        let passes_opportunity_anchor =
            candidate_matches_opportunity_anchor(goal, &candidate, semantics.op_kind);
        if !passes_binding || !passes_opportunity_anchor {
            let required_target = goal.key.entity.or(goal.key.place).or(match goal.anchor {
                OpportunityAnchor::Entity(entity) | OpportunityAnchor::Place(entity) => {
                    Some(entity)
                }
                OpportunityAnchor::None => None,
            });
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
            update_expansion_candidate_outcome(
                &mut expansion_candidates,
                expansion_trace_index,
                crate::decision_trace::ExpansionCandidateOutcome::Filtered(
                    crate::decision_trace::ExpansionCandidateFilterReason::BindingMismatch {
                        required_target,
                    },
                ),
            );
            continue;
        }

        if !goal
            .key
            .kind
            .candidate_is_available(&node.state, semantics.op_kind)
            || move_cargo_unavailable_for_seller_backed_sale_lot(
                goal,
                &candidate,
                semantics.op_kind,
                &node.state,
            )
        {
            update_root_candidate_outcome(
                &mut root_candidates,
                trace_index,
                crate::decision_trace::RootCandidateOutcome::Filtered(
                    crate::decision_trace::RootCandidateFilterReason::GoalUnavailable,
                ),
            );
            update_expansion_candidate_outcome(
                &mut expansion_candidates,
                expansion_trace_index,
                crate::decision_trace::ExpansionCandidateOutcome::Filtered(
                    crate::decision_trace::ExpansionCandidateFilterReason::GoalUnavailable,
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
            update_expansion_candidate_outcome(
                &mut expansion_candidates,
                expansion_trace_index,
                crate::decision_trace::ExpansionCandidateOutcome::Filtered(
                    crate::decision_trace::ExpansionCandidateFilterReason::PlaceBlocker {
                        place,
                        blocking_fact,
                    },
                ),
            );
            continue;
        }

        candidate.trace_index = trace_index;
        candidate.expansion_trace_index = expansion_trace_index;
        filtered.push(candidate);
    }
    filtered
}

fn omitted_anchor_for_candidate(
    goal: &GoalOffer,
    state: &PlanningState<'_>,
    candidate: &SearchCandidate,
) -> Option<OmissionReason> {
    let anchor = match goal.anchor {
        OpportunityAnchor::Entity(entity) => Some(entity),
        OpportunityAnchor::Place(_) | OpportunityAnchor::None => {
            candidate.authoritative_targets.first().copied()
        }
    }?;
    if state
        .entity_kind_ref(PlanningEntityRef::Authoritative(anchor))
        .is_some()
    {
        return None;
    }
    let actor = state.snapshot().actor();
    worldwake_sim::GoalBeliefView::observation_omission_log(state, actor)?
        .entries
        .iter()
        .rev()
        .find(|entry| entry.omitted_entity == anchor)
        .map(|entry| entry.reason)
}

fn candidate_matches_opportunity_anchor(
    goal: &GoalOffer,
    candidate: &SearchCandidate,
    op_kind: PlannerOpKind,
) -> bool {
    if goal.key.kind != GoalKind::Wash || op_kind != PlannerOpKind::Wash {
        return true;
    }

    match goal.anchor {
        OpportunityAnchor::Entity(anchor) => {
            candidate.authoritative_targets.first() == Some(&anchor)
        }
        OpportunityAnchor::Place(_) | OpportunityAnchor::None => true,
    }
}

fn move_cargo_unavailable_for_seller_backed_sale_lot(
    goal: &GoalOffer,
    candidate: &SearchCandidate,
    op_kind: PlannerOpKind,
    state: &PlanningState<'_>,
) -> bool {
    if op_kind != PlannerOpKind::MoveCargo {
        return false;
    }
    if !matches!(
        goal.key.kind,
        GoalKind::AcquireCommodity { .. }
            | GoalKind::ConsumeOwnedCommodity { .. }
            | GoalKind::RestockCommodity { .. }
            | GoalKind::TreatWounds { .. }
    ) {
        return false;
    }

    let Some(target) = candidate.authoritative_targets.first().copied() else {
        return false;
    };
    state
        .seller_for_sale_lot(target)
        .is_some_and(|seller| seller != state.snapshot().actor())
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub(super) fn apply_commodity_relevance_filter(
    candidates: &mut Vec<SearchCandidate>,
    goal: &GoalOffer,
    state: &PlanningState<'_>,
    context: CommodityFilterContext<'_>,
    root_candidates: Option<&mut Vec<crate::decision_trace::RootCandidateTrace>>,
) {
    apply_commodity_relevance_filter_with_expansion_trace(
        candidates,
        goal,
        state,
        context,
        CandidateFilterTraceSinks {
            expansion_candidates: None,
            root_candidates,
        },
    );
}

pub(super) fn apply_commodity_relevance_filter_with_expansion_trace(
    candidates: &mut Vec<SearchCandidate>,
    goal: &GoalOffer,
    state: &PlanningState<'_>,
    context: CommodityFilterContext<'_>,
    trace_sinks: CandidateFilterTraceSinks<'_>,
) {
    let CommodityFilterContext {
        tactical_goal,
        semantics_table,
        registry,
        recipes,
    } = context;
    let Some(goal_commodity) = tactical_goal
        .and_then(|goal| match goal {
            super::TacticalGoal::AcquirePrerequisite { commodity, .. }
            | super::TacticalGoal::SocialQuery { commodity, .. } => Some(*commodity),
            super::TacticalGoal::Explore { .. } | super::TacticalGoal::TravelToGoal { .. } => None,
        })
        .or_else(|| goal.key.kind.target_commodity(recipes))
    else {
        return;
    };

    let CandidateFilterTraceSinks {
        expansion_candidates,
        root_candidates,
    } = trace_sinks;
    let mut root_candidates = root_candidates;
    let mut expansion_candidates = expansion_candidates;
    candidates.retain(|candidate| {
        let Some(semantics) = semantics_table.get(&candidate.def_id) else {
            return true;
        };
        let (keep, candidate_commodity) = commodity_filter_outcome(
            candidate,
            semantics.op_kind,
            &goal.key.kind,
            state,
            registry,
            goal_commodity,
        );
        if !keep {
            update_root_candidate_outcome(
                &mut root_candidates,
                candidate.trace_index,
                crate::decision_trace::RootCandidateOutcome::Filtered(
                    crate::decision_trace::RootCandidateFilterReason::CommodityIrrelevant {
                        candidate_commodity,
                        goal_commodity,
                    },
                ),
            );
            update_expansion_candidate_outcome(
                &mut expansion_candidates,
                candidate.expansion_trace_index,
                crate::decision_trace::ExpansionCandidateOutcome::Filtered(
                    crate::decision_trace::ExpansionCandidateFilterReason::CommodityIrrelevant {
                        candidate_commodity,
                        goal_commodity,
                    },
                ),
            );
        }
        keep
    });
}

fn commodity_filter_outcome(
    candidate: &SearchCandidate,
    op_kind: PlannerOpKind,
    goal_kind: &worldwake_core::GoalKind,
    state: &PlanningState<'_>,
    registry: &ActionDefRegistry,
    goal_commodity: worldwake_core::CommodityKind,
) -> (bool, Option<worldwake_core::CommodityKind>) {
    match op_kind {
        PlannerOpKind::MoveCargo => {
            if registry
                .get(candidate.def_id)
                .is_some_and(|def| def.name == "put_down")
                && let Some(target) = candidate.authoritative_targets.first().copied()
            {
                let put_down_can_advance_goal = matches!(
                    goal_kind,
                    worldwake_core::GoalKind::ProduceCommodity { .. }
                        | worldwake_core::GoalKind::SellCommodity { .. }
                );
                return if let Some(candidate_commodity) = state.item_lot_commodity(target) {
                    (
                        put_down_can_advance_goal && candidate_commodity == goal_commodity,
                        Some(candidate_commodity),
                    )
                } else {
                    let candidate_commodity =
                        state.resource_source(target).map(|source| source.commodity);
                    (
                        put_down_can_advance_goal && candidate_commodity == Some(goal_commodity),
                        candidate_commodity,
                    )
                };
            }
            if registry
                .get(candidate.def_id)
                .is_some_and(|def| def.name == "pick_up")
                && let Some(target) = candidate.authoritative_targets.first().copied()
            {
                return match state.item_lot_commodity(target) {
                    Some(candidate_commodity) => {
                        let actor = PlanningEntityRef::Authoritative(state.snapshot().actor());
                        let target_ref = PlanningEntityRef::Authoritative(target);
                        let loose_ground_lot = state.direct_possessor(target).is_none()
                            && state.direct_container(target).is_none();
                        let colocated = state.effective_place_ref(actor)
                            == state.effective_place_ref(target_ref);
                        let lawful_pickup = state.can_control_ref(actor, target_ref);
                        let already_holds_self_consume_stock =
                            matches!(
                                goal_kind,
                                worldwake_core::GoalKind::AcquireCommodity {
                                    purpose: worldwake_core::CommodityPurpose::SelfConsume,
                                    ..
                                }
                            ) && direct_possession_quantity(state, candidate_commodity) > 0;
                        (
                            loose_ground_lot
                                && colocated
                                && !state.has_sale_listing(target)
                                && lawful_pickup
                                && !already_holds_self_consume_stock
                                && candidate_commodity == goal_commodity,
                            Some(candidate_commodity),
                        )
                    }
                    None => (
                        false,
                        state.resource_source(target).map(|source| source.commodity),
                    ),
                };
            }
            (true, None)
        }
        PlannerOpKind::Trade => match candidate
            .payload_override
            .as_ref()
            .and_then(ActionPayload::as_trade)
            .and_then(|payload| state.item_lot_commodity(payload.sale_lot))
        {
            Some(candidate_commodity) => (
                candidate_commodity == goal_commodity,
                Some(candidate_commodity),
            ),
            None => (true, None),
        },
        PlannerOpKind::StockManagement => {
            let target = candidate.planning_targets.first().copied().or_else(|| {
                candidate
                    .authoritative_targets
                    .first()
                    .copied()
                    .map(PlanningEntityRef::Authoritative)
            });
            let Some(target) = target else {
                return (false, None);
            };
            match state.item_lot_commodity_ref(target) {
                Some(candidate_commodity) => (
                    candidate_commodity == goal_commodity,
                    Some(candidate_commodity),
                ),
                None => (false, None),
            }
        }
        PlannerOpKind::QueueForFacilityUse => {
            let Some(intended_action) = candidate
                .payload_override
                .as_ref()
                .and_then(ActionPayload::as_queue_for_facility_use)
                .map(|payload| payload.intended_action)
            else {
                return (true, None);
            };
            let Some(payload) = registry.get(intended_action).map(|def| &def.payload) else {
                return (true, None);
            };
            payload_commodity_filter_outcome(payload, state, candidate, goal_commodity)
        }
        PlannerOpKind::Harvest | PlannerOpKind::Craft => {
            let Some(payload) = candidate
                .payload_override
                .as_ref()
                .or_else(|| registry.get(candidate.def_id).map(|def| &def.payload))
            else {
                return (true, None);
            };
            payload_commodity_filter_outcome(payload, state, candidate, goal_commodity)
        }
        _ => (true, None),
    }
}

fn payload_commodity_filter_outcome(
    payload: &ActionPayload,
    state: &PlanningState<'_>,
    candidate: &SearchCandidate,
    goal_commodity: worldwake_core::CommodityKind,
) -> (bool, Option<worldwake_core::CommodityKind>) {
    if let Some(harvest) = payload.as_harvest() {
        let candidate_commodity = harvest.output_commodity;
        return (
            candidate_commodity == goal_commodity,
            Some(candidate_commodity),
        );
    }

    if let Some(craft) = payload.as_craft() {
        let contains_goal = craft
            .inputs
            .iter()
            .chain(craft.outputs.iter())
            .any(|(commodity, _)| *commodity == goal_commodity);
        let candidate_commodity = craft
            .outputs
            .first()
            .or_else(|| craft.inputs.first())
            .map(|(commodity, _)| *commodity);
        return (contains_goal, candidate_commodity);
    }

    if let Some(target) = candidate.authoritative_targets.first().copied()
        && let Some(candidate_commodity) =
            state.resource_source(target).map(|source| source.commodity)
    {
        return (
            candidate_commodity == goal_commodity,
            Some(candidate_commodity),
        );
    }

    (true, None)
}

fn direct_possession_quantity(
    state: &PlanningState<'_>,
    commodity: worldwake_core::CommodityKind,
) -> u32 {
    let actor = PlanningEntityRef::Authoritative(state.snapshot().actor());
    state
        .direct_possessions_ref(actor)
        .into_iter()
        .filter(|entity| state.item_lot_commodity_ref(*entity) == Some(commodity))
        .map(|entity| state.commodity_quantity_ref(entity, commodity).0)
        .sum()
}

fn goal_synthesized_candidates(
    goal: &GoalOffer,
    state: &PlanningState<'_>,
    registry: &ActionDefRegistry,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    relevant_defs: &BTreeSet<ActionDefId>,
    existing_candidates: &[SearchCandidate],
) -> Vec<SearchCandidate> {
    let actor_place = state.effective_place(state.snapshot().actor());
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
            match goal.synthesized_root_candidate_targets(def, *semantics, actor_place) {
                RootCandidateSynthesis::Targets(authoritative_targets) => Some(SearchCandidate {
                    def_id: *def_id,
                    authoritative_targets: authoritative_targets.clone(),
                    planning_targets: synthesized_planning_targets(
                        goal,
                        state,
                        *semantics,
                        authoritative_targets,
                    ),
                    payload_override: None,
                    planner_only: false,
                    trace_index: None,
                    expansion_trace_index: None,
                }),
                RootCandidateSynthesis::NoSynthesisPath
                | RootCandidateSynthesis::UnsupportedGoalOp
                | RootCandidateSynthesis::TargetDerivationFailed => None,
            }
        })
        .collect::<Vec<_>>()
}

fn synthesized_planning_targets(
    goal: &GoalOffer,
    state: &PlanningState<'_>,
    semantics: PlannerOpSemantics,
    authoritative_targets: Vec<EntityId>,
) -> Vec<PlanningEntityRef> {
    match (&goal.key.kind, semantics.op_kind) {
        // Accusation payload binds to the accused entity, but the lawful
        // execution location is the crime register's home place.
        (GoalKind::Accuse { crime_register, .. }, PlannerOpKind::Accuse) => {
            state.record_data(*crime_register).map_or_else(
                || {
                    authoritative_targets
                        .into_iter()
                        .map(PlanningEntityRef::Authoritative)
                        .collect()
                },
                |record| vec![PlanningEntityRef::Authoritative(record.home_place)],
            )
        }
        _ => authoritative_targets
            .into_iter()
            .map(PlanningEntityRef::Authoritative)
            .collect(),
    }
}

fn record_root_operator_omissions(
    goal: &GoalOffer,
    state: &PlanningState<'_>,
    registry: &ActionDefRegistry,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    candidates: &[SearchCandidate],
    sink: Option<&mut Vec<crate::decision_trace::RootOperatorOmissionTrace>>,
) {
    let Some(sink) = sink else {
        return;
    };
    let actor_place = state.effective_place(state.snapshot().actor());
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
            match goal.synthesized_root_candidate_targets(def, *semantics, actor_place) {
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
    goal: &GoalOffer,
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

fn required_trade_counterparty(goal: &GoalOffer) -> Option<EntityId> {
    let seller = goal.evidence_entities.iter().copied().next()?;
    if goal.evidence_entities.len() != 1 {
        return None;
    }
    match goal.key.kind {
        GoalKind::AcquireCommodity { .. }
        | GoalKind::RestockCommodity { .. }
        | GoalKind::TreatWounds { .. } => Some(seller),
        _ => None,
    }
}

fn affordance_matches_grounded_opportunity(goal: &GoalOffer, affordance: &Affordance) -> bool {
    let Some(required_counterparty) = required_trade_counterparty(goal) else {
        return true;
    };
    let Some(trade) = affordance
        .payload_override
        .as_ref()
        .and_then(ActionPayload::as_trade)
    else {
        return true;
    };
    trade.counterparty == required_counterparty
}

pub(super) fn search_candidates_from_affordance(
    goal: &GoalOffer,
    state: &PlanningState<'_>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    affordance: &Affordance,
) -> Vec<SearchCandidate> {
    if !affordance_matches_grounded_opportunity(goal, affordance) {
        return Vec::new();
    }
    let Some(def) = registry.get(affordance.def_id) else {
        return vec![SearchCandidate {
            def_id: affordance.def_id,
            authoritative_targets: affordance.bound_targets.clone(),
            planning_targets: affordance
                .bound_targets
                .iter()
                .copied()
                .map(PlanningEntityRef::Authoritative)
                .collect(),
            payload_override: affordance.payload_override.clone(),
            planner_only: false,
            trace_index: None,
            expansion_trace_index: None,
        }];
    };
    let planning_targets = match (&goal.key.kind, def.name.as_str()) {
        // Accusation payload binds to the accused entity, but the lawful
        // execution location is the crime register's home place.
        (GoalKind::Accuse { crime_register, .. }, "accuse") => {
            state.record_data(*crime_register).map_or_else(
                || {
                    affordance
                        .bound_targets
                        .iter()
                        .copied()
                        .map(PlanningEntityRef::Authoritative)
                        .collect()
                },
                |record| vec![PlanningEntityRef::Authoritative(record.home_place)],
            )
        }
        _ => affordance
            .bound_targets
            .iter()
            .copied()
            .map(PlanningEntityRef::Authoritative)
            .collect(),
    };
    let base = SearchCandidate {
        def_id: affordance.def_id,
        authoritative_targets: affordance.bound_targets.clone(),
        planning_targets,
        payload_override: affordance.payload_override.clone(),
        planner_only: false,
        trace_index: None,
        expansion_trace_index: None,
    };
    if matches!(
        def.payload,
        ActionPayload::Harvest(_) | ActionPayload::Craft(_)
    ) && !matches!(
        affordance.contention_status,
        ContentionStatus::Unmanaged | ContentionStatus::Granted
    ) {
        return Vec::new();
    }
    if matches!(def.name.as_str(), "loot" | "bury" | "heal")
        && !matches!(
            affordance.contention_status,
            ContentionStatus::Unmanaged | ContentionStatus::Granted
        )
    {
        return Vec::new();
    }
    if !matches!(
        def.name.as_str(),
        "queue_for_facility_use" | "queue_for_corpse_use" | "queue_for_care_target"
    ) {
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
        .and_then(|entity| entity.temporal.facility_queue.as_ref())
        .is_none()
    {
        return Vec::new();
    }
    let Some(intended_actions) =
        queue_intended_actions_for(goal, state, registry, facility, def.name.as_str())
    else {
        return Vec::new();
    };
    if state.is_actor_queued_at_facility(facility) {
        return Vec::new();
    }

    let require_current_affordability = def.name == "queue_for_care_target";

    intended_actions
        .into_iter()
        .filter(|action_id| {
            !require_current_affordability
                || intended_action_is_currently_affordable(
                    goal, state, registry, handlers, *action_id,
                )
        })
        .filter(|action_id| !state.has_actor_facility_grant(facility, *action_id))
        .map(|action_id| SearchCandidate {
            payload_override: Some(ActionPayload::QueueForFacilityUse(
                QueueForFacilityUsePayload {
                    intended_action: action_id,
                },
            )),
            ..base.clone()
        })
        .collect()
}

fn intended_action_is_currently_affordable(
    goal: &GoalOffer,
    state: &PlanningState<'_>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    intended_action: ActionDefId,
) -> bool {
    let allowed_defs = BTreeSet::from([intended_action]);
    get_affordances_for_defs(
        state,
        state.snapshot().actor(),
        registry,
        handlers,
        &allowed_defs,
    )
    .into_iter()
    .any(|affordance| affordance_matches_grounded_opportunity(goal, &affordance))
}

fn queue_intended_actions_for(
    goal: &GoalOffer,
    state: &PlanningState<'_>,
    registry: &ActionDefRegistry,
    entity: EntityId,
    queue_action_name: &str,
) -> Option<Vec<ActionDefId>> {
    let actions = match queue_action_name {
        "queue_for_facility_use" => {
            let workstation_tag = state.workstation_tag(entity)?;
            match goal.key.kind {
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
            }
        }
        "queue_for_corpse_use" => match goal.key.kind {
            GoalKind::LootCorpse { corpse } if corpse == entity => registry
                .iter()
                .find(|def| def.name == "loot")
                .map(|def| vec![def.id])
                .unwrap_or_default(),
            GoalKind::BuryCorpse { corpse, .. } if corpse == entity => registry
                .iter()
                .find(|def| def.name == "bury")
                .map(|def| vec![def.id])
                .unwrap_or_default(),
            _ => Vec::new(),
        },
        "queue_for_care_target" => match goal.key.kind {
            GoalKind::TreatWounds { patient } if patient == entity => registry
                .iter()
                .find(|def| def.name == "heal")
                .map(|def| vec![def.id])
                .unwrap_or_default(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };

    (!actions.is_empty()).then_some(actions)
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
        expansion_trace_index: None,
    }
}

pub(super) fn unsupported_goal(_goal: &GoalKind) -> bool {
    false
}

/// Returns `Some((place, blocking_fact))` if the candidate is blocked by a
/// place-scoped blocker, `None` otherwise.
pub(super) fn candidate_blocked_by_place(
    candidate: &SearchCandidate,
    goal: &GoalOffer,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    blocked: &BlockerMemory,
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
pub(super) fn candidate_action_place(
    candidate: &SearchCandidate,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) -> Option<EntityId> {
    let semantics = semantics_table.get(&candidate.def_id)?;
    match semantics.op_kind {
        PlannerOpKind::Travel => candidate.authoritative_targets.first().copied(),
        PlannerOpKind::MoveCargo => candidate
            .authoritative_targets
            .first()
            .copied()
            .and_then(|target| {
                node.state
                    .effective_place_ref(PlanningEntityRef::Authoritative(target))
            })
            .or_else(|| {
                node.state
                    .effective_place_ref(PlanningEntityRef::Authoritative(
                        node.state.snapshot().actor(),
                    ))
            }),
        _ => node
            .state
            .effective_place_ref(PlanningEntityRef::Authoritative(
                node.state.snapshot().actor(),
            )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_candidate_trace_from_candidate_defaults_to_emitter_source() {
        let candidate = SearchCandidate {
            def_id: ActionDefId(0),
            authoritative_targets: vec![EntityId {
                slot: 7,
                generation: 0,
            }],
            planning_targets: Vec::new(),
            payload_override: None,
            planner_only: false,
            trace_index: None,
            expansion_trace_index: None,
        };
        let registry = ActionDefRegistry::new();
        let semantics_table = BTreeMap::new();

        let trace = root_candidate_trace_from_candidate(
            &candidate,
            &registry,
            &semantics_table,
            None,
            crate::decision_trace::CandidateSource::Emitter,
        );

        assert_eq!(
            trace.source,
            crate::decision_trace::CandidateSource::Emitter
        );
    }

    #[test]
    fn root_candidate_trace_preserves_opportunity_compiler_source() {
        let candidate = SearchCandidate {
            def_id: ActionDefId(0),
            authoritative_targets: vec![EntityId {
                slot: 7,
                generation: 0,
            }],
            planning_targets: Vec::new(),
            payload_override: None,
            planner_only: false,
            trace_index: None,
            expansion_trace_index: None,
        };
        let registry = ActionDefRegistry::new();
        let semantics_table = BTreeMap::new();

        let trace = root_candidate_trace_from_candidate(
            &candidate,
            &registry,
            &semantics_table,
            None,
            crate::decision_trace::CandidateSource::OpportunityCompiler,
        );

        assert_eq!(
            trace.source,
            crate::decision_trace::CandidateSource::OpportunityCompiler
        );
    }
}
