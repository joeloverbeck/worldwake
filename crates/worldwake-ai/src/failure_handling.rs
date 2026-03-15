use crate::{
    authoritative_target, AgentDecisionRuntime, JourneyClearReason, PlannedStep, PlannerOpKind,
    PlanningBudget,
};
use worldwake_core::{
    BlockedIntent, BlockedIntentMemory, BlockingFact, CommodityKind, EntityId, GoalKey, GoalKind,
    Quantity, Tick,
};
use worldwake_sim::{
    AbortReason, ActionAbortRequestReason, ActionPayload, ExternalAbortReason, InterruptReason,
    ReplanNeeded, RuntimeBeliefView,
};

pub struct PlanFailureContext<'a> {
    pub view: &'a dyn RuntimeBeliefView,
    pub agent: EntityId,
    pub goal_key: GoalKey,
    pub failed_step: &'a PlannedStep,
    pub replan_signal: Option<&'a ReplanNeeded>,
    pub current_tick: Tick,
}

pub fn handle_plan_failure(
    context: &PlanFailureContext<'_>,
    runtime: &mut AgentDecisionRuntime,
    blocked_memory: &mut BlockedIntentMemory,
    budget: &PlanningBudget,
) {
    runtime.current_plan = None;
    runtime.clear_journey_commitment_with_reason(JourneyClearReason::PlanFailed);
    runtime.materialization_bindings.clear();

    let blocking_fact = derive_blocking_fact(
        context.view,
        context.agent,
        &context.goal_key,
        context.failed_step,
        context.replan_signal,
    );
    let expires_tick = context.current_tick + u64::from(blocking_fact_ttl(blocking_fact, budget));

    blocked_memory.record(BlockedIntent {
        goal_key: context.goal_key,
        blocking_fact,
        related_entity: related_entity(context.failed_step),
        related_place: related_place(
            context.view,
            context.agent,
            &context.goal_key,
            context.failed_step,
        ),
        related_action: None,
        observed_tick: context.current_tick,
        expires_tick,
    });
    runtime.dirty = true;
}

pub fn clear_resolved_blockers(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    blocked_memory: &mut BlockedIntentMemory,
    current_tick: Tick,
) {
    blocked_memory.expire(current_tick);
    blocked_memory
        .intents
        .retain(|intent| !blocker_resolved(view, agent, intent));
}

fn derive_blocking_fact(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    goal_key: &GoalKey,
    step: &PlannedStep,
    replan_signal: Option<&ReplanNeeded>,
) -> BlockingFact {
    if target_gone(view, step) {
        return BlockingFact::TargetGone;
    }

    match step.op_kind {
        PlannerOpKind::Travel => {
            if no_known_path(view, agent, step) {
                return BlockingFact::NoKnownPath;
            }
        }
        PlannerOpKind::Trade => {
            if let Some(fact) = classify_trade_failure(view, agent, goal_key, step) {
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
        PlannerOpKind::Sleep
        | PlannerOpKind::Relieve
        | PlannerOpKind::QueueForFacilityUse
        | PlannerOpKind::MoveCargo
        | PlannerOpKind::Loot
        | PlannerOpKind::Bury
        | PlannerOpKind::Tell => {}
    }

    if danger_too_high(view, agent) {
        return BlockingFact::DangerTooHigh;
    }

    if let Some(fact) = replan_signal.and_then(map_abort_reason) {
        return fact;
    }

    BlockingFact::Unknown
}

fn classify_trade_failure(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    goal_key: &GoalKey,
    step: &PlannedStep,
) -> Option<BlockingFact> {
    let payload = step.payload_override.as_ref()?.as_trade()?;
    let commodity = goal_key.commodity.unwrap_or(payload.requested_commodity);
    let place = view.effective_place(agent)?;

    if view.commodity_quantity(payload.counterparty, payload.requested_commodity)
        < payload.requested_quantity
    {
        return Some(BlockingFact::SellerOutOfStock);
    }

    if view.commodity_quantity(agent, payload.offered_commodity) < payload.offered_quantity {
        return Some(if payload.offered_commodity == CommodityKind::Coin {
            BlockingFact::TooExpensive
        } else {
            BlockingFact::MissingInput(payload.offered_commodity)
        });
    }

    let sellers = view
        .agents_selling_at(place, commodity)
        .into_iter()
        .filter(|seller| *seller != agent)
        .collect::<Vec<_>>();

    if sellers.is_empty() {
        return Some(BlockingFact::NoKnownSeller);
    }

    None
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
    {
        if let Some(missing_tool) = payload
            .required_tool_kinds
            .iter()
            .copied()
            .find(|tool| view.unique_item_count(agent, *tool) == 0)
        {
            return Some(BlockingFact::MissingTool(missing_tool));
        }
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
        | PlannerOpKind::Sleep
        | PlannerOpKind::Relieve
        | PlannerOpKind::Trade
        | PlannerOpKind::QueueForFacilityUse
        | PlannerOpKind::Harvest
        | PlannerOpKind::Craft
        | PlannerOpKind::MoveCargo
        | PlannerOpKind::Loot
        | PlannerOpKind::Bury
        | PlannerOpKind::Tell
        | PlannerOpKind::Attack
        | PlannerOpKind::Defend => None,
    }?;

    (view.commodity_quantity(agent, commodity) == Quantity(0))
        .then_some(BlockingFact::MissingInput(commodity))
}

fn target_gone(view: &dyn RuntimeBeliefView, step: &PlannedStep) -> bool {
    if matches!(step.op_kind, PlannerOpKind::Travel) {
        return false;
    }

    let Some(target) = related_entity(step) else {
        return false;
    };

    match step.op_kind {
        PlannerOpKind::Trade
        | PlannerOpKind::QueueForFacilityUse
        | PlannerOpKind::MoveCargo
        | PlannerOpKind::Loot
        | PlannerOpKind::Bury
        | PlannerOpKind::Harvest
        | PlannerOpKind::Craft => view.entity_kind(target).is_none(),
        PlannerOpKind::Consume
        | PlannerOpKind::Sleep
        | PlannerOpKind::Relieve
        | PlannerOpKind::Wash
        | PlannerOpKind::Heal
        | PlannerOpKind::Tell
        | PlannerOpKind::Attack
        | PlannerOpKind::Defend => view.entity_kind(target).is_none() || view.is_dead(target),
        PlannerOpKind::Travel => false,
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

fn map_abort_reason(signal: &ReplanNeeded) -> Option<BlockingFact> {
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

fn map_handler_abort_reason(reason: &ActionAbortRequestReason) -> Option<BlockingFact> {
    match reason {
        ActionAbortRequestReason::PayloadEntityMismatch { .. }
        | ActionAbortRequestReason::TargetNotColocated { .. }
        | ActionAbortRequestReason::TargetNotDead { .. }
        | ActionAbortRequestReason::TargetNotAlive { .. } => Some(BlockingFact::TargetGone),
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
    } else {
        None
    }
}

fn blocker_resolved(view: &dyn RuntimeBeliefView, agent: EntityId, intent: &BlockedIntent) -> bool {
    match intent.blocking_fact {
        BlockingFact::NoKnownPath => {
            let Some(target_place) = intent.related_place else {
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
            let Some(commodity) = intent.goal_key.commodity else {
                return false;
            };
            let Some(current_place) = view.effective_place(agent) else {
                return false;
            };
            view.agents_selling_at(current_place, commodity)
                .into_iter()
                .any(|seller| seller != agent)
        }
        BlockingFact::SellerOutOfStock => {
            let Some(seller) = intent.related_entity else {
                return false;
            };
            let Some(commodity) = intent.goal_key.commodity else {
                return false;
            };
            view.entity_kind(seller).is_some()
                && view.commodity_quantity(seller, commodity) > Quantity(0)
        }
        BlockingFact::TooExpensive => {
            view.commodity_quantity(agent, CommodityKind::Coin) > Quantity(0)
        }
        BlockingFact::ExclusiveFacilityUnavailable | BlockingFact::Unknown => false,
        BlockingFact::SourceDepleted => {
            let Some(source) = intent.related_entity else {
                return false;
            };
            view.resource_source(source)
                .is_some_and(|resource| resource.available_quantity > Quantity(0))
        }
        BlockingFact::WorkstationBusy => intent
            .related_entity
            .is_some_and(|workstation| !view.has_production_job(workstation)),
        BlockingFact::ReservationConflict => intent
            .related_entity
            .is_some_and(|entity| view.reservation_ranges(entity).is_empty()),
        BlockingFact::MissingTool(kind) => view.unique_item_count(agent, kind) > 0,
        BlockingFact::MissingInput(commodity) => {
            view.commodity_quantity(agent, commodity) > Quantity(0)
        }
        BlockingFact::TargetGone => match intent.goal_key.kind {
            GoalKind::Heal { .. } | GoalKind::ReduceDanger => intent
                .related_entity
                .is_some_and(|entity| view.entity_kind(entity).is_some() && view.is_alive(entity)),
            _ => intent
                .related_entity
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
        PlannerOpKind::Trade => step
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
        | PlannerOpKind::Sleep
        | PlannerOpKind::Relieve
        | PlannerOpKind::Wash => None,
        PlannerOpKind::Bury
        | PlannerOpKind::Consume
        | PlannerOpKind::QueueForFacilityUse
        | PlannerOpKind::Harvest
        | PlannerOpKind::Craft
        | PlannerOpKind::MoveCargo
        | PlannerOpKind::Heal
        | PlannerOpKind::Tell
        | PlannerOpKind::Defend => step.targets.first().copied().and_then(authoritative_target),
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
        | PlannerOpKind::Defend => goal_key.place.or_else(|| view.effective_place(agent)),
        PlannerOpKind::Tell => view.effective_place(agent),
    }
}

fn blocking_fact_ttl(fact: BlockingFact, budget: &PlanningBudget) -> u32 {
    match fact {
        BlockingFact::SellerOutOfStock
        | BlockingFact::WorkstationBusy
        | BlockingFact::ReservationConflict
        | BlockingFact::ExclusiveFacilityUnavailable
        | BlockingFact::TargetGone
        | BlockingFact::Unknown => budget.transient_block_ticks,
        BlockingFact::NoKnownPath
        | BlockingFact::NoKnownSeller
        | BlockingFact::TooExpensive
        | BlockingFact::SourceDepleted
        | BlockingFact::MissingTool(_)
        | BlockingFact::MissingInput(_)
        | BlockingFact::DangerTooHigh
        | BlockingFact::CombatTooRisky => budget.structural_block_ticks,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        blocking_fact_ttl, clear_resolved_blockers, derive_blocking_fact, handle_plan_failure,
        PlanFailureContext,
    };
    use crate::{
        AgentDecisionRuntime, HypotheticalEntityId, PlanTerminalKind, PlannedPlan, PlannedStep,
        PlannerOpKind, PlanningBudget, PlanningEntityRef,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, BlockedIntent, BlockedIntentMemory, BlockingFact, CombatProfile,
        CommodityConsumableProfile, CommodityKind, CommodityPurpose, DemandObservation,
        DriveThresholds, EntityId, EntityKind, GoalKey, GoalKind, HomeostaticNeeds,
        InTransitOnEdge, LoadUnits, MerchandiseProfile, MetabolismProfile, Quantity, RecipeId,
        ResourceSource, Tick, TickRange, TradeDispositionProfile, UniqueItemKind, WorkstationTag,
        Wound,
    };
    use worldwake_sim::{
        AbortReason, ActionDuration, ActionPayload, CombatActionPayload, CraftActionPayload,
        DurationExpr, InterruptReason, ReplanNeeded, RuntimeBeliefView, TradeActionPayload,
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
        sellers: BTreeMap<(EntityId, CommodityKind), Vec<EntityId>>,
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
        fn item_lot_commodity(&self, _entity: EntityId) -> Option<CommodityKind> {
            None
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
        fn travel_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::TravelDispositionProfile> {
            None
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
        fn agents_selling_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.sellers
                .get(&(place, commodity))
                .cloned()
                .unwrap_or_default()
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
            Some(ActionDuration::Finite(1))
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

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
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(1),
                requested_commodity: CommodityKind::Bread,
                requested_quantity: Quantity(1),
            })),
            op_kind: PlannerOpKind::Trade,
            estimated_ticks: 3,
            is_materialization_barrier: true,
            expected_materializations: Vec::new(),
        }
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
            current_goal: Some(goal),
            current_plan: Some(PlannedPlan::new(
                goal,
                vec![step],
                PlanTerminalKind::ProgressBarrier,
            )),
            journey_committed_goal: Some(goal),
            journey_committed_destination: Some(entity(99)),
            journey_established_at: Some(Tick(10)),
            dirty: false,
            last_priority_class: None,
            ..AgentDecisionRuntime::default()
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
        view.sellers
            .insert((place, CommodityKind::Bread), vec![seller]);
        view.commodity_quantities
            .insert((agent, CommodityKind::Coin), Quantity(1));
        let mut runtime = runtime_with_plan(goal, step.clone());
        let mut blocked = BlockedIntentMemory::default();

        handle_plan_failure(
            &PlanFailureContext {
                view: &view,
                agent,
                goal_key: goal,
                failed_step: &step,
                replan_signal: None,
                current_tick: Tick(20),
            },
            &mut runtime,
            &mut blocked,
            &PlanningBudget::default(),
        );

        assert_eq!(runtime.current_plan, None);
        assert!(runtime.dirty);
        assert_eq!(runtime.current_goal, Some(goal));
        assert_eq!(runtime.journey_committed_goal, None);
        assert_eq!(runtime.journey_committed_destination, None);
        assert_eq!(runtime.journey_established_at, None);
        assert!(blocked.is_blocked(&goal, Tick(20)));
        assert_eq!(blocked.intents.len(), 1);
        assert_eq!(
            blocked.intents[0].blocking_fact,
            BlockingFact::SellerOutOfStock
        );
        assert_eq!(blocked.intents[0].related_entity, Some(seller));
        assert_eq!(blocked.intents[0].related_place, Some(place));
        assert_eq!(
            blocked.intents[0].expires_tick,
            Tick(20 + u64::from(PlanningBudget::default().transient_block_ticks))
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
        view.sellers
            .insert((place, CommodityKind::Bread), vec![seller]);
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
    fn derive_blocking_fact_detects_no_known_seller_when_market_is_empty() {
        let agent = entity(1);
        let place = entity(10);
        let seller = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.extend([agent, seller]);
        view.entity_kinds.insert(agent, EntityKind::Agent);
        view.entity_kinds.insert(seller, EntityKind::Agent);
        view.effective_places.insert(agent, place);
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
            Some(&signal),
        );
        assert_eq!(fact, BlockingFact::DangerTooHigh);
    }

    #[test]
    fn blocking_fact_ttl_uses_budget_classification() {
        let budget = PlanningBudget::default();

        assert_eq!(
            blocking_fact_ttl(BlockingFact::SellerOutOfStock, &budget),
            budget.transient_block_ticks
        );
        assert_eq!(
            blocking_fact_ttl(BlockingFact::NoKnownSeller, &budget),
            budget.structural_block_ticks
        );
        assert_eq!(
            blocking_fact_ttl(BlockingFact::Unknown, &budget),
            budget.transient_block_ticks
        );
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

        let mut blocked = BlockedIntentMemory {
            intents: vec![
                BlockedIntent {
                    goal_key: goal,
                    blocking_fact: BlockingFact::SellerOutOfStock,
                    related_entity: Some(seller),
                    related_place: Some(place),
                    related_action: None,
                    observed_tick: Tick(1),
                    expires_tick: Tick(30),
                },
                BlockedIntent {
                    goal_key: GoalKey::from(GoalKind::ProduceCommodity {
                        recipe_id: RecipeId(4),
                    }),
                    blocking_fact: BlockingFact::WorkstationBusy,
                    related_entity: Some(workstation),
                    related_place: Some(place),
                    related_action: None,
                    observed_tick: Tick(1),
                    expires_tick: Tick(30),
                },
                BlockedIntent {
                    goal_key: GoalKey::from(GoalKind::Sleep),
                    blocking_fact: BlockingFact::Unknown,
                    related_entity: None,
                    related_place: None,
                    related_action: None,
                    observed_tick: Tick(1),
                    expires_tick: Tick(5),
                },
            ],
        };

        clear_resolved_blockers(&view, agent, &mut blocked, Tick(10));
        assert_eq!(blocked.intents.len(), 0);
    }
}
