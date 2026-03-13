use crate::{
    ActionDef, ActionDefRegistry, ActionHandler, ActionHandlerRegistry, ActionPayload, Affordance,
    BeliefView, Constraint, ConsumableEffect, Precondition, TargetSpec,
};
use worldwake_core::{ActionDefId, EntityId, EntityKind};

#[must_use]
pub fn get_affordances(
    view: &dyn BeliefView,
    actor: EntityId,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> Vec<Affordance> {
    let mut affordances = Vec::new();

    for def in registry.iter() {
        if !def
            .actor_constraints
            .iter()
            .all(|constraint| evaluate_constraint(constraint, actor, view))
        {
            continue;
        }

        let mut def_affordances = Vec::new();
        let mut bound_targets = Vec::new();
        enumerate_bindings(
            &def.targets,
            actor,
            view,
            &mut bound_targets,
            &mut def_affordances,
            def.id,
        );
        def_affordances.retain(|affordance| {
            def.preconditions.iter().all(|precondition| {
                evaluate_precondition(*precondition, actor, &affordance.bound_targets, view)
            })
        });
        let handler = handlers.get(def.handler).unwrap_or_else(|| {
            panic!(
                "action def {} references missing handler {}",
                def.id.0, def.handler.0
            )
        });
        for affordance in &def_affordances {
            affordances.extend(expand_payload_variants(def, handler, affordance, view));
        }
    }

    affordances.sort();
    affordances.dedup();
    affordances
}

#[must_use]
pub fn requested_affordance_matches(
    affordance: &Affordance,
    def: &ActionDef,
    handler: &ActionHandler,
    actor: EntityId,
    targets: &[EntityId],
    payload_override: Option<&ActionPayload>,
    view: &dyn BeliefView,
) -> bool {
    if affordance.matches_request_identity(def, actor, targets, payload_override) {
        return true;
    }

    let Some(requested_payload) = payload_override else {
        return false;
    };
    if affordance.actor != actor
        || affordance.def_id != def.id
        || affordance.bound_targets != targets
    {
        return false;
    }
    if !matches!(affordance.effective_payload(def), ActionPayload::None)
        || !matches!(def.payload, ActionPayload::None)
    {
        return false;
    }

    (handler.payload_override_is_valid)(def, actor, targets, requested_payload, view)
}

fn expand_payload_variants(
    def: &ActionDef,
    handler: &crate::ActionHandler,
    affordance: &Affordance,
    view: &dyn BeliefView,
) -> Vec<Affordance> {
    payload_variants(
        def,
        handler,
        affordance.actor,
        &affordance.bound_targets,
        view,
    )
    .into_iter()
    .map(|payload_override| Affordance {
        payload_override,
        ..affordance.clone()
    })
    .collect()
}

fn payload_variants(
    def: &ActionDef,
    handler: &crate::ActionHandler,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn BeliefView,
) -> Vec<Option<ActionPayload>> {
    if !matches!(def.payload, ActionPayload::None) {
        return vec![Some(def.payload.clone())];
    }
    let mut variants = (handler.affordance_payloads)(def, actor, targets, view)
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();
    if variants.is_empty() {
        variants.push(None);
    } else {
        variants.sort();
        variants.dedup();
    }
    variants
}

#[must_use]
pub fn evaluate_constraint(
    constraint: &Constraint,
    actor: EntityId,
    view: &dyn BeliefView,
) -> bool {
    match constraint {
        Constraint::ActorAlive => view.is_alive(actor),
        Constraint::ActorNotIncapacitated => !view.is_incapacitated(actor),
        Constraint::ActorNotDead => !view.is_dead(actor),
        Constraint::ActorHasControl => view.has_control(actor),
        Constraint::ActorNotInTransit => !view.is_in_transit(actor),
        Constraint::ActorAtPlace(place) => view.effective_place(actor) == Some(*place),
        Constraint::ActorAtPlaceTag(tag) => view
            .effective_place(actor)
            .is_some_and(|place| view.place_has_tag(place, *tag)),
        Constraint::ActorKnowsRecipe(recipe) => view.knows_recipe(actor, *recipe),
        Constraint::ActorHasUniqueItemKind { kind, min_count } => {
            view.unique_item_count(actor, *kind) >= *min_count
        }
        Constraint::ActorHasCommodity { kind, min_qty } => {
            view.commodity_quantity(actor, *kind) >= *min_qty
        }
        Constraint::ActorKind(kind) => view.entity_kind(actor) == Some(*kind),
    }
}

#[must_use]
pub fn evaluate_precondition(
    precondition: Precondition,
    actor: EntityId,
    targets: &[EntityId],
    view: &dyn BeliefView,
) -> bool {
    match precondition {
        Precondition::ActorAlive => view.is_alive(actor),
        Precondition::ActorCanControlTarget(index) => targets
            .get(usize::from(index))
            .is_some_and(|target| view.can_control(actor, *target)),
        Precondition::TargetExists(index) => targets
            .get(usize::from(index))
            .is_some_and(|target| view.is_alive(*target)),
        Precondition::TargetAlive(index) => targets
            .get(usize::from(index))
            .is_some_and(|target| !view.is_dead(*target)),
        Precondition::TargetDead(index) => targets
            .get(usize::from(index))
            .is_some_and(|target| view.is_dead(*target)),
        Precondition::TargetIsAgent(index) => targets
            .get(usize::from(index))
            .is_some_and(|target| view.entity_kind(*target) == Some(EntityKind::Agent)),
        Precondition::TargetAtActorPlace(index) => {
            let Some(target) = targets.get(usize::from(index)).copied() else {
                return false;
            };
            let Some(actor_place) = view.effective_place(actor) else {
                return false;
            };
            view.effective_place(target) == Some(actor_place)
        }
        Precondition::TargetAdjacentToActor(index) => {
            let Some(target) = targets.get(usize::from(index)).copied() else {
                return false;
            };
            let Some(actor_place) = view.effective_place(actor) else {
                return false;
            };
            view.adjacent_places(actor_place).contains(&target)
        }
        Precondition::TargetKind { target_index, kind } => targets
            .get(usize::from(target_index))
            .is_some_and(|target| view.entity_kind(*target) == Some(kind)),
        Precondition::TargetCommodity { target_index, kind } => targets
            .get(usize::from(target_index))
            .is_some_and(|target| view.item_lot_commodity(*target) == Some(kind)),
        Precondition::TargetHasWorkstationTag { target_index, tag } => targets
            .get(usize::from(target_index))
            .is_some_and(|target| view.workstation_tag(*target) == Some(tag)),
        Precondition::TargetHasResourceSource {
            target_index,
            commodity,
            min_available,
        } => targets
            .get(usize::from(target_index))
            .and_then(|target| view.resource_source(*target))
            .is_some_and(|source| {
                source.commodity == commodity && source.available_quantity >= min_available
            }),
        Precondition::TargetNotInContainer(target_index) => targets
            .get(usize::from(target_index))
            .is_some_and(|target| view.direct_container(*target).is_none()),
        Precondition::TargetUnpossessed(target_index) => targets
            .get(usize::from(target_index))
            .is_some_and(|target| view.direct_possessor(*target).is_none()),
        Precondition::TargetDirectlyPossessedByActor(target_index) => targets
            .get(usize::from(target_index))
            .is_some_and(|target| view.direct_possessor(*target) == Some(actor)),
        Precondition::TargetLacksProductionJob(target_index) => targets
            .get(usize::from(target_index))
            .is_some_and(|target| !view.has_production_job(*target)),
        Precondition::TargetHasConsumableEffect {
            target_index,
            effect,
        } => targets
            .get(usize::from(target_index))
            .and_then(|target| view.item_lot_consumable_profile(*target))
            .is_some_and(|profile| match effect {
                ConsumableEffect::Hunger => profile.hunger_relief_per_unit.value() > 0,
                ConsumableEffect::Thirst => profile.thirst_relief_per_unit.value() > 0,
            }),
        Precondition::TargetHasWounds(target_index) => targets
            .get(usize::from(target_index))
            .is_some_and(|target| view.has_wounds(*target)),
    }
}

#[must_use]
fn enumerate_targets(spec: &TargetSpec, actor: EntityId, view: &dyn BeliefView) -> Vec<EntityId> {
    let mut targets = match spec {
        TargetSpec::SpecificEntity(entity) => view
            .is_alive(*entity)
            .then_some(*entity)
            .into_iter()
            .collect::<Vec<_>>(),
        TargetSpec::EntityAtActorPlace { kind } => {
            let Some(place) = view.effective_place(actor) else {
                return Vec::new();
            };
            view.entities_at(place)
                .into_iter()
                .filter(|entity| view.entity_kind(*entity) == Some(*kind))
                .collect::<Vec<_>>()
        }
        TargetSpec::EntityDirectlyPossessedByActor { kind } => view
            .direct_possessions(actor)
            .into_iter()
            .filter(|entity| view.entity_kind(*entity) == Some(*kind))
            .collect::<Vec<_>>(),
        TargetSpec::AdjacentPlace => {
            let Some(place) = view.effective_place(actor) else {
                return Vec::new();
            };
            view.adjacent_places(place)
                .into_iter()
                .filter(|entity| {
                    view.entity_kind(*entity) == Some(worldwake_core::EntityKind::Place)
                })
                .collect::<Vec<_>>()
        }
    };

    targets.sort();
    targets.dedup();
    targets
}

fn enumerate_bindings(
    specs: &[TargetSpec],
    actor: EntityId,
    view: &dyn BeliefView,
    current: &mut Vec<EntityId>,
    affordances: &mut Vec<Affordance>,
    def_id: ActionDefId,
) {
    if let Some((spec, remaining)) = specs.split_first() {
        for target in enumerate_targets(spec, actor, view) {
            current.push(target);
            enumerate_bindings(remaining, actor, view, current, affordances, def_id);
            current.pop();
        }
        return;
    }

    affordances.push(Affordance {
        def_id,
        actor,
        bound_targets: current.clone(),
        payload_override: None,
        explanation: None,
    });
}

#[cfg(test)]
mod tests {
    use super::{enumerate_targets, evaluate_constraint, evaluate_precondition, get_affordances};
    use crate::{
        ActionDef, ActionDefRegistry, ActionDomain, ActionError, ActionHandler, ActionHandlerId,
        ActionHandlerRegistry, ActionPayload, ActionProgress, ActionState, CombatActionPayload,
        Constraint, ConsumableEffect, DeterministicRng, DurationExpr, Interruptibility,
        OmniscientBeliefView, Precondition, ReservationReq, TargetSpec, TradeActionPayload,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, ActionDefId, BodyCostPerTick, CauseRef, CombatProfile,
        CombatWeaponRef, CommodityConsumableProfile, CommodityKind, ControlSource,
        DemandObservation, DriveThresholds, EntityId, EntityKind, EventLog, HomeostaticNeeds,
        InTransitOnEdge, LoadUnits, MerchandiseProfile, MetabolismProfile, Quantity, RecipeId,
        ResourceSource, Tick, TradeDispositionProfile, UniqueItemKind, VisibilitySpec, WitnessData,
        WorkstationTag, World, WorldTxn, Wound,
    };

    #[derive(Default)]
    struct StubBeliefView {
        alive: BTreeMap<EntityId, bool>,
        kinds: BTreeMap<EntityId, EntityKind>,
        places: BTreeMap<EntityId, EntityId>,
        in_transit: BTreeMap<EntityId, bool>,
        colocated: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
        adjacent_places: BTreeMap<EntityId, Vec<EntityId>>,
        known_recipes: BTreeMap<EntityId, Vec<RecipeId>>,
        unique_items: BTreeMap<(EntityId, UniqueItemKind), u32>,
        commodities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        item_lot_commodities: BTreeMap<EntityId, CommodityKind>,
        consumable_profiles: BTreeMap<EntityId, CommodityConsumableProfile>,
        direct_containers: BTreeMap<EntityId, EntityId>,
        direct_possessors: BTreeMap<EntityId, EntityId>,
        workstation_tags: BTreeMap<EntityId, WorkstationTag>,
        resource_sources: BTreeMap<EntityId, ResourceSource>,
        production_jobs: BTreeMap<EntityId, bool>,
        wounds: BTreeMap<EntityId, bool>,
        controllable: BTreeMap<(EntityId, EntityId), bool>,
        control: BTreeMap<EntityId, bool>,
        needs: BTreeMap<EntityId, HomeostaticNeeds>,
        demand_memories: BTreeMap<EntityId, Vec<DemandObservation>>,
        merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
        wound_lists: BTreeMap<EntityId, Vec<Wound>>,
    }

    impl crate::BeliefView for StubBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.get(&entity).copied().unwrap_or(false)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.is_alive(entity)
                .then(|| self.kinds.get(&entity).copied())
                .flatten()
        }

        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.places.get(&entity).copied()
        }

        fn is_in_transit(&self, entity: EntityId) -> bool {
            self.in_transit.get(&entity).copied().unwrap_or(false)
        }

        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.colocated.get(&place).cloned().unwrap_or_default()
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
        }

        fn knows_recipe(&self, actor: EntityId, recipe: RecipeId) -> bool {
            self.known_recipes
                .get(&actor)
                .is_some_and(|recipes| recipes.contains(&recipe))
        }

        fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32 {
            self.unique_items.get(&(holder, kind)).copied().unwrap_or(0)
        }

        fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
            self.commodities
                .get(&(holder, kind))
                .copied()
                .unwrap_or(Quantity(0))
        }

        fn controlled_commodity_quantity_at_place(
            &self,
            actor: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Quantity {
            self.local_controlled_lots_for(actor, place, commodity)
                .into_iter()
                .fold(Quantity(0), |total, entity| {
                    let quantity = self
                        .commodities
                        .get(&(entity, commodity))
                        .copied()
                        .unwrap_or(Quantity(0));
                    Quantity(total.0 + quantity.0)
                })
        }

        fn local_controlled_lots_for(
            &self,
            actor: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Vec<EntityId> {
            let mut entities = self
                .entities_at(place)
                .into_iter()
                .filter(|entity| self.item_lot_commodity(*entity) == Some(commodity))
                .filter(|entity| self.can_control(actor, *entity))
                .collect::<Vec<_>>();
            entities.sort();
            entities.dedup();
            entities
        }

        fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
            self.item_lot_commodities.get(&entity).copied()
        }

        fn item_lot_consumable_profile(
            &self,
            entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            self.consumable_profiles.get(&entity).copied()
        }

        fn direct_container(&self, entity: EntityId) -> Option<EntityId> {
            self.direct_containers.get(&entity).copied()
        }

        fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
            self.direct_possessors.get(&entity).copied()
        }

        fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag> {
            self.workstation_tags.get(&entity).copied()
        }

        fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
            self.resource_sources.get(&entity).cloned()
        }

        fn has_production_job(&self, entity: EntityId) -> bool {
            self.production_jobs.get(&entity).copied().unwrap_or(false)
        }

        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
            self.controllable
                .get(&(actor, entity))
                .copied()
                .unwrap_or(false)
        }

        fn has_control(&self, entity: EntityId) -> bool {
            self.control.get(&entity).copied().unwrap_or(false)
        }

        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn reservation_conflicts(
            &self,
            _entity: EntityId,
            _range: worldwake_core::TickRange,
        ) -> bool {
            false
        }

        fn reservation_ranges(&self, _entity: EntityId) -> Vec<worldwake_core::TickRange> {
            Vec::new()
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            !self.is_alive(entity)
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds.get(&entity).copied().unwrap_or(false)
        }

        fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
            self.needs.get(&agent).copied()
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
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
            self.wound_lists.get(&agent).cloned().unwrap_or_default()
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

        fn known_recipes(&self, actor: EntityId) -> Vec<RecipeId> {
            self.known_recipes.get(&actor).cloned().unwrap_or_default()
        }

        fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId> {
            self.colocated
                .get(&place)
                .into_iter()
                .flatten()
                .copied()
                .filter(|entity| self.workstation_tags.get(entity).copied() == Some(tag))
                .collect()
        }

        fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.colocated
                .get(&place)
                .into_iter()
                .flatten()
                .copied()
                .filter(|entity| {
                    self.resource_sources
                        .get(entity)
                        .is_some_and(|source| source.commodity == commodity)
                })
                .collect()
        }

        fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
            self.demand_memories
                .get(&agent)
                .cloned()
                .unwrap_or_default()
        }

        fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile> {
            self.merchandise_profiles.get(&agent).cloned()
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
            self.adjacent_places(place)
                .into_iter()
                .map(|adjacent| (adjacent, NonZeroU32::MIN))
                .collect()
        }

        fn estimate_duration(
            &self,
            _actor: EntityId,
            duration: &crate::DurationExpr,
            _targets: &[EntityId],
            _payload: &crate::ActionPayload,
        ) -> Option<crate::ActionDuration> {
            duration
                .fixed_ticks()
                .map(crate::ActionDuration::Finite)
                .or(Some(crate::ActionDuration::Indefinite))
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_start(
        _def: &ActionDef,
        _instance: &crate::ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<Option<ActionState>, ActionError> {
        Ok(None)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_tick(
        _def: &ActionDef,
        _instance: &crate::ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<ActionProgress, ActionError> {
        Ok(ActionProgress::Continue)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_commit(
        _def: &ActionDef,
        _instance: &crate::ActionInstance,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<crate::CommitOutcome, ActionError> {
        Ok(crate::CommitOutcome::empty())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn noop_abort(
        _def: &ActionDef,
        _instance: &crate::ActionInstance,
        _reason: &crate::AbortReason,
        _rng: &mut DeterministicRng,
        _txn: &mut WorldTxn<'_>,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn handler_registry(count: usize) -> ActionHandlerRegistry {
        let mut handlers = ActionHandlerRegistry::new();
        for _ in 0..count {
            handlers.register(ActionHandler::new(
                noop_start,
                noop_tick,
                noop_commit,
                noop_abort,
            ));
        }
        handlers
    }

    fn sample_action_def(
        id: ActionDefId,
        actor_constraints: Vec<Constraint>,
        targets: Vec<TargetSpec>,
        preconditions: Vec<Precondition>,
    ) -> ActionDef {
        ActionDef {
            id,
            name: format!("action-{}", id.0),
            domain: ActionDomain::Generic,
            actor_constraints,
            targets,
            preconditions,
            reservation_requirements: vec![ReservationReq { target_index: 0 }],
            duration: DurationExpr::Fixed(NonZeroU32::new(3).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::ActorAlive],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(id.0),
        }
    }

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        )
    }

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    #[test]
    fn enumerate_targets_filters_and_sorts_entities_for_actor_place() {
        let actor = entity(1);
        let place = entity(10);
        let matching_a = entity(30);
        let matching_b = entity(20);
        let other_kind = entity(40);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(matching_a, true);
        view.alive.insert(matching_b, true);
        view.alive.insert(other_kind, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(matching_a, EntityKind::Facility);
        view.kinds.insert(matching_b, EntityKind::Facility);
        view.kinds.insert(other_kind, EntityKind::ItemLot);
        view.places.insert(actor, place);
        view.colocated
            .insert(place, vec![matching_a, other_kind, matching_b, matching_a]);

        let targets = enumerate_targets(
            &TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Facility,
            },
            actor,
            &view,
        );

        assert_eq!(targets, vec![matching_b, matching_a]);
    }

    #[test]
    fn enumerate_targets_returns_adjacent_places_for_travel_specs() {
        let actor = entity(1);
        let place = entity(10);
        let dest_a = entity(20);
        let dest_b = entity(30);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(dest_a, true);
        view.alive.insert(dest_b, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(dest_a, EntityKind::Place);
        view.kinds.insert(dest_b, EntityKind::Place);
        view.places.insert(actor, place);
        view.adjacent_places
            .insert(place, vec![dest_b, dest_a, dest_a]);

        let targets = enumerate_targets(&TargetSpec::AdjacentPlace, actor, &view);

        assert_eq!(targets, vec![dest_a, dest_b]);
    }

    #[test]
    fn evaluate_constraint_checks_control_and_commodity_requirements() {
        let actor = entity(1);
        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.control.insert(actor, true);
        view.commodities
            .insert((actor, CommodityKind::Bread), Quantity(3));

        assert!(evaluate_constraint(&Constraint::ActorAlive, actor, &view));
        assert!(evaluate_constraint(
            &Constraint::ActorHasControl,
            actor,
            &view
        ));
        view.unique_items
            .insert((actor, UniqueItemKind::SimpleTool), 1);
        assert!(evaluate_constraint(
            &Constraint::ActorHasUniqueItemKind {
                kind: UniqueItemKind::SimpleTool,
                min_count: 1,
            },
            actor,
            &view,
        ));
        assert!(evaluate_constraint(
            &Constraint::ActorHasCommodity {
                kind: CommodityKind::Bread,
                min_qty: Quantity(2),
            },
            actor,
            &view,
        ));
        assert!(!evaluate_constraint(
            &Constraint::ActorHasCommodity {
                kind: CommodityKind::Water,
                min_qty: Quantity(1),
            },
            actor,
            &view,
        ));
        assert!(evaluate_constraint(
            &Constraint::ActorNotInTransit,
            actor,
            &view,
        ));
        view.in_transit.insert(actor, true);
        assert!(!evaluate_constraint(
            &Constraint::ActorNotInTransit,
            actor,
            &view,
        ));
    }

    #[test]
    fn evaluate_precondition_returns_false_for_out_of_bounds_target_index() {
        let actor = entity(1);
        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);

        assert!(!evaluate_precondition(
            Precondition::TargetExists(2),
            actor,
            &[entity(4)],
            &view,
        ));
        assert!(!evaluate_precondition(
            Precondition::TargetKind {
                target_index: 1,
                kind: EntityKind::Facility,
            },
            actor,
            &[entity(4)],
            &view,
        ));
        assert!(!evaluate_precondition(
            Precondition::ActorCanControlTarget(3),
            actor,
            &[entity(4)],
            &view,
        ));
    }

    #[test]
    fn get_affordances_filters_by_control_and_consumable_effect() {
        let actor = entity(1);
        let place = entity(10);
        let bread = entity(20);
        let medicine = entity(30);

        let mut view = StubBeliefView::default();
        for entity in [actor, bread, medicine] {
            view.alive.insert(entity, true);
        }
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.kinds.insert(medicine, EntityKind::ItemLot);
        view.places.insert(actor, place);
        view.places.insert(bread, place);
        view.places.insert(medicine, place);
        view.colocated.insert(place, vec![medicine, bread]);
        view.item_lot_commodities
            .insert(bread, CommodityKind::Bread);
        view.item_lot_commodities
            .insert(medicine, CommodityKind::Medicine);
        view.consumable_profiles.insert(
            bread,
            CommodityKind::Bread.spec().consumable_profile.unwrap(),
        );
        view.controllable.insert((actor, bread), true);

        let mut registry = ActionDefRegistry::new();
        registry.register(sample_action_def(
            ActionDefId(0),
            vec![Constraint::ActorAlive],
            vec![TargetSpec::EntityAtActorPlace {
                kind: EntityKind::ItemLot,
            }],
            vec![
                Precondition::ActorCanControlTarget(0),
                Precondition::TargetHasConsumableEffect {
                    target_index: 0,
                    effect: ConsumableEffect::Hunger,
                },
            ],
        ));
        let handlers = handler_registry(registry.len());

        let affordances = get_affordances(&view, actor, &registry, &handlers);

        assert_eq!(affordances.len(), 1);
        assert_eq!(affordances[0].bound_targets, vec![bread]);
    }

    #[test]
    fn get_affordances_filters_targets_without_wounds() {
        let actor = entity(1);
        let place = entity(10);
        let wounded = entity(20);
        let healthy = entity(30);

        let mut view = StubBeliefView::default();
        for entity in [actor, wounded, healthy] {
            view.alive.insert(entity, true);
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
        }
        view.control.insert(actor, true);
        view.colocated.insert(place, vec![healthy, wounded]);
        view.wounds.insert(wounded, true);

        let mut registry = ActionDefRegistry::new();
        registry.register(sample_action_def(
            ActionDefId(0),
            vec![Constraint::ActorAlive],
            vec![TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            vec![Precondition::TargetHasWounds(0)],
        ));
        let handlers = handler_registry(registry.len());

        let affordances = get_affordances(&view, actor, &registry, &handlers);

        assert_eq!(affordances.len(), 1);
        assert_eq!(affordances[0].bound_targets, vec![wounded]);
    }

    #[test]
    fn get_affordances_sorts_and_deduplicates_equivalent_results() {
        let actor = entity(1);
        let place = entity(10);
        let target_a = entity(20);
        let target_b = entity(30);

        let mut view = StubBeliefView::default();
        for entity in [actor, target_a, target_b] {
            view.alive.insert(entity, true);
        }
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(target_a, EntityKind::Facility);
        view.kinds.insert(target_b, EntityKind::Facility);
        view.places.insert(actor, place);
        view.places.insert(target_a, place);
        view.places.insert(target_b, place);
        view.colocated
            .insert(place, vec![target_b, target_a, target_b]);

        let mut registry = ActionDefRegistry::new();
        registry.register(sample_action_def(
            ActionDefId(0),
            vec![Constraint::ActorAlive],
            vec![TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Facility,
            }],
            vec![Precondition::TargetAtActorPlace(0)],
        ));
        registry.register(sample_action_def(
            ActionDefId(1),
            vec![Constraint::ActorAlive],
            vec![TargetSpec::SpecificEntity(target_b)],
            vec![Precondition::TargetExists(0)],
        ));
        let handlers = handler_registry(registry.len());

        let affordances = get_affordances(&view, actor, &registry, &handlers);

        assert_eq!(affordances.len(), 3);
        assert_eq!(affordances[0].def_id, ActionDefId(0));
        assert_eq!(affordances[0].bound_targets, vec![target_a]);
        assert_eq!(affordances[1].def_id, ActionDefId(0));
        assert_eq!(affordances[1].bound_targets, vec![target_b]);
        assert_eq!(affordances[2].def_id, ActionDefId(1));
        assert_eq!(affordances[2].bound_targets, vec![target_b]);
    }

    #[test]
    fn get_affordances_materializes_definition_payload_identity() {
        let actor = entity(1);
        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.kinds.insert(actor, EntityKind::Agent);

        let mut registry = ActionDefRegistry::new();
        registry.register(ActionDef {
            id: ActionDefId(0),
            name: "harvest:test".to_string(),
            domain: ActionDomain::Production,
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
            payload: ActionPayload::Harvest(crate::HarvestActionPayload {
                recipe_id: RecipeId(3),
                required_workstation_tag: WorkstationTag::OrchardRow,
                output_commodity: CommodityKind::Apple,
                output_quantity: Quantity(1),
                required_tool_kinds: Vec::new(),
            }),
            handler: ActionHandlerId(0),
        });
        let handlers = handler_registry(registry.len());

        let affordances = get_affordances(&view, actor, &registry, &handlers);
        assert_eq!(affordances.len(), 1);
        assert!(matches!(
            affordances[0].payload_override,
            Some(ActionPayload::Harvest(_))
        ));
    }

    #[test]
    fn get_affordances_expands_attack_weapon_payload_variants() {
        let actor = entity(1);
        let place = entity(10);
        let target = entity(20);

        let mut view = StubBeliefView::default();
        for entity in [actor, target] {
            view.alive.insert(entity, true);
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
        }
        view.colocated.insert(place, vec![target]);
        view.commodities
            .insert((actor, CommodityKind::Sword), Quantity(1));

        let mut registry = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(
            ActionHandler::new(noop_start, noop_tick, noop_commit, noop_abort)
                .with_affordance_payloads(|_def, actor, targets, view| {
                    let Some(target) = targets.first().copied() else {
                        return Vec::new();
                    };
                    let mut payloads = vec![ActionPayload::Combat(CombatActionPayload {
                        target,
                        weapon: CombatWeaponRef::Unarmed,
                    })];
                    if view.commodity_quantity(actor, CommodityKind::Sword) > Quantity(0) {
                        payloads.push(ActionPayload::Combat(CombatActionPayload {
                            target,
                            weapon: CombatWeaponRef::Commodity(CommodityKind::Sword),
                        }));
                    }
                    payloads
                }),
        );
        registry.register(ActionDef {
            id: ActionDefId(0),
            name: "attack".to_string(),
            domain: ActionDomain::Combat,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: vec![
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
                Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::Agent,
                },
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Indefinite,
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
        });

        let affordances = get_affordances(&view, actor, &registry, &handlers);
        let payloads = affordances
            .into_iter()
            .map(|affordance| affordance.payload_override.unwrap())
            .collect::<Vec<_>>();
        assert_eq!(payloads.len(), 2);
        assert!(
            payloads.contains(&ActionPayload::Combat(CombatActionPayload {
                target,
                weapon: CombatWeaponRef::Unarmed,
            }))
        );
        assert!(
            payloads.contains(&ActionPayload::Combat(CombatActionPayload {
                target,
                weapon: CombatWeaponRef::Commodity(CommodityKind::Sword),
            }))
        );
    }

    #[test]
    fn get_affordances_expands_trade_into_concrete_payloads() {
        let actor = entity(1);
        let seller = entity(2);
        let place = entity(10);

        let mut view = StubBeliefView::default();
        for entity in [actor, seller] {
            view.alive.insert(entity, true);
            view.kinds.insert(entity, EntityKind::Agent);
            view.places.insert(entity, place);
        }
        view.colocated.insert(place, vec![seller]);
        view.commodities
            .insert((actor, CommodityKind::Coin), Quantity(1));
        view.commodities
            .insert((seller, CommodityKind::Bread), Quantity(1));
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(
                worldwake_core::Permille::new(900).unwrap(),
                worldwake_core::Permille::new(0).unwrap(),
                worldwake_core::Permille::new(0).unwrap(),
                worldwake_core::Permille::new(0).unwrap(),
                worldwake_core::Permille::new(0).unwrap(),
            ),
        );
        view.needs.insert(seller, HomeostaticNeeds::new_sated());
        view.merchandise_profiles.insert(
            seller,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(place),
            },
        );

        let mut registry = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        handlers.register(
            ActionHandler::new(noop_start, noop_tick, noop_commit, noop_abort)
                .with_affordance_payloads(|_def, actor, targets, view| {
                    let Some(counterparty) = targets.first().copied() else {
                        return Vec::new();
                    };
                    if view.commodity_quantity(actor, CommodityKind::Coin) == Quantity(0)
                        || view.commodity_quantity(counterparty, CommodityKind::Bread)
                            == Quantity(0)
                    {
                        return Vec::new();
                    }
                    vec![ActionPayload::Trade(TradeActionPayload {
                        counterparty,
                        offered_commodity: CommodityKind::Coin,
                        offered_quantity: Quantity(1),
                        requested_commodity: CommodityKind::Bread,
                        requested_quantity: Quantity(1),
                    })]
                }),
        );
        registry.register(ActionDef {
            id: ActionDefId(0),
            name: "trade".to_string(),
            domain: ActionDomain::Trade,
            actor_constraints: Vec::new(),
            targets: vec![TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Agent,
            }],
            preconditions: vec![
                Precondition::ActorAlive,
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(0),
                Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::Agent,
                },
            ],
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Indefinite,
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
        });

        let affordances = get_affordances(&view, actor, &registry, &handlers);
        assert_eq!(affordances.len(), 1);
        assert_eq!(
            affordances[0].payload_override,
            Some(ActionPayload::Trade(TradeActionPayload {
                counterparty: seller,
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(1),
                requested_commodity: CommodityKind::Bread,
                requested_quantity: Quantity(1),
            }))
        );
    }

    #[test]
    fn get_affordances_filters_false_constraints_preconditions_and_missing_targets() {
        let actor = entity(1);
        let place = entity(10);
        let target = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(target, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(target, EntityKind::Facility);
        view.places.insert(actor, place);
        view.places.insert(target, place);
        view.colocated.insert(place, vec![target]);

        let mut registry = ActionDefRegistry::new();
        registry.register(sample_action_def(
            ActionDefId(0),
            vec![Constraint::ActorHasControl],
            vec![TargetSpec::SpecificEntity(target)],
            vec![Precondition::TargetExists(0)],
        ));
        registry.register(sample_action_def(
            ActionDefId(1),
            vec![Constraint::ActorAlive],
            vec![TargetSpec::SpecificEntity(target)],
            vec![Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Container,
            }],
        ));
        registry.register(sample_action_def(
            ActionDefId(2),
            vec![Constraint::ActorAlive],
            vec![TargetSpec::SpecificEntity(entity(99))],
            vec![Precondition::TargetExists(0)],
        ));
        registry.register(sample_action_def(
            ActionDefId(3),
            vec![Constraint::ActorAlive],
            vec![TargetSpec::SpecificEntity(target)],
            vec![Precondition::TargetExists(4)],
        ));
        let handlers = handler_registry(registry.len());

        let affordances = get_affordances(&view, actor, &registry, &handlers);

        assert!(affordances.is_empty());
    }

    #[test]
    fn get_affordances_filters_out_travel_for_actors_already_in_transit() {
        let actor = entity(1);
        let place = entity(10);
        let destination = entity(20);

        let mut view = StubBeliefView::default();
        for entity in [actor, destination] {
            view.alive.insert(entity, true);
        }
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(destination, EntityKind::Place);
        view.control.insert(actor, true);
        view.places.insert(actor, place);
        view.in_transit.insert(actor, true);
        view.adjacent_places.insert(place, vec![destination]);

        let mut registry = ActionDefRegistry::new();
        registry.register(sample_action_def(
            ActionDefId(0),
            vec![
                Constraint::ActorAlive,
                Constraint::ActorHasControl,
                Constraint::ActorNotInTransit,
            ],
            vec![TargetSpec::AdjacentPlace],
            vec![
                Precondition::TargetExists(0),
                Precondition::TargetAdjacentToActor(0),
                Precondition::TargetKind {
                    target_index: 0,
                    kind: EntityKind::Place,
                },
            ],
        ));
        let handlers = handler_registry(registry.len());

        assert!(get_affordances(&view, actor, &registry, &handlers).is_empty());
    }

    #[test]
    fn omniscient_belief_view_affordances_match_for_human_and_ai_control() {
        let mut human_world = World::new(build_prototype_world()).unwrap();
        let human = {
            let mut txn = new_txn(&mut human_world, 1);
            let human = txn.create_agent("Aster", ControlSource::Human).unwrap();
            commit_txn(txn);
            human
        };

        let mut ai_world = World::new(build_prototype_world()).unwrap();
        let ai = {
            let mut txn = new_txn(&mut ai_world, 1);
            let ai = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            commit_txn(txn);
            ai
        };

        let mut registry = ActionDefRegistry::new();
        registry.register(sample_action_def(
            ActionDefId(0),
            vec![Constraint::ActorAlive],
            Vec::new(),
            vec![Precondition::ActorAlive],
        ));
        registry.register(sample_action_def(
            ActionDefId(1),
            vec![Constraint::ActorHasControl],
            Vec::new(),
            vec![Precondition::ActorAlive],
        ));
        let handlers = handler_registry(registry.len());

        let human_affordances = get_affordances(
            &OmniscientBeliefView::new(&human_world),
            human,
            &registry,
            &handlers,
        );
        let ai_affordances = get_affordances(
            &OmniscientBeliefView::new(&ai_world),
            ai,
            &registry,
            &handlers,
        );

        assert_eq!(human_affordances.len(), 2);
        assert_eq!(ai_affordances.len(), 2);
        assert_eq!(
            human_affordances
                .iter()
                .map(|affordance| (affordance.def_id, affordance.bound_targets.clone()))
                .collect::<Vec<_>>(),
            ai_affordances
                .iter()
                .map(|affordance| (affordance.def_id, affordance.bound_targets.clone()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn omniscient_belief_view_none_control_only_changes_actor_has_control_actions() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::None).unwrap();
            commit_txn(txn);
            actor
        };

        let mut registry = ActionDefRegistry::new();
        registry.register(sample_action_def(
            ActionDefId(0),
            vec![Constraint::ActorAlive],
            Vec::new(),
            vec![Precondition::ActorAlive],
        ));
        registry.register(sample_action_def(
            ActionDefId(1),
            vec![Constraint::ActorHasControl],
            Vec::new(),
            vec![Precondition::ActorAlive],
        ));
        let handlers = handler_registry(registry.len());

        let affordances = get_affordances(
            &OmniscientBeliefView::new(&world),
            actor,
            &registry,
            &handlers,
        );

        assert_eq!(affordances.len(), 1);
        assert_eq!(affordances[0].def_id, ActionDefId(0));
    }

    #[test]
    fn divergent_belief_views_produce_different_affordances() {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let target_a = entity(20);
        let target_b = entity(21);

        let mut view_a = StubBeliefView::default();
        let mut view_b = StubBeliefView::default();

        for view in [&mut view_a, &mut view_b] {
            view.alive.insert(actor, true);
            view.alive.insert(target_a, true);
            view.alive.insert(target_b, true);
            view.kinds.insert(actor, EntityKind::Agent);
            view.kinds.insert(target_a, EntityKind::Facility);
            view.kinds.insert(target_b, EntityKind::Facility);
        }

        view_a.places.insert(actor, place_a);
        view_a.places.insert(target_a, place_a);
        view_a.colocated.insert(place_a, vec![target_a]);

        view_b.places.insert(actor, place_b);
        view_b.places.insert(target_b, place_b);
        view_b.colocated.insert(place_b, vec![target_b]);

        let mut registry = ActionDefRegistry::new();
        registry.register(sample_action_def(
            ActionDefId(0),
            vec![Constraint::ActorAlive],
            vec![TargetSpec::EntityAtActorPlace {
                kind: EntityKind::Facility,
            }],
            vec![Precondition::TargetAtActorPlace(0)],
        ));
        let handlers = handler_registry(registry.len());

        let affordances_a = get_affordances(&view_a, actor, &registry, &handlers);
        let affordances_b = get_affordances(&view_b, actor, &registry, &handlers);

        assert_eq!(affordances_a.len(), 1);
        assert_eq!(affordances_b.len(), 1);
        assert_eq!(affordances_a[0].bound_targets, vec![target_a]);
        assert_eq!(affordances_b[0].bound_targets, vec![target_b]);
        assert_ne!(affordances_a, affordances_b);
    }
}
