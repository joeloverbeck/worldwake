use crate::{
    ActionDefRegistry, Affordance, BeliefView, Constraint, ConsumableEffect, Precondition,
    TargetSpec,
};
use worldwake_core::EntityId;

#[must_use]
pub fn get_affordances(
    view: &dyn BeliefView,
    actor: EntityId,
    registry: &ActionDefRegistry,
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

        let mut bound_targets = Vec::new();
        enumerate_bindings(
            &def.targets,
            actor,
            view,
            &mut bound_targets,
            &mut affordances,
            def.id,
        );
        affordances.retain(|affordance| {
            affordance.def_id != def.id
                || def.preconditions.iter().all(|precondition| {
                    evaluate_precondition(*precondition, actor, &affordance.bound_targets, view)
                })
        });
    }

    affordances.sort();
    affordances.dedup();
    affordances
}

#[must_use]
fn evaluate_constraint(constraint: &Constraint, actor: EntityId, view: &dyn BeliefView) -> bool {
    match constraint {
        Constraint::ActorAlive => view.is_alive(actor),
        Constraint::ActorHasControl => view.has_control(actor),
        Constraint::ActorNotInTransit => !view.is_in_transit(actor),
        Constraint::ActorAtPlace(place) => view.effective_place(actor) == Some(*place),
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
fn evaluate_precondition(
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
    def_id: crate::ActionDefId,
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
        explanation: None,
    });
}

#[cfg(test)]
mod tests {
    use super::{enumerate_targets, evaluate_constraint, evaluate_precondition, get_affordances};
    use crate::{
        ActionDef, ActionDefId, ActionDefRegistry, ActionHandlerId, ActionPayload, Constraint,
        ConsumableEffect, DurationExpr, Interruptibility, OmniscientBeliefView, Precondition,
        ReservationReq, TargetSpec,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, BodyCostPerTick, CauseRef, CommodityConsumableProfile,
        CommodityKind, ControlSource, EntityId, EntityKind, EventLog, Quantity, RecipeId,
        ResourceSource, Tick, UniqueItemKind, VisibilitySpec, WitnessData, WorkstationTag, World,
        WorldTxn,
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
        controllable: BTreeMap<(EntityId, EntityId), bool>,
        control: BTreeMap<EntityId, bool>,
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

        fn reservation_conflicts(
            &self,
            _entity: EntityId,
            _range: worldwake_core::TickRange,
        ) -> bool {
            false
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
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

        let affordances = get_affordances(&view, actor, &registry);

        assert_eq!(affordances.len(), 1);
        assert_eq!(affordances[0].bound_targets, vec![bread]);
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

        let affordances = get_affordances(&view, actor, &registry);

        assert_eq!(affordances.len(), 3);
        assert_eq!(affordances[0].def_id, ActionDefId(0));
        assert_eq!(affordances[0].bound_targets, vec![target_a]);
        assert_eq!(affordances[1].def_id, ActionDefId(0));
        assert_eq!(affordances[1].bound_targets, vec![target_b]);
        assert_eq!(affordances[2].def_id, ActionDefId(1));
        assert_eq!(affordances[2].bound_targets, vec![target_b]);
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

        let affordances = get_affordances(&view, actor, &registry);

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

        assert!(get_affordances(&view, actor, &registry).is_empty());
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

        let human_affordances =
            get_affordances(&OmniscientBeliefView::new(&human_world), human, &registry);
        let ai_affordances = get_affordances(&OmniscientBeliefView::new(&ai_world), ai, &registry);

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

        let affordances = get_affordances(&OmniscientBeliefView::new(&world), actor, &registry);

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

        let affordances_a = get_affordances(&view_a, actor, &registry);
        let affordances_b = get_affordances(&view_b, actor, &registry);

        assert_eq!(affordances_a.len(), 1);
        assert_eq!(affordances_b.len(), 1);
        assert_eq!(affordances_a[0].bound_targets, vec![target_a]);
        assert_eq!(affordances_b[0].bound_targets, vec![target_b]);
        assert_ne!(affordances_a, affordances_b);
    }
}
