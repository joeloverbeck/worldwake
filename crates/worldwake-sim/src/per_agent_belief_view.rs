use crate::{
    estimate_duration_from_beliefs, ActionDefRegistry, ActionDuration, ActionInstance,
    ActionInstanceId, ActionPayload, DurationExpr, RuntimeBeliefView,
};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use worldwake_core::{
    is_incapacitated, load_of_entity, AgentBeliefStore, BeliefConfidencePolicy,
    BelievedEntityState, CarryCapacity, CombatProfile, CommodityConsumableProfile, CommodityKind,
    ControlSource, DemandObservation, DriveThresholds, EntityId, EntityKind, GrantedFacilityUse,
    HomeostaticNeeds, InTransitOnEdge, InstitutionalBeliefRead, LoadUnits, MerchandiseProfile,
    MetabolismProfile, OfficeData, Permille, PlaceTag, Quantity, RecipeId,
    RecipientKnowledgeStatus, ResourceSource, TellMemoryKey, TellProfile, Tick, TickRange,
    ToldBeliefMemory, TradeDispositionProfile, TravelDispositionProfile, UniqueItemKind,
    WorkstationTag, World, Wound,
};

#[derive(Clone, Copy)]
pub struct PerAgentBeliefRuntime<'a> {
    pub active_actions: &'a BTreeMap<ActionInstanceId, ActionInstance>,
    pub action_defs: &'a ActionDefRegistry,
}

impl<'a> PerAgentBeliefRuntime<'a> {
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

pub struct PerAgentBeliefView<'w> {
    agent: EntityId,
    current_tick: Tick,
    world: &'w World,
    belief_store: &'w AgentBeliefStore,
    runtime: Option<PerAgentBeliefRuntime<'w>>,
}

impl<'w> PerAgentBeliefView<'w> {
    #[must_use]
    pub const fn new(
        agent: EntityId,
        world: &'w World,
        belief_store: &'w AgentBeliefStore,
    ) -> Self {
        Self::new_at_tick(agent, Tick(0), world, belief_store)
    }

    #[must_use]
    pub const fn new_at_tick(
        agent: EntityId,
        current_tick: Tick,
        world: &'w World,
        belief_store: &'w AgentBeliefStore,
    ) -> Self {
        Self {
            agent,
            current_tick,
            world,
            belief_store,
            runtime: None,
        }
    }

    #[must_use]
    pub const fn with_runtime(
        agent: EntityId,
        world: &'w World,
        belief_store: &'w AgentBeliefStore,
        runtime: PerAgentBeliefRuntime<'w>,
    ) -> Self {
        Self::with_runtime_at_tick(agent, Tick(0), world, belief_store, runtime)
    }

    #[must_use]
    pub const fn with_runtime_at_tick(
        agent: EntityId,
        current_tick: Tick,
        world: &'w World,
        belief_store: &'w AgentBeliefStore,
        runtime: PerAgentBeliefRuntime<'w>,
    ) -> Self {
        Self {
            agent,
            current_tick,
            world,
            belief_store,
            runtime: Some(runtime),
        }
    }

    #[must_use]
    pub fn from_world(agent: EntityId, world: &'w World) -> Self {
        Self::from_world_at_tick(agent, Tick(0), world)
    }

    #[must_use]
    pub fn from_world_at_tick(agent: EntityId, current_tick: Tick, world: &'w World) -> Self {
        let belief_store = world
            .get_component_agent_belief_store(agent)
            .expect("agents must have AgentBeliefStore before constructing PerAgentBeliefView");
        Self::new_at_tick(agent, current_tick, world, belief_store)
    }

    #[must_use]
    pub fn with_runtime_from_world(
        agent: EntityId,
        world: &'w World,
        runtime: PerAgentBeliefRuntime<'w>,
    ) -> Self {
        Self::with_runtime_from_world_at_tick(agent, Tick(0), world, runtime)
    }

    #[must_use]
    pub fn with_runtime_from_world_at_tick(
        agent: EntityId,
        current_tick: Tick,
        world: &'w World,
        runtime: PerAgentBeliefRuntime<'w>,
    ) -> Self {
        let belief_store = world
            .get_component_agent_belief_store(agent)
            .expect("agents must have AgentBeliefStore before constructing PerAgentBeliefView");
        Self::with_runtime_at_tick(agent, current_tick, world, belief_store, runtime)
    }

    fn believed_entity(&self, entity: EntityId) -> Option<&BelievedEntityState> {
        (entity != self.agent)
            .then(|| self.belief_store.get_entity(&entity))
            .flatten()
    }

    fn knows_entity(&self, entity: EntityId) -> bool {
        entity == self.agent || self.believed_entity(entity).is_some()
    }

    fn shares_local_context(&self, agent: EntityId, other: EntityId) -> bool {
        if self.effective_place(agent) == self.effective_place(other)
            && self.effective_place(agent).is_some()
        {
            return true;
        }

        matches!(
            (self.in_transit_state(agent), self.in_transit_state(other)),
            (Some(agent_transit), Some(other_transit))
                if agent_transit.edge_id == other_transit.edge_id
        )
    }

    fn authoritative_local_controlled_lots_for(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Vec<EntityId> {
        let mut entities = self
            .world
            .entities_effectively_at(place)
            .into_iter()
            .filter(|entity| self.item_lot_commodity(*entity) == Some(commodity))
            .filter(|entity| self.can_control(agent, *entity))
            .collect::<Vec<_>>();
        entities.sort();
        entities.dedup();
        entities
    }
}

impl RuntimeBeliefView for PerAgentBeliefView<'_> {
    fn current_tick(&self) -> Tick {
        self.current_tick
    }

    fn is_alive(&self, entity: EntityId) -> bool {
        if entity == self.agent {
            return self.world.is_alive(entity);
        }

        self.believed_entity(entity)
            .is_some_and(|state| state.alive)
    }

    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
        match self.world.entity_kind(entity) {
            Some(EntityKind::Place) => Some(EntityKind::Place),
            kind => self.knows_entity(entity).then_some(kind).flatten(),
        }
    }

    fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
        if entity == self.agent {
            return self.world.effective_place(entity);
        }

        self.believed_entity(entity)
            .and_then(|state| state.last_known_place)
    }

    fn is_in_transit(&self, entity: EntityId) -> bool {
        if entity == self.agent {
            return self.world.is_in_transit(entity);
        }

        false
    }

    fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
        let mut entities = self
            .belief_store
            .known_entities
            .iter()
            .filter_map(|(entity, state)| {
                (state.last_known_place == Some(place)).then_some(*entity)
            })
            .collect::<Vec<_>>();
        if self.world.effective_place(self.agent) == Some(place) {
            entities.push(self.agent);
        }
        entities.sort();
        entities.dedup();
        entities
    }

    fn known_entity_beliefs(&self, agent: EntityId) -> Vec<(EntityId, BelievedEntityState)> {
        if agent != self.agent {
            return Vec::new();
        }

        self.belief_store
            .known_entities
            .iter()
            .map(|(entity, state)| (*entity, state.clone()))
            .collect()
    }

    fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
        if holder == self.agent {
            return self.world.possessions_of(holder);
        }

        Vec::new()
    }

    fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
        self.world.topology().neighbors(place)
    }

    fn knows_recipe(&self, actor: EntityId, recipe: RecipeId) -> bool {
        (actor == self.agent)
            && self
                .world
                .get_component_known_recipes(actor)
                .is_some_and(|known| known.recipes.contains(&recipe))
    }

    fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32 {
        if holder == self.agent {
            return self.world.controlled_unique_item_count(holder, kind);
        }

        0
    }

    fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
        if holder == self.agent {
            return self.world.controlled_commodity_quantity(holder, kind);
        }

        self.believed_entity(holder)
            .and_then(|state| state.last_known_inventory.get(&kind).copied())
            .unwrap_or(Quantity(0))
    }

    fn controlled_commodity_quantity_at_place(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Quantity {
        if agent != self.agent {
            return Quantity(0);
        }

        self.authoritative_local_controlled_lots_for(agent, place, commodity)
            .into_iter()
            .filter_map(|entity| self.world.get_component_item_lot(entity))
            .fold(Quantity(0), |total, lot| {
                Quantity(
                    total
                        .0
                        .checked_add(lot.quantity.0)
                        .expect("local controlled commodity quantity overflowed"),
                )
            })
    }

    fn local_controlled_lots_for(
        &self,
        agent: EntityId,
        place: EntityId,
        commodity: CommodityKind,
    ) -> Vec<EntityId> {
        if agent != self.agent {
            return Vec::new();
        }

        self.authoritative_local_controlled_lots_for(agent, place, commodity)
    }

    fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
        let accessible =
            self.knows_entity(entity) || self.world.possessor_of(entity) == Some(self.agent);
        accessible
            .then(|| {
                self.world
                    .get_component_item_lot(entity)
                    .map(|lot| lot.commodity)
            })
            .flatten()
    }

    fn item_lot_consumable_profile(&self, entity: EntityId) -> Option<CommodityConsumableProfile> {
        let commodity = self.item_lot_commodity(entity)?;
        commodity.spec().consumable_profile
    }

    fn direct_container(&self, entity: EntityId) -> Option<EntityId> {
        let accessible =
            self.knows_entity(entity) || self.world.possessor_of(entity) == Some(self.agent);
        accessible
            .then(|| self.world.direct_container(entity))
            .flatten()
    }

    fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
        let accessible =
            self.knows_entity(entity) || self.world.possessor_of(entity) == Some(self.agent);
        accessible
            .then(|| self.world.possessor_of(entity))
            .flatten()
    }

    fn believed_owner_of(&self, entity: EntityId) -> Option<EntityId> {
        let accessible =
            self.knows_entity(entity) || self.world.owner_of(entity) == Some(self.agent);
        accessible.then(|| self.world.owner_of(entity)).flatten()
    }

    fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag> {
        if entity == self.agent {
            return self
                .world
                .get_component_workstation_marker(entity)
                .map(|marker| marker.0);
        }

        self.believed_entity(entity)
            .and_then(|state| state.workstation_tag)
    }

    fn has_exclusive_facility_policy(&self, entity: EntityId) -> bool {
        self.world
            .get_component_exclusive_facility_policy(entity)
            .is_some()
    }

    fn facility_queue_position(&self, facility: EntityId, actor: EntityId) -> Option<u32> {
        self.world
            .get_component_facility_use_queue(facility)
            .and_then(|queue| queue.position_of(actor))
    }

    fn facility_grant(&self, facility: EntityId) -> Option<&GrantedFacilityUse> {
        self.world
            .get_component_facility_use_queue(facility)
            .and_then(|queue| queue.granted.as_ref())
    }

    fn facility_queue_join_tick(&self, facility: EntityId, actor: EntityId) -> Option<Tick> {
        self.world
            .get_component_facility_use_queue(facility)
            .and_then(|queue| {
                queue
                    .waiting
                    .values()
                    .find(|queued| queued.actor == actor)
                    .map(|queued| queued.queued_at)
            })
    }

    fn facility_queue_patience_ticks(&self, agent: EntityId) -> Option<NonZeroU32> {
        self.world
            .get_component_facility_queue_disposition_profile(agent)
            .and_then(|profile| profile.queue_patience_ticks)
    }

    fn place_has_tag(&self, place: EntityId, tag: PlaceTag) -> bool {
        self.world.place_has_tag(place, tag)
    }

    fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
        if entity == self.agent {
            return self.world.get_component_resource_source(entity).cloned();
        }

        self.believed_entity(entity)
            .and_then(|state| state.resource_source.clone())
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

    fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
        self.world
            .get_component_carry_capacity(entity)
            .map(|CarryCapacity(capacity)| *capacity)
    }

    fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
        load_of_entity(self.world, entity).ok()
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
        if entity == self.agent {
            return self.world.get_component_dead_at(entity).is_some();
        }

        self.believed_entity(entity)
            .is_some_and(|state| !state.alive)
    }

    fn is_incapacitated(&self, entity: EntityId) -> bool {
        if entity == self.agent {
            let Some(wounds) = self.world.get_component_wound_list(entity) else {
                return false;
            };
            let Some(profile) = self.world.get_component_combat_profile(entity) else {
                return false;
            };
            return is_incapacitated(wounds, profile);
        }

        false
    }

    fn has_wounds(&self, entity: EntityId) -> bool {
        if entity == self.agent {
            return self
                .world
                .get_component_wound_list(entity)
                .is_some_and(|wounds| !wounds.wounds.is_empty());
        }

        self.believed_entity(entity)
            .is_some_and(|state| !state.wounds.is_empty())
    }

    fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
        (agent == self.agent)
            .then(|| self.world.get_component_homeostatic_needs(agent).copied())
            .flatten()
    }

    fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
        (agent == self.agent)
            .then(|| self.world.get_component_drive_thresholds(agent).copied())
            .flatten()
    }

    fn belief_confidence_policy(&self, agent: EntityId) -> BeliefConfidencePolicy {
        assert_eq!(
            agent, self.agent,
            "belief_confidence_policy is a self-authoritative read and must only be requested for the acting agent"
        );
        self.world
            .get_component_perception_profile(agent)
            .map(|profile| profile.confidence_policy)
            .expect(
                "acting agents must have PerceptionProfile before reading belief confidence policy",
            )
    }

    fn metabolism_profile(&self, agent: EntityId) -> Option<MetabolismProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_metabolism_profile(agent).copied())
            .flatten()
    }

    fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_trade_disposition_profile(agent)
                    .cloned()
            })
            .flatten()
    }

    fn travel_disposition_profile(&self, agent: EntityId) -> Option<TravelDispositionProfile> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_travel_disposition_profile(agent)
                    .cloned()
            })
            .flatten()
    }

    fn tell_profile(&self, agent: EntityId) -> Option<TellProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_tell_profile(agent).copied())
            .flatten()
    }

    fn told_belief_memories(&self, agent: EntityId) -> Vec<(TellMemoryKey, ToldBeliefMemory)> {
        if agent != self.agent {
            return Vec::new();
        }

        self.belief_store
            .told_beliefs
            .iter()
            .map(|(key, memory)| (*key, memory.clone()))
            .collect()
    }

    fn told_belief_memory(
        &self,
        actor: EntityId,
        counterparty: EntityId,
        subject: EntityId,
    ) -> Option<ToldBeliefMemory> {
        if actor != self.agent {
            return None;
        }

        let profile = self.tell_profile(actor)?;
        self.belief_store
            .told_belief_memory(
                &TellMemoryKey {
                    counterparty,
                    subject,
                },
                self.current_tick,
                &profile,
            )
            .cloned()
    }

    fn recipient_knowledge_status(
        &self,
        actor: EntityId,
        counterparty: EntityId,
        subject: EntityId,
    ) -> Option<RecipientKnowledgeStatus> {
        if actor != self.agent {
            return None;
        }

        let current_belief = self.belief_store.get_entity(&subject)?;
        let profile = self.tell_profile(actor)?;
        Some(self.belief_store.recipient_knowledge_status(
            &TellMemoryKey {
                counterparty,
                subject,
            },
            current_belief,
            self.current_tick,
            &profile,
        ))
    }

    fn combat_profile(&self, agent: EntityId) -> Option<CombatProfile> {
        (agent == self.agent)
            .then(|| self.world.get_component_combat_profile(agent).copied())
            .flatten()
    }

    fn courage(&self, agent: EntityId) -> Option<Permille> {
        if agent == self.agent {
            return self
                .world
                .get_component_utility_profile(agent)
                .map(|p| p.courage);
        }
        self.believed_entity(agent)
            .and_then(|state| state.last_known_courage)
    }

    fn consultation_speed_factor(&self, agent: EntityId) -> Option<Permille> {
        (agent == self.agent)
            .then(|| {
                self.world
                    .get_component_perception_profile(agent)
                    .map(|profile| profile.consultation_speed_factor)
            })
            .flatten()
    }

    fn wounds(&self, agent: EntityId) -> Vec<Wound> {
        if agent == self.agent {
            return self
                .world
                .get_component_wound_list(agent)
                .map(|wounds| wounds.wounds.clone())
                .unwrap_or_default();
        }

        self.believed_entity(agent)
            .map(|state| state.wounds.clone())
            .unwrap_or_default()
    }

    fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> {
        if agent != self.agent {
            return Vec::new();
        }

        let mut hostiles = self
            .hostile_targets_of(agent)
            .into_iter()
            .chain(self.world.hostile_towards(agent))
            .filter(|entity| self.entity_kind(*entity) == Some(EntityKind::Agent))
            .filter(|entity| self.shares_local_context(agent, *entity))
            .filter(|entity| {
                self.believed_entity(*entity)
                    .is_some_and(|belief| belief.alive)
            })
            .collect::<BTreeSet<_>>();
        hostiles.extend(self.current_attackers_of(agent));
        hostiles.into_iter().collect()
    }

    fn hostile_targets_of(&self, agent: EntityId) -> Vec<EntityId> {
        if agent != self.agent {
            return Vec::new();
        }

        self.world
            .hostile_targets_of(agent)
            .into_iter()
            .filter(|entity| self.entity_kind(*entity) == Some(EntityKind::Agent))
            .filter(|entity| self.shares_local_context(agent, *entity))
            .filter(|entity| {
                self.believed_entity(*entity)
                    .is_some_and(|belief| belief.alive)
            })
            .collect()
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
        self.entities_at(place)
            .into_iter()
            .filter(|entity| self.entity_kind(*entity) == Some(EntityKind::Agent))
            .filter(|entity| {
                self.world
                    .get_component_merchandise_profile(*entity)
                    .is_some_and(|profile| profile.sale_kinds.contains(&commodity))
            })
            .collect()
    }

    fn known_recipes(&self, agent: EntityId) -> Vec<RecipeId> {
        if agent != self.agent {
            return Vec::new();
        }

        self.world
            .get_component_known_recipes(agent)
            .map(|known| known.recipes.iter().copied().collect())
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
        if agent != self.agent {
            return Vec::new();
        }

        self.world
            .get_component_demand_memory(agent)
            .map(|memory| memory.observations.clone())
            .unwrap_or_default()
    }

    fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile> {
        if agent == self.agent || self.believed_entity(agent).is_some() {
            return self.world.get_component_merchandise_profile(agent).cloned();
        }

        None
    }

    fn corpse_entities_at(&self, place: EntityId) -> Vec<EntityId> {
        self.entities_at(place)
            .into_iter()
            .filter(|entity| self.is_dead(*entity))
            .collect()
    }

    fn record_data(&self, record: EntityId) -> Option<worldwake_core::RecordData> {
        (self.entity_kind(record) == Some(EntityKind::Record))
            .then(|| self.world.get_component_record_data(record).cloned())
            .flatten()
    }

    fn office_data(&self, office: EntityId) -> Option<OfficeData> {
        (self.entity_kind(office) == Some(EntityKind::Office))
            .then(|| self.world.get_component_office_data(office).cloned())
            .flatten()
    }

    fn office_holder(&self, office: EntityId) -> Option<EntityId> {
        if self.entity_kind(office) != Some(EntityKind::Office) {
            return None;
        }

        self.world
            .office_holder(office)
            .filter(|holder| *holder == self.agent || self.believed_entity(*holder).is_some())
    }

    fn believed_office_holder(
        &self,
        office: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        self.belief_store.believed_office_holder(office)
    }

    fn factions_of(&self, member: EntityId) -> Vec<EntityId> {
        if member != self.agent && self.believed_entity(member).is_none() {
            return Vec::new();
        }

        self.world.factions_of(member)
    }

    fn loyalty_to(&self, subject: EntityId, target: EntityId) -> Option<worldwake_core::Permille> {
        if subject != self.agent {
            return None;
        }
        if target != self.agent && self.believed_entity(target).is_none() {
            return None;
        }

        self.world.loyalty_to(subject, target)
    }

    fn support_declaration(&self, supporter: EntityId, office: EntityId) -> Option<EntityId> {
        if supporter != self.agent || self.entity_kind(office) != Some(EntityKind::Office) {
            return None;
        }

        self.world.support_declaration(supporter, office)
    }

    fn believed_support_declaration(
        &self,
        office: EntityId,
        supporter: EntityId,
    ) -> InstitutionalBeliefRead<Option<EntityId>> {
        self.belief_store
            .believed_support_declaration(office, supporter)
    }

    fn support_declarations_for_office(&self, office: EntityId) -> Vec<(EntityId, EntityId)> {
        // Pre-E14: delegate to world directly, matching support_declaration() pattern.
        // Post-E14: gate by observation (agent must have perceived each declaration).
        self.world.support_declarations_for_office(office)
    }

    fn believed_support_declarations_for_office(
        &self,
        office: EntityId,
    ) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)> {
        self.belief_store
            .believed_support_declarations_for_office(office)
    }

    fn in_transit_state(&self, entity: EntityId) -> Option<InTransitOnEdge> {
        if entity == self.agent {
            return self.world.get_component_in_transit_on_edge(entity).cloned();
        }

        None
    }

    fn adjacent_places_with_travel_ticks(&self, place: EntityId) -> Vec<(EntityId, NonZeroU32)> {
        self.world
            .topology()
            .outgoing_edges(place)
            .iter()
            .filter_map(|edge_id| self.world.topology().edge(*edge_id))
            .map(|edge| {
                (
                    edge.to(),
                    NonZeroU32::new(edge.travel_time_ticks()).unwrap(),
                )
            })
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

crate::impl_goal_belief_view!(PerAgentBeliefView<'_>);

#[cfg(test)]
mod tests {
    use super::{PerAgentBeliefRuntime, PerAgentBeliefView};
    use crate::{
        ActionDef, ActionDefRegistry, ActionDomain, ActionDuration, ActionHandlerId,
        ActionInstance, ActionInstanceId, ActionPayload, ActionStatus, Constraint, DurationExpr,
        GoalBeliefView, Interruptibility, Precondition, ReservationReq, RuntimeBeliefView,
        TargetSpec,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        build_believed_entity_state, build_prototype_world, ActionDefId, AgentBeliefStore,
        BeliefConfidencePolicy, BelievedEntityState, BodyCostPerTick, BodyPart, CauseRef,
        CombatProfile, CommodityKind, ControlSource, EntityKind, EventLog, FactionData,
        FactionPurpose, InstitutionalBeliefKey, InstitutionalBeliefRead, InstitutionalClaim,
        InstitutionalKnowledgeSource, MerchandiseProfile, OfficeData, PerceptionProfile,
        Permille, Quantity, RecordData, RecordKind, RecipientKnowledgeStatus, ResourceSource,
        SuccessionLaw, TellMemoryKey, Tick, ToldBeliefMemory, UtilityProfile, VisibilitySpec,
        WitnessData, WorkstationMarker, WorkstationTag, World, WorldTxn, Wound, WoundCause,
        WoundId,
    };

    fn assert_goal_belief_view<T: GoalBeliefView>() {}
    fn assert_runtime_belief_view<T: RuntimeBeliefView>() {}

    fn entity_belief(
        place: worldwake_core::EntityId,
        alive: bool,
        bread: u32,
        observed_tick: u64,
    ) -> BelievedEntityState {
        let mut inventory = BTreeMap::new();
        inventory.insert(CommodityKind::Bread, Quantity(bread));
        BelievedEntityState {
            last_known_place: Some(place),
            last_known_inventory: inventory,
            workstation_tag: None,
            resource_source: None,
            alive,
            wounds: if alive {
                Vec::new()
            } else {
                vec![sample_wound()]
            },
            last_known_courage: None,
            observed_tick: Tick(observed_tick),
            source: worldwake_core::PerceptionSource::DirectObservation,
        }
    }

    fn sample_wound() -> Wound {
        Wound {
            id: WoundId(1),
            body_part: BodyPart::Torso,
            cause: WoundCause::Combat {
                attacker: entity(99),
                weapon: worldwake_core::CombatWeaponRef::Unarmed,
            },
            severity: Permille::new(250).unwrap(),
            inflicted_at: Tick(5),
            bleed_rate_per_tick: Permille::new(5).unwrap(),
        }
    }

    fn entity(slot: u32) -> worldwake_core::EntityId {
        worldwake_core::EntityId {
            slot,
            generation: 0,
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

    fn create_record(
        txn: &mut WorldTxn<'_>,
        place: worldwake_core::EntityId,
        issuer: worldwake_core::EntityId,
        kind: RecordKind,
    ) {
        let _ = txn
            .create_record(RecordData {
                record_kind: kind,
                home_place: place,
                issuer,
                consultation_ticks: 4,
                max_entries_per_consult: 6,
                entries: Vec::new(),
                next_entry_id: 0,
            })
            .unwrap();
    }

    fn commit_txn(txn: WorldTxn<'_>) {
        let mut log = EventLog::new();
        let _ = txn.commit(&mut log);
    }

    fn attack_action_def(id: ActionDefId) -> ActionDef {
        ActionDef {
            id,
            name: "attack".to_string(),
            domain: ActionDomain::Combat,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::EntityAtActorPlace {
                kind: worldwake_core::EntityKind::Agent,
            }],
            preconditions: vec![Precondition::ActorAlive, Precondition::TargetAlive(0)],
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

    #[test]
    fn per_agent_belief_view_implements_goal_and_runtime_surfaces() {
        assert_goal_belief_view::<PerAgentBeliefView<'_>>();
        assert_runtime_belief_view::<PerAgentBeliefView<'_>>();
    }

    #[test]
    fn self_queries_are_authoritative_and_other_queries_use_beliefs() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place = places[0];
        let believed_place = places[1];
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(other, place).unwrap();
            commit_txn(txn);
            (agent, other)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(other, entity_belief(believed_place, false, 7, 10));

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::homeostatic_needs(&view, agent),
            world.get_component_homeostatic_needs(agent).copied()
        );
        assert_eq!(
            RuntimeBeliefView::effective_place(&view, agent),
            Some(place)
        );
        assert_eq!(
            RuntimeBeliefView::commodity_quantity(&view, agent, CommodityKind::Bread),
            world.controlled_commodity_quantity(agent, CommodityKind::Bread)
        );
        assert_eq!(
            RuntimeBeliefView::effective_place(&view, other),
            Some(believed_place)
        );
        assert!(!RuntimeBeliefView::is_alive(&view, other));
        assert!(RuntimeBeliefView::is_dead(&view, other));
        assert_eq!(
            RuntimeBeliefView::commodity_quantity(&view, other, CommodityKind::Bread),
            Quantity(7)
        );
        assert_eq!(
            RuntimeBeliefView::wounds(&view, other),
            vec![sample_wound()]
        );
    }

    #[test]
    fn unknown_entities_and_unbelieved_merchants_stay_hidden() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, believed_merchant, hidden_merchant) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let believed_merchant = txn.create_agent("Seller", ControlSource::Ai).unwrap();
            let hidden_merchant = txn.create_agent("Hidden", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(believed_merchant, place).unwrap();
            txn.set_ground_location(hidden_merchant, place).unwrap();
            txn.set_component_merchandise_profile(
                believed_merchant,
                MerchandiseProfile {
                    sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                    home_market: Some(place),
                },
            )
            .unwrap();
            txn.set_component_merchandise_profile(
                hidden_merchant,
                MerchandiseProfile {
                    sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                    home_market: Some(place),
                },
            )
            .unwrap();
            commit_txn(txn);
            (agent, believed_merchant, hidden_merchant)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(believed_merchant, entity_belief(place, true, 3, 5));
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::effective_place(&view, hidden_merchant),
            None
        );
        assert!(!RuntimeBeliefView::is_alive(&view, hidden_merchant));
        assert_eq!(
            RuntimeBeliefView::commodity_quantity(&view, hidden_merchant, CommodityKind::Bread),
            Quantity(0)
        );
        assert_eq!(
            RuntimeBeliefView::entities_at(&view, place),
            vec![agent, believed_merchant]
        );
        assert_eq!(
            RuntimeBeliefView::agents_selling_at(&view, place, CommodityKind::Bread),
            vec![believed_merchant]
        );
    }

    #[test]
    fn stale_beliefs_do_not_auto_refresh_from_world() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place_a = places[0];
        let place_b = places[1];
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place_a).unwrap();
            txn.set_ground_location(other, place_b).unwrap();
            commit_txn(txn);
            (agent, other)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(other, entity_belief(place_a, true, 1, 2));
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(world.effective_place(other), Some(place_b));
        assert_eq!(
            RuntimeBeliefView::effective_place(&view, other),
            Some(place_a)
        );
    }

    #[test]
    fn known_entity_beliefs_expose_only_actor_subjective_memory() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(other, place).unwrap();
            commit_txn(txn);
            (agent, other)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(other, entity_belief(place, true, 2, 4));
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::known_entity_beliefs(&view, agent),
            vec![(other, entity_belief(place, true, 2, 4))]
        );
        assert!(
            RuntimeBeliefView::known_entity_beliefs(&view, other).is_empty(),
            "belief enumeration should not expose another agent's store through this view"
        );
    }

    #[test]
    fn runtime_view_exposes_retention_aware_told_belief_memory_and_recipient_status() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, listener, subject) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let listener = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let subject = txn.create_agent("Cora", ControlSource::Ai).unwrap();
            for entity in [agent, listener, subject] {
                txn.set_ground_location(entity, place).unwrap();
            }
            commit_txn(txn);
            (agent, listener, subject)
        };

        let current_belief = entity_belief(place, true, 2, 6);
        let mut stale_belief = current_belief.clone();
        stale_belief
            .last_known_inventory
            .insert(CommodityKind::Bread, Quantity(1));

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(subject, current_belief.clone());
        beliefs.record_told_belief(
            TellMemoryKey {
                counterparty: listener,
                subject,
            },
            ToldBeliefMemory {
                shared_state: worldwake_core::to_shared_belief_snapshot(&stale_belief),
                told_tick: Tick(4),
            },
        );

        let view = PerAgentBeliefView::new_at_tick(agent, Tick(6), &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::told_belief_memory(&view, agent, listener, subject)
                .map(|m| m.told_tick),
            Some(Tick(4))
        );
        assert_eq!(
            RuntimeBeliefView::recipient_knowledge_status(&view, agent, listener, subject),
            Some(RecipientKnowledgeStatus::SpeakerHasOnlyToldStaleBelief)
        );

        let expired_view = PerAgentBeliefView::new_at_tick(agent, Tick(60), &world, &beliefs);
        assert_eq!(
            RuntimeBeliefView::told_belief_memory(&expired_view, agent, listener, subject),
            None
        );
        assert_eq!(
            RuntimeBeliefView::recipient_knowledge_status(&expired_view, agent, listener, subject),
            Some(RecipientKnowledgeStatus::SpeakerPreviouslyToldButMemoryExpired)
        );
    }

    #[test]
    fn runtime_view_hides_other_agents_conversation_memory() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, other, listener, subject) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let listener = txn.create_agent("Cora", ControlSource::Ai).unwrap();
            let subject = txn.create_agent("Dain", ControlSource::Ai).unwrap();
            for entity in [agent, other, listener, subject] {
                txn.set_ground_location(entity, place).unwrap();
            }
            commit_txn(txn);
            (agent, other, listener, subject)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(subject, entity_belief(place, true, 1, 4));
        beliefs.record_told_belief(
            TellMemoryKey {
                counterparty: listener,
                subject,
            },
            ToldBeliefMemory {
                shared_state: worldwake_core::to_shared_belief_snapshot(&entity_belief(
                    place, true, 1, 4,
                )),
                told_tick: Tick(4),
            },
        );

        let view = PerAgentBeliefView::new_at_tick(agent, Tick(6), &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::told_belief_memories(&view, agent).len(),
            1
        );
        assert!(
            RuntimeBeliefView::told_belief_memories(&view, other).is_empty(),
            "conversation memory should remain actor-local"
        );
        assert_eq!(
            RuntimeBeliefView::told_belief_memory(&view, other, listener, subject),
            None
        );
        assert_eq!(
            RuntimeBeliefView::recipient_knowledge_status(&view, other, listener, subject),
            None
        );
    }

    #[test]
    fn tell_profile_returns_none_when_component_missing() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.clear_component_tell_profile(agent).unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(RuntimeBeliefView::tell_profile(&view, agent), None);
    }

    #[test]
    fn belief_confidence_policy_returns_actor_policy() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_perception_profile(
                agent,
                PerceptionProfile {
                    confidence_policy: BeliefConfidencePolicy {
                        rumor_base: Permille::new(875).unwrap(),
                        staleness_penalty_per_tick: Permille::new(7).unwrap(),
                        ..BeliefConfidencePolicy::default()
                    },
                    ..PerceptionProfile::default()
                },
            )
            .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        let expected = world
            .get_component_perception_profile(agent)
            .unwrap()
            .confidence_policy;

        assert_eq!(
            RuntimeBeliefView::belief_confidence_policy(&view, agent),
            expected
        );
        assert_eq!(
            GoalBeliefView::belief_confidence_policy(&view, agent),
            expected
        );
    }

    #[test]
    #[should_panic(expected = "self-authoritative read")]
    fn belief_confidence_policy_rejects_non_self_reads() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Bryn", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(other, place).unwrap();
            commit_txn(txn);
            (agent, other)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        let _ = RuntimeBeliefView::belief_confidence_policy(&view, other);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn remote_facility_discovery_requires_believed_entity_snapshot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place = places[0];
        let remote_place = world.topology().neighbors(place)[0];
        let (agent, workstation) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let workstation = txn.create_entity(EntityKind::Facility);
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(workstation, remote_place).unwrap();
            txn.set_component_workstation_marker(
                workstation,
                WorkstationMarker(WorkstationTag::OrchardRow),
            )
            .unwrap();
            txn.set_component_resource_source(
                workstation,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(9),
                    max_quantity: Quantity(12),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                },
            )
            .unwrap();
            commit_txn(txn);
            (agent, workstation)
        };

        let empty_beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &empty_beliefs);
        assert!(
            RuntimeBeliefView::adjacent_places_with_travel_ticks(&view, place)
                .iter()
                .any(|(adjacent, _)| *adjacent == remote_place),
            "public route topology should remain available"
        );
        assert_eq!(
            RuntimeBeliefView::entity_kind(&view, remote_place),
            Some(EntityKind::Place),
            "public route knowledge should include place identity"
        );
        assert!(
            RuntimeBeliefView::matching_workstations_at(
                &view,
                remote_place,
                WorkstationTag::OrchardRow
            )
            .is_empty(),
            "remote workstation discovery must not come from authoritative scans"
        );
        assert!(
            RuntimeBeliefView::resource_sources_at(&view, remote_place, CommodityKind::Apple)
                .is_empty(),
            "remote resource-source discovery must not come from authoritative scans"
        );
        assert_eq!(RuntimeBeliefView::workstation_tag(&view, workstation), None);
        assert_eq!(RuntimeBeliefView::resource_source(&view, workstation), None);

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(
            workstation,
            build_believed_entity_state(
                &world,
                workstation,
                Tick(2),
                worldwake_core::PerceptionSource::DirectObservation,
            )
            .expect("facility should build a believed snapshot"),
        );

        {
            let mut txn = new_txn(&mut world, 3);
            txn.set_component_resource_source(
                workstation,
                ResourceSource {
                    commodity: CommodityKind::Apple,
                    available_quantity: Quantity(3),
                    max_quantity: Quantity(12),
                    regeneration_ticks_per_unit: None,
                    last_regeneration_tick: None,
                },
            )
            .unwrap();
            commit_txn(txn);
        }

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        assert_eq!(
            RuntimeBeliefView::matching_workstations_at(
                &view,
                remote_place,
                WorkstationTag::OrchardRow
            ),
            vec![workstation]
        );
        assert_eq!(
            RuntimeBeliefView::resource_sources_at(&view, remote_place, CommodityKind::Apple),
            vec![workstation]
        );
        assert_eq!(
            RuntimeBeliefView::workstation_tag(&view, workstation),
            Some(WorkstationTag::OrchardRow)
        );
        assert_eq!(
            RuntimeBeliefView::resource_source(&view, workstation),
            Some(ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(9),
                max_quantity: Quantity(12),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            }),
            "belief-side facility/resource knowledge should remain stale until refreshed"
        );
    }

    #[test]
    fn runtime_helpers_support_attacker_visibility_and_duration_estimation() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let place = places[0];
        let destination = world.topology().neighbors(place)[0];
        let (agent, attacker) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let attacker = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(attacker, place).unwrap();
            commit_txn(txn);
            (agent, attacker)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(attacker, entity_belief(place, true, 0, 3));

        let mut defs = ActionDefRegistry::new();
        defs.register(attack_action_def(ActionDefId(0)));
        let mut actions = BTreeMap::new();
        actions.insert(
            ActionInstanceId(7),
            ActionInstance {
                instance_id: ActionInstanceId(7),
                def_id: ActionDefId(0),
                actor: attacker,
                targets: vec![agent],
                payload: ActionPayload::None,
                start_tick: Tick(3),
                remaining_duration: ActionDuration::new(2),
                status: ActionStatus::Active,
                reservation_ids: Vec::new(),
                local_state: None,
            },
        );
        let runtime = PerAgentBeliefRuntime::new(&actions, &defs);
        let view = PerAgentBeliefView::with_runtime(agent, &world, &beliefs, runtime);

        assert_eq!(
            RuntimeBeliefView::current_attackers_of(&view, agent),
            vec![attacker]
        );
        assert_eq!(
            view.estimate_duration(
                agent,
                &DurationExpr::TravelToTarget { target_index: 0 },
                &[destination],
                &ActionPayload::None,
            ),
            Some(crate::ActionDuration::new(
                NonZeroU32::new(
                    world
                        .topology()
                        .outgoing_edges(place)
                        .iter()
                        .filter_map(|edge_id| world.topology().edge(*edge_id))
                        .find(|edge| edge.to() == destination)
                        .unwrap()
                        .travel_time_ticks()
                )
                .unwrap()
                .get(),
            ))
        );
    }

    #[test]
    fn visible_hostiles_exclude_dead_believed_targets() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, attacker) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let attacker = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(attacker, place).unwrap();
            commit_txn(txn);
            (agent, attacker)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(attacker, entity_belief(place, false, 0, 3));

        let mut txn = new_txn(&mut world, 1);
        txn.add_hostility(agent, attacker).unwrap();
        commit_txn(txn);

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert!(
            RuntimeBeliefView::visible_hostiles_for(&view, agent).is_empty(),
            "dead believed hostiles should not continue to project danger"
        );
        assert!(
            RuntimeBeliefView::hostile_targets_of(&view, agent).is_empty(),
            "dead believed hostiles should not remain actionable hostile targets"
        );
    }

    #[test]
    fn estimate_duration_uses_actor_defend_stance_ticks_from_combat_profile() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_combat_profile(
                agent,
                CombatProfile::new(
                    Permille::new(1000).unwrap(),
                    Permille::new(700).unwrap(),
                    Permille::new(600).unwrap(),
                    Permille::new(550).unwrap(),
                    Permille::new(75).unwrap(),
                    Permille::new(20).unwrap(),
                    Permille::new(15).unwrap(),
                    Permille::new(120).unwrap(),
                    Permille::new(30).unwrap(),
                    NonZeroU32::new(6).unwrap(),
                    NonZeroU32::new(10).unwrap(),
                ),
            )
            .unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            view.estimate_duration(
                agent,
                &DurationExpr::ActorDefendStance,
                &[],
                &ActionPayload::None,
            ),
            Some(ActionDuration::new(10))
        );
    }

    #[test]
    fn estimate_duration_uses_actor_consultation_speed_for_records() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, record) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_component_perception_profile(
                agent,
                PerceptionProfile {
                    consultation_speed_factor: Permille::new(250).unwrap(),
                    ..PerceptionProfile::default()
                },
            )
            .unwrap();
            let record = txn
                .create_record(worldwake_core::RecordData {
                    record_kind: worldwake_core::RecordKind::OfficeRegister,
                    home_place: place,
                    issuer: agent,
                    consultation_ticks: 8,
                    max_entries_per_consult: 4,
                    entries: Vec::new(),
                    next_entry_id: 0,
                })
                .unwrap();
            commit_txn(txn);
            (agent, record)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(
            record,
            build_believed_entity_state(
                &world,
                record,
                Tick(2),
                worldwake_core::PerceptionSource::DirectObservation,
            )
            .unwrap(),
        );
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            view.estimate_duration(
                agent,
                &DurationExpr::ConsultRecord { target_index: 0 },
                &[record],
                &ActionPayload::None,
            ),
            Some(ActionDuration::new(2))
        );
    }

    #[test]
    fn political_queries_expose_known_public_office_state_and_actor_private_relations() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, holder, office, faction) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let holder = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let office = txn.create_office("Ledger Hall").unwrap();
            let faction = txn.create_faction("River Pact").unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(holder, place).unwrap();
            txn.add_member(agent, faction).unwrap();
            txn.add_member(holder, faction).unwrap();
            txn.set_component_office_data(
                office,
                OfficeData {
                    title: "Steward".to_string(),
                    jurisdiction: place,
                    succession_law: SuccessionLaw::Support,
                    eligibility_rules: vec![worldwake_core::EligibilityRule::FactionMember(
                        faction,
                    )],
                    succession_period_ticks: 6,
                    vacancy_since: None,
                },
            )
            .unwrap();
            create_record(&mut txn, place, agent, RecordKind::OfficeRegister);
            create_record(&mut txn, place, agent, RecordKind::SupportLedger);
            txn.set_loyalty(agent, holder, Permille::new(620).unwrap())
                .unwrap();
            txn.assign_office(office, holder).unwrap();
            txn.declare_support(agent, office, holder).unwrap();
            txn.set_component_faction_data(
                faction,
                FactionData {
                    name: "River Pact".to_string(),
                    purpose: FactionPurpose::Political,
                },
            )
            .unwrap();
            commit_txn(txn);
            (agent, holder, office, faction)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(holder, entity_belief(place, true, 0, 3));
        beliefs.update_entity(office, entity_belief(place, true, 0, 3));

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::office_data(&view, office)
                .unwrap()
                .jurisdiction,
            place
        );
        assert_eq!(
            RuntimeBeliefView::office_holder(&view, office),
            Some(holder)
        );
        assert_eq!(RuntimeBeliefView::factions_of(&view, agent), vec![faction]);
        assert_eq!(
            RuntimeBeliefView::loyalty_to(&view, agent, holder),
            Some(Permille::new(620).unwrap())
        );
        assert_eq!(
            RuntimeBeliefView::support_declaration(&view, agent, office),
            Some(holder)
        );
    }

    #[test]
    fn believed_office_holder_reads_from_institutional_belief_store() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, holder, office) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let holder = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let office = txn.create_office("Ledger Hall").unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(holder, place).unwrap();
            commit_txn(txn);
            (agent, holder, office)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.institutional_beliefs.insert(
            InstitutionalBeliefKey::OfficeHolderOf { office },
            vec![worldwake_core::BelievedInstitutionalClaim {
                claim: InstitutionalClaim::OfficeHolder {
                    office,
                    holder: Some(holder),
                    effective_tick: Tick(3),
                },
                source: InstitutionalKnowledgeSource::WitnessedEvent,
                learned_tick: Tick(4),
                learned_at: Some(place),
            }],
        );

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::believed_office_holder(&view, office),
            InstitutionalBeliefRead::Certain(Some(holder))
        );
    }

    #[test]
    fn believed_support_declarations_for_office_reads_from_institutional_belief_store() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, supporter, candidate, office) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let supporter = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let candidate = txn.create_agent("Cora", ControlSource::Ai).unwrap();
            let office = txn.create_office("Ledger Hall").unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(supporter, place).unwrap();
            txn.set_ground_location(candidate, place).unwrap();
            commit_txn(txn);
            (agent, supporter, candidate, office)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.institutional_beliefs.insert(
            InstitutionalBeliefKey::SupportFor { supporter, office },
            vec![worldwake_core::BelievedInstitutionalClaim {
                claim: InstitutionalClaim::SupportDeclaration {
                    office,
                    supporter,
                    candidate: Some(candidate),
                    effective_tick: Tick(5),
                },
                source: InstitutionalKnowledgeSource::RecordConsultation {
                    record: office,
                    entry_id: worldwake_core::RecordEntryId(1),
                },
                learned_tick: Tick(6),
                learned_at: Some(place),
            }],
        );

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(
            RuntimeBeliefView::believed_support_declaration(&view, office, supporter),
            InstitutionalBeliefRead::Certain(Some(candidate))
        );
        assert_eq!(
            RuntimeBeliefView::believed_support_declarations_for_office(&view, office),
            vec![(
                supporter,
                InstitutionalBeliefRead::Certain(Some(candidate)),
            )]
        );
    }

    #[test]
    fn believed_owner_of_returns_owner_when_agent_knows_entity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let lot = txn
                .create_item_lot_with_owner(CommodityKind::Bread, Quantity(3), place, Some(agent))
                .unwrap();
            commit_txn(txn);
            (agent, lot)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(lot, entity_belief(place, true, 3, 10));

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        assert_eq!(
            RuntimeBeliefView::believed_owner_of(&view, lot),
            Some(agent)
        );
    }

    #[test]
    fn believed_owner_of_returns_none_for_unowned_entity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let lot = txn
                .create_item_lot_with_owner(CommodityKind::Bread, Quantity(3), place, None)
                .unwrap();
            commit_txn(txn);
            (agent, lot)
        };

        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(lot, entity_belief(place, true, 3, 10));

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        assert_eq!(RuntimeBeliefView::believed_owner_of(&view, lot), None);
    }

    #[test]
    fn believed_owner_of_returns_none_for_unknown_entity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let other = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(other, place).unwrap();
            let lot = txn
                .create_item_lot_with_owner(CommodityKind::Bread, Quantity(3), place, Some(other))
                .unwrap();
            commit_txn(txn);
            (agent, lot)
        };

        // Agent has NO belief about this lot
        let beliefs = AgentBeliefStore::new();

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        assert_eq!(RuntimeBeliefView::believed_owner_of(&view, lot), None);
    }

    #[test]
    fn believed_owner_of_returns_owner_when_self_is_owner() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, lot) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let lot = txn
                .create_item_lot_with_owner(CommodityKind::Bread, Quantity(3), place, Some(agent))
                .unwrap();
            commit_txn(txn);
            (agent, lot)
        };

        // Agent has NO belief entry, but is the owner — accessible via self-ownership check
        let beliefs = AgentBeliefStore::new();

        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        assert_eq!(
            RuntimeBeliefView::believed_owner_of(&view, lot),
            Some(agent)
        );
    }

    #[test]
    fn courage_returns_profile_value_for_self_and_believed_for_observed() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(other, place).unwrap();
            txn.set_component_utility_profile(
                agent,
                UtilityProfile {
                    courage: Permille::new(750).unwrap(),
                    ..UtilityProfile::default()
                },
            )
            .unwrap();
            txn.set_component_utility_profile(
                other,
                UtilityProfile {
                    courage: Permille::new(200).unwrap(),
                    ..UtilityProfile::default()
                },
            )
            .unwrap();
            commit_txn(txn);
            (agent, other)
        };

        // Beliefs include courage for the observed agent.
        let mut belief_state = entity_belief(place, true, 0, 3);
        belief_state.last_known_courage = Some(Permille::new(200).unwrap());
        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(other, belief_state);
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        // Self-authoritative: returns own courage
        assert_eq!(
            RuntimeBeliefView::courage(&view, agent),
            Some(Permille::new(750).unwrap())
        );
        // Other agent: returns believed courage
        assert_eq!(
            RuntimeBeliefView::courage(&view, other),
            Some(Permille::new(200).unwrap())
        );

        // GoalBeliefView delegation matches
        assert_eq!(
            GoalBeliefView::courage(&view, agent),
            Some(Permille::new(750).unwrap())
        );
        assert_eq!(
            GoalBeliefView::courage(&view, other),
            Some(Permille::new(200).unwrap())
        );
    }

    #[test]
    fn courage_returns_none_for_observed_agent_without_courage_belief() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, other) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(other, place).unwrap();
            commit_txn(txn);
            (agent, other)
        };

        // Beliefs exist for other but without courage (last_known_courage = None).
        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(other, entity_belief(place, true, 0, 3));
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(RuntimeBeliefView::courage(&view, other), None);
    }

    #[test]
    fn courage_returns_none_for_unknown_agent() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, unknown) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let unknown = txn.create_agent("Ghost", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(unknown, place).unwrap();
            commit_txn(txn);
            (agent, unknown)
        };

        // No beliefs about the unknown agent at all.
        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(RuntimeBeliefView::courage(&view, unknown), None);
    }

    #[test]
    fn courage_returns_none_when_no_utility_profile() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.clear_component_utility_profile(agent).unwrap();
            commit_txn(txn);
            agent
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);

        assert_eq!(RuntimeBeliefView::courage(&view, agent), None);
    }

    #[test]
    fn support_declarations_for_office_returns_all_declarations() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, holder, other, office) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let holder = txn.create_agent("Bram", ControlSource::Ai).unwrap();
            let other = txn.create_agent("Cora", ControlSource::Ai).unwrap();
            let office = txn.create_office("Ledger Hall").unwrap();
            let faction = txn.create_faction("River Pact").unwrap();
            txn.set_ground_location(agent, place).unwrap();
            txn.set_ground_location(holder, place).unwrap();
            txn.set_ground_location(other, place).unwrap();
            txn.add_member(agent, faction).unwrap();
            txn.add_member(holder, faction).unwrap();
            txn.add_member(other, faction).unwrap();
            txn.set_component_office_data(
                office,
                OfficeData {
                    title: "Steward".to_string(),
                    jurisdiction: place,
                    succession_law: SuccessionLaw::Support,
                    eligibility_rules: vec![worldwake_core::EligibilityRule::FactionMember(
                        faction,
                    )],
                    succession_period_ticks: 6,
                    vacancy_since: None,
                },
            )
            .unwrap();
            create_record(&mut txn, place, agent, RecordKind::SupportLedger);
            txn.declare_support(agent, office, holder).unwrap();
            txn.declare_support(other, office, holder).unwrap();
            commit_txn(txn);
            (agent, holder, other, office)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        let declarations = RuntimeBeliefView::support_declarations_for_office(&view, office);
        assert_eq!(declarations.len(), 2);
        assert!(declarations.contains(&(agent, holder)));
        assert!(declarations.contains(&(other, holder)));
    }

    #[test]
    fn support_declarations_for_office_returns_empty_when_no_declarations() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let (agent, office) = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            let office = txn.create_office("Ledger Hall").unwrap();
            txn.set_ground_location(agent, place).unwrap();
            commit_txn(txn);
            (agent, office)
        };

        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        let declarations = RuntimeBeliefView::support_declarations_for_office(&view, office);
        assert!(declarations.is_empty());
    }

    #[test]
    fn support_declarations_for_office_returns_empty_for_non_office_entity() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = new_txn(&mut world, 1);
            let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
            txn.set_ground_location(agent, place).unwrap();
            commit_txn(txn);
            agent
        };

        // Query using the agent ID as if it were an office — should return empty.
        let beliefs = AgentBeliefStore::new();
        let view = PerAgentBeliefView::new(agent, &world, &beliefs);
        let declarations = RuntimeBeliefView::support_declarations_for_office(&view, agent);
        assert!(declarations.is_empty());
    }
}
