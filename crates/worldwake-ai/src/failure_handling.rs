use crate::{AgentDecisionRuntime, DirtySet, PlannedStep, PlannerOpKind, authoritative_target};
use worldwake_core::{
    AffordanceKey, Blocker, BlockerClearingCondition, BlockerKey, BlockerMemory, BlockingFact,
    ClearingBaseline, CognitiveProfile, CommodityKind, ContentionIntents, Discrepancy,
    DiscrepancyClearing, DiscrepancyEntry, DiscrepancyMemory, DiscrepancySource,
    EntityBeliefAspect, EntityId, GoalKey, GoalKind, IntentionFrame, Quantity, Tick,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionPayload, ActionStartFailure,
    ActionStartFailureReason, ExternalAbortReason, InterruptReason, ReplanNeeded,
    RuntimeBeliefView,
};

#[derive(Clone, Copy)]
pub enum ExecutionFailure<'a> {
    Replan(&'a ReplanNeeded),
    Start(&'a ActionStartFailure),
}

pub struct PlanFailureContext<'a> {
    pub view: &'a dyn RuntimeBeliefView,
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub failed_step: &'a PlannedStep,
    pub method_id: Option<worldwake_core::MethodSchemaId>,
    pub execution_failure: Option<ExecutionFailure<'a>>,
    pub belief_discrepancy: Option<Discrepancy>,
    pub current_tick: Tick,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureClassification {
    Blocker(BlockingFact),
    Discrepancy(Discrepancy),
}

pub fn handle_plan_failure(
    context: &PlanFailureContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    jc: &mut Option<IntentionFrame>,
    blocked_memory: &mut BlockerMemory,
    discrepancy_memory: &mut DiscrepancyMemory,
    facility_intents: &mut ContentionIntents,
    cognitive: &CognitiveProfile,
) -> FailureClassification {
    runtime.current_plan = None;
    if jc.is_some() {
        runtime.last_frame_clear_reason = Some(worldwake_core::FrameClearReason::PlanFailed);
    }
    *jc = None;
    runtime.materialization_bindings.clear();
    facility_intents.intents.clear();

    let classification = classify_discrepancy(
        context.view,
        context.agent,
        &context.goal_key,
        context.failed_step,
        context.method_id,
        context.execution_failure,
        context.belief_discrepancy,
    );
    record_failure_classification(
        context,
        classification,
        runtime,
        blocked_memory,
        discrepancy_memory,
        cognitive,
    )
}

pub fn clear_resolved_failures(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    blocked_memory: &mut BlockerMemory,
    discrepancy_memory: &mut DiscrepancyMemory,
    current_tick: Tick,
) {
    blocked_memory.expire(current_tick);
    blocked_memory.sweep_cleared(|intent| is_blocker_cleared(view, agent, intent));
    discrepancy_memory.expire(current_tick);
    discrepancy_memory.clear_by_condition(|entry| is_discrepancy_cleared(view, agent, entry));
}

pub(crate) fn classify_discrepancy(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    goal_key: &GoalKey,
    step: &PlannedStep,
    method_id: Option<worldwake_core::MethodSchemaId>,
    execution_failure: Option<ExecutionFailure<'_>>,
    belief_discrepancy: Option<Discrepancy>,
) -> FailureClassification {
    if let Some(discrepancy) = belief_discrepancy {
        return FailureClassification::Discrepancy(discrepancy);
    }

    if target_gone(view, agent, step) {
        return FailureClassification::Blocker(BlockingFact::TargetGone);
    }

    match step.op_kind {
        PlannerOpKind::Travel => {
            if no_known_path(view, agent, step) {
                return FailureClassification::Blocker(BlockingFact::NoKnownPath);
            }
        }
        PlannerOpKind::Trade | PlannerOpKind::StaffMarket | PlannerOpKind::StockManagement => {
            if let Some(fact) =
                classify_trade_failure(view, agent, goal_key, step, execution_failure)
            {
                return FailureClassification::Blocker(fact);
            }
        }
        PlannerOpKind::Harvest | PlannerOpKind::Craft => {
            if let Some(fact) = classify_production_failure(view, agent, step) {
                return FailureClassification::Blocker(fact);
            }
        }
        PlannerOpKind::Consume | PlannerOpKind::Heal => {
            if let Some(fact) = classify_input_failure(view, agent, goal_key, step) {
                return FailureClassification::Blocker(fact);
            }
        }
        PlannerOpKind::Attack | PlannerOpKind::Defend => {
            if combat_too_risky(view, agent) {
                return FailureClassification::Blocker(BlockingFact::CombatTooRisky);
            }
        }
        PlannerOpKind::Wash
        | PlannerOpKind::Patrol
        | PlannerOpKind::Sleep
        | PlannerOpKind::Relieve
        | PlannerOpKind::EstablishCamp
        | PlannerOpKind::QueueForFacilityUse
        | PlannerOpKind::MoveCargo
        | PlannerOpKind::DropItem
        | PlannerOpKind::Loot
        | PlannerOpKind::Bury
        | PlannerOpKind::Tell
        | PlannerOpKind::ConsultRecord
        | PlannerOpKind::Bribe
        | PlannerOpKind::Threaten
        | PlannerOpKind::Accuse
        | PlannerOpKind::Fine
        | PlannerOpKind::Exile
        | PlannerOpKind::DeclareSupport
        | PlannerOpKind::PressForceClaim
        | PlannerOpKind::YieldForceClaim
        | PlannerOpKind::Investigate
        | PlannerOpKind::AskWitness
        | PlannerOpKind::SearchPlace
        | PlannerOpKind::AskAboutPerson
        | PlannerOpKind::ReportMissing
        | PlannerOpKind::EscortToSafety
        | PlannerOpKind::ReportFound
        | PlannerOpKind::ClaimBounty
        | PlannerOpKind::WithdrawBounty
        | PlannerOpKind::PostBounty
        | PlannerOpKind::PostNotice => {}
    }

    if danger_too_high(view, agent) {
        return FailureClassification::Blocker(BlockingFact::DangerTooHigh);
    }

    if let Some(classification @ FailureClassification::Discrepancy(Discrepancy::NoLegalBinding)) =
        execution_failure.and_then(|failure| map_execution_failure(failure, step))
    {
        return classification;
    }

    if local_commodity_availability_contradicted(view, agent, goal_key, step) {
        return FailureClassification::Discrepancy(Discrepancy::BeliefContradicted);
    }

    if let Some(classification) =
        execution_failure.and_then(|failure| map_execution_failure(failure, step))
    {
        return classification;
    }

    if let Some(method_id) = method_id {
        return FailureClassification::Discrepancy(Discrepancy::MethodFailure(
            worldwake_core::MethodFailureContext {
                method_id,
                kind: worldwake_core::MethodFailureKind::SubgoalUnachievable,
                subgoal_index: None,
            },
        ));
    }

    FailureClassification::Discrepancy(Discrepancy::ImproperPlanningState)
}

pub(crate) fn record_failure_classification(
    context: &PlanFailureContext<'_>,
    classification: FailureClassification,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockerMemory,
    discrepancy_memory: &mut DiscrepancyMemory,
    cognitive: &CognitiveProfile,
) -> FailureClassification {
    let mut blocker_key = BlockerKey {
        goal_key: context.goal_key,
        place: related_place(
            context.view,
            context.agent,
            &context.goal_key,
            context.failed_step,
        ),
        target: related_entity(context.failed_step),
        action_def: Some(context.failed_step.def_id),
    };

    // Place-scoping rewrites the target to `None` when the failure is
    // "this place has no source of the commodity at all." Contention
    // failures (the source exists but is currently busy or queued)
    // must keep the target so that `derive_clearing_condition` can
    // anchor the blocker on the contended facility's
    // `ResourceExtractionQueues`/`ContentionQueue` substrate. Without
    // this guard, harvest start failures at a single-source place are
    // scoped to the place, lose facility identity, and can only expire
    // via TTL.
    if !is_contention_blocker(&classification)
        && !matches!(
            classification,
            FailureClassification::Discrepancy(Discrepancy::NoLegalBinding)
        )
        && should_scope_local_commodity_unavailability_to_place(
            context.view,
            context.agent,
            &blocker_key,
            context.failed_step,
        )
    {
        blocker_key.target = None;
    }

    let recorded = match classification {
        FailureClassification::Blocker(blocking_fact) => {
            let expires_tick =
                context.current_tick + u64::from(blocking_fact_ttl(blocking_fact, cognitive));
            let (clearing_condition, baseline_snapshot) =
                derive_clearing_condition(context.view, context.agent, blocking_fact, &blocker_key);
            let scope = blocker_key.into();
            blocked_memory.record(Blocker {
                scope,
                blocking_fact,
                diagnostic_context: None,
                observed_tick: context.current_tick,
                expires_tick,
                clearing_condition: BlockerClearingCondition::for_scope_and_fact(
                    scope,
                    blocking_fact,
                    clearing_condition,
                ),
                baseline_snapshot,
                source_event: None,
            });
            FailureClassification::Blocker(blocking_fact)
        }
        FailureClassification::Discrepancy(discrepancy) => {
            let expires_tick =
                context.current_tick + u64::from(discrepancy_ttl(discrepancy, cognitive));
            discrepancy_memory.record(DiscrepancyEntry {
                scope: blocker_key.into(),
                discrepancy,
                observed_tick: context.current_tick,
                expires_tick,
                clearing_condition: derive_discrepancy_clearing(
                    discrepancy,
                    &blocker_key,
                    context.execution_failure,
                ),
                source: DiscrepancySource::ReadPhaseInference,
            });
            FailureClassification::Discrepancy(discrepancy)
        }
    };
    runtime.dirty.insert(DirtySet::REPLAN_SIGNAL);
    recorded
}

pub fn discrepancy_for_target_belief_status(
    status: worldwake_sim::belief_view::BeliefStatus,
) -> Option<Discrepancy> {
    match status {
        worldwake_sim::belief_view::BeliefStatus::Stale => Some(Discrepancy::BeliefStale),
        worldwake_sim::belief_view::BeliefStatus::Contradicted => {
            Some(Discrepancy::BeliefContradicted)
        }
        worldwake_sim::belief_view::BeliefStatus::Certain
        | worldwake_sim::belief_view::BeliefStatus::Probable
        | worldwake_sim::belief_view::BeliefStatus::Disputed => None,
    }
}

pub fn exact_target_belief_discrepancy(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    step: &PlannedStep,
) -> Option<Discrepancy> {
    let target = related_entity(step)?;
    let envelope = view.believed_target_location(agent, target);
    discrepancy_for_target_belief_status(envelope.status)
}

fn classify_trade_failure(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    goal_key: &GoalKey,
    step: &PlannedStep,
    execution_failure: Option<ExecutionFailure<'_>>,
) -> Option<BlockingFact> {
    let payload = step.payload_override.as_ref()?.as_trade()?;
    let sale_lot_commodity = view.item_lot_commodity(payload.sale_lot);
    let commodity = goal_key
        .commodity
        .or(sale_lot_commodity)
        .unwrap_or(CommodityKind::Coin);
    let place = view.effective_place(agent)?;

    if let Some(fact) =
        classify_trade_execution_failure(agent, sale_lot_commodity, payload, execution_failure)
    {
        return Some(fact);
    }

    if sale_lot_commodity.is_some_and(|c| {
        view.commodity_quantity(payload.counterparty, c) < payload.requested_quantity
    }) {
        return Some(BlockingFact::SellerOutOfStock);
    }

    if view.commodity_quantity(agent, payload.offered_commodity) < payload.offered_quantity {
        return Some(if payload.offered_commodity == CommodityKind::Coin {
            BlockingFact::TooExpensive
        } else {
            BlockingFact::MissingInput(payload.offered_commodity)
        });
    }

    let has_other_seller = view
        .listed_sale_lots_at(place, commodity)
        .into_iter()
        .any(|lot| {
            view.seller_for_sale_lot(lot)
                .is_some_and(|seller| seller != agent)
        });

    if !has_other_seller {
        return Some(BlockingFact::NoKnownSeller);
    }

    None
}

fn classify_trade_execution_failure(
    agent: EntityId,
    sale_lot_commodity: Option<CommodityKind>,
    payload: &worldwake_sim::TradeActionPayload,
    execution_failure: Option<ExecutionFailure<'_>>,
) -> Option<BlockingFact> {
    match execution_failure? {
        ExecutionFailure::Start(failure) => {
            classify_trade_start_failure_reason(agent, sale_lot_commodity, payload, &failure.reason)
        }
        ExecutionFailure::Replan(signal) => {
            classify_trade_abort_reason(agent, sale_lot_commodity, payload, &signal.reason)
        }
    }
}

fn classify_trade_start_failure_reason(
    agent: EntityId,
    sale_lot_commodity: Option<CommodityKind>,
    payload: &worldwake_sim::TradeActionPayload,
    reason: &ActionStartFailureReason,
) -> Option<BlockingFact> {
    match reason {
        ActionStartFailureReason::AbortRequested(reason) => {
            classify_trade_handler_abort_reason(agent, sale_lot_commodity, payload, reason)
        }
        _ => None,
    }
}

fn classify_trade_abort_reason(
    agent: EntityId,
    sale_lot_commodity: Option<CommodityKind>,
    payload: &worldwake_sim::TradeActionPayload,
    reason: &AbortReason,
) -> Option<BlockingFact> {
    match reason {
        AbortReason::ExternalAbort {
            kind: ExternalAbortReason::HandlerRequested { reason },
            ..
        } => classify_trade_handler_abort_reason(agent, sale_lot_commodity, payload, reason),
        _ => None,
    }
}

fn classify_trade_handler_abort_reason(
    agent: EntityId,
    sale_lot_commodity: Option<CommodityKind>,
    payload: &worldwake_sim::TradeActionPayload,
    reason: &ActionAbortRequestReason,
) -> Option<BlockingFact> {
    match reason {
        ActionAbortRequestReason::HolderLacksAccessibleCommodity {
            holder, commodity, ..
        } if *holder == payload.counterparty
            && sale_lot_commodity.is_some_and(|c| *commodity == c) =>
        {
            Some(BlockingFact::SellerOutOfStock)
        }
        ActionAbortRequestReason::HolderLacksAccessibleCommodity {
            holder, commodity, ..
        } if *holder == agent && *commodity == payload.offered_commodity => {
            Some(if *commodity == CommodityKind::Coin {
                BlockingFact::TooExpensive
            } else {
                BlockingFact::MissingInput(*commodity)
            })
        }
        ActionAbortRequestReason::TradeBundleRejected { acceptance, .. } => match acceptance {
            worldwake_sim::TradeAcceptance::Accept => None,
            worldwake_sim::TradeAcceptance::Reject { reason } => match reason {
                worldwake_sim::TradeRejectionReason::InsufficientPayment
                | worldwake_sim::TradeRejectionReason::PostTradeStateWorse => {
                    Some(BlockingFact::TooExpensive)
                }
                worldwake_sim::TradeRejectionReason::NoNeed => Some(BlockingFact::NoKnownSeller),
            },
        },
        _ => None,
    }
}

/// True when the classification represents a contention failure on a
/// concrete facility (the source/workstation exists and has the resource,
/// but is currently being held or queued for by another actor). Such
/// blockers must retain their target so that the contention substrate
/// (`ResourceExtractionQueues` for harvest sources, `ContentionQueue`
/// for legacy workstations) can drive clearing.
fn is_contention_blocker(classification: &FailureClassification) -> bool {
    matches!(
        classification,
        FailureClassification::Blocker(
            BlockingFact::ReservationConflict { .. }
                | BlockingFact::ExclusiveFacilityUnavailable
                | BlockingFact::WorkstationBusy,
        )
    )
}

fn reservation_conflict_for(affordance: AffordanceKey) -> BlockingFact {
    BlockingFact::ReservationConflict {
        affordance,
        contention_event: None,
    }
}

fn step_affordance(step: &PlannedStep) -> Option<AffordanceKey> {
    step.targets
        .first()
        .copied()
        .and_then(authoritative_target)
        .map(|facility| AffordanceKey {
            facility,
            action: step.def_id,
        })
}

fn should_scope_local_commodity_unavailability_to_place(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    blocker_key: &BlockerKey,
    step: &PlannedStep,
) -> bool {
    if !matches!(
        blocker_key.goal_key.kind,
        GoalKind::AcquireCommodity { .. } | GoalKind::RestockCommodity { .. }
    ) {
        return false;
    }

    if !matches!(
        step.op_kind,
        PlannerOpKind::MoveCargo | PlannerOpKind::Trade | PlannerOpKind::Harvest
    ) {
        return false;
    }

    let Some(place) = blocker_key.place else {
        return false;
    };
    let Some(commodity) = blocker_key.goal_key.commodity else {
        return false;
    };
    if view.effective_place(agent) != Some(place) {
        return false;
    }

    !place_has_local_commodity_support(view, agent, place, commodity, blocker_key.target)
}

fn local_commodity_availability_contradicted(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    goal_key: &GoalKey,
    step: &PlannedStep,
) -> bool {
    let blocker_key = BlockerKey {
        goal_key: *goal_key,
        place: related_place(view, agent, goal_key, step),
        target: related_entity(step),
        action_def: Some(step.def_id),
    };
    should_scope_local_commodity_unavailability_to_place(view, agent, &blocker_key, step)
}

fn classify_production_failure(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    step: &PlannedStep,
) -> Option<BlockingFact> {
    if let Some(payload) = step
        .payload_override
        .as_ref()
        .and_then(ActionPayload::as_harvest)
        && let Some(missing_tool) = payload
            .required_tool_kinds
            .iter()
            .copied()
            .find(|tool| view.unique_item_count(agent, *tool) == 0)
    {
        return Some(BlockingFact::MissingTool(missing_tool));
    }

    if let Some(payload) = step
        .payload_override
        .as_ref()
        .and_then(ActionPayload::as_craft)
    {
        if let Some(missing_tool) = payload
            .required_tool_kinds
            .iter()
            .copied()
            .find(|tool| view.unique_item_count(agent, *tool) == 0)
        {
            return Some(BlockingFact::MissingTool(missing_tool));
        }

        if let Some((commodity, _)) = payload
            .inputs
            .iter()
            .find(|(commodity, quantity)| view.commodity_quantity(agent, *commodity) < *quantity)
        {
            return Some(BlockingFact::MissingInput(*commodity));
        }
    }

    let workstation = step
        .targets
        .first()
        .copied()
        .and_then(authoritative_target)?;
    if view.has_production_job(workstation) {
        return Some(BlockingFact::WorkstationBusy);
    }
    if !view.reservation_ranges(workstation).is_empty() {
        return Some(reservation_conflict_for(AffordanceKey {
            facility: workstation,
            action: step.def_id,
        }));
    }
    // Multi-slot harvest start: every slot has a foreign grant. The agent
    // is enqueued by `record_harvest_start_failure` and replans; classify
    // as a contention conflict (matches the pre-multi-slot single-slot
    // temporal-reservation semantics).
    if view
        .resource_extraction_queues(workstation)
        .is_some_and(|queues| {
            !queues.queues.is_empty()
                && queues
                    .queues
                    .iter()
                    .all(|queue| queue.granted.as_ref().is_some_and(|g| g.actor != agent))
        })
    {
        return Some(reservation_conflict_for(AffordanceKey {
            facility: workstation,
            action: step.def_id,
        }));
    }
    if view
        .resource_source(workstation)
        .is_some_and(|source| source.available_quantity == Quantity(0))
    {
        return Some(BlockingFact::SourceDepleted);
    }

    None
}

fn classify_input_failure(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    goal_key: &GoalKey,
    step: &PlannedStep,
) -> Option<BlockingFact> {
    let commodity = match step.op_kind {
        PlannerOpKind::Heal => Some(CommodityKind::Medicine),
        PlannerOpKind::Consume => goal_key.commodity,
        PlannerOpKind::Travel
        | PlannerOpKind::Patrol
        | PlannerOpKind::Sleep
        | PlannerOpKind::Relieve
        | PlannerOpKind::Trade
        | PlannerOpKind::StaffMarket
        | PlannerOpKind::StockManagement
        | PlannerOpKind::EstablishCamp
        | PlannerOpKind::QueueForFacilityUse
        | PlannerOpKind::Harvest
        | PlannerOpKind::Craft
        | PlannerOpKind::MoveCargo
        | PlannerOpKind::DropItem
        | PlannerOpKind::Loot
        | PlannerOpKind::Bury
        | PlannerOpKind::Tell
        | PlannerOpKind::ConsultRecord
        | PlannerOpKind::Attack
        | PlannerOpKind::Defend
        | PlannerOpKind::Bribe
        | PlannerOpKind::Threaten
        | PlannerOpKind::Accuse
        | PlannerOpKind::Fine
        | PlannerOpKind::Exile
        | PlannerOpKind::DeclareSupport
        | PlannerOpKind::PressForceClaim
        | PlannerOpKind::YieldForceClaim
        | PlannerOpKind::Investigate
        | PlannerOpKind::AskWitness
        | PlannerOpKind::SearchPlace
        | PlannerOpKind::AskAboutPerson
        | PlannerOpKind::ReportMissing
        | PlannerOpKind::EscortToSafety
        | PlannerOpKind::ReportFound
        | PlannerOpKind::ClaimBounty
        | PlannerOpKind::WithdrawBounty
        | PlannerOpKind::PostBounty
        | PlannerOpKind::PostNotice
        | PlannerOpKind::Wash => None,
    }?;

    (view.commodity_quantity(agent, commodity) == Quantity(0))
        .then_some(BlockingFact::MissingInput(commodity))
}

fn target_gone(view: &dyn RuntimeBeliefView, agent: EntityId, step: &PlannedStep) -> bool {
    if matches!(step.op_kind, PlannerOpKind::Travel | PlannerOpKind::Patrol) {
        return false;
    }

    let Some(target) = related_entity(step) else {
        return false;
    };

    match step.op_kind {
        PlannerOpKind::Trade
        | PlannerOpKind::StaffMarket
        | PlannerOpKind::StockManagement
        | PlannerOpKind::EstablishCamp
        | PlannerOpKind::QueueForFacilityUse
        | PlannerOpKind::MoveCargo
        | PlannerOpKind::DropItem
        | PlannerOpKind::Loot
        | PlannerOpKind::Bury
        | PlannerOpKind::Harvest
        | PlannerOpKind::Craft
        | PlannerOpKind::ClaimBounty
        | PlannerOpKind::WithdrawBounty
        | PlannerOpKind::PostBounty
        | PlannerOpKind::PostNotice => view.entity_kind(target).is_none(),
        PlannerOpKind::Attack | PlannerOpKind::Defend => {
            if view.entity_kind(target).is_none() || view.is_dead(target) {
                return true;
            }
            // Pursuit arrival failure: target is alive but not co-located.
            // This covers the case where a pursuer arrives at the believed
            // place and the target has moved elsewhere.
            let actor_place = view.effective_place(agent);
            let target_place = view.effective_place(target);
            actor_place.is_some() && target_place != actor_place
        }
        PlannerOpKind::Consume
        | PlannerOpKind::Sleep
        | PlannerOpKind::Relieve
        | PlannerOpKind::Wash
        | PlannerOpKind::Heal
        | PlannerOpKind::Tell
        | PlannerOpKind::ConsultRecord
        | PlannerOpKind::Bribe
        | PlannerOpKind::Threaten
        | PlannerOpKind::Accuse
        | PlannerOpKind::Fine
        | PlannerOpKind::Exile
        | PlannerOpKind::DeclareSupport
        | PlannerOpKind::PressForceClaim
        | PlannerOpKind::YieldForceClaim
        | PlannerOpKind::Investigate
        | PlannerOpKind::AskWitness
        | PlannerOpKind::SearchPlace
        | PlannerOpKind::AskAboutPerson
        | PlannerOpKind::ReportMissing
        | PlannerOpKind::EscortToSafety
        | PlannerOpKind::ReportFound => view.entity_kind(target).is_none() || view.is_dead(target),
        PlannerOpKind::Travel | PlannerOpKind::Patrol => false,
    }
}

fn no_known_path(view: &dyn RuntimeBeliefView, agent: EntityId, step: &PlannedStep) -> bool {
    let Some(current_place) = view.effective_place(agent) else {
        return false;
    };
    let Some(target_place) = step.targets.first().copied().and_then(authoritative_target) else {
        return false;
    };

    !view
        .adjacent_places_with_travel_ticks(current_place)
        .into_iter()
        .any(|(adjacent, _)| adjacent == target_place)
}

fn danger_too_high(view: &dyn RuntimeBeliefView, agent: EntityId) -> bool {
    !view.current_attackers_of(agent).is_empty() && !view.has_wounds(agent)
}

fn combat_too_risky(view: &dyn RuntimeBeliefView, agent: EntityId) -> bool {
    !view.current_attackers_of(agent).is_empty()
        || (!view.visible_hostiles_for(agent).is_empty() && view.has_wounds(agent))
}

fn map_execution_failure(
    failure: ExecutionFailure<'_>,
    step: &PlannedStep,
) -> Option<FailureClassification> {
    match failure {
        ExecutionFailure::Replan(signal) => map_replan_abort_reason(signal, step),
        ExecutionFailure::Start(failure) => map_start_failure_reason(failure, step),
    }
}

fn map_replan_abort_reason(
    signal: &ReplanNeeded,
    step: &PlannedStep,
) -> Option<FailureClassification> {
    match &signal.reason {
        AbortReason::CommitConditionFailed { condition } => match condition {
            worldwake_sim::Precondition::TargetAdjacentToActor(_) => {
                Some(FailureClassification::Blocker(BlockingFact::NoKnownPath))
            }
            worldwake_sim::Precondition::TargetLacksProductionJob(_) => Some(
                FailureClassification::Blocker(BlockingFact::WorkstationBusy),
            ),
            worldwake_sim::Precondition::TargetHasResourceSource { .. } => {
                Some(FailureClassification::Blocker(BlockingFact::SourceDepleted))
            }
            _ => None,
        },
        AbortReason::Interrupted { kind, detail } => match kind {
            InterruptReason::DangerNearby => {
                Some(FailureClassification::Blocker(BlockingFact::DangerTooHigh))
            }
            InterruptReason::Reprioritized => None,
            InterruptReason::Other => detail
                .as_deref()
                .and_then(|detail| parse_abort_detail(detail, step)),
        },
        AbortReason::ExternalAbort { kind, detail } => match kind {
            ExternalAbortReason::TargetDestroyed => {
                Some(FailureClassification::Blocker(BlockingFact::TargetGone))
            }
            ExternalAbortReason::ActorMarkedDead | ExternalAbortReason::CancelledByInput { .. } => {
                None
            }
            ExternalAbortReason::HandlerRequested { reason } => map_handler_abort_reason(reason),
            ExternalAbortReason::Other => detail
                .as_deref()
                .and_then(|detail| parse_abort_detail(detail, step)),
        },
        // A commit-time authoritative re-validation failure carries the same
        // `PreconditionFailed`-style detail a start-time failure would, so
        // classify it identically (see `map_start_failure_reason`).
        AbortReason::AuthoritativeRevalidationFailed { detail } => {
            classify_precondition_failure_detail(detail, step)
                .or_else(|| parse_abort_detail(detail, step))
        }
    }
}

fn map_start_failure_reason(
    failure: &ActionStartFailure,
    step: &PlannedStep,
) -> Option<FailureClassification> {
    match &failure.reason {
        ActionStartFailureReason::ReservationUnavailable(facility) => Some(
            FailureClassification::Blocker(reservation_conflict_for(AffordanceKey {
                facility: *facility,
                action: failure.def_id,
            })),
        ),
        ActionStartFailureReason::PreconditionFailed(detail) => {
            classify_precondition_failure_detail(detail, step)
                .or_else(|| parse_abort_detail(detail, step))
        }
        ActionStartFailureReason::InvalidTarget(_) => {
            Some(FailureClassification::Blocker(BlockingFact::TargetGone))
        }
        ActionStartFailureReason::AbortRequested(reason) => map_handler_abort_reason(reason),
    }
}

fn classify_precondition_failure_detail(
    detail: &str,
    step: &PlannedStep,
) -> Option<FailureClassification> {
    let detail = detail.to_ascii_lowercase();
    if detail.contains("exactidentityrequired") {
        return Some(FailureClassification::Discrepancy(
            Discrepancy::NoLegalBinding,
        ));
    }
    if detail.contains("targetunownedoractorcontrols") {
        return Some(FailureClassification::Discrepancy(
            Discrepancy::NoLegalBinding,
        ));
    }
    if detail.contains("targetatactorplace")
        || detail.contains("targetdirectlypossessedbyactor")
        || detail.contains("targetgrounded")
    {
        return Some(FailureClassification::Discrepancy(
            Discrepancy::ImproperPlanningState,
        ));
    }
    // Multi-slot harvest start: all extraction slots occupied. Semantically
    // equivalent to the old single-slot temporal-reservation conflict —
    // another agent currently holds the slot, the actor is enqueued by the
    // failure handler, and a replan is appropriate. Classify as
    // `ReservationConflict` so the blocker memory matches the prior
    // contention-conflict expiry/clearing semantics.
    if detail.contains("extraction_slots_full") {
        return step_affordance(step)
            .map(reservation_conflict_for)
            .map(FailureClassification::Blocker);
    }
    None
}

fn map_handler_abort_reason(reason: &ActionAbortRequestReason) -> Option<FailureClassification> {
    match reason {
        ActionAbortRequestReason::PayloadEntityMismatch { .. }
        | ActionAbortRequestReason::TargetNotColocated { .. }
        | ActionAbortRequestReason::TargetNotDead { .. }
        | ActionAbortRequestReason::TargetNotAlive { .. }
        | ActionAbortRequestReason::TargetIncapacitated { .. } => {
            Some(FailureClassification::Blocker(BlockingFact::TargetGone))
        }
        ActionAbortRequestReason::ActorAlreadyHasCombatStance { .. }
        | ActionAbortRequestReason::CommodityNotCombatWeapon { .. }
        | ActionAbortRequestReason::ActorMissingCombatProfile { .. }
        | ActionAbortRequestReason::TargetMissingCombatProfile { .. } => {
            Some(FailureClassification::Blocker(BlockingFact::CombatTooRisky))
        }
        ActionAbortRequestReason::ActorNotPlaced { .. } => {
            Some(FailureClassification::Blocker(BlockingFact::NoKnownPath))
        }
        ActionAbortRequestReason::TargetLacksWounds { .. }
        | ActionAbortRequestReason::TargetHasNoWounds { .. }
        | ActionAbortRequestReason::SelfTargetForbidden { .. } => Some(
            FailureClassification::Discrepancy(Discrepancy::NoLegalBinding),
        ),
        ActionAbortRequestReason::ActorMissingWeaponCommodity { commodity, .. }
        | ActionAbortRequestReason::HolderLacksAccessibleCommodity { commodity, .. } => Some(
            FailureClassification::Blocker(BlockingFact::MissingInput(*commodity)),
        ),
        ActionAbortRequestReason::TradeBundleRejected { acceptance, .. } => match acceptance {
            worldwake_sim::TradeAcceptance::Accept => None,
            worldwake_sim::TradeAcceptance::Reject { reason } => match reason {
                worldwake_sim::TradeRejectionReason::InsufficientPayment
                | worldwake_sim::TradeRejectionReason::PostTradeStateWorse => {
                    Some(FailureClassification::Blocker(BlockingFact::TooExpensive))
                }
                worldwake_sim::TradeRejectionReason::NoNeed => Some(
                    FailureClassification::Discrepancy(Discrepancy::NoWillingCounterparty),
                ),
            },
        },
        ActionAbortRequestReason::SaleLotNotListed { .. }
        | ActionAbortRequestReason::SaleLotNotPossessedBySeller { .. } => Some(
            FailureClassification::Blocker(BlockingFact::SellerOutOfStock),
        ),
        // ViolationNoLongerActive: justice/violation expired, no further classification.
        // HarvestSourceDepleted: mid-harvest source depletion is observable directly via
        // `available_quantity == 0` and `LastHarvestTrace` aftermath (FND-29A); the
        // replan loop reads the live world state, so no specific blocker classification
        // is needed at this layer.
        ActionAbortRequestReason::ViolationNoLongerActive { .. }
        | ActionAbortRequestReason::HarvestSourceDepleted { .. } => None,
    }
}

fn parse_abort_detail(detail: &str, step: &PlannedStep) -> Option<FailureClassification> {
    let detail = detail.to_ascii_lowercase();
    if detail.contains("danger") {
        Some(FailureClassification::Blocker(BlockingFact::DangerTooHigh))
    } else if detail.contains("risk") || detail.contains("combat") {
        Some(FailureClassification::Blocker(BlockingFact::CombatTooRisky))
    } else if detail.contains("reservation") {
        step_affordance(step)
            .map(reservation_conflict_for)
            .map(FailureClassification::Blocker)
    } else if detail.contains("seller") || detail.contains("stock") {
        Some(FailureClassification::Discrepancy(
            Discrepancy::NoWillingCounterparty,
        ))
    } else if detail.contains("path") {
        Some(FailureClassification::Blocker(BlockingFact::NoKnownPath))
    } else if detail.contains("route") {
        Some(FailureClassification::Discrepancy(
            Discrepancy::RouteUnknown,
        ))
    } else if detail.contains("destroyed") || detail.contains("gone") {
        Some(FailureClassification::Blocker(BlockingFact::TargetGone))
    } else if detail.contains("contention") || detail.contains("grant") || detail.contains("queue")
    {
        Some(FailureClassification::Blocker(
            BlockingFact::ExclusiveFacilityUnavailable,
        ))
    } else if detail.contains("budget") || detail.contains("exhaust") {
        Some(FailureClassification::Discrepancy(
            Discrepancy::SearchBudgetExhausted,
        ))
    } else {
        None
    }
}

fn derive_clearing_condition(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    blocking_fact: BlockingFact,
    blocker_key: &BlockerKey,
) -> (BlockerClearingCondition, Option<ClearingBaseline>) {
    match blocking_fact {
        BlockingFact::SellerOutOfStock => {
            let (Some(place), Some(seller)) = (blocker_key.place, blocker_key.target) else {
                return (BlockerClearingCondition::TtlOnly, None);
            };
            let Some(commodity) = blocker_key.goal_key.commodity else {
                return (BlockerClearingCondition::TtlOnly, None);
            };
            (
                BlockerClearingCondition::CommodityAvailabilityChanged { commodity, place },
                Some(ClearingBaseline::CommodityQuantity {
                    quantity: view.commodity_quantity(seller, commodity),
                }),
            )
        }
        BlockingFact::TooExpensive => (
            BlockerClearingCondition::InventoryChanged {
                commodity: CommodityKind::Coin,
            },
            Some(ClearingBaseline::InventoryQuantity {
                quantity: view.commodity_quantity(agent, CommodityKind::Coin),
            }),
        ),
        BlockingFact::MissingInput(commodity) => (
            BlockerClearingCondition::InventoryChanged { commodity },
            Some(ClearingBaseline::InventoryQuantity {
                quantity: view.commodity_quantity(agent, commodity),
            }),
        ),
        BlockingFact::MissingTool(kind) => (
            BlockerClearingCondition::UniqueItemAcquired { kind },
            Some(ClearingBaseline::UniqueItemCount(
                view.unique_item_count(agent, kind),
            )),
        ),
        BlockingFact::NoKnownSeller => {
            let Some(commodity) = blocker_key.goal_key.commodity else {
                return (BlockerClearingCondition::TtlOnly, None);
            };
            let Some(place) = blocker_key.place else {
                return (BlockerClearingCondition::TtlOnly, None);
            };
            (
                BlockerClearingCondition::CommodityAvailabilityChanged { commodity, place },
                None,
            )
        }
        BlockingFact::NoKnownPath => {
            let Some(destination) = blocker_key.place else {
                return (BlockerClearingCondition::TtlOnly, None);
            };
            (
                BlockerClearingCondition::PathDiscovered { destination },
                Some(ClearingBaseline::PathKnown(false)),
            )
        }
        BlockingFact::TargetGone => match blocker_key.goal_key.kind {
            GoalKind::RaidTarget { .. } | GoalKind::EngageHostile { .. } => {
                (BlockerClearingCondition::TtlOnly, None)
            }
            _ => {
                if blocker_key.target.is_none()
                    && let (Some(commodity), Some(place)) =
                        (blocker_key.goal_key.commodity, blocker_key.place)
                {
                    return (
                        BlockerClearingCondition::CommodityAvailabilityChanged { commodity, place },
                        None,
                    );
                }
                let Some(entity) = blocker_key.target else {
                    return (BlockerClearingCondition::TtlOnly, None);
                };
                (
                    BlockerClearingCondition::EntityReappeared { entity },
                    Some(ClearingBaseline::EntityBelieved(false)),
                )
            }
        },
        BlockingFact::DangerTooHigh | BlockingFact::CombatTooRisky => {
            let Some(place) = blocker_key.place.or_else(|| view.effective_place(agent)) else {
                return (BlockerClearingCondition::TtlOnly, None);
            };
            (BlockerClearingCondition::DangerReduced { place }, None)
        }
        BlockingFact::WorkstationBusy
        | BlockingFact::ExclusiveFacilityUnavailable
        | BlockingFact::ReservationConflict { .. } => {
            let Some(facility) = blocker_key.target else {
                return (BlockerClearingCondition::TtlOnly, None);
            };
            // Resource sources expose contention through `ResourceExtractionQueues`;
            // legacy workstations expose it through `ContentionQueue`. The two
            // substrates do not coexist on a single entity, so reading both and
            // taking the first hit yields the correct baseline either way (FND-26).
            let position = view
                .extraction_slot_queue_position(facility, agent)
                .or_else(|| view.facility_queue_position(facility, agent));
            (
                BlockerClearingCondition::ContentionChanged { facility },
                Some(ClearingBaseline::ContentionPosition(position)),
            )
        }
        BlockingFact::SourceDepleted => {
            let (Some(commodity), Some(place)) =
                (blocker_key.goal_key.commodity, blocker_key.place)
            else {
                return (BlockerClearingCondition::TtlOnly, None);
            };
            let quantity = blocker_key
                .target
                .and_then(|source| view.resource_source(source))
                .map_or(Quantity(0), |resource| resource.available_quantity);
            (
                BlockerClearingCondition::CommodityAvailabilityChanged { commodity, place },
                Some(ClearingBaseline::CommodityQuantity { quantity }),
            )
        }
        BlockingFact::PatienceExhausted | BlockingFact::NoBuyer => {
            (BlockerClearingCondition::TtlOnly, None)
        }
    }
}

fn derive_discrepancy_clearing(
    discrepancy: Discrepancy,
    blocker_key: &BlockerKey,
    execution_failure: Option<ExecutionFailure<'_>>,
) -> DiscrepancyClearing {
    if matches!(discrepancy, Discrepancy::BeliefContradicted)
        && let Some(target) = blocker_key.target
        && let Some(claim_key) = contradiction_claim_key(target, execution_failure)
    {
        return DiscrepancyClearing::BeliefUpdate { claim_key };
    }

    // ReobservationOf is only meaningful for discrepancy classes that
    // re-perceiving the target genuinely resolves: the agent saw the target
    // again and now has a fresh observation that supersedes the recorded
    // discrepancy. Apply it only to belief-staleness / belief-contradiction /
    // missing-observation classes where that semantics holds. For other
    // classes — ImproperPlanningState, PartialExecutionDrift,
    // SearchBudgetExhausted, RouteUnknown, NoLegalBinding,
    // NoWillingCounterparty — re-perceiving the target proves nothing about
    // whether the failure mode is resolved. If we used ReobservationOf for
    // these, the entry would clear the moment the agent travels back into
    // observation range, immediately re-enabling the same broken plan and
    // producing oscillation loops in survival scenarios.
    let target_reobservation_resolves = matches!(
        discrepancy,
        Discrepancy::BeliefStale
            | Discrepancy::BeliefContradicted
            | Discrepancy::Omission(_)
            | Discrepancy::MissingObservation
    );
    if target_reobservation_resolves && let Some(target) = blocker_key.target {
        return DiscrepancyClearing::ReobservationOf { target };
    }

    DiscrepancyClearing::TtlExpiry
}

fn contradiction_claim_key(
    target: EntityId,
    execution_failure: Option<ExecutionFailure<'_>>,
) -> Option<worldwake_core::BeliefClaimKey> {
    let reason = match execution_failure? {
        ExecutionFailure::Replan(signal) => match &signal.reason {
            AbortReason::ExternalAbort {
                kind: ExternalAbortReason::HandlerRequested { reason },
                ..
            } => reason,
            _ => return None,
        },
        ExecutionFailure::Start(failure) => match &failure.reason {
            ActionStartFailureReason::AbortRequested(reason) => reason,
            _ => return None,
        },
    };

    let aspect = match reason {
        ActionAbortRequestReason::TargetLacksWounds { .. }
        | ActionAbortRequestReason::TargetHasNoWounds { .. } => EntityBeliefAspect::Wounded,
        _ => return None,
    };

    Some(worldwake_core::BeliefClaimKey {
        subject: target,
        aspect,
    })
}

fn is_discrepancy_cleared(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    entry: &DiscrepancyEntry,
) -> bool {
    match entry.clearing_condition {
        DiscrepancyClearing::TtlExpiry | DiscrepancyClearing::WorldStructureChange => false,
        DiscrepancyClearing::ReobservationOf { target } => view
            .agent_belief_store(agent)
            .and_then(|store| store.get_entity(&target))
            .and_then(worldwake_core::BelievedEntityState::last_observed_tick)
            .is_some_and(|tick| tick > entry.observed_tick),
        DiscrepancyClearing::BeliefUpdate { claim_key } => view
            .agent_belief_store(agent)
            .and_then(|store| store.entity_claims.get(&claim_key.subject))
            .is_some_and(|claims| {
                claims.iter().any(|claim| {
                    claim.aspect == claim_key.aspect && claim.acquired_tick > entry.observed_tick
                })
            }),
        DiscrepancyClearing::CommodityAvailabilityChanged { commodity, place } => {
            commodity_availability_reobserved(view, agent, commodity, place, entry.observed_tick)
        }
    }
}

fn commodity_availability_reobserved(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    commodity: CommodityKind,
    place: EntityId,
    observed_tick: Tick,
) -> bool {
    if view.effective_place(agent) == Some(place)
        && view.entities_at(place).into_iter().any(|entity| {
            view.item_lot_commodity(entity) == Some(commodity)
                || view.resource_source(entity).is_some_and(|source| {
                    source.commodity == commodity && source.available_quantity > Quantity(0)
                })
        })
    {
        return true;
    }

    view.agent_belief_store(agent).is_some_and(|store| {
        store.known_entities.values().any(|state| {
            state.last_known_place == Some(place)
                && state
                    .last_observed_tick()
                    .is_some_and(|tick| tick > observed_tick)
                && (state.resource_source.as_ref().is_some_and(|source| {
                    source.commodity == commodity && source.available_quantity > Quantity(0)
                }) || state
                    .last_known_inventory
                    .get(&commodity)
                    .copied()
                    .unwrap_or(Quantity(0))
                    > Quantity(0))
        })
    })
}

/// Decide whether a `ReservationConflict` blocker on `facility` has been
/// cleared for `agent`. Two substrates carry contention here:
///
/// - Resource sources expose per-slot occupancy through
///   `ResourceExtractionQueues`. A `ReservationConflict` raised by
///   `extraction_slots_full` clears when the actor either holds a slot
///   grant or is in a position to claim one immediately (slot free and
///   actor is the head waiter, or actor isn't enqueued and a slot has no
///   waiters). Position-only progress (e.g., moving from rank 3 to rank
///   2 while another actor still holds the grant) does not clear the
///   blocker, since re-emitting the goal would only produce another
///   `extraction_slots_full` start failure.
/// - Legacy workstations using temporal reservations clear when the
///   target's reservation ranges are empty.
///
/// The two substrates do not coexist on a single entity (FND-26), so
/// this helper checks each independently and returns true on the first
/// hit. The `baseline` parameter is reserved for future expiry/prune
/// signals (e.g., position decrease combined with a structurally
/// invalid head); the current implementation does not consult it.
fn reservation_conflict_cleared(
    view: &dyn RuntimeBeliefView,
    facility: EntityId,
    agent: EntityId,
    baseline: Option<u32>,
) -> bool {
    let _ = baseline;
    if view.has_extraction_queues(facility) {
        // Resource sources expose contention exclusively through
        // `ResourceExtractionQueues`. The legacy
        // temporal-reservation path never applies here — harvest
        // sources have no `reservation_ranges`, so falling back to
        // it would clear every blocker spuriously.
        if view.actor_holds_extraction_slot_grant(facility, agent) {
            return true;
        }
        return view.actor_can_claim_extraction_slot(facility, agent);
    }
    // Legacy workstation path: clearing fires when the facility's
    // temporal reservations are gone.
    view.reservation_ranges(facility).is_empty()
}

fn is_blocker_cleared(view: &dyn RuntimeBeliefView, agent: EntityId, blocker: &Blocker) -> bool {
    match (&blocker.clearing_condition, &blocker.baseline_snapshot) {
        (
            BlockerClearingCondition::CommodityAvailabilityChanged { commodity, place },
            Some(ClearingBaseline::CommodityQuantity { quantity: baseline }),
        ) => match blocker.blocking_fact {
            BlockingFact::SellerOutOfStock => blocker.scope.exact_target().is_some_and(|seller| {
                view.entity_kind(seller).is_some()
                    && view.commodity_quantity(seller, *commodity) > Quantity(0)
            }),
            BlockingFact::SourceDepleted => blocker
                .scope
                .exact_target()
                .and_then(|source| view.resource_source(source))
                .is_some_and(|resource| resource.available_quantity > Quantity(0)),
            _ => view.locally_observed_commodity_quantity(agent, *place, *commodity) != *baseline,
        },
        (BlockerClearingCondition::CommodityAvailabilityChanged { commodity, place }, None) => {
            place_has_local_commodity_support(view, agent, *place, *commodity, None)
        }
        (
            BlockerClearingCondition::InventoryChanged { commodity },
            Some(ClearingBaseline::InventoryQuantity { quantity: baseline }),
        ) => match blocker.blocking_fact {
            BlockingFact::TooExpensive | BlockingFact::MissingInput(_) => {
                view.commodity_quantity(agent, *commodity) > Quantity(0)
            }
            _ => view.commodity_quantity(agent, *commodity) != *baseline,
        },
        (
            BlockerClearingCondition::UniqueItemAcquired { kind },
            Some(ClearingBaseline::UniqueItemCount(baseline)),
        ) => match blocker.blocking_fact {
            BlockingFact::MissingTool(_) => view.unique_item_count(agent, *kind) > 0,
            _ => view.unique_item_count(agent, *kind) != *baseline,
        },
        (
            BlockerClearingCondition::PathDiscovered { destination },
            Some(ClearingBaseline::PathKnown(false)),
        ) => {
            let Some(current_place) = view.effective_place(agent) else {
                return false;
            };
            view.adjacent_places_with_travel_ticks(current_place)
                .into_iter()
                .any(|(adjacent, _)| adjacent == *destination)
        }
        (
            BlockerClearingCondition::EntityReappeared { entity },
            Some(ClearingBaseline::EntityBelieved(false)),
        ) => match blocker.scope.exact_goal_key().unwrap().kind {
            GoalKind::TreatWounds { .. } | GoalKind::ReduceDanger => {
                view.entity_kind(*entity).is_some() && view.is_alive(*entity)
            }
            _ => view.entity_kind(*entity).is_some(),
        },
        (BlockerClearingCondition::DangerReduced { .. }, _) => {
            view.current_attackers_of(agent).is_empty()
                && view.visible_hostiles_for(agent).is_empty()
        }
        (
            BlockerClearingCondition::ContentionChanged { facility },
            Some(ClearingBaseline::ContentionPosition(baseline)),
        ) => match blocker.blocking_fact {
            BlockingFact::WorkstationBusy => !view.has_production_job(*facility),
            BlockingFact::ReservationConflict { .. } => {
                reservation_conflict_cleared(view, *facility, agent, *baseline)
            }
            BlockingFact::ExclusiveFacilityUnavailable => {
                view.facility_queue_position(*facility, agent)
                    .zip(*baseline)
                    .is_some_and(|(current, baseline)| current < baseline)
                    || view
                        .facility_grant(*facility)
                        .is_some_and(|grant| grant.actor == agent)
            }
            _ => false,
        },
        (BlockerClearingCondition::ContentionChanged { facility }, None) => {
            match blocker.blocking_fact {
                BlockingFact::WorkstationBusy => !view.has_production_job(*facility),
                BlockingFact::ReservationConflict { .. } => {
                    reservation_conflict_cleared(view, *facility, agent, None)
                }
                BlockingFact::ExclusiveFacilityUnavailable => view
                    .facility_grant(*facility)
                    .is_some_and(|grant| grant.actor == agent),
                _ => false,
            }
        }
        _ => false,
    }
}

pub(crate) fn place_has_local_commodity_support(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    place: EntityId,
    commodity: CommodityKind,
    excluded_target: Option<EntityId>,
) -> bool {
    view.listed_sale_lots_at(place, commodity)
        .into_iter()
        .filter(|lot| Some(*lot) != excluded_target)
        .any(|lot| {
            view.seller_for_sale_lot(lot)
                .is_some_and(|seller| seller != agent)
        })
        || view
            .entities_at(place)
            .into_iter()
            .filter(|entity| Some(*entity) != excluded_target)
            .any(|entity| {
                view.item_lot_commodity(entity) == Some(commodity)
                    && view.commodity_quantity(entity, commodity) > Quantity(0)
                    && view.direct_container(entity).is_none()
                    && view.direct_possessor(entity).is_none()
            })
        || view
            .resource_sources_at(place, commodity)
            .into_iter()
            .filter(|source| Some(*source) != excluded_target)
            .any(|source| {
                view.resource_source(source)
                    .is_some_and(|resource| resource.available_quantity > Quantity(0))
            })
        || view
            .corpse_entities_at(place)
            .into_iter()
            .filter(|corpse| Some(*corpse) != excluded_target)
            .any(|corpse| view.commodity_quantity(corpse, commodity) > Quantity(0))
}

fn related_entity(step: &PlannedStep) -> Option<EntityId> {
    match step.op_kind {
        PlannerOpKind::Trade | PlannerOpKind::StaffMarket | PlannerOpKind::StockManagement => step
            .payload_override
            .as_ref()
            .and_then(ActionPayload::as_trade)
            .map(|payload| payload.counterparty)
            .or_else(|| step.targets.first().copied().and_then(authoritative_target)),
        PlannerOpKind::Attack => step
            .payload_override
            .as_ref()
            .and_then(ActionPayload::as_combat)
            .map(|payload| payload.target)
            .or_else(|| step.targets.first().copied().and_then(authoritative_target)),
        PlannerOpKind::Loot => step
            .payload_override
            .as_ref()
            .and_then(ActionPayload::as_loot)
            .map(|payload| payload.target)
            .or_else(|| step.targets.first().copied().and_then(authoritative_target)),
        PlannerOpKind::Travel
        | PlannerOpKind::Patrol
        | PlannerOpKind::Sleep
        | PlannerOpKind::Relieve
        | PlannerOpKind::Wash
        | PlannerOpKind::EstablishCamp
        | PlannerOpKind::Investigate
        | PlannerOpKind::SearchPlace
        | PlannerOpKind::PostBounty
        | PlannerOpKind::PostNotice => None,
        PlannerOpKind::Bury
        | PlannerOpKind::Consume
        | PlannerOpKind::QueueForFacilityUse
        | PlannerOpKind::Harvest
        | PlannerOpKind::Craft
        | PlannerOpKind::MoveCargo
        | PlannerOpKind::DropItem
        | PlannerOpKind::Heal
        | PlannerOpKind::Tell
        | PlannerOpKind::ConsultRecord
        | PlannerOpKind::Defend
        | PlannerOpKind::AskWitness
        | PlannerOpKind::AskAboutPerson
        | PlannerOpKind::ReportMissing
        | PlannerOpKind::EscortToSafety
        | PlannerOpKind::ReportFound
        | PlannerOpKind::Accuse
        | PlannerOpKind::Fine
        | PlannerOpKind::Exile
        | PlannerOpKind::ClaimBounty
        | PlannerOpKind::WithdrawBounty => {
            step.targets.first().copied().and_then(authoritative_target)
        }
        PlannerOpKind::Bribe => step
            .payload_override
            .as_ref()
            .and_then(ActionPayload::as_bribe)
            .map(|payload| payload.target)
            .or_else(|| step.targets.first().copied().and_then(authoritative_target)),
        PlannerOpKind::Threaten => step
            .payload_override
            .as_ref()
            .and_then(ActionPayload::as_threaten)
            .map(|payload| payload.target)
            .or_else(|| step.targets.first().copied().and_then(authoritative_target)),
        PlannerOpKind::DeclareSupport => step
            .payload_override
            .as_ref()
            .and_then(ActionPayload::as_declare_support)
            .map(|payload| payload.office),
        PlannerOpKind::PressForceClaim => step
            .payload_override
            .as_ref()
            .and_then(ActionPayload::as_press_force_claim)
            .map(|payload| payload.office),
        PlannerOpKind::YieldForceClaim => step
            .payload_override
            .as_ref()
            .and_then(ActionPayload::as_yield_force_claim)
            .map(|payload| payload.office),
    }
}

fn related_place(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    goal_key: &GoalKey,
    step: &PlannedStep,
) -> Option<EntityId> {
    match step.op_kind {
        PlannerOpKind::Travel => step.targets.first().copied().and_then(authoritative_target),
        PlannerOpKind::Trade
        | PlannerOpKind::StaffMarket
        | PlannerOpKind::StockManagement
        | PlannerOpKind::EstablishCamp
        | PlannerOpKind::QueueForFacilityUse
        | PlannerOpKind::Harvest
        | PlannerOpKind::Craft
        | PlannerOpKind::DropItem => view.effective_place(agent).or(goal_key.place),
        PlannerOpKind::MoveCargo => related_entity(step)
            .and_then(|target| view.effective_place(target))
            .or(goal_key.place)
            .or_else(|| view.effective_place(agent)),
        PlannerOpKind::Bury => step
            .targets
            .get(1)
            .copied()
            .and_then(authoritative_target)
            .and_then(|burial_site| view.effective_place(burial_site))
            .or_else(|| view.effective_place(agent)),
        PlannerOpKind::Consume
        | PlannerOpKind::Sleep
        | PlannerOpKind::Relieve
        | PlannerOpKind::Wash
        | PlannerOpKind::Heal
        | PlannerOpKind::Loot
        | PlannerOpKind::Attack
        | PlannerOpKind::Defend
        | PlannerOpKind::Patrol => goal_key.place.or_else(|| view.effective_place(agent)),
        PlannerOpKind::Tell
        | PlannerOpKind::ConsultRecord
        | PlannerOpKind::Bribe
        | PlannerOpKind::Threaten
        | PlannerOpKind::Accuse
        | PlannerOpKind::Fine
        | PlannerOpKind::Exile
        | PlannerOpKind::DeclareSupport
        | PlannerOpKind::PressForceClaim
        | PlannerOpKind::YieldForceClaim
        | PlannerOpKind::Investigate
        | PlannerOpKind::AskWitness
        | PlannerOpKind::AskAboutPerson
        | PlannerOpKind::ReportMissing
        | PlannerOpKind::ReportFound
        | PlannerOpKind::ClaimBounty
        | PlannerOpKind::WithdrawBounty
        | PlannerOpKind::PostBounty
        | PlannerOpKind::PostNotice => view.effective_place(agent),
        PlannerOpKind::SearchPlace | PlannerOpKind::EscortToSafety => {
            goal_key.place.or_else(|| view.effective_place(agent))
        }
    }
}

fn blocking_fact_ttl(fact: BlockingFact, cognitive: &CognitiveProfile) -> u32 {
    match fact {
        BlockingFact::SellerOutOfStock
        | BlockingFact::WorkstationBusy
        | BlockingFact::ReservationConflict { .. }
        | BlockingFact::ExclusiveFacilityUnavailable
        | BlockingFact::TargetGone => cognitive.transient_block_ticks,
        BlockingFact::NoKnownPath
        | BlockingFact::NoKnownSeller
        | BlockingFact::TooExpensive
        | BlockingFact::SourceDepleted
        | BlockingFact::MissingTool(_)
        | BlockingFact::MissingInput(_)
        | BlockingFact::DangerTooHigh
        | BlockingFact::CombatTooRisky
        | BlockingFact::PatienceExhausted
        | BlockingFact::NoBuyer => cognitive.structural_block_ticks,
    }
}

#[allow(dead_code)]
fn discrepancy_ttl(discrepancy: Discrepancy, cognitive: &CognitiveProfile) -> u32 {
    match discrepancy {
        Discrepancy::BeliefStale => cognitive.stale_belief_backoff_ticks,
        Discrepancy::BeliefContradicted | Discrepancy::SourceInvalidated => {
            cognitive.contradicted_belief_backoff_ticks
        }
        Discrepancy::ImproperPlanningState => cognitive.improper_state_backoff_ticks,
        Discrepancy::MissingObservation | Discrepancy::Omission(_) => {
            cognitive.missing_observation_backoff_ticks
        }
        Discrepancy::NoLegalBinding => cognitive.no_legal_binding_backoff_ticks,
        Discrepancy::NoWillingCounterparty => cognitive.counterparty_refusal_backoff_ticks,
        Discrepancy::RouteUnknown => cognitive.route_unknown_backoff_ticks,
        Discrepancy::SearchBudgetExhausted => cognitive.search_exhaustion_backoff_ticks,
        Discrepancy::PartialExecutionDrift => cognitive.partial_drift_backoff_ticks,
        Discrepancy::NeedHorizonExceeded { .. }
        | Discrepancy::ArtifactNotActionable { .. }
        | Discrepancy::AbandonConditionFired(_)
        | Discrepancy::MethodFailure(_) => cognitive.structural_block_ticks,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ExecutionFailure, FailureClassification, PlanFailureContext, blocking_fact_ttl,
        classify_discrepancy, clear_resolved_failures, derive_clearing_condition, discrepancy_ttl,
        handle_plan_failure, is_blocker_cleared, reservation_conflict_for,
    };
    use crate::{
        AgentDecisionRuntime, HypotheticalEntityId, PlanTerminalKind, PlannedPlan, PlannedStep,
        PlannerOpKind, PlanningEntityRef, ProfileFixture,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        AcquisitionQuantity, ActionDefId, AffordanceKey, Blocker, BlockerClearingCondition,
        BlockerKey, BlockerMemory, BlockingFact, ClearingBaseline, CognitiveProfile, CombatProfile,
        CommodityConsumableProfile, CommodityKind, CommodityPurpose, ContentionGrant,
        ContentionIntents, DemandObservation, Discrepancy, DiscrepancyClearing, DiscrepancyEntry,
        DiscrepancyMemory, DiscrepancySource, DriveThresholds, EntityId, EntityKind, FrameState,
        GoalKey, GoalKind, HomeostaticNeeds, InTransitOnEdge, IntentionDomain, IntentionFrame,
        LoadUnits, MerchandiseProfile, MetabolismProfile, OmissionReason, Quantity, RecipeId,
        ResourceSource, Tick, TickRange, TradeDispositionProfile, UniqueItemKind, WorkstationTag,
        Wound,
    };
    use worldwake_sim::{
        AbortReason, ActionAbortRequestReason, ActionDuration, ActionPayload, ActionStartFailure,
        ActionStartFailureReason, CombatActionPayload, ControlBeliefView, CraftActionPayload,
        DeclareSupportActionPayload, DurationExpr, EntityBeliefView, InterruptReason,
        ProfileBeliefView, ReplanNeeded, RequestAttemptTrace, RequestBindingKind,
        RequestProvenance, ResolvedRequestTrace, RuntimeBeliefView, SpatialBeliefView,
        TemporalBeliefView, TradeActionPayload,
        belief_view::{BeliefStatus, BeliefValue},
    };

    #[derive(Default)]
    struct TestBeliefView {
        alive: BTreeSet<EntityId>,
        dead: BTreeSet<EntityId>,
        entity_kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
        adjacent_places: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
        unique_items: BTreeMap<(EntityId, UniqueItemKind), u32>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        resource_sources: BTreeMap<EntityId, ResourceSource>,
        production_jobs: BTreeSet<EntityId>,
        reservation_ranges: BTreeMap<EntityId, Vec<TickRange>>,
        facility_queue_positions: BTreeMap<(EntityId, EntityId), u32>,
        facility_grants: BTreeMap<EntityId, ContentionGrant>,
        extraction_slot_positions: BTreeMap<(EntityId, EntityId), u32>,
        extraction_slot_grants: BTreeSet<(EntityId, EntityId)>,
        extraction_slot_claimable: BTreeSet<(EntityId, EntityId)>,
        extraction_facilities: BTreeSet<EntityId>,
        wounds: BTreeMap<EntityId, Vec<Wound>>,
        attackers: BTreeMap<EntityId, Vec<EntityId>>,
        hostiles: BTreeMap<EntityId, Vec<EntityId>>,
        listed_lots: BTreeMap<(EntityId, CommodityKind), Vec<EntityId>>,
        lot_sellers: BTreeMap<EntityId, EntityId>,
        lot_commodities: BTreeMap<EntityId, CommodityKind>,
        believed_target_locations: BTreeMap<(EntityId, EntityId), BeliefValue<Option<EntityId>>>,
    }

    impl ControlBeliefView for TestBeliefView {
        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            true
        }

        fn has_control(&self, entity: EntityId) -> bool {
            self.entity_kinds.get(&entity) == Some(&EntityKind::Agent)
        }
    }

    impl worldwake_sim::BelievedAuthorityView for TestBeliefView {}

    impl EntityBeliefView for TestBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }
        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.entity_kinds.get(&entity).copied()
        }
        fn is_dead(&self, entity: EntityId) -> bool {
            self.dead.contains(&entity)
        }
        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }
        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn believed_target_location(
            &self,
            agent: EntityId,
            target: EntityId,
        ) -> BeliefValue<Option<EntityId>> {
            self.believed_target_locations
                .get(&(agent, target))
                .copied()
                .unwrap_or_else(|| worldwake_sim::belief_view::stale_default_value(None))
        }
    }

    impl ProfileBeliefView for TestBeliefView {
        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }
        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }
        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }
    }

    impl SpatialBeliefView for TestBeliefView {
        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.effective_places.get(&entity).copied()
        }
        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }
        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.entities_at.get(&place).cloned().unwrap_or_default()
        }
        fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
            self.adjacent_places
                .get(&place)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|(place, _)| place)
                .collect()
        }
        fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
            false
        }

        fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> {
            None
        }
        fn adjacent_places_with_travel_ticks(
            &self,
            place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            self.adjacent_places
                .get(&place)
                .cloned()
                .unwrap_or_default()
        }
    }

    impl TemporalBeliefView for TestBeliefView {
        fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool {
            self.reservation_ranges(entity)
                .into_iter()
                .any(|existing| existing.overlaps(&range))
        }

        fn reservation_ranges(&self, entity: EntityId) -> Vec<TickRange> {
            self.reservation_ranges
                .get(&entity)
                .cloned()
                .unwrap_or_default()
        }

        fn facility_queue_position(&self, facility: EntityId, actor: EntityId) -> Option<u32> {
            self.facility_queue_positions
                .get(&(facility, actor))
                .copied()
        }

        fn facility_grant(&self, facility: EntityId) -> Option<&ContentionGrant> {
            self.facility_grants.get(&facility)
        }

        fn extraction_slot_queue_position(&self, source: EntityId, actor: EntityId) -> Option<u32> {
            self.extraction_slot_positions
                .get(&(source, actor))
                .copied()
        }

        fn actor_holds_extraction_slot_grant(&self, source: EntityId, actor: EntityId) -> bool {
            self.extraction_slot_grants.contains(&(source, actor))
        }

        fn actor_can_claim_extraction_slot(&self, source: EntityId, actor: EntityId) -> bool {
            self.extraction_slot_claimable.contains(&(source, actor))
        }

        fn has_extraction_queues(&self, source: EntityId) -> bool {
            self.extraction_facilities.contains(&source)
        }

        fn estimate_duration(
            &self,
            _actor: EntityId,
            _duration: &DurationExpr,
            _targets: &[EntityId],
            _payload: &ActionPayload,
        ) -> Option<ActionDuration> {
            Some(ActionDuration::new(1))
        }
    }

    impl RuntimeBeliefView for TestBeliefView {}
    impl worldwake_sim::LocalPhysicalObservationView for TestBeliefView {}

    impl worldwake_sim::SocialBeliefView for TestBeliefView {
        fn belief_confidence_policy(
            &self,
            _agent: EntityId,
        ) -> worldwake_core::BeliefConfidencePolicy {
            worldwake_core::BeliefConfidencePolicy::default()
        }

        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::IntentionDispositionProfile> {
            None
        }
    }

    impl worldwake_sim::PoliticalBeliefView for TestBeliefView {}

    impl worldwake_sim::CombatBeliefView for TestBeliefView {
        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            None
        }

        fn wounds(&self, agent: EntityId) -> Vec<Wound> {
            self.wounds.get(&agent).cloned().unwrap_or_default()
        }

        fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> {
            self.hostiles.get(&agent).cloned().unwrap_or_default()
        }

        fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId> {
            self.attackers.get(&agent).cloned().unwrap_or_default()
        }

        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds
                .get(&entity)
                .is_some_and(|wounds| !wounds.is_empty())
        }
    }

    impl worldwake_sim::EconomicBeliefView for TestBeliefView {
        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }

        fn controlled_commodity_quantity_at_place(
            &self,
            _actor: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Quantity {
            Quantity(0)
        }

        fn local_controlled_lots_for(
            &self,
            _actor: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn listed_sale_lots_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.listed_lots
                .get(&(place, commodity))
                .cloned()
                .unwrap_or_default()
        }

        fn seller_for_sale_lot(&self, lot: EntityId) -> Option<EntityId> {
            self.lot_sellers.get(&lot).copied()
        }

        fn has_sale_listing(&self, lot: EntityId) -> bool {
            self.lot_sellers.contains_key(&lot)
        }

        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }

        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }
    }

    impl worldwake_sim::InventoryBeliefView for TestBeliefView {
        fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
            self.direct_possessions
                .get(&holder)
                .cloned()
                .unwrap_or_default()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }

        fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32 {
            self.unique_items.get(&(holder, kind)).copied().unwrap_or(0)
        }

        fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
            self.commodity_quantities
                .get(&(holder, kind))
                .copied()
                .unwrap_or(Quantity(0))
        }

        fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
            self.lot_commodities.get(&entity).copied()
        }

        fn item_lot_consumable_profile(
            &self,
            _entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            None
        }

        fn direct_container(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn direct_possessor(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }
    }

    impl worldwake_sim::FacilityBeliefView for TestBeliefView {
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
            self.resource_sources.get(&entity).cloned()
        }

        fn has_production_job(&self, entity: EntityId) -> bool {
            self.production_jobs.contains(&entity)
        }

        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.entities_at(place)
                .into_iter()
                .filter(|entity| {
                    self.resource_sources
                        .get(entity)
                        .is_some_and(|source| source.commodity == commodity)
                })
                .collect()
        }
    }

    fn cognitive(reasoning: &ProfileFixture) -> CognitiveProfile {
        CognitiveProfile {
            max_candidates_per_expansion: CognitiveProfile::default().max_candidates_per_expansion,
            max_plan_depth: reasoning.max_plan_depth,
            max_travel_candidates_per_expansion: CognitiveProfile::default()
                .max_travel_candidates_per_expansion,
            snapshot_travel_horizon: reasoning.snapshot_travel_horizon,
            max_node_expansions: reasoning.max_node_expansions,
            switch_margin: reasoning.switch_margin,
            planning_switch_margin: CognitiveProfile::default().planning_switch_margin,
            transient_block_ticks: reasoning.transient_block_ticks,
            structural_block_ticks: reasoning.structural_block_ticks,
            stale_belief_backoff_ticks: CognitiveProfile::default().stale_belief_backoff_ticks,
            contradicted_belief_backoff_ticks: CognitiveProfile::default()
                .contradicted_belief_backoff_ticks,
            improper_state_backoff_ticks: CognitiveProfile::default().improper_state_backoff_ticks,
            missing_observation_backoff_ticks: CognitiveProfile::default()
                .missing_observation_backoff_ticks,
            no_legal_binding_backoff_ticks: CognitiveProfile::default()
                .no_legal_binding_backoff_ticks,
            counterparty_refusal_backoff_ticks: CognitiveProfile::default()
                .counterparty_refusal_backoff_ticks,
            route_unknown_backoff_ticks: CognitiveProfile::default().route_unknown_backoff_ticks,
            route_segment_blocker_ticks: CognitiveProfile::default().route_segment_blocker_ticks,
            counterparty_blocker_ticks: CognitiveProfile::default().counterparty_blocker_ticks,
            search_exhaustion_backoff_ticks: CognitiveProfile::default()
                .search_exhaustion_backoff_ticks,
            partial_drift_backoff_ticks: CognitiveProfile::default().partial_drift_backoff_ticks,
            expectation_tolerance_ticks: CognitiveProfile::default().expectation_tolerance_ticks,
            guard_min_confidence_ceiling: CognitiveProfile::default().guard_min_confidence_ceiling,
            repair_memory_ticks: CognitiveProfile::default().repair_memory_ticks,
            learned_opportunity_memory_ticks: CognitiveProfile::default()
                .learned_opportunity_memory_ticks,
            survey_memory_capacity: CognitiveProfile::default().survey_memory_capacity,
            survey_memory_retention_ticks: CognitiveProfile::default()
                .survey_memory_retention_ticks,
            initial_cooldown_ticks: reasoning.initial_cooldown_ticks,
            max_cooldown_ticks: reasoning.max_cooldown_ticks,
            landmark_extraction_depth: CognitiveProfile::default().landmark_extraction_depth,
            use_ff_heuristic: CognitiveProfile::default().use_ff_heuristic,
            decision_history_alternatives: CognitiveProfile::default()
                .decision_history_alternatives,
            detour_budget_permille: CognitiveProfile::default().detour_budget_permille,
            compile_opportunity_cap: CognitiveProfile::default().compile_opportunity_cap,
            repair_budget_fraction: CognitiveProfile::default().repair_budget_fraction,
            causal_links_per_step_cap: CognitiveProfile::default().causal_links_per_step_cap,
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn target_location_belief(
        place: Option<EntityId>,
        status: BeliefStatus,
    ) -> BeliefValue<Option<EntityId>> {
        BeliefValue {
            value: place,
            confidence: worldwake_core::Permille::new_unchecked(900),
            acquired_tick: Tick(5),
            claimed_event_tick: Some(Tick(5)),
            status,
        }
    }

    const fn sample_request(input_sequence_no: u64) -> ResolvedRequestTrace {
        ResolvedRequestTrace {
            attempt: RequestAttemptTrace {
                input_sequence_no,
                provenance: RequestProvenance::AiPlan,
            },
            binding: RequestBindingKind::ReproducedAffordance,
        }
    }

    const TRADE_SALE_LOT: EntityId = EntityId {
        slot: 50,
        generation: 1,
    };

    fn trade_goal() -> GoalKey {
        GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        })
    }

    fn trade_step(counterparty: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(1),
            targets: vec![PlanningEntityRef::Authoritative(counterparty)],
            target_place: None,
            payload_override: Some(ActionPayload::Trade(TradeActionPayload {
                counterparty,
                sale_lot: TRADE_SALE_LOT,
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(1),
                requested_quantity: Quantity(1),
            })),
            op_kind: PlannerOpKind::Trade,
            estimated_ticks: 3,
            is_materialization_barrier: true,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    /// Registers the dummy sale lot commodity in a test belief view so that
    /// `item_lot_commodity(TRADE_SALE_LOT)` returns `Some(CommodityKind::Bread)`.
    fn register_trade_sale_lot(view: &mut TestBeliefView) {
        view.lot_commodities
            .insert(TRADE_SALE_LOT, CommodityKind::Bread);
    }

    fn travel_step(place: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(2),
            targets: vec![PlanningEntityRef::Authoritative(place)],
            target_place: Some(place),
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 2,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    fn craft_step(workstation: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(3),
            targets: vec![PlanningEntityRef::Authoritative(workstation)],
            target_place: None,
            payload_override: Some(ActionPayload::Craft(CraftActionPayload {
                recipe_id: RecipeId(4),
                required_workstation_tag: WorkstationTag::Mill,
                inputs: vec![(CommodityKind::Grain, Quantity(2))],
                outputs: vec![(CommodityKind::Bread, Quantity(1))],
                required_tool_kinds: vec![UniqueItemKind::SimpleTool],
            })),
            op_kind: PlannerOpKind::Craft,
            estimated_ticks: 4,
            is_materialization_barrier: true,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    fn claim_office_goal(office: EntityId) -> GoalKey {
        GoalKey::from(GoalKind::ClaimOffice { office })
    }

    fn declare_support_step(office: EntityId, candidate: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(6),
            targets: Vec::new(),
            target_place: None,
            payload_override: Some(ActionPayload::DeclareSupport(DeclareSupportActionPayload {
                office,
                candidate,
            })),
            op_kind: PlannerOpKind::DeclareSupport,
            estimated_ticks: 1,
            is_materialization_barrier: true,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    fn attack_step(target: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(4),
            targets: vec![PlanningEntityRef::Authoritative(target)],
            target_place: None,
            payload_override: Some(ActionPayload::Combat(CombatActionPayload {
                target,
                weapon: worldwake_core::CombatWeaponRef::Unarmed,
            })),
            op_kind: PlannerOpKind::Attack,
            estimated_ticks: 0,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    fn hypothetical_consume_step() -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(5),
            targets: vec![PlanningEntityRef::Hypothetical(HypotheticalEntityId(9))],
            target_place: None,
            payload_override: None,
            op_kind: PlannerOpKind::Consume,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    fn runtime_with_plan(goal: GoalKey, step: PlannedStep) -> AgentDecisionRuntime {
        AgentDecisionRuntime {
            current_plan: Some(PlannedPlan::new(
                worldwake_core::OpportunityKey {
                    goal_key: goal,
                    anchor: worldwake_core::OpportunityAnchor::None,
                },
                goal,
                vec![step],
                PlanTerminalKind::SearchBudgetExhausted {
                    budget_consumed: 0,
                    budget_total: 0,
                },
            )),
            dirty: crate::DirtySet::default(),
            last_priority_class: None,
            ..AgentDecisionRuntime::default()
        }
    }

    fn jc_for_goal(goal: GoalKey) -> IntentionFrame {
        IntentionFrame {
            goal,
            domain: IntentionDomain::Travel {
                destination: entity(99),
            },
            assumptions: Vec::new(),
            state: FrameState::Active,
            established_at: Tick(10),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 10,
            motive_refs: Vec::new(),
            resume_conditions: Vec::new(),
            abandon_conditions: Vec::new(),
            explicit_claims: Vec::new(),
            causal_links: Vec::new(),
        }
    }

    #[test]
    fn handle_plan_failure_drops_plan_records_blocker_and_marks_runtime_dirty() {
        let agent = entity(1);
        let place = entity(10);
        let seller = entity(2);
        let goal = trade_goal();
        let step = trade_step(seller);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        register_trade_sale_lot(&mut view);
        view.listed_lots
            .insert((place, CommodityKind::Bread), vec![TRADE_SALE_LOT]);
        view.lot_sellers.insert(TRADE_SALE_LOT, seller);
        view.commodity_quantities
            .insert((agent, CommodityKind::Coin), Quantity(1));
        let mut runtime = runtime_with_plan(goal, step.clone());
        let mut jc = Some(jc_for_goal(goal));
        let mut blocked = BlockerMemory::default();
        let mut discrepancies = DiscrepancyMemory::default();

        handle_plan_failure(
            &PlanFailureContext {
                view: &view,
                agent,
                goal_key: goal,
                failed_step: &step,
                method_id: None,
                execution_failure: None,
                belief_discrepancy: None,
                current_tick: Tick(20),
            },
            &mut runtime,
            &mut jc,
            &mut blocked,
            &mut discrepancies,
            &mut ContentionIntents::default(),
            &cognitive(&ProfileFixture::default()),
        );

        assert_eq!(runtime.current_plan, None);
        assert!(!runtime.dirty.is_empty());
        assert!(jc.is_none());
        assert_eq!(blocked.intents.len(), 1);
        let intent = blocked.intents.values().next().unwrap();
        assert_eq!(intent.blocking_fact, BlockingFact::SellerOutOfStock);
        assert_eq!(intent.scope.exact_target(), Some(seller));
        assert_eq!(intent.scope.exact_place(), Some(place));
        assert_eq!(intent.scope.exact_action_def(), Some(ActionDefId(1)));
        assert_eq!(
            intent.clearing_condition,
            BlockerClearingCondition::CommodityAvailabilityChanged {
                commodity: CommodityKind::Bread,
                place,
            }
        );
        assert_eq!(
            intent.baseline_snapshot,
            Some(ClearingBaseline::CommodityQuantity {
                quantity: Quantity(0),
            })
        );
        assert_eq!(
            intent.expires_tick,
            Tick(20 + u64::from(ProfileFixture::default().transient_block_ticks))
        );
    }

    fn move_cargo_step(target: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(7),
            targets: vec![PlanningEntityRef::Authoritative(target)],
            target_place: None,
            payload_override: None,
            op_kind: PlannerOpKind::MoveCargo,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        }
    }

    fn derive_blocking_fact(
        view: &dyn RuntimeBeliefView,
        agent: EntityId,
        goal_key: &GoalKey,
        failed_step: &PlannedStep,
        execution_failure: Option<ExecutionFailure<'_>>,
    ) -> BlockingFact {
        match classify_discrepancy(
            view,
            agent,
            goal_key,
            failed_step,
            None,
            execution_failure,
            None,
        ) {
            FailureClassification::Blocker(fact) => fact,
            FailureClassification::Discrepancy(discrepancy) => {
                panic!("expected blocker classification, got discrepancy {discrepancy:?}")
            }
        }
    }

    #[test]
    fn method_selected_unclassified_failure_records_method_failure_discrepancy() {
        let view = TestBeliefView::default();
        let agent = entity(1);
        let goal = GoalKey::from(GoalKind::Sleep);
        let step = PlannedStep {
            def_id: ActionDefId(12),
            targets: Vec::new(),
            target_place: None,
            payload_override: None,
            op_kind: PlannerOpKind::Sleep,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        };
        let method_id = worldwake_core::MethodSchemaId(12);

        let classification =
            classify_discrepancy(&view, agent, &goal, &step, Some(method_id), None, None);

        assert_eq!(
            classification,
            FailureClassification::Discrepancy(Discrepancy::MethodFailure(
                worldwake_core::MethodFailureContext {
                    method_id,
                    kind: worldwake_core::MethodFailureKind::SubgoalUnachievable,
                    subgoal_index: None,
                }
            ))
        );
    }

    fn clear_resolved_blockers(
        view: &dyn RuntimeBeliefView,
        agent: EntityId,
        blocked: &mut BlockerMemory,
        current_tick: Tick,
    ) {
        clear_resolved_failures(
            view,
            agent,
            blocked,
            &mut DiscrepancyMemory::default(),
            current_tick,
        );
    }

    #[test]
    fn handle_plan_failure_scopes_remote_move_cargo_blocker_to_target_place() {
        let agent = entity(1);
        let home = entity(10);
        let remote_place = entity(11);
        let bread_lot = entity(2);
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let step = move_cargo_step(bread_lot);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(bread_lot, EntityKind::ItemLot);
        view.effective_places.insert(agent, home);
        view.effective_places.insert(bread_lot, remote_place);
        view.lot_commodities.insert(bread_lot, CommodityKind::Bread);
        let mut runtime = runtime_with_plan(goal, step.clone());
        let mut jc = Some(jc_for_goal(goal));
        let mut blocked = BlockerMemory::default();
        let mut discrepancies = DiscrepancyMemory::default();

        handle_plan_failure(
            &PlanFailureContext {
                view: &view,
                agent,
                goal_key: goal,
                failed_step: &step,
                method_id: None,
                execution_failure: Some(ExecutionFailure::Start(&ActionStartFailure {
                    tick: Tick(20),
                    actor: agent,
                    def_id: step.def_id,
                    request: sample_request(1),
                    reason: ActionStartFailureReason::PreconditionFailed(
                        "TargetAtActorPlace(0)".to_string(),
                    ),
                })),
                belief_discrepancy: None,
                current_tick: Tick(20),
            },
            &mut runtime,
            &mut jc,
            &mut blocked,
            &mut discrepancies,
            &mut ContentionIntents::default(),
            &cognitive(&ProfileFixture::default()),
        );

        assert!(blocked.intents.is_empty());
        let entry = discrepancies.entries.values().next().unwrap();
        assert_eq!(entry.discrepancy, Discrepancy::ImproperPlanningState);
        assert_eq!(entry.scope.exact_target(), Some(bread_lot));
        assert_eq!(entry.scope.exact_place(), Some(remote_place));
        assert_eq!(entry.scope.exact_action_def(), Some(step.def_id));
    }

    #[test]
    fn handle_plan_failure_records_unlawful_pickup_as_exact_target_legal_discrepancy() {
        let agent = entity(1);
        let home = entity(10);
        let owner = entity(3);
        let water_lot = entity(2);
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let step = move_cargo_step(water_lot);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, owner]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(owner, EntityKind::Agent);
        view.entity_kinds.insert(water_lot, EntityKind::ItemLot);
        view.effective_places.insert(agent, home);
        view.effective_places.insert(owner, home);
        view.effective_places.insert(water_lot, home);
        view.entities_at.insert(home, vec![agent, owner, water_lot]);
        view.lot_commodities.insert(water_lot, CommodityKind::Water);
        let mut runtime = runtime_with_plan(goal, step.clone());
        let mut jc = Some(jc_for_goal(goal));
        let mut blocked = BlockerMemory::default();
        let mut discrepancies = DiscrepancyMemory::default();

        let classification = handle_plan_failure(
            &PlanFailureContext {
                view: &view,
                agent,
                goal_key: goal,
                failed_step: &step,
                method_id: None,
                execution_failure: Some(ExecutionFailure::Start(&ActionStartFailure {
                    tick: Tick(20),
                    actor: agent,
                    def_id: step.def_id,
                    request: sample_request(1),
                    reason: ActionStartFailureReason::PreconditionFailed(
                        "TargetUnownedOrActorControls(0)".to_string(),
                    ),
                })),
                belief_discrepancy: None,
                current_tick: Tick(20),
            },
            &mut runtime,
            &mut jc,
            &mut blocked,
            &mut discrepancies,
            &mut ContentionIntents::default(),
            &cognitive(&ProfileFixture::default()),
        );

        assert_eq!(
            classification,
            FailureClassification::Discrepancy(Discrepancy::NoLegalBinding)
        );
        assert!(blocked.intents.is_empty());
        let entry = discrepancies.entries.values().next().unwrap();
        assert_eq!(entry.discrepancy, Discrepancy::NoLegalBinding);
        assert_eq!(entry.scope.exact_place(), Some(home));
        assert_eq!(entry.scope.exact_target(), Some(water_lot));
        assert_eq!(entry.scope.exact_action_def(), Some(step.def_id));
        assert_eq!(entry.clearing_condition, DiscrepancyClearing::TtlExpiry);
    }

    #[test]
    fn handle_plan_failure_scopes_local_missing_commodity_target_to_place() {
        let agent = entity(1);
        let home = entity(10);
        let missing_lot = entity(2);
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let step = move_cargo_step(missing_lot);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.effective_places.insert(agent, home);
        let mut runtime = runtime_with_plan(goal, step.clone());
        let mut jc = Some(jc_for_goal(goal));
        let mut blocked = BlockerMemory::default();
        let mut discrepancies = DiscrepancyMemory::default();

        handle_plan_failure(
            &PlanFailureContext {
                view: &view,
                agent,
                goal_key: goal,
                failed_step: &step,
                method_id: None,
                execution_failure: None,
                belief_discrepancy: None,
                current_tick: Tick(20),
            },
            &mut runtime,
            &mut jc,
            &mut blocked,
            &mut discrepancies,
            &mut ContentionIntents::default(),
            &cognitive(&ProfileFixture::default()),
        );

        let entry = blocked
            .intents
            .values()
            .next()
            .expect("local target-gone acquire failure should record a blocker");
        assert_eq!(entry.blocking_fact, BlockingFact::TargetGone);
        assert_eq!(entry.scope.exact_place(), Some(home));
        assert_eq!(entry.scope.exact_target(), None);
        assert_eq!(
            entry.clearing_condition,
            BlockerClearingCondition::CommodityAvailabilityChanged {
                commodity: CommodityKind::Bread,
                place: home,
            }
        );
        assert_eq!(entry.baseline_snapshot, None);
        assert!(discrepancies.entries.is_empty());
    }

    #[test]
    fn handle_plan_failure_records_local_commodity_contradiction_as_place_scoped_discrepancy() {
        let agent = entity(1);
        let home = entity(10);
        let local_lot = entity(2);
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        });
        let step = move_cargo_step(local_lot);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(local_lot, EntityKind::ItemLot);
        view.effective_places.insert(agent, home);
        view.effective_places.insert(local_lot, home);
        view.entities_at.insert(home, vec![local_lot]);
        view.lot_commodities.insert(local_lot, CommodityKind::Bread);
        let mut runtime = runtime_with_plan(goal, step.clone());
        let mut jc = Some(jc_for_goal(goal));
        let mut blocked = BlockerMemory::default();
        let mut discrepancies = DiscrepancyMemory::default();

        handle_plan_failure(
            &PlanFailureContext {
                view: &view,
                agent,
                goal_key: goal,
                failed_step: &step,
                method_id: None,
                execution_failure: Some(ExecutionFailure::Start(&ActionStartFailure {
                    tick: Tick(20),
                    actor: agent,
                    def_id: step.def_id,
                    request: sample_request(1),
                    reason: ActionStartFailureReason::PreconditionFailed(
                        "TargetAtActorPlace(0)".to_string(),
                    ),
                })),
                belief_discrepancy: None,
                current_tick: Tick(20),
            },
            &mut runtime,
            &mut jc,
            &mut blocked,
            &mut discrepancies,
            &mut ContentionIntents::default(),
            &cognitive(&ProfileFixture::default()),
        );

        assert!(blocked.intents.is_empty());
        let entry = discrepancies
            .entries
            .values()
            .next()
            .expect("local unsupported acquire failure should record a discrepancy");
        assert_eq!(entry.discrepancy, Discrepancy::BeliefContradicted);
        assert_eq!(entry.scope.exact_place(), Some(home));
        assert_eq!(entry.scope.exact_target(), None);
    }

    #[test]
    fn handle_plan_failure_records_stale_exact_target_revalidation_as_discrepancy() {
        let agent = entity(1);
        let target = entity(2);
        let place = entity(10);
        let goal = GoalKey::from(GoalKind::RaidTarget { target });
        let step = attack_step(target);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, target]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(target, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(target, place);
        view.believed_target_locations.insert(
            (agent, target),
            target_location_belief(Some(place), BeliefStatus::Stale),
        );
        let mut runtime = runtime_with_plan(goal, step.clone());
        let mut jc = Some(jc_for_goal(goal));
        let mut blocked = BlockerMemory::default();
        let mut discrepancies = DiscrepancyMemory::default();

        handle_plan_failure(
            &PlanFailureContext {
                view: &view,
                agent,
                goal_key: goal,
                failed_step: &step,
                method_id: None,
                execution_failure: None,
                belief_discrepancy: super::exact_target_belief_discrepancy(&view, agent, &step),
                current_tick: Tick(20),
            },
            &mut runtime,
            &mut jc,
            &mut blocked,
            &mut discrepancies,
            &mut ContentionIntents::default(),
            &cognitive(&ProfileFixture::default()),
        );

        let entry = discrepancies.entries.values().next().unwrap();
        assert_eq!(entry.discrepancy, Discrepancy::BeliefStale);
        assert_eq!(entry.scope.exact_target(), Some(target));
    }

    #[test]
    fn handle_plan_failure_records_contradicted_exact_target_revalidation_as_discrepancy() {
        let agent = entity(1);
        let target = entity(2);
        let place = entity(10);
        let goal = GoalKey::from(GoalKind::RaidTarget { target });
        let step = attack_step(target);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, target]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(target, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        view.effective_places.insert(target, place);
        view.believed_target_locations.insert(
            (agent, target),
            target_location_belief(Some(place), BeliefStatus::Contradicted),
        );
        let mut runtime = runtime_with_plan(goal, step.clone());
        let mut jc = Some(jc_for_goal(goal));
        let mut blocked = BlockerMemory::default();
        let mut discrepancies = DiscrepancyMemory::default();

        handle_plan_failure(
            &PlanFailureContext {
                view: &view,
                agent,
                goal_key: goal,
                failed_step: &step,
                method_id: None,
                execution_failure: None,
                belief_discrepancy: super::exact_target_belief_discrepancy(&view, agent, &step),
                current_tick: Tick(20),
            },
            &mut runtime,
            &mut jc,
            &mut blocked,
            &mut discrepancies,
            &mut ContentionIntents::default(),
            &cognitive(&ProfileFixture::default()),
        );

        let entry = discrepancies.entries.values().next().unwrap();
        assert_eq!(entry.discrepancy, Discrepancy::BeliefContradicted);
        assert_eq!(entry.scope.exact_target(), Some(target));
    }

    #[test]
    fn derive_clearing_condition_seller_out_of_stock() {
        let agent = entity(1);
        let place = entity(10);
        let seller = entity(2);
        let mut view = TestBeliefView::default();
        view.commodity_quantities
            .insert((seller, CommodityKind::Bread), Quantity(4));
        let blocker_key = BlockerKey {
            goal_key: trade_goal(),
            place: Some(place),
            target: Some(seller),
            action_def: Some(ActionDefId(1)),
        };

        let (condition, baseline) =
            derive_clearing_condition(&view, agent, BlockingFact::SellerOutOfStock, &blocker_key);

        assert_eq!(
            condition,
            BlockerClearingCondition::CommodityAvailabilityChanged {
                commodity: CommodityKind::Bread,
                place,
            }
        );
        assert_eq!(
            baseline,
            Some(ClearingBaseline::CommodityQuantity {
                quantity: Quantity(4),
            })
        );
    }

    #[test]
    fn derive_clearing_condition_too_expensive() {
        let agent = entity(1);
        let mut view = TestBeliefView::default();
        view.commodity_quantities
            .insert((agent, CommodityKind::Coin), Quantity(7));

        let (condition, baseline) = derive_clearing_condition(
            &view,
            agent,
            BlockingFact::TooExpensive,
            &sample_blocker_key_for(GoalKey::from(GoalKind::Sleep)),
        );

        assert_eq!(
            condition,
            BlockerClearingCondition::InventoryChanged {
                commodity: CommodityKind::Coin,
            }
        );
        assert_eq!(
            baseline,
            Some(ClearingBaseline::InventoryQuantity {
                quantity: Quantity(7),
            })
        );
    }

    #[test]
    fn derive_clearing_condition_missing_input() {
        let agent = entity(1);
        let mut view = TestBeliefView::default();
        view.commodity_quantities
            .insert((agent, CommodityKind::Grain), Quantity(2));

        let (condition, baseline) = derive_clearing_condition(
            &view,
            agent,
            BlockingFact::MissingInput(CommodityKind::Grain),
            &sample_blocker_key_for(GoalKey::from(GoalKind::Sleep)),
        );

        assert_eq!(
            condition,
            BlockerClearingCondition::InventoryChanged {
                commodity: CommodityKind::Grain,
            }
        );
        assert_eq!(
            baseline,
            Some(ClearingBaseline::InventoryQuantity {
                quantity: Quantity(2),
            })
        );
    }

    #[test]
    fn derive_clearing_condition_missing_tool() {
        let agent = entity(1);
        let mut view = TestBeliefView::default();
        view.unique_items
            .insert((agent, UniqueItemKind::SimpleTool), 1);

        let (condition, baseline) = derive_clearing_condition(
            &view,
            agent,
            BlockingFact::MissingTool(UniqueItemKind::SimpleTool),
            &sample_blocker_key_for(GoalKey::from(GoalKind::Sleep)),
        );

        assert_eq!(
            condition,
            BlockerClearingCondition::UniqueItemAcquired {
                kind: UniqueItemKind::SimpleTool,
            }
        );
        assert_eq!(baseline, Some(ClearingBaseline::UniqueItemCount(1)));
    }

    #[test]
    fn derive_clearing_condition_ttl_only_fallback() {
        let agent = entity(1);
        let target = entity(2);
        let pursuit_key = BlockerKey {
            goal_key: GoalKey::from(GoalKind::RaidTarget { target }),
            place: Some(entity(10)),
            target: Some(target),
            action_def: Some(ActionDefId(4)),
        };

        for (fact, key) in [
            (
                BlockingFact::PatienceExhausted,
                sample_blocker_key_for(GoalKey::from(GoalKind::Sleep)),
            ),
            (
                BlockingFact::NoBuyer,
                sample_blocker_key_for(GoalKey::from(GoalKind::SellCommodity {
                    commodity: CommodityKind::Bread,
                })),
            ),
            (BlockingFact::TargetGone, pursuit_key),
        ] {
            let (condition, baseline) =
                derive_clearing_condition(&TestBeliefView::default(), agent, fact, &key);
            assert_eq!(condition, BlockerClearingCondition::TtlOnly, "{fact:?}");
            assert_eq!(baseline, None, "{fact:?}");
        }
    }

    #[test]
    fn derive_clearing_condition_no_known_path() {
        let agent = entity(1);
        let destination = entity(11);
        let blocker_key = BlockerKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            place: Some(destination),
            target: None,
            action_def: Some(ActionDefId(2)),
        };

        let (condition, baseline) = derive_clearing_condition(
            &TestBeliefView::default(),
            agent,
            BlockingFact::NoKnownPath,
            &blocker_key,
        );

        assert_eq!(
            condition,
            BlockerClearingCondition::PathDiscovered { destination }
        );
        assert_eq!(baseline, Some(ClearingBaseline::PathKnown(false)));
    }

    #[test]
    fn derive_clearing_condition_target_gone_non_pursuit() {
        let agent = entity(1);
        let target = entity(2);
        let blocker_key = BlockerKey {
            goal_key: GoalKey::from(GoalKind::ReduceDanger),
            place: Some(entity(10)),
            target: Some(target),
            action_def: Some(ActionDefId(4)),
        };

        let (condition, baseline) = derive_clearing_condition(
            &TestBeliefView::default(),
            agent,
            BlockingFact::TargetGone,
            &blocker_key,
        );

        assert_eq!(
            condition,
            BlockerClearingCondition::EntityReappeared { entity: target }
        );
        assert_eq!(baseline, Some(ClearingBaseline::EntityBelieved(false)));
    }

    #[test]
    fn derive_clearing_condition_contention_blockers_capture_queue_baseline_when_available() {
        let agent = entity(1);
        let facility = entity(3);
        let mut view = TestBeliefView::default();
        view.facility_queue_positions.insert((facility, agent), 2);
        let blocker_key = BlockerKey {
            goal_key: GoalKey::from(GoalKind::ProduceCommodity {
                recipe_id: RecipeId(4),
            }),
            place: Some(entity(10)),
            target: Some(facility),
            action_def: Some(ActionDefId(3)),
        };

        let (condition, baseline) = derive_clearing_condition(
            &view,
            agent,
            reservation_conflict(facility, ActionDefId(3)),
            &blocker_key,
        );

        assert_eq!(
            condition,
            BlockerClearingCondition::ContentionChanged { facility }
        );
        assert_eq!(
            baseline,
            Some(ClearingBaseline::ContentionPosition(Some(2)))
        );
    }

    #[test]
    fn is_blocker_cleared_commodity_availability_changed() {
        let agent = entity(1);
        let place = entity(10);
        let seller = entity(2);
        let blocker = Blocker {
            scope: BlockerKey {
                goal_key: trade_goal(),
                place: Some(place),
                target: Some(seller),
                action_def: Some(ActionDefId(1)),
            }
            .into(),
            blocking_fact: BlockingFact::SellerOutOfStock,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::CommodityAvailabilityChanged {
                commodity: CommodityKind::Bread,
                place,
            },
            baseline_snapshot: Some(ClearingBaseline::CommodityQuantity {
                quantity: Quantity(0),
            }),
            source_event: None,
        };

        let mut view = TestBeliefView::default();
        view.commodity_quantities
            .insert((seller, CommodityKind::Bread), Quantity(2));
        view.entity_kinds.insert(seller, EntityKind::Agent);

        assert!(is_blocker_cleared(&view, agent, &blocker));
    }

    #[test]
    fn is_blocker_cleared_inventory_changed() {
        let agent = entity(1);
        let blocker = Blocker {
            scope: sample_blocker_key_for(GoalKey::from(GoalKind::Sleep)).into(),
            blocking_fact: BlockingFact::TooExpensive,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::InventoryChanged {
                commodity: CommodityKind::Coin,
            },
            baseline_snapshot: Some(ClearingBaseline::InventoryQuantity {
                quantity: Quantity(0),
            }),
            source_event: None,
        };

        let mut view = TestBeliefView::default();
        view.commodity_quantities
            .insert((agent, CommodityKind::Coin), Quantity(3));

        assert!(is_blocker_cleared(&view, agent, &blocker));
    }

    #[test]
    fn is_blocker_cleared_unique_item_acquired() {
        let agent = entity(1);
        let blocker = Blocker {
            scope: sample_blocker_key_for(GoalKey::from(GoalKind::Sleep)).into(),
            blocking_fact: BlockingFact::MissingTool(UniqueItemKind::SimpleTool),
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::UniqueItemAcquired {
                kind: UniqueItemKind::SimpleTool,
            },
            baseline_snapshot: Some(ClearingBaseline::UniqueItemCount(0)),
            source_event: None,
        };

        let mut view = TestBeliefView::default();
        view.unique_items
            .insert((agent, UniqueItemKind::SimpleTool), 1);

        assert!(is_blocker_cleared(&view, agent, &blocker));
    }

    #[test]
    fn is_blocker_cleared_path_discovered() {
        let agent = entity(1);
        let current_place = entity(10);
        let destination = entity(11);
        let blocker = Blocker {
            scope: sample_blocker_key_for(GoalKey::from(GoalKind::Sleep)).into(),
            blocking_fact: BlockingFact::NoKnownPath,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::PathDiscovered { destination },
            baseline_snapshot: Some(ClearingBaseline::PathKnown(false)),
            source_event: None,
        };

        let mut view = TestBeliefView::default();
        view.effective_places.insert(agent, current_place);
        view.adjacent_places.insert(
            current_place,
            vec![(destination, NonZeroU32::new(2).unwrap())],
        );

        assert!(is_blocker_cleared(&view, agent, &blocker));
    }

    #[test]
    fn is_blocker_cleared_entity_reappeared() {
        let agent = entity(1);
        let target = entity(2);
        let blocker = Blocker {
            scope: BlockerKey {
                goal_key: GoalKey::from(GoalKind::ReduceDanger),
                place: Some(entity(10)),
                target: Some(target),
                action_def: Some(ActionDefId(4)),
            }
            .into(),
            blocking_fact: BlockingFact::TargetGone,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::EntityReappeared { entity: target },
            baseline_snapshot: Some(ClearingBaseline::EntityBelieved(false)),
            source_event: None,
        };

        let mut view = TestBeliefView::default();
        view.entity_kinds.insert(target, EntityKind::Agent);
        view.alive.insert(target);

        assert!(is_blocker_cleared(&view, agent, &blocker));
    }

    #[test]
    fn is_blocker_cleared_danger_reduced() {
        let agent = entity(1);
        let blocker = Blocker {
            scope: sample_blocker_key_for(GoalKey::from(GoalKind::ReduceDanger)).into(),
            blocking_fact: BlockingFact::DangerTooHigh,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::DangerReduced { place: entity(10) },
            baseline_snapshot: None,
            source_event: None,
        };

        assert!(is_blocker_cleared(
            &TestBeliefView::default(),
            agent,
            &blocker
        ));
    }

    #[test]
    fn is_blocker_cleared_contention_changed() {
        let agent = entity(1);
        let facility = entity(3);
        let blocker = Blocker {
            scope: BlockerKey {
                goal_key: GoalKey::from(GoalKind::ProduceCommodity {
                    recipe_id: RecipeId(4),
                }),
                place: Some(entity(10)),
                target: Some(facility),
                action_def: Some(ActionDefId(3)),
            }
            .into(),
            blocking_fact: BlockingFact::ExclusiveFacilityUnavailable,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::ContentionChanged { facility },
            baseline_snapshot: Some(ClearingBaseline::ContentionPosition(Some(2))),
            source_event: None,
        };

        let mut view = TestBeliefView::default();
        view.facility_queue_positions.insert((facility, agent), 1);

        assert!(is_blocker_cleared(&view, agent, &blocker));
    }

    fn reservation_conflict_blocker_at(
        agent: EntityId,
        facility: EntityId,
        baseline_position: Option<u32>,
    ) -> Blocker {
        let _ = agent;
        Blocker {
            scope: BlockerKey {
                goal_key: GoalKey::from(GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Water,
                    purpose: CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                }),
                place: Some(entity(10)),
                target: Some(facility),
                action_def: Some(ActionDefId(3)),
            }
            .into(),
            blocking_fact: reservation_conflict(facility, ActionDefId(3)),
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::ContentionChanged { facility },
            baseline_snapshot: Some(ClearingBaseline::ContentionPosition(baseline_position)),
            source_event: None,
        }
    }

    #[test]
    fn derive_clearing_condition_reservation_conflict_uses_extraction_slot_position() {
        // S127QUAAWAACQ-010: harvest source contention is exposed via
        // `ResourceExtractionQueues`, not the legacy `ContentionQueue`.
        // The baseline must therefore prefer the extraction-slot position.
        let agent = entity(1);
        let source = entity(3);
        let mut view = TestBeliefView::default();
        view.extraction_slot_positions.insert((source, agent), 2);
        let blocker_key = BlockerKey {
            goal_key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: CommodityPurpose::SelfConsume,
                quantity: AcquisitionQuantity::single(),
            }),
            place: Some(entity(10)),
            target: Some(source),
            action_def: Some(ActionDefId(11)),
        };

        let (condition, baseline) = derive_clearing_condition(
            &view,
            agent,
            reservation_conflict(source, ActionDefId(11)),
            &blocker_key,
        );

        assert_eq!(
            condition,
            BlockerClearingCondition::ContentionChanged { facility: source }
        );
        assert_eq!(
            baseline,
            Some(ClearingBaseline::ContentionPosition(Some(2))),
            "harvest sources expose contention via ResourceExtractionQueues",
        );
    }

    #[test]
    fn is_blocker_cleared_holds_when_position_decreases_but_slot_still_held() {
        // Position-only progress is insufficient to clear the blocker:
        // moving from rank 2 to rank 1 while another actor still holds
        // the grant means re-emitting the goal would just produce another
        // `extraction_slots_full` start failure. The blocker must stay
        // active until the actor can actually claim a slot (FND-26 FIFO
        // semantics enforced by `grant_or_signal_full`).
        let agent = entity(1);
        let source = entity(3);
        let blocker = reservation_conflict_blocker_at(agent, source, Some(2));

        let mut view = TestBeliefView::default();
        view.extraction_facilities.insert(source);
        view.extraction_slot_positions.insert((source, agent), 1);

        assert!(
            !is_blocker_cleared(&view, agent, &blocker),
            "queue position improved but slot still held — blocker must stay active",
        );
    }

    #[test]
    fn is_blocker_cleared_when_actor_is_promoted_to_extraction_slot_grant() {
        let agent = entity(1);
        let source = entity(3);
        let blocker = reservation_conflict_blocker_at(agent, source, Some(2));

        let mut view = TestBeliefView::default();
        view.extraction_facilities.insert(source);
        view.extraction_slot_grants.insert((source, agent));

        assert!(
            is_blocker_cleared(&view, agent, &blocker),
            "an actor newly granted a slot must see their conflict blocker clear",
        );
    }

    #[test]
    fn is_blocker_cleared_when_actor_left_queue_and_slot_is_available() {
        // FOUNDATIONS Section VI Scenario E "Competing Claimants -> Queue
        // -> Expiry/Prune -> Next Actor Acts": the actor was queued at
        // baseline, has since been removed from the queue (out-of-band
        // abandonment), and a slot is now free — the next harvest start
        // request would succeed, so the blocker must clear.
        let agent = entity(1);
        let source = entity(3);
        let blocker = reservation_conflict_blocker_at(agent, source, Some(2));

        let mut view = TestBeliefView::default();
        view.extraction_facilities.insert(source);
        view.extraction_slot_claimable.insert((source, agent));

        assert!(
            is_blocker_cleared(&view, agent, &blocker),
            "actor no longer queued + slot available must clear the blocker",
        );
    }

    #[test]
    fn is_blocker_cleared_holds_when_extraction_position_unchanged_and_no_slot_available() {
        // Negative case: nothing in the world has changed since the
        // blocker was recorded — position is the same and no slot has
        // freed. The blocker must NOT clear.
        let agent = entity(1);
        let source = entity(3);
        let blocker = reservation_conflict_blocker_at(agent, source, Some(2));

        let mut view = TestBeliefView::default();
        view.extraction_facilities.insert(source);
        view.extraction_slot_positions.insert((source, agent), 2);

        assert!(
            !is_blocker_cleared(&view, agent, &blocker),
            "no contention change must leave the blocker active",
        );
    }

    #[test]
    fn is_blocker_cleared_ttl_only_never_clears() {
        let agent = entity(1);
        let blocker = Blocker {
            scope: sample_blocker_key_for(GoalKey::from(GoalKind::Sleep)).into(),
            blocking_fact: BlockingFact::NoKnownPath,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
            source_event: None,
        };

        assert!(!is_blocker_cleared(
            &TestBeliefView::default(),
            agent,
            &blocker
        ));
    }

    #[test]
    fn is_blocker_cleared_no_known_seller_listing_appears() {
        let agent = entity(1);
        let place = entity(10);
        let seller = entity(2);
        let blocker = Blocker {
            scope: BlockerKey {
                goal_key: trade_goal(),
                place: Some(place),
                target: None,
                action_def: Some(ActionDefId(1)),
            }
            .into(),
            blocking_fact: BlockingFact::NoKnownSeller,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::CommodityAvailabilityChanged {
                commodity: CommodityKind::Bread,
                place,
            },
            baseline_snapshot: None,
            source_event: None,
        };

        let mut view = TestBeliefView::default();
        register_trade_sale_lot(&mut view);
        view.listed_lots
            .insert((place, CommodityKind::Bread), vec![TRADE_SALE_LOT]);
        view.lot_sellers.insert(TRADE_SALE_LOT, seller);

        assert!(is_blocker_cleared(&view, agent, &blocker));
    }

    #[test]
    fn is_blocker_cleared_missing_baseline_falls_back() {
        let agent = entity(1);
        let blocker = Blocker {
            scope: sample_blocker_key_for(GoalKey::from(GoalKind::Sleep)).into(),
            blocking_fact: BlockingFact::TooExpensive,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::InventoryChanged {
                commodity: CommodityKind::Coin,
            },
            baseline_snapshot: None,
            source_event: None,
        };

        let mut view = TestBeliefView::default();
        view.commodity_quantities
            .insert((agent, CommodityKind::Coin), Quantity(4));

        assert!(!is_blocker_cleared(&view, agent, &blocker));
    }

    #[test]
    fn is_blocker_cleared_pursuit_target_gone_ttl_only() {
        let agent = entity(1);
        let target = entity(2);
        let blocker = Blocker {
            scope: BlockerKey {
                goal_key: GoalKey::from(GoalKind::RaidTarget { target }),
                place: Some(entity(10)),
                target: Some(target),
                action_def: Some(ActionDefId(4)),
            }
            .into(),
            blocking_fact: BlockingFact::TargetGone,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
            source_event: None,
        };

        let mut view = TestBeliefView::default();
        view.entity_kinds.insert(target, EntityKind::Agent);
        view.alive.insert(target);

        assert!(!is_blocker_cleared(&view, agent, &blocker));
    }

    fn sample_blocker_key_for(goal_key: GoalKey) -> BlockerKey {
        BlockerKey {
            goal_key,
            place: None,
            target: None,
            action_def: None,
        }
    }

    fn reservation_conflict(facility: EntityId, action: ActionDefId) -> BlockingFact {
        reservation_conflict_for(AffordanceKey { facility, action })
    }

    #[test]
    fn derive_blocking_fact_detects_seller_out_of_stock() {
        let agent = entity(1);
        let place = entity(10);
        let seller = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        register_trade_sale_lot(&mut view);
        view.listed_lots
            .insert((place, CommodityKind::Bread), vec![TRADE_SALE_LOT]);
        view.lot_sellers.insert(TRADE_SALE_LOT, seller);
        view.commodity_quantities
            .insert((agent, CommodityKind::Coin), Quantity(1));

        let fact = derive_blocking_fact(&view, agent, &trade_goal(), &trade_step(seller), None);
        assert_eq!(fact, BlockingFact::SellerOutOfStock);
    }

    #[test]
    fn derive_blocking_fact_detects_no_known_path() {
        let agent = entity(1);
        let from = entity(10);
        let to = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.effective_places.insert(agent, from);

        let fact = derive_blocking_fact(
            &view,
            agent,
            &GoalKey::from(GoalKind::Sleep),
            &travel_step(to),
            None,
        );
        assert_eq!(fact, BlockingFact::NoKnownPath);
    }

    #[test]
    fn derive_blocking_fact_detects_target_gone() {
        let agent = entity(1);
        let target = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);

        let fact = derive_blocking_fact(
            &view,
            agent,
            &GoalKey::from(GoalKind::ReduceDanger),
            &attack_step(target),
            None,
        );
        assert_eq!(fact, BlockingFact::TargetGone);
    }

    #[test]
    fn derive_blocking_fact_treats_hypothetical_consume_loss_as_missing_input() {
        let agent = entity(1);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);

        let fact = derive_blocking_fact(
            &view,
            agent,
            &GoalKey::from(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            }),
            &hypothetical_consume_step(),
            None,
        );

        assert_eq!(fact, BlockingFact::MissingInput(CommodityKind::Bread));
    }

    #[test]
    fn derive_blocking_fact_detects_workstation_busy() {
        let agent = entity(1);
        let workstation = entity(3);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(workstation, EntityKind::Facility);
        view.production_jobs.insert(workstation);
        view.unique_items
            .insert((agent, UniqueItemKind::SimpleTool), 1);
        view.commodity_quantities
            .insert((agent, CommodityKind::Grain), Quantity(2));

        let fact = derive_blocking_fact(
            &view,
            agent,
            &GoalKey::from(GoalKind::ProduceCommodity {
                recipe_id: RecipeId(4),
            }),
            &craft_step(workstation),
            None,
        );
        assert_eq!(fact, BlockingFact::WorkstationBusy);
    }

    #[test]
    fn derive_blocking_fact_detects_reservation_conflict() {
        let agent = entity(1);
        let workstation = entity(3);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(workstation, EntityKind::Facility);
        view.unique_items
            .insert((agent, UniqueItemKind::SimpleTool), 1);
        view.commodity_quantities
            .insert((agent, CommodityKind::Grain), Quantity(2));
        view.reservation_ranges.insert(
            workstation,
            vec![TickRange::new(Tick(8), Tick(12)).unwrap()],
        );

        let fact = derive_blocking_fact(
            &view,
            agent,
            &GoalKey::from(GoalKind::ProduceCommodity {
                recipe_id: RecipeId(4),
            }),
            &craft_step(workstation),
            None,
        );
        assert_eq!(fact, reservation_conflict(workstation, ActionDefId(3)));
    }

    #[test]
    fn derive_blocking_fact_uses_structured_start_failure_when_view_is_insufficient() {
        let agent = entity(1);
        let workstation = entity(3);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(workstation, EntityKind::Facility);
        view.effective_places.insert(agent, entity(10));
        view.unique_items
            .insert((agent, UniqueItemKind::SimpleTool), 1);
        view.commodity_quantities
            .insert((agent, CommodityKind::Grain), Quantity(2));
        let start_failure = ActionStartFailure {
            tick: Tick(4),
            actor: agent,
            def_id: ActionDefId(3),
            request: sample_request(4),
            reason: ActionStartFailureReason::ReservationUnavailable(workstation),
        };

        let fact = derive_blocking_fact(
            &view,
            agent,
            &GoalKey::from(GoalKind::ProduceCommodity {
                recipe_id: RecipeId(4),
            }),
            &craft_step(workstation),
            Some(ExecutionFailure::Start(&start_failure)),
        );

        assert_eq!(fact, reservation_conflict(workstation, ActionDefId(3)));
    }

    #[test]
    fn derive_blocking_fact_uses_authoritative_trade_start_failure_when_belief_is_stale() {
        let agent = entity(1);
        let place = entity(10);
        let seller = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        register_trade_sale_lot(&mut view);
        view.listed_lots
            .insert((place, CommodityKind::Bread), vec![TRADE_SALE_LOT]);
        view.lot_sellers.insert(TRADE_SALE_LOT, seller);
        view.commodity_quantities
            .insert((agent, CommodityKind::Coin), Quantity(1));
        view.commodity_quantities
            .insert((seller, CommodityKind::Bread), Quantity(1));
        let start_failure = ActionStartFailure {
            tick: Tick(4),
            actor: agent,
            def_id: ActionDefId(3),
            request: sample_request(5),
            reason: ActionStartFailureReason::AbortRequested(
                ActionAbortRequestReason::HolderLacksAccessibleCommodity {
                    holder: seller,
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(1),
                },
            ),
        };

        let fact = derive_blocking_fact(
            &view,
            agent,
            &trade_goal(),
            &trade_step(seller),
            Some(ExecutionFailure::Start(&start_failure)),
        );

        assert_eq!(fact, BlockingFact::SellerOutOfStock);
    }

    #[test]
    fn handle_plan_failure_keeps_stale_political_start_failures_on_shared_unknown_path() {
        let agent = entity(1);
        let place = entity(10);
        let office = entity(20);
        let goal = claim_office_goal(office);
        let step = declare_support_step(office, agent);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.effective_places.insert(agent, place);
        let start_failure = ActionStartFailure {
            tick: Tick(4),
            actor: agent,
            def_id: ActionDefId(6),
            request: sample_request(6),
            reason: ActionStartFailureReason::PreconditionFailed(format!(
                "office {office} is not vacant"
            )),
        };
        let mut runtime = runtime_with_plan(goal, step.clone());
        let mut jc = Some(jc_for_goal(goal));
        let mut blocked = BlockerMemory::default();
        let mut discrepancies = DiscrepancyMemory::default();
        let budget = ProfileFixture::default();

        handle_plan_failure(
            &PlanFailureContext {
                view: &view,
                agent,
                goal_key: goal,
                failed_step: &step,
                method_id: None,
                execution_failure: Some(ExecutionFailure::Start(&start_failure)),
                belief_discrepancy: None,
                current_tick: Tick(20),
            },
            &mut runtime,
            &mut jc,
            &mut blocked,
            &mut discrepancies,
            &mut ContentionIntents::default(),
            &cognitive(&budget),
        );

        assert_eq!(runtime.current_plan, None);
        assert!(!runtime.dirty.is_empty());
        assert!(blocked.intents.is_empty());
        assert_eq!(discrepancies.entries.len(), 1);
        let entry = discrepancies.entries.values().next().unwrap();
        assert_eq!(entry.discrepancy, Discrepancy::ImproperPlanningState);
        assert_eq!(entry.scope.exact_target(), Some(office));
        assert_eq!(entry.scope.exact_place(), Some(place));
        assert_eq!(entry.scope.exact_action_def(), Some(ActionDefId(6)));
        assert_eq!(
            entry.expires_tick,
            Tick(20 + u64::from(cognitive(&budget).improper_state_backoff_ticks))
        );
    }

    #[test]
    fn derive_blocking_fact_detects_no_known_seller_when_market_is_empty() {
        let agent = entity(1);
        let place = entity(10);
        let seller = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.effective_places.insert(agent, place);
        register_trade_sale_lot(&mut view);
        view.commodity_quantities
            .insert((agent, CommodityKind::Coin), Quantity(1));
        view.commodity_quantities
            .insert((seller, CommodityKind::Bread), Quantity(3));

        let fact = derive_blocking_fact(&view, agent, &trade_goal(), &trade_step(seller), None);
        assert_eq!(fact, BlockingFact::NoKnownSeller);
    }

    #[test]
    fn derive_blocking_fact_falls_back_to_abort_reason_hint() {
        let agent = entity(1);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        let step = PlannedStep {
            def_id: ActionDefId(5),
            targets: Vec::new(),
            target_place: None,
            payload_override: None,
            op_kind: PlannerOpKind::Sleep,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
            guard: None,
            expectations: Vec::new(),
        };
        let signal = ReplanNeeded {
            agent,
            failed_action_def: ActionDefId(5),
            failed_instance: worldwake_sim::ActionInstanceId(7),
            reason: AbortReason::interrupted(InterruptReason::DangerNearby),
            tick: Tick(4),
        };

        let fact = derive_blocking_fact(
            &view,
            agent,
            &GoalKey::from(GoalKind::Sleep),
            &step,
            Some(ExecutionFailure::Replan(&signal)),
        );
        assert_eq!(fact, BlockingFact::DangerTooHigh);
    }

    #[test]
    fn blocking_fact_ttl_uses_budget_classification() {
        let budget = ProfileFixture::default();

        assert_eq!(
            blocking_fact_ttl(BlockingFact::SellerOutOfStock, &cognitive(&budget)),
            budget.transient_block_ticks
        );
        assert_eq!(
            blocking_fact_ttl(BlockingFact::NoKnownSeller, &cognitive(&budget)),
            budget.structural_block_ticks
        );
    }

    #[test]
    fn discrepancy_ttl_uses_class_specific_defaults() {
        let cognitive = CognitiveProfile::default();

        assert_eq!(discrepancy_ttl(Discrepancy::BeliefStale, &cognitive), 30);
        assert_eq!(
            discrepancy_ttl(Discrepancy::BeliefContradicted, &cognitive),
            60
        );
        assert_eq!(
            discrepancy_ttl(Discrepancy::ImproperPlanningState, &cognitive),
            2
        );
        assert_eq!(
            discrepancy_ttl(Discrepancy::MissingObservation, &cognitive),
            20
        );
        assert_eq!(
            discrepancy_ttl(
                Discrepancy::Omission(OmissionReason::OverBudget {
                    budget: 5,
                    candidates_seen: 10,
                }),
                &cognitive
            ),
            20
        );
        assert_eq!(
            discrepancy_ttl(Discrepancy::NoLegalBinding, &cognitive),
            120
        );
        assert_eq!(
            discrepancy_ttl(Discrepancy::NoWillingCounterparty, &cognitive),
            40
        );
        assert_eq!(discrepancy_ttl(Discrepancy::RouteUnknown, &cognitive), 200);
        assert_eq!(
            discrepancy_ttl(Discrepancy::SearchBudgetExhausted, &cognitive),
            100
        );
        assert_eq!(
            discrepancy_ttl(Discrepancy::PartialExecutionDrift, &cognitive),
            4
        );
        assert_eq!(
            discrepancy_ttl(
                Discrepancy::MethodFailure(worldwake_core::MethodFailureContext {
                    method_id: worldwake_core::MethodSchemaId(7),
                    kind: worldwake_core::MethodFailureKind::SubgoalUnachievable,
                    subgoal_index: Some(2),
                }),
                &cognitive
            ),
            cognitive.structural_block_ticks
        );
    }

    #[test]
    fn discrepancy_ttl_respects_profile_override() {
        let cognitive = CognitiveProfile {
            stale_belief_backoff_ticks: 11,
            contradicted_belief_backoff_ticks: 12,
            improper_state_backoff_ticks: 13,
            missing_observation_backoff_ticks: 14,
            no_legal_binding_backoff_ticks: 15,
            counterparty_refusal_backoff_ticks: 16,
            route_unknown_backoff_ticks: 17,
            search_exhaustion_backoff_ticks: 18,
            partial_drift_backoff_ticks: 19,
            ..CognitiveProfile::default()
        };

        assert_eq!(discrepancy_ttl(Discrepancy::BeliefStale, &cognitive), 11);
        assert_eq!(
            discrepancy_ttl(Discrepancy::BeliefContradicted, &cognitive),
            12
        );
        assert_eq!(
            discrepancy_ttl(Discrepancy::ImproperPlanningState, &cognitive),
            13
        );
        assert_eq!(
            discrepancy_ttl(Discrepancy::MissingObservation, &cognitive),
            14
        );
        assert_eq!(
            discrepancy_ttl(
                Discrepancy::Omission(OmissionReason::OverBudget {
                    budget: 5,
                    candidates_seen: 10,
                }),
                &cognitive
            ),
            14
        );
        assert_eq!(discrepancy_ttl(Discrepancy::NoLegalBinding, &cognitive), 15);
        assert_eq!(
            discrepancy_ttl(Discrepancy::NoWillingCounterparty, &cognitive),
            16
        );
        assert_eq!(discrepancy_ttl(Discrepancy::RouteUnknown, &cognitive), 17);
        assert_eq!(
            discrepancy_ttl(Discrepancy::SearchBudgetExhausted, &cognitive),
            18
        );
        assert_eq!(
            discrepancy_ttl(Discrepancy::PartialExecutionDrift, &cognitive),
            19
        );
        assert_eq!(
            discrepancy_ttl(
                Discrepancy::MethodFailure(worldwake_core::MethodFailureContext {
                    method_id: worldwake_core::MethodSchemaId(7),
                    kind: worldwake_core::MethodFailureKind::SubgoalUnachievable,
                    subgoal_index: Some(2),
                }),
                &cognitive
            ),
            cognitive.structural_block_ticks
        );
    }

    #[test]
    fn transient_blockers_unchanged_ttl() {
        let budget = ProfileFixture::default();
        let transient_facts = [
            BlockingFact::SellerOutOfStock,
            BlockingFact::WorkstationBusy,
            reservation_conflict(entity(3), ActionDefId(3)),
            BlockingFact::ExclusiveFacilityUnavailable,
            BlockingFact::TargetGone,
        ];
        for fact in transient_facts {
            assert_eq!(
                blocking_fact_ttl(fact, &cognitive(&budget)),
                20,
                "{fact:?} should still use transient_block_ticks (20)"
            );
        }
    }

    #[test]
    fn unknown_blocker_carries_diagnostic_context() {
        let agent = entity(1);
        let place = entity(10);
        let office = entity(20);
        let goal = claim_office_goal(office);
        let step = declare_support_step(office, agent);
        let mut view = TestBeliefView::default();
        view.alive.insert(agent);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(office, EntityKind::Office);
        view.effective_places.insert(agent, place);
        let start_failure = ActionStartFailure {
            tick: Tick(4),
            actor: agent,
            def_id: ActionDefId(6),
            request: sample_request(6),
            reason: ActionStartFailureReason::PreconditionFailed("test".to_string()),
        };
        let mut runtime = runtime_with_plan(goal, step.clone());
        let mut jc = Some(jc_for_goal(goal));
        let mut blocked = BlockerMemory::default();
        let mut discrepancies = DiscrepancyMemory::default();
        let budget = ProfileFixture::default();

        handle_plan_failure(
            &PlanFailureContext {
                view: &view,
                agent,
                goal_key: goal,
                failed_step: &step,
                method_id: None,
                execution_failure: Some(ExecutionFailure::Start(&start_failure)),
                belief_discrepancy: None,
                current_tick: Tick(20),
            },
            &mut runtime,
            &mut jc,
            &mut blocked,
            &mut discrepancies,
            &mut ContentionIntents::default(),
            &cognitive(&budget),
        );

        assert!(blocked.intents.is_empty());
        let entry = discrepancies.entries.values().next().unwrap();
        assert_eq!(entry.discrepancy, Discrepancy::ImproperPlanningState);
        assert_eq!(entry.scope.exact_action_def(), Some(step.def_id));
    }

    #[test]
    fn clear_resolved_blockers_removes_restored_and_expired_entries() {
        let agent = entity(1);
        let place = entity(10);
        let seller = entity(2);
        let workstation = entity(3);
        let goal = trade_goal();
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.entity_kinds.insert(workstation, EntityKind::Facility);
        view.effective_places.insert(agent, place);
        view.commodity_quantities
            .insert((seller, CommodityKind::Bread), Quantity(2));
        view.commodity_quantities
            .insert((place, CommodityKind::Bread), Quantity(2));

        let mut blocked = BlockerMemory::default();
        let bk1 = BlockerKey {
            goal_key: goal,
            place: Some(place),
            target: Some(seller),
            action_def: Some(ActionDefId(1)),
        };
        blocked.record(Blocker {
            scope: bk1.into(),
            blocking_fact: BlockingFact::SellerOutOfStock,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(30),
            clearing_condition:
                worldwake_core::BlockerClearingCondition::CommodityAvailabilityChanged {
                    commodity: CommodityKind::Bread,
                    place,
                },
            baseline_snapshot: Some(ClearingBaseline::CommodityQuantity {
                quantity: Quantity(0),
            }),
            source_event: None,
        });
        let bk2 = BlockerKey {
            goal_key: GoalKey::from(GoalKind::ProduceCommodity {
                recipe_id: RecipeId(4),
            }),
            place: Some(place),
            target: Some(workstation),
            action_def: Some(ActionDefId(3)),
        };
        blocked.record(Blocker {
            scope: bk2.into(),
            blocking_fact: BlockingFact::WorkstationBusy,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(30),
            clearing_condition: worldwake_core::BlockerClearingCondition::ContentionChanged {
                facility: workstation,
            },
            baseline_snapshot: Some(ClearingBaseline::ContentionPosition(Some(2))),
            source_event: None,
        });
        let bk3 = BlockerKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            place: None,
            target: None,
            action_def: None,
        };
        blocked.record(Blocker {
            scope: bk3.into(),
            blocking_fact: BlockingFact::NoKnownPath,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(5),
            clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
            source_event: None,
        });

        view.facility_grants.insert(
            workstation,
            ContentionGrant {
                actor: agent,
                intended_action: ActionDefId(3),
                granted_at: Tick(9),
                expires_at: Tick(12),
            },
        );

        clear_resolved_blockers(&view, agent, &mut blocked, Tick(10));
        assert_eq!(blocked.intents.len(), 0);
    }

    #[test]
    fn pursuit_arrival_failure_records_target_gone_when_not_colocated() {
        let agent = entity(1);
        let target = entity(2);
        let agent_place = entity(10);
        let target_place = entity(11);

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, target]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(target, EntityKind::Agent);
        // Agent at agent_place, target at target_place → not co-located.
        view.effective_places.insert(agent, agent_place);
        view.effective_places.insert(target, target_place);

        let fact = derive_blocking_fact(
            &view,
            agent,
            &GoalKey::from(GoalKind::RaidTarget { target }),
            &attack_step(target),
            None,
        );
        assert_eq!(fact, BlockingFact::TargetGone);
    }

    #[test]
    fn pursuit_target_gone_blocker_does_not_auto_resolve() {
        let agent = entity(1);
        let target = entity(2);
        let agent_place = entity(10);
        let target_place = entity(11);

        let mut view = TestBeliefView::default();
        view.alive.extend([agent, target]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(target, EntityKind::Agent);
        view.effective_places.insert(agent, agent_place);
        view.effective_places.insert(target, target_place);

        let goal = GoalKey::from(GoalKind::RaidTarget { target });

        let mut blocked = BlockerMemory::default();
        blocked.record(Blocker {
            scope: BlockerKey {
                goal_key: goal,
                place: Some(agent_place),
                target: Some(target),
                action_def: Some(ActionDefId(4)),
            }
            .into(),
            blocking_fact: BlockingFact::TargetGone,
            diagnostic_context: None,
            observed_tick: Tick(5),
            expires_tick: Tick(50),
            clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
            source_event: None,
        });

        // The blocker should NOT auto-resolve even though the target entity
        // still exists — pursuit TargetGone relies on TTL expiry.
        clear_resolved_blockers(&view, agent, &mut blocked, Tick(10));
        assert_eq!(
            blocked.intents.len(),
            1,
            "pursuit TargetGone blocker should not auto-resolve"
        );
    }

    // ── Regression: derive_discrepancy_clearing scope ─────────────────────
    //
    // ReobservationOf clearing is only meaningful for discrepancy classes
    // that re-perceiving the target genuinely resolves. For other classes
    // (ImproperPlanningState, PartialExecutionDrift, SearchBudgetExhausted,
    // RouteUnknown, NoLegalBinding, NoWillingCounterparty), re-perceiving
    // the target proves nothing about whether the failure mode is resolved.
    // If we apply ReobservationOf to those classes, the entry clears the
    // moment the agent travels back into observation range, immediately
    // re-enabling the same broken plan and producing oscillation loops in
    // survival scenarios. These tests pin the scope of `ReobservationOf`
    // application against accidental regression.

    fn discrepancy_blocker_key_with_target(target: EntityId) -> BlockerKey {
        BlockerKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            place: None,
            target: Some(target),
            action_def: Some(ActionDefId(1)),
        }
    }

    fn discrepancy_blocker_key_without_target() -> BlockerKey {
        BlockerKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            place: None,
            target: None,
            action_def: Some(ActionDefId(1)),
        }
    }

    #[test]
    fn discrepancy_clearing_is_ttl_expiry_for_planner_state_classes() {
        let target = EntityId {
            slot: 7,
            generation: 0,
        };
        let key = discrepancy_blocker_key_with_target(target);

        // Classes where re-perceiving the target does NOT validate
        // resolution must use TtlExpiry, even when a target is present.
        for discrepancy in [
            Discrepancy::ImproperPlanningState,
            Discrepancy::PartialExecutionDrift,
            Discrepancy::SearchBudgetExhausted,
            Discrepancy::RouteUnknown,
            Discrepancy::NoLegalBinding,
            Discrepancy::NoWillingCounterparty,
            Discrepancy::MethodFailure(worldwake_core::MethodFailureContext {
                method_id: worldwake_core::MethodSchemaId(7),
                kind: worldwake_core::MethodFailureKind::SubgoalUnachievable,
                subgoal_index: Some(2),
            }),
        ] {
            assert_eq!(
                super::derive_discrepancy_clearing(discrepancy, &key, None),
                worldwake_core::DiscrepancyClearing::TtlExpiry,
                "{discrepancy:?} with target should clear by TTL, not ReobservationOf",
            );
        }
    }

    #[test]
    fn discrepancy_clearing_uses_reobservation_for_perceptive_classes() {
        let target = EntityId {
            slot: 7,
            generation: 0,
        };
        let key = discrepancy_blocker_key_with_target(target);

        // Classes where re-perception genuinely supersedes the discrepancy
        // continue to use ReobservationOf when a target is present.
        for discrepancy in [
            Discrepancy::BeliefStale,
            Discrepancy::BeliefContradicted,
            Discrepancy::Omission(OmissionReason::OverBudget {
                budget: 5,
                candidates_seen: 10,
            }),
            Discrepancy::MissingObservation,
        ] {
            assert_eq!(
                super::derive_discrepancy_clearing(discrepancy, &key, None),
                worldwake_core::DiscrepancyClearing::ReobservationOf { target },
                "{discrepancy:?} with target should clear on re-observation",
            );
        }
    }

    #[test]
    fn discrepancy_clearing_is_ttl_expiry_for_targetless_entries_in_all_classes() {
        let key = discrepancy_blocker_key_without_target();

        // Without a target, every discrepancy class falls through to
        // TtlExpiry — the only viable clearing path.
        for discrepancy in [
            Discrepancy::BeliefStale,
            Discrepancy::BeliefContradicted,
            Discrepancy::ImproperPlanningState,
            Discrepancy::PartialExecutionDrift,
            Discrepancy::SearchBudgetExhausted,
            Discrepancy::RouteUnknown,
            Discrepancy::NoLegalBinding,
            Discrepancy::NoWillingCounterparty,
            Discrepancy::Omission(OmissionReason::OverBudget {
                budget: 5,
                candidates_seen: 10,
            }),
            Discrepancy::MissingObservation,
        ] {
            assert_eq!(
                super::derive_discrepancy_clearing(discrepancy, &key, None),
                worldwake_core::DiscrepancyClearing::TtlExpiry,
                "{discrepancy:?} without target must use TtlExpiry",
            );
        }
    }

    #[test]
    fn commodity_availability_clearing_triggers_on_fresh_local_source() {
        let agent = EntityId {
            slot: 1,
            generation: 0,
        };
        let place = EntityId {
            slot: 2,
            generation: 0,
        };
        let source = EntityId {
            slot: 3,
            generation: 0,
        };
        let mut view = TestBeliefView::default();
        view.effective_places.insert(agent, place);
        view.entities_at.insert(place, vec![source]);
        view.resource_sources.insert(
            source,
            ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(2),
                max_quantity: Quantity(2),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
                extraction_slots: std::num::NonZeroU8::new(1).unwrap(),
                extraction_duration_ticks: std::num::NonZeroU32::new(1).unwrap(),
            },
        );

        let entry = DiscrepancyEntry {
            scope: discrepancy_blocker_key_with_target(source).into(),
            discrepancy: Discrepancy::BeliefContradicted,
            observed_tick: Tick(10),
            expires_tick: Tick(200),
            source: DiscrepancySource::ReadPhaseInference,
            clearing_condition: DiscrepancyClearing::CommodityAvailabilityChanged {
                commodity: CommodityKind::Apple,
                place,
            },
        };

        assert!(super::is_discrepancy_cleared(&view, agent, &entry));
    }
}
