use crate::BeliefView;
use worldwake_core::{
    CommodityConsumableProfile, CommodityKind, ControlSource, EntityId, EntityKind, Quantity,
    RecipeId, ResourceSource, TickRange, UniqueItemKind, WorkstationTag, World,
};

/// Temporary stand-in until E14 provides per-agent belief stores.
/// MUST NOT be used in agent-facing code after E14 lands.
/// Wraps `&World` directly and returns authoritative truth, not beliefs.
pub struct OmniscientBeliefView<'w> {
    world: &'w World,
}

impl<'w> OmniscientBeliefView<'w> {
    #[must_use]
    pub const fn new(world: &'w World) -> Self {
        Self { world }
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
}

#[cfg(test)]
mod tests {
    use super::OmniscientBeliefView;
    use crate::BeliefView;
    use worldwake_core::{
        build_prototype_world, CauseRef, CommodityKind, Container, ControlSource, EventLog,
        LoadUnits, Quantity, RecipeId, ResourceSource, Tick, TickRange, VisibilitySpec,
        WitnessData, WorkstationMarker, WorkstationTag, World, WorldTxn,
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
        assert_eq!(view.adjacent_places(place), world.topology().neighbors(place));
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
            txn.set_component_known_recipes(actor, worldwake_core::KnownRecipes::with([RecipeId(3)]))
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
        assert_eq!(view.workstation_tag(workstation), Some(WorkstationTag::OrchardRow));
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
}
