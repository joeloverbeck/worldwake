use crate::PlannedStep;
use worldwake_core::EntityId;
use worldwake_sim::{get_affordances, ActionDefRegistry, ActionHandlerRegistry, BeliefView};

#[must_use]
pub fn revalidate_next_step(
    view: &dyn BeliefView,
    actor: EntityId,
    step: &PlannedStep,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> bool {
    let Some(def) = registry.get(step.def_id) else {
        return false;
    };
    get_affordances(view, actor, registry, handlers)
        .into_iter()
        .any(|affordance| {
            affordance.matches_request_identity(
                def,
                actor,
                &step.targets,
                step.payload_override.as_ref(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::revalidate_next_step;
    use crate::{PlannedStep, PlannerOpKind};
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        BodyCostPerTick, CombatProfile, CommodityConsumableProfile, CommodityKind,
        DemandObservation, DriveThresholds, EntityId, EntityKind, HomeostaticNeeds,
        InTransitOnEdge, MerchandiseProfile, MetabolismProfile, Permille, Quantity, RecipeId,
        ResourceSource, TickRange, TradeDispositionProfile, UniqueItemKind, VisibilitySpec,
        WorkstationTag, Wound,
    };
    use worldwake_sim::{
        ActionDef, ActionDefId, ActionDefRegistry, ActionDuration, ActionError, ActionHandler,
        ActionHandlerId, ActionHandlerRegistry, ActionPayload, ActionProgress, ActionState,
        BeliefView, Constraint, DeterministicRng, DurationExpr, Interruptibility, Precondition,
        TargetSpec,
    };

    #[derive(Default)]
    struct TestBeliefView {
        alive: BTreeSet<EntityId>,
        kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        adjacent_places: BTreeMap<EntityId, Vec<EntityId>>,
        adjacent_with_ticks: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
    }

    impl BeliefView for TestBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.kinds.get(&entity).copied()
        }

        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.effective_places.get(&entity).copied()
        }

        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }

        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
            self.adjacent_places
                .get(&place)
                .cloned()
                .unwrap_or_default()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }

        fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
            Quantity(0)
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

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }

        fn has_control(&self, _entity: EntityId) -> bool {
            true
        }

        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }

        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            !self.is_alive(entity)
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }

        fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
            None
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }

        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            Some(MetabolismProfile::default())
        }

        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }

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

        fn agents_selling_at(&self, _place: EntityId, _commodity: CommodityKind) -> Vec<EntityId> {
            Vec::new()
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

        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
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
            self.adjacent_with_ticks
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

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_start(
        _def: &ActionDef,
        _instance: &worldwake_sim::ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        Ok(None)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_tick(
        _def: &ActionDef,
        _instance: &worldwake_sim::ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        Ok(ActionProgress::Continue)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_commit(
        _def: &ActionDef,
        _instance: &worldwake_sim::ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut worldwake_core::WorldTxn<'_>,
    ) -> Result<worldwake_sim::CommitOutcome, ActionError> {
        Ok(worldwake_sim::CommitOutcome::empty())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_abort(
        _def: &ActionDef,
        _instance: &worldwake_sim::ActionInstance,
        _reason: &worldwake_sim::AbortReason,
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
            domain: worldwake_sim::ActionDomain::Travel,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::AdjacentPlace],
            preconditions: vec![Precondition::TargetAdjacentToActor(0)],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
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
            domain: worldwake_sim::ActionDomain::Production,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: Vec::new(),
            preconditions: vec![Precondition::ActorAlive],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
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
        });
        (registry, handlers)
    }

    fn sample_step(def_id: ActionDefId, target: EntityId) -> PlannedStep {
        PlannedStep {
            def_id,
            targets: vec![target],
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 1,
            is_materialization_barrier: false,
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
            &registry,
            &handlers,
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
            &view, actor, &step, &registry, &handlers,
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
            &view, actor, &step, &registry, &handlers,
        ));
    }
}
