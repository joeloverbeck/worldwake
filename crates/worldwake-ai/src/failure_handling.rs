use crate::{AgentDecisionRuntime, DirtySet, PlannedStep, PlannerOpKind, authoritative_target};
use worldwake_core::{
    BlockedIntent, BlockedIntentMemory, BlockerClearingCondition, BlockerDiagnostic, BlockerKey,
    BlockingFact, CognitiveProfile, CommodityKind, EntityId, GoalKey, GoalKind, IntentionFrame,
    Quantity, Tick,
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
    pub execution_failure: Option<ExecutionFailure<'a>>,
    pub current_tick: Tick,
}

pub fn handle_plan_failure(
    context: &PlanFailureContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    jc: &mut Option<IntentionFrame>,
    blocked_memory: &mut BlockedIntentMemory,
    cognitive: &CognitiveProfile,
) {
    runtime.current_plan = None;
    if jc.is_some() {
        runtime.last_frame_clear_reason = Some(worldwake_core::FrameClearReason::PlanFailed);
    }
    *jc = None;
    runtime.materialization_bindings.clear();

    let blocking_fact = derive_blocking_fact(
        context.view,
        context.agent,
        &context.goal_key,
        context.failed_step,
        context.execution_failure,
    );
    let expires_tick =
        context.current_tick + u64::from(blocking_fact_ttl(blocking_fact, cognitive));

    let blocker_key = BlockerKey {
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

    let diagnostic_context = if matches!(blocking_fact, BlockingFact::Unknown) {
        Some(BlockerDiagnostic {
            action_def: context.failed_step.def_id,
        })
    } else {
        None
    };

    blocked_memory.record(BlockedIntent {
        blocker_key,
        blocking_fact,
        diagnostic_context,
        observed_tick: context.current_tick,
        expires_tick,
        clearing_condition: BlockerClearingCondition::TtlOnly,
        baseline_snapshot: None,
    });
    runtime.dirty.insert(DirtySet::REPLAN_SIGNAL);
}

pub fn clear_resolved_blockers(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    blocked_memory: &mut BlockedIntentMemory,
    current_tick: Tick,
) {
    blocked_memory.expire(current_tick);
    blocked_memory.sweep_cleared(|intent| blocker_resolved(view, agent, intent));
}

fn derive_blocking_fact(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    goal_key: &GoalKey,
    step: &PlannedStep,
    execution_failure: Option<ExecutionFailure<'_>>,
) -> BlockingFact {
    if target_gone(view, agent, step) {
        return BlockingFact::TargetGone;
    }

    match step.op_kind {
        PlannerOpKind::Travel => {
            if no_known_path(view, agent, step) {
                return BlockingFact::NoKnownPath;
            }
        }
        PlannerOpKind::Trade | PlannerOpKind::StaffMarket | PlannerOpKind::StockManagement => {
            if let Some(fact) =
                classify_trade_failure(view, agent, goal_key, step, execution_failure)
            {
                return fact;
            }
        }
        PlannerOpKind::Harvest | PlannerOpKind::Craft => {
            if let Some(fact) = classify_production_failure(view, agent, step) {
                return fact;
            }
        }
        PlannerOpKind::Consume | PlannerOpKind::Wash | PlannerOpKind::Heal => {
            if let Some(fact) = classify_input_failure(view, agent, goal_key, step) {
                return fact;
            }
        }
        PlannerOpKind::Attack | PlannerOpKind::Defend => {
            if combat_too_risky(view, agent) {
                return BlockingFact::CombatTooRisky;
            }
        }
        PlannerOpKind::Patrol
        | PlannerOpKind::Sleep
        | PlannerOpKind::Relieve
        | PlannerOpKind::EstablishCamp
        | PlannerOpKind::QueueForFacilityUse
        | PlannerOpKind::MoveCargo
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
        | PlannerOpKind::ClaimBounty
        | PlannerOpKind::PostBounty
        | PlannerOpKind::PostNotice => {}
    }

    if danger_too_high(view, agent) {
        return BlockingFact::DangerTooHigh;
    }

    if let Some(fact) = execution_failure.and_then(map_execution_failure) {
        return fact;
    }

    BlockingFact::Unknown
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
        return Some(BlockingFact::ReservationConflict);
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
        PlannerOpKind::Wash => Some(CommodityKind::Water),
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
        | PlannerOpKind::ClaimBounty
        | PlannerOpKind::PostBounty
        | PlannerOpKind::PostNotice => None,
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
        | PlannerOpKind::Loot
        | PlannerOpKind::Bury
        | PlannerOpKind::Harvest
        | PlannerOpKind::Craft
        | PlannerOpKind::ClaimBounty
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
        | PlannerOpKind::AskWitness => view.entity_kind(target).is_none() || view.is_dead(target),
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

fn map_execution_failure(failure: ExecutionFailure<'_>) -> Option<BlockingFact> {
    match failure {
        ExecutionFailure::Replan(signal) => map_replan_abort_reason(signal),
        ExecutionFailure::Start(failure) => map_start_failure_reason(&failure.reason),
    }
}

fn map_replan_abort_reason(signal: &ReplanNeeded) -> Option<BlockingFact> {
    match &signal.reason {
        AbortReason::CommitConditionFailed { condition } => match condition {
            worldwake_sim::Precondition::TargetAdjacentToActor(_) => {
                Some(BlockingFact::NoKnownPath)
            }
            worldwake_sim::Precondition::TargetLacksProductionJob(_) => {
                Some(BlockingFact::WorkstationBusy)
            }
            worldwake_sim::Precondition::TargetHasResourceSource { .. } => {
                Some(BlockingFact::SourceDepleted)
            }
            _ => None,
        },
        AbortReason::Interrupted { kind, detail } => match kind {
            InterruptReason::DangerNearby => Some(BlockingFact::DangerTooHigh),
            InterruptReason::Reprioritized => None,
            InterruptReason::Other => detail.as_deref().and_then(parse_abort_detail),
        },
        AbortReason::ExternalAbort { kind, detail } => match kind {
            ExternalAbortReason::TargetDestroyed => Some(BlockingFact::TargetGone),
            ExternalAbortReason::ActorMarkedDead | ExternalAbortReason::CancelledByInput { .. } => {
                None
            }
            ExternalAbortReason::HandlerRequested { reason } => map_handler_abort_reason(reason),
            ExternalAbortReason::Other => detail.as_deref().and_then(parse_abort_detail),
        },
    }
}

fn map_start_failure_reason(reason: &ActionStartFailureReason) -> Option<BlockingFact> {
    match reason {
        ActionStartFailureReason::ReservationUnavailable(_) => {
            Some(BlockingFact::ReservationConflict)
        }
        ActionStartFailureReason::PreconditionFailed(detail) => parse_abort_detail(detail),
        ActionStartFailureReason::InvalidTarget(_) => Some(BlockingFact::TargetGone),
        ActionStartFailureReason::AbortRequested(reason) => map_handler_abort_reason(reason),
    }
}

fn map_handler_abort_reason(reason: &ActionAbortRequestReason) -> Option<BlockingFact> {
    match reason {
        ActionAbortRequestReason::PayloadEntityMismatch { .. }
        | ActionAbortRequestReason::TargetNotColocated { .. }
        | ActionAbortRequestReason::TargetNotDead { .. }
        | ActionAbortRequestReason::TargetNotAlive { .. }
        | ActionAbortRequestReason::TargetIncapacitated { .. } => Some(BlockingFact::TargetGone),
        ActionAbortRequestReason::ActorAlreadyHasCombatStance { .. }
        | ActionAbortRequestReason::CommodityNotCombatWeapon { .. }
        | ActionAbortRequestReason::ActorMissingCombatProfile { .. }
        | ActionAbortRequestReason::TargetMissingCombatProfile { .. } => {
            Some(BlockingFact::CombatTooRisky)
        }
        ActionAbortRequestReason::ActorNotPlaced { .. } => Some(BlockingFact::NoKnownPath),
        ActionAbortRequestReason::TargetLacksWounds { .. }
        | ActionAbortRequestReason::TargetHasNoWounds { .. }
        | ActionAbortRequestReason::SelfTargetForbidden { .. } => Some(BlockingFact::Unknown),
        ActionAbortRequestReason::ActorMissingWeaponCommodity { commodity, .. }
        | ActionAbortRequestReason::HolderLacksAccessibleCommodity { commodity, .. } => {
            Some(BlockingFact::MissingInput(*commodity))
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
        ActionAbortRequestReason::SaleLotNotListed { .. }
        | ActionAbortRequestReason::SaleLotNotPossessedBySeller { .. } => {
            Some(BlockingFact::SellerOutOfStock)
        }
    }
}

fn parse_abort_detail(detail: &str) -> Option<BlockingFact> {
    let detail = detail.to_ascii_lowercase();
    if detail.contains("danger") {
        Some(BlockingFact::DangerTooHigh)
    } else if detail.contains("risk") || detail.contains("combat") {
        Some(BlockingFact::CombatTooRisky)
    } else if detail.contains("reservation") {
        Some(BlockingFact::ReservationConflict)
    } else if detail.contains("seller") || detail.contains("stock") {
        Some(BlockingFact::SellerOutOfStock)
    } else if detail.contains("path") || detail.contains("route") {
        Some(BlockingFact::NoKnownPath)
    } else if detail.contains("destroyed") || detail.contains("gone") {
        Some(BlockingFact::TargetGone)
    } else if detail.contains("contention") || detail.contains("grant") || detail.contains("queue")
    {
        Some(BlockingFact::ExclusiveFacilityUnavailable)
    } else {
        None
    }
}

fn blocker_resolved(view: &dyn RuntimeBeliefView, agent: EntityId, intent: &BlockedIntent) -> bool {
    match intent.blocking_fact {
        BlockingFact::NoKnownPath => {
            let Some(target_place) = intent.blocker_key.place else {
                return false;
            };
            let Some(current_place) = view.effective_place(agent) else {
                return false;
            };
            view.adjacent_places_with_travel_ticks(current_place)
                .into_iter()
                .any(|(adjacent, _)| adjacent == target_place)
        }
        BlockingFact::NoKnownSeller => {
            let Some(commodity) = intent.blocker_key.goal_key.commodity else {
                return false;
            };
            let Some(current_place) = view.effective_place(agent) else {
                return false;
            };
            view.listed_sale_lots_at(current_place, commodity)
                .into_iter()
                .any(|lot| {
                    view.seller_for_sale_lot(lot)
                        .is_some_and(|seller| seller != agent)
                })
        }
        BlockingFact::SellerOutOfStock => {
            let Some(seller) = intent.blocker_key.target else {
                return false;
            };
            let Some(commodity) = intent.blocker_key.goal_key.commodity else {
                return false;
            };
            view.entity_kind(seller).is_some()
                && view.commodity_quantity(seller, commodity) > Quantity(0)
        }
        BlockingFact::TooExpensive => {
            view.commodity_quantity(agent, CommodityKind::Coin) > Quantity(0)
        }
        BlockingFact::ExclusiveFacilityUnavailable
        | BlockingFact::Unknown
        | BlockingFact::PatienceExhausted
        | BlockingFact::AssumptionFailed
        | BlockingFact::NoBuyer => false,
        BlockingFact::SourceDepleted => {
            let Some(source) = intent.blocker_key.target else {
                return false;
            };
            view.resource_source(source)
                .is_some_and(|resource| resource.available_quantity > Quantity(0))
        }
        BlockingFact::WorkstationBusy => intent
            .blocker_key
            .target
            .is_some_and(|workstation| !view.has_production_job(workstation)),
        BlockingFact::ReservationConflict => intent
            .blocker_key
            .target
            .is_some_and(|entity| view.reservation_ranges(entity).is_empty()),
        BlockingFact::MissingTool(kind) => view.unique_item_count(agent, kind) > 0,
        BlockingFact::MissingInput(commodity) => {
            view.commodity_quantity(agent, commodity) > Quantity(0)
        }
        BlockingFact::TargetGone => match intent.blocker_key.goal_key.kind {
            GoalKind::TreatWounds { .. } | GoalKind::ReduceDanger => intent
                .blocker_key
                .target
                .is_some_and(|entity| view.entity_kind(entity).is_some() && view.is_alive(entity)),
            // Pursuit arrival failure: target was alive but not co-located.
            // Do not auto-resolve — let the TTL expire so repeated pursuit
            // at the same stale place is suppressed.
            GoalKind::RaidTarget { .. } | GoalKind::EngageHostile { .. } => false,
            _ => intent
                .blocker_key
                .target
                .is_some_and(|entity| view.entity_kind(entity).is_some()),
        },
        BlockingFact::DangerTooHigh | BlockingFact::CombatTooRisky => {
            view.current_attackers_of(agent).is_empty()
                && view.visible_hostiles_for(agent).is_empty()
        }
    }
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
        | PlannerOpKind::PostBounty
        | PlannerOpKind::PostNotice => None,
        PlannerOpKind::Bury
        | PlannerOpKind::Consume
        | PlannerOpKind::QueueForFacilityUse
        | PlannerOpKind::Harvest
        | PlannerOpKind::Craft
        | PlannerOpKind::MoveCargo
        | PlannerOpKind::Heal
        | PlannerOpKind::Tell
        | PlannerOpKind::ConsultRecord
        | PlannerOpKind::Defend
        | PlannerOpKind::AskWitness
        | PlannerOpKind::Accuse
        | PlannerOpKind::Fine
        | PlannerOpKind::Exile
        | PlannerOpKind::ClaimBounty => {
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
        | PlannerOpKind::MoveCargo => view.effective_place(agent).or(goal_key.place),
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
        | PlannerOpKind::ClaimBounty
        | PlannerOpKind::PostBounty
        | PlannerOpKind::PostNotice => view.effective_place(agent),
    }
}

fn blocking_fact_ttl(fact: BlockingFact, cognitive: &CognitiveProfile) -> u32 {
    match fact {
        BlockingFact::SellerOutOfStock
        | BlockingFact::WorkstationBusy
        | BlockingFact::ReservationConflict
        | BlockingFact::ExclusiveFacilityUnavailable
        | BlockingFact::TargetGone => cognitive.transient_block_ticks,
        BlockingFact::Unknown => cognitive.unknown_block_ticks,
        BlockingFact::NoKnownPath
        | BlockingFact::NoKnownSeller
        | BlockingFact::TooExpensive
        | BlockingFact::SourceDepleted
        | BlockingFact::MissingTool(_)
        | BlockingFact::MissingInput(_)
        | BlockingFact::DangerTooHigh
        | BlockingFact::CombatTooRisky
        | BlockingFact::PatienceExhausted
        | BlockingFact::AssumptionFailed
        | BlockingFact::NoBuyer => cognitive.structural_block_ticks,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ExecutionFailure, PlanFailureContext, blocking_fact_ttl, clear_resolved_blockers,
        derive_blocking_fact, handle_plan_failure,
    };
    use crate::{
        AgentDecisionRuntime, HypotheticalEntityId, PlanTerminalKind, PlannedPlan, PlannedStep,
        PlannerOpKind, PlanningEntityRef, ProfileFixture,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, BlockedIntent, BlockedIntentMemory, BlockerKey, BlockingFact,
        CognitiveProfile, CombatProfile, CommodityConsumableProfile, CommodityKind,
        CommodityPurpose, DemandObservation, DriveThresholds, EntityId, EntityKind, FrameState,
        GoalKey, GoalKind, HomeostaticNeeds, InTransitOnEdge, IntentionDomain, IntentionFrame,
        LoadUnits, MerchandiseProfile, MetabolismProfile, Quantity, RecipeId, ResourceSource, Tick,
        TickRange, TradeDispositionProfile, UniqueItemKind, WorkstationTag, Wound,
    };
    use worldwake_sim::{
        AbortReason, ActionAbortRequestReason, ActionDuration, ActionPayload, ActionStartFailure,
        ActionStartFailureReason, CombatActionPayload, CraftActionPayload,
        DeclareSupportActionPayload, DurationExpr, InterruptReason, ReplanNeeded,
        RequestAttemptTrace, RequestBindingKind, RequestProvenance, ResolvedRequestTrace,
        RuntimeBeliefView, TradeActionPayload,
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
        wounds: BTreeMap<EntityId, Vec<Wound>>,
        attackers: BTreeMap<EntityId, Vec<EntityId>>,
        hostiles: BTreeMap<EntityId, Vec<EntityId>>,
        listed_lots: BTreeMap<(EntityId, CommodityKind), Vec<EntityId>>,
        lot_sellers: BTreeMap<EntityId, EntityId>,
        lot_commodities: BTreeMap<EntityId, CommodityKind>,
    }

    impl RuntimeBeliefView for TestBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }
        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.entity_kinds.get(&entity).copied()
        }
        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.effective_places.get(&entity).copied()
        }
        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }
        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.entities_at.get(&place).cloned().unwrap_or_default()
        }
        fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
            self.direct_possessions
                .get(&holder)
                .cloned()
                .unwrap_or_default()
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
        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }
        fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
            self.resource_sources.get(&entity).cloned()
        }
        fn has_production_job(&self, entity: EntityId) -> bool {
            self.production_jobs.contains(&entity)
        }
        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            true
        }
        fn has_control(&self, entity: EntityId) -> bool {
            self.entity_kinds.get(&entity) == Some(&EntityKind::Agent)
        }

        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }
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
        fn is_dead(&self, entity: EntityId) -> bool {
            self.dead.contains(&entity)
        }
        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }
        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds
                .get(&entity)
                .is_some_and(|wounds| !wounds.is_empty())
        }
        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }
        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }
        fn belief_confidence_policy(
            &self,
            _agent: EntityId,
        ) -> worldwake_core::BeliefConfidencePolicy {
            worldwake_core::BeliefConfidencePolicy::default()
        }
        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }
        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }
        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::IntentionDispositionProfile> {
            None
        }
        fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
            false
        }
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
        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
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
                    self.resource_source(*entity)
                        .is_some_and(|source| source.commodity == commodity)
                })
                .collect()
        }
        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }
        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }
        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
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

    fn cognitive(reasoning: &ProfileFixture) -> CognitiveProfile {
        CognitiveProfile {
            max_candidates_to_plan: reasoning.max_candidates_to_plan,
            max_plan_depth: reasoning.max_plan_depth,
            snapshot_travel_horizon: reasoning.snapshot_travel_horizon,
            max_node_expansions: reasoning.max_node_expansions,
            switch_margin: reasoning.switch_margin,
            transient_block_ticks: reasoning.transient_block_ticks,
            unknown_block_ticks: reasoning.unknown_block_ticks,
            structural_block_ticks: reasoning.structural_block_ticks,
            initial_cooldown_ticks: reasoning.initial_cooldown_ticks,
            max_cooldown_ticks: reasoning.max_cooldown_ticks,
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
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
        })
    }

    fn trade_step(counterparty: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(1),
            targets: vec![PlanningEntityRef::Authoritative(counterparty)],
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
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 2,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
        }
    }

    fn craft_step(workstation: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(3),
            targets: vec![PlanningEntityRef::Authoritative(workstation)],
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
        }
    }

    fn claim_office_goal(office: EntityId) -> GoalKey {
        GoalKey::from(GoalKind::ClaimOffice { office })
    }

    fn declare_support_step(office: EntityId, candidate: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(6),
            targets: Vec::new(),
            payload_override: Some(ActionPayload::DeclareSupport(DeclareSupportActionPayload {
                office,
                candidate,
            })),
            op_kind: PlannerOpKind::DeclareSupport,
            estimated_ticks: 1,
            is_materialization_barrier: true,
            expected_materializations: Vec::new(),
        }
    }

    fn attack_step(target: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(4),
            targets: vec![PlanningEntityRef::Authoritative(target)],
            payload_override: Some(ActionPayload::Combat(CombatActionPayload {
                target,
                weapon: worldwake_core::CombatWeaponRef::Unarmed,
            })),
            op_kind: PlannerOpKind::Attack,
            estimated_ticks: 0,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
        }
    }

    fn hypothetical_consume_step() -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(5),
            targets: vec![PlanningEntityRef::Hypothetical(HypotheticalEntityId(9))],
            payload_override: None,
            op_kind: PlannerOpKind::Consume,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
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
                PlanTerminalKind::ProgressBarrier,
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
        let mut blocked = BlockedIntentMemory::default();

        handle_plan_failure(
            &PlanFailureContext {
                view: &view,
                agent,
                goal_key: goal,
                failed_step: &step,
                execution_failure: None,
                current_tick: Tick(20),
            },
            &mut runtime,
            &mut jc,
            &mut blocked,
            &cognitive(&ProfileFixture::default()),
        );

        assert_eq!(runtime.current_plan, None);
        assert!(!runtime.dirty.is_empty());
        assert!(jc.is_none());
        assert_eq!(blocked.intents.len(), 1);
        let intent = blocked.intents.values().next().unwrap();
        assert_eq!(intent.blocking_fact, BlockingFact::SellerOutOfStock);
        assert_eq!(intent.blocker_key.target, Some(seller));
        assert_eq!(intent.blocker_key.place, Some(place));
        assert_eq!(intent.blocker_key.action_def, Some(ActionDefId(1)));
        assert_eq!(
            intent.expires_tick,
            Tick(20 + u64::from(ProfileFixture::default().transient_block_ticks))
        );
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
        assert_eq!(fact, BlockingFact::ReservationConflict);
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

        assert_eq!(fact, BlockingFact::ReservationConflict);
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
        let mut blocked = BlockedIntentMemory::default();
        let budget = ProfileFixture::default();

        handle_plan_failure(
            &PlanFailureContext {
                view: &view,
                agent,
                goal_key: goal,
                failed_step: &step,
                execution_failure: Some(ExecutionFailure::Start(&start_failure)),
                current_tick: Tick(20),
            },
            &mut runtime,
            &mut jc,
            &mut blocked,
            &cognitive(&budget),
        );

        assert_eq!(runtime.current_plan, None);
        assert!(!runtime.dirty.is_empty());
        assert_eq!(blocked.intents.len(), 1);
        let intent = blocked.intents.values().next().unwrap();
        assert_eq!(intent.blocking_fact, BlockingFact::Unknown);
        assert_eq!(intent.blocker_key.target, Some(office));
        assert_eq!(intent.blocker_key.place, Some(place));
        assert_eq!(intent.blocker_key.action_def, Some(ActionDefId(6)));
        assert_eq!(
            intent.expires_tick,
            Tick(20 + u64::from(budget.unknown_block_ticks))
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
            payload_override: None,
            op_kind: PlannerOpKind::Sleep,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
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
        assert_eq!(
            blocking_fact_ttl(BlockingFact::Unknown, &cognitive(&budget)),
            budget.unknown_block_ticks
        );
    }

    #[test]
    fn unknown_blocker_uses_dedicated_ttl() {
        let budget = ProfileFixture::default();
        let ttl = blocking_fact_ttl(BlockingFact::Unknown, &cognitive(&budget));
        assert_eq!(ttl, 5);
        assert_ne!(ttl, budget.transient_block_ticks);
    }

    #[test]
    fn transient_blockers_unchanged_ttl() {
        let budget = ProfileFixture::default();
        let transient_facts = [
            BlockingFact::SellerOutOfStock,
            BlockingFact::WorkstationBusy,
            BlockingFact::ReservationConflict,
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
        let mut blocked = BlockedIntentMemory::default();
        let budget = ProfileFixture::default();

        handle_plan_failure(
            &PlanFailureContext {
                view: &view,
                agent,
                goal_key: goal,
                failed_step: &step,
                execution_failure: Some(ExecutionFailure::Start(&start_failure)),
                current_tick: Tick(20),
            },
            &mut runtime,
            &mut jc,
            &mut blocked,
            &cognitive(&budget),
        );

        let intent = blocked.intents.values().next().unwrap();
        assert_eq!(intent.blocking_fact, BlockingFact::Unknown);
        let diag = intent
            .diagnostic_context
            .expect("Unknown blocker must have diagnostic_context");
        assert_eq!(diag.action_def, step.def_id);
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

        let mut blocked = BlockedIntentMemory::default();
        let bk1 = BlockerKey {
            goal_key: goal,
            place: Some(place),
            target: Some(seller),
            action_def: Some(ActionDefId(1)),
        };
        blocked.record(BlockedIntent {
            blocker_key: bk1,
            blocking_fact: BlockingFact::SellerOutOfStock,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(30),
            clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
        });
        let bk2 = BlockerKey {
            goal_key: GoalKey::from(GoalKind::ProduceCommodity {
                recipe_id: RecipeId(4),
            }),
            place: Some(place),
            target: Some(workstation),
            action_def: Some(ActionDefId(3)),
        };
        blocked.record(BlockedIntent {
            blocker_key: bk2,
            blocking_fact: BlockingFact::WorkstationBusy,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(30),
            clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
        });
        let bk3 = BlockerKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            place: None,
            target: None,
            action_def: None,
        };
        blocked.record(BlockedIntent {
            blocker_key: bk3,
            blocking_fact: BlockingFact::Unknown,
            diagnostic_context: None,
            observed_tick: Tick(1),
            expires_tick: Tick(5),
            clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
        });

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

        let mut blocked = BlockedIntentMemory::default();
        blocked.record(BlockedIntent {
            blocker_key: BlockerKey {
                goal_key: goal,
                place: Some(agent_place),
                target: Some(target),
                action_def: Some(ActionDefId(4)),
            },
            blocking_fact: BlockingFact::TargetGone,
            diagnostic_context: None,
            observed_tick: Tick(5),
            expires_tick: Tick(50),
            clearing_condition: worldwake_core::BlockerClearingCondition::TtlOnly,
            baseline_snapshot: None,
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
}
