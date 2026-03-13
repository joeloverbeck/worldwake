use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::num::NonZeroU32;
use worldwake_core::{
    CombatProfile, CommodityConsumableProfile, CommodityKind, DemandObservation, DriveThresholds,
    EntityId, EntityKind, GrantedFacilityUse, HomeostaticNeeds, InTransitOnEdge, LoadUnits,
    MerchandiseProfile, MetabolismProfile, PlaceTag, Quantity, RecipeId, ResourceSource, TickRange,
    TradeDispositionProfile, UniqueItemKind, WorkstationTag, Wound,
};
use worldwake_sim::BeliefView;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotFacilityQueue {
    pub(crate) actor_queue_position: Option<u32>,
    pub(crate) active_grant: Option<GrantedFacilityUse>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotEntity {
    pub(crate) kind: Option<EntityKind>,
    pub(crate) effective_place: Option<EntityId>,
    pub(crate) in_transit_state: Option<InTransitOnEdge>,
    pub(crate) direct_container: Option<EntityId>,
    pub(crate) direct_possessor: Option<EntityId>,
    pub(crate) direct_possessions: BTreeSet<EntityId>,
    pub(crate) known_recipes: Vec<RecipeId>,
    pub(crate) unique_item_counts: BTreeMap<UniqueItemKind, u32>,
    pub(crate) commodity_quantities: BTreeMap<CommodityKind, Quantity>,
    pub(crate) item_lot_commodity: Option<CommodityKind>,
    pub(crate) carry_capacity: Option<LoadUnits>,
    pub(crate) intrinsic_load: LoadUnits,
    pub(crate) item_lot_consumable_profile: Option<CommodityConsumableProfile>,
    pub(crate) workstation_tag: Option<WorkstationTag>,
    pub(crate) resource_source: Option<ResourceSource>,
    pub(crate) action_flags: SnapshotActionFlags,
    pub(crate) lifecycle: SnapshotLifecycle,
    pub(crate) wounds: Vec<Wound>,
    pub(crate) homeostatic_needs: Option<HomeostaticNeeds>,
    pub(crate) drive_thresholds: Option<DriveThresholds>,
    pub(crate) metabolism_profile: Option<MetabolismProfile>,
    pub(crate) trade_disposition_profile: Option<TradeDispositionProfile>,
    pub(crate) combat_profile: Option<CombatProfile>,
    pub(crate) visible_hostiles: Vec<EntityId>,
    pub(crate) current_attackers: Vec<EntityId>,
    pub(crate) demand_memory: Vec<DemandObservation>,
    pub(crate) merchandise_profile: Option<MerchandiseProfile>,
    pub(crate) reservation_ranges: Vec<TickRange>,
    pub(crate) facility_queue: Option<SnapshotFacilityQueue>,
}

impl Default for SnapshotEntity {
    fn default() -> Self {
        Self {
            kind: None,
            effective_place: None,
            in_transit_state: None,
            direct_container: None,
            direct_possessor: None,
            direct_possessions: BTreeSet::new(),
            known_recipes: Vec::new(),
            unique_item_counts: BTreeMap::new(),
            commodity_quantities: BTreeMap::new(),
            item_lot_commodity: None,
            carry_capacity: None,
            intrinsic_load: LoadUnits(0),
            item_lot_consumable_profile: None,
            workstation_tag: None,
            resource_source: None,
            action_flags: SnapshotActionFlags::default(),
            lifecycle: SnapshotLifecycle::default(),
            wounds: Vec::new(),
            homeostatic_needs: None,
            drive_thresholds: None,
            metabolism_profile: None,
            trade_disposition_profile: None,
            combat_profile: None,
            visible_hostiles: Vec::new(),
            current_attackers: Vec::new(),
            demand_memory: Vec::new(),
            merchandise_profile: None,
            reservation_ranges: Vec::new(),
            facility_queue: None,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SnapshotActionFlags {
    pub(crate) has_production_job: bool,
    pub(crate) controllable_by_actor: bool,
    pub(crate) has_control: bool,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SnapshotLifecycle {
    pub(crate) alive: bool,
    pub(crate) dead: bool,
    pub(crate) incapacitated: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct SnapshotPlace {
    pub(crate) entities: BTreeSet<EntityId>,
    pub(crate) tags: BTreeSet<PlaceTag>,
    pub(crate) adjacent_places_with_travel_ticks: Vec<(EntityId, NonZeroU32)>,
}

pub struct PlanningSnapshot {
    pub(crate) actor: EntityId,
    pub(crate) entities: BTreeMap<EntityId, SnapshotEntity>,
    pub(crate) places: BTreeMap<EntityId, SnapshotPlace>,
}

impl PlanningSnapshot {
    #[must_use]
    pub fn build(
        view: &dyn BeliefView,
        actor: EntityId,
        evidence_entities: &BTreeSet<EntityId>,
        evidence_places: &BTreeSet<EntityId>,
        travel_horizon: u8,
    ) -> Self {
        let included_places = collect_places(
            view,
            actor,
            evidence_entities,
            evidence_places,
            travel_horizon,
        );
        let included_entities = collect_entities(view, actor, evidence_entities, &included_places);
        let places = build_snapshot_places(view, &included_places, &included_entities);
        let entities = included_entities
            .iter()
            .copied()
            .map(|entity| {
                (
                    entity,
                    build_snapshot_entity(view, actor, entity, evidence_entities, &included_places),
                )
            })
            .collect();

        Self {
            actor,
            entities,
            places,
        }
    }

    #[must_use]
    pub fn actor(&self) -> EntityId {
        self.actor
    }
}

fn build_snapshot_places(
    view: &dyn BeliefView,
    included_places: &BTreeSet<EntityId>,
    included_entities: &BTreeSet<EntityId>,
) -> BTreeMap<EntityId, SnapshotPlace> {
    included_places
        .iter()
        .copied()
        .map(|place| {
            let entities = included_entities
                .iter()
                .copied()
                .filter(|entity| view.effective_place(*entity) == Some(place))
                .collect();
            let adjacent_places_with_travel_ticks = view
                .adjacent_places_with_travel_ticks(place)
                .into_iter()
                .filter(|(adjacent, _)| included_places.contains(adjacent))
                .collect();
            let tags = PlaceTag::ALL
                .into_iter()
                .filter(|tag| view.place_has_tag(place, *tag))
                .collect();
            (
                place,
                SnapshotPlace {
                    entities,
                    tags,
                    adjacent_places_with_travel_ticks,
                },
            )
        })
        .collect()
}

fn build_snapshot_entity(
    view: &dyn BeliefView,
    actor: EntityId,
    entity: EntityId,
    evidence_entities: &BTreeSet<EntityId>,
    included_places: &BTreeSet<EntityId>,
) -> SnapshotEntity {
    SnapshotEntity {
        kind: view.entity_kind(entity),
        effective_place: view.effective_place(entity),
        in_transit_state: view.in_transit_state(entity),
        direct_container: view.direct_container(entity),
        direct_possessor: view.direct_possessor(entity),
        direct_possessions: view
            .direct_possessions(entity)
            .into_iter()
            .filter(|possessed| {
                included_entities_contains(
                    view,
                    *possessed,
                    actor,
                    evidence_entities,
                    included_places,
                )
            })
            .collect(),
        known_recipes: view.known_recipes(entity),
        unique_item_counts: collect_unique_item_counts(view, entity),
        commodity_quantities: collect_commodity_quantities(view, entity),
        item_lot_commodity: view.item_lot_commodity(entity),
        carry_capacity: view.carry_capacity(entity),
        intrinsic_load: view.load_of_entity(entity).unwrap_or(LoadUnits(0)),
        item_lot_consumable_profile: view.item_lot_consumable_profile(entity),
        workstation_tag: view.workstation_tag(entity),
        resource_source: view.resource_source(entity),
        action_flags: SnapshotActionFlags {
            has_production_job: view.has_production_job(entity),
            controllable_by_actor: view.can_control(actor, entity),
            has_control: view.has_control(entity),
        },
        lifecycle: SnapshotLifecycle {
            alive: view.is_alive(entity),
            dead: view.is_dead(entity),
            incapacitated: view.is_incapacitated(entity),
        },
        wounds: view.wounds(entity),
        homeostatic_needs: view.homeostatic_needs(entity),
        drive_thresholds: view.drive_thresholds(entity),
        metabolism_profile: view.metabolism_profile(entity),
        trade_disposition_profile: view.trade_disposition_profile(entity),
        combat_profile: view.combat_profile(entity),
        visible_hostiles: view.visible_hostiles_for(entity),
        current_attackers: view.current_attackers_of(entity),
        demand_memory: view.demand_memory(entity),
        merchandise_profile: view.merchandise_profile(entity),
        reservation_ranges: view.reservation_ranges(entity),
        facility_queue: snapshot_facility_queue(view, actor, entity),
    }
}

fn snapshot_facility_queue(
    view: &dyn BeliefView,
    actor: EntityId,
    entity: EntityId,
) -> Option<SnapshotFacilityQueue> {
    let has_policy = view.has_exclusive_facility_policy(entity);
    let actor_queue_position = view.facility_queue_position(entity, actor);
    let active_grant = view.facility_grant(entity).cloned();
    (has_policy || actor_queue_position.is_some() || active_grant.is_some()).then_some(
        SnapshotFacilityQueue {
        actor_queue_position,
        active_grant,
    },
    )
}

fn collect_unique_item_counts(
    view: &dyn BeliefView,
    entity: EntityId,
) -> BTreeMap<UniqueItemKind, u32> {
    UniqueItemKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let count = view.unique_item_count(entity, kind);
            (count > 0).then_some((kind, count))
        })
        .collect()
}

fn collect_commodity_quantities(
    view: &dyn BeliefView,
    entity: EntityId,
) -> BTreeMap<CommodityKind, Quantity> {
    CommodityKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let quantity = view.commodity_quantity(entity, kind);
            (quantity > Quantity(0)).then_some((kind, quantity))
        })
        .collect()
}

#[must_use]
pub fn build_planning_snapshot(
    view: &dyn BeliefView,
    actor: EntityId,
    evidence_entities: &BTreeSet<EntityId>,
    evidence_places: &BTreeSet<EntityId>,
    travel_horizon: u8,
) -> PlanningSnapshot {
    PlanningSnapshot::build(
        view,
        actor,
        evidence_entities,
        evidence_places,
        travel_horizon,
    )
}

fn collect_places(
    view: &dyn BeliefView,
    actor: EntityId,
    evidence_entities: &BTreeSet<EntityId>,
    evidence_places: &BTreeSet<EntityId>,
    travel_horizon: u8,
) -> BTreeSet<EntityId> {
    let mut included = evidence_places.clone();

    if let Some(actor_place) = view.effective_place(actor) {
        included.insert(actor_place);
        let mut frontier = VecDeque::from([(actor_place, 0u8)]);
        let mut visited = BTreeSet::from([actor_place]);
        while let Some((place, depth)) = frontier.pop_front() {
            if depth >= travel_horizon {
                continue;
            }
            for (adjacent, _) in view.adjacent_places_with_travel_ticks(place) {
                if visited.insert(adjacent) {
                    included.insert(adjacent);
                    frontier.push_back((adjacent, depth.saturating_add(1)));
                }
            }
        }
    }

    for entity in evidence_entities {
        if let Some(place) = view.effective_place(*entity) {
            included.insert(place);
        }
        if let Some(transit) = view.in_transit_state(*entity) {
            included.insert(transit.origin);
            included.insert(transit.destination);
        }
    }

    included
}

fn collect_entities(
    view: &dyn BeliefView,
    actor: EntityId,
    evidence_entities: &BTreeSet<EntityId>,
    included_places: &BTreeSet<EntityId>,
) -> BTreeSet<EntityId> {
    let mut included = BTreeSet::from([actor]);
    included.extend(evidence_entities.iter().copied());
    included.extend(included_places.iter().copied());

    for place in included_places {
        included.extend(view.entities_at(*place));
    }

    let mut frontier: VecDeque<_> = included.iter().copied().collect();
    while let Some(entity) = frontier.pop_front() {
        for related in view.direct_possessions(entity) {
            if included.insert(related) {
                frontier.push_back(related);
            }
        }
        if let Some(container) = view.direct_container(entity) {
            if included.insert(container) {
                frontier.push_back(container);
            }
        }
        if let Some(possessor) = view.direct_possessor(entity) {
            if included.insert(possessor) {
                frontier.push_back(possessor);
            }
        }
    }

    included
}

fn included_entities_contains(
    view: &dyn BeliefView,
    entity: EntityId,
    actor: EntityId,
    evidence_entities: &BTreeSet<EntityId>,
    included_places: &BTreeSet<EntityId>,
) -> bool {
    entity == actor
        || evidence_entities.contains(&entity)
        || view
            .effective_place(entity)
            .is_some_and(|place| included_places.contains(&place))
        || view.direct_possessor(entity).is_some()
        || view.direct_container(entity).is_some()
}

#[cfg(test)]
mod tests {
    use super::{build_planning_snapshot, SnapshotFacilityQueue};
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, CombatProfile, CommodityConsumableProfile, CommodityKind, DemandObservation,
        DriveThresholds, EntityId, EntityKind, GrantedFacilityUse, HomeostaticNeeds,
        InTransitOnEdge, LoadUnits, MerchandiseProfile, MetabolismProfile, Quantity, RecipeId,
        ResourceSource, Tick, TickRange, TradeDispositionProfile, UniqueItemKind, WorkstationTag,
        Wound,
    };
    use worldwake_sim::{ActionDuration, ActionPayload, BeliefView, DurationExpr};

    #[derive(Default)]
    struct StubBeliefView {
        alive: BTreeMap<EntityId, bool>,
        kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        adjacent: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
        carry_capacities: BTreeMap<EntityId, LoadUnits>,
        entity_loads: BTreeMap<EntityId, LoadUnits>,
        exclusive_facilities: BTreeSet<EntityId>,
        facility_queue_positions: BTreeMap<(EntityId, EntityId), u32>,
        facility_grants: BTreeMap<EntityId, GrantedFacilityUse>,
    }

    impl BeliefView for StubBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.get(&entity).copied().unwrap_or(false)
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

        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.entities_at.get(&place).cloned().unwrap_or_default()
        }

        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
            self.adjacent_places_with_travel_ticks(place)
                .into_iter()
                .map(|(adjacent, _)| adjacent)
                .collect()
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

        fn has_exclusive_facility_policy(&self, entity: EntityId) -> bool {
            self.exclusive_facilities.contains(&entity)
        }

        fn facility_queue_position(&self, facility: EntityId, actor: EntityId) -> Option<u32> {
            self.facility_queue_positions
                .get(&(facility, actor))
                .copied()
        }

        fn facility_grant(&self, facility: EntityId) -> Option<&GrantedFacilityUse> {
            self.facility_grants.get(&facility)
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
            actor == entity
        }

        fn has_control(&self, entity: EntityId) -> bool {
            self.kinds.get(&entity) == Some(&EntityKind::Agent)
        }

        fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.carry_capacities.get(&entity).copied()
        }

        fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.entity_loads.get(&entity).copied()
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
            self.adjacent.get(&place).cloned().unwrap_or_default()
        }

        fn estimate_duration(
            &self,
            _actor: EntityId,
            _duration: &DurationExpr,
            _targets: &[EntityId],
            _payload: &ActionPayload,
        ) -> Option<ActionDuration> {
            None
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    #[test]
    fn build_snapshot_includes_actor_evidence_and_places_within_horizon() {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let place_c = entity(12);
        let remote_place = entity(19);
        let evidence_entity = entity(2);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(evidence_entity, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(evidence_entity, EntityKind::Agent);
        view.effective_places.insert(actor, place_a);
        view.effective_places.insert(evidence_entity, remote_place);
        view.entities_at.insert(place_a, vec![actor]);
        view.entities_at.insert(place_b, vec![]);
        view.entities_at.insert(place_c, vec![]);
        view.entities_at.insert(remote_place, vec![evidence_entity]);
        view.adjacent
            .insert(place_a, vec![(place_b, NonZeroU32::new(3).unwrap())]);
        view.adjacent.insert(
            place_b,
            vec![
                (place_a, NonZeroU32::new(3).unwrap()),
                (place_c, NonZeroU32::new(5).unwrap()),
            ],
        );
        view.adjacent
            .insert(place_c, vec![(place_b, NonZeroU32::new(5).unwrap())]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([evidence_entity]),
            &BTreeSet::new(),
            1,
        );

        assert!(snapshot.entities.contains_key(&actor));
        assert!(snapshot.entities.contains_key(&evidence_entity));
        assert!(snapshot.places.contains_key(&place_a));
        assert!(snapshot.places.contains_key(&place_b));
        assert!(snapshot.places.contains_key(&remote_place));
        assert!(!snapshot.places.contains_key(&place_c));
    }

    #[test]
    fn build_snapshot_does_not_pull_in_unreachable_places_without_evidence() {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let place_c = entity(12);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.effective_places.insert(actor, place_a);
        view.entities_at.insert(place_a, vec![actor]);
        view.entities_at.insert(place_b, vec![]);
        view.entities_at.insert(place_c, vec![]);
        view.adjacent
            .insert(place_a, vec![(place_b, NonZeroU32::new(1).unwrap())]);
        view.adjacent.insert(
            place_b,
            vec![
                (place_a, NonZeroU32::new(1).unwrap()),
                (place_c, NonZeroU32::new(1).unwrap()),
            ],
        );
        view.adjacent
            .insert(place_c, vec![(place_b, NonZeroU32::new(1).unwrap())]);

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);

        assert!(snapshot.places.contains_key(&place_a));
        assert!(snapshot.places.contains_key(&place_b));
        assert!(!snapshot.places.contains_key(&place_c));
    }

    #[test]
    fn build_snapshot_captures_carry_capacity_and_intrinsic_load() {
        let actor = entity(1);
        let place = entity(10);
        let lot = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(place, true);
        view.alive.insert(lot, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(lot, EntityKind::ItemLot);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(lot, place);
        view.entities_at.insert(place, vec![actor, lot]);
        view.carry_capacities.insert(actor, LoadUnits(9));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(lot, LoadUnits(6));

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);

        assert_eq!(
            snapshot
                .entities
                .get(&actor)
                .and_then(|entity| entity.carry_capacity),
            Some(LoadUnits(9))
        );
        assert_eq!(
            snapshot
                .entities
                .get(&lot)
                .map(|entity| entity.intrinsic_load),
            Some(LoadUnits(6))
        );
        assert_eq!(
            snapshot
                .entities
                .get(&place)
                .map(|entity| entity.intrinsic_load),
            Some(LoadUnits(0))
        );
    }

    #[test]
    fn build_snapshot_captures_facility_queue_position_for_planning_actor() {
        let actor = entity(1);
        let place = entity(10);
        let facility = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(place, true);
        view.alive.insert(facility, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(facility, EntityKind::Facility);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(facility, place);
        view.entities_at.insert(place, vec![actor, facility]);
        view.facility_queue_positions.insert((facility, actor), 2);

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);

        assert_eq!(
            snapshot
                .entities
                .get(&facility)
                .and_then(|entity| entity.facility_queue.as_ref())
                .and_then(|queue| queue.actor_queue_position),
            Some(2)
        );
    }

    #[test]
    fn build_snapshot_captures_active_facility_grant() {
        let actor = entity(1);
        let other = entity(2);
        let place = entity(10);
        let facility = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(other, true);
        view.alive.insert(place, true);
        view.alive.insert(facility, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(other, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(facility, EntityKind::Facility);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(other, place);
        view.effective_places.insert(facility, place);
        view.entities_at.insert(place, vec![actor, other, facility]);
        view.facility_grants.insert(
            facility,
            GrantedFacilityUse {
                actor: other,
                intended_action: ActionDefId(77),
                granted_at: Tick(5),
                expires_at: Tick(8),
            },
        );

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);

        assert_eq!(
            snapshot
                .entities
                .get(&facility)
                .and_then(|entity| entity.facility_queue.as_ref())
                .and_then(|queue| queue.active_grant.clone()),
            Some(GrantedFacilityUse {
                actor: other,
                intended_action: ActionDefId(77),
                granted_at: Tick(5),
                expires_at: Tick(8),
            })
        );
    }

    #[test]
    fn build_snapshot_omits_facility_queue_data_when_none_exists() {
        let actor = entity(1);
        let place = entity(10);
        let facility = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(place, true);
        view.alive.insert(facility, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(facility, EntityKind::Facility);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(facility, place);
        view.entities_at.insert(place, vec![actor, facility]);

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);

        assert_eq!(
            snapshot
                .entities
                .get(&facility)
                .and_then(|entity| entity.facility_queue.as_ref()),
            None
        );
    }

    #[test]
    fn build_snapshot_keeps_empty_facility_queue_data_for_exclusive_facility() {
        let actor = entity(1);
        let place = entity(10);
        let facility = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(place, true);
        view.alive.insert(facility, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(facility, EntityKind::Facility);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(facility, place);
        view.entities_at.insert(place, vec![actor, facility]);
        view.exclusive_facilities.insert(facility);

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 0);

        assert_eq!(
            snapshot
                .entities
                .get(&facility)
                .and_then(|entity| entity.facility_queue.as_ref()),
            Some(&SnapshotFacilityQueue {
                actor_queue_position: None,
                active_grant: None,
            })
        );
    }
}
