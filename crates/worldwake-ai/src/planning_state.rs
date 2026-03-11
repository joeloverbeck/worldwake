use crate::planning_snapshot::PlanningSnapshot;
use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    CombatProfile, CommodityKind, DemandObservation, DriveThresholds, EntityId, EntityKind,
    HomeostaticNeeds, InTransitOnEdge, MetabolismProfile, Permille, Quantity, RecipeId,
    ResourceSource, TickRange, TradeDispositionProfile, UniqueItemKind, WorkstationTag, Wound,
};
use worldwake_sim::{
    estimate_duration_from_beliefs, ActionDuration, ActionPayload, BeliefView, DurationExpr,
};

#[derive(Clone)]
pub struct PlanningState<'snapshot> {
    snapshot: &'snapshot PlanningSnapshot,
    entity_place_overrides: BTreeMap<EntityId, Option<EntityId>>,
    direct_container_overrides: BTreeMap<EntityId, Option<EntityId>>,
    direct_possessor_overrides: BTreeMap<EntityId, Option<EntityId>>,
    resource_quantity_overrides: BTreeMap<EntityId, Quantity>,
    commodity_quantity_overrides: BTreeMap<(EntityId, CommodityKind), Quantity>,
    reservation_shadows: BTreeMap<EntityId, Vec<TickRange>>,
    removed_entities: BTreeSet<EntityId>,
    needs_overrides: BTreeMap<EntityId, HomeostaticNeeds>,
    pain_overrides: BTreeMap<EntityId, Permille>,
}

impl<'snapshot> PlanningState<'snapshot> {
    #[must_use]
    pub fn new(snapshot: &'snapshot PlanningSnapshot) -> Self {
        Self {
            snapshot,
            entity_place_overrides: BTreeMap::new(),
            direct_container_overrides: BTreeMap::new(),
            direct_possessor_overrides: BTreeMap::new(),
            resource_quantity_overrides: BTreeMap::new(),
            commodity_quantity_overrides: BTreeMap::new(),
            reservation_shadows: BTreeMap::new(),
            removed_entities: BTreeSet::new(),
            needs_overrides: BTreeMap::new(),
            pain_overrides: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> &'snapshot PlanningSnapshot {
        self.snapshot
    }

    #[must_use]
    pub fn move_entity(mut self, entity: EntityId, destination: EntityId) -> Self {
        self.entity_place_overrides
            .insert(entity, Some(destination));
        self
    }

    #[must_use]
    pub fn move_actor_to(self, destination: EntityId) -> Self {
        let actor = self.snapshot.actor();
        self.move_entity(actor, destination)
    }

    #[must_use]
    pub fn move_lot_to_holder(
        mut self,
        lot: EntityId,
        holder: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Self {
        let previous_holder = self.direct_possessor(lot);
        self.direct_possessor_overrides.insert(lot, Some(holder));
        self.direct_container_overrides.insert(lot, None);
        if let Some(place) = self.resolve_effective_place(holder, &mut BTreeSet::new()) {
            self.entity_place_overrides.insert(lot, Some(place));
        }

        if let Some(previous_holder) = previous_holder {
            let current = self.commodity_quantity(previous_holder, commodity);
            let next = Quantity(current.0.saturating_sub(quantity.0));
            self.commodity_quantity_overrides
                .insert((previous_holder, commodity), next);
        }
        let current = self.commodity_quantity(holder, commodity);
        let next = Quantity(current.0.saturating_add(quantity.0));
        self.commodity_quantity_overrides
            .insert((holder, commodity), next);
        self
    }

    #[must_use]
    pub fn consume_commodity(mut self, commodity: CommodityKind) -> Self {
        let actor = self.snapshot.actor();
        let Some(mut needs) = self.homeostatic_needs(actor) else {
            return self;
        };
        let Some(thresholds) = self.drive_thresholds(actor) else {
            return self;
        };

        match commodity {
            CommodityKind::Bread | CommodityKind::Apple | CommodityKind::Grain => {
                needs.hunger = thresholds
                    .hunger
                    .low()
                    .saturating_sub(Permille::new(1).unwrap());
            }
            CommodityKind::Water => {
                needs.thirst = thresholds
                    .thirst
                    .low()
                    .saturating_sub(Permille::new(1).unwrap());
            }
            _ => {}
        }

        self.needs_overrides.insert(actor, needs);
        self
    }

    #[must_use]
    pub fn use_resource(mut self, source: EntityId, remaining_quantity: Quantity) -> Self {
        self.resource_quantity_overrides
            .insert(source, remaining_quantity);
        self
    }

    #[must_use]
    pub fn reserve(mut self, entity: EntityId, range: TickRange) -> Self {
        self.reservation_shadows
            .entry(entity)
            .or_default()
            .push(range);
        self
    }

    #[must_use]
    pub fn mark_removed(mut self, entity: EntityId) -> Self {
        self.removed_entities.insert(entity);
        self.entity_place_overrides.insert(entity, None);
        self.direct_container_overrides.insert(entity, None);
        self.direct_possessor_overrides.insert(entity, None);
        self
    }

    #[must_use]
    pub fn with_homeostatic_needs(mut self, entity: EntityId, needs: HomeostaticNeeds) -> Self {
        self.needs_overrides.insert(entity, needs);
        self
    }

    #[must_use]
    pub fn with_pain(mut self, entity: EntityId, pain: Permille) -> Self {
        self.pain_overrides.insert(entity, pain);
        self
    }

    #[must_use]
    pub fn pain_summary(&self, entity: EntityId) -> Option<Permille> {
        self.pain_overrides.get(&entity).copied().or_else(|| {
            self.snapshot.entities.get(&entity).map(|snapshot| {
                let total = snapshot.wounds.iter().fold(0u16, |acc, wound| {
                    acc.saturating_add(wound.severity.value())
                });
                Permille::new(total.min(1000)).unwrap()
            })
        })
    }

    fn resolve_effective_place(
        &self,
        entity: EntityId,
        visited: &mut BTreeSet<EntityId>,
    ) -> Option<EntityId> {
        if !visited.insert(entity) || self.removed_entities.contains(&entity) {
            return None;
        }
        if let Some(override_place) = self.entity_place_overrides.get(&entity) {
            return *override_place;
        }
        if let Some(possessor) = self.direct_possessor(entity) {
            return self.resolve_effective_place(possessor, visited);
        }
        if let Some(container) = self.direct_container(entity) {
            return self.resolve_effective_place(container, visited);
        }
        self.snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.effective_place)
    }
}

impl BeliefView for PlanningState<'_> {
    fn is_alive(&self, entity: EntityId) -> bool {
        !self.removed_entities.contains(&entity)
            && self
                .snapshot
                .entities
                .get(&entity)
                .is_some_and(|snapshot| snapshot.lifecycle.alive)
    }

    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
        self.is_alive(entity)
            .then(|| {
                self.snapshot
                    .entities
                    .get(&entity)
                    .and_then(|snapshot| snapshot.kind)
            })
            .flatten()
    }

    fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
        self.resolve_effective_place(entity, &mut BTreeSet::new())
    }

    fn is_in_transit(&self, entity: EntityId) -> bool {
        self.in_transit_state(entity).is_some()
    }

    fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
        let mut entities = self
            .snapshot
            .entities
            .keys()
            .copied()
            .filter(|entity| self.effective_place(*entity) == Some(place))
            .filter(|entity| !self.removed_entities.contains(entity))
            .collect::<Vec<_>>();
        entities.sort();
        entities.dedup();
        entities
    }

    fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
        let mut entities = self
            .snapshot
            .entities
            .keys()
            .copied()
            .filter(|entity| self.direct_possessor(*entity) == Some(holder))
            .filter(|entity| !self.removed_entities.contains(entity))
            .collect::<Vec<_>>();
        entities.sort();
        entities.dedup();
        entities
    }

    fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
        self.adjacent_places_with_travel_ticks(place)
            .into_iter()
            .map(|(adjacent, _)| adjacent)
            .collect()
    }

    fn knows_recipe(&self, actor: EntityId, recipe: RecipeId) -> bool {
        self.known_recipes(actor).contains(&recipe)
    }

    fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32 {
        self.snapshot
            .entities
            .get(&holder)
            .and_then(|snapshot| snapshot.unique_item_counts.get(&kind).copied())
            .unwrap_or(0)
    }

    fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
        self.commodity_quantity_overrides
            .get(&(holder, kind))
            .copied()
            .or_else(|| {
                self.snapshot
                    .entities
                    .get(&holder)
                    .and_then(|snapshot| snapshot.commodity_quantities.get(&kind).copied())
            })
            .unwrap_or(Quantity(0))
    }

    fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
        self.snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.item_lot_commodity)
    }

    fn item_lot_consumable_profile(
        &self,
        entity: EntityId,
    ) -> Option<worldwake_core::CommodityConsumableProfile> {
        self.snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.item_lot_consumable_profile)
    }

    fn direct_container(&self, entity: EntityId) -> Option<EntityId> {
        if self.removed_entities.contains(&entity) {
            return None;
        }
        match self.direct_container_overrides.get(&entity) {
            Some(override_value) => *override_value,
            None => self
                .snapshot
                .entities
                .get(&entity)
                .and_then(|snapshot| snapshot.direct_container),
        }
    }

    fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
        if self.removed_entities.contains(&entity) {
            return None;
        }
        match self.direct_possessor_overrides.get(&entity) {
            Some(override_value) => *override_value,
            None => self
                .snapshot
                .entities
                .get(&entity)
                .and_then(|snapshot| snapshot.direct_possessor),
        }
    }

    fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag> {
        self.snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.workstation_tag)
    }

    fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
        let mut source = self
            .snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.resource_source.clone())?;
        if let Some(quantity) = self.resource_quantity_overrides.get(&entity).copied() {
            source.available_quantity = quantity;
        }
        Some(source)
    }

    fn has_production_job(&self, entity: EntityId) -> bool {
        self.snapshot
            .entities
            .get(&entity)
            .is_some_and(|snapshot| snapshot.action_flags.has_production_job)
    }

    fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
        actor == self.snapshot.actor()
            && self
                .snapshot
                .entities
                .get(&entity)
                .is_some_and(|snapshot| snapshot.action_flags.controllable_by_actor)
    }

    fn has_control(&self, entity: EntityId) -> bool {
        self.snapshot
            .entities
            .get(&entity)
            .is_some_and(|snapshot| snapshot.action_flags.has_control)
    }

    fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool {
        self.reservation_shadows
            .get(&entity)
            .into_iter()
            .flatten()
            .any(|shadow| shadow.overlaps(&range))
            || self
                .snapshot
                .entities
                .get(&entity)
                .into_iter()
                .flat_map(|snapshot| snapshot.reservation_ranges.iter())
                .any(|existing| existing.overlaps(&range))
    }

    fn reservation_ranges(&self, entity: EntityId) -> Vec<TickRange> {
        let mut ranges = self
            .snapshot
            .entities
            .get(&entity)
            .map(|snapshot| snapshot.reservation_ranges.clone())
            .unwrap_or_default();
        if let Some(shadows) = self.reservation_shadows.get(&entity) {
            ranges.extend(shadows.iter().copied());
        }
        ranges
    }

    fn is_dead(&self, entity: EntityId) -> bool {
        self.removed_entities.contains(&entity)
            || self
                .snapshot
                .entities
                .get(&entity)
                .is_some_and(|snapshot| snapshot.lifecycle.dead)
    }

    fn is_incapacitated(&self, entity: EntityId) -> bool {
        self.snapshot
            .entities
            .get(&entity)
            .is_some_and(|snapshot| snapshot.lifecycle.incapacitated)
    }

    fn has_wounds(&self, entity: EntityId) -> bool {
        self.snapshot
            .entities
            .get(&entity)
            .is_some_and(|snapshot| !snapshot.wounds.is_empty())
    }

    fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
        self.needs_overrides.get(&agent).copied().or_else(|| {
            self.snapshot
                .entities
                .get(&agent)
                .and_then(|snapshot| snapshot.homeostatic_needs)
        })
    }

    fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.drive_thresholds)
    }

    fn metabolism_profile(&self, agent: EntityId) -> Option<MetabolismProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.metabolism_profile)
    }

    fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.trade_disposition_profile.clone())
    }

    fn combat_profile(&self, agent: EntityId) -> Option<CombatProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.combat_profile)
    }

    fn wounds(&self, agent: EntityId) -> Vec<Wound> {
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| snapshot.wounds.clone())
            .unwrap_or_default()
    }

    fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> {
        let agent_place = self.effective_place(agent);
        let agent_transit = self.in_transit_state(agent);
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| {
                snapshot
                    .visible_hostiles
                    .iter()
                    .copied()
                    .filter(|entity| !self.removed_entities.contains(entity))
                    .filter(|entity| {
                        self.effective_place(*entity) == agent_place
                            || agent_transit.is_some()
                                && self.in_transit_state(*entity) == agent_transit
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn current_attackers_of(&self, agent: EntityId) -> Vec<EntityId> {
        let agent_place = self.effective_place(agent);
        let agent_transit = self.in_transit_state(agent);
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| {
                snapshot
                    .current_attackers
                    .iter()
                    .copied()
                    .filter(|entity| !self.removed_entities.contains(entity))
                    .filter(|entity| {
                        self.effective_place(*entity) == agent_place
                            || agent_transit.is_some()
                                && self.in_transit_state(*entity) == agent_transit
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn agents_selling_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
        let mut sellers = self
            .entities_at(place)
            .into_iter()
            .filter(|entity| self.entity_kind(*entity) == Some(EntityKind::Agent))
            .filter(|entity| {
                self.snapshot
                    .entities
                    .get(entity)
                    .and_then(|snapshot| snapshot.merchandise_profile.as_ref())
                    .is_some_and(|profile| profile.sale_kinds.contains(&commodity))
            })
            .collect::<Vec<_>>();
        sellers.sort();
        sellers.dedup();
        sellers
    }

    fn known_recipes(&self, agent: EntityId) -> Vec<RecipeId> {
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| snapshot.known_recipes.clone())
            .unwrap_or_default()
    }

    fn matching_workstations_at(&self, place: EntityId, tag: WorkstationTag) -> Vec<EntityId> {
        self.entities_at(place)
            .into_iter()
            .filter(|entity| self.workstation_tag(*entity) == Some(tag))
            .collect()
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

    fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
        self.snapshot
            .entities
            .get(&agent)
            .map(|snapshot| snapshot.demand_memory.clone())
            .unwrap_or_default()
    }

    fn merchandise_profile(&self, agent: EntityId) -> Option<worldwake_core::MerchandiseProfile> {
        self.snapshot
            .entities
            .get(&agent)
            .and_then(|snapshot| snapshot.merchandise_profile.clone())
    }

    fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId> {
        self.entities_at(place)
            .into_iter()
            .filter(|entity| self.is_dead(*entity))
            .collect()
    }

    fn in_transit_state(&self, entity: EntityId) -> Option<InTransitOnEdge> {
        self.snapshot
            .entities
            .get(&entity)
            .and_then(|snapshot| snapshot.in_transit_state.clone())
    }

    fn adjacent_places_with_travel_ticks(
        &self,
        place: EntityId,
    ) -> Vec<(EntityId, std::num::NonZeroU32)> {
        self.snapshot
            .places
            .get(&place)
            .map(|snapshot| snapshot.adjacent_places_with_travel_ticks.clone())
            .unwrap_or_default()
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
    use super::PlanningState;
    use crate::planning_snapshot::build_planning_snapshot;
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        BodyCostPerTick, CombatProfile, CommodityConsumableProfile, CommodityKind,
        DemandObservation, DemandObservationReason, DriveThresholds, EntityId, EntityKind,
        HomeostaticNeeds, InTransitOnEdge, MerchandiseProfile, MetabolismProfile, Permille,
        Quantity, RecipeId, ResourceSource, Tick, TickRange, TradeDispositionProfile,
        UniqueItemKind, WorkstationTag, Wound, WoundCause, WoundId,
    };
    use worldwake_sim::{
        get_affordances, ActionDef, ActionDefId, ActionDefRegistry, ActionDomain, ActionDuration,
        ActionHandlerId, ActionPayload, BeliefView, Constraint, DurationExpr, Interruptibility,
        Precondition, ReservationReq, TargetSpec,
    };

    #[derive(Default)]
    struct StubBeliefView {
        alive: BTreeMap<EntityId, bool>,
        kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessors: BTreeMap<EntityId, EntityId>,
        adjacent: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
        item_lot_commodities: BTreeMap<EntityId, CommodityKind>,
        consumable_profiles: BTreeMap<EntityId, CommodityConsumableProfile>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        resource_sources: BTreeMap<EntityId, ResourceSource>,
        needs: BTreeMap<EntityId, HomeostaticNeeds>,
        thresholds: BTreeMap<EntityId, DriveThresholds>,
        demand_memory: BTreeMap<EntityId, Vec<DemandObservation>>,
        merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
        reservations: BTreeMap<EntityId, Vec<TickRange>>,
        durations: BTreeMap<(EntityId, ActionDefId), ActionDuration>,
        wounds: BTreeMap<EntityId, Vec<Wound>>,
        hostiles: BTreeMap<EntityId, Vec<EntityId>>,
        attackers: BTreeMap<EntityId, Vec<EntityId>>,
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

        fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
            self.direct_possessions
                .get(&holder)
                .cloned()
                .unwrap_or_default()
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

        fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
            self.commodity_quantities
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

        fn direct_container(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
            self.direct_possessors.get(&entity).copied()
        }

        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
            self.resource_sources.get(&entity).cloned()
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
            actor == entity || self.direct_possessor(entity) == Some(actor)
        }

        fn has_control(&self, entity: EntityId) -> bool {
            self.kinds.get(&entity) == Some(&EntityKind::Agent)
        }

        fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool {
            self.reservations
                .get(&entity)
                .into_iter()
                .flatten()
                .any(|existing| existing.overlaps(&range))
        }

        fn reservation_ranges(&self, entity: EntityId) -> Vec<TickRange> {
            self.reservations.get(&entity).cloned().unwrap_or_default()
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            !self.is_alive(entity)
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds
                .get(&entity)
                .is_some_and(|wounds| !wounds.is_empty())
        }

        fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
            self.needs.get(&agent).copied()
        }

        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
            self.thresholds.get(&agent).copied()
        }

        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            None
        }

        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
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
            self.entities_at(place)
                .into_iter()
                .filter(|entity| {
                    self.merchandise_profiles
                        .get(entity)
                        .is_some_and(|profile| profile.sale_kinds.contains(&commodity))
                })
                .collect()
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
                    self.resource_sources
                        .get(entity)
                        .is_some_and(|source| source.commodity == commodity)
                })
                .collect()
        }

        fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
            self.demand_memory.get(&agent).cloned().unwrap_or_default()
        }

        fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile> {
            self.merchandise_profiles.get(&agent).cloned()
        }

        fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.entities_at(place)
                .into_iter()
                .filter(|entity| self.is_dead(*entity))
                .collect()
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
            actor: EntityId,
            _duration: &DurationExpr,
            targets: &[EntityId],
            _payload: &ActionPayload,
        ) -> Option<ActionDuration> {
            let def_id = ActionDefId(targets.first().map_or(0, |target| target.slot));
            self.durations.get(&(actor, def_id)).copied()
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

    fn sample_registry() -> ActionDefRegistry {
        let mut registry = ActionDefRegistry::new();
        registry.register(ActionDef {
            id: ActionDefId(0),
            name: "eat".to_string(),
            domain: ActionDomain::Needs,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::EntityDirectlyPossessedByActor {
                kind: EntityKind::ItemLot,
            }],
            preconditions: vec![
                Precondition::TargetCommodity {
                    target_index: 0,
                    kind: CommodityKind::Bread,
                },
                Precondition::TargetHasConsumableEffect {
                    target_index: 0,
                    effect: worldwake_sim::ConsumableEffect::Hunger,
                },
            ],
            reservation_requirements: vec![ReservationReq { target_index: 0 }],
            duration: DurationExpr::Fixed(NonZeroU32::new(3).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::ActorAlive],
            visibility: worldwake_core::VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
        });
        registry
    }

    fn test_view() -> (StubBeliefView, EntityId, EntityId, EntityId, EntityId) {
        let actor = entity(1);
        let town = entity(10);
        let field = entity(11);
        let bread = entity(20);

        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(town, true);
        view.alive.insert(field, true);
        view.alive.insert(bread, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(field, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(bread, town);
        view.entities_at.insert(town, vec![actor, bread]);
        view.entities_at.insert(field, vec![]);
        view.direct_possessions.insert(actor, vec![bread]);
        view.direct_possessors.insert(bread, actor);
        view.item_lot_commodities
            .insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityConsumableProfile::new(NonZeroU32::new(2).unwrap(), pm(250), pm(0), pm(0)),
        );
        view.commodity_quantities
            .insert((actor, CommodityKind::Bread), Quantity(1));
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(700), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(2),
                place: town,
                tick: Tick(3),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );
        view.resource_sources.insert(
            bread,
            ResourceSource {
                commodity: CommodityKind::Bread,
                available_quantity: Quantity(4),
                max_quantity: Quantity(4),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        );
        view.adjacent
            .insert(town, vec![(field, NonZeroU32::new(5).unwrap())]);
        view.adjacent
            .insert(field, vec![(town, NonZeroU32::new(5).unwrap())]);
        view.wounds.insert(
            actor,
            vec![Wound {
                id: WoundId(1),
                body_part: worldwake_core::BodyPart::Torso,
                cause: WoundCause::Deprivation(worldwake_core::DeprivationKind::Starvation),
                severity: pm(200),
                inflicted_at: Tick(1),
                bleed_rate_per_tick: pm(0),
            }],
        );
        (view, actor, town, field, bread)
    }

    #[test]
    fn planning_state_implements_belief_view() {
        fn assert_impl<T: BeliefView>() {}
        assert_impl::<PlanningState<'_>>();
    }

    #[test]
    fn planning_state_without_overrides_matches_snapshot_answers() {
        let (view, actor, town, _field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot);

        assert_eq!(state.effective_place(actor), Some(town));
        assert_eq!(state.direct_possessions(actor), vec![bread]);
        assert_eq!(
            state.commodity_quantity(actor, CommodityKind::Bread),
            Quantity(1)
        );
        assert_eq!(state.demand_memory(actor), view.demand_memory(actor));
    }

    #[test]
    fn movement_and_possession_overrides_update_effective_queries() {
        let (view, actor, _town, field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot)
            .move_actor_to(field)
            .move_lot_to_holder(bread, actor, CommodityKind::Bread, Quantity(1));

        assert_eq!(state.effective_place(actor), Some(field));
        assert_eq!(state.effective_place(bread), Some(field));
        assert_eq!(state.entities_at(field), vec![actor, bread]);
        assert_eq!(state.direct_possessions(actor), vec![bread]);
    }

    #[test]
    fn resource_and_reservation_overrides_are_visible() {
        let (view, actor, _town, _field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let range = TickRange::new(Tick(4), Tick(6)).unwrap();
        let state = PlanningState::new(&snapshot)
            .use_resource(bread, Quantity(1))
            .reserve(bread, range);

        assert_eq!(
            state
                .resource_source(bread)
                .map(|source| source.available_quantity),
            Some(Quantity(1))
        );
        assert!(state.reservation_conflicts(bread, range));
    }

    #[test]
    fn removing_target_updates_lifecycle_and_affordances() {
        let (view, actor, _town, _field, bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let registry = sample_registry();

        let base = PlanningState::new(&snapshot);
        let removed = base.clone().mark_removed(bread);

        assert_eq!(get_affordances(&base, actor, &registry).len(), 1);
        assert!(removed.is_dead(bread));
        assert!(!removed.is_alive(bread));
        assert!(removed
            .entities_at(entity(10))
            .iter()
            .all(|entity| *entity != bread));
        assert!(get_affordances(&removed, actor, &registry).is_empty());
    }

    #[test]
    fn consume_override_reduces_hunger_conservatively() {
        let (view, actor, _town, _field, _bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let state = PlanningState::new(&snapshot).consume_commodity(CommodityKind::Bread);
        let thresholds = state.drive_thresholds(actor).unwrap();

        assert!(state.homeostatic_needs(actor).unwrap().hunger < thresholds.hunger.low());
    }

    #[test]
    fn overlay_clones_share_snapshot_owned_heavy_vectors() {
        let (view, actor, _town, field, _bread) = test_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 1);
        let base = PlanningState::new(&snapshot);
        let moved = base.clone().move_actor_to(field);

        let base_wounds = &base.snapshot().entities.get(&actor).unwrap().wounds;
        let moved_wounds = &moved.snapshot().entities.get(&actor).unwrap().wounds;
        let base_demand = &base.snapshot().entities.get(&actor).unwrap().demand_memory;
        let moved_demand = &moved.snapshot().entities.get(&actor).unwrap().demand_memory;

        assert!(std::ptr::eq(base_wounds.as_ptr(), moved_wounds.as_ptr()));
        assert!(std::ptr::eq(base_demand.as_ptr(), moved_demand.as_ptr()));
    }

    #[test]
    fn hostile_queries_respect_hypothetical_location_changes() {
        let actor = entity(1);
        let attacker = entity(2);
        let town = entity(10);
        let field = entity(11);
        let mut view = StubBeliefView::default();
        view.alive.insert(actor, true);
        view.alive.insert(attacker, true);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(attacker, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(field, EntityKind::Place);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(attacker, town);
        view.entities_at.insert(town, vec![actor, attacker]);
        view.entities_at.insert(field, vec![]);
        view.adjacent
            .insert(town, vec![(field, NonZeroU32::new(1).unwrap())]);
        view.adjacent
            .insert(field, vec![(town, NonZeroU32::new(1).unwrap())]);
        view.thresholds.insert(actor, DriveThresholds::default());
        view.hostiles.insert(actor, vec![attacker]);
        view.attackers.insert(actor, vec![attacker]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([attacker]),
            &BTreeSet::from([town, field]),
            1,
        );

        let moved = PlanningState::new(&snapshot).move_actor_to(field);

        assert!(moved.visible_hostiles_for(actor).is_empty());
        assert!(moved.current_attackers_of(actor).is_empty());
    }
}
