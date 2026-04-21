use crate::{
    MaterializationBindings, PlannedPlan, PlannedStep, PlannerOpKind, authoritative_target,
    decision_trace::PursuitInvalidationReason, resolve_planning_targets_with,
};
use std::collections::BTreeSet;
use worldwake_core::{EntityId, GoalKind, Tick, belief_confidence};
use worldwake_sim::{
    ActionDefRegistry, ActionHandlerRegistry, Affordance, RuntimeBeliefView, TargetSpec,
    belief_view::BeliefStatus, evaluate_constraint, evaluate_precondition,
    get_affordances_for_defs, requested_affordance_matches,
};

#[must_use]
pub fn revalidate_next_step(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    step: &PlannedStep,
    bindings: &MaterializationBindings,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> bool {
    let Some(def) = registry.get(step.def_id) else {
        return false;
    };
    let Some(handler) = handlers.get(def.handler) else {
        return false;
    };
    let Some(targets) = resolve_planning_targets_with(&step.targets, |id| bindings.resolve(id))
    else {
        return false;
    };
    if exact_target_belief_contradicted(view, actor, &targets, def) {
        return false;
    }
    let single_def = BTreeSet::from([step.def_id]);
    let affordance_match = get_affordances_for_defs(view, actor, registry, handlers, &single_def)
        .into_iter()
        .any(|affordance| {
            requested_affordance_matches(
                &affordance,
                def,
                handler,
                actor,
                &targets,
                step.payload_override.as_ref(),
                view,
            )
        });
    affordance_match
        || revalidate_best_effort_payload_override_step(view, actor, step, &targets, def, handler)
        || revalidate_exact_target_step(view, actor, step, &targets, def, handler)
}

fn exact_target_belief_contradicted(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    targets: &[EntityId],
    def: &worldwake_sim::ActionDef,
) -> bool {
    def.targets.len() == targets.len()
        && def
            .targets
            .iter()
            .all(|spec| matches!(spec, TargetSpec::SpecificEntity(_)))
        && targets.iter().copied().any(|target| {
            view.believed_target_location(actor, target).status == BeliefStatus::Contradicted
        })
}

fn revalidate_best_effort_payload_override_step(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    step: &PlannedStep,
    targets: &[EntityId],
    def: &worldwake_sim::ActionDef,
    handler: &worldwake_sim::ActionHandler,
) -> bool {
    let Some(payload_override) = step.payload_override.as_ref() else {
        return false;
    };
    if !matches!(def.payload, worldwake_sim::ActionPayload::None) {
        return false;
    }
    if !def
        .actor_constraints
        .iter()
        .all(|constraint| evaluate_constraint(constraint, actor, view))
    {
        return false;
    }
    if !def
        .preconditions
        .iter()
        .copied()
        .all(|precondition| evaluate_precondition(precondition, actor, targets, view))
    {
        return false;
    }

    (handler.payload_override_is_valid)(def, actor, targets, payload_override, view)
}

fn revalidate_exact_target_step(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    step: &PlannedStep,
    targets: &[EntityId],
    def: &worldwake_sim::ActionDef,
    handler: &worldwake_sim::ActionHandler,
) -> bool {
    if def.targets.len() != targets.len()
        || !def
            .targets
            .iter()
            .all(|spec| matches!(spec, TargetSpec::SpecificEntity(_)))
    {
        return false;
    }

    let synthetic = Affordance {
        def_id: step.def_id,
        actor,
        bound_targets: targets.to_vec(),
        payload_override: None,
        explanation: None,
        contention_status: worldwake_core::ContentionStatus::Unmanaged,
    };
    requested_affordance_matches(
        &synthetic,
        def,
        handler,
        actor,
        targets,
        step.payload_override.as_ref(),
        view,
    )
}

/// Returns `true` if the given plan is a remote pursuit plan
/// (a `RaidTarget` or `EngageHostile` goal with Travel + Attack steps).
fn is_pursuit_plan(plan: &PlannedPlan) -> bool {
    let is_pursuit_goal = matches!(
        plan.goal.kind,
        GoalKind::RaidTarget { .. } | GoalKind::EngageHostile { .. }
    );
    if !is_pursuit_goal || plan.steps.len() < 2 {
        return false;
    }
    let has_travel = plan
        .steps
        .iter()
        .any(|s| s.op_kind == PlannerOpKind::Travel);
    let has_attack = plan
        .steps
        .iter()
        .any(|s| s.op_kind == PlannerOpKind::Attack);
    has_travel && has_attack
}

/// Extract the pursuit target from a pursuit plan's goal.
fn pursuit_target(plan: &PlannedPlan) -> Option<EntityId> {
    match plan.goal.kind {
        GoalKind::RaidTarget { target } | GoalKind::EngageHostile { target } => Some(target),
        _ => None,
    }
}

/// Extract the planned travel destination (the place the pursuit plan intends
/// to reach before attacking). This is the target of the last Travel step.
fn planned_pursuit_destination(plan: &PlannedPlan) -> Option<EntityId> {
    plan.steps
        .iter()
        .rev()
        .find(|s| s.op_kind == PlannerOpKind::Travel)
        .and_then(|s| s.targets.first().copied())
        .and_then(authoritative_target)
}

/// Check whether an active pursuit plan's underlying belief assumptions
/// are still valid. Returns `false` (plan invalid) when:
///
/// - The target's believed place has changed from the plan's destination
/// - Target is believed dead, place unknown, or now co-located
/// - Derived confidence has decayed below `min_location_confidence`
///
/// This is called during the dirty-flag phase each tick for agents with
/// active pursuit plans. Confidence is re-derived each tick, never cached.
///
/// Returns `None` when the plan is still valid, or `Some(reason)` when invalid.
#[must_use]
pub fn is_pursuit_plan_invalid(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    plan: &PlannedPlan,
    current_tick: Tick,
) -> Option<PursuitInvalidationReason> {
    if !is_pursuit_plan(plan) {
        return None;
    }
    let target = pursuit_target(plan)?;

    let Some(profile) = view.pursuit_profile(actor) else {
        return Some(PursuitInvalidationReason::NoProfile);
    };

    // Extract the target's believed state from the actor's beliefs.
    let beliefs = view.known_entity_beliefs(actor);
    let target_state = beliefs.iter().find(|(id, _)| *id == target).map(|(_, s)| s);
    let Some(state) = target_state else {
        return Some(PursuitInvalidationReason::NoBelief);
    };

    if !state.alive {
        return Some(PursuitInvalidationReason::TargetDead);
    }

    let Some(believed_place) = state.last_known_place else {
        return Some(PursuitInvalidationReason::PlaceUnknown);
    };

    // Co-located → local combat, not remote pursuit.
    let actor_place = view.effective_place(actor);
    if actor_place == Some(believed_place) {
        return Some(PursuitInvalidationReason::CoLocated);
    }

    // Check if believed place still matches the plan's destination.
    if let Some(planned_dest) = planned_pursuit_destination(plan)
        && believed_place != planned_dest
    {
        return Some(PursuitInvalidationReason::PlaceChanged);
    }

    // Re-derive confidence and check against profile threshold.
    let staleness = current_tick
        .0
        .saturating_sub(state.last_observed_tick().unwrap_or(Tick(0)).0);
    let policy = view.belief_confidence_policy(actor);
    let confidence = belief_confidence(&state.source, staleness, &policy);
    if confidence < profile.min_location_confidence {
        return Some(PursuitInvalidationReason::ConfidenceDecayed);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{is_pursuit_plan_invalid, revalidate_next_step};
    use crate::decision_trace::PursuitInvalidationReason;
    use crate::{
        ExpectedMaterialization, HypotheticalEntityId, MaterializationBindings, PlanTerminalKind,
        PlannedPlan, PlannedStep, PlannerOpKind, PlanningEntityRef,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, BelievedEntityState, BodyCostPerTick, CombatProfile,
        CommodityConsumableProfile, CommodityKind, DemandObservation, DriveThresholds, EntityId,
        EntityKind, GoalKey, GoalKind, HomeostaticNeeds, InTransitOnEdge, LoadUnits,
        MerchandiseProfile, MetabolismProfile, OpportunityAnchor, OpportunityKey, PerceptionSource,
        Permille, PursuitProfile, Quantity, RecipeId, ResourceSource, Tick, TickRange,
        TradeDispositionProfile, UniqueItemKind, VisibilitySpec, WorkstationTag, Wound,
    };
    use worldwake_sim::{
        ActionDef, ActionDefRegistry, ActionDuration, ActionError, ActionHandler, ActionHandlerId,
        ActionHandlerRegistry, ActionPayload, ActionProgress, ActionState, Constraint,
        ControlBeliefView, DeterministicRng, DurationExpr, EntityBeliefView, Interruptibility,
        Precondition, ProfileBeliefView, RuntimeBeliefView, SpatialBeliefView, TargetSpec,
        TemporalBeliefView, TransportActionPayload,
        belief_view::{BeliefStatus, BeliefValue},
    };

    #[derive(Default)]
    struct TestBeliefView {
        alive: BTreeSet<EntityId>,
        kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        adjacent_places: BTreeMap<EntityId, Vec<EntityId>>,
        adjacent_with_ticks: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
        lot_commodities: BTreeMap<EntityId, CommodityKind>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        carry_capacities: BTreeMap<EntityId, LoadUnits>,
        entity_loads: BTreeMap<EntityId, LoadUnits>,
        entity_beliefs: BTreeMap<EntityId, Vec<(EntityId, BelievedEntityState)>>,
        pursuit_profiles: BTreeMap<EntityId, PursuitProfile>,
        believed_target_locations: BTreeMap<(EntityId, EntityId), BeliefValue<Option<EntityId>>>,
    }

    impl ControlBeliefView for TestBeliefView {
        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }

        fn has_control(&self, _entity: EntityId) -> bool {
            true
        }
    }

    impl EntityBeliefView for TestBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.kinds.get(&entity).copied()
        }
        fn is_dead(&self, entity: EntityId) -> bool {
            !self.is_alive(entity)
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
            Some(MetabolismProfile::default())
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
            self.adjacent_with_ticks
                .get(&place)
                .cloned()
                .unwrap_or_default()
        }
    }

    impl TemporalBeliefView for TestBeliefView {
        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }

        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
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

    impl worldwake_sim::SocialBeliefView for TestBeliefView {
        fn belief_confidence_policy(
            &self,
            _agent: EntityId,
        ) -> worldwake_core::BeliefConfidencePolicy {
            worldwake_core::BeliefConfidencePolicy::default()
        }

        fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
            self.entity_beliefs.get(&agent).cloned().unwrap_or_default()
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
            Some(CombatProfile::new(
                pm(1000),
                pm(700),
                pm(620),
                pm(580),
                pm(80),
                pm(25),
                pm(18),
                pm(120),
                pm(35),
                NonZeroU32::new(6).unwrap(),
                NonZeroU32::new(10).unwrap(),
            ))
        }
        fn wounds(&self, _agent: EntityId) -> Vec<Wound> {
            Vec::new()
        }
        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
        fn pursuit_profile(&self, agent: EntityId) -> Option<PursuitProfile> {
            self.pursuit_profiles.get(&agent).cloned()
        }
        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
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
        fn listed_sale_lots_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
        fn seller_for_sale_lot(&self, _lot: EntityId) -> Option<EntityId> {
            None
        }
        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }
        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }
    }

    impl worldwake_sim::InventoryBeliefView for TestBeliefView {
        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
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

        fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.carry_capacities.get(&entity).copied()
        }

        fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.entity_loads.get(&entity).copied()
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }
    }

    impl worldwake_sim::FacilityBeliefView for TestBeliefView {
        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_start(
        _def: &ActionDef,
        _instance: &mut worldwake_sim::ActionInstance,
        _context: &worldwake_sim::ActionExecutionContext<'_>,
        _rng: &mut DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        Ok(None)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_tick(
        _def: &ActionDef,
        _instance: &mut worldwake_sim::ActionInstance,
        _context: &worldwake_sim::ActionExecutionContext<'_>,
        _rng: &mut DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        Ok(ActionProgress::Continue)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_commit(
        _def: &ActionDef,
        _instance: &worldwake_sim::ActionInstance,
        _context: &worldwake_sim::ActionExecutionContext<'_>,
        _event_log: &worldwake_core::EventLog,
        _rng: &mut DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<worldwake_sim::CommitOutcome, ActionError> {
        Ok(worldwake_sim::CommitOutcome::empty())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_abort(
        _def: &ActionDef,
        _instance: &worldwake_sim::ActionInstance,
        _context: &worldwake_sim::ActionExecutionContext<'_>,
        _reason: &worldwake_sim::AbortReason,
        _event_log: &worldwake_core::EventLog,
        _rng: &mut DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn build_registry() -> (ActionDefRegistry, ActionHandlerRegistry) {
        let mut registry = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            noop_start,
            noop_tick,
            noop_commit,
            noop_abort,
        ));
        registry.register(ActionDef {
            id: ActionDefId(0),
            name: "travel".to_string(),
            domain: worldwake_core::ActionDomain::Travel,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::AdjacentPlace],
            preconditions: vec![Precondition::TargetAdjacentToActor(0)],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
        });
        (registry, handlers)
    }

    fn build_payload_registry() -> (ActionDefRegistry, ActionHandlerRegistry) {
        let mut registry = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(ActionHandler::new(
            noop_start,
            noop_tick,
            noop_commit,
            noop_abort,
        ));
        registry.register(ActionDef {
            id: ActionDefId(0),
            name: "harvest:test".to_string(),
            domain: worldwake_core::ActionDomain::Production,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: Vec::new(),
            preconditions: vec![Precondition::ActorAlive],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::Harvest(worldwake_sim::HarvestActionPayload {
                recipe_id: worldwake_core::RecipeId(4),
                required_workstation_tag: worldwake_core::WorkstationTag::OrchardRow,
                output_commodity: CommodityKind::Apple,
                output_quantity: Quantity(1),
                required_tool_kinds: Vec::new(),
            }),
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
        });
        (registry, handlers)
    }

    fn transport_payload_override_is_valid(
        def: &ActionDef,
        actor: EntityId,
        targets: &[EntityId],
        payload: &ActionPayload,
        view: &dyn RuntimeBeliefView,
    ) -> bool {
        if def.name != "pick_up" {
            return false;
        }
        let Some(TransportActionPayload { quantity }) = payload.as_transport() else {
            return false;
        };
        let Some(target) = targets.first().copied() else {
            return false;
        };
        let Some(commodity) = view.item_lot_commodity(target) else {
            return false;
        };
        let Some(carry_capacity) = view.carry_capacity(actor) else {
            return false;
        };
        let Some(load) = view.load_of_entity(actor) else {
            return false;
        };
        let fit =
            carry_capacity.0.saturating_sub(load.0) / worldwake_core::load_per_unit(commodity).0;
        *quantity > Quantity(0)
            && *quantity <= view.commodity_quantity(target, commodity)
            && quantity.0 <= fit
    }

    fn build_transport_registry() -> (ActionDefRegistry, ActionHandlerRegistry) {
        let mut registry = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(
            ActionHandler::new(noop_start, noop_tick, noop_commit, noop_abort)
                .with_payload_override_validator(transport_payload_override_is_valid),
        );
        registry.register(ActionDef {
            id: ActionDefId(0),
            name: "pick_up".to_string(),
            domain: worldwake_core::ActionDomain::Transport,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::EntityAtActorPlace {
                kind: EntityKind::ItemLot,
            }],
            preconditions: vec![Precondition::TargetAtActorPlace(0)],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
        });
        (registry, handlers)
    }

    fn accuse_payload_override_is_valid(
        def: &ActionDef,
        actor: EntityId,
        targets: &[EntityId],
        payload: &ActionPayload,
        view: &dyn RuntimeBeliefView,
    ) -> bool {
        if def.name != "accuse:test" {
            return false;
        }
        let Some(payload) = payload.as_accuse() else {
            return false;
        };
        let Some(accused) = targets.first().copied() else {
            return false;
        };
        actor != accused
            && view.is_alive(accused)
            && payload.violation_id == worldwake_core::ViolationId(7)
    }

    fn build_specific_entity_payload_registry() -> (ActionDefRegistry, ActionHandlerRegistry) {
        let mut registry = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(
            ActionHandler::new(noop_start, noop_tick, noop_commit, noop_abort)
                .with_payload_override_validator(accuse_payload_override_is_valid),
        );
        registry.register(ActionDef {
            id: ActionDefId(0),
            name: "accuse:test".to_string(),
            domain: worldwake_core::ActionDomain::Social,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::SpecificEntity(entity(0))],
            preconditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::Agent,
                },
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
        });
        (registry, handlers)
    }

    fn posting_payload_override_is_valid(
        def: &ActionDef,
        _actor: EntityId,
        targets: &[EntityId],
        payload: &ActionPayload,
        view: &dyn RuntimeBeliefView,
    ) -> bool {
        if def.name != "post_bounty:test" {
            return false;
        }
        let Some(payload) = payload.as_post_bounty() else {
            return false;
        };
        let Some(posting_place) = targets.first().copied() else {
            return false;
        };
        posting_place == payload.posting_place
            && view.entity_kind(posting_place) == Some(EntityKind::Place)
            && view.entity_kind(payload.claim_place) == Some(EntityKind::Place)
    }

    fn build_explicit_payload_registry() -> (ActionDefRegistry, ActionHandlerRegistry) {
        let mut registry = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(
            ActionHandler::new(noop_start, noop_tick, noop_commit, noop_abort)
                .with_affordance_payloads(|_, _, _, _| Vec::new())
                .with_payload_override_validator(posting_payload_override_is_valid),
        );
        registry.register(ActionDef {
            id: ActionDefId(0),
            name: "post_bounty:test".to_string(),
            domain: worldwake_core::ActionDomain::Social,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::ActorPlace],
            preconditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::Place,
                },
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: worldwake_core::Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
        });
        (registry, handlers)
    }

    fn sample_step(def_id: ActionDefId, target: EntityId) -> PlannedStep {
        PlannedStep {
            def_id,
            targets: vec![PlanningEntityRef::Authoritative(target)],
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
        }
    }

    fn target_location_belief(
        place: Option<EntityId>,
        status: BeliefStatus,
    ) -> BeliefValue<Option<EntityId>> {
        BeliefValue {
            value: place,
            confidence: pm(900),
            acquired_tick: Tick(4),
            claimed_event_tick: Some(Tick(4)),
            status,
        }
    }

    #[test]
    fn matching_affordance_binding_revalidates_true() {
        let actor = entity(1);
        let origin = entity(10);
        let destination = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, origin, destination]);
        view.kinds.insert(origin, EntityKind::Place);
        view.kinds.insert(destination, EntityKind::Place);
        view.effective_places.insert(actor, origin);
        view.adjacent_places.insert(origin, vec![destination]);
        view.adjacent_with_ticks
            .insert(origin, vec![(destination, NonZeroU32::new(1).unwrap())]);

        let (registry, handlers) = build_registry();
        assert!(revalidate_next_step(
            &view,
            actor,
            &sample_step(ActionDefId(0), destination),
            &MaterializationBindings::new(),
            &registry,
            &handlers,
        ));
    }

    #[test]
    fn contradicted_exact_target_belief_fails_revalidation() {
        let actor = entity(1);
        let origin = entity(10);
        let destination = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, origin, destination]);
        view.kinds.insert(origin, EntityKind::Place);
        view.kinds.insert(destination, EntityKind::Agent);
        view.effective_places.insert(actor, origin);
        view.believed_target_locations.insert(
            (actor, destination),
            target_location_belief(Some(destination), BeliefStatus::Contradicted),
        );

        let (registry, handlers) = build_specific_entity_payload_registry();
        assert!(!revalidate_next_step(
            &view,
            actor,
            &sample_step(ActionDefId(0), destination),
            &MaterializationBindings::new(),
            &registry,
            &handlers,
        ));
    }

    #[test]
    fn certain_exact_target_belief_preserves_revalidation() {
        let actor = entity(1);
        let origin = entity(10);
        let destination = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, origin, destination]);
        view.kinds.insert(origin, EntityKind::Place);
        view.kinds.insert(destination, EntityKind::Agent);
        view.effective_places.insert(actor, origin);
        view.believed_target_locations.insert(
            (actor, destination),
            target_location_belief(Some(destination), BeliefStatus::Certain),
        );

        let (registry, handlers) = build_specific_entity_payload_registry();
        assert!(revalidate_next_step(
            &view,
            actor,
            &sample_step(ActionDefId(0), destination),
            &MaterializationBindings::new(),
            &registry,
            &handlers,
        ));
    }

    #[test]
    fn different_targets_fail_revalidation() {
        let actor = entity(1);
        let origin = entity(10);
        let available = entity(11);
        let missing = entity(12);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, origin, available, missing]);
        view.kinds.insert(origin, EntityKind::Place);
        view.kinds.insert(available, EntityKind::Place);
        view.kinds.insert(missing, EntityKind::Place);
        view.effective_places.insert(actor, origin);
        view.adjacent_places.insert(origin, vec![available]);
        view.adjacent_with_ticks
            .insert(origin, vec![(available, NonZeroU32::new(1).unwrap())]);

        let (registry, handlers) = build_registry();
        assert!(!revalidate_next_step(
            &view,
            actor,
            &sample_step(ActionDefId(0), missing),
            &MaterializationBindings::new(),
            &registry,
            &handlers,
        ));
    }

    #[test]
    fn hypothetical_targets_fail_revalidation_without_binding() {
        let actor = entity(1);
        let mut view = TestBeliefView::default();
        view.alive.insert(actor);

        let step = PlannedStep {
            def_id: ActionDefId(0),
            targets: vec![PlanningEntityRef::Hypothetical(HypotheticalEntityId(3))],
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
        };

        let (registry, handlers) = build_registry();
        assert!(!revalidate_next_step(
            &view,
            actor,
            &step,
            &MaterializationBindings::new(),
            &registry,
            &handlers,
        ));
    }

    #[test]
    fn hypothetical_targets_revalidate_when_binding_exists() {
        let actor = entity(1);
        let origin = entity(10);
        let destination = entity(11);
        let hypothetical = HypotheticalEntityId(3);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, origin, destination]);
        view.kinds.insert(origin, EntityKind::Place);
        view.kinds.insert(destination, EntityKind::Place);
        view.effective_places.insert(actor, origin);
        view.adjacent_places.insert(origin, vec![destination]);
        view.adjacent_with_ticks
            .insert(origin, vec![(destination, NonZeroU32::new(1).unwrap())]);
        let step = PlannedStep {
            def_id: ActionDefId(0),
            targets: vec![PlanningEntityRef::Hypothetical(hypothetical)],
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: vec![ExpectedMaterialization {
                tag: worldwake_sim::MaterializationTag::SplitOffLot,
                hypothetical_id: hypothetical,
            }],
        };
        let mut bindings = MaterializationBindings::new();
        bindings.bind(hypothetical, destination);

        let (registry, handlers) = build_registry();
        assert!(revalidate_next_step(
            &view, actor, &step, &bindings, &registry, &handlers,
        ));
    }

    #[test]
    fn different_action_def_fails_revalidation() {
        let actor = entity(1);
        let origin = entity(10);
        let destination = entity(11);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, origin, destination]);
        view.kinds.insert(origin, EntityKind::Place);
        view.kinds.insert(destination, EntityKind::Place);
        view.effective_places.insert(actor, origin);
        view.adjacent_places.insert(origin, vec![destination]);
        view.adjacent_with_ticks
            .insert(origin, vec![(destination, NonZeroU32::new(1).unwrap())]);

        let (registry, handlers) = build_registry();
        assert!(!revalidate_next_step(
            &view,
            actor,
            &sample_step(ActionDefId(99), destination),
            &MaterializationBindings::new(),
            &registry,
            &handlers,
        ));
    }

    #[test]
    fn different_effective_payload_fails_revalidation() {
        let actor = entity(1);
        let mut view = TestBeliefView::default();
        view.alive.insert(actor);

        let mut step = sample_step(ActionDefId(0), entity(99));
        step.targets.clear();
        step.payload_override = Some(ActionPayload::Harvest(
            worldwake_sim::HarvestActionPayload {
                recipe_id: worldwake_core::RecipeId(4),
                required_workstation_tag: worldwake_core::WorkstationTag::OrchardRow,
                output_commodity: CommodityKind::Apple,
                output_quantity: Quantity(2),
                required_tool_kinds: Vec::new(),
            },
        ));

        let (registry, handlers) = build_payload_registry();
        assert!(!revalidate_next_step(
            &view,
            actor,
            &step,
            &MaterializationBindings::new(),
            &registry,
            &handlers,
        ));
    }

    #[test]
    fn matching_effective_payload_revalidates_true() {
        let actor = entity(1);
        let mut view = TestBeliefView::default();
        view.alive.insert(actor);

        let mut step = sample_step(ActionDefId(0), entity(99));
        step.targets.clear();
        step.payload_override = Some(ActionPayload::Harvest(
            worldwake_sim::HarvestActionPayload {
                recipe_id: worldwake_core::RecipeId(4),
                required_workstation_tag: worldwake_core::WorkstationTag::OrchardRow,
                output_commodity: CommodityKind::Apple,
                output_quantity: Quantity(1),
                required_tool_kinds: Vec::new(),
            },
        ));

        let (registry, handlers) = build_payload_registry();
        assert!(revalidate_next_step(
            &view,
            actor,
            &step,
            &MaterializationBindings::new(),
            &registry,
            &handlers,
        ));
    }

    #[test]
    fn transport_payload_override_revalidates_against_base_affordance() {
        let actor = entity(1);
        let place = entity(10);
        let lot = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place, lot]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(lot, EntityKind::ItemLot);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(lot, place);
        view.entities_at.insert(place, vec![lot]);
        view.lot_commodities.insert(lot, CommodityKind::Bread);
        view.commodity_quantities
            .insert((lot, CommodityKind::Bread), Quantity(3));
        view.carry_capacities.insert(actor, LoadUnits(4));
        view.entity_loads.insert(actor, LoadUnits(0));

        let mut step = sample_step(ActionDefId(0), lot);
        step.op_kind = PlannerOpKind::MoveCargo;
        step.payload_override = Some(ActionPayload::Transport(TransportActionPayload {
            quantity: Quantity(1),
        }));

        let (registry, handlers) = build_transport_registry();
        assert!(revalidate_next_step(
            &view,
            actor,
            &step,
            &MaterializationBindings::new(),
            &registry,
            &handlers,
        ));
    }

    #[test]
    fn specific_entity_payload_override_revalidates_with_concrete_step_target() {
        let actor = entity(1);
        let accused = entity(2);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, accused]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(accused, EntityKind::Agent);

        let step = PlannedStep {
            def_id: ActionDefId(0),
            targets: vec![PlanningEntityRef::Authoritative(accused)],
            payload_override: Some(ActionPayload::Accuse(worldwake_sim::AccuseActionPayload {
                violation_id: worldwake_core::ViolationId(7),
            })),
            op_kind: PlannerOpKind::Accuse,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
        };

        let (registry, handlers) = build_specific_entity_payload_registry();
        assert!(revalidate_next_step(
            &view,
            actor,
            &step,
            &MaterializationBindings::new(),
            &registry,
            &handlers,
        ));
    }

    #[test]
    fn explicit_payload_variant_steps_revalidate_via_best_effort_fallback() {
        let actor = entity(1);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.effective_places.insert(actor, place);

        let step = PlannedStep {
            def_id: ActionDefId(0),
            targets: vec![PlanningEntityRef::Authoritative(place)],
            payload_override: Some(ActionPayload::PostBounty(
                worldwake_sim::PostBountyActionPayload {
                    posting_place: place,
                    issuing_authority: None,
                    expires_at: None,
                    jurisdiction: Some(place),
                    target: worldwake_core::BountyTarget::EliminateEntity { target: entity(2) },
                    proof_requirement: worldwake_core::ProofRequirement::PhysicalEvidence,
                    reward_commodity: CommodityKind::Coin,
                    reward_quantity: Quantity(3),
                    reward_source: worldwake_core::RewardSource::PersonalFunds { issuer: actor },
                    claim_place: place,
                },
            )),
            op_kind: PlannerOpKind::PostBounty,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
        };

        let (registry, handlers) = build_explicit_payload_registry();
        assert!(revalidate_next_step(
            &view,
            actor,
            &step,
            &MaterializationBindings::new(),
            &registry,
            &handlers,
        ));
    }

    // ── Pursuit plan validity tests ──────────────────────────────────

    fn pursuit_plan(target: EntityId, destination: EntityId) -> PlannedPlan {
        PlannedPlan::new(
            OpportunityKey {
                goal_key: GoalKey::from(GoalKind::RaidTarget { target }),
                anchor: OpportunityAnchor::Entity(target),
            },
            GoalKey::from(GoalKind::RaidTarget { target }),
            vec![
                PlannedStep {
                    def_id: ActionDefId(10),
                    targets: vec![PlanningEntityRef::Authoritative(destination)],
                    payload_override: None,
                    op_kind: PlannerOpKind::Travel,
                    estimated_ticks: 3,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                },
                PlannedStep {
                    def_id: ActionDefId(11),
                    targets: vec![PlanningEntityRef::Authoritative(target)],
                    payload_override: None,
                    op_kind: PlannerOpKind::Attack,
                    estimated_ticks: 0,
                    is_materialization_barrier: false,
                    expected_materializations: Vec::new(),
                },
            ],
            PlanTerminalKind::GoalSatisfied,
        )
    }

    fn alive_belief(place: Option<EntityId>, observed: Tick) -> BelievedEntityState {
        BelievedEntityState {
            believed_kind: None,
            last_known_place: place,
            last_known_inventory: BTreeMap::new(),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: None,
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            ..BelievedEntityState::single_observation_defaults(
                observed,
                PerceptionSource::DirectObservation,
            )
        }
    }

    fn dead_belief(place: Option<EntityId>, observed: Tick) -> BelievedEntityState {
        let mut s = alive_belief(place, observed);
        s.alive = false;
        s
    }

    fn default_pursuit_profile() -> PursuitProfile {
        PursuitProfile {
            min_location_confidence: Permille::new(500).unwrap(),
            max_pursuit_travel_ticks: NonZeroU32::new(10).unwrap(),
        }
    }

    #[test]
    fn pursuit_valid_when_belief_matches_plan() {
        let actor = entity(1);
        let target = entity(2);
        let dest = entity(10);
        let actor_place = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, target]);
        view.effective_places.insert(actor, actor_place);
        view.pursuit_profiles
            .insert(actor, default_pursuit_profile());
        view.entity_beliefs
            .insert(actor, vec![(target, alive_belief(Some(dest), Tick(1)))]);

        let plan = pursuit_plan(target, dest);
        assert!(is_pursuit_plan_invalid(&view, actor, &plan, Tick(2)).is_none());
    }

    #[test]
    fn pursuit_invalidated_on_place_change() {
        let actor = entity(1);
        let target = entity(2);
        let original_dest = entity(10);
        let new_place = entity(11);
        let actor_place = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, target]);
        view.effective_places.insert(actor, actor_place);
        view.pursuit_profiles
            .insert(actor, default_pursuit_profile());
        // Target is now believed at new_place, but plan targets original_dest.
        view.entity_beliefs.insert(
            actor,
            vec![(target, alive_belief(Some(new_place), Tick(1)))],
        );

        let plan = pursuit_plan(target, original_dest);
        assert_eq!(
            is_pursuit_plan_invalid(&view, actor, &plan, Tick(2)),
            Some(PursuitInvalidationReason::PlaceChanged),
        );
    }

    #[test]
    fn pursuit_invalidated_on_confidence_decay() {
        let actor = entity(1);
        let target = entity(2);
        let dest = entity(10);
        let actor_place = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, target]);
        view.effective_places.insert(actor, actor_place);
        // Profile requires 500 permille confidence.
        view.pursuit_profiles
            .insert(actor, default_pursuit_profile());
        // Belief observed a long time ago → staleness will decay confidence
        // below 500 permille with default policy.
        view.entity_beliefs
            .insert(actor, vec![(target, alive_belief(Some(dest), Tick(0)))]);

        // At tick 1000, staleness is 1000 ticks → confidence should be very low.
        let plan = pursuit_plan(target, dest);
        assert_eq!(
            is_pursuit_plan_invalid(&view, actor, &plan, Tick(1000)),
            Some(PursuitInvalidationReason::ConfidenceDecayed),
        );
    }

    #[test]
    fn pursuit_drops_when_target_believed_dead() {
        let actor = entity(1);
        let target = entity(2);
        let dest = entity(10);
        let actor_place = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, target]);
        view.effective_places.insert(actor, actor_place);
        view.pursuit_profiles
            .insert(actor, default_pursuit_profile());
        view.entity_beliefs
            .insert(actor, vec![(target, dead_belief(Some(dest), Tick(1)))]);

        let plan = pursuit_plan(target, dest);
        assert_eq!(
            is_pursuit_plan_invalid(&view, actor, &plan, Tick(2)),
            Some(PursuitInvalidationReason::TargetDead),
        );
    }

    #[test]
    fn pursuit_drops_when_target_place_unknown() {
        let actor = entity(1);
        let target = entity(2);
        let dest = entity(10);
        let actor_place = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, target]);
        view.effective_places.insert(actor, actor_place);
        view.pursuit_profiles
            .insert(actor, default_pursuit_profile());
        // Belief has no known place.
        view.entity_beliefs
            .insert(actor, vec![(target, alive_belief(None, Tick(1)))]);

        let plan = pursuit_plan(target, dest);
        assert_eq!(
            is_pursuit_plan_invalid(&view, actor, &plan, Tick(2)),
            Some(PursuitInvalidationReason::PlaceUnknown),
        );
    }

    #[test]
    fn pursuit_drops_when_target_co_located() {
        let actor = entity(1);
        let target = entity(2);
        let same_place = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, target]);
        view.effective_places.insert(actor, same_place);
        view.pursuit_profiles
            .insert(actor, default_pursuit_profile());
        // Target now believed at actor's place → co-located, not remote.
        view.entity_beliefs.insert(
            actor,
            vec![(target, alive_belief(Some(same_place), Tick(1)))],
        );

        let plan = pursuit_plan(target, same_place);
        assert_eq!(
            is_pursuit_plan_invalid(&view, actor, &plan, Tick(2)),
            Some(PursuitInvalidationReason::CoLocated),
        );
    }

    #[test]
    fn non_pursuit_plan_always_valid() {
        let actor = entity(1);
        let view = TestBeliefView::default();
        // A single-step travel plan (not a pursuit).
        let plan = PlannedPlan::new(
            OpportunityKey {
                goal_key: GoalKey::from(GoalKind::Sleep),
                anchor: OpportunityAnchor::None,
            },
            GoalKey::from(GoalKind::Sleep),
            vec![PlannedStep {
                def_id: ActionDefId(0),
                targets: Vec::new(),
                payload_override: None,
                op_kind: PlannerOpKind::Sleep,
                estimated_ticks: 5,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
            }],
            PlanTerminalKind::GoalSatisfied,
        );
        assert!(is_pursuit_plan_invalid(&view, actor, &plan, Tick(0)).is_none());
    }
}
