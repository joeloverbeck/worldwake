use crate::{
    estimate_duration_from_beliefs, ActionDefRegistry, ActionDuration, ActionInstance,
    ActionInstanceId, ActionPayload, BeliefView, DurationExpr,
};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use worldwake_core::{
    is_incapacitated, CombatProfile, CommodityConsumableProfile, CommodityKind, ControlSource,
    DemandObservation, DriveThresholds, EntityId, EntityKind, HomeostaticNeeds, InTransitOnEdge,
    MerchandiseProfile, MetabolismProfile, Quantity, RecipeId, ResourceSource, TickRange,
    TradeDispositionProfile, UniqueItemKind, WorkstationTag, World, Wound,
};

#[derive(Clone, Copy)]
pub struct OmniscientBeliefRuntime<'a> {
    pub active_actions: &'a BTreeMap<ActionInstanceId, ActionInstance>,
    pub action_defs: &'a ActionDefRegistry,
}

impl<'a> OmniscientBeliefRuntime<'a> {
    #[must_use]
    pub const fn new(
        active_actions: &'a BTreeMap<ActionInstanceId, ActionInstance>,
        action_defs: &'a ActionDefRegistry,
    ) -> Self {
        Self {
            active_actions,
            action_defs,
        }
    }
}

/// Temporary stand-in until E14 provides per-agent belief stores.
/// MUST NOT be used in agent-facing code after E14 lands.
/// Wraps `&World` directly and returns authoritative truth, not beliefs.
pub struct OmniscientBeliefView<'w> {
    world: &'w World,
    runtime: Option<OmniscientBeliefRuntime<'w>>,
}

impl<'w> OmniscientBeliefView<'w> {
    #[must_use]
    pub const fn new(world: &'w World) -> Self {
        Self {
            world,
            runtime: None,
        }
    }

    #[must_use]
    pub const fn with_runtime(world: &'w World, runtime: OmniscientBeliefRuntime<'w>) -> Self {
        Self {
            world,
            runtime: Some(runtime),
        }
    }

    fn shares_local_context(&self, agent: EntityId, other: EntityId) -> bool {
        if self.world.effective_place(agent) == self.world.effective_place(other)
            && self.world.effective_place(agent).is_some()
        {
            return true;
        }

        matches!(
            (
                self.world.get_component_in_transit_on_edge(agent),
                self.world.get_component_in_transit_on_edge(other),
            ),
            (Some(agent_transit), Some(other_transit))
                if agent_transit.edge_id == other_transit.edge_id
        )
    }

    fn local_agent_entities_at(&self, place: EntityId) -> impl Iterator<Item = EntityId> + '_ {
        self.world
            .entities_effectively_at(place)
            .into_iter()
            .filter(|entity| self.world.entity_kind(*entity) == Some(EntityKind::Agent))
    }
}

impl BeliefView for OmniscientBeliefView<'_> {
    fn is_alive(&self, entity: EntityId) -> bool {
        self.world.is_alive(entity)
    }

    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
        self.world
            .is_alive(entity)
            .then(|| self.world.entity_kind(entity))
            .flatten()
    }

    fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
        self.world.effective_place(entity)
    }

    fn is_in_transit(&self, entity: EntityId) -> bool {
        self.world.is_in_transit(entity)
    }

    fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
        self.world.entities_effectively_at(place)
    }

    fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
        self.world.possessions_of(holder)
    }

    fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
        self.world.topology().neighbors(place)
    }

    fn knows_recipe(&self, actor: EntityId, recipe: RecipeId) -> bool {
        self.world
            .get_component_known_recipes(actor)
            .is_some_and(|known| known.recipes.contains(&recipe))
    }

    fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32 {
        self.world.controlled_unique_item_count(holder, kind)
    }

    fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
        self.world.controlled_commodity_quantity(holder, kind)
    }

    fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
        self.world
            .get_component_item_lot(entity)
            .map(|lot| lot.commodity)
    }

    fn item_lot_consumable_profile(&self, entity: EntityId) -> Option<CommodityConsumableProfile> {
        let commodity = self.item_lot_commodity(entity)?;
        commodity.spec().consumable_profile
    }

    fn direct_container(&self, entity: EntityId) -> Option<EntityId> {
        self.world.direct_container(entity)
    }

    fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
        self.world.possessor_of(entity)
    }

    fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag> {
        self.world
            .get_component_workstation_marker(entity)
            .map(|marker| marker.0)
    }

    fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
        self.world.get_component_resource_source(entity).cloned()
    }

    fn has_production_job(&self, entity: EntityId) -> bool {
        self.world.has_component_production_job(entity)
    }

    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
        self.world.can_exercise_control(actor, entity).is_ok()
    }

    fn has_control(&self, entity: EntityId) -> bool {
        self.world
            .get_component_agent_data(entity)
            .is_some_and(|agent_data| agent_data.control_source != ControlSource::None)
    }

    fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool {
        self.world
            .reservations_for(entity)
            .into_iter()
            .any(|reservation| reservation.range.overlaps(&range))
    }

    fn reservation_ranges(&self, entity: EntityId) -> Vec<TickRange> {
        self.world
            .reservations_for(entity)
            .into_iter()
            .map(|reservation| reservation.range)
            .collect()
    }

    fn is_dead(&self, entity: EntityId) -> bool {
        self.world.get_component_dead_at(entity).is_some()
    }

    fn is_incapacitated(&self, entity: EntityId) -> bool {
        let Some(wounds) = self.world.get_component_wound_list(entity) else {
            return false;
        };
        let Some(profile) = self.world.get_component_combat_profile(entity) else {
            return false;
        };
        is_incapacitated(wounds, profile)
    }

    fn has_wounds(&self, entity: EntityId) -> bool {
        self.world
            .get_component_wound_list(entity)
            .is_some_and(|wounds| !wounds.wounds.is_empty())
    }

    fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
        self.world.get_component_homeostatic_needs(agent).copied()
    }

    fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
        self.world.get_component_drive_thresholds(agent).copied()
    }

    fn metabolism_profile(&self, agent: EntityId) -> Option<MetabolismProfile> {
        self.world.get_component_metabolism_profile(agent).copied()
    }

    fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile> {
        self.world.get_component_trade_disposition_profile(agent).cloned()
    }

    fn combat_profile(&self, agent: EntityId) -> Option<CombatProfile> {
        self.world.get_component_combat_profile(agent).copied()
    }

    fn wounds(&self, agent: EntityId) -> Vec<Wound> {
        self.world
            .get_component_wound_list(agent)
            .map(|wounds| wounds.wounds.clone())
            .unwrap_or_default()
    }

    fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> {
        let mut hostiles = self
            .world
            .hostile_targets_of(agent)
            .into_iter()
            .chain(self.world.hostile_towards(agent))
            .filter(|entity| self.world.entity_kind(*entity) == Some(EntityKind::Agent))
            .filter(|entity| self.shares_local_context(agent, *entity))
            .collect::<BTreeSet<_>>();
        hostiles.extend(self.current_attackers_of(agent));
        hostiles.into_iter().collect()
    }

    fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId> {
        let Some(runtime) = self.runtime else {
            return Vec::new();
        };

        runtime
            .active_actions
            .values()
            .filter(|action| action.actor != agent)
            .filter(|action| action.targets.contains(&agent))
            .filter(|action| self.shares_local_context(agent, action.actor))
            .filter_map(|action| {
                let def = runtime.action_defs.get(action.def_id)?;
                (def.domain.counts_as_combat_engagement() && def.name == "attack")
                    .then_some(action.actor)
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn agents_selling_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
        self.local_agent_entities_at(place)
            .filter(|entity| {
                self.world
                    .get_component_merchandise_profile(*entity)
                    .is_some_and(|profile| profile.sale_kinds.contains(&commodity))
            })
            .collect()
    }

    fn known_recipes(&self, agent: EntityId) -> Vec<RecipeId> {
        self.world
            .get_component_known_recipes(agent)
            .map(|known| known.recipes.iter().copied().collect())
            .unwrap_or_default()
    }

    fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId> {
        self.world
            .entities_effectively_at(place)
            .into_iter()
            .filter(|entity| {
                self.world
                    .get_component_workstation_marker(*entity)
                    .is_some_and(|marker| marker.0 == tag)
            })
            .collect()
    }

    fn resource_sources_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
        self.world
            .entities_effectively_at(place)
            .into_iter()
            .filter(|entity| {
                self.world
                    .get_component_resource_source(*entity)
                    .is_some_and(|source| source.commodity == commodity)
            })
            .collect()
    }

    fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
        self.world
            .get_component_demand_memory(agent)
            .map(|memory| memory.observations.clone())
            .unwrap_or_default()
    }

    fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile> {
        self.world.get_component_merchandise_profile(agent).cloned()
    }

    fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId> {
        self.local_agent_entities_at(place)
            .filter(|entity| self.is_dead(*entity))
            .collect()
    }

    fn in_transit_state(&self, entity: EntityId) -> Option<InTransitOnEdge> {
        self.world.get_component_in_transit_on_edge(entity).cloned()
    }

    fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)> {
        self.world
            .topology()
            .outgoing_edges(place)
            .iter()
            .filter_map(|edge_id| self.world.topology().edge(*edge_id))
            .map(|edge| (edge.to(), NonZeroU32::new(edge.travel_time_ticks()).unwrap()))
            .collect()
    }

    fn estimate_duration(
        &self,
        actor: EntityId,
        duration: &DurationExpr,
        targets: &[EntityId],
        payload: &ActionPayload,
    ) -> Option<ActionDuration> {
        estimate_duration_from_beliefs(self, actor, duration, targets, payload)
    }
}

#[cfg(test)]
mod tests {
    use super::{OmniscientBeliefRuntime, OmniscientBeliefView};
    use crate::{
        ActionDef, ActionDefId, ActionDefRegistry, ActionDomain, ActionDuration, ActionHandlerId,
        ActionInstance, ActionInstanceId, ActionPayload, ActionStatus, BeliefView,
        Constraint, DurationExpr, Interruptibility, Precondition, ReservationReq, TargetSpec,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_prototype_world, BodyCostPerTick, BodyPart, CauseRef, CommodityKind, Container,
        ControlSource, DeadAt, DemandMemory, DemandObservation, DemandObservationReason,
        DriveThresholds, EventLog, HomeostaticNeeds, InTransitOnEdge, LoadUnits,
        MerchandiseProfile, Permille, Quantity, RecipeId, ResourceSource, Tick, TickRange,
        VisibilitySpec, WitnessData, WorkstationMarker, WorkstationTag, World, WorldTxn, Wound,
        WoundCause, WoundId, WoundList,
    };

    fn assert_belief_view<T: BeliefView>() {}

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

    fn open_container(capacity: u32) -> Container {
        Container {
            capacity: LoadUnits(capacity),
            allowed_commodities: None,
            allows_unique_items: true,
            allows_nested_containers: true,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn attack_action_def(id: ActionDefId) -> ActionDef {
        ActionDef {
            id,
            name: "attack".to_string(),
            domain: ActionDomain::Combat,
            actor_constraints: vec![
                Constraint::ActorAlive,
                Constraint::ActorNotDead,
                Constraint::ActorNotIncapacitated,
            ],
            targets: vec![TargetSpec::EntityAtActorPlace {
                kind: worldwake_core::EntityKind::Agent,
            }],
            preconditions: vec![
                Precondition::ActorAlive,
                Precondition::TargetExists(0),
                Precondition::TargetAlive(0),
            ],
            reservation_requirements: Vec::<ReservationReq>::new(),
            duration: DurationExpr::CombatWeapon,
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::ActorAlive],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
        }
    }

    fn defend_action_def(id: ActionDefId) -> ActionDef {
        ActionDef {
            id,
            name: "defend".to_string(),
            domain: ActionDomain::Combat,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: Vec::new(),
            preconditions: vec![Precondition::ActorAlive],
            reservation_requirements: Vec::<ReservationReq>::new(),
            duration: DurationExpr::Indefinite,
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::ActorAlive],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(1),
        }
    }

    #[test]
    fn omniscient_belief_view_implements_belief_view() {
        assert_belief_view::<OmniscientBeliefView<'_>>();
    }

    #[test]
    fn is_alive_and_entity_kind_reflect_world_lifecycle() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (archived, live) = {
            let mut txn = new_txn(&mut world, 1);
            let archived = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            let live = txn
                .create_item_lot(CommodityKind::Coin, Quantity(2))
                .unwrap();
            commit_txn(txn);
            (archived, live)
        };

        let mut txn = new_txn(&mut world, 3);
        txn.archive_entity(archived).unwrap();
        commit_txn(txn);

        let view = OmniscientBeliefView::new(&world);

        assert!(!view.is_alive(archived));
        assert!(view.is_alive(live));
        assert_eq!(view.entity_kind(archived), None);
        assert_eq!(
            view.entity_kind(live),
            Some(worldwake_core::EntityKind::ItemLot)
        );
    }

    #[test]
    fn effective_place_and_entities_at_include_contained_entities() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place = places[0];
        let other_place = places[1];

        let (root, inner, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let root = txn.create_container(open_container(20)).unwrap();
            let inner = txn.create_container(open_container(10)).unwrap();
            let lot = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            commit_txn(txn);
            (root, inner, lot)
        };

        {
            let mut txn = new_txn(&mut world, 4);
            txn.set_ground_location(root, place).unwrap();
            txn.put_into_container(inner, root).unwrap();
            txn.put_into_container(lot, inner).unwrap();
            txn.move_container_subtree(root, other_place).unwrap();
            commit_txn(txn);
        }

        let view = OmniscientBeliefView::new(&world);

        assert_eq!(view.effective_place(root), Some(other_place));
        assert_eq!(view.effective_place(inner), Some(other_place));
        assert_eq!(view.effective_place(lot), Some(other_place));
        assert_eq!(view.entities_at(place), Vec::new());
        assert_eq!(view.entities_at(other_place), vec![root, inner, lot]);
    }

    #[test]
    fn transit_and_adjacency_queries_reflect_world_topology_and_placement() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place = places[0];
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            commit_txn(txn);
            actor
        };

        let mut txn = new_txn(&mut world, 2);
        txn.set_in_transit(actor).unwrap();
        commit_txn(txn);

        let view = OmniscientBeliefView::new(&world);

        assert!(view.is_in_transit(actor));
        assert_eq!(view.effective_place(actor), None);
        assert_eq!(
            view.adjacent_places(place),
            world.topology().neighbors(place)
        );
    }

    #[test]
    fn commodity_quantity_sums_possessed_lots_and_contents_of_possessed_containers() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (actor, loose_bread, bag, bag_bread, bag_water, foreign_bread) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let loose_bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(2))
                .unwrap();
            let bag = txn.create_container(open_container(100)).unwrap();
            let bag_bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(4))
                .unwrap();
            let bag_water = txn
                .create_item_lot(CommodityKind::Water, Quantity(9))
                .unwrap();
            let foreign_bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(8))
                .unwrap();
            commit_txn(txn);
            (actor, loose_bread, bag, bag_bread, bag_water, foreign_bread)
        };

        {
            let mut txn = new_txn(&mut world, 7);
            txn.set_possessor(loose_bread, actor).unwrap();
            txn.set_possessor(bag, actor).unwrap();
            txn.set_ground_location(bag, place).unwrap();
            txn.put_into_container(bag_bread, bag).unwrap();
            txn.put_into_container(bag_water, bag).unwrap();
            commit_txn(txn);
        }

        let view = OmniscientBeliefView::new(&world);

        assert_eq!(
            view.commodity_quantity(actor, CommodityKind::Bread),
            Quantity(6)
        );
        assert_eq!(
            view.commodity_quantity(actor, CommodityKind::Water),
            Quantity(9)
        );
        assert_eq!(
            view.commodity_quantity(actor, CommodityKind::Coin),
            Quantity(0)
        );
        assert_eq!(
            view.commodity_quantity(foreign_bread, CommodityKind::Bread),
            Quantity(8)
        );
        assert_eq!(
            view.item_lot_commodity(loose_bread),
            Some(CommodityKind::Bread)
        );
        assert_eq!(
            view.item_lot_consumable_profile(bag_water)
                .unwrap()
                .thirst_relief_per_unit
                .value(),
            CommodityKind::Water
                .spec()
                .consumable_profile
                .unwrap()
                .thirst_relief_per_unit
                .value()
        );
        assert!(view.can_control(actor, bag_bread));
        assert!(!view.can_control(actor, foreign_bread));
    }

    #[test]
    fn has_control_requires_non_none_control_source() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (human, ai, dormant, item) = {
            let mut txn = new_txn(&mut world, 1);
            let human = txn.create_agent("Aster", ControlSource::Human).unwrap();
            let ai = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let dormant = txn.create_agent("Cato", ControlSource::None).unwrap();
            let item = txn
                .create_item_lot(CommodityKind::Coin, Quantity(1))
                .unwrap();
            commit_txn(txn);
            (human, ai, dormant, item)
        };

        let view = OmniscientBeliefView::new(&world);

        assert!(view.has_control(human));
        assert!(view.has_control(ai));
        assert!(!view.has_control(dormant));
        assert!(!view.has_control(item));
    }

    #[test]
    fn reservation_conflicts_uses_tick_range_overlap() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let (actor, item) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let item = txn
                .create_item_lot(CommodityKind::Coin, Quantity(1))
                .unwrap();
            commit_txn(txn);
            (actor, item)
        };

        {
            let mut txn = new_txn(&mut world, 3);
            txn.try_reserve(item, actor, TickRange::new(Tick(5), Tick(8)).unwrap())
                .unwrap();
            commit_txn(txn);
        }

        let view = OmniscientBeliefView::new(&world);

        assert!(view.reservation_conflicts(item, TickRange::new(Tick(4), Tick(6)).unwrap()));
        assert!(view.reservation_conflicts(item, TickRange::new(Tick(7), Tick(10)).unwrap()));
        assert!(!view.reservation_conflicts(item, TickRange::new(Tick(1), Tick(4)).unwrap()));
        assert!(!view.reservation_conflicts(item, TickRange::new(Tick(9), Tick(12)).unwrap()));
    }

    #[test]
    fn production_facts_reflect_known_recipes_workstations_and_sources() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (actor, workstation) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let workstation = txn.create_entity(worldwake_core::EntityKind::Facility);
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(workstation, place).unwrap();
            txn.set_component_known_recipes(
                actor,
                worldwake_core::KnownRecipes::with([RecipeId(3)]),
            )
            .unwrap();
            txn.set_component_workstation_marker(
                workstation,
                WorkstationMarker(WorkstationTag::OrchardRow),
            )
            .unwrap();
            txn.set_component_resource_source(
                workstation,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(4),
                    max_quantity: Quantity(6),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                },
            )
            .unwrap();
            commit_txn(txn);
            (actor, workstation)
        };

        let view = OmniscientBeliefView::new(&world);

        assert!(view.knows_recipe(actor, RecipeId(3)));
        assert_eq!(
            view.workstation_tag(workstation),
            Some(WorkstationTag::OrchardRow)
        );
        assert_eq!(
            view.resource_source(workstation),
            Some(ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(4),
                max_quantity: Quantity(6),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            })
        );
    }

    #[test]
    fn extended_component_queries_reflect_local_components_and_filter_remote_entities() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let local_place = places[0];
        let remote_place = places[1];

        let (
            actor,
            local_seller,
            remote_seller,
            local_hostile,
            local_corpse,
            remote_corpse,
            local_workstation,
            remote_workstation,
        ) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let local_seller = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let remote_seller = txn.create_agent("Cato", ControlSource::Ai).unwrap();
            let local_hostile = txn.create_agent("Dara", ControlSource::Ai).unwrap();
            let remote_hostile = txn.create_agent("Enid", ControlSource::Ai).unwrap();
            let local_corpse = txn.create_agent("Fara", ControlSource::None).unwrap();
            let remote_corpse = txn.create_agent("Glen", ControlSource::None).unwrap();
            let local_workstation = txn.create_entity(worldwake_core::EntityKind::Facility);
            let remote_workstation = txn.create_entity(worldwake_core::EntityKind::Facility);

            for entity in [actor, local_seller, local_hostile, local_corpse, local_workstation] {
                txn.set_ground_location(entity, local_place).unwrap();
            }
            for entity in [remote_seller, remote_hostile, remote_corpse, remote_workstation] {
                txn.set_ground_location(entity, remote_place).unwrap();
            }

            txn.set_component_homeostatic_needs(
                actor,
                HomeostaticNeeds::new(pm(320), pm(450), pm(120), pm(40), pm(80)),
            )
            .unwrap();
            txn.set_component_drive_thresholds(actor, DriveThresholds::default())
                .unwrap();
            txn.set_component_wound_list(
                actor,
                WoundList {
                    wounds: vec![Wound {
                        id: WoundId(1),
                        body_part: BodyPart::Torso,
                        cause: WoundCause::Deprivation(worldwake_core::DeprivationKind::Starvation),
                        severity: pm(250),
                        inflicted_at: Tick(1),
                        bleed_rate_per_tick: pm(0),
                    }],
                },
            )
            .unwrap();
            txn.set_component_known_recipes(
                actor,
                worldwake_core::KnownRecipes::with([RecipeId(2), RecipeId(4)]),
            )
            .unwrap();
            txn.set_component_demand_memory(
                actor,
                DemandMemory {
                    observations: vec![DemandObservation {
                        commodity: CommodityKind::Bread,
                        quantity: Quantity(3),
                        place: local_place,
                        tick: Tick(2),
                        counterparty: Some(local_seller),
                        reason: DemandObservationReason::WantedToBuyButSellerOutOfStock,
                    }],
                },
            )
            .unwrap();
            txn.set_component_merchandise_profile(
                actor,
                MerchandiseProfile {
                    sale_kinds: BTreeSet::from([CommodityKind::Bread, CommodityKind::Water]),
                    home_market: Some(local_place),
                },
            )
            .unwrap();
            txn.set_component_merchandise_profile(
                local_seller,
                MerchandiseProfile {
                    sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                    home_market: Some(local_place),
                },
            )
            .unwrap();
            txn.set_component_merchandise_profile(
                remote_seller,
                MerchandiseProfile {
                    sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                    home_market: Some(remote_place),
                },
            )
            .unwrap();
            txn.set_component_workstation_marker(
                local_workstation,
                WorkstationMarker(WorkstationTag::Mill),
            )
            .unwrap();
            txn.set_component_workstation_marker(
                remote_workstation,
                WorkstationMarker(WorkstationTag::Mill),
            )
            .unwrap();
            txn.set_component_resource_source(
                local_workstation,
                ResourceSource {
                    commodity: CommodityKind::Grain,
                    available_quantity: Quantity(5),
                    max_quantity: Quantity(7),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                },
            )
            .unwrap();
            txn.set_component_resource_source(
                remote_workstation,
                ResourceSource {
                    commodity: CommodityKind::Grain,
                    available_quantity: Quantity(9),
                    max_quantity: Quantity(9),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                },
            )
            .unwrap();
            txn.set_component_dead_at(local_corpse, DeadAt(Tick(3))).unwrap();
            txn.set_component_dead_at(remote_corpse, DeadAt(Tick(3))).unwrap();
            txn.add_hostility(actor, local_hostile).unwrap();
            txn.add_hostility(actor, remote_hostile).unwrap();
            commit_txn(txn);

            (
                actor,
                local_seller,
                remote_seller,
                local_hostile,
                local_corpse,
                remote_corpse,
                local_workstation,
                remote_workstation,
            )
        };

        let view = OmniscientBeliefView::new(&world);

        assert_eq!(
            view.homeostatic_needs(actor),
            Some(HomeostaticNeeds::new(pm(320), pm(450), pm(120), pm(40), pm(80)))
        );
        assert_eq!(view.drive_thresholds(actor), Some(DriveThresholds::default()));
        assert_eq!(view.wounds(actor).len(), 1);
        assert!(view.has_wounds(actor));
        assert_eq!(view.visible_hostiles_for(actor), vec![local_hostile]);
        assert_eq!(
            view.agents_selling_at(local_place, CommodityKind::Bread),
            vec![actor, local_seller]
        );
        assert_eq!(
            view.agents_selling_at(remote_place, CommodityKind::Bread),
            vec![remote_seller]
        );
        assert_eq!(view.known_recipes(actor), vec![RecipeId(2), RecipeId(4)]);
        assert_eq!(
            view.matching_workstations_at(local_place, WorkstationTag::Mill),
            vec![local_workstation]
        );
        assert_eq!(
            view.matching_workstations_at(remote_place, WorkstationTag::Mill),
            vec![remote_workstation]
        );
        assert_eq!(
            view.resource_sources_at(local_place, CommodityKind::Grain),
            vec![local_workstation]
        );
        assert_eq!(
            view.resource_sources_at(remote_place, CommodityKind::Grain),
            vec![remote_workstation]
        );
        assert_eq!(view.demand_memory(actor).len(), 1);
        assert_eq!(
            view.merchandise_profile(actor).unwrap().sale_kinds,
            BTreeSet::from([CommodityKind::Bread, CommodityKind::Water])
        );
        assert_eq!(view.corpse_entities_at(local_place), vec![local_corpse]);
        assert_eq!(view.corpse_entities_at(remote_place), vec![remote_corpse]);
    }

    #[test]
    fn runtime_attackers_transit_state_and_duration_queries_use_runtime_and_world_semantics() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let origin = places[0];
        let destination = world.topology().neighbors(origin)[0];
        let remote_place = places[2];
        let edge = world
            .topology()
            .unique_direct_edge(origin, destination)
            .unwrap()
            .unwrap()
            .clone();

        let (actor, local_attacker, remote_attacker, defender, traveler) = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let local_attacker = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let remote_attacker = txn.create_agent("Cato", ControlSource::Ai).unwrap();
            let defender = txn.create_agent("Dara", ControlSource::Ai).unwrap();
            let traveler = txn.create_agent("Enid", ControlSource::Ai).unwrap();

            txn.set_ground_location(actor, origin).unwrap();
            txn.set_ground_location(local_attacker, origin).unwrap();
            txn.set_ground_location(defender, origin).unwrap();
            txn.set_ground_location(remote_attacker, remote_place).unwrap();
            txn.set_in_transit(traveler).unwrap();
            txn.set_component_in_transit_on_edge(
                traveler,
                InTransitOnEdge {
                    edge_id: edge.id(),
                    origin,
                    destination,
                    departure_tick: Tick(5),
                    arrival_tick: Tick(u64::from(5 + edge.travel_time_ticks())),
                },
            )
            .unwrap();
            commit_txn(txn);

            (actor, local_attacker, remote_attacker, defender, traveler)
        };

        let mut action_defs = ActionDefRegistry::new();
        action_defs.register(attack_action_def(ActionDefId(0)));
        action_defs.register(defend_action_def(ActionDefId(1)));

        let active_actions = BTreeMap::from([
            (
                ActionInstanceId(1),
                ActionInstance {
                    instance_id: ActionInstanceId(1),
                    def_id: ActionDefId(0),
                    payload: ActionPayload::None,
                    actor: local_attacker,
                    targets: vec![actor],
                    start_tick: Tick(7),
                    remaining_duration: ActionDuration::Finite(2),
                    status: ActionStatus::Active,
                    reservation_ids: Vec::new(),
                    local_state: None,
                },
            ),
            (
                ActionInstanceId(2),
                ActionInstance {
                    instance_id: ActionInstanceId(2),
                    def_id: ActionDefId(0),
                    payload: ActionPayload::None,
                    actor: remote_attacker,
                    targets: vec![actor],
                    start_tick: Tick(7),
                    remaining_duration: ActionDuration::Finite(2),
                    status: ActionStatus::Active,
                    reservation_ids: Vec::new(),
                    local_state: None,
                },
            ),
            (
                ActionInstanceId(3),
                ActionInstance {
                    instance_id: ActionInstanceId(3),
                    def_id: ActionDefId(1),
                    payload: ActionPayload::None,
                    actor: defender,
                    targets: vec![actor],
                    start_tick: Tick(7),
                    remaining_duration: ActionDuration::Indefinite,
                    status: ActionStatus::Active,
                    reservation_ids: Vec::new(),
                    local_state: None,
                },
            ),
        ]);

        let view = OmniscientBeliefView::with_runtime(
            &world,
            OmniscientBeliefRuntime::new(&active_actions, &action_defs),
        );

        assert_eq!(view.current_attackers_of(actor), vec![local_attacker]);
        assert_eq!(view.visible_hostiles_for(actor), vec![local_attacker]);
        assert_eq!(
            view.in_transit_state(traveler),
            Some(InTransitOnEdge {
                edge_id: edge.id(),
                origin,
                destination,
                departure_tick: Tick(5),
                arrival_tick: Tick(u64::from(5 + edge.travel_time_ticks())),
            })
        );
        assert_eq!(
            view.adjacent_places_with_travel_ticks(origin),
            world.topology()
                .outgoing_edges(origin)
                .iter()
                .filter_map(|edge_id| world.topology().edge(*edge_id))
                .map(|edge| (edge.to(), NonZeroU32::new(edge.travel_time_ticks()).unwrap()))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            view.estimate_duration(
                actor,
                &DurationExpr::TravelToTarget { target_index: 0 },
                &[destination],
                &ActionPayload::None,
            ),
            Some(ActionDuration::Finite(edge.travel_time_ticks()))
        );
        assert_eq!(
            view.estimate_duration(
                actor,
                &DurationExpr::Fixed(NonZeroU32::new(3).unwrap()),
                &[],
                &ActionPayload::None,
            ),
            Some(ActionDuration::Finite(3))
        );
        assert_eq!(
            view.estimate_duration(actor, &DurationExpr::Indefinite, &[], &ActionPayload::None),
            Some(ActionDuration::Indefinite)
        );
    }
}
